//! What the runner said last about one branch's turn.
//!
//! **A read-modify-write, not a replace.** `scripts/land`'s `say()` merges
//! new fields into whatever an outcome file already held, because different
//! points in a turn set different subsets of it — a `gating` line sets
//! `logs`; a `conflict` sets `conflicts` and keeps the `place` a resubmit
//! must not lose. [`merge_outcome`] is that operation.

use serde::{Deserialize, Serialize};

use super::codec::{self, ReadStateError, WriteStateError};
use super::dir::StateDir;

/// A queue place: nanoseconds, of no fixed epoch here — whoever mints one
/// carries the clock. `i64` rather than `u128`: `scripts/land` reads
/// `time.time_ns()` straight into it, and this fits every value that in
/// practice does too, comfortably past this century.
pub type Place = i64;

/// What one stored outcome says about a branch's turn.
///
/// **Not `refused` and not `unknown`.** `scripts/land`'s exit-code table
/// (`docs/practices/running-locally.md`, "Landing a branch") names ten
/// values; two of them never reach this file. `refused` is raised by
/// `Refused` before a branch joins the line — no outcome exists yet to hold
/// it — and `unknown` is what `--status` says when [`read_outcome`] returns
/// `None`, not a value a stored [`Outcome`] carries. The eight left are
/// exactly this enum.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum OutcomeState {
    Waiting,
    Gating,
    Merging,
    Landed,
    Red,
    Conflict,
    Ungated,
    Stopped,
}

impl OutcomeState {
    /// The exit code `scripts/land`'s `EXIT` table gives this state, whether
    /// read from a fresh turn or from `--status` on an old one.
    pub fn exit_code(self) -> i32 {
        match self {
            OutcomeState::Landed => 0,
            OutcomeState::Waiting | OutcomeState::Gating | OutcomeState::Merging => 3,
            OutcomeState::Red => 4,
            OutcomeState::Conflict => 5,
            OutcomeState::Ungated => 6,
            OutcomeState::Stopped => 7,
        }
    }
}

/// What is known about one branch's most recent turn.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct Outcome {
    pub branch: String,
    pub state: OutcomeState,
    pub detail: String,
    pub updated: String,
    pub runner: u32,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub pr: Option<u64>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub place: Option<Place>,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub logs: Vec<String>,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub failed: Vec<String>,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub already: Vec<String>,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub new_lines: Vec<String>,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub conflicts: Vec<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub pushed: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub gated_base: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub candidate: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub merge_commit: Option<String>,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub cleanup: Vec<String>,
}

impl Outcome {
    fn blank(branch: &str) -> Outcome {
        Outcome {
            branch: branch.to_string(),
            // Overwritten unconditionally below, in every caller: a blank
            // outcome is never read before `merge_outcome` sets its own.
            state: OutcomeState::Waiting,
            detail: String::new(),
            updated: String::new(),
            runner: 0,
            pr: None,
            place: None,
            logs: Vec::new(),
            failed: Vec::new(),
            already: Vec::new(),
            new_lines: Vec::new(),
            conflicts: Vec::new(),
            pushed: None,
            gated_base: None,
            candidate: None,
            merge_commit: None,
            cleanup: Vec::new(),
        }
    }
}

/// The fields one call site sets. `None` leaves whatever an existing outcome
/// already held; `Some` — including an empty collection — overwrites it, the
/// same distinction Python's `held.update(extra)` draws by which keyword
/// arguments a call actually passed.
#[derive(Clone, Debug, Default)]
pub struct OutcomePatch {
    pub pr: Option<u64>,
    pub place: Option<Place>,
    pub logs: Option<Vec<String>>,
    pub failed: Option<Vec<String>>,
    pub already: Option<Vec<String>>,
    pub new_lines: Option<Vec<String>>,
    pub conflicts: Option<Vec<String>>,
    pub pushed: Option<String>,
    pub gated_base: Option<String>,
    pub candidate: Option<String>,
    pub merge_commit: Option<String>,
    pub cleanup: Option<Vec<String>>,
}

pub fn read_outcome(dir: &StateDir, branch: &str) -> Result<Option<Outcome>, ReadStateError> {
    codec::read("outcome", &dir.outcome_path(branch))
}

/// Read the outcome on disk, apply `patch` over it, set the fields every
/// call sets fresh, and write the result back.
///
/// `updated` is taken as an argument rather than read here: this crate
/// reads no clock of its own — `crates/fleet/src/clock.rs`'s own doc names
/// itself the one place that does, and a later stage wires its
/// [`fleet::Clock`] reading down to this call.
pub fn merge_outcome(
    dir: &StateDir,
    branch: &str,
    state: OutcomeState,
    detail: impl Into<String>,
    updated: &str,
    patch: OutcomePatch,
) -> Result<Outcome, MergeOutcomeError> {
    let mut merged = read_outcome(dir, branch)
        .map_err(MergeOutcomeError::Read)?
        .unwrap_or_else(|| Outcome::blank(branch));

    merged.pr = patch.pr.or(merged.pr);
    merged.place = patch.place.or(merged.place);
    merged.logs = patch.logs.unwrap_or(merged.logs);
    merged.failed = patch.failed.unwrap_or(merged.failed);
    merged.already = patch.already.unwrap_or(merged.already);
    merged.new_lines = patch.new_lines.unwrap_or(merged.new_lines);
    merged.conflicts = patch.conflicts.unwrap_or(merged.conflicts);
    merged.pushed = patch.pushed.or(merged.pushed);
    merged.gated_base = patch.gated_base.or(merged.gated_base);
    merged.candidate = patch.candidate.or(merged.candidate);
    merged.merge_commit = patch.merge_commit.or(merged.merge_commit);
    merged.cleanup = patch.cleanup.unwrap_or(merged.cleanup);

    merged.branch = branch.to_string();
    merged.state = state;
    merged.detail = detail.into();
    merged.updated = updated.to_string();
    merged.runner = std::process::id();

    codec::write(&dir.outcome_path(branch), &merged).map_err(MergeOutcomeError::Write)?;
    Ok(merged)
}

#[derive(Debug)]
pub enum MergeOutcomeError {
    Read(ReadStateError),
    Write(WriteStateError),
}

impl std::fmt::Display for MergeOutcomeError {
    fn fmt(&self, out: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            MergeOutcomeError::Read(why) => write!(out, "{why}"),
            MergeOutcomeError::Write(why) => write!(out, "{why}"),
        }
    }
}

impl std::error::Error for MergeOutcomeError {
    fn source(&self) -> Option<&(dyn std::error::Error + 'static)> {
        match self {
            MergeOutcomeError::Read(why) => Some(why),
            MergeOutcomeError::Write(why) => Some(why),
        }
    }
}
