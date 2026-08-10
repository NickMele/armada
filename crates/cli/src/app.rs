//! The runtime every verb is handed: the seams, the store, and the machine
//! facts — assembled once, at the entrypoint, and passed down.
//!
//! Nothing below this file reads the environment, the current directory or
//! `$HOME` (`ARCHITECTURE.md` §1.4). That is not ceremony: `--project` and
//! `--all` operate on workspaces that are *not* the current directory, so a
//! function that can re-derive "the workspace" from cwd makes scoping to a
//! different one require lying about cwd — and you are one `chdir` away from a
//! race between two concurrent runs, which is the exact scenario charkit exists
//! to make safe.

use charkit_adapters::db::{AcquireOutcome, Db};
use charkit_adapters::machine::MachineConfig;
use charkit_adapters::{docker, fs, machine};
use charkit_core::ctx::{Clock, Ctx, Fetch, Run};
use charkit_core::error::{CharError, ErrClass};
use charkit_core::id::WorkspaceId;
use charkit_core::lease::{
    is_cold, ClaimAction, ClaimEvent, ClaimState, LeaseId, LeaseKind, Policy,
};
use charkit_core::reap::{self, LabelledResource, ReapPlan};
use charkit_core::registry::OwnedKind;
use std::collections::BTreeMap;
use std::path::{Path, PathBuf};

/// Everything a verb needs and may not go and find for itself.
pub struct App<R: Run, C: Clock, F: Fetch> {
    /// The three seams and the workspace.
    pub ctx: Ctx<R, C, F>,
    /// The machine-global store.
    pub db: Db,
    /// `~/.char/config.toml`, or its defaults.
    pub machine: MachineConfig,
    /// This installation's namespace, from `char.db`.
    pub namespace: String,
    /// This boot, so a row from a previous one is stale by definition.
    pub boot_id: String,
    /// The environment char was started with, captured once.
    pub inherited: BTreeMap<String, String>,
}

impl<R: Run, C: Clock, F: Fetch> App<R, C, F> {
    /// The directory subprocesses run in. Always the workspace root — PLAN.md
    /// §3.4 calls it a constant rather than a discovery, and `char explain`
    /// leans on that.
    pub fn cwd(&self) -> PathBuf {
        self.ctx
            .workspace
            .as_ref()
            .map(|w| w.root.clone())
            .unwrap_or_else(|| PathBuf::from("/"))
    }

    /// The two variables **every** child inherits (PLAN.md §2.4).
    ///
    /// Neither is declared anywhere and both are always present, so a script
    /// char has never been told anything about still knows which workspace it
    /// is in. `CHAR_RUN_ID` arrives with runs, in phase 3.
    pub fn child_env(&self) -> BTreeMap<String, String> {
        let mut env = BTreeMap::new();
        if let Some(workspace) = &self.ctx.workspace {
            env.insert(
                "CHAR_WORKSPACE".to_string(),
                workspace.id.as_str().to_string(),
            );
        }
        env
    }

    /// Take a lease, driving [`charkit_core::lease::step`].
    ///
    /// The reducer decides; this loop only performs. That split is what puts
    /// the hard cases — a holder that died between heartbeat and reap, a
    /// ceiling expiring mid-wait — inside a unit test rather than inside a
    /// concurrency bug someone reproduces twice a year.
    pub fn acquire(
        &mut self,
        lease: LeaseId,
        policy: Policy,
        ceiling_ms: Option<u64>,
    ) -> Result<LeaseId, CharError> {
        let pid = charkit_adapters::posix::pid();
        let started = machine::process_start_at(&self.ctx.run, &self.cwd(), pid);

        let mut state = ClaimState::new(lease.clone(), policy, ceiling_ms);
        let mut event = ClaimEvent::Start;

        loop {
            let (next, actions) = charkit_core::lease::step(state, event);
            state = next;
            let mut pending: Option<ClaimEvent> = None;

            for action in actions {
                match action {
                    ClaimAction::Attempt => {
                        let now = self.ctx.now.mono();
                        let outcome = self.db.try_acquire(
                            &state.lease,
                            now,
                            &self.boot_id,
                            pid,
                            started.as_deref(),
                        )?;
                        pending = Some(match outcome {
                            AcquireOutcome::Granted => ClaimEvent::Granted,
                            AcquireOutcome::Held(row) => {
                                let holder = charkit_adapters::db::holder_of(&row, now);
                                if is_cold(row.heartbeat_mono, now, row.boot_id == self.boot_id) {
                                    ClaimEvent::HolderCold(holder)
                                } else {
                                    ClaimEvent::Held(holder)
                                }
                            }
                        });
                    }
                    ClaimAction::Reclaim(_) => {
                        self.db.release_lease(state.lease.kind, &state.lease.key)?;
                    }
                    ClaimAction::Sleep { ms } => {
                        let until = self.ctx.now.mono().saturating_add(ms);
                        self.ctx.now.sleep_until(until);
                        pending = Some(ClaimEvent::Slept { ms });
                    }
                    // Phase 2's verbs are not runs, so there is no `results[]`
                    // to put a `WAITING` row in yet. Phase 3's scheduler emits
                    // it; the action exists here because the reducer is the
                    // contract and inventing it twice is the failure this
                    // whole module exists to prevent.
                    ClaimAction::Report(_) => {}
                    ClaimAction::Granted => return Ok(lease),
                    ClaimAction::Failed(error) => return Err(error),
                }
            }

            match pending {
                Some(next) => event = next,
                // A step that produced no action to react to would spin.
                None => {
                    return Err(CharError {
                        class: ErrClass::CharBug,
                        r#where: "lease".to_string(),
                        message: format!("the claim loop stalled in {:?}", state.phase),
                        next_action: None,
                    })
                }
            }
        }
    }

