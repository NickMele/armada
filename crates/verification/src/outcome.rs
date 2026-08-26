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

    /// The content of the injected message, exactly as it goes to the session.
    pub fn text(&self) -> &str {
        &self.text
    }
}
