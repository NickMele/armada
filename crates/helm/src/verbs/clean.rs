//! `char clean` — release everything this workspace owns.
//!
//! **The order is fixed, because a SIGKILL part-way through must not make
//! things worse:**
//!
//! ```text
//! 0. run lease   HELD THROUGHOUT — released last, at step 7
//! 1. leases      release this workspace's cpu-slots and exclusives only
//! 2. processes   killpg TERM -> grace -> KILL, confirm gone
//! 3. docker      containers, then networks and volumes, then built images
//! 4. ports       release the block
//! 5. rows        delete owned/workspaces rows
//! 6. .char/      remove the directory
//! 7. run lease   release
//! ```
//!
//! Kill and remove **before** deleting any row: a row deleted first leaves a
//! live process with no record, which is the unreclaimable state this whole
//! design exists to prevent, while a resource removed first leaves a stale row
//! the next `init` reaps for free. The asymmetry is total — one direction
//! degrades to a leak, the other to a no-op.
//!
//! **Step 0 is the whole point**, and an earlier version of this list got it
//! backwards: it released the run lease *first*, annotated "so nothing new
//! starts", which is precisely what lets something new start. A concurrent
//! `char up` takes the freed lease and starts services into a workspace being
//! torn down.

use armada_core::config::ResolvedConfig;
use armada_core::ctx::{Clock, Fetch, Run};
use armada_core::envelope::{
    aggregate, CleanData, CleanDryRun, Envelope, Released, ResultRow, Unreclaimed,
};
use armada_core::error::{CharError, ConfigWhere, ErrClass, Status};
use armada_core::id::WorkspaceId;
use armada_core::lease::{is_cold, LeaseId, LeaseKind, Policy};
use armada_core::reap::{LabelledResource, LeftAlone, PathStat, ReapPlan, ReapTarget, Reported};
use armada_core::registry::{OwnedKind, WorkspaceRow};
use armada_core::scope::{self, Lens};
use armada_core::template::{self, Site, Vars};
use armada_manifest::{docker, fs};
use std::collections::BTreeMap;
use std::path::{Path, PathBuf};
use std::time::Duration;

use crate::app::{self, App, RELEASED_EARLY};
use crate::args::Common;
use crate::verbs::{load_foreign_config, Output};

/// What `clean` was asked to do beyond its scope.
#[derive(Debug, Clone, Copy)]
pub struct Filters {
    /// Also remove declared `owns.files`.
    pub artifacts: bool,
    /// Only workspaces whose directory no longer exists.
    pub orphaned: bool,
    /// Override the live-lease guard.
    pub force: bool,
    /// Rebuild an unreadable `char.db` from labels alone.
    pub force_rebuild: bool,
}

/// Run it.
pub fn run<R: Run, C: Clock, F: Fetch>(
    app: &mut App<R, C, F>,
    common: Common,
    filters: Filters,
) -> Result<Output, CharError> {
    let me = app.ctx.workspace.as_ref().map(|w| w.id.clone());
    let project = app.ctx.workspace.as_ref().and_then(|w| w.project.clone());

    let rows = app.db.workspaces()?;
    let selected: Vec<WorkspaceRow> = match (&me, common.lens) {
        // `--all` runs from outside any workspace, which is the case it is most
        // needed in: nothing else on the machine reaps orphaned ports and
        // containers, and a rule that made it resolve a local workspace first
        // would fail before it could do the one job only it does.
        (None, Lens::All) => rows.clone(),
        (None, _) => return Err(app.ctx.workspace().unwrap_err()),
        (Some(me), lens) => scope::select(&rows, lens, me, project.as_ref())
            .into_iter()
            .cloned()
            .collect(),
    };

    let selected: Vec<WorkspaceRow> = selected
        .into_iter()
        .filter(|row| !filters.orphaned || fs::stat(&row.path) == PathStat::Missing)
        .collect();

    if common.dry_run {
        return dry(app, &selected, filters).map(|e| Output::CleanDryRun(Box::new(e)));
    }

    // `--all` takes a **machine** lease, not a workspace one: it runs from
    // outside any workspace and the per-workspace run lease has nothing to
    // attach to, which would leave the most destructive operation in the tool
    // as the only mutating verb with no lock.
    let lease = match (common.lens, &me) {
        (Lens::All, _) => LeaseId::machine(),
        (_, Some(me)) => LeaseId::run(me.clone()),
        (_, None) => LeaseId::machine(),
    };

    // **The self-exemption below is only sound when the lease char just took is
    // *this workspace's* run lease.** Under `--all` the lease is the machine
    // one, whose row carries a NULL workspace, so a live lease on `me` belongs
    // to some other process — and exempting it would have `char clean --all`,
    // run from inside a worktree, tear down the `char init` running in it.
    let holds_my_run_lease = lease.kind == LeaseKind::Run && lease.workspace == me;

    let envelope = app::with_lease(app, lease, Policy::FailFast, None, |app| {
        clean_all(app, &selected, filters, me.as_ref(), holds_my_run_lease)
    })?;
    Ok(Output::Clean(Box::new(envelope)))
}

