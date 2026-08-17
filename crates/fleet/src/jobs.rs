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
        // # The staging name is this writer's, not this Job's
        //
        // **It was `<uuid>.json.new` — one fixed path per Job, shared by every
        // process saving it.** Two concurrent saves of one Job both wrote that
        // file, the first `rename` moved it, and the second failed `ENOENT`
        // trying to move a path that no longer existed. The error read "could
        // not write the Job index: No such file or directory" and named the
        // *final* path, which is the one place that was fine.
        //
        // Recorded twice on one machine, both times during concurrent activity
        // — a Drone's `Stop` hook ticking its Job while a verb was writing the
        // same record. Concurrency here is ordinary and expected: the relay is
        // a process Armada does not own, and PLAN.md §15.3 has it firing
        // whenever an exchange ends.
        //
        // A pid **and a counter** make the staging file this writer's alone, so
        // a race is two harmless writes and two renames, and the last one wins.
        // The counter is not decoration: the first fix here was the pid alone,
        // and the test below reproduced the collision anyway, because two
        // threads of one process share a pid. Processes are the case seen in the
        // wild; threads are the case the test could run.
        // **That is the same outcome the single-path version was trying to
        // have** — a whole record replaced atomically — minus the failure. It
        // does not order the writers, and it is not meant to: whoever renames
        // last has the newest observation, which is what every reader of this
        // store already assumes.
        static WRITER: std::sync::atomic::AtomicU64 = std::sync::atomic::AtomicU64::new(0);
        let mine = WRITER.fetch_add(1, std::sync::atomic::Ordering::Relaxed);
        let staging = final_path.with_extension(format!("json.{}.{mine}.new", std::process::id()));
        std::fs::write(&staging, text).map_err(|e| self.broken("write", &staging, &e))?;
        std::fs::rename(&staging, &final_path).map_err(|e| {
            // **Take our own staging file with us.** A per-writer name cannot be
            // reused by the next attempt the way the old fixed one was, so a
            // failed rename that left it behind would leak one file per failure
            // forever. A `SIGKILL` between the write and the rename still
            // leaks, which is why `all()` skips anything not named `*.json`
            // rather than trusting the directory to hold only records.
            let _ = std::fs::remove_file(&staging);
            self.broken("write", &final_path, &e)
        })
    }

    /// One Job by uuid.
    pub fn load(&self, uuid: &str) -> Result<Job, ArmadaError> {
        let path = self.path(uuid);
        let text = std::fs::read_to_string(&path).map_err(|e| {
            // **A record that is not there is not a broken machine.** This said
            // "could not read the Job index: No such file or directory" and
            // told the reader to check that `~/.armada/jobs/` was readable —
            // advice for a directory that was fine. The ordinary causes are a
            // reaped Job and a mistyped handle, and neither is helped by
            // retrying unchanged, which is what `Environment` invites.
            if e.kind() == std::io::ErrorKind::NotFound {
                return ArmadaError {
                    class: ErrClass::BadInvocation,
                    r#where: uuid.to_string(),
                    message: format!("no Job here with the id {uuid}"),
                    next_action: Some(
                        "`armada fleet ls` lists every Job; a reaped Job is gone for good"
                            .to_string(),
                    ),
                };
            }
            self.broken("read", &path, &e)
        })?;
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
    ///
    /// **An ambiguous *name* is refused too, and that is the decision worth
    /// stating.** A uuid is identity; a name is a label, and nothing enforces
    /// that a label is unique — [`Store::name_is_taken`] only refuses to reuse
    /// the name of a Job that is still *live*, so a finished `rate-limit` and a
    /// running `rate-limit` are both ordinary and both on disk. Resolution used
    /// to take the first match over a list sorted by creation, and say nothing
    /// about the choice.
    ///
    /// **A live Job does not win the tie**, which was the tempting fix. It is
    /// the same coin flip with better odds: a person who typed a name that
    /// means two things knows which one they meant and Armada does not, and
    /// `kill` is not undoable. So both candidates are named — with the state,
    /// the step and the day that tell them apart — and the uuid is what
    /// disambiguates.
    pub fn find(&self, handle: &str) -> Result<Job, ArmadaError> {
        // **An empty handle is refused before anything is matched.** Every uuid
        // starts with the empty string, so `starts_with("")` matched the whole
        // fleet and the refusal read *"`` is the start of 12 Jobs"* — a sentence
        // about a prefix nobody typed, listing every Job on the machine, and
        // ending in *"name one by uuid"* as though the reader had been too
        // vague rather than passed nothing at all.
        //
        // Recorded twice on this machine before it was noticed, because a
        // refusal that lists twelve real Jobs looks like a hard question rather
        // than a bug.
        if handle.trim().is_empty() {
            return Err(ArmadaError {
                class: ErrClass::BadInvocation,
                r#where: "job".to_string(),
                message: "no Job was named".to_string(),
                next_action: Some(
                    "`armada fleet ls` lists them; name one by handle or uuid".to_string(),
                ),
            });
        }
        let jobs = self.all()?;
        let named: Vec<&Job> = jobs.iter().filter(|job| job.name == handle).collect();
        match named.as_slice() {
            [job] => return Ok((*job).clone()),
            [] => {}
            several => return Err(ambiguous(handle, several, "names")),
        }
        let matches: Vec<&Job> = jobs
            .iter()
            .filter(|job| job.uuid.starts_with(handle))
            .collect();
        match matches.as_slice() {
            [job] => Ok((*job).clone()),
            [] => Err(unknown(handle, &jobs)),
            several => Err(ambiguous(handle, several, "is the start of")),
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

/// How long a uuid has to be before it is a handle a person can type.
///
/// **Eight, which is what `git` settled on for the same problem**: long enough
/// that a collision is not a practical worry over the number of Jobs one machine
/// has, short enough to retype from a screen.
pub const SHORT_UUID: usize = 8;

/// A uuid as a handle: the first [`SHORT_UUID`] characters.
pub fn short(uuid: &str) -> &str {
    match uuid.char_indices().nth(SHORT_UUID) {
        Some((at, _)) => &uuid[..at],
        None => uuid,
    }
}

/// A handle that means more than one Job, refused rather than resolved.
///
/// **Every candidate, with what tells them apart.** A refusal that only said
/// "two Jobs" would leave the reader to run `ls` and match the rows up by eye;
/// the state, the step and the day are what a person actually recognises a Job
/// by, and the short uuid is what they type next.
fn ambiguous(handle: &str, several: &[&Job], verb: &str) -> ArmadaError {
    let described: Vec<String> = several
        .iter()
        .map(|job| {
            format!(
                "{} {} at {}, {}",
                short(&job.uuid),
                job.state.word(),
                match job.step.is_empty() {
                    true => "no step",
                    false => job.step.as_str(),
                },
                // The day, not the instant: the timestamp is only ever displayed
                // and a reader is telling "this morning's" from "last week's".
                job.created_at.split('T').next().unwrap_or(&job.created_at)
            )
        })
        .collect();
    ArmadaError {
        class: ErrClass::BadInvocation,
        r#where: handle.to_string(),
        message: format!(
            "`{handle}` {verb} {} Jobs: {}",
            several.len(),
            described.join("; ")
        ),
        next_action: Some(format!("name one by uuid: `{}`", short(&several[0].uuid))),
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
            budget_set: Vec::new(),
            uuid: uuid.to_string(),
            name: name.to_string(),
            workflow: "feature".to_string(),
            confidence: Some(0.91),
            repo: "api".to_string(),
            repo_root: "~/code/api".to_string(),
            worktree: format!("~/.armada/workspaces/api/{name}"),
            branch: format!("armada/{name}"),
            port_block: None,
            budget: DEFAULT_BUDGET,
            state,
            step: "implement".to_string(),
            verdict: None,
            drone: None,
            created_at: "2026-08-09T14:02:11Z".to_string(),
            created_ms,
            spend: Spend::default(),
            task: "do the thing".to_string(),
            progress: Vec::new(),
            attempts: std::collections::BTreeMap::new(),
            waited_ms: 0,
            waiting_from_ms: None,
            transitions: Vec::new(),
            pending: None,
            facts: std::collections::BTreeMap::new(),
            kin: Default::default(),
            ticked_turns: 0,
            doing: None,
            daemon_acts: Vec::new(),
            main_moved_at: None,
        }
    }

    fn store() -> (tempfile::TempDir, Store) {
        let home = tempfile::tempdir().unwrap();
        let store = Store::at(home.path());
        (home, store)
    }

    /// **Two writers saving one Job do not fail.**
    ///
    /// The staging path was `<uuid>.json.new` — one fixed name per Job, shared
    /// by every process saving it. Two concurrent saves both wrote that file,
    /// the first `rename` moved it, and the second failed `ENOENT` trying to
    /// move a path that no longer existed. The error read "could not write the
    /// Job index: No such file or directory" and named the *final* path, which
    /// is the one place that was fine.
    ///
    /// Recorded twice on a real machine, both times while a Drone's `Stop` hook
    /// was ticking its Job and a verb was writing the same record. Concurrency
    /// here is ordinary: the relay is a process Armada does not own and PLAN.md
    /// §15.3 has it firing whenever an exchange ends.
    ///
    /// **Threads rather than a contrived interleaving**, because the failure is
    /// a genuine race and the honest way to catch it is to run it. Fifty rounds
    /// across two writers hit it reliably before the fix; a test that only saved
    /// twice in sequence never would, which is why the fixed staging name
    /// survived this long.
    #[test]
    fn two_writers_saving_one_job_do_not_collide() {
        let (home, _) = store();
        let root = home.path().to_path_buf();
        let record = job("shared", "one-uuid", JobState::Running, 10);

        let failures: Vec<String> = std::thread::scope(|scope| {
            let hands: Vec<_> = (0..2)
                .map(|_| {
                    let root = root.clone();
                    let record = record.clone();
                    scope.spawn(move || {
                        let store = Store::at(&root);
                        (0..50)
                            .filter_map(|_| store.save(&record).err())
                            .map(|error| error.message)
                            .collect::<Vec<_>>()
                    })
                })
                .collect();
            hands
                .into_iter()
                .flat_map(|hand| hand.join().unwrap())
                .collect()
        });

        assert!(
            failures.is_empty(),
            "a concurrent save failed {} times: {:?}",
            failures.len(),
            &failures[..failures.len().min(3)]
        );
        // And the record is still readable — a race that left a torn file would
        // be the same bug with a different symptom.
        assert_eq!(Store::at(&root).load("one-uuid").unwrap().name, "shared");
    }

    /// **A Job that is not there is a bad handle, not a broken machine.**
    ///
    /// `load` reported every `read_to_string` failure as `Environment` with
    /// "could not read the Job index: No such file or directory" and the next
    /// action "check `~/.armada/jobs/` is readable, then retry unchanged" —
    /// which is advice for a directory that was fine, and a retry that cannot
    /// help. A reaped Job and a mistyped handle both arrive this way, and both
    /// were recorded as a failure of the environment.
    #[test]
    fn asking_for_a_job_that_is_not_there_is_a_bad_handle() {
        let (_home, store) = store();
        let error = store.load("no-such-uuid").unwrap_err();

        assert_eq!(error.class, ErrClass::BadInvocation);
        assert!(
            error
                .message
                .contains("no Job here with the id no-such-uuid"),
            "the message still blames the index: {}",
            error.message
        );
        // A real permission or corruption failure must still read as one, so
        // the two are not collapsed into a single friendly message.
        assert!(!error.message.contains("Job index"));
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

    /// **An empty handle is refused as an empty handle, not as a vague prefix.**
    ///
    /// Every uuid starts with the empty string, so `starts_with("")` matched the
    /// whole fleet and the refusal read *"`` is the start of 12 Jobs"* — listing
    /// every Job on the machine and ending in *"name one by uuid"*, as though
    /// the reader had been imprecise rather than passed nothing at all.
    /// Recorded twice on a real machine before anybody noticed, because a
    /// refusal that lists twelve real Jobs reads like a hard question.
    #[test]
    fn no_handle_at_all_is_refused_without_listing_the_whole_fleet() {
        let (_home, store) = store();
        for (name, uuid) in [("one", "8f2a-aaa"), ("two", "91bc-bbb")] {
            store.save(&job(name, uuid, JobState::Running, 10)).unwrap();
        }
        for handle in ["", "   "] {
            let error = store.find(handle).unwrap_err();
            assert_eq!(error.class, ErrClass::BadInvocation);
            assert_eq!(error.message, "no Job was named");
            assert!(
                !error.message.contains("8f2a") && !error.message.contains("91bc"),
                "the refusal listed the fleet at a caller who named nothing: {}",
                error.message
            );
            assert!(error.next_action.unwrap().contains("armada fleet ls"));
        }
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
        // Both candidates, by the uuid that disambiguates them — the *name* is
        // not the answer here, because two Jobs can share one.
        assert!(
            error.message.contains("8f2a-aaa") && error.message.contains("8f2a-bbb"),
            "{}",
            error.message
        );
        assert!(error.next_action.unwrap().contains("uuid"));
    }

    /// **A name two Jobs share is refused, and both are named.** This is the
    /// shape a person actually had on disk: two records both called
    /// `this-test`, the older one `ABORTED` at `explore` and the newer one
    /// `RUNNING` at `plan` and holding a port block. Resolution took the first
    /// match over a list sorted by creation and said nothing about the choice,
    /// so `kill this-test` would have ended the wrong one — and a kill is not
    /// undoable.
    ///
    /// **A live Job does not win the tie.** That is the same coin flip with
    /// better odds: the person who typed the name knows which they meant and
    /// Armada does not.
    #[test]
    fn a_name_two_jobs_share_is_refused_rather_than_resolved() {
        let (_home, store) = store();
        let mut old = job("this-test", "c19d0a34-1111", JobState::Aborted, 10);
        old.step = "explore".to_string();
        let mut new = job("this-test", "94b1fd2e-2222", JobState::Running, 20);
        new.step = "plan".to_string();
        store.save(&old).unwrap();
        store.save(&new).unwrap();

        let error = store.find("this-test").unwrap_err();
        assert_eq!(error.class, ErrClass::BadInvocation);
        assert_eq!(error.class.exit_code(), 2);

        // Both candidates, with what tells them apart.
        for word in [
            "c19d0a34", "94b1fd2e", "ABORTED", "RUNNING", "explore", "plan",
        ] {
            assert!(
                error.message.contains(word),
                "the refusal does not name `{word}`: {}",
                error.message
            );
        }
        // And the disambiguator is a uuid a person can retype.
        let next = error.next_action.expect("a next action");
        assert!(next.contains("uuid"), "{next}");
        assert!(next.contains("c19d0a34"), "{next}");

        // The uuid always works, whichever one was meant.
        assert_eq!(store.find("94b1fd2e").unwrap().uuid, "94b1fd2e-2222");
        assert_eq!(store.find("c19d0a34").unwrap().uuid, "c19d0a34-1111");
    }

    /// **Ended does not stop being a candidate.** Both Jobs are durable records
    /// and either could be the one meant, so an `ABORTED` and a `DONE` of one
    /// name are ambiguous exactly as a live pair would be.
    #[test]
    fn a_name_only_finished_jobs_hold_is_ambiguous_too() {
        let (_home, store) = store();
        store
            .save(&job("this-test", "aaaa-old", JobState::Aborted, 10))
            .unwrap();
        store
            .save(&job("this-test", "bbbb-new", JobState::Done, 20))
            .unwrap();
        assert_eq!(
            store.find("this-test").unwrap_err().class,
            ErrClass::BadInvocation
        );
        assert_eq!(store.find("bbbb").unwrap().uuid, "bbbb-new");
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
