//! The runtime every verb is handed: the seams, the store, and the machine
//! facts — assembled once, at the entrypoint, and passed down.
//!
//! Nothing below this file reads the environment, the current directory or
//! `$HOME` (`ARCHITECTURE.md` §1.4). That is not ceremony: `--project` and
//! `--all` operate on workspaces that are *not* the current directory, so a
//! function that can re-derive "the workspace" from cwd makes scoping to a
//! different one require lying about cwd — and you are one `chdir` away from a
//! race between two concurrent runs, which is the exact scenario Armada exists
//! to make safe.

use armada_core::ctx::{Clock, Ctx, Fetch, Run};
use armada_core::error::{ArmadaError, ErrClass};
use armada_core::id::WorkspaceId;
use armada_core::lease::{
    is_cold, ClaimAction, ClaimEvent, ClaimState, LeaseId, LeaseKind, Policy,
};
use armada_core::reap::{self, LabelledResource, ReapPlan};
use armada_core::registry::OwnedKind;
use armada_manifest::db::{AcquireOutcome, Db, SlotOutcome as AcquireSlots};
use armada_manifest::machine::MachineConfig;
use armada_manifest::{docker, fs, machine};
use std::collections::BTreeMap;
use std::path::{Path, PathBuf};

/// Where one non-blocking claim got to.
///
/// **The reason this type exists is that the shell's event loop may never
/// block.** A blocking claim answers "someone is in the way" by sleeping, which
/// parks whatever loop called it — for `check` that is the run loop, so for the
/// length of the wait no deadline fires, no exited child is reaped and no
/// interrupt is observed. `Waiting` hands that decision back to the caller,
/// whose next turn comes round in milliseconds.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Claim {
    /// Held by this process now, and these are the rows to give back.
    Granted(Vec<LeaseId>),
    /// A live holder is in the way. What to say about it went to `on_wait`.
    Waiting,
}

/// Where one turn of the claim reducer got to. The private half of [`Claim`]:
/// it carries the sleep the reducer asked for, which only a blocking caller
/// has any use for.
enum Turn {
    /// The lease is ours.
    Granted,
    /// A live holder is in the way, and the reducer asked for this long.
    Waiting {
        /// Milliseconds.
        ms: u64,
    },
}

/// This process, as the store records a holder: its pid and its start time.
///
/// The start time is what tells a recycled pid from the original, and reading
/// it costs a subprocess — so it is read once per claim rather than once per
/// attempt.
type Claimant = (i32, Option<String>);

/// Everything a verb needs and may not go and find for itself.
pub struct App<R: Run, C: Clock, F: Fetch> {
    /// The three seams and the workspace.
    pub ctx: Ctx<R, C, F>,
    /// The machine-global store.
    pub db: Db,
    /// `~/.armada/machine.yml`, or its defaults.
    pub machine: MachineConfig,
    /// This installation's namespace, from `manifest.db`.
    pub namespace: String,
    /// This boot, so a row from a previous one is stale by definition.
    pub boot_id: String,
    /// The environment Armada was started with, captured once.
    pub inherited: BTreeMap<String, String>,
    /// The run this invocation belongs to, when it is inside one.
    ///
    /// Set for `check` and cleared everywhere else, which is what makes
    /// [`App::child_env`] able to answer PLAN.md §2.4 without asking anything:
    /// `armada manifest up` is not a run and has no run id, so a service's environment
    /// carries `ARMADA_WORKSPACE` alone.
    pub run: Option<armada_core::run::RunId>,
    /// The leases this invocation is holding right now, innermost last.
    ///
    /// Two things need it and neither can re-derive it: a heartbeat has to be
    /// renewed from whatever loop is currently running, and a teardown that
    /// deletes a workspace's leases must not delete the one it is standing on.
    pub held: Vec<LeaseId>,
}