fn clean_all<R: Run, C: Clock, F: Fetch>(
    app: &mut App<R, C, F>,
    selected: &[WorkspaceRow],
    filters: Filters,
    me: Option<&WorkspaceId>,
    holds_my_run_lease: bool,
) -> Result<Envelope<CleanData>, CharError> {
    let mut reaped = app.reap()?;
    let mut results = Vec::new();
    let mut skipped = Vec::new();
    let mut unreclaimed = Vec::new();

    for row in selected {
        // Reaping may already have dropped this row — a workspace whose
        // directory vanished. Nothing left to do, and saying so beats
        // reporting a failure for work that succeeded.
        if reaped.workspaces.contains(&row.id) {
            results.push(ResultRow::new(row.id.to_string(), Status::Clean));
            continue;
        }

        // **`--all` skips any workspace holding a live lease, and reports what
        // it skipped.** On the five-concurrent-agents premise this project is
        // built around, the unguarded version stops four live stacks and
        // deletes their `node_modules` while their agents are mid-run.
        // Skipping is right rather than refusing: reclaiming disk from the
        // eleven idle workspaces is still the thing you asked for.
        let mine = holds_my_run_lease && Some(&row.id) == me;
        if !filters.force && !mine && holds_a_live_lease(app, &row.id)? {
            skipped.push(row.id.to_string());
            continue;
        }

        unreclaimed.extend(collect_unreclaimed(app, row)?);
        results.push(clean_one(app, row, filters, &mut reaped.skipped)?);
    }

    // **The one implementation of the precedence rule.** Counting rows here
    // would be a second one, and PLAN.md §3.1's stated reason for fixing the
    // order is that two implementations cannot disagree — so the class, the
    // `where` and the message all come from `envelope::aggregate`, and this
    // verb decides only the terminal *state*, which is a different axis.
    let error = aggregate(&results, "workspaces");

    // `PARTIAL` earns its place here: "three of five worked" and "nothing
    // worked" demand different actions and would otherwise both read `FAILED`.
    // Terminal state describes *what happened*; the error class states *why*,
    // and the exit code follows the class and never the state.
    let failed = results
        .iter()
        .filter(|row| row.status != Status::Clean)
        .count();
    let status = match (failed, results.len()) {
        (0, _) => Status::Clean,
        (failed, total) if failed == total => Status::Failed,
        _ => Status::Partial,
    };

    let data = CleanData {
        reaped,
        results,
        unreclaimed,
        skipped,
    };

    Ok(match error {
        None => Envelope::ok("clean", me.cloned(), Status::Clean, data),
        Some(error) => {
            let mut envelope = Envelope::failed("clean", me.cloned(), error, data);
            envelope.status = status;
            envelope
        }
    })
}

