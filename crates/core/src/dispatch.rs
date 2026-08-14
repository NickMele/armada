//! The dispatch record: what char knew at the moment it ran a check
//! (PLAN.md §3.4).
//!
//! **The model is `docker inspect`.** `inspect` can answer everything about a
//! container because the daemon recorded it at create time and kept it, not
//! because it recomputes anything on demand. This is the same: a read of a
//! record, never a computation.
//!
//! **Written at dispatch, because most of it cannot be recovered afterwards.**
//! Query `manifest.db` an hour later and it truthfully reports who holds the browser
//! *now*, which is a different and useless answer to "what was `web:e2e`
//! waiting on when it timed out". Live state answers a different question than
//! the one being asked, so the record has to be made at the time or not at all.
//! `char explain` in phase 5 is a reader with nothing to read without this.
//!
//! **A reconstruction that disagrees is worse than none.** An agent that
//! reimplements the substitution and the argv split produces a command it
//! *believes* ran; if its quote handling differs in one case it diagnoses a
//! command that never executed, and nothing reveals the divergence. The value
//! here is authority — this is what actually ran — not availability.
//!
//! **It inherits `docker inspect`'s shape and not its mistake.** `inspect` dumps
//! environment variable *values*, which is a well-known way secrets escape, and
//! the compose form of it is measured in `docs/traps.md` inlining `.env` values
//! into its output. This record carries environment **names only**
//! (`ARCHITECTURE.md` §1.8) — a diagnosis channel that bypassed the scrubber
//! would make that invariant an invariant with an exception.

use crate::error::CharError;
use crate::id::WorkspaceId;
use crate::lease::LeaseKind;
use crate::schedule::{CheckId, Event, Plan};
use serde::{Deserialize, Serialize};
use std::collections::BTreeMap;
use std::path::{Path, PathBuf};

/// How much of a failing check's output the signature hashes.
///
/// The tail rather than the head: a tool prints its summary last, and a run that
/// scrolled ten thousand lines of progress differs from the same failure without
/// the progress. Four kilobytes is enough for a stack trace and a summary line
/// and small enough that hashing it is free.
pub const SIGNATURE_TAIL_BYTES: usize = 4 * 1024;

/// What char knew when it ran one check.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Dispatch {
    /// Which check.
    pub check: CheckId,
    /// **The argv char executed, post-substitution.** Only recoverable by
    /// reimplementing char: the substitution, the `${files}` set and the argv
    /// split with quote handling.
    pub argv: Vec<String>,
    /// The working directory. Recoverable — PLAN.md §4.1 fixes it at the
    /// workspace root, so it is a constant rather than a discovery — and
    /// recorded anyway, because the record's value is *authority* and a reader
    /// that has to look one field up elsewhere will eventually look it up wrong.
    pub cwd: PathBuf,
    /// **Names only.** See the module note.
    pub env: Vec<String>,
    /// The `${files}` set, exactly as it was expanded into argv.
    #[serde(skip_serializing_if = "Vec::is_empty", default)]
    pub files: Vec<String>,
    /// The leases held at the moment of the spawn.
    #[serde(skip_serializing_if = "Vec::is_empty", default)]
    pub leases: Vec<String>,
    /// **What this check waited on, and who held it.** Point-in-time state that
    /// no longer exists: this is the row PLAN.md §3.4 marks "No" in the
    /// recoverable column, and the only useful answer to "why has this taken
    /// fifteen minutes".
    #[serde(skip_serializing_if = "Vec::is_empty", default)]
    pub waited_on: Vec<Waited>,
    /// The monotonic reading at dispatch.
    pub dispatched_at_mono: u64,
    /// **Completed when the check finishes**, because an exit code and an output
    /// tail do not exist yet at dispatch. The rest of this record is written
    /// before the child starts, for the same reason `up` records before it
    /// spawns: the failure mode must be a stale row, never an untracked
    /// resource.
    #[serde(skip_serializing_if = "Option::is_none", default)]
    pub signature: Option<Signature>,
}

impl Dispatch {
    /// The record for a check about to be spawned.
    pub fn new(plan: &Plan, cwd: &Path, files: &[String], now_mono: u64) -> Self {
        Dispatch {
            check: plan.id.clone(),
            argv: plan.argv.clone(),
            cwd: cwd.to_path_buf(),
            env: plan.env.names(),
            files: files.to_vec(),
            leases: Vec::new(),
            waited_on: Vec::new(),
            dispatched_at_mono: now_mono,
            signature: None,
        }
    }