impl<R: Run, C: Clock, F: Fetch> App<R, C, F> {
    /// The directory subprocesses run in. Always the workspace root — PLAN.md
    /// §3.4 calls it a constant rather than a discovery, and `armada manifest explain`
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
    /// Neither is declared anywhere, so a script Armada has never been told
    /// anything about still knows which workspace it is in.
    ///
    /// **`ARMADA_RUN_ID` is present only inside a run**, and the two travel
    /// together on purpose: a child reading them decides whether to *join* this
    /// run or start its own, and that decision is `ARMADA_WORKSPACE` matching the
    /// workspace it resolved for itself (PLAN.md §3.2.1). One without the other
    /// is not an inheritance — which is why `armada manifest up`, which is not a run,
    /// sets the workspace alone.
    pub fn child_env(&self) -> BTreeMap<String, String> {
        let mut env = BTreeMap::new();
        if let Some(workspace) = &self.ctx.workspace {
            env.insert(
                "ARMADA_WORKSPACE".to_string(),
                workspace.id.as_str().to_string(),
            );
        }
        if let Some(run) = &self.run {
            env.insert("ARMADA_RUN_ID".to_string(), run.to_string());
        }
        env
    }

    /// Take a lease, driving [`armada_core::lease::step`].
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
    ) -> Result<LeaseId, ArmadaError> {
        self.acquire_reporting(lease, policy, ceiling_ms, &mut |_| {})
    }

    /// Take a lease, and say out loud what is in the way while waiting.
    ///
    /// **This is the consumer `ClaimAction::Report` was built without.** Phase 2
    /// had no run with a `results[]` to put a `WAITING` row in, so the action
    /// existed and nothing performed it; a run has one, and the row is the whole
    /// difference between a wait that is visible and a wait that is silent. An
    /// earlier design's defect was never the blocking — it was blocking
    /// invisibly and without a ceiling (PLAN.md §4.3).
    pub fn acquire_reporting(
        &mut self,
        lease: LeaseId,
        policy: Policy,
        ceiling_ms: Option<u64>,
        on_wait: &mut dyn FnMut(armada_core::lease::WaitingOn),
    ) -> Result<LeaseId, ArmadaError> {
        let who = self.claimant();
        let mut state = ClaimState::new(lease.clone(), policy, ceiling_ms);
        let mut event = ClaimEvent::Start;
        // The row the last attempt saw. A reclaim deletes *that* row and no
        // other, so a holder that renewed between the observation and the
        // delete keeps its lease.
        let mut observed: Option<armada_core::registry::LeaseRow> = None;

        loop {
            let (next, turn) = self.turn(state, event, &mut observed, &who, on_wait)?;
            state = next;
            match turn {
                Turn::Granted => return Ok(lease),
                Turn::Waiting { ms } => {
                    let until = self.ctx.now.mono().saturating_add(ms);
                    self.ctx.now.sleep_until(until);
                    // **Renewed from the loop that waits, not a background
                    // timer.** Without this a claim queueing behind another
                    // workspace would stop renewing the leases this invocation
                    // *already holds* — including its own run lease — and a
                    // lease goes cold after sixty seconds of silence. A third
                    // process would then reclaim the run lease out from under a
                    // run that is behaving exactly as specified.
                    self.renew_held();
                    event = ClaimEvent::Slept { ms };
                }
            }
        }
    }

    /// One turn at a named lease, **without sleeping**.
    ///
    /// The blocking claim above is a caller that answers `Turn::Waiting` by
    /// sleeping. This is the caller that answers it by going back to its own
    /// event loop — which is what `verbs::check`'s loop must do, because
    /// waiting inside one action is waiting with no deadline firing and no
    /// exited child reaped (`ARCHITECTURE.md` §1.2: **the loop never blocks**).
    ///
    /// No ceiling is passed, and that is not an omission: a ceiling is measured
    /// in accumulated sleeps, and a caller that does not sleep here has to
    /// measure the wait against its own clock. `verbs::check` reads it off the
    /// scheduler's `Waiting::since_mono`, which is the only record of when the
    /// claim began.
    pub fn claim(
        &mut self,
        lease: &LeaseId,
        on_wait: &mut dyn FnMut(armada_core::lease::WaitingOn),
    ) -> Result<Claim, ArmadaError> {
        let who = self.claimant();
        let state = ClaimState::new(lease.clone(), Policy::Block, None);
        let mut observed: Option<armada_core::registry::LeaseRow> = None;
        let (_, turn) = self.turn(state, ClaimEvent::Start, &mut observed, &who, on_wait)?;
        Ok(match turn {
            Turn::Granted => Claim::Granted(vec![lease.clone()]),
            Turn::Waiting { .. } => Claim::Waiting,
        })
    }

    /// One turn at `count` CPU slots, **without sleeping**.
    ///
    /// **Through `lease::step`, not around it** — the visible wait and the
    /// meaning of `Attempt` are stated there, and a second loop would be a
    /// second set of answers. `Attempt` means *re-decide and try*, which is
    /// exactly what a counted budget needs: losing means the answer changed, so
    /// the next attempt asks the store again rather than waiting for a slot it
    /// already picked.
    ///
    /// The slots' *identity* is the store's, chosen all-or-nothing inside one
    /// transaction — see
    /// [`Db::try_acquire_slots`](armada_manifest::db::Db::try_acquire_slots)
    /// for why a partial claim deadlocks and why numbering a check's own slots
    /// `0..cost` hangs the second check that starts.
    pub fn claim_slots(
        &mut self,
        workspace: &WorkspaceId,
        count: u32,
        total: u32,
        on_wait: &mut dyn FnMut(armada_core::lease::WaitingOn),
    ) -> Result<Claim, ArmadaError> {
        let (pid, started) = self.claimant();
        let claim = LeaseId {
            workspace: Some(workspace.clone()),
            kind: LeaseKind::CpuSlot,
            // A budget has no one key. This names the *claim* so the reducer
            // and the payload have something to say; the rows the store writes
            // carry the real slot numbers.
            key: format!("{count} of {total}"),
        };

        let mut state = ClaimState::new(claim, Policy::Block, None);
        let mut event = ClaimEvent::Start;
        let mut granted: Vec<LeaseId> = Vec::new();

        loop {
            let (next, actions) = armada_core::lease::step(state, event);
            state = next;
            let mut pending: Option<ClaimEvent> = None;

            for action in actions {
                match action {
                    ClaimAction::Attempt => {
                        let now = self.ctx.now.mono();
                        let outcome = self.db.try_acquire_slots(
                            workspace,
                            count,
                            total,
                            now,
                            &self.boot_id,
                            pid,
                            started.as_deref(),
                        )?;
                        pending = Some(match outcome {
                            AcquireSlots::Granted(leases) => {
                                granted = leases;
                                ClaimEvent::Granted
                            }
                            AcquireSlots::Short { held, free } => {
                                on_wait(armada_core::lease::WaitingOn::CpuSlot {
                                    cpu_slot: count,
                                    available: free,
                                });
                                ClaimEvent::Held(match held {
                                    Some(row) => armada_manifest::db::holder_of(&row, now),
                                    None => armada_core::lease::Holder {
                                        workspace: Some(workspace.clone()),
                                        pid,
                                        held_ms: 0,
                                    },
                                })
                            }
                        });
                    }
                    // A cold slot is reclaimed inside the same transaction that
                    // counts them, because a slot has no fixed key for the
                    // claim loop to name.
                    ClaimAction::Reclaim(_) => {}
                    // The caller's loop is the thing that waits.
                    ClaimAction::Sleep { .. } => return Ok(Claim::Waiting),
                    ClaimAction::Report(waiting_on) => on_wait(waiting_on),
                    ClaimAction::Granted => return Ok(Claim::Granted(granted)),
                    ClaimAction::Failed(error) => return Err(error),
                }
            }

            match pending {
                Some(next) => event = next,
                None => {
                    return Err(ArmadaError {
                        class: ErrClass::ArmadaBug,
                        r#where: "cpu-slot".to_string(),
                        message: format!("the slot claim stalled in {:?}", state.phase),
                        next_action: None,
                    })
                }
            }
        }
    }

    /// Who this process is, as the store records a holder.
    fn claimant(&self) -> Claimant {
        let pid = armada_manifest::posix::pid();
        let started = machine::process_start_at(&self.ctx.run, &self.cwd(), pid);
        (pid, started)
    }

    /// Drive [`armada_core::lease::step`] as far as it goes without sleeping.
    ///
    /// **The one statement of what an attempt is**, so the two callers above
    /// cannot disagree about it: what `Attempt` asks the store, when a cold
    /// holder is reclaimed, and which row a reclaim deletes. They differ only
    /// in what they do with `Turn::Waiting`.
    fn turn(
        &mut self,
        state: ClaimState,
        entry: ClaimEvent,
        observed: &mut Option<armada_core::registry::LeaseRow>,
        who: &Claimant,
        on_wait: &mut dyn FnMut(armada_core::lease::WaitingOn),
    ) -> Result<(ClaimState, Turn), ArmadaError> {
        let (pid, started) = who;
        let mut state = state;
        let mut event = entry;

        loop {
            let (next, actions) = armada_core::lease::step(state, event);
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
                            *pid,
                            started.as_deref(),
                        )?;
                        pending = Some(match outcome {
                            AcquireOutcome::Granted => {
                                *observed = None;
                                ClaimEvent::Granted
                            }
                            AcquireOutcome::Held(row) => {
                                let holder = armada_manifest::db::holder_of(&row, now);
                                let cold =
                                    is_cold(row.heartbeat_mono, now, row.boot_id == self.boot_id);
                                *observed = Some(row);
                                if cold {
                                    ClaimEvent::HolderCold(holder)
                                } else {
                                    ClaimEvent::Held(holder)
                                }
                            }
                        });
                    }
                    ClaimAction::Reclaim(_) => {
                        if let Some(row) = observed.as_ref() {
                            self.db
                                .reclaim_lease(state.lease.kind, &state.lease.key, row)?;
                        }
                    }
                    ClaimAction::Sleep { ms } => return Ok((state, Turn::Waiting { ms })),
                    ClaimAction::Report(waiting_on) => on_wait(waiting_on),
                    ClaimAction::Granted => return Ok((state, Turn::Granted)),
                    ClaimAction::Failed(error) => return Err(error),
                }
            }

            match pending {
                Some(next) => event = next,
                // A step that produced no action to react to would spin.
                None => {
                    return Err(ArmadaError {
                        class: ErrClass::ArmadaBug,
                        r#where: "lease".to_string(),
                        message: format!("the claim loop stalled in {:?}", state.phase),
                        next_action: None,
                    })
                }
            }
        }
    }

    /// Give a lease back.
    pub fn release(&mut self, lease: &LeaseId) -> Result<(), ArmadaError> {
        self.db.release_lease(lease.kind, &lease.key)
    }

    /// Renew every lease this invocation holds, from whatever loop is running.
    ///
    /// The same rule as the child-process heartbeat and for the same reason: a
    /// lease goes cold after sixty seconds of silence, and Armada's own work can
    /// take longer than that — a hung Docker daemon answers each call at the
    /// deadline. A holder that stops renewing while it is still working gets
    /// its lease reclaimed underneath it. A failed renewal is not fatal; the
    /// next one, or the next attempt at whatever failed, reports it.
    pub fn renew_held(&mut self) {
        if self.held.is_empty() {
            return;
        }
        let now = self.ctx.now.mono();
        for lease in std::mem::take(&mut self.held) {
            let _ = self.db.renew(&lease, now);
            self.held.push(lease);
        }
    }

    /// Make one docker call with the held leases' heartbeat driven from inside
    /// it.
    ///
    /// **Every docker call Armada makes on its own account goes through here**,
    /// and the renewal is handed *down* to the adapter rather than wrapped
    /// around it. Wrapping is not enough: `docker::remove` runs one `docker rm`
    /// per handle and `list_labelled` is an `ls` plus an `inspect`, each capped
    /// at `docker_timeout` — so three handles against a hung daemon is ninety
    /// seconds inside a single wrapper call, and a lease goes cold at sixty. A
    /// renewal the adapter drives per subprocess is bounded by one call however
    /// many an adapter function makes, and a future fifth caller inherits it
    /// without knowing it exists.
    fn docker_call<T>(
        &mut self,
        body: impl FnOnce(&R, &Path, std::time::Duration, &mut dyn FnMut()) -> T,
    ) -> T {
        let cwd = self.cwd();
        let timeout = self.machine.docker_deadline();
        let held = self.held.clone();
        let db = &mut self.db;
        let clock = &self.ctx.now;
        let run = &self.ctx.run;
        let mut tick = || {
            let stamp = clock.mono();
            for lease in &held {
                let _ = db.renew(lease, stamp);
            }
        };
        body(run, &cwd, timeout, &mut tick)
    }

    /// Whether the daemon is reachable.
    pub fn docker_ready(&mut self) -> Result<(), ArmadaError> {
        self.docker_call(|run, cwd, timeout, tick| docker::daemon_ready(run, cwd, timeout, tick))
    }

    /// Every resource of one kind carrying Armada's labels.
    pub fn docker_list_labelled(
        &mut self,
        kind: docker::Kind,
    ) -> Result<Vec<LabelledResource>, ArmadaError> {
        self.docker_call(|run, cwd, timeout, tick| {
            docker::list_labelled(run, cwd, timeout, kind, tick)
        })
    }

    /// Every handle matching a declared `owns:` selector.
    pub fn docker_list_by_selector(
        &mut self,
        kind: docker::Kind,
        selector: &str,
    ) -> Result<Vec<String>, ArmadaError> {
        self.docker_call(|run, cwd, timeout, tick| {
            docker::list_by_selector(run, cwd, timeout, kind, selector, tick)
        })
    }

    /// Remove resources by handle, best-effort per handle.
    pub fn docker_remove(
        &mut self,
        kind: docker::Kind,
        references: &[String],
    ) -> Vec<(String, Option<ArmadaError>)> {
        self.docker_call(|run, cwd, timeout, tick| {
            docker::remove(run, cwd, timeout, kind, references, tick)
        })
    }

    /// Drop a workspace's rows, keeping the leases this invocation holds.
    ///
    /// See [`Db::release_workspace`](armada_manifest::db::Db::release_workspace):
    /// `clean` dismantles a workspace while standing on that workspace's own
    /// run lease.
    pub fn forget_workspace(&mut self, workspace: &WorkspaceId) -> Result<(), ArmadaError> {
        let held = std::mem::take(&mut self.held);
        let outcome = self.db.release_workspace(workspace, &held);
        self.held = held;
        outcome
    }

    /// The three reap passes, **decided but not executed**.
    ///
    /// Split from [`App::reap`] so `--dry-run` previews exactly what the real
    /// pass would do rather than an approximation of it. Listing and stat-ing
    /// change nothing, so the preview is free of side effects and identical to
    /// the decision the executing pass makes a moment later.
    pub fn plan_reap(&mut self) -> Result<ReapPlan, ArmadaError> {
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
        // `armada manifest init` because Docker Desktop is closed would make the ownership
        // layer unusable for the majority of repos that never start a
        // container.
        match self.docker_ready() {
            Ok(()) => {
                for kind in docker::Kind::ALL {
                    // **An enumeration that fails is skipped, exactly as an
                    // unreachable daemon is** — the *skipped* category, not the
                    // *refused* one. `list_labelled` lists ids and then
                    // inspects them as a second call, and `docker inspect`
                    // exits non-zero if any of those objects has disappeared in
                    // between; the thing that vanished was somebody else's and
                    // is already gone, so nothing Armada owns failed to be
                    // reclaimed. A failed *removal* is the other category and
                    // is never routed here.
                    match self.docker_list_labelled(kind) {
                        Ok(found) => {
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
                        Err(error) => plan.skipped.push(skipped_enumeration(&error)),
                    }
                }
            }
            Err(error) => plan.skipped.push(skipped_enumeration(&error)),
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
    pub fn reap(&mut self) -> Result<ReapPlan, ArmadaError> {
        let mut plan = self.plan_reap()?;
        let mut survived: Vec<String> = Vec::new();

        for id in &plan.workspaces {
            // **Kill before forgetting, the same order `clean` uses.** Dropping
            // the rows first would leave an orphaned workspace's recorded
            // process groups alive with no record of them anywhere, which is
            // the unreclaimable state the machine-global store exists to
            // prevent.
            let stopped = self.stop_owned_processes(id)?;
            // A group still alive after the SIGKILL escalation is the *refused*
            // category, and it is reported here for the same reason `clean`
            // puts it in `results[].error`: the reap plan has no per-row status
            // to fail, but a reclaim Armada could not complete must never be
            // silent.
            survived.extend(
                stopped
                    .survived
                    .iter()
                    .map(|pgid| format!("process group {pgid} of {id} survived SIGKILL")),
            );
            self.forget_workspace(id)?;
        }
        plan.skipped.extend(survived);

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
            let removed = self.docker_remove(kind, std::slice::from_ref(&target.reference));
            // Remove first, forget second: a resource removed before its row
            // leaves a stale row the next `init` reaps for free, while the
            // other order leaves a live resource nothing can find.
            if removed.iter().all(|(_, error)| error.is_none()) {
                self.db
                    .delete_owned(&target.workspace, target.kind, &target.reference)?;
            }
        }

        // Pass 3, as a compare-and-delete for the same reason the claim loop's
        // reclaim is one: the rows were read once, and a holder that was at
        // 59.9 seconds of silence when they were read renews immediately
        // afterwards. Deleting on `(kind, key)` alone would take that now-warm
        // lease and let a third process acquire it while the holder is still
        // working, which is two holders of one exclusive with no error
        // anywhere.
        let now = self.ctx.now.mono();
        for row in self.db.leases()? {
            if is_cold(row.heartbeat_mono, now, row.boot_id == self.boot_id) {
                self.db.reclaim_lease(row.kind, &row.key, &row)?;
            }
        }

        Ok(plan)
    }

    /// Stop every process group a workspace owns, and say what happened.
    ///
    /// **Nothing is killed that Armada cannot prove is its own.** A row from a
    /// previous boot, or one whose pid now has a different start time, is a
    /// recycled pid rather than an orphaned service — so the row is dropped and
    /// no signal is sent.
    pub fn stop_owned_processes(
        &mut self,
        workspace: &WorkspaceId,
    ) -> Result<StopSummary, ArmadaError> {
        let rows = self.db.owned(Some(workspace))?;
        let mut summary = StopSummary::default();

        for row in rows.iter().filter(|r| r.kind == OwnedKind::Pgid) {
            // **A pgid of zero is not a pgid.** `killpg(0, …)` signals the
            // *caller's* own group, so a `0` written by any future recorder —
            // `ProcessGroup::spawn` stores one for a child that got no new
            // session — would have `armada manifest clean` SIGTERM and then SIGKILL Armada
            // itself and everything sharing its foreground group. A row Armada
            // cannot act on is dropped, exactly as an unparseable one is.
            let pgid = match row.reference.parse::<i32>() {
                Ok(pgid) if pgid > 0 => pgid,
                Ok(_) | Err(_) => {
                    self.db
                        .delete_owned(workspace, OwnedKind::Pgid, &row.reference)?;
                    continue;
                }
            };
            let observed = machine::process_start_at(&self.ctx.run, &self.cwd(), pgid);
            if reap::pgid_is_ours(
                row.boot_id.as_deref(),
                row.pid_started_at.as_deref(),
                &self.boot_id,
                observed.as_deref(),
            ) {
                let report =
                    armada_manifest::posix::stop_group(pgid, armada_manifest::process::GRACE);
                // `GRACE` per surviving group, plus a `ps` spawn each: the same
                // waiting a docker call does, so the same renewal.
                self.renew_held();
                match (report.existed, report.gone) {
                    (true, true) => summary.stopped += 1,
                    // A group that survived SIGKILL is uninterruptible or
                    // unreachable, and saying nothing about it is the silence
                    // this whole layer is written against.
                    (true, false) => summary.survived.push(pgid),
                    (false, _) => {}
                }
            }
            self.db
                .delete_owned(workspace, OwnedKind::Pgid, &row.reference)?;
        }
        Ok(summary)
    }
}

/// How a failed *enumeration* is worded, wherever it happens.
///
/// **Two categories, and they must not merge.** Armada could not *look* — a list
/// or an inspect that lost a race with another workspace's teardown — which
/// proves nothing about what Armada owns and is recorded without failing
/// anything. Armada could not *reclaim* — a `docker rm` that returned non-zero
/// for a handle this workspace owns, or a process group still alive after
/// SIGKILL — which is a real leak and fails the row that owns it. Merging them
/// would make a benign race under five concurrent worktrees exit 1, and that is
/// the concurrency this project is built around.
pub fn skipped_enumeration(error: &ArmadaError) -> String {
    format!("labelled resources: {}", error.message)
}

/// What [`App::stop_owned_processes`] managed, and what it did not.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct StopSummary {
    /// Groups that existed and are gone now.
    pub stopped: usize,
    /// Groups still alive after SIGTERM, the grace period and SIGKILL.
    pub survived: Vec<i32>,
}