/// One workspace, in the fixed order.
fn clean_one<R: Run, C: Clock, F: Fetch>(
    app: &mut App<R, C, F>,
    row: &WorkspaceRow,
    filters: Filters,
    skipped: &mut Vec<String>,
) -> Result<ResultRow, CharError> {
    let mut released = Released::default();
    // **Everything that would not go is named, in one of two channels that must
    // not merge.** `refused` is a reclaim char attempted and could not
    // complete — a `docker rm` returning non-zero for a handle this workspace
    // owns, or a process group still alive after SIGKILL. That is a real leak,
    // so it fails this row. `skipped` is an enumeration char could not perform,
    // which is the same event `plan_reap` records there and proves nothing
    // about what this workspace still holds; it must not fail anything.
    let mut refused: Vec<String> = Vec::new();

    // 1. Only the *resource* leases go early, and only because holding a
    //    cpu-slot while tearing down blocks other workspaces for no reason.
    for lease in app.db.leases()? {
        if lease.workspace.as_ref() == Some(&row.id) && RELEASED_EARLY.contains(&lease.kind) {
            app.db.release_lease(lease.kind, &lease.key)?;
        }
    }

    // 2. Processes.
    let stopped = app.stop_owned_processes(&row.id)?;
    released.processes = stopped.stopped;
    for pgid in &stopped.survived {
        refused.push(format!(
            "process group {pgid} was still alive after SIGKILL"
        ));
    }

    // 3. Docker, in dependency order: a network with a container attached will
    //    not go.
    // An unreachable daemon is the *skipped* category too, and recording it is
    // what keeps this honest: with Docker closed every count below stays zero
    // and nothing is refused, so a silent probe would let the row report
    // `CLEAN` while the workspace's labelled containers, networks and volumes
    // survive — and their `char.workspace_path` outlives the row, so the next
    // reap classifies them as belonging to a live workspace and only ever
    // reports them. That is the founding bug of this project, reached through a
    // run that said it succeeded.
    let daemon = match app.docker_ready() {
        Ok(()) => true,
        Err(error) => {
            skipped.push(app::skipped_enumeration(&error));
            false
        }
    };
    if daemon {
        for kind in docker::Kind::ALL {
            // A failed enumeration is the *skipped* category, treated exactly
            // as `plan_reap` treats it: a concurrent `clean` in another
            // workspace removing a container between the `ls` and the
            // `inspect` makes `docker inspect` exit non-zero, and the object
            // that vanished was that workspace's and is already gone. Nothing
            // of *this* workspace failed to release, so a benign race under the
            // five concurrent worktrees this project assumes must not make the
            // verb exit 1.
            let found = match app.docker_list_labelled(kind) {
                Ok(found) => found,
                Err(error) => {
                    skipped.push(app::skipped_enumeration(&error));
                    continue;
                }
            };
            // **Filters on both labels, not the id alone.** `workspace_id` is
            // 32 bits, and every `owns:` selector is id-only — so a collision
            // would have `char clean` in one workspace destroy another's live
            // containers, the single thing the flat-siblings model exists to
            // prevent.
            let mine: Vec<String> = found
                .iter()
                .filter(|resource| {
                    resource.workspace == row.id
                        && resource.workspace_path == row.path
                        && resource.namespace.as_deref() == Some(app.namespace.as_str())
                })
                .map(|resource| resource.reference.clone())
                .collect();

            // The other category: a handle this workspace owns that would not
            // go is a leak char is about to stop tracking, so it fails the row.
            let removed = app.docker_remove(kind, &mine);
            count_removed(kind, &removed, &mut released, &mut refused);
        }
    }

    // A `commands:` entry's `owns:` is a **selector, not a record** — it runs
    // ad hoc, so there is no "while it was up" window to record against, and
    // `clean` evaluates the declaration against docker instead. This is a
    // distinct code path from reading the `owned` table, and it is the one the
    // done-when exercises.
    if daemon {
        if let Some(config) = load_foreign_config(&row.path, &app.machine) {
            released_from_selectors(app, row, &config, &mut released, &mut refused, skipped)?;
        }
    }

    // 4 and 5. The block goes back with the row, in one transaction — two
    // statements could leave a workspace row with no block, which is a shape
    // nothing else in the code knows how to read. The run lease this
    // invocation is standing on survives it, and is released last.
    app.forget_workspace(&row.id)?;
    released.port_block = true;

    // 6. `.char/`. **`clean` releases resources; it does not undo
    //    installation** — `node_modules` and a populated `.venv` survive by
    //    design unless `--artifacts` is passed.
    if fs::stat(&row.path) != PathStat::Missing {
        let _ = fs::remove_char_dir(&row.path);
    }

    if filters.artifacts {
        released.files = remove_artifacts(app, row)?;
    }

    let status = if refused.is_empty() {
        Status::Clean
    } else {
        Status::Failed
    };
    let mut result = ResultRow::new(row.id.to_string(), status);
    result.path = Some(row.path.display().to_string());
    result.released = Some(released);
    if !refused.is_empty() {
        result.error = Some(CharError {
            class: ErrClass::ToolFailed,
            r#where: row.id.to_string(),
            message: refused.join("; "),
            next_action: Some(
                "remove what is named above by hand, then re-run `char clean`".to_string(),
            ),
        });
    }
    Ok(result)
}

