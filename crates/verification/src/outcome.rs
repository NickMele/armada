//! What the Drone is told once the gate has ruled, and when it is told nothing
//! at all.
//!
//! # The wording here is drafted, not sanctioned
//!
//! `docs/contracts/agent-prompt.md` governs every turn Fleet injects and lists
//! six of them. **The gate's own outcome turn is not one of the six** — the
//! mechanism was decided and the wording never written. What is below is a
//! draft in the shape the contract's other drafts take, and it is marked as one
//! rather than presented as agreed copy.
//!
//! # A failure produces a turn only where there is something to do with it
//!
//! A failed Check used to end the Job outright, so the Drone was terminated
//! rather than told: a turn explaining a verdict to a process about to be
//! killed spends a Drone's remaining tool call (measured: an injected message
//! is delivered at the next turn boundary, so it costs whatever is left of the
//! current one) to deliver information nobody reads.
//!
//! That argument holds exactly while the failure is terminal.
//! [`handed_back`](OutcomeTurn::handed_back) is the case where it is not — the
//! step's retry budget has room, the Drone is about to work the step again, and
//! what the check printed is the whole of what it needs. Where the budget is
//! spent the Job still ends and the Drone is still not told; the reason goes to
//! the person who opens the branch.
//!
//! # No counter, ever
//!
//! An injected turn carries no attempt count, no remaining budget and no
//! consequence. A Drone one attempt from escalation has the strongest possible
//! incentive to satisfy a bar rather than do the work, and this type has no
//! constructor that takes a number. That holds hardest on the hand-back:
//! "this is your last try" is the sentence most likely to produce a weakened
//! assertion instead of a fix.

use config::ResolvedStep;

use crate::mechanical::CheckFailed;

/// How much of one Check's output goes into the turn.
///
/// `checks_runner` captures 64KB per stream and keeps the tail, which is the
/// right amount to keep on disk for a person and far too much to inject into a
/// session — the whole of it would cost more context than the work. This is the
/// tail of the tail, and it is a tail rather than a head because a failing
/// command says why at the end.
///
/// A number with no measurement behind it, which is why it is spelled once
/// here rather than at the call site.
const KEPT_FOR_THE_TURN: usize = 2_000;

/// What the mechanical tier did on a step that advanced.
///
/// **Two counts, because three sentences are true of three different steps**:
/// every declared check ran and passed, some were not run because they cover
/// paths this step did not touch, or none was run at all. A turn that said the
/// first about any of the three would be telling a Drone a check passed that
/// nobody ran — the same lie [`OutcomeTurn::approved`] exists to avoid at a
/// human gate.
///
/// **No number reaches the Drone.** The counts decide which sentence, and the
/// sentence carries none of them, for this module's own reason: a Drone given
/// an arithmetic has an incentive to satisfy it.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Verified {
    declared: usize,
    skipped: usize,
}

impl Verified {
    /// Read off the checks the gate ran. One constructor, taking the set — so
    /// nothing can assemble a pair of counts that no step produced.
    pub fn of(ran: &crate::mechanical::Ran) -> Verified {
        Verified {
            declared: ran.count(),
            skipped: ran.skipped(),
        }
    }

    fn told(&self, label: &str) -> String {
        match (self.declared, self.skipped) {
            // A step that declares nothing, and a step whose every check ran.
            // The sentence is vacuously true of the first and was already
            // being told to it.
            (_, 0) => format!("{label} is verified. It passed every check the step declared."),
            (declared, skipped) if declared == skipped => format!(
                "{label} is verified. No check the step declares covers what you \
                 changed, so none was run."
            ),
            _ => format!(
                "{label} is verified. It passed every check that covers what you \
                 changed; the rest cover paths this step did not touch and were \
                 not run."
            ),
        }
    }
}

/// A turn Fleet injects into a live session.
///
/// **It is not a verdict a Drone can act on selectively.** Three constructors,
/// each reached from one place in `fleet::gate`: two say the step moved on and
/// the third says it did not and is being worked again. There is none that
/// says a step failed and is over — that turn does not exist, because there is
/// nobody left to read it.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct OutcomeTurn {
    text: String,
}