    /// Record a lease this check holds.
    pub fn holding(&mut self, kind: LeaseKind, key: &str) {
        let held = format!("{kind}:{key}");
        if !self.leases.contains(&held) {
            self.leases.push(held);
        }
    }

    /// Record something this check queued behind, and who had it.
    pub fn waited(&mut self, kind: LeaseKind, key: &str, holder: Option<WorkspaceId>, ms: u64) {
        self.waited_on.push(Waited {
            kind: kind.to_string(),
            key: key.to_string(),
            held_by: holder,
            waited_ms: ms,
        });
    }
}

/// One thing a check queued behind.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Waited {
    /// The lease class, spelled as `manifest.db` spells it.
    pub kind: String,
    /// Which lease within the class — the exclusive's name, or the slot.
    pub key: String,
    /// **The workspace that had it.** `None` for a claim char never saw a holder
    /// for, which is honest: the record says what was observed and never guesses.
    #[serde(skip_serializing_if = "Option::is_none", default)]
    pub held_by: Option<WorkspaceId>,
    /// How long the wait lasted.
    pub waited_ms: u64,
}

/// A fingerprint for *same or different*, never a diagnosis.
///
/// `(check_id, exit_code, blake3(normalised tail of output))` (PLAN.md §3.4).
/// **Deterministic, so two runs of one bug always match** — which is what makes
/// the history row possible, and the history row is the one that changes an
/// agent's behaviour. "This check failed the same way in the last three runs,
/// none of which touched its files" and "this check passed twenty minutes ago
/// and the only change since is one file" are opposite problems, and a stack
/// trace is identical in both.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Signature {
    /// The check that failed.
    pub check: CheckId,
    /// What it exited with.
    pub exit_code: i32,
    /// `blake3` of the normalised tail, hex.
    pub digest: String,
}

/// Compute a failure signature.
pub fn signature(check: &CheckId, exit_code: i32, output: &str, scrub: &Scrub) -> Signature {
    let tail = tail_of(output, SIGNATURE_TAIL_BYTES);
    let normalised = scrub.normalise(tail);
    Signature {
        check: check.clone(),
        exit_code,
        digest: blake3::hash(normalised.as_bytes()).to_hex().to_string(),
    }
}

/// The last `limit` bytes of a string, cut at a character boundary.
fn tail_of(text: &str, limit: usize) -> &str {
    if text.len() <= limit {
        return text;
    }
    let mut start = text.len() - limit;
    while start < text.len() && !text.is_char_boundary(start) {
        start += 1;
    }
    &text[start..]
}

/// What normalisation strips, given this run's own particulars.
///
/// **Everything here differs between two runs of the same failure**, and leaving
/// any of it in makes the signature a fingerprint of the run rather than of the
/// bug — which is worse than having none, because the history row would then
/// always read "this is new".
pub struct Scrub {
    root: String,
    workspace: String,
}

impl Scrub {
    /// A scrubber for one workspace.
    pub fn new(root: &Path, workspace: &WorkspaceId) -> Self {
        Scrub {
            root: root.display().to_string(),
            workspace: workspace.as_str().to_string(),
        }
    }

    /// Strip the run's particulars out of a failure's output.
    ///
    /// **A path inside the workspace keeps its filename, and a path outside it
    /// does not.** That asymmetry is the whole of this function, and getting it
    /// wrong in either direction breaks the signature:
    ///
    /// - Leaving an outside path in — `/opt/homebrew/lib/python3.12/…`, a
    ///   temporary directory, another developer's checkout — makes the same
    ///   failure on two machines look like two failures, so the history row
    ///   always reads "this is new" and is worth nothing.
    /// - Erasing an inside path makes two *different* failures look like one.
    ///   `services/api/views.py:12` and `services/api/models.py:12` are not the
    ///   same bug, and an agent told "same as last time" stops reading. This
    ///   was the live defect in the first version of this function: the generic
    ///   absolute-path rule ran over the workspace-relative remainder and ate
    ///   the filename with it. A mutation that deleted the root substitution
    ///   changed no test, which is how it was found.
    pub fn normalise(&self, text: &str) -> String {
        let mut out = text.replace(&self.root, ROOT);
        if !self.workspace.is_empty() {
            out = out.replace(&self.workspace, "<workspace>");
        }
        out = elide_foreign_paths(&out);
        for (pattern, replacement) in patterns() {
            out = pattern.replace_all(&out, *replacement).into_owned();
        }
        out
    }
}

