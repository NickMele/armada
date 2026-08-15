//! `.armada/run/<run-id>/` on disk: creating it, writing the record, and reaping
//! the ones that have aged out.
//!
//! The decisions are all in [`armada_core::run`] — the id format, which
//! directories to reap, what a log is called. This module is the half that
//! touches the filesystem, which `ARCHITECTURE.md` §1.1 deliberately does not
//! fake: the semantics that matter here are `ENOENT` versus `EACCES` and an
//! unlinked inode that still accepts writes, and those are exactly the ones a
//! fake gets wrong.

use armada_core::error::{ArmadaError, ErrClass};
use armada_core::run::{log_name, runs_to_reap, Detached, RunId, RunRecord};
use armada_core::schedule::CheckId;
use std::io;
use std::path::{Path, PathBuf};

/// `.armada/run/`, given a workspace root.
pub fn runs_dir(root: &Path) -> PathBuf {
    root.join(".armada").join("run")
}

/// `.armada/run/<run-id>/`.
pub fn run_dir(root: &Path, run: &RunId) -> PathBuf {
    runs_dir(root).join(run.as_str())
}

/// Where one check's output goes.
pub fn log_path(root: &Path, run: &RunId, check: &CheckId) -> PathBuf {
    run_dir(root, run).join("logs").join(log_name(check))
}

/// The same path a `results[]` row reports, which is workspace-relative.
///
/// **Workspace-relative because the primary consumer is an agent reading output
/// and editing files** (PLAN.md §4.1). An absolute path also puts the machine's
/// home directory into `--json`, which the privacy rules exist to keep out of
/// anything Armada writes down.
pub fn log_reference(run: &RunId, check: &CheckId) -> String {
    format!(".armada/run/{}/logs/{}", run, log_name(check))
}

/// Create `.armada/run/<run-id>/logs/`.
pub fn prepare(root: &Path, run: &RunId) -> Result<PathBuf, ArmadaError> {
    let dir = run_dir(root, run);
    let logs = dir.join("logs");
    std::fs::create_dir_all(&logs).map_err(|e| environment(&logs, "create", &e))?;
    Ok(dir)
}

/// Every run directory this workspace has kept.
///
/// **A name that is not a run id is ignored rather than reported.** `.armada/` is
/// an ordinary directory on a developer's machine: an editor's swap file, a
/// `.DS_Store`, a directory someone copied there to look at — none of them are
/// Armada's, and none of them is a reason for `armada manifest check` to refuse to start.
/// What Armada reaps is what Armada can prove it wrote.
pub fn present(root: &Path) -> Result<Vec<RunId>, ArmadaError> {
    let dir = runs_dir(root);
    let entries = match std::fs::read_dir(&dir) {
        Ok(entries) => entries,
        Err(e) if e.kind() == io::ErrorKind::NotFound => return Ok(Vec::new()),
        Err(e) => return Err(environment(&dir, "read", &e)),
    };

    let mut found = Vec::new();
    for entry in entries.flatten() {
        if !entry.path().is_dir() {
            continue;
        }
        if let Some(name) = entry.file_name().to_str() {
            if let Ok(run) = RunId::parse(name) {
                found.push(run);
            }
        }
    }
    found.sort();
    Ok(found)
}

/// Reap old run directories, keeping the most recent `retention` and never
/// touching a live one. Returns what was removed.
///
/// **At the start of each run** (PLAN.md §4.2), because the alternative —
/// coupling retention to `armada manifest clean` — means either logs live forever or you
/// lose the evidence from a failed run the moment you release a port.
pub fn reap(
    root: &Path,
    retention: u32,
    live: &[RunId],
) -> Result<(Vec<RunId>, Vec<String>), ArmadaError> {
    let present = present(root)?;
    let mut removed = Vec::new();
    let mut skipped = Vec::new();

    for run in runs_to_reap(&present, retention, live) {
        let dir = run_dir(root, &run);
        match std::fs::remove_dir_all(&dir) {
            Ok(()) => removed.push(run),
            Err(e) if e.kind() == io::ErrorKind::NotFound => removed.push(run),
            // **Reported, never silent, and never fatal.** A run directory Armada
            // could not remove is disk that stays used; a `armada manifest check` that
            // refuses to start because of it is a repo nobody can check. The
            // two categories are the ones the reap passes already keep apart:
            // Armada could not *look*, which proves nothing, against Armada could
            // not *reclaim*, which is a real leak and is said out loud.
            Err(e) => skipped.push(format!("{}: {e}", dir.display())),
        }
    }
    Ok((removed, skipped))
}