/// Assemble the runtime. The only place `$HOME`, the cwd and the environment
/// are read.
pub fn build<R: Run, C: Clock, F: Fetch>(
    ctx: Ctx<R, C, F>,
    home: &Path,
    inherited: BTreeMap<String, String>,
) -> Result<App<R, C, F>, ArmadaError> {
    let db = Db::open(&machine::armada_home(home))?;
    let namespace = db.namespace()?;
    let machine_config = MachineConfig::read(&machine::armada_home(home))?;

    let cwd = ctx
        .workspace
        .as_ref()
        .map(|w| w.root.clone())
        .unwrap_or_else(|| PathBuf::from("/"));

    // Required rather than optional. Without a boot id every lease looks stale
    // across a reboot and every recorded pgid is unreclaimable — so Armada would
    // either steal live leases or leak processes forever, and both are worse
    // than refusing to run on a platform that cannot answer.
    let boot_id = machine::boot_id(&ctx.run, &cwd).ok_or_else(|| ArmadaError {
        class: ErrClass::Environment,
        r#where: "boot_id".to_string(),
        message: "this machine reports no boot id".to_string(),
        next_action: Some(
            "Armada needs `sysctl kern.bootsessionuuid` on darwin or \
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
        run: None,
        held: Vec::new(),
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
    body: impl FnOnce(&mut App<R, C, F>) -> Result<T, ArmadaError>,
) -> Result<T, ArmadaError> {
    let held = app.acquire(lease, policy, ceiling_ms)?;
    app.held.push(held.clone());
    let outcome = body(app);
    app.held.pop();
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
/// what lets something new start: a concurrent `armada manifest up` takes the freed lease
/// and starts services into a workspace being torn down.
pub const RELEASED_EARLY: [LeaseKind; 2] = [LeaseKind::CpuSlot, LeaseKind::Exclusive];
