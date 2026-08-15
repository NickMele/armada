//! The Job index: `~/.armada/jobs/<uuid>.json`.
//!
//! **The record is written before anything runs and survives everything
//! afterwards** (PLAN.md §14.1). A Drone that exits, crashes or is killed does
//! not end its Job, because everything the Job *is* is here — which also means
//! `armada fleet kill` can find and release a Job whose worktree somebody
//! deleted by hand.
//!
//! **The filesystem is not faked** (`ARCHITECTURE.md` §1.1). A fake would prove
//! things about the fake; these tests write into a `TempDir`.

use armada_core::error::{ArmadaError, ErrClass};
use armada_core::fleet::job::Job;
use std::path::{Path, PathBuf};

/// The Job index, rooted at `~/.armada/jobs/`.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Store {
    root: PathBuf,
}

impl Store {
    /// The index under a given `~/.armada/`.
    pub fn at(armada_home: &Path) -> Store {
        Store {
            root: crate::home::jobs(armada_home),
        }
    }

    /// Where the index is.
    pub fn root(&self) -> &Path {
        &self.root
    }

    /// Write a Job's record.
    ///
    /// **Written whole and then renamed into place.** A crash halfway through a
    /// `write` leaves a truncated JSON document that `all()` would report as a
    /// corrupt index; a rename is atomic on every filesystem Armada runs on, so
    /// the record is either the old one or the new one.
    pub fn save(&self, job: &Job) -> Result<(), ArmadaError> {
        std::fs::create_dir_all(&self.root).map_err(|e| self.broken("create", &self.root, &e))?;
        let text = serde_json::to_string_pretty(job).map_err(|e| ArmadaError {
            class: ErrClass::ArmadaBug,
            r#where: job.uuid.clone(),
            message: format!("a Job record would not serialize: {e}"),
            next_action: None,
        })?;
        let final_path = self.path(&job.uuid);
        let staging = final_path.with_extension("json.new");
        std::fs::write(&staging, text).map_err(|e| self.broken("write", &staging, &e))?;
        std::fs::rename(&staging, &final_path).map_err(|e| self.broken("write", &final_path, &e))
    }

    /// One Job by uuid.
    pub fn load(&self, uuid: &str) -> Result<Job, ArmadaError> {
        let path = self.path(uuid);
        let text = std::fs::read_to_string(&path).map_err(|e| self.broken("read", &path, &e))?;
        serde_json::from_str(&text).map_err(|e| ArmadaError {
            class: ErrClass::Environment,
            r#where: path.display().to_string(),
            message: format!("this Job's record does not parse: {e}"),
            next_action: Some("delete the record, or restore it from a backup".to_string()),
        })
    }

    /// Every Job, oldest first.
    ///
    /// **A record that will not parse is skipped rather than fatal.** `armada
    /// fleet ls` reports rather than judges (`commands/fleet/ls.md`), and one
    /// damaged file must not hide the four Jobs that are fine — which is the
    /// state a reader is most likely to be in when they run it.
    pub fn all(&self) -> Result<Vec<Job>, ArmadaError> {
        let entries = match std::fs::read_dir(&self.root) {
            Ok(entries) => entries,
            // No index is an empty fleet, not a broken machine: it is what every
            // machine looks like before its first spawn.
            Err(e) if e.kind() == std::io::ErrorKind::NotFound => return Ok(Vec::new()),
            Err(e) => return Err(self.broken("read", &self.root, &e)),
        };

        let mut jobs: Vec<Job> = entries
            .filter_map(|entry| {
                let path = entry.ok()?.path();
                (path.extension()?.to_str()? == "json").then_some(())?;
                serde_json::from_str(&std::fs::read_to_string(&path).ok()?).ok()
            })
            .collect();
        jobs.sort_by(|a: &Job, b: &Job| {
            (a.created_ms, a.name.clone()).cmp(&(b.created_ms, b.name.clone()))
        });
        Ok(jobs)
    }

    /// One Job by the handle a person typed: its name, or its uuid.
    ///
    /// **A name over a uuid prefix**, because the name is what the table showed
    /// and the uuid is what the transcript is called. An ambiguous prefix is
    /// refused rather than resolved to the first match — killing the wrong Job
    /// is not recoverable by re-running the command.
    pub fn find(&self, handle: &str) -> Result<Job, ArmadaError> {
        let jobs = self.all()?;
        if let Some(job) = jobs.iter().find(|job| job.name == handle) {
            return Ok(job.clone());
        }
        let matches: Vec<&Job> = jobs
            .iter()
            .filter(|job| job.uuid.starts_with(handle))
            .collect();
        match matches.as_slice() {
            [job] => Ok((*job).clone()),
            [] => Err(unknown(handle, &jobs)),
            several => Err(ArmadaError {
                class: ErrClass::BadInvocation,
                r#where: handle.to_string(),
                message: format!("`{handle}` names {} Jobs", several.len()),
                next_action: Some(format!(
                    "use a name: {}",
                    several
                        .iter()
                        .map(|job| job.name.as_str())
                        .collect::<Vec<_>>()
                        .join(", ")
                )),
            }),
        }
    }