impl OutcomeTurn {
    /// The step passed, and the Drone continues.
    ///
    /// `next` is the step that follows, or `None` where the one that passed was
    /// the last. The two cases read differently on purpose: a Drone told only
    /// "verified" at the end of a workflow keeps working.
    ///
    /// `verified` is what the mechanical tier actually did, and it changes the
    /// opening sentence for [`approved`](OutcomeTurn::approved)'s reason: a
    /// Drone told it "passed every check the step declared" when a Check
    /// covering paths it never touched was not run has been told a check passed
    /// that nobody ran.
    pub fn advanced(
        passed: &ResolvedStep,
        next: Option<&ResolvedStep>,
        verified: Verified,
    ) -> OutcomeTurn {
        let label = passed.label();
        let opening = verified.told(label);
        let text = match next {
            Some(next) => format!(
                "{opening}\n\n\
                 Go on to {}. Submit when it is done, then wait.",
                next.label()
            ),
            None => format!(
                "{opening}\n\n\
                 That was the last part of this task. Nothing further is yours. Stop here."
            ),
        };
        OutcomeTurn { text }
    }

    /// A person took the work at a human gate, and the Drone continues.
    ///
    /// **A turn of its own, because the other one would be a lie.**
    /// [`advanced`](OutcomeTurn::advanced) says the step passed every check it
    /// declared, which is what the mechanical gate ruled; a human gate is a
    /// person reading the work and deciding, and the Drone is told that instead
    /// of being told a check passed that nobody ran.
    ///
    /// It carries no part of what the person said. Where there is something to
    /// change, the act is `request_changes` and the words go with it — an
    /// approval that quoted a reviewer would be an instruction wearing a
    /// verdict's shape.
    pub fn approved(passed: &ResolvedStep, next: Option<&ResolvedStep>) -> OutcomeTurn {
        let label = passed.label();
        let text = match next {
            Some(next) => format!(
                "{label} was reviewed and accepted.\n\n\
                 Go on to {}. Submit when it is done, then wait.",
                next.label()
            ),
            None => format!(
                "{label} was reviewed and accepted.\n\n\
                 That was the last part of this task. Nothing further is yours. Stop here."
            ),
        };
        OutcomeTurn { text }
    }

    /// The same turn, with what happened to the branch underneath added.
    ///
    /// `None` leaves it alone, which is how a base that did not move stays
    /// unannounced — a turn saying nothing happened spends a Drone's tool call
    /// to deliver nothing.
    pub fn and(self, moved: Option<TheBaseMoved>) -> OutcomeTurn {
        let Some(moved) = moved else {
            return self;
        };
        OutcomeTurn {
            text: format!("{}\n\n{}", self.text, moved.told()),
        }
    }

    /// The step's mechanical gate failed, its budget has room, and the work is
    /// going back to the Drone that did it.
    ///
    /// **The output is the point.** A Drone told only that `test` failed knows
    /// less than a person reading the same row, and the run it is about to do
    /// would begin by running the check itself to find out — which is the turn
    /// paying for information it already had.
    ///
    /// `printed` carries what each named check put on its streams. A failure
    /// with nothing printed — an empty diff, a scope violation — has no entry
    /// and gets none: the expectation and what was produced already say the
    /// whole of it.
    ///
    /// It ends by naming the one thing a hand-back invites. A Drone that cannot
    /// make a test pass can always make the test stop asking, and this is the
    /// moment that becomes tempting; the gaming check catches it afterwards and
    /// saying so first is cheaper than catching it.
    pub fn handed_back(
        failed: &ResolvedStep,
        failures: &[CheckFailed],
        printed: &[Printed<'_>],
    ) -> OutcomeTurn {
        let label = failed.label();
        let said = failures
            .iter()
            .map(|failure| {
                format!(
                    "- expected {}, and {}",
                    failure.expected(),
                    failure.produced()
                )
            })
            .collect::<Vec<String>>()
            .join("\n");
        let mut text =
            format!("{label} did not pass. This is what the checks found:\n\n{said}\n\n");
        for one in printed {
            text.push_str(&one.quoted());
        }
        text.push_str(
            "Work the same step again and submit when it is done. Fix what the output says is \
             wrong.\n\nDo not change what a check runs, and do not weaken, narrow, skip or \
             delete a test to get past it. A check that stops asking is not a check that passed, \
             and it is looked for.",
        );
        OutcomeTurn { text }
    }

    /// The content of the injected message, exactly as it goes to the session.
    pub fn text(&self) -> &str {
        &self.text
    }
}

/// What one named Check put on its streams, for the turn that hands it back.
///
/// Borrowed rather than owned, and `&str` rather than any runner type: this
/// crate does not depend on `checks-runner` and adding a dependency so that a
/// turn could be built would put the thing that runs commands underneath the
/// thing that decides verdicts.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Printed<'a> {
    pub check: &'a str,
    pub said: &'a str,
}

