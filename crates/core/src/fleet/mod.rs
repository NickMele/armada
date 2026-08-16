//! Fleet's pure decisions: the two enums it owns, the Job record, the
//! workflow document, and every argv it builds (PLAN.md §14).
//!
//! **A Job is durable and a Drone is not.** A Job is a uuid, a git worktree, a
//! port block, a transcript, a budget and — when it finishes — a verdict; a
//! Drone is the process executing it. Everything a Job *is* survives on disk,
//! which is why a Drone that exits, crashes or is killed does not end it
//! (PLAN.md §14.1).
//!
//! Nothing here spawns, writes or reads a file. The shell that does is
//! `armada-fleet`; what lives here is the part a test can call with values and
//! get values back — including **every argv**, because argv is where the bugs
//! are (`ARCHITECTURE.md` §1.1) and because a Drone's argv is the one thing no
//! test may execute for real (PHASES.md §8.5).

pub mod advance;
pub mod bridge;
pub mod classify;
pub mod drone;
pub mod gate;
pub mod job;
pub mod probe;
pub mod workflow;

use serde::{Deserialize, Serialize};
use std::fmt;

/// What a Job is doing right now.
///
/// **Fleet's own enum, and not Manifest's [`Status`](crate::error::Status)**
/// (PLAN.md §14.3). `BLOCKED` is a legal Job state and deliberately not a
/// Manifest terminal state: exit codes are `f(error.class)`, a blocked run
/// carries no class, and a merge gate would therefore read exit `0` as success.
/// The constraint applies to one enum and not the other, which is exactly why
/// they are separate.
///
/// **One spelling, everywhere, and it is the JSON spelling** — SCREAMING in the
/// payload and in the human render, `FAILED` and never `FAIL`.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "SCREAMING_SNAKE_CASE")]
pub enum JobState {
    /// Minted, with nothing started yet.
    Queued,
    /// Work is in flight. **Not the same as "a Drone is alive"** — a Job with no
    /// live Drone is the ordinary resting state between turns (PLAN.md §14.1).
    Running,
    /// Deliberately held: it asked something and is waiting on an answer.
    Paused,
    /// **An observation rather than a state the Drone reports.** A Job is
    /// stalled when **its Drone is gone and nothing has ticked it** — the one
    /// condition a busy Drone cannot self-report, which is why it belongs to
    /// the observer.
    ///
    /// **This is the word that was missing for eight hours** (`020` §6). A Job
    /// whose Drone exited and whose `Stop` hook never relayed the event drew as
    /// `RUNNING`, which is the same word a Job with a live Drone gets — so a
    /// dead Job and a working one were indistinguishable on the screen.
    Stalled,
    /// **A Drone ended its exchange having said nothing through its tools**
    /// (`020` §6) — no verdict, no progress note, no question in the inbox.
    ///
    /// **Not the same failure as [`JobState::Stalled`], and that is why it is a
    /// second word.** A stalled Job produced a signal nothing carried; a silent
    /// one produced no signal at all, so whatever it decided exists only in a
    /// transcript nothing reads (PLAN.md §15.2 forbids reading it). The two
    /// need different things done about them: a stall wants the relay fixed, a
    /// silence wants the Drone's brief looked at.
    Silent,
    /// It cannot proceed without an external change.
    Blocked,
    /// Stopped from outside — `armada fleet kill`.
    Aborted,
    /// It reached a verdict.
    Done,
}

impl JobState {
    /// Every state, in the order `glossary.md` lists them.
    ///
    /// **A named set rather than a hand-typed list per caller.** The Bridge's
    /// `--filter state=` refuses a word that is not one of these, and a list
    /// that lived at that call site would go stale the first time an eighth
    /// state is added — which is the same argument that makes every `match` on
    /// this enum exhaustive.
    pub const ALL: [JobState; 8] = [
        JobState::Queued,
        JobState::Running,
        JobState::Paused,
        JobState::Stalled,
        JobState::Silent,
        JobState::Blocked,
        JobState::Aborted,
        JobState::Done,
    ];