/// What the workspace root is replaced with.
const ROOT: &str = "<root>";

/// Collapse every absolute path **except** the one immediately following a
/// [`ROOT`] marker, which is this repo's own and is the actionable half.
///
/// Written by hand rather than as a pattern because the rule is "not preceded
/// by", and char validates with a Rust regex engine that has no lookbehind —
/// the same constraint PLAN.md §4.1.1 records for the JSON Schema, arrived at
/// from the other direction.
fn elide_foreign_paths(text: &str) -> String {
    let pattern = absolute_path();
    let mut out = String::with_capacity(text.len());
    let mut cursor = 0;
    for found in pattern.find_iter(text) {
        out.push_str(&text[cursor..found.start()]);
        if out.ends_with(ROOT) {
            out.push_str(found.as_str());
        } else {
            out.push_str("<path>");
        }
        cursor = found.end();
    }
    out.push_str(&text[cursor..]);
    out
}

/// Two or more path segments, which is what makes it a path rather than a
/// ratio or a flag.
fn absolute_path() -> &'static regex::Regex {
    use std::sync::OnceLock;
    static PATTERN: OnceLock<regex::Regex> = OnceLock::new();
    PATTERN
        .get_or_init(|| regex::Regex::new(r"(/[A-Za-z0-9._+-]+){2,}").expect("a literal pattern"))
}

/// The pattern set, compiled once.
///
/// **Deliberately the list PLAN.md §3.4 states and nothing more** — absolute
/// paths, the workspace id, timings and pids. An earlier version of this
/// function also normalised anything that looked like a port, which is not on
/// that list and which collapsed `views.py:1234` and `views.py:5678` into one
/// string: two different line numbers, one signature. Each pattern here is a
/// thing that provably differs between two runs of *one* failure; anything
/// broader starts erasing the difference between two *different* ones, and a
/// signature that collides is worse than one that is too specific — an agent
/// told "same as last time" stops reading.
fn patterns() -> &'static [(regex::Regex, &'static str)] {
    use std::sync::OnceLock;
    static PATTERNS: OnceLock<Vec<(regex::Regex, &'static str)>> = OnceLock::new();
    PATTERNS.get_or_init(|| {
        vec![
            // A pid, in the two spellings tools actually use.
            (
                regex::Regex::new(r"\b(pid|PID)[= ]\d+").expect("a literal pattern"),
                "$1=<pid>",
            ),
            // A duration. `1.23s`, `450ms`, `2m 3s`.
            (
                regex::Regex::new(r"\b\d+(\.\d+)?\s?(ms|s|m|us|µs)\b").expect("a literal pattern"),
                "<time>",
            ),
        ]
    })
}

/// The dispatch records and the reducer's event sequence for one run.
///
/// **The event sequence is the second dividend from choosing a reducer.** The
/// first was compile-time exhaustiveness; this is that char already produces a
/// complete ordered account of the run — every lease granted and denied, every
/// spawn, every deadline, every exit — and persisting it gives `explain`
/// something `docker inspect` has no equivalent of: a trace that **replays
/// through `step()`**. It costs one append per event.
#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct Journal {
    /// One per check char actually dispatched, in id order.
    #[serde(skip_serializing_if = "BTreeMap::is_empty", default)]
    pub dispatches: BTreeMap<CheckId, Dispatch>,
    /// Every event the shell fed the reducer, in order.
    #[serde(skip_serializing_if = "Vec::is_empty", default)]
    pub events: Vec<Event>,
}

impl Journal {
    /// Append an event. **Every event, in order** — a sequence with a hole in it
    /// replays to a different state, which is the one assertion this whole
    /// record makes possible.
    pub fn observed(&mut self, event: &Event) {
        self.events.push(event.clone());
    }