/// Tally one kind's removals, and name the handles that would not go.
fn count_removed(
    kind: docker::Kind,
    removed: &[(String, Option<CharError>)],
    released: &mut Released,
    refused: &mut Vec<String>,
) {
    let mut count = 0;
    for (reference, error) in removed {
        match error {
            None => count += 1,
            Some(error) => refused.push(format!("{reference}: {}", error.message)),
        }
    }
    match kind {
        docker::Kind::Container => released.containers += count,
        docker::Kind::Network => released.networks += count,
        docker::Kind::Volume => released.volumes += count,
        docker::Kind::Image => released.images += count,
    }
}

/// Evaluate declared `owns:` selectors against docker.
fn released_from_selectors<R: Run, C: Clock, F: Fetch>(
    app: &mut App<R, C, F>,
    row: &WorkspaceRow,
    config: &ResolvedConfig,
    released: &mut Released,
    refused: &mut Vec<String>,
    skipped: &mut Vec<String>,
) -> Result<(), CharError> {
    let ports =
        armada_core::ports::assign_ports(config, row.ports, "armada.yml").unwrap_or_default();

    // Resolved up front, so the docker calls below can take the whole `App`:
    // they renew the heartbeat, and a template borrowing the inherited
    // environment would pin it immutably for the length of the loop.
    let resolved: Vec<(docker::Kind, String)> = {
        let vars = Vars::new(row.id.as_str(), &ports, &app.inherited);
        let at = ConfigWhere::Path {
            file: "armada.yml".to_string(),
            path: "owns".to_string(),
        };

        let mut selectors: Vec<(docker::Kind, &String)> = Vec::new();
        for entry in config.commands.values() {
            for (kind, selector) in [
                (docker::Kind::Container, &entry.owns_containers),
                (docker::Kind::Network, &entry.owns_networks),
                (docker::Kind::Image, &entry.owns_images),
            ] {
                if let Some(selector) = selector {
                    selectors.push((kind, selector));
                }
            }
        }
        for component in config.components.values() {
            if let Some(run) = &component.run {
                let common = run.common();
                for (kind, selector) in [
                    (docker::Kind::Container, &common.owns_containers),
                    (docker::Kind::Network, &common.owns_networks),
                    (docker::Kind::Image, &common.owns_images),
                ] {
                    if let Some(selector) = selector {
                        selectors.push((kind, selector));
                    }
                }
            }
        }

        let mut resolved = Vec::with_capacity(selectors.len());
        for (kind, selector) in selectors {
            resolved.push((
                kind,
                template::substitute(selector, &vars, Site::Argv, &at)?,
            ));
        }
        resolved
    };

    for (kind, selector) in resolved {
        // Enumeration, so `skipped`; the removal below is a reclaim, so
        // `refused`. See the two categories in `clean_one`.
        let matched = match app.docker_list_by_selector(kind, &selector) {
            Ok(matched) => matched,
            Err(error) => {
                skipped.push(app::skipped_enumeration(&error));
                continue;
            }
        };
        let removed = app.docker_remove(kind, &matched);
        count_removed(kind, &removed, released, refused);
    }
    Ok(())
}