    /// The word, in both audiences.
    pub const fn word(self) -> &'static str {
        match self {
            JobState::Queued => "QUEUED",
            JobState::Running => "RUNNING",
            JobState::Paused => "PAUSED",
            JobState::Stalled => "STALLED",
            JobState::Silent => "SILENT",
            JobState::Blocked => "BLOCKED",
            JobState::Aborted => "ABORTED",
            JobState::Done => "DONE",
        }
    }

    /// Whether this Job is finished with, for `ls`'s default lens and for
    /// `kill --all-finished`.
    pub const fn is_over(self) -> bool {
        matches!(self, JobState::Done | JobState::Aborted)
    }

    /// Whether this Job is waiting on a person.
    ///
    /// **`STALLED` is not in this set.** A stalled Job needs somebody to *look*,
    /// which is what the table already says; needing an *answer* is a claim the
    /// inbox backs with an entry, and conflating the two is how "needs my
    /// attention" becomes noise (PLAN.md §15.4).
    /// **`SILENT` is not in this set either**, for the same reason: it is a
    /// thing to look at, not a question with an entry behind it. A Drone that
    /// said nothing asked nothing.
    pub const fn needs_a_person(self) -> bool {
        matches!(self, JobState::Blocked | JobState::Paused)
    }

    /// What a bulk reap does with a Job in this state
    /// (`commands/fleet/reap.md`).
    ///
    /// **A state you might still act on is not garbage**, and that one sentence
    /// decides every row of the match. `DONE` and `ABORTED` are finished and
    /// nothing further will happen to them. `STALLED` and `BLOCKED` are the two
    /// a person may still push along — a stalled Job is one whose Drone died,
    /// and starting it again is a reasonable next move rather than a reason to
    /// delete the branch it was working on. `PAUSED` is the strongest case of
    /// all: it *means* needs-you, so reaping it by default would delete the very
    /// Jobs the fleet is asking about.
    ///
    /// **`RUNNING` is never offered**, because the observation is what produces
    /// it: a Job only observes as `RUNNING` while its process group is provably
    /// still Armada's ([`crate::fleet::job::observe`]). A record that *says*
    /// `RUNNING` with a dead Drone observes as `STALLED` or `PAUSED` and is
    /// offered by those rows — which is what makes the leaked port block of a
    /// Job nobody noticed had died reclaimable at all.
    ///
    /// **`QUEUED` is never offered either**, for the opposite reason: it has
    /// spent nothing, holds nothing and is about to start.
    pub const fn reaping(self) -> Reaping {
        match self {
            JobState::Done | JobState::Aborted => Reaping::Taken,
            // `SILENT` joins the offered rows for `STALLED`'s reason exactly:
            // its Drone is gone, it holds a worktree and a port block, and
            // starting it again is a reasonable next move rather than a reason
            // to delete the branch it was working on.
            JobState::Stalled | JobState::Silent | JobState::Blocked | JobState::Paused => {
                Reaping::Offered
            }
            JobState::Running | JobState::Queued => Reaping::Never,
        }
    }
}

/// What a Job is **in the middle of**, while a verb with a duration runs
/// (`020` §5).
///
/// # The bug this is the state machine for
///
/// A Job was aborted, `y` was pressed, and the screen said nothing for several
/// seconds while `armada manifest clean` talked to docker. **The abort
/// succeeded** — and a working abort and a hung one looked identical, because
/// neither of them said anything at all.
///
/// **A status word rather than a spinner**, which is the decision `020` §5
/// takes: the row already has a status column, an action with a duration is a
/// state the Job is genuinely in, and a spinner would be a second vocabulary
/// for the same fact. What makes it legible is naming the slow part beside it
/// — `docker 12s…` — because *which* of the three or four things an abort does
/// is the part that is taking the time is the only thing the reader wants.
///
/// **These are not [`JobState`]s**, deliberately. A Job being aborted is still
/// `RUNNING` on disk until the abort finishes; folding the two into one enum
/// would mean a crash halfway through an abort left a record claiming a state
/// no verb ever reached. This is a *transient* laid over the durable one, and
/// [`crate::fleet::job::Job::status_word`] is where the two are composed.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "SCREAMING_SNAKE_CASE")]
pub enum Acting {
    /// `armada fleet kill` — stopping the Drone and releasing what it held.
    Aborting,
    /// `armada fleet reap` — the same, in bulk.
    Reaping,
    /// `armada fleet pause` — stopping the Drone and leaving everything else.
    Pausing,
}