    /// Give a lease back.
    pub fn release(&mut self, lease: &LeaseId) -> Result<(), CharError> {
        self.db.release_lease(lease.kind, &lease.key)
    }

    /// The three reap passes, **decided but not executed**.
    ///
    /// Split from [`App::reap`] so `--dry-run` previews exactly what the real
    /// pass would do rather than an approximation of it. Listing and stat-ing
    /// change nothing, so the preview is free of side effects and identical to
    /// the decision the executing pass makes a moment later.
    pub fn plan_reap(&mut self) -> Result<ReapPlan, CharError> {
        let mut plan = ReapPlan::default();

        // Pass 1: the registry.
        let rows = self.db.workspaces()?;
        let stated: Vec<_> = rows
            .into_iter()
            .map(|row| {
                let stat = fs::stat(&row.path);
                (row, stat)
            })
            .collect();
        plan.workspaces = reap::registry_pass(&stated);

        // Pass 2: labelled resources. Skipped rather than fatal when Docker is
        // not there: a repo with no services needs no daemon, and failing
        // `char init` because Docker Desktop is closed would make the ownership
        // layer unusable for the majority of repos that never start a
        // container.
        let timeout = self.machine.docker_deadline();
        match docker::daemon_ready(&self.ctx.run, &self.cwd(), timeout) {
            Ok(()) => {
                for kind in docker::Kind::ALL {
                    let found = docker::list_labelled(&self.ctx.run, &self.cwd(), timeout, kind)?;
                    let stated: Vec<(LabelledResource, reap::PathStat)> = found
                        .into_iter()
                        .map(|resource| {
                            let stat = fs::stat(&resource.workspace_path);
                            (resource, stat)
                        })
                        .collect();
                    let (remove, reported) = reap::resource_pass(&stated, &self.namespace);
                    plan.resources.extend(remove);
                    plan.reported.extend(reported);
                }
            }
            Err(error) => plan
                .skipped
                .push(format!("labelled resources: {}", error.message)),
        }

        // Pass 3: cold leases.
        let now = self.ctx.now.mono();
        plan.leases = reap::lease_pass(&self.db.leases()?, now, &self.boot_id);

        Ok(plan)
    }

    /// The three reap passes, executed.
    ///
    /// **Runs at `init` *and* `clean`, because `init`-only misses shrinkage
    /// entirely.** Repeated worktree create/destroy always runs `init` in the
    /// new one — true for churn, false for the last one. Delete worktree 5 of 5
    /// and nothing reaps until somebody creates worktree 6, which on a
    /// shrinking project is never.
    pub fn reap(&mut self) -> Result<ReapPlan, CharError> {
        let plan = self.plan_reap()?;
        let timeout = self.machine.docker_deadline();

        for id in &plan.workspaces {
            self.db.release_workspace(id)?;
        }

        for target in &plan.resources {
            let kind = match target.kind {
                OwnedKind::Container => docker::Kind::Container,
                OwnedKind::Network => docker::Kind::Network,
                OwnedKind::Volume => docker::Kind::Volume,
                OwnedKind::Image => docker::Kind::Image,
                // A pgid or a release command never reaches pass 2: neither is
                // a labelled docker resource, and both are handled where they
                // are recorded.
                OwnedKind::Pgid | OwnedKind::Release => continue,
            };
            let removed = docker::remove(
                &self.ctx.run,
                &self.cwd(),
                timeout,
                kind,
                std::slice::from_ref(&target.reference),
            );
            // Remove first, forget second: a resource removed before its row
            // leaves a stale row the next `init` reaps for free, while the
            // other order leaves a live resource nothing can find.
            if removed.iter().all(|(_, error)| error.is_none()) {
                self.db
                    .delete_owned(&target.workspace, target.kind, &target.reference)?;
            }
        }

        let now = self.ctx.now.mono();
        for row in self.db.leases()? {
            if is_cold(row.heartbeat_mono, now, row.boot_id == self.boot_id) {
                self.db.release_lease(row.kind, &row.key)?;
            }
        }

        Ok(plan)
    }

