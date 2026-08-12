//! The run: its id, where it writes, and which old ones get reaped.
//!
//! ```text
//! .char/
//!   run/<run-id>/
//!     state.json                    per-check status, verdict, and the
//!                                   dispatch record §3.4 reads
//!     logs/<component>.<check>.log  one per check
//! ```
//!
//! **Nothing reclaimable lives here** (PLAN.md §4.2). A workspace directory is
//! deleted by `rm -rf` or `git worktree remove`, neither of which consults char,
//! so anything recorded only here is gone precisely when it is most needed. Run
//! artifacts are safe because a run without its workspace is meaningless anyway
//! — which is exactly why the port block, the owned rows and the leases are in
//! `~/.char/char.db` instead.
//!
//! **Log growth is a separate problem with a separate answer.** Coupling
//! retention to `char clean` would mean either logs live forever or you lose the
//! evidence from a failed run the moment you release a port. So old run
//! directories are reaped at the *start* of each run, keeping the most recent N
//! and never touching one whose run lease is live.

use crate::error::{CharError, ErrClass};
use crate::id::WorkspaceId;
use crate::schedule::{CheckId, State};
use serde::Serialize;
use std::fmt;

/// Crockford's base32 alphabet.
///
/// It excludes `I`, `L`, `O` and `U`: the first three because they are read as
/// `1`, `1` and `0`, and `U` so that no id spells an English obscenity by
/// accident. char ids are copied out of terminals and pasted into
/// `char explain --run`, which is the case the alphabet was designed for.
const ALPHABET: &[u8; 32] = b"0123456789ABCDEFGHJKMNPQRSTVWXYZ";

/// Characters encoding the timestamp: 48 bits of milliseconds, which lasts
/// until the year 10889.
const TIME_LEN: usize = 10;

/// Characters of per-process entropy.
const ENTROPY_LEN: usize = 6;

/// Total length of a run id.
pub const RUN_ID_LEN: usize = TIME_LEN + ENTROPY_LEN;

/// A run's id.
///
/// **Time-ordered, so lexicographic order is chronological order.** That is not
/// decoration: retention keeps "the most recent N", and the cheapest correct
/// implementation of that is sorting the directory names. An id that did not
/// sort would need every run's mtime read back off a filesystem that may have
/// been restored from a backup or synced from another machine.
///
/// **What phase 3 decided, because the corpus specifies the shape and not the
/// format.** PLAN.md writes `01J8X2` throughout — in `data.run_id`, in
/// `.char/run/01J8X2/logs/`, in `char explain --run 01J8X2` — and those six
/// characters are exactly the leading edge of a time-ordered base32 id, so the
/// format here is the illustration made real rather than a departure from it.
/// The remaining characters are what stop two runs a millisecond apart from
/// being one directory.
#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize)]
#[serde(transparent)]
pub struct RunId(String);

impl RunId {
    /// Mint an id from a wall-clock reading and this process's entropy.
    ///
    /// **Pure, and that is why the two inputs are arguments.** A clock and a
    /// random source are both I/O; the encoding is a decision. The shell reads
    /// `Clock::wall_ms` and mixes its own entropy, and a test mints a fixed id
    /// by passing fixed numbers — which is what lets a golden snapshot of a run
    /// payload exist at all.
    ///
    /// Wall clock rather than monotonic, deliberately: a monotonic reading is
    /// meaningless across a reboot, and these ids have to sort against the ones
    /// already on disk from last week.
    pub fn mint(wall_ms: u64, entropy: u64) -> Self {
        let mut out = String::with_capacity(RUN_ID_LEN);
        // 48 bits, most significant character first, so the string sorts the
        // way the number does.
        for position in (0..TIME_LEN).rev() {
            let index = (wall_ms >> (position * 5)) & 0x1f;
            out.push(ALPHABET[index as usize] as char);
        }
        for position in (0..ENTROPY_LEN).rev() {
            let index = (entropy >> (position * 5)) & 0x1f;
            out.push(ALPHABET[index as usize] as char);
        }
        RunId(out)
    }