impl Acting {
    /// Every one, so a render can be asserted over the set.
    pub const ALL: [Acting; 3] = [Acting::Aborting, Acting::Reaping, Acting::Pausing];

    /// The word, in both audiences and in the payload.
    pub const fn word(self) -> &'static str {
        match self {
            Acting::Aborting => "ABORTING",
            Acting::Reaping => "REAPING",
            Acting::Pausing => "PAUSING",
        }
    }
}

impl fmt::Display for Acting {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(self.word())
    }
}

/// What a bulk reap does with one Job, before anybody has toggled anything.
///
/// **Three answers rather than two**, because the middle one is the feature: a
/// preview that only showed what it was about to take would hide the Jobs whose
/// cost of *not* reaping — a port block held, a worktree on disk — is exactly
/// what a person is deciding about.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Reaping {
    /// Not listed at all.
    Never,
    /// Listed, and left for a person to turn on.
    Offered,
    /// Listed, and taken unless a person turns it off.
    Taken,
}

impl Reaping {
    /// Whether a reap lists this Job at all.
    pub const fn is_offered(self) -> bool {
        !matches!(self, Reaping::Never)
    }

    /// Whether a reap takes it with nothing said.
    pub const fn is_default(self) -> bool {
        matches!(self, Reaping::Taken)
    }
}

impl fmt::Display for JobState {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(self.word())
    }
}

/// How a step ended (PLAN.md §14.3).
///
/// A verdict answers *how did the step end*. It is not the same question as
/// *what is this Job doing right now*, and conflating them is what produced four
/// competing status vocabularies in earlier drafts.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "SCREAMING_SNAKE_CASE")]
pub enum Verdict {
    /// Advance to the next step.
    Pass,
    /// Retry the same step with the evidence attached, until a ceiling stops it.
    Failed,
    /// Cannot proceed without an external change. Stop, record why, raise it.
    Blocked,
    /// A judgement call is yours — or a ceiling was reached.
    NeedsHuman,
}

/// What a Drone's proposal is **about**
/// ([`docs/reserved/008`](../../../../docs/reserved/008-armada-injects-its-own-skills.md)).
///
/// # `008`'s open question, answered: both, and the difference is written down
///
/// *"May a Drone propose a guild change, or only a manifest one?"* A guild is
/// **the user** — their voice, their workflows, what they are willing to let an
/// unattended agent do. A manifest is a repository's description of itself. The
/// blast radius of an agent editing those two is not remotely the same.
///
/// **But nothing here edits either.** A proposal writes one inbox entry and
/// changes nothing at all, so the question *may it* collapses into *is it worth
/// reading* — and a Drone that has noticed the same workflow step fail in three
/// repositories has noticed something about the person. Refusing that outright
/// would lose it to keep a distinction the mechanism does not need.
///
/// So both are allowed, the asymmetry lives in
/// [`crate::skill::BODY`] — which tells a Drone to be sparing with `guild`,
/// because it is looking at one repository — and the enforcement is that
/// **applying a proposal is somebody else's verb**. Armada verifies; it does not
/// take an agent's word.
///
/// # Two, and a third is refused rather than guessed at
///
/// The same rule `Verdict` follows: a word outside the set is an error, not a
/// nearest match. A proposal filed against a subject Armada has no concept of is
/// a row nobody can act on, which is the whole complaint
/// `docs/reserved/005-inbox-label-not-identity.md` was raised about.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum Subject {
    /// This repository's `armada.yml` — a command, a check, a service, a port.
    Manifest,
    /// The person's own guild — a standing preference, a workflow step.
    Guild,
}

