//! Which of a step's runs a record belongs to.
//!
//! `docs/concepts/workflow.md` resolves a loop step's evidence to "the most
//! recent iteration's work product plus **every prior verdict**", because
//! *"keeping all the verdicts is what shows the same note went unaddressed
//! three times, which is the judgement `iteration_cap` exists to force."* A
//! record keyed by step alone can only say what the last run found.
//!
//! # It is not `retry_count`, and it is not `iteration_count`
//!
//! Both are `job_steps` columns in `domain/job-fields.toml` and both are
//! contested there: `domain/workflows.toml` records that whose `retry_count` a
//! backward jump increments "is undefined", and `workflowdef-fields.toml`
//! leaves open "whose `iteration_count` increments — the gate step's or the
//! routed-to step's". Naming this one of those would settle those questions
//! silently.
//!
//! **An [`Attempt`] counts something observed rather than something policy
//! decides**: the times the step's log says it entered `running`. Whether a run
//! was a retry or an iteration is the open question, and the record does not
//! have to answer it in order to keep both runs. It is also the registry's own
//! word for the unit — `attempt_cap` "bounds total attempts across all
//! iterations of a step" — so this is what those counters get defined against
//! rather than a third vocabulary beside them.

use core::fmt;
use core::num::NonZeroU32;

/// Which run of a step a record was produced by. One-based.
///
/// **There is no constructor that takes an arbitrary number.** The two that
/// exist say where the value came from: [`runs_begun`](Attempt::runs_begun)
/// derives it from a step's log, and [`stored`](Attempt::stored) validates one
/// read back off a row. A caller cannot invent an attempt ordinal and have it
/// disagree with the history, because there is no call that would let it.
///
/// [`NonZeroU32`] rather than a bare `u32`: a record belongs to a run and there
/// is no run zero, so a column holding one was written by something that did
/// not share this type and is a refusal rather than a default.
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct Attempt(NonZeroU32);

impl Attempt {
    /// The run a step is on before it has ever been resumed.
    pub const FIRST: Attempt = Attempt(NonZeroU32::MIN);

    /// The attempt in progress, given how many times a step's log records it
    /// entering `running`.
    ///
    /// **Zero runs begun is still the first attempt.** A record can only be
    /// about a run, so one written before the step's first `running` row
    /// belongs to the run it is part of — and there is exactly one it can be.
    /// Numbering that record zero would put it in a run that never happened.
    pub fn runs_begun(entries_into_running: u32) -> Attempt {
        match NonZeroU32::new(entries_into_running) {
            Some(count) => Attempt(count),
            None => Attempt::FIRST,
        }
    }

    /// An attempt read back off a stored row. `None` on zero, which is not an
    /// attempt and is therefore a malformed column rather than a first run.
    pub fn stored(number: u32) -> Option<Attempt> {
        NonZeroU32::new(number).map(Attempt)
    }

    /// The ordinal, for a column or a rendering. One-based.
    pub fn number(self) -> u32 {
        self.0.get()
    }
}

impl fmt::Display for Attempt {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}", self.0.get())
    }
}

#[cfg(test)]
mod tests {
    use super::Attempt;

    #[test]
    fn a_step_that_has_never_run_is_still_on_its_first_attempt() {
        assert_eq!(Attempt::runs_begun(0), Attempt::FIRST);
        assert_eq!(Attempt::runs_begun(0).number(), 1);
    }

    #[test]
    fn the_fourth_entry_into_running_is_the_fourth_attempt() {
        assert_eq!(Attempt::runs_begun(4).number(), 4);
    }

    #[test]
    fn a_stored_zero_is_refused_rather_than_read_as_a_first_run() {
        assert_eq!(Attempt::stored(0), None);
        assert_eq!(Attempt::stored(3), Some(Attempt::runs_begun(3)));
    }

    /// The order is the whole point of the type: a reader asks for the latest.
    #[test]
    fn attempts_order_by_their_number() {
        assert!(Attempt::runs_begun(1) < Attempt::runs_begun(2));
    }
}