    /// Whether a live Job already answers to this name.
    ///
    /// **Live only.** A name is a handle rather than a key, and refusing to
    /// reuse the name of a Job that finished last week would make every good
    /// name single-use.
    pub fn name_is_taken(&self, name: &str) -> Result<bool, ArmadaError> {
        Ok(self
            .all()?
            .iter()
            .any(|job| job.name == name && !job.state.is_over()))
    }

    /// A name nobody live is using: `rate-limit`, then `rate-limit-2`.
    pub fn free_name(&self, wanted: &str) -> Result<String, ArmadaError> {
        if !self.name_is_taken(wanted)? {
            return Ok(wanted.to_string());
        }
        for suffix in 2..100 {
            let candidate = format!("{wanted}-{suffix}");
            if !self.name_is_taken(&candidate)? {
                return Ok(candidate);
            }
        }
        Err(ArmadaError {
            class: ErrClass::BadInvocation,
            r#where: wanted.to_string(),
            message: format!("a hundred live Jobs are already called `{wanted}`"),
            next_action: Some("pass --name, or kill some of them".to_string()),
        })
    }

    /// Drop a Job's record.
    ///
    /// **`armada fleet kill` does not call this**, and that is the design: a
    /// killed Job is marked ended and keeps its record, because the record is
    /// how the transcript is found afterwards. This exists for the caller that
    /// genuinely wants the row gone.
    pub fn forget(&self, uuid: &str) -> Result<(), ArmadaError> {
        let path = self.path(uuid);
        match std::fs::remove_file(&path) {
            Ok(()) => Ok(()),
            Err(e) if e.kind() == std::io::ErrorKind::NotFound => Ok(()),
            Err(e) => Err(self.broken("remove", &path, &e)),
        }
    }

    fn path(&self, uuid: &str) -> PathBuf {
        self.root.join(format!("{uuid}.json"))
    }

    /// **`environment`, and never `tool_failed`.** A Job index Armada cannot
    /// read is the machine being broken rather than the repository being wrong,
    /// and the correct response is the identical command after a person fixes
    /// the disk (`ARCHITECTURE.md` §1.7).
    fn broken(&self, verb: &str, path: &Path, error: &std::io::Error) -> ArmadaError {
        ArmadaError {
            class: ErrClass::Environment,
            r#where: path.display().to_string(),
            message: format!("could not {verb} the Job index: {error}"),
            next_action: Some(
                "check ~/.armada/jobs/ is readable, then retry unchanged".to_string(),
            ),
        }
    }
}