impl Subject {
    /// The word, as it is written and as it is read.
    ///
    /// **Lower case, unlike every status word in this module, and deliberately.**
    /// `PASS` and `NEEDS_HUMAN` scream because they are *states* a table draws in
    /// a STATUS column; this is a noun inside a sentence a person reads — *"a
    /// Drone proposes a change to the manifest"* — and a shouted noun mid-prose
    /// reads as a state that does not exist.
    pub const fn word(self) -> &'static str {
        match self {
            Subject::Manifest => "manifest",
            Subject::Guild => "guild",
        }
    }

    /// The word a caller wrote, as the enum.
    ///
    /// Case-insensitive for [`crate::fleet::Verdict`]'s reason — a model writing
    /// `Manifest` meant `manifest`, and sending it round a loop over
    /// capitalisation costs a turn to teach it nothing.
    pub fn parse(word: &str) -> Option<Subject> {
        match word.trim().to_ascii_lowercase().as_str() {
            "manifest" => Some(Subject::Manifest),
            "guild" => Some(Subject::Guild),
            _ => None,
        }
    }
}

impl fmt::Display for Subject {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(self.word())
    }
}

impl Verdict {
    /// The word, in both audiences. `NEEDS_HUMAN` carries its underscore into
    /// the render, because the payload spells it that way and one spelling is
    /// the rule.
    pub const fn word(self) -> &'static str {
        match self {
            Verdict::Pass => "PASS",
            Verdict::Failed => "FAILED",
            Verdict::Blocked => "BLOCKED",
            Verdict::NeedsHuman => "NEEDS_HUMAN",
        }
    }

    /// The Job state a verdict leaves the Job in.
    pub const fn settles_to(self) -> JobState {
        match self {
            // `PASS` advances rather than ends: the workflow decides whether
            // there is a next step, and `DONE` is that decision's outcome and
            // not this one's.
            Verdict::Pass => JobState::Running,
            Verdict::Failed => JobState::Running,
            Verdict::Blocked => JobState::Blocked,
            Verdict::NeedsHuman => JobState::Paused,
        }
    }
}

impl fmt::Display for Verdict {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(self.word())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// **The rule the three enums share** (PLAN.md §14.3, `glossary.md`): one
    /// spelling everywhere, and it is the JSON spelling. `crates/core/src/error.rs`
    /// enforces it for Manifest's `Status`; these two inherit it rather than
    /// reinventing the `FAIL`/`FAILED` split that had to be removed once already.
    #[test]
    fn every_job_state_is_spelled_the_same_in_both_audiences() {
        for (state, word) in [
            (JobState::Queued, "QUEUED"),
            (JobState::Running, "RUNNING"),
            (JobState::Paused, "PAUSED"),
            (JobState::Stalled, "STALLED"),
            (JobState::Silent, "SILENT"),
            (JobState::Blocked, "BLOCKED"),
            (JobState::Aborted, "ABORTED"),
            (JobState::Done, "DONE"),
        ] {
            assert_eq!(state.word(), word);
            assert_eq!(state.to_string(), word);
            assert_eq!(
                serde_json::to_string(&state).unwrap(),
                format!("\"{word}\"")
            );
        }
    }

    /// `FAILED`, never `FAIL` — and `NEEDS_HUMAN` keeps its underscore on the
    /// screen, because the payload has one.
    #[test]
    fn every_verdict_is_spelled_the_same_in_both_audiences() {
        for (verdict, word) in [
            (Verdict::Pass, "PASS"),
            (Verdict::Failed, "FAILED"),
            (Verdict::Blocked, "BLOCKED"),
            (Verdict::NeedsHuman, "NEEDS_HUMAN"),
        ] {
            assert_eq!(verdict.word(), word);
            assert_eq!(verdict.to_string(), word);
            assert_eq!(
                serde_json::to_string(&verdict).unwrap(),
                format!("\"{word}\"")
            );
        }
    }

    /// A Job state read back out of a record on disk is the same state that was
    /// written. The index survives a reboot, so this is the round trip that has
    /// to hold.
    #[test]
    fn a_job_state_round_trips_through_the_index() {
        for state in JobState::ALL {
            let json = serde_json::to_string(&state).unwrap();
            let back: JobState = serde_json::from_str(&json).unwrap();
            assert_eq!(back, state);
        }
    }