/// `--artifacts`: the declared `owns.files`, and only those.
///
/// **char never guesses which files are artifacts.** Inferring `node_modules`,
/// `.venv` or `.next` from a repo scan is a stack-detection engine, which the
/// plan rules out. They are declared, or they are not char's.
fn remove_artifacts<R: Run, C: Clock, F: Fetch>(
    app: &mut App<R, C, F>,
    row: &WorkspaceRow,
) -> Result<usize, CharError> {
    // A no-op under `--orphaned`, where the directory and its files are already
    // gone with the workspace.
    if fs::stat(&row.path) == PathStat::Missing {
        return Ok(0);
    }
    let Some(config) = load_foreign_config(&row.path, &app.machine) else {
        return Ok(0);
    };
    let ports =
        armada_core::ports::assign_ports(&config, row.ports, "armada.yml").unwrap_or_default();
    let vars = Vars::new(row.id.as_str(), &ports, &app.inherited);
    let at = ConfigWhere::Path {
        file: "armada.yml".to_string(),
        path: "owns.files".to_string(),
    };

    let mut removed = 0;
    for path in declared_files(&config) {
        let resolved = template::substitute(path, &vars, Site::Argv, &at)?;
        if fs::remove_owned_file(&row.path, &resolved)? {
            removed += 1;
        }
    }
    Ok(removed)
}

/// Every declared `owns.files`, from all three places a repo may declare one.
///
/// **One collection, shared by the preview and the deletion.** `--dry-run` is
/// the safety mechanism for `--artifacts`, so a preview built from a shorter
/// list than the real pass deletes from is worse than no preview: it reads as a
/// complete answer and is not one.
fn declared_files(config: &ResolvedConfig) -> Vec<&String> {
    let mut declared: Vec<&String> = Vec::new();
    for component in config.components.values() {
        declared.extend(component.owns_files.iter());
        if let Some(run) = &component.run {
            declared.extend(run.common().owns_files.iter());
        }
    }
    for entry in config.commands.values() {
        declared.extend(entry.owns_files.iter());
    }
    declared
}

/// The declared external resources char is about to stop knowing about.
///
/// Collected **before** the rows are deleted, because after that there is
/// nothing left to report — and reporting is the entire mechanism: char records
/// these and never runs them.
fn collect_unreclaimed<R: Run, C: Clock, F: Fetch>(
    app: &App<R, C, F>,
    row: &WorkspaceRow,
) -> Result<Vec<Unreclaimed>, CharError> {
    Ok(app
        .db
        .owned(Some(&row.id))?
        .into_iter()
        .filter(|owned| owned.kind == OwnedKind::Release)
        .map(|owned| Unreclaimed {
            workspace: row.id.clone(),
            command: owned.reference,
            workspace_exists: fs::stat(&row.path) != PathStat::Missing,
        })
        .collect())
}

fn holds_a_live_lease<R: Run, C: Clock, F: Fetch>(
    app: &App<R, C, F>,
    workspace: &WorkspaceId,
) -> Result<bool, CharError> {
    let now = app.ctx.now.mono();
    Ok(app.db.leases()?.iter().any(|lease| {
        lease.workspace.as_ref() == Some(workspace)
            && lease.kind != LeaseKind::Machine
            && !is_cold(lease.heartbeat_mono, now, lease.boot_id == app.boot_id)
    }))
}

/// `--dry-run`. **char computes this from its own state and needs no help from
/// the repo:** it knows what it claimed, what it labelled, and what the current
/// scope selects.
fn dry<R: Run, C: Clock, F: Fetch>(
    app: &mut App<R, C, F>,
    selected: &[WorkspaceRow],
    filters: Filters,
) -> Result<Envelope<CleanDryRun>, CharError> {
    let plan = app.plan_reap()?;
    let mut preview = CleanDryRun {
        would_release: selected
            .iter()
            .map(|row| format!("ports {}-{} ({})", row.ports.from, row.ports.to, row.id))
            .collect(),
        would_remove: plan
            .resources
            .iter()
            .map(|target| format!("{} {}", target.kind, target.reference))
            .collect(),
        would_delete: Vec::new(),
        would_report: Vec::new(),
    };

    for row in selected {
        for owned in app.db.owned(Some(&row.id))? {
            match owned.kind {
                OwnedKind::Release => preview.would_report.push(owned.reference),
                OwnedKind::Container
                | OwnedKind::Network
                | OwnedKind::Volume
                | OwnedKind::Image => preview
                    .would_remove
                    .push(format!("{} {}", owned.kind, owned.reference)),
                OwnedKind::Pgid => preview
                    .would_remove
                    .push(format!("process group {}", owned.reference)),
            }
        }
        if filters.artifacts {
            preview.would_delete.extend(would_delete(app, row));
        }
    }

    Ok(Envelope::ok(
        "clean",
        app.ctx.workspace.as_ref().map(|w| w.id.clone()),
        Status::Clean,
        preview,
    ))
}

