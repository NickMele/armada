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

use charkit_adapters::{docker, fs};
use charkit_core::config::ResolvedConfig;
use charkit_core::ctx::{Clock, Fetch, Run};
use charkit_core::envelope::{CleanData, CleanDryRun, Envelope, Released, ResultRow, Unreclaimed};
use charkit_core::error::{CharError, ConfigWhere, ErrClass, Status};
use charkit_core::id::WorkspaceId;
use charkit_core::lease::{is_cold, LeaseId, LeaseKind, Policy};
use charkit_core::reap::PathStat;
use charkit_core::registry::{OwnedKind, WorkspaceRow};
use charkit_core::scope::{self, Lens};
use charkit_core::template::{self, Site, Vars};
use std::collections::BTreeMap;
use std::path::Path;

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
    if filters.force_rebuild {
        return Err(CharError {
            class: ErrClass::BadInvocation,
            r#where: "--force-rebuild".to_string(),
            message: "rebuilding char.db from labels alone is not built yet".to_string(),
            next_action: Some(
                "remove ~/.char/char.db by hand; `char init` recreates it and reaps by label"
                    .to_string(),
            ),
        });
    }

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

    let envelope = app::with_lease(app, lease, Policy::FailFast, None, |app| {
        clean_all(app, &selected, filters, me.as_ref())
    })?;
    Ok(Output::Clean(Box::new(envelope)))
}

fn clean_all<R: Run, C: Clock, F: Fetch>(
    app: &mut App<R, C, F>,
    selected: &[WorkspaceRow],
    filters: Filters,
    me: Option<&WorkspaceId>,
) -> Result<Envelope<CleanData>, CharError> {
    let reaped = app.reap()?;
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
        if !filters.force && holds_a_live_lease(app, &row.id)? && Some(&row.id) != me {
            skipped.push(row.id.to_string());
            continue;
        }

        unreclaimed.extend(collect_unreclaimed(app, row)?);
        results.push(clean_one(app, row, filters)?);
    }

    let failed = results
        .iter()
        .filter(|r| r.status == Status::Failed)
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

    Ok(match status {
        Status::Clean => Envelope::ok("clean", me.cloned(), Status::Clean, data),
        other => {
            let mut envelope = Envelope::failed(
                "clean",
                me.cloned(),
                CharError {
                    class: ErrClass::ToolFailed,
                    r#where: "clean".to_string(),
                    message: format!(
                        "{failed} of {} workspaces did not release",
                        data.results.len()
                    ),
                    next_action: None,
                },
                data,
            );
            // `PARTIAL` earns its place here: "three of five worked" and
            // "nothing worked" demand different actions and would otherwise
            // both read `FAILED`.
            envelope.status = other;
            envelope
        }
    })
}

/// One workspace, in the fixed order.
fn clean_one<R: Run, C: Clock, F: Fetch>(
    app: &mut App<R, C, F>,
    row: &WorkspaceRow,
    filters: Filters,
) -> Result<ResultRow, CharError> {
    let mut released = Released::default();

    // 1. Only the *resource* leases go early, and only because holding a
    //    cpu-slot while tearing down blocks other workspaces for no reason.
    for lease in app.db.leases()? {
        if lease.workspace.as_ref() == Some(&row.id) && RELEASED_EARLY.contains(&lease.kind) {
            app.db.release_lease(lease.kind, &lease.key)?;
        }
    }

    // 2. Processes.
    released.processes = app.stop_owned_processes(&row.id)?;

    // 3. Docker, in dependency order: a network with a container attached will
    //    not go.
    let timeout = app.machine.docker_deadline();
    let daemon = docker::daemon_ready(&app.ctx.run, &app.cwd(), timeout).is_ok();
    if daemon {
        for kind in docker::Kind::ALL {
            let found = docker::list_labelled(&app.ctx.run, &app.cwd(), timeout, kind)?;
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

            let removed = docker::remove(&app.ctx.run, &app.cwd(), timeout, kind, &mine);
            let count = removed.iter().filter(|(_, error)| error.is_none()).count();
            match kind {
                docker::Kind::Container => released.containers += count,
                docker::Kind::Network => released.networks += count,
                docker::Kind::Volume => released.volumes += count,
                docker::Kind::Image => released.images += count,
            }
        }
    }

    // A `commands:` entry's `owns:` is a **selector, not a record** — it runs
    // ad hoc, so there is no "while it was up" window to record against, and
    // `clean` evaluates the declaration against docker instead. This is a
    // distinct code path from reading the `owned` table, and it is the one the
    // done-when exercises.
    if daemon {
        if let Some(config) = load_foreign_config(&row.path, &app.machine) {
            released_from_selectors(app, row, &config, &mut released)?;
        }
    }

    // 4 and 5. The block goes back with the row, in one transaction — two
    // statements could leave a workspace row with no block, which is a shape
    // nothing else in the code knows how to read.
    app.db.release_workspace(&row.id)?;
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

    let mut result = ResultRow::new(row.id.to_string(), Status::Clean);
    result.path = Some(row.path.display().to_string());
    result.released = Some(released);
    Ok(result)
}