    /// Stop every process group a workspace owns, and say what happened.
    ///
    /// **Nothing is killed that char cannot prove is its own.** A row from a
    /// previous boot, or one whose pid now has a different start time, is a
    /// recycled pid rather than an orphaned service — so the row is dropped and
    /// no signal is sent.
    pub fn stop_owned_processes(&mut self, workspace: &WorkspaceId) -> Result<usize, CharError> {
        let rows = self.db.owned(Some(workspace))?;
        let mut stopped = 0;

        for row in rows.iter().filter(|r| r.kind == OwnedKind::Pgid) {
            let Ok(pgid) = row.reference.parse::<i32>() else {
                self.db
                    .delete_owned(workspace, OwnedKind::Pgid, &row.reference)?;
                continue;
            };
            let observed = machine::process_start_at(&self.ctx.run, &self.cwd(), pgid);
            if reap::pgid_is_ours(
                row.boot_id.as_deref(),
                row.pid_started_at.as_deref(),
                &self.boot_id,
                observed.as_deref(),
            ) {
                let report =
                    charkit_adapters::posix::stop_group(pgid, charkit_adapters::process::GRACE);
                if report.existed && report.gone {
                    stopped += 1;
                }
            }
            self.db
                .delete_owned(workspace, OwnedKind::Pgid, &row.reference)?;
        }
        Ok(stopped)
    }
}

/// Assemble the runtime. The only place `$HOME`, the cwd and the environment
/// are read.
pub fn build<R: Run, C: Clock, F: Fetch>(
    ctx: Ctx<R, C, F>,
    home: &Path,
    inherited: BTreeMap<String, String>,
) -> Result<App<R, C, F>, CharError> {
    let db = Db::open(&machine::char_home(home))?;
    let namespace = db.namespace()?;
    let machine_config = MachineConfig::read(&machine::char_home(home))?;

    let cwd = ctx
        .workspace
        .as_ref()
        .map(|w| w.root.clone())
        .unwrap_or_else(|| PathBuf::from("/"));

    // Required rather than optional. Without a boot id every lease looks stale
    // across a reboot and every recorded pgid is unreclaimable — so char would
    // either steal live leases or leak processes forever, and both are worse
    // than refusing to run on a platform that cannot answer.
    let boot_id = machine::boot_id(&ctx.run, &cwd).ok_or_else(|| CharError {
        class: ErrClass::Environment,
        r#where: "boot_id".to_string(),
        message: "this machine reports no boot id".to_string(),
        next_action: Some(
            "char needs `sysctl kern.bootsessionuuid` on darwin or \
             /proc/sys/kernel/random/boot_id on Linux"
                .to_string(),
        ),
    })?;

    Ok(App {
        ctx,
        db,
        machine: machine_config,
        namespace,
        boot_id,
        inherited,
    })
}

/// Hold a lease for the length of one operation, releasing it whatever happens.
///
/// **The run lease covers `init`, `up`, `down`, `clean`, `check` and every
/// `commands:` entry — everything that mutates.** Two agents in the same
/// worktree is the ordinary case this project assumes, and without it their
/// `init` runs interleave setup steps against the same tree, or one's `clean`
/// tears down what the other's `up` is mid-way through starting.
pub fn with_lease<R: Run, C: Clock, F: Fetch, T>(
    app: &mut App<R, C, F>,
    lease: LeaseId,
    policy: Policy,
    ceiling_ms: Option<u64>,
    body: impl FnOnce(&mut App<R, C, F>) -> Result<T, CharError>,
) -> Result<T, CharError> {
    let held = app.acquire(lease, policy, ceiling_ms)?;
    let outcome = body(app);
    // Released on the failure path too: a verb that dies holding the run lease
    // would make the next invocation wait out a cold heartbeat for no reason.
    let _ = app.release(&held);
    outcome
}

/// The kinds of lease `clean` releases early, and the ones it must not.
///
/// **Only the *resource* leases go early**, and only because holding a cpu-slot
/// while tearing down blocks other workspaces for no reason. The run lease is
/// held throughout and released last — an earlier version of `clean`'s ordering
/// released it first, annotated "so nothing new starts", which is precisely
/// what lets something new start: a concurrent `char up` takes the freed lease
/// and starts services into a workspace being torn down.
pub const RELEASED_EARLY: [LeaseKind; 2] = [LeaseKind::CpuSlot, LeaseKind::Exclusive];