/// What `--dry-run --artifacts` would delete — the same list
/// [`remove_artifacts`] works from, resolved the same way.
fn would_delete<R: Run, C: Clock, F: Fetch>(app: &App<R, C, F>, row: &WorkspaceRow) -> Vec<String> {
    let Some(config) = load_foreign_config(&row.path, &app.machine) else {
        return Vec::new();
    };
    let ports: BTreeMap<String, u16> =
        armada_core::ports::assign_ports(&config, row.ports, "armada.yml").unwrap_or_default();
    let vars = Vars::new(row.id.as_str(), &ports, &app.inherited);
    let at = ConfigWhere::Path {
        file: "armada.yml".to_string(),
        path: "owns.files".to_string(),
    };
    declared_files(&config)
        .into_iter()
        .filter_map(|path| template::substitute(path, &vars, Site::Argv, &at).ok())
        .collect()
}

/// `char clean --orphaned --force-rebuild` — the way out of a `char.db` char
/// cannot read (PLAN.md §4.3).
///
/// **The recovery path must not need the thing that is broken**, which is why
/// this does not go through [`crate::app::App`] at all: building one opens the
/// database, so a verb that reached this function through the ordinary path
/// would already have failed. It runs from the entrypoint, before anything has
/// been opened.
///
/// It ignores the existing database, enumerates by **label alone** —
/// `char.workspace_path` is a real path and `stat` still works — reaps what is
/// unambiguously dead, and writes a fresh database. **It is the one operation
/// that trusts labels over rows, which is why it is explicit rather than
/// automatic.**
///
/// Two deliberate departures from the ordinary reap, both stated because they
/// are the cost of the recovery:
///
/// - **The namespace filter does not apply**, because the namespace is one of
///   the things that was lost — and it stays inapplicable on the run where
///   `meta` was still legible, so the recovery behaves the same way whether or
///   not the peek happened to succeed. So this removes a labelled resource
///   whose path is `ENOENT` even if another installation stamped it. That is
///   exactly the cross-namespace removal PLAN.md §2.3.1 forbids for the
///   automatic passes — permitted here only because the caller asked for it by
///   name, with two flags, after their database stopped working. Which of the
///   two cases the run was in is stated in `reaped.skipped`, because that is
///   the half the caller reads.
/// - **The unreadable file is moved aside, not deleted.** A recovery that
///   destroys the evidence of what it recovered from cannot be diagnosed
///   afterwards, and the file is the only artifact that could say what went
///   wrong.
///
/// `--dry-run` previews it and changes nothing — no file is moved, no
/// replacement is created and no removal is run. This is the most destructive
/// operation in the tool, so it is the one a preview matters most for;
/// enumerating is still permitted, because listing and `stat`-ing change
/// nothing.
pub fn rebuild<R: Run, C: Clock>(
    run: &R,
    now: &C,
    home: &Path,
    dry_run: bool,
) -> Result<Output, CharError> {
    let char_home = armada_manifest::machine::char_home(home);
    let db_path = char_home.join("char.db");

    // Best-effort, and the recovery does not depend on it: a database can be
    // unreadable in ways that still leave `meta` legible, and carrying the old
    // namespace across keeps every resource already stamped with it reapable.
    let recovered = armada_manifest::db::Db::peek_namespace(&db_path);

    // Read before anything is opened, and it opens nothing itself: the deadline
    // has to be available on a path whose whole premise is that the database is
    // not.
    let machine = armada_manifest::machine::MachineConfig::read(&char_home)?;
    let timeout = machine.docker_deadline();
    let cwd = home.to_path_buf();

    // **Every sidecar is moved with the file, and the presence of `char.db` is
    // not what decides it.** An interrupted write can leave `char.db-wal` and
    // `char.db-shm` behind with no `char.db` at all (PLAN.md §4.3), and a fresh
    // database opened alongside a foreign WAL is a worse state than the one
    // being recovered from.
    let sidecars: Vec<PathBuf> = ["", "-wal", "-shm"]
        .iter()
        .map(|suffix| char_home.join(format!("char.db{suffix}")))
        .filter(|path| path.exists())
        .collect();

    if dry_run {
        return Ok(Output::CleanDryRun(Box::new(rebuild_preview(
            run,
            &cwd,
            timeout,
            &db_path,
            &sidecars,
            recovered.as_deref(),
        ))));
    }

    let stamp = now.wall_rfc3339().replace(':', "-");
    let mut moved_aside = Vec::new();
    for from in &sidecars {
        let Some(name) = from.file_name().and_then(|name| name.to_str()) else {
            continue;
        };
        let to = char_home.join(format!("{name}.unreadable-{stamp}"));
        if std::fs::rename(from, &to).is_ok() {
            moved_aside.push(to.display().to_string());
        }
    }

    let mut db = armada_manifest::db::Db::open(&char_home)?;
    if let Some(namespace) = &recovered {
        db.adopt_namespace(namespace)?;
    }

    let mut reaped = ReapPlan::default();
    for note in moved_aside {
        reaped.skipped.push(format!("moved aside: {note}"));
    }
    reaped.skipped.push(namespace_note(recovered.as_deref()));

    // Two channels, exactly as `clean_one` keeps them: an enumeration char
    // could not perform is `skipped` and proves nothing, while a removal char
    // attempted and could not complete is a leak on a workspace that is
    // provably gone — so it fails a row rather than being reported as a
    // resource that was deliberately left alone.
    let found = survey(run, &cwd, timeout, &mut reaped.skipped);
    let mut refused: BTreeMap<String, Vec<String>> = BTreeMap::new();

    for (kind, resource, stat) in found {
        // By label alone, and only on `ENOENT`. Every other errno still means
        // "adopt or report, never remove" — that rule is not what broke.
        let reason = match stat {
            PathStat::Present => LeftAlone::WorkspaceLive,
            PathStat::Unreadable(_) => LeftAlone::PathUnreadable,
            PathStat::Missing => {
                let removed = docker::remove(
                    run,
                    &cwd,
                    timeout,
                    kind,
                    std::slice::from_ref(&resource.reference),
                    &mut || {},
                );
                let failures: Vec<String> = removed
                    .iter()
                    .filter_map(|(reference, error)| {
                        error
                            .as_ref()
                            .map(|e| format!("{reference}: {}", e.message))
                    })
                    .collect();
                if failures.is_empty() {
                    reaped.resources.push(ReapTarget {
                        kind: resource.kind,
                        reference: resource.reference,
                        workspace: resource.workspace,
                    });
                } else {
                    refused
                        .entry(resource.workspace.to_string())
                        .or_default()
                        .extend(failures);
                }
                continue;
            }
        };
        reaped.reported.push(Reported {
            kind: resource.kind,
            reference: resource.reference,
            workspace: resource.workspace,
            reason,
        });
    }

    let results: Vec<ResultRow> = refused
        .into_iter()
        .map(|(workspace, refused)| {
            let mut row = ResultRow::new(workspace.clone(), Status::Failed);
            row.error = Some(CharError {
                class: ErrClass::ToolFailed,
                r#where: workspace,
                message: refused.join("; "),
                next_action: Some(
                    "remove what is named above by hand, then re-run `char clean --all \
                     --orphaned` — the store is readable again, so the ordinary reap is \
                     enough and re-running the rebuild would only set another database aside"
                        .to_string(),
                ),
            });
            row
        })
        .collect();

    let error = aggregate(&results, "workspaces");
    let data = CleanData {
        reaped,
        results,
        unreclaimed: Vec::new(),
        skipped: Vec::new(),
    };

    Ok(Output::Clean(Box::new(match error {
        None => Envelope::ok("clean", None, Status::Clean, data),
        Some(error) => Envelope::failed("clean", None, error, data),
    })))
}