    /// **The three action words, in the one spelling.** They are drawn in a
    /// status column beside [`JobState`]'s words, so they obey the same rule.
    #[test]
    fn every_acting_word_is_spelled_the_same_in_both_audiences() {
        for (acting, word) in [
            (Acting::Aborting, "ABORTING"),
            (Acting::Reaping, "REAPING"),
            (Acting::Pausing, "PAUSING"),
        ] {
            assert_eq!(acting.word(), word);
            assert_eq!(acting.to_string(), word);
            assert_eq!(
                serde_json::to_string(&acting).unwrap(),
                format!("\"{word}\"")
            );
        }
        assert_eq!(Acting::ALL.len(), 3);
    }

    /// **An action word is never a Job state.** They are drawn in the same
    /// column and they mean different things — one is what a Job *is*, the
    /// other is what somebody is doing *to* it — so a reader who learns one
    /// vocabulary must not find a word from the other in it.
    #[test]
    fn no_action_word_collides_with_a_job_state() {
        for acting in Acting::ALL {
            assert!(
                !JobState::ALL.iter().any(|s| s.word() == acting.word()),
                "`{acting}` is also a Job state"
            );
        }
    }

    /// **`STALLED` does not claim to need an answer.** It needs somebody to
    /// look, which the table already says; a Job that needs an answer has an
    /// inbox entry backing the claim.
    #[test]
    fn only_a_blocked_or_paused_job_claims_to_need_a_person() {
        assert!(JobState::Blocked.needs_a_person());
        assert!(JobState::Paused.needs_a_person());
        for state in [
            JobState::Queued,
            JobState::Running,
            JobState::Stalled,
            JobState::Silent,
            JobState::Aborted,
            JobState::Done,
        ] {
            assert!(!state.needs_a_person(), "{state} claimed to need a person");
        }
    }

    #[test]
    fn only_done_and_aborted_are_over() {
        assert!(JobState::Done.is_over());
        assert!(JobState::Aborted.is_over());
        for state in [
            JobState::Queued,
            JobState::Running,
            JobState::Paused,
            JobState::Stalled,
            JobState::Silent,
            JobState::Blocked,
        ] {
            assert!(!state.is_over(), "{state} reported itself finished");
        }
    }

    /// **A `PASS` does not end a Job.** Whether there is a next step is the
    /// workflow's answer, not the verdict's — and a `PASS` that settled to
    /// `DONE` would close a `feature` Job the moment its plan step passed.
    #[test]
    fn a_passing_step_leaves_the_job_running() {
        assert_eq!(Verdict::Pass.settles_to(), JobState::Running);
        assert_eq!(Verdict::Failed.settles_to(), JobState::Running);
        assert_eq!(Verdict::Blocked.settles_to(), JobState::Blocked);
        assert_eq!(Verdict::NeedsHuman.settles_to(), JobState::Paused);
    }

    /// **Two subjects, read in any case, and nothing else is guessed at** — the
    /// rule `word_to_verdict` follows, for the reason `docs/reserved/008` gives:
    /// a proposal filed against a subject Armada has no concept of is a row
    /// nobody can act on.
    #[test]
    fn a_proposal_is_about_the_manifest_or_the_guild_and_nothing_else() {
        assert_eq!(Subject::parse("manifest"), Some(Subject::Manifest));
        assert_eq!(Subject::parse("guild"), Some(Subject::Guild));
        assert_eq!(Subject::parse("  Guild "), Some(Subject::Guild));
        assert_eq!(Subject::parse("MANIFEST"), Some(Subject::Manifest));
        // Inverted once: a parser that accepted everything would pass every
        // assertion above.
        assert_eq!(Subject::parse("armada.yml"), None);
        assert_eq!(Subject::parse("workflow"), None);
        assert_eq!(Subject::parse(""), None);
    }

    /// The word is lower case on purpose — it is a noun in a sentence, not a
    /// state in a column — and it round-trips through its own parser so the
    /// written form and the read form cannot drift.
    #[test]
    fn a_subjects_word_is_lower_case_and_parses_back_to_itself() {
        for subject in [Subject::Manifest, Subject::Guild] {
            let word = subject.word();
            assert_eq!(word, word.to_ascii_lowercase(), "{word} is shouting");
            assert_eq!(Subject::parse(word), Some(subject));
            assert_eq!(
                serde_json::to_string(&subject).unwrap(),
                format!("\"{word}\"")
            );
        }
    }
}