/// Write `state.json`.
///
/// **Written to a temporary file and renamed**, because the reader is
/// `armada manifest explain` and the writer is a run that may be SIGKILLed. `rename(2)` is
/// atomic within a filesystem, so a reader sees either the previous record or
/// this one and never half of one — and a run's record is rewritten on every
/// state change, so the window is not a rare one.
pub fn write_record(root: &Path, record: &RunRecord) -> Result<PathBuf, ArmadaError> {
    let dir = run_dir(root, &record.run_id);
    let target = dir.join("state.json");
    let temporary = dir.join("state.json.writing");

    let json = serde_json::to_string_pretty(record).map_err(|e| ArmadaError {
        class: ErrClass::ArmadaBug,
        r#where: target.display().to_string(),
        message: format!("cannot serialize the run record: {e}"),
        next_action: None,
    })?;

    std::fs::write(&temporary, json.as_bytes())
        .map_err(|e| environment(&temporary, "write", &e))?;
    std::fs::rename(&temporary, &target).map_err(|e| environment(&target, "replace", &e))?;
    Ok(target)
}

/// Read `state.json` back.
///
/// **`None` for a directory with no record**, which is an ordinary state and
/// not a fault: a run reaped mid-write, or a directory created by a `--detach`
/// whose child never got as far as writing one.
pub fn read_record(root: &Path, run: &RunId) -> Result<Option<RunRecord>, ArmadaError> {
    read_json(&run_dir(root, run).join("state.json"), "the run record")
}

/// Where a detached run's own output goes, and the reference a row reports.
///
/// Workspace-relative for the same reason a check's log is
/// ([`log_reference`]): the reader is an agent that will open it, and an
/// absolute path puts the machine's home directory into `--json`.
pub fn detach_log(root: &Path, run: &RunId) -> PathBuf {
    run_dir(root, run).join("detach.log")
}

/// The same path, as a `results[]` row reports it.
pub fn detach_log_reference(run: &RunId) -> String {
    format!(".armada/run/{run}/detach.log")
}

/// Write `detached.json`.
///
/// Same temporary-and-rename as [`write_record`], for a weaker version of the
/// same reason: this one is written once, but its reader is a poll that may
/// arrive during the write.
pub fn write_detached(root: &Path, run: &RunId, detached: &Detached) -> Result<(), ArmadaError> {
    let dir = run_dir(root, run);
    let target = dir.join("detached.json");
    let temporary = dir.join("detached.json.writing");
    let json = serde_json::to_string_pretty(detached).map_err(|e| ArmadaError {
        class: ErrClass::ArmadaBug,
        r#where: target.display().to_string(),
        message: format!("cannot serialize the detach record: {e}"),
        next_action: None,
    })?;
    std::fs::write(&temporary, json.as_bytes())
        .map_err(|e| environment(&temporary, "write", &e))?;
    std::fs::rename(&temporary, &target).map_err(|e| environment(&target, "replace", &e))
}

/// Read `detached.json` back. `None` means the run was never detached.
pub fn read_detached(root: &Path, run: &RunId) -> Result<Option<Detached>, ArmadaError> {
    read_json(
        &run_dir(root, run).join("detached.json"),
        "the detach record",
    )
}

/// One JSON document off disk, where absent and unreadable are different
/// answers.
///
/// **A file that does not exist is `None`; one that does and will not parse is
/// an error.** Folding the second into the first would make a truncated record
/// read as *"this run was never detached"*, which is the one wrong answer a
/// poller acts on.
fn read_json<T: serde::de::DeserializeOwned>(
    path: &Path,
    what: &str,
) -> Result<Option<T>, ArmadaError> {
    let text = match std::fs::read_to_string(path) {
        Ok(text) => text,
        Err(e) if e.kind() == io::ErrorKind::NotFound => return Ok(None),
        Err(e) => return Err(environment(path, "read", &e)),
    };
    serde_json::from_str(&text)
        .map(Some)
        .map_err(|e| ArmadaError {
            class: ErrClass::Environment,
            r#where: path.display().to_string(),
            message: format!("cannot read {what} at {}: {e}", path.display()),
            next_action: Some("`armada manifest check` starts a fresh run".to_string()),
        })
}