fn unknown(handle: &str, jobs: &[Job]) -> ArmadaError {
    ArmadaError {
        class: ErrClass::BadInvocation,
        r#where: handle.to_string(),
        message: format!("no Job called `{handle}`"),
        next_action: Some(match jobs.first() {
            Some(job) => format!("`armada fleet ls` lists them — `{}` is one", job.name),
            None => "`armada fleet ls` lists them; there are none yet".to_string(),
        }),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use armada_core::fleet::job::Spend;
    use armada_core::fleet::workflow::DEFAULT_BUDGET;
    use armada_core::fleet::JobState;

    fn job(name: &str, uuid: &str, state: JobState, created_ms: u64) -> Job {
        Job {
            uuid: uuid.to_string(),
            name: name.to_string(),
            workflow: "feature".to_string(),
            confidence: Some(0.91),
            repo: "api".to_string(),
            worktree: format!("~/.armada/workspaces/api/{name}"),
            branch: format!("armada/{name}"),
            port_block: None,
            budget: DEFAULT_BUDGET,
            state,
            step: "implement".to_string(),
            verdict: None,
            created_at: "2026-08-09T14:02:11Z".to_string(),
            created_ms,
            spend: Spend::default(),
            task: "do the thing".to_string(),
        }
    }

    fn store() -> (tempfile::TempDir, Store) {
        let home = tempfile::tempdir().unwrap();
        let store = Store::at(home.path());
        (home, store)
    }

    #[test]
    fn a_job_written_is_a_job_read_back() {
        let (_home, store) = store();
        let written = job("rate-limit", "8f2a-1", JobState::Running, 10);
        store.save(&written).unwrap();
        assert_eq!(store.load("8f2a-1").unwrap(), written);
    }

    /// **A machine before its first spawn is an empty fleet, not a broken
    /// one.** `armada fleet ls` exits 0 whenever the index is readable, and an
    /// absent index is readable.
    #[test]
    fn an_index_that_does_not_exist_yet_is_an_empty_fleet() {
        let (_home, store) = store();
        assert_eq!(store.all().unwrap(), Vec::new());
        assert!(!store.name_is_taken("rate-limit").unwrap());
    }

    #[test]
    fn jobs_come_back_oldest_first() {
        let (_home, store) = store();
        store
            .save(&job("second", "b", JobState::Running, 200))
            .unwrap();
        store
            .save(&job("first", "a", JobState::Running, 100))
            .unwrap();
        assert_eq!(
            store
                .all()
                .unwrap()
                .iter()
                .map(|job| job.name.clone())
                .collect::<Vec<_>>(),
            ["first", "second"]
        );
    }

    /// **One damaged record must not hide the Jobs that are fine.** `ls` reports
    /// rather than judges, and a reader running it after a crash is exactly the
    /// person who needs the other four rows.
    #[test]
    fn a_record_that_will_not_parse_is_skipped_rather_than_fatal() {
        let (_home, store) = store();
        store
            .save(&job("intact", "a", JobState::Running, 100))
            .unwrap();
        std::fs::write(store.root().join("broken.json"), "{not json").unwrap();
        let jobs = store.all().unwrap();
        assert_eq!(jobs.len(), 1);
        assert_eq!(jobs[0].name, "intact");
    }

    #[test]
    fn a_job_is_found_by_its_name_or_by_a_uuid_prefix() {
        let (_home, store) = store();
        store
            .save(&job("rate-limit", "8f2a1c40-33b1", JobState::Running, 10))
            .unwrap();
        assert_eq!(store.find("rate-limit").unwrap().uuid, "8f2a1c40-33b1");
        assert_eq!(store.find("8f2a").unwrap().name, "rate-limit");
    }

    /// **An ambiguous prefix is refused rather than resolved to the first
    /// match.** Killing the wrong Job is not recoverable by re-running the
    /// command.
    #[test]
    fn a_prefix_that_names_two_jobs_is_refused_and_names_both() {
        let (_home, store) = store();
        store
            .save(&job("one", "8f2a-aaa", JobState::Running, 10))
            .unwrap();
        store
            .save(&job("two", "8f2a-bbb", JobState::Running, 20))
            .unwrap();
        let error = store.find("8f2a").unwrap_err();
        assert_eq!(error.class, ErrClass::BadInvocation);
        let next = error.next_action.unwrap();
        assert!(next.contains("one") && next.contains("two"), "{next}");
    }

    #[test]
    fn an_unknown_handle_is_a_bad_invocation_that_points_at_ls() {
        let (_home, store) = store();
        let error = store.find("nonesuch").unwrap_err();
        assert_eq!(error.class, ErrClass::BadInvocation);
        assert_eq!(error.class.exit_code(), 2);
        assert!(error.next_action.unwrap().contains("armada fleet ls"));
    }

    /// **A name is a handle, not a key.** Refusing to reuse the name of a Job
    /// that finished last week would make every good name single-use.
    #[test]
    fn a_finished_jobs_name_is_free_again() {
        let (_home, store) = store();
        store
            .save(&job("rate-limit", "a", JobState::Done, 10))
            .unwrap();
        assert!(!store.name_is_taken("rate-limit").unwrap());
        assert_eq!(store.free_name("rate-limit").unwrap(), "rate-limit");
    }

    #[test]
    fn a_live_jobs_name_is_taken_and_the_next_one_is_numbered() {
        let (_home, store) = store();
        store
            .save(&job("rate-limit", "a", JobState::Running, 10))
            .unwrap();
        assert!(store.name_is_taken("rate-limit").unwrap());
        assert_eq!(store.free_name("rate-limit").unwrap(), "rate-limit-2");

        store
            .save(&job("rate-limit-2", "b", JobState::Blocked, 20))
            .unwrap();
        assert_eq!(store.free_name("rate-limit").unwrap(), "rate-limit-3");
    }

    /// A record is written whole: a reader never sees half a document, because
    /// the write lands under a staging name and is renamed into place.
    #[test]
    fn saving_twice_replaces_the_record_and_leaves_no_staging_file() {
        let (_home, store) = store();
        store.save(&job("a", "u", JobState::Queued, 1)).unwrap();
        store.save(&job("a", "u", JobState::Running, 1)).unwrap();
        assert_eq!(store.load("u").unwrap().state, JobState::Running);
        assert_eq!(store.all().unwrap().len(), 1, "one Job, one record");
    }

    #[test]
    fn forgetting_a_job_that_is_already_gone_is_not_an_error() {
        let (_home, store) = store();
        store.save(&job("a", "u", JobState::Done, 1)).unwrap();
        store.forget("u").unwrap();
        store.forget("u").unwrap();
        assert!(store.all().unwrap().is_empty());
    }
}