/// What `--force-rebuild` would do, with nothing done.
///
/// **The replacement database is stated as plainly as the move-aside**, because
/// it is the more consequential half and the one a caller can misread: "moved
/// aside" on its own reads as *the file is set safe and the store is otherwise
/// preserved*, when in fact every workspace row, port block and lease record on
/// the machine goes with it. Whether the old namespace comes across decides
/// whether resources already stamped stay reapable, so the preview says which
/// of the two cases this run would be.
fn rebuild_preview<R: Run>(
    run: &R,
    cwd: &Path,
    timeout: Duration,
    db_path: &Path,
    sidecars: &[PathBuf],
    recovered: Option<&str>,
) -> Envelope<CleanDryRun> {
    let mut would_release: Vec<String> = sidecars
        .iter()
        .map(|path| format!("move aside {}", path.display()))
        .collect();
    would_release.push(match recovered {
        Some(namespace) => format!(
            "create a fresh {} in its place, carrying the recovered namespace {namespace} so \
             resources already stamped with it stay reapable — every workspace row, port \
             block and lease record on this machine goes with the old file",
            db_path.display()
        ),
        None => format!(
            "create a fresh {} in its place with a new namespace, since the old one could not \
             be read — every workspace row, port block and lease record on this machine goes \
             with the old file, and resources stamped with the old namespace are no longer \
             matchable by it",
            db_path.display()
        ),
    });

    let mut preview = CleanDryRun {
        would_release,
        ..Default::default()
    };

    let mut notes = vec![namespace_note(recovered)];
    for (_, resource, stat) in survey(run, cwd, timeout, &mut notes) {
        let named = format!("{} {}", resource.kind, resource.reference);
        match stat {
            PathStat::Missing => preview.would_remove.push(named),
            PathStat::Present => preview
                .would_report
                .push(format!("{named}: its workspace is alive")),
            PathStat::Unreadable(why) => preview.would_report.push(format!(
                "{named}: its path could not be read ({why}), which is not the same as gone"
            )),
        }
    }
    preview.would_report.extend(notes);

    Envelope::ok("clean", None, Status::Clean, preview)
}