    /// Adopt an id read back off disk or out of `CHAR_RUN_ID`.
    ///
    /// **Validated rather than trusted, because the id becomes a path.**
    /// `CHAR_RUN_ID` arrives from the environment (PLAN.md §2.4) and a child
    /// may set it to anything at all; a value of `../../etc` reaching
    /// `.char/run/<id>/` is a directory traversal in the one variable char
    /// promises to set for every process it spawns.
    pub fn parse(text: &str) -> Result<Self, CharError> {
        let bad = |message: &str| CharError {
            class: ErrClass::BadInvocation,
            r#where: "run-id".to_string(),
            message: message.to_string(),
            next_action: Some("`char status` lists the runs this workspace has kept".to_string()),
        };
        if text.len() != RUN_ID_LEN {
            return Err(bad(&format!(
                "a run id is {RUN_ID_LEN} characters; this one is {}",
                text.len()
            )));
        }
        if !text.bytes().all(|b| ALPHABET.contains(&b)) {
            return Err(bad("a run id is Crockford base32, in upper case"));
        }
        Ok(RunId(text.to_string()))
    }

    /// The id as written.
    pub fn as_str(&self) -> &str {
        &self.0
    }

    /// The milliseconds this id encodes.
    pub fn wall_ms(&self) -> u64 {
        self.0.bytes().take(TIME_LEN).fold(0u64, |acc, b| {
            let index = ALPHABET.iter().position(|a| *a == b).unwrap_or(0) as u64;
            (acc << 5) | index
        })
    }
}

impl fmt::Display for RunId {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(&self.0)
    }
}

/// The log file one check writes to, relative to the run directory.
///
/// `<component>.<check>.log`, from the derived id `<component>:<check>`
/// (PLAN.md §4.2). The colon becomes a dot because a colon in a filename is
/// legal on POSIX and a nuisance everywhere else — it is a path separator in
/// some tooling and needs quoting in most shells, and these paths are printed
/// for a human to open.
pub fn log_name(check: &CheckId) -> String {
    format!("{}.log", check.as_str().replace(':', "."))
}

/// Which retained run directories to remove, in the order they are removed.
///
/// **Keep the most recent `retention`, and never touch a live one.** The live
/// set is not a nicety: `run_retention` defaults to 10 and a workspace that has
/// churned through ten runs since a detached one started would otherwise delete
/// the directory that run is still writing to — and PLAN.md §2.3.1 measures what
/// that looks like from inside, which is that writes to the already-open fd
/// **succeed silently** into an unlinked inode. The run keeps going and its logs
/// go nowhere.
///
/// A live run is kept *and* counts toward the retention budget, so the answer
/// never depends on the order the two rules are applied in.
pub fn runs_to_reap(present: &[RunId], retention: u32, live: &[RunId]) -> Vec<RunId> {
    let mut sorted: Vec<&RunId> = present.iter().collect();
    // Newest first. Ids are time-ordered, which is the whole reason the format
    // is what it is.
    sorted.sort_by(|a, b| b.cmp(a));
    sorted
        .into_iter()
        .skip(retention as usize)
        .filter(|id| !live.contains(id))
        .cloned()
        .collect()
}

/// What `.char/run/<run-id>/state.json` holds.
///
/// **Written when the check runs, because most of it cannot be recovered
/// afterwards** (PLAN.md §3.4). `char explain` in phase 5 is a reader with
/// nothing to read without it: query `char.db` an hour later and it truthfully
/// reports who holds the browser *now*, which is a different and useless answer
/// to "what was `web:e2e` waiting on when it timed out".
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct RunRecord {
    /// The same global version the `--json` envelope carries, so one number
    /// covers the whole CLI contract.
    pub schema_version: u32,
    /// This run.
    pub run_id: RunId,
    /// The workspace it belongs to.
    pub workspace: WorkspaceId,
    /// Wall clock, RFC 3339. **Only ever displayed** — never compared, because
    /// a backwards NTP step would make it lie about ordering, which is what the
    /// monotonic readings inside `state` are for.
    pub started_at: String,
    /// The scheduler's state, which is the run.
    pub state: State,
}

