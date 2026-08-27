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
//! # A failure produces no turn
//!
//! At M1 a failed Check ends the Job: there is no retry, no Judge and no
//! escalation, so the Drone is terminated rather than told. A turn explaining a
//! verdict to a process about to be killed spends a Drone's remaining tool call
//! (measured: an injected message is delivered at the next turn boundary, so it
//! costs whatever is left of the current one) to deliver information nobody
//! reads. The reason goes to the person who opens the branch — which is what
//! "a failed step fails the Job and a person reads the branch" means.
//!
//! # No counter, ever
//!
//! An injected turn carries no attempt count, no remaining budget and no
//! consequence. A Drone one attempt from escalation has the strongest possible
//! incentive to satisfy a bar rather than do the work, and this type has no
//! constructor that takes a number.

use config::ResolvedStep;

/// A turn Fleet injects into a live session.
///
/// **It is not a verdict a Drone can act on selectively.** There is one
/// constructor and it is reached only from the advance path, so no failure
/// state can produce one.
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
    pub fn advanced(passed: &ResolvedStep, next: Option<&ResolvedStep>) -> OutcomeTurn {
        let label = passed.label();
        let text = match next {
            Some(next) => format!(
                "{label} is verified. It passed every check the step declared.\n\n\
                 Go on to {}. Submit when it is done, then wait.",
                next.label()
            ),
            None => format!(
                "{label} is verified. It passed every check the step declared.\n\n\
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

    /// The content of the injected message, exactly as it goes to the session.
    pub fn text(&self) -> &str {
        &self.text
    }
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