/// Evaluate declared `owns:` selectors against docker.
fn released_from_selectors<R: Run, C: Clock, F: Fetch>(
    app: &mut App<R, C, F>,
    row: &WorkspaceRow,
    config: &ResolvedConfig,
    released: &mut Released,
) -> Result<(), CharError> {
    let ports =
        charkit_core::ports::assign_ports(config, row.ports, "char.yml").unwrap_or_default();
    let vars = Vars::new(row.id.as_str(), &ports, &app.inherited);
    let timeout = app.machine.docker_deadline();

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

    for (kind, selector) in selectors {
        let at = ConfigWhere::Path {
            file: "char.yml".to_string(),
            path: "owns".to_string(),
        };
        let resolved = template::substitute(selector, &vars, Site::Argv, &at)?;
        let matched = docker::list_by_selector(&app.ctx.run, &app.cwd(), timeout, kind, &resolved)?;
        let removed = docker::remove(&app.ctx.run, &app.cwd(), timeout, kind, &matched);
        let count = removed.iter().filter(|(_, error)| error.is_none()).count();
        match kind {
            docker::Kind::Container => released.containers += count,
            docker::Kind::Network => released.networks += count,
            docker::Kind::Volume => released.volumes += count,
            docker::Kind::Image => released.images += count,
        }
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
        charkit_core::ports::assign_ports(&config, row.ports, "char.yml").unwrap_or_default();
    let vars = Vars::new(row.id.as_str(), &ports, &app.inherited);
    let at = ConfigWhere::Path {
        file: "char.yml".to_string(),
        path: "owns.files".to_string(),
    };

    let mut removed = 0;
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

    for path in declared {
        let resolved = template::substitute(path, &vars, Site::Argv, &at)?;
        if fs::remove_owned_file(&row.path, &resolved)? {
            removed += 1;
        }
    }
    Ok(removed)
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
            preview
                .would_delete
                .extend(declared_files(&row.path, app, row));
        }
    }

    Ok(Envelope::ok(
        "clean",
        app.ctx.workspace.as_ref().map(|w| w.id.clone()),
        Status::Clean,
        preview,
    ))
}

fn declared_files<R: Run, C: Clock, F: Fetch>(
    root: &Path,
    app: &App<R, C, F>,
    row: &WorkspaceRow,
) -> Vec<String> {
    let Some(config) = load_foreign_config(root, &app.machine) else {
        return Vec::new();
    };
    let ports: BTreeMap<String, u16> =
        charkit_core::ports::assign_ports(&config, row.ports, "char.yml").unwrap_or_default();
    let vars = Vars::new(row.id.as_str(), &ports, &app.inherited);
    let at = ConfigWhere::Path {
        file: "char.yml".to_string(),
        path: "owns.files".to_string(),
    };
    let mut files = Vec::new();
    for component in config.components.values() {
        for path in &component.owns_files {
            if let Ok(resolved) = template::substitute(path, &vars, Site::Argv, &at) {
                files.push(resolved);
            }
        }
    }
    files
}