/// Every labelled resource on this daemon, with the `stat` of the path it
/// carries.
///
/// Listing and `stat`-ing change nothing, which is what lets the preview share
/// this with the pass that removes — a preview built from a different
/// enumeration than the real pass acts on reads as a complete answer and is not
/// one.
fn survey<R: Run>(
    run: &R,
    cwd: &Path,
    timeout: Duration,
    skipped: &mut Vec<String>,
) -> Vec<(docker::Kind, LabelledResource, PathStat)> {
    // No lease to renew: the rebuild path holds none, because the store the
    // leases live in is the thing that is broken.
    let tick = &mut || {};
    let mut found = Vec::new();

    match docker::daemon_ready(run, cwd, timeout, tick) {
        Ok(()) => {
            for kind in docker::Kind::ALL {
                match docker::list_labelled(run, cwd, timeout, kind, tick) {
                    Ok(listed) => found.extend(listed.into_iter().map(|resource| {
                        let stat = fs::stat(&resource.workspace_path);
                        (kind, resource, stat)
                    })),
                    Err(error) => skipped.push(format!("enumeration: {}", error.message)),
                }
            }
        }
        Err(error) => skipped.push(format!("labelled resources: {}", error.message)),
    }
    found
}

/// What the caller has to know about the scope and the namespace on this path,
/// stated as it actually behaves.
///
/// **The scope is stated first, and unconditionally.** `PLAN.md` §4.3 spells
/// the recovery as `char clean --orphaned --force-rebuild`, so that invocation
/// is accepted as written — which means a caller can reach this pass from a
/// command line that reads workspace-scoped and get machine-scoped,
/// cross-namespace work. Telling them what it did is the honest half of
/// accepting the documented form; refusing it was not.
///
/// **The removal is by label alone in both namespace cases**, so a note
/// promising that another installation's resources are preserved would be false
/// — and the note is the half the caller actually reads.
fn namespace_note(recovered: Option<&str>) -> String {
    let scope = concat!(
        "this pass is machine-scoped whichever flags reached it: it enumerates every ",
        "labelled resource on this daemon and removes on ENOENT across namespaces, so it ",
        "may remove a resource another char installation stamped",
    );
    let namespace = match recovered {
        Some(_) => concat!(
            "the previous namespace was recovered and carried across, so resources already ",
            "stamped with it stay reapable — but the enumeration above is still by label ",
            "alone and the namespace filter does not bound it",
        ),
        None => concat!(
            "the previous namespace could not be read, so char cannot tell its own dead ",
            "resources from another installation's; this path removes on ENOENT regardless",
        ),
    };
    format!("{scope}; {namespace}")
}