impl Printed<'_> {
    /// The block that goes into the turn: the tail of what was printed, fenced,
    /// with a line saying it was cut where it was.
    fn quoted(&self) -> String {
        let said = self.said.trim();
        if said.is_empty() {
            return String::new();
        }
        let (cut, kept) = tail(said);
        let opening = match cut {
            false => format!("What `{}` printed:", self.check),
            true => format!("The last of what `{}` printed:", self.check),
        };
        format!("{opening}\n\n```\n{kept}\n```\n\n")
    }
}

/// The last [`KEPT_FOR_THE_TURN`] bytes, cut at a line boundary so the quote
/// does not begin mid-word, and whether anything was dropped.
fn tail(said: &str) -> (bool, &str) {
    if said.len() <= KEPT_FOR_THE_TURN {
        return (false, said);
    }
    let from = said.len() - KEPT_FOR_THE_TURN;
    let kept = said
        .get(from..)
        .and_then(|tail| tail.find('\n').map(|at| &tail[at + 1..]))
        .unwrap_or(said);
    (true, kept)
}

/// What happened to the branch a Drone is working on while it worked.
///
/// **Told, not asked.** The Drone has just submitted and holds no git, so
/// nothing here is a decision it could have taken part in — and a conflict is
/// work rather than a question, which is why the second variant reads as an
/// instruction and not as an apology.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum TheBaseMoved {
    /// It moved and the branch was brought up to it cleanly.
    BroughtUpToDate { base: String, commits: usize },
    /// It moved, the branch was brought up to it, and files were left with
    /// conflict markers in them.
    Conflicted { base: String, files: Vec<String> },
    /// It moved and the branch could not be put on top of it, so nothing moved.
    /// **Nothing here is the Drone's to fix** — it is told because it is going
    /// on to work against a tree that is behind.
    CouldNotFollow { base: String },
}

impl TheBaseMoved {
    /// The paragraph that goes into the turn.
    fn told(&self) -> String {
        match self {
            TheBaseMoved::BroughtUpToDate { base, commits } => format!(
                "While you worked, `{base}` moved on by {commits} commit(s) and your branch \
                 has been put on top of it. Anything you are about to change may have \
                 changed underneath you — re-read a file before you edit it."
            ),
            TheBaseMoved::Conflicted { base, files } => format!(
                "While you worked, `{base}` moved on and your branch has been put on top of \
                 it. These files were left with conflict markers in them and resolving them \
                 is part of the work:\n\n{}\n\nOpen each one, keep what belongs, and \
                 remove every marker before you submit again.",
                files
                    .iter()
                    .map(|file| format!("- {file}"))
                    .collect::<Vec<_>>()
                    .join("\n")
            ),
            TheBaseMoved::CouldNotFollow { base } => format!(
                "While you worked, `{base}` moved on, and this branch could not be put on \
                 top of it. It is exactly where you left it. Carry on — somebody will \
                 reconcile the two."
            ),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn advanced() -> OutcomeTurn {
        OutcomeTurn {
            text: String::from("Implement is verified."),
        }
    }

    #[test]
    fn a_base_that_did_not_move_leaves_the_turn_exactly_as_it_was() {
        assert_eq!(advanced().and(None), advanced());
    }

    #[test]
    fn a_clean_catch_up_tells_the_drone_what_moved_and_how_much() {
        let told = advanced()
            .and(Some(TheBaseMoved::BroughtUpToDate {
                base: String::from("main"),
                commits: 3,
            }))
            .text()
            .to_string();
        assert!(told.starts_with("Implement is verified."), "{told}");
        assert!(
            told.contains("`main`") && told.contains("3 commit"),
            "{told}"
        );
        assert!(
            told.contains("re-read a file before you edit it"),
            "the reason it is being told at all: {told}"
        );
    }

    #[test]
    fn a_conflict_is_handed_over_as_work_and_names_every_file() {
        let told = advanced()
            .and(Some(TheBaseMoved::Conflicted {
                base: String::from("main"),
                files: vec![String::from("src/log.rs"), String::from("src/write.rs")],
            }))
            .text()
            .to_string();
        assert!(told.contains("- src/log.rs"), "{told}");
        assert!(told.contains("- src/write.rs"), "{told}");
        assert!(
            told.contains("part of the work"),
            "a conflict is work, not a question: {told}"
        );
    }

    /// No count of anything the Drone could be measured on. The rule this type
    /// already carries about attempts holds for what moved underneath it too.
    #[test]
    fn nothing_told_here_is_a_number_a_drone_could_be_judged_on() {
        let told = advanced()
            .and(Some(TheBaseMoved::CouldNotFollow {
                base: String::from("main"),
            }))
            .text()
            .to_string();
        assert!(told.contains("exactly where you left it"), "{told}");
        assert!(!told.contains("attempt"), "{told}");
    }
}