    /// Record a dispatch.
    pub fn dispatched(&mut self, dispatch: Dispatch) {
        self.dispatches.insert(dispatch.check.clone(), dispatch);
    }

    /// Complete a dispatch with the signature of how it failed.
    ///
    /// A check that passed gets no signature: the signature answers "is this the
    /// same failure as last time", and a success is not a failure.
    pub fn failed(
        &mut self,
        check: &CheckId,
        exit_code: i32,
        output: &str,
        scrub: &Scrub,
    ) -> Result<(), CharError> {
        match self.dispatches.get_mut(check) {
            Some(dispatch) => {
                dispatch.signature = Some(signature(check, exit_code, output, scrub));
                Ok(())
            }
            // A signature for a check char never dispatched is a char bug rather
            // than a user error: the shell only reaches here from an exit it
            // observed, and it cannot have observed one it never started.
            None => Err(CharError {
                class: crate::error::ErrClass::CharBug,
                r#where: check.to_string(),
                message: format!("a failure was recorded for {check}, which was never dispatched"),
                next_action: None,
            }),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::schedule::{EnvDelta, Plan};

    fn plan() -> Plan {
        let mut env = EnvDelta::default();
        env.set.insert("RAILS_ENV".to_string(), "test".to_string());
        env.secrets = vec!["DB_PASSWORD".to_string()];
        Plan {
            id: CheckId::new("api:test"),
            argv: vec![
                "pytest".to_string(),
                "sub/semi;echo INJECTED.py".to_string(),
            ],
            env,
            files: vec!["sub/semi;echo INJECTED.py".to_string()],
            timeout_ms: 600_000,
            cost: 4,
            exclusives: vec!["browser".to_string()],
            needs: Vec::new(),
            log: Some(".armada/run/01J8X2/logs/api.test.log".to_string()),
            blocked: None,
            skip: None,
        }
    }

    fn scrub() -> Scrub {
        Scrub::new(
            Path::new("/srv/repo"),
            &WorkspaceId::from_stored("a3f91c02"),
        )
    }

    /// **Only recoverable by reimplementing char**, so it is recorded verbatim
    /// — including a filename that is a legal POSIX name and an injection under
    /// a shell. The record's value is authority.
    #[test]
    fn the_record_carries_the_argv_that_actually_ran() {
        let files = vec!["sub/semi;echo INJECTED.py".to_string()];
        let record = Dispatch::new(&plan(), Path::new("/srv/repo"), &files, 1_000);
        assert_eq!(
            record.argv,
            vec!["pytest", "sub/semi;echo INJECTED.py"],
            "the argv was re-derived rather than recorded"
        );
        assert_eq!(record.files, files);
        assert_eq!(record.cwd, Path::new("/srv/repo"));
        assert_eq!(record.dispatched_at_mono, 1_000);
    }

    /// `docker inspect` dumps environment *values*, which is a well-known way
    /// secrets escape. This record carries names.
    #[test]
    fn the_record_carries_environment_names_and_never_a_value() {
        let record = Dispatch::new(&plan(), Path::new("/srv/repo"), &[], 0);
        assert_eq!(record.env, vec!["DB_PASSWORD", "RAILS_ENV"]);

        let json = serde_json::to_string(&record).expect("a record serializes");
        assert!(json.contains("DB_PASSWORD"), "the name is recorded");
        assert!(
            !json.contains("\"test\""),
            "an env value reached the record: {json}"
        );
    }

    /// Point-in-time state that no longer exists — the row PLAN.md §3.4 marks
    /// "No" in the recoverable column.
    #[test]
    fn the_record_names_what_was_waited_on_and_who_held_it() {
        let mut record = Dispatch::new(&plan(), Path::new("/srv/repo"), &[], 0);
        record.holding(LeaseKind::Exclusive, "browser");
        record.holding(LeaseKind::CpuSlot, "0");
        record.waited(
            LeaseKind::Exclusive,
            "browser",
            Some(WorkspaceId::from_stored("7c21ab90")),
            44_000,
        );

        assert_eq!(record.leases, vec!["exclusive:browser", "cpu-slot:0"]);
        assert_eq!(
            record.waited_on,
            vec![Waited {
                kind: "exclusive".to_string(),
                key: "browser".to_string(),
                held_by: Some(WorkspaceId::from_stored("7c21ab90")),
                waited_ms: 44_000,
            }]
        );
    }

    #[test]
    fn holding_the_same_lease_twice_records_it_once() {
        let mut record = Dispatch::new(&plan(), Path::new("/srv/repo"), &[], 0);
        record.holding(LeaseKind::Exclusive, "browser");
        record.holding(LeaseKind::Exclusive, "browser");
        assert_eq!(record.leases.len(), 1);
    }

    // ------------------------------------------------------------ signature

    /// **Deterministic, so two runs of one bug always match.** Everything that
    /// differs between two runs of one failure is normalised away first.
    #[test]
    fn two_runs_of_one_failure_produce_one_signature() {
        let first = "\
            running 3 tests\n\
            pid=4212 started\n\
            FAILED /srv/repo/services/api/views.py:12: assertion failed\n\
            finished in 3.12s\n";
        let second = "\
            running 3 tests\n\
            pid=9981 started\n\
            FAILED /srv/repo/services/api/views.py:12: assertion failed\n\
            finished in 41.9s\n";

        let a = signature(&CheckId::new("api:test"), 1, first, &scrub());
        let b = signature(&CheckId::new("api:test"), 1, second, &scrub());
        assert_eq!(a, b, "a pid and a duration changed the fingerprint");
        assert_eq!(a.digest.len(), 64, "blake3, hex");
    }

    /// The same output on two machines is the same failure. A signature that
    /// disagrees across checkouts makes the history row always read "this is
    /// new", which is the same as having none.
    #[test]
    fn the_same_failure_in_two_checkouts_produces_one_signature() {
        let text = |root: &str| {
            format!(
                "FAILED {root}/services/api/views.py:12: assertion failed in workspace a3f91c02\n"
            )
        };
        let here = Scrub::new(
            Path::new("/srv/repo"),
            &WorkspaceId::from_stored("a3f91c02"),
        );
        let there = Scrub::new(
            Path::new("/home/agent/wt-4"),
            &WorkspaceId::from_stored("7c21ab90"),
        );
        assert_eq!(
            signature(&CheckId::new("api:test"), 1, &text("/srv/repo"), &here),
            signature(
                &CheckId::new("api:test"),
                1,
                &text("/home/agent/wt-4").replace("a3f91c02", "7c21ab90"),
                &there
            )
        );
    }

    /// **A signature that collides is worse than one that is too specific** — an
    /// agent told "same as last time" stops reading. Two genuinely different
    /// failures must not agree.
    #[test]
    fn two_different_failures_produce_two_signatures() {
        let one = signature(
            &CheckId::new("api:test"),
            1,
            "FAILED test_login: assertion failed\n",
            &scrub(),
        );
        let two = signature(
            &CheckId::new("api:test"),
            1,
            "FAILED test_logout: assertion failed\n",
            &scrub(),
        );
        assert_ne!(one, two);

        // The exit code is part of the tuple, so the same output with a
        // different code is a different failure.
        let three = signature(
            &CheckId::new("api:test"),
            2,
            "FAILED test_login: assertion failed\n",
            &scrub(),
        );
        assert_ne!(one, three);

        // And so is the check id, so two checks failing identically stay apart.
        let four = signature(
            &CheckId::new("web:test"),
            1,
            "FAILED test_login: assertion failed\n",
            &scrub(),
        );
        assert_ne!(one, four);
    }

    /// **The defect a mutation found, pinned.** Two different files inside the
    /// workspace are two different failures, and the first version of
    /// `normalise` collapsed both to `<root><path>` because the generic
    /// absolute-path rule ran over the workspace-relative remainder.
    #[test]
    fn two_failures_in_two_files_of_one_workspace_stay_apart() {
        let one = signature(
            &CheckId::new("api:test"),
            1,
            "FAILED /srv/repo/services/api/views.py:12: assertion failed\n",
            &scrub(),
        );
        let two = signature(
            &CheckId::new("api:test"),
            1,
            "FAILED /srv/repo/services/api/models.py:12: assertion failed\n",
            &scrub(),
        );
        assert_ne!(one, two, "two files collapsed into one signature");
    }

    /// The two halves of the asymmetry, read directly rather than through a
    /// digest, so a failure here says which half broke.
    #[test]
    fn a_path_inside_the_workspace_keeps_its_name_and_one_outside_does_not() {
        let normalised = scrub().normalise(
            "File \"/srv/repo/services/api/views.py\", line 12, in handler\n               imported from /opt/homebrew/lib/python3.12/site-packages/thing.py\n",
        );
        assert!(
            normalised.contains("<root>/services/api/views.py"),
            "the actionable path was erased: {normalised}"
        );
        assert!(
            normalised.contains("<path>"),
            "a foreign path survived: {normalised}"
        );
        assert!(
            !normalised.contains("homebrew"),
            "a machine-specific path survived: {normalised}"
        );
    }

    /// A line number is not a port, and PLAN.md §3.4's list does not mention
    /// ports at all. An earlier version normalised anything four or five digits
    /// after a colon, which made two line numbers one signature.
    #[test]
    fn a_line_number_is_not_normalised_away() {
        let one = scrub().normalise("views.py:1234: failed\n");
        let two = scrub().normalise("views.py:5678: failed\n");
        assert_ne!(one, two, "{one} == {two}");
    }

    /// The tail rather than the head: a tool prints its summary last, and a run
    /// that scrolled ten thousand lines of progress is the same failure as one
    /// that did not.
    #[test]
    fn only_the_tail_of_a_long_output_is_hashed() {
        let ending = "FAILED test_login: assertion failed\n";
        let short = format!("{}{ending}", "progress\n".repeat(4));
        let long = format!("{}{ending}", "progress\n".repeat(100_000));

        assert_ne!(
            signature(&CheckId::new("api:test"), 1, &short, &scrub()),
            signature(&CheckId::new("api:test"), 1, &long, &scrub()),
            "four lines of context is inside the window and should count"
        );

        let longer = format!("{}{ending}", "progress\n".repeat(200_000));
        assert_eq!(
            signature(&CheckId::new("api:test"), 1, &long, &scrub()),
            signature(&CheckId::new("api:test"), 1, &longer, &scrub()),
            "beyond the window the two runs are the same failure"
        );
    }

    /// Cutting a tail at a byte offset inside a character is a panic, and a
    /// tool's output is not guaranteed to be ASCII.
    #[test]
    fn a_tail_that_lands_inside_a_character_does_not_panic() {
        let text = "é".repeat(SIGNATURE_TAIL_BYTES);
        let cut = tail_of(&text, SIGNATURE_TAIL_BYTES / 2 + 1);
        assert!(cut.len() <= SIGNATURE_TAIL_BYTES / 2 + 1);
        assert!(cut.chars().all(|c| c == 'é'));
    }

    // -------------------------------------------------------------- journal

    /// Every event, in order. A sequence with a hole in it replays to a
    /// different state, which is the assertion this record exists to make
    /// possible.
    #[test]
    fn the_journal_keeps_every_event_in_the_order_it_arrived() {
        let mut journal = Journal::default();
        let events = [
            Event::Started,
            Event::Tick { now_mono: 1_000 },
            Event::ChildExited {
                check: CheckId::new("api:test"),
                code: 1,
            },
        ];
        for event in &events {
            journal.observed(event);
        }
        assert_eq!(journal.events, events);
    }

    #[test]
    fn a_signature_completes_the_dispatch_it_belongs_to() {
        let mut journal = Journal::default();
        journal.dispatched(Dispatch::new(&plan(), Path::new("/srv/repo"), &[], 0));
        journal
            .failed(&CheckId::new("api:test"), 1, "FAILED\n", &scrub())
            .expect("the check was dispatched");

        let recorded = &journal.dispatches[&CheckId::new("api:test")];
        assert_eq!(recorded.signature.as_ref().unwrap().exit_code, 1);
    }

    /// The shell can only reach here from an exit it observed, and it cannot
    /// have observed one it never started — so this is char's bug, not the
    /// caller's, and `char_bug` is the class that says "stop; retrying will not
    /// help".
    #[test]
    fn a_signature_for_a_check_that_never_ran_is_a_char_bug() {
        let mut journal = Journal::default();
        let error = journal
            .failed(&CheckId::new("ghost:test"), 1, "", &scrub())
            .unwrap_err();
        assert_eq!(error.class, crate::error::ErrClass::CharBug);
        assert_eq!(error.class.exit_code(), 70);
    }
}