fn environment(path: &Path, verb: &str, error: &io::Error) -> ArmadaError {
    ArmadaError {
        class: ErrClass::Environment,
        r#where: path.display().to_string(),
        message: format!("cannot {verb} {}: {error}", path.display()),
        next_action: None,
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use armada_core::id::WorkspaceId;
    use armada_core::schedule::State;

    fn id(n: u64) -> RunId {
        RunId::mint(1_786_000_000_000 + n, 7)
    }

    fn workspace() -> tempfile::TempDir {
        let dir = tempfile::tempdir().expect("a scratch workspace");
        crate::fs::create_armada_dir(dir.path()).expect("`.armada/`");
        dir
    }

    #[test]
    fn preparing_a_run_creates_its_directory_and_its_logs_directory() {
        let dir = workspace();
        let run = id(0);
        let made = prepare(dir.path(), &run).unwrap();
        assert!(made.is_dir());
        assert!(made.join("logs").is_dir());
        assert_eq!(present(dir.path()).unwrap(), vec![run]);
    }

    #[test]
    fn a_workspace_that_has_never_run_anything_lists_no_runs() {
        let dir = tempfile::tempdir().unwrap();
        assert!(
            present(dir.path()).unwrap().is_empty(),
            "no `.armada/` at all"
        );
        crate::fs::create_armada_dir(dir.path()).unwrap();
        assert!(
            present(dir.path()).unwrap().is_empty(),
            "an empty `.armada/`"
        );
    }

    /// `.armada/` is an ordinary directory on a developer's machine. A stray file
    /// is not Armada's and is not a reason for `armada manifest check` to refuse to start.
    #[test]
    fn a_directory_that_is_not_a_run_is_ignored_rather_than_reaped_or_reported() {
        let dir = workspace();
        let run = id(0);
        prepare(dir.path(), &run).unwrap();
        std::fs::create_dir_all(runs_dir(dir.path()).join("not-a-run-id")).unwrap();
        std::fs::write(runs_dir(dir.path()).join(".DS_Store"), b"x").unwrap();

        assert_eq!(present(dir.path()).unwrap(), vec![run]);

        let (removed, skipped) = reap(dir.path(), 0, &[]).unwrap();
        assert_eq!(removed.len(), 1, "only the real run was reaped");
        assert!(skipped.is_empty());
        assert!(
            runs_dir(dir.path()).join("not-a-run-id").is_dir(),
            "Armada removed a directory it did not write"
        );
    }

    #[test]
    fn reaping_keeps_the_most_recent_and_removes_the_rest_from_disk() {
        let dir = workspace();
        let runs: Vec<RunId> = (0..5).map(id).collect();
        for run in &runs {
            prepare(dir.path(), run).unwrap();
        }

        let (removed, skipped) = reap(dir.path(), 3, &[]).unwrap();
        assert!(skipped.is_empty());
        assert_eq!(removed.len(), 2);
        assert_eq!(present(dir.path()).unwrap(), runs[2..].to_vec());
        for gone in &removed {
            assert!(!run_dir(dir.path(), gone).exists());
        }
    }

    /// The measured hazard this rule exists for: writes to an already-open fd
    /// **succeed silently** into an unlinked inode, so deleting a live run's
    /// directory does not fail — the run keeps going and its logs go nowhere.
    #[test]
    fn a_live_run_survives_a_reap_that_would_otherwise_take_it() {
        let dir = workspace();
        let runs: Vec<RunId> = (0..5).map(id).collect();
        for run in &runs {
            prepare(dir.path(), run).unwrap();
        }
        let oldest = runs[0].clone();

        let (removed, _) = reap(dir.path(), 3, std::slice::from_ref(&oldest)).unwrap();
        assert!(!removed.contains(&oldest));
        assert!(
            run_dir(dir.path(), &oldest).is_dir(),
            "the live run is gone"
        );
        assert!(present(dir.path()).unwrap().contains(&oldest));
    }

    #[test]
    fn a_record_is_written_whole_and_reads_back_as_json() {
        let dir = workspace();
        let run = id(0);
        prepare(dir.path(), &run).unwrap();

        let record = RunRecord::new(
            run.clone(),
            WorkspaceId::from_stored("a3f91c02"),
            "2026-08-11T14:02:11Z".to_string(),
            State::new(dir.path().to_path_buf(), 6, Vec::new()),
        );
        let written = write_record(dir.path(), &record).unwrap();
        assert_eq!(written, run_dir(dir.path(), &run).join("state.json"));

        let text = std::fs::read_to_string(&written).unwrap();
        let parsed: serde_json::Value = serde_json::from_str(&text).expect("whole JSON");
        assert_eq!(parsed["run_id"], run.as_str());
        assert_eq!(parsed["workspace"], "a3f91c02");
        assert_eq!(parsed["schema_version"], 2);

        // The temporary is renamed, not left behind — a reader listing the
        // directory must not find two records.
        assert!(!run_dir(dir.path(), &run)
            .join("state.json.writing")
            .exists());
    }

    /// Rewriting replaces rather than appending, and a reader never sees half a
    /// record: the record is rewritten on every state change, so the window is
    /// not a rare one.
    #[test]
    fn rewriting_a_record_replaces_it_atomically() {
        let dir = workspace();
        let run = id(0);
        prepare(dir.path(), &run).unwrap();
        let mut record = RunRecord::new(
            run.clone(),
            WorkspaceId::from_stored("a3f91c02"),
            "2026-08-11T14:02:11Z".to_string(),
            State::new(dir.path().to_path_buf(), 6, Vec::new()),
        );
        write_record(dir.path(), &record).unwrap();

        record.state.now_mono = 42;
        let written = write_record(dir.path(), &record).unwrap();
        let parsed: serde_json::Value =
            serde_json::from_str(&std::fs::read_to_string(&written).unwrap()).unwrap();
        assert_eq!(parsed["state"]["now_mono"], 42);
        assert_eq!(
            std::fs::read_to_string(&written)
                .unwrap()
                .matches("run_id")
                .count(),
            1,
            "the record was appended to rather than replaced"
        );
    }

    /// The path in `results[].log` is workspace-relative, so it is directly
    /// actionable for the agent reading it and carries no machine's home
    /// directory into `--json`.
    #[test]
    fn the_reported_log_path_is_workspace_relative() {
        let run = id(0);
        let reference = log_reference(&run, &CheckId::new("api:lint"));
        assert_eq!(reference, format!(".armada/run/{run}/logs/api.lint.log"));
        assert!(!reference.starts_with('/'));
        assert_eq!(
            log_path(Path::new("/srv/repo"), &run, &CheckId::new("api:lint")),
            Path::new("/srv/repo").join(&reference)
        );
    }

    // ----------------------------------------------------- the detach record

    fn a_detached(pgid: i32) -> Detached {
        Detached {
            pgid,
            boot_id: "boot-a".to_string(),
            started_at: Some("Mon Aug 11 14:02:11 2026".to_string()),
            log: detach_log_reference(&id(0)),
        }
    }

    #[test]
    fn a_detach_record_is_written_where_the_poll_looks_for_it() {
        let dir = workspace();
        let run = id(0);
        prepare(dir.path(), &run).unwrap();
        write_detached(dir.path(), &run, &a_detached(4212)).unwrap();

        assert_eq!(
            read_detached(dir.path(), &run).unwrap(),
            Some(a_detached(4212))
        );
        assert_eq!(
            detach_log(dir.path(), &run),
            dir.path().join(detach_log_reference(&run))
        );
        assert!(!detach_log_reference(&run).starts_with('/'));
    }

    /// **Absent and unreadable are different answers**, and folding them
    /// together is what would make a truncated record read as *"this run was
    /// never detached"* — the one wrong answer a poller acts on, because it
    /// stops asking about a process that is still there.
    #[test]
    fn a_missing_record_is_none_and_a_corrupt_one_is_an_error() {
        let dir = workspace();
        let run = id(0);
        prepare(dir.path(), &run).unwrap();

        assert_eq!(read_detached(dir.path(), &run).unwrap(), None);
        assert!(read_record(dir.path(), &run).unwrap().is_none());

        std::fs::write(run_dir(dir.path(), &run).join("detached.json"), "{").unwrap();
        let error = read_detached(dir.path(), &run).expect_err("half a document is a fault");
        assert_eq!(error.class, ErrClass::Environment);
        assert!(error.next_action.is_some(), "no way forward is offered");
    }

    /// A run directory that has never existed answers `None` rather than
    /// failing: `--status` names the run it could not find, and a read that
    /// errored on `ENOENT` would report the machine as broken instead.
    #[test]
    fn a_run_that_was_never_created_reads_back_as_nothing() {
        let dir = workspace();
        assert!(read_record(dir.path(), &id(7)).unwrap().is_none());
        assert!(read_detached(dir.path(), &id(7)).unwrap().is_none());
    }

    /// The record a run wrote reads back as the record it wrote, which is what
    /// `--status` depends on and what nothing else asserts end to end.
    #[test]
    fn a_run_record_reads_back_as_what_was_written() {
        let dir = workspace();
        let run = id(0);
        prepare(dir.path(), &run).unwrap();
        let record = RunRecord::new(
            run.clone(),
            WorkspaceId::from_stored("a3f91c02"),
            "2026-08-11T14:02:11Z".to_string(),
            State::new(dir.path().to_path_buf(), 6, Vec::new()),
        );
        write_record(dir.path(), &record).unwrap();
        assert_eq!(read_record(dir.path(), &run).unwrap(), Some(record));
    }
}