impl RunRecord {
    /// A record for a run that is starting.
    pub fn new(run_id: RunId, workspace: WorkspaceId, started_at: String, state: State) -> Self {
        RunRecord {
            schema_version: crate::envelope::SCHEMA_VERSION,
            run_id,
            workspace,
            started_at,
            state,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::path::PathBuf;

    fn id(ms: u64, entropy: u64) -> RunId {
        RunId::mint(ms, entropy)
    }

    #[test]
    fn a_run_id_is_sixteen_crockford_characters() {
        let run = id(1_786_000_000_000, 0x3f_ffff);
        assert_eq!(run.as_str().len(), RUN_ID_LEN);
        assert!(
            run.as_str().bytes().all(|b| ALPHABET.contains(&b)),
            "{run} left the alphabet"
        );
        assert!(
            !run.as_str().contains(['I', 'L', 'O', 'U']),
            "{run} contains a character Crockford excludes"
        );
        // Without this the test passes over a mint that returns one constant,
        // which is 16 characters of the alphabet too.
        assert_ne!(id(1_786_000_000_000, 1), id(1_786_000_000_000, 2));
        assert_ne!(id(1_786_000_000_000, 1), id(1_786_000_000_001, 1));
    }

    /// **Lexicographic order is chronological order**, which is what makes
    /// retention a sort rather than a filesystem stat of every directory.
    #[test]
    fn later_runs_sort_after_earlier_ones_however_the_entropy_falls() {
        let earlier = id(1_786_000_000_000, u64::MAX);
        let later = id(1_786_000_000_001, 0);
        assert!(earlier < later, "{earlier} did not sort before {later}");

        let mut ids = [
            id(1_786_000_000_002, 7),
            id(1_786_000_000_000, 7),
            id(1_786_000_000_001, 7),
        ];
        ids.sort();
        let times: Vec<u64> = ids.iter().map(RunId::wall_ms).collect();
        assert_eq!(
            times,
            vec![1_786_000_000_000, 1_786_000_000_001, 1_786_000_000_002]
        );
    }

    /// PLAN.md writes `01J8X2` throughout, and those six characters are the
    /// leading edge of a time-ordered base32 id rather than a different format.
    #[test]
    fn the_documents_own_example_is_the_leading_edge_of_a_real_id() {
        // 2024-09-20T00:00:00Z, the era the corpus was written in.
        let run = id(1_726_790_400_000, 0);
        assert!(
            run.as_str().starts_with("01J"),
            "{run} should begin the way PLAN.md's examples do"
        );
        // And the era this is being written in has moved on by one character,
        // which is why a fresh id does not read identically to the document.
        let now = id(1_786_000_000_000, 0);
        assert!(now.as_str().starts_with("01K"), "{now}");
    }

    #[test]
    fn a_minted_id_round_trips_through_parse() {
        let run = id(1_786_000_000_000, 12_345);
        assert_eq!(RunId::parse(run.as_str()).unwrap(), run);
        // The reading survives the encoding, which is what the sort depends on.
        assert_eq!(run.wall_ms(), 1_786_000_000_000);
        assert_eq!(RunId::parse(run.as_str()).unwrap().wall_ms(), run.wall_ms());
    }

    /// **`CHAR_RUN_ID` arrives from the environment and becomes a path.** A
    /// child may set it to anything, and `../../etc` reaching `.char/run/<id>/`
    /// is a traversal in the one variable char promises to set on every process
    /// it spawns.
    #[test]
    fn a_run_id_that_would_escape_the_run_directory_is_refused() {
        for hostile in [
            "../../etc/passwd",
            "..",
            "/absolute/path0",
            "0123456789ABCDE/",
            "0123456789abcdef",
            "0123456789ABCDEI",
            "",
            "01J8X2",
        ] {
            let refused = RunId::parse(hostile);
            assert!(refused.is_err(), "`{hostile}` was accepted as a run id");
            assert_eq!(
                refused.unwrap_err().class,
                ErrClass::BadInvocation,
                "`{hostile}`"
            );
        }
    }

    #[test]
    fn a_check_writes_to_a_log_named_for_its_component_and_check() {
        assert_eq!(log_name(&CheckId::new("api:lint")), "api.lint.log");
        assert_eq!(log_name(&CheckId::new("web:e2e")), "web.e2e.log");
    }

    #[test]
    fn retention_keeps_the_most_recent_and_reaps_the_rest() {
        let runs: Vec<RunId> = (0..12).map(|n| id(1_786_000_000_000 + n, 0)).collect();
        let reaped = runs_to_reap(&runs, 10, &[]);
        assert_eq!(reaped.len(), 2);
        // The two oldest, and nothing else.
        assert!(reaped.contains(&runs[0]) && reaped.contains(&runs[1]));
        assert!(!reaped.contains(&runs[11]));
    }

    #[test]
    fn nothing_is_reaped_before_the_retention_count_is_reached() {
        let runs: Vec<RunId> = (0..3).map(|n| id(1_786_000_000_000 + n, 0)).collect();
        assert!(runs_to_reap(&runs, 10, &[]).is_empty());
        assert!(runs_to_reap(&[], 10, &[]).is_empty());
        // The other side of the same boundary, without which this passes over a
        // reaper that never reaps anything.
        assert_eq!(runs_to_reap(&runs, 2, &[]), vec![runs[0].clone()]);
    }

    /// **The measured reason this rule exists**: writes to an already-open log
    /// fd succeed silently into an unlinked inode, so deleting a live run's
    /// directory does not fail — the run keeps going and its logs go nowhere.
    #[test]
    fn a_live_run_is_never_reaped_however_old_it_is() {
        let oldest = id(1_786_000_000_000, 0);
        let mut runs = vec![oldest.clone()];
        runs.extend((1..12).map(|n| id(1_786_000_000_000 + n, 0)));

        let reaped = runs_to_reap(&runs, 10, std::slice::from_ref(&oldest));
        assert!(
            !reaped.contains(&oldest),
            "the live run was reaped: {reaped:?}"
        );
        assert_eq!(reaped.len(), 1, "the other over-count run still goes");
    }

    /// Retention is a count of *runs*, and a live one is one of them — so the
    /// answer does not depend on which rule is applied first.
    #[test]
    fn a_live_run_still_counts_toward_the_retention_budget() {
        let runs: Vec<RunId> = (0..12).map(|n| id(1_786_000_000_000 + n, 0)).collect();
        let newest = runs[11].clone();
        assert_eq!(
            runs_to_reap(&runs, 10, &[newest]).len(),
            2,
            "keeping the live run in addition to ten would keep eleven"
        );
    }

    #[test]
    fn a_run_record_carries_the_one_schema_version() {
        let record = RunRecord::new(
            id(1_786_000_000_000, 1),
            WorkspaceId::from_stored("a3f91c02"),
            "2026-08-11T14:02:11Z".to_string(),
            State::new(PathBuf::from("/srv/repo"), 6, Vec::new()),
        );
        assert_eq!(record.schema_version, crate::envelope::SCHEMA_VERSION);
        let json = serde_json::to_string(&record).expect("a run record serializes");
        assert!(json.contains("\"schema_version\":1"), "{json}");
        assert!(json.contains("\"workspace\":\"a3f91c02\""), "{json}");
    }
}
