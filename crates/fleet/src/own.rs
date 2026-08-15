//! Recording a Drone's process group where **Manifest's reaper will find it**.
//!
//! A Drone is a long-lived detached process group in a worktree that is a
//! workspace, which is precisely what `armada manifest up` already records for a
//! `command` service. So Fleet writes the same row, and gets three things it
//! would otherwise have had to build:
//!
//! | Already true of a service's group | Now true of a Drone |
//! |---|---|
//! | `armada manifest clean` kills it and drops the row | `armada fleet kill` runs `clean`, so the Drone dies with the rest of the workspace |
//! | `armada manifest clean --all --orphaned` reclaims it when the directory is gone | **an orphaned Drone is reaped by the pass that reaps an orphaned service** |
//! | nothing is killed that Armada cannot prove is its own | the same two stamps, checked by the same function |
//!
//! **The row carries no component**, and that is deliberate rather than lazy.
//! `armada manifest down <service>` acts only on rows whose component it names,
//! and its own source says a row without one "is left alone here and reclaimed
//! by `clean`, which takes everything". A Drone is not a service anybody asked
//! `down` for, and it must not be stopped by a verb aimed at the repository's
//! own processes — but it must still be reclaimed by the verb that reclaims
//! everything.
//!
//! **Manifest is not being told about a Job.** It is being told about a process
//! group in a workspace, which is a repository fact. Nothing here hands it a Job
//! id, a uuid or a model name, so the rule that keeps Manifest agent-agnostic
//! holds (`ARCHITECTURE.md` §1.9).

use armada_core::ctx::Run;
use armada_core::error::ArmadaError;
use armada_core::fleet::job::Handle;
use armada_core::registry::{OwnedKind, OwnedRow};
use armada_manifest::db::Db;
use armada_manifest::discovery;
use std::path::Path;

/// Record a Drone's group against the workspace its worktree is.
///
/// **A worktree that does not resolve is reported rather than fatal.** The Job's
/// own record already carries the handle, so `armada fleet kill` can still stop
/// the Drone; what is lost is only the machine-global backstop, and failing the
/// spawn over it would trade a live Job for a bookkeeping row.
pub fn record_drone(
    run: &impl Run,
    armada_home: &Path,
    worktree: &Path,
    handle: &Handle,
) -> Result<bool, ArmadaError> {
    let Ok(workspace) = discovery::resolve(run, worktree) else {
        return Ok(false);
    };
    let mut db = Db::open(armada_home)?;
    db.record_owned(&OwnedRow {
        workspace: workspace.id,
        kind: OwnedKind::Pgid,
        reference: handle.pgid.to_string(),
        // **Both stamps, or the row is a permanent phantom.** Without a start
        // time a recycled pid is indistinguishable from a live Drone, so `clean`
        // could only ever report it — forever, across reboots (PLAN.md §2.3.1).
        boot_id: Some(handle.boot_id.clone()),
        pid_started_at: handle.started_at.clone(),
        component: None,
    })?;
    Ok(true)
}

/// Forget a Drone's group, once it is gone.
///
/// Called when a Drone is replaced by a resumed one, so a Job that has been
/// answered five times does not accumulate five rows naming pids that no longer
/// exist. **Best-effort**: a row left behind is reaped by the cold-pid rule the
/// next time anything runs, which is why this returning an error would be worse
/// than it doing nothing.
pub fn forget_drone(run: &impl Run, armada_home: &Path, worktree: &Path, pgid: i32) {
    let Ok(workspace) = discovery::resolve(run, worktree) else {
        return;
    };
    let Ok(mut db) = Db::open(armada_home) else {
        return;
    };
    let _ = db.delete_owned(&workspace.id, OwnedKind::Pgid, &pgid.to_string());
}
