//! `.char/run/<run-id>/` on disk: creating it, writing the record, and reaping
//! the ones that have aged out.
//!
//! The decisions are all in [`charkit_core::run`] — the id format, which
//! directories to reap, what a log is called. This module is the half that
//! touches the filesystem, which `ARCHITECTURE.md` §1.1 deliberately does not
//! fake: the semantics that matter here are `ENOENT` versus `EACCES` and an
//! unlinked inode that still accepts writes, and those are exactly the ones a
//! fake gets wrong.

use charkit_core::error::{CharError, ErrClass};
use charkit_core::run::{log_name, runs_to_reap, RunId, RunRecord};
use charkit_core::schedule::CheckId;
use std::io;
use std::path::{Path, PathBuf};

/// `.char/run/`, given a workspace root.
pub fn runs_dir(root: &Path) -> PathBuf {
    root.join(".char").join("run")
}

/// `.char/run/<run-id>/`.
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
/// anything char writes down.
pub fn log_reference(run: &RunId, check: &CheckId) -> String {
    format!(".char/run/{}/logs/{}", run, log_name(check))
}

/// Create `.char/run/<run-id>/logs/`.
pub fn prepare(root: &Path, run: &RunId) -> Result<PathBuf, CharError> {
    let dir = run_dir(root, run);
    let logs = dir.join("logs");
    std::fs::create_dir_all(&logs).map_err(|e| environment(&logs, "create", &e))?;
    Ok(dir)
}

/// Every run directory this workspace has kept.
///
/// **A name that is not a run id is ignored rather than reported.** `.char/` is
/// an ordinary directory on a developer's machine: an editor's swap file, a
/// `.DS_Store`, a directory someone copied there to look at — none of them are
/// char's, and none of them is a reason for `char check` to refuse to start.
/// What char reaps is what char can prove it wrote.
pub fn present(root: &Path) -> Result<Vec<RunId>, CharError> {
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
/// coupling retention to `char clean` — means either logs live forever or you
/// lose the evidence from a failed run the moment you release a port.
pub fn reap(
    root: &Path,
    retention: u32,
    live: &[RunId],
) -> Result<(Vec<RunId>, Vec<String>), CharError> {
    let present = present(root)?;
    let mut removed = Vec::new();
    let mut skipped = Vec::new();

    for run in runs_to_reap(&present, retention, live) {
        let dir = run_dir(root, &run);
        match std::fs::remove_dir_all(&dir) {
            Ok(()) => removed.push(run),
            Err(e) if e.kind() == io::ErrorKind::NotFound => removed.push(run),
            // **Reported, never silent, and never fatal.** A run directory char
            // could not remove is disk that stays used; a `char check` that
            // refuses to start because of it is a repo nobody can check. The
            // two categories are the ones the reap passes already keep apart:
            // char could not *look*, which proves nothing, against char could
            // not *reclaim*, which is a real leak and is said out loud.
            Err(e) => skipped.push(format!("{}: {e}", dir.display())),
        }
    }
    Ok((removed, skipped))
}

/// Write `state.json`.
///
/// **Written to a temporary file and renamed**, because the reader is
/// `char explain` and the writer is a run that may be SIGKILLed. `rename(2)` is
/// atomic within a filesystem, so a reader sees either the previous record or
/// this one and never half of one — and a run's record is rewritten on every
/// state change, so the window is not a rare one.
pub fn write_record(root: &Path, record: &RunRecord) -> Result<PathBuf, CharError> {
    let dir = run_dir(root, &record.run_id);
    let target = dir.join("state.json");
    let temporary = dir.join("state.json.writing");

    let json = serde_json::to_string_pretty(record).map_err(|e| CharError {
        class: ErrClass::CharBug,
        r#where: target.display().to_string(),
        message: format!("cannot serialize the run record: {e}"),
        next_action: None,
    })?;

    std::fs::write(&temporary, json.as_bytes())
        .map_err(|e| environment(&temporary, "write", &e))?;
    std::fs::rename(&temporary, &target).map_err(|e| environment(&target, "replace", &e))?;
    Ok(target)
}

fn environment(path: &Path, verb: &str, error: &io::Error) -> CharError {
    CharError {
        class: ErrClass::Environment,
        r#where: path.display().to_string(),
        message: format!("cannot {verb} {}: {error}", path.display()),
        next_action: None,
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use charkit_core::id::WorkspaceId;
    use charkit_core::schedule::State;

    fn id(n: u64) -> RunId {
        RunId::mint(1_786_000_000_000 + n, 7)
    }

    fn workspace() -> tempfile::TempDir {
        let dir = tempfile::tempdir().expect("a scratch workspace");
        crate::fs::create_char_dir(dir.path()).expect("`.char/`");
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
            "no `.char/` at all"
        );
        crate::fs::create_char_dir(dir.path()).unwrap();
        assert!(present(dir.path()).unwrap().is_empty(), "an empty `.char/`");
    }

    /// `.char/` is an ordinary directory on a developer's machine. A stray file
    /// is not char's and is not a reason for `char check` to refuse to start.
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
            "char removed a directory it did not write"
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
        assert_eq!(parsed["schema_version"], 1);

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
        assert_eq!(reference, format!(".char/run/{run}/logs/api.lint.log"));
        assert!(!reference.starts_with('/'));
        assert_eq!(
            log_path(Path::new("/srv/repo"), &run, &CheckId::new("api:lint")),
            Path::new("/srv/repo").join(&reference)
        );
    }
}
