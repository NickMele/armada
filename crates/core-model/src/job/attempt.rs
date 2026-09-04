//! Which of a step's runs a record belongs to.
//!
//! `docs/concepts/workflow.md` resolves a loop step's evidence to "the most
//! recent iteration's work product plus **every prior verdict**", because
//! *"keeping all the verdicts is what shows the same note went unaddressed
//! three times, which is the judgement `iteration_cap` exists to force."* A
//! record keyed by step alone can only say what the last run found.
//!
//! # Three types over one log, and no two of them are the same number
//!
//! **An [`Attempt`] counts something observed rather than something policy
//! decides**: the times the step's log says it entered `running`. It is also
//! the registry's own word for the unit — `attempt_cap` "bounds total attempts
//! across all iterations of a step".
//!
//! [`Iteration`] is the same discipline over the loop's edge, and [`Spent`] is
//! the retry budget's own count, which resets where an attempt does not.
//!
//! **[`Spent`] exists because the wrong one compiled.**
//! `ResolvedStep::may_hand_back` took an [`Attempt`], so a step on its second
//! pass arrived at its retry gate with the first pass's runs already charged —
//! and `retry_limit`'s registry row says the opposite: *"Resets on a loop
//! return — re-entry as designed is a fresh attempt budget."* Three types, and
//! no call site where two of them are interchangeable.
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

/// Which pass over a step this is. One-based.
///
/// **The loop counterpart of [`Attempt`], and constructed the same two ways for
/// the same reason**: [`returns_made`](Iteration::returns_made) derives it from
/// the step's own log and [`stored`](Iteration::stored) validates one read back
/// off a row. There is no constructor taking an arbitrary number, so nothing
/// can hand a cap a count the history does not support.
///
/// # What it counts, and what it does not claim
///
/// The entries into `running` from `advanced` in this step's log. That edge is
/// the loop return and only the loop return, so the count is an observation
/// rather than a policy — exactly [`Attempt`]'s standing, and for the same
/// reason: the questions the registry leaves open are about *whose* counter
/// increments, and an observation about one step's log does not have to answer
/// them to be true about that step.
///
/// **It is therefore not yet `iteration_count`.** That counter is the emitting
/// step's — `docs/journeys/triage-queue.md` settles it, because a cap and the
/// count it bounds must not be split or `loop_cap` never fires, and because the
/// emitting-step reading is the only one that survives two loops sharing a
/// target. The emitting step has no move of its own on a return, so it has
/// nothing to count yet, and this reads as the routed-to step's passes — which
/// is true about that step, renders as "draft · 3rd", and is not the number the
/// cap is asked against.
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct Iteration(NonZeroU32);

impl Iteration {
    /// The pass a step is on before anything has routed back to it. **Every
    /// step of a linear workflow is on this one forever**, which is why the
    /// type is not an `Option`: a step that has never looped is on its first
    /// pass, not on no pass.
    pub const FIRST: Iteration = Iteration(NonZeroU32::MIN);

    /// The pass in progress, given how many times a step's log records it
    /// entering `running` from `advanced`.
    ///
    /// **Off by one against the returns, deliberately.** A step redone once is
    /// on its second pass, and the arithmetic lives here rather than at the
    /// caller for the reason `ResolvedStep::may_hand_back` gives about the
    /// retry budget: a caller adding the one itself is a second place the
    /// off-by-one can be wrong.
    pub fn returns_made(returns: u32) -> Iteration {
        match returns.checked_add(1).and_then(NonZeroU32::new) {
            Some(count) => Iteration(count),
            // Unreachable off a real log — it would need four billion returns
            // past a cap of five — and saturating is the only answer that is
            // not a lie: the highest pass this type can name is nearer the
            // truth than wrapping to the first.
            None => Iteration(NonZeroU32::MAX),
        }
    }

    /// A pass read back off a stored row. `None` on zero, which is not a pass
    /// and is a malformed column rather than a first one.
    pub fn stored(number: u32) -> Option<Iteration> {
        NonZeroU32::new(number).map(Iteration)
    }

    /// The ordinal, for a rendering. One-based, so a rail reads "iteration 3 of
    /// 5" off this and the cap without arithmetic of its own.
    pub fn number(self) -> u32 {
        self.0.get()
    }
}

impl fmt::Display for Iteration {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}", self.0.get())
    }
}

#[cfg(test)]
mod iteration_tests {
    use super::Iteration;

    #[test]
    fn a_step_nothing_has_returned_to_is_on_its_first_pass() {
        assert_eq!(Iteration::returns_made(0), Iteration::FIRST);
        assert_eq!(Iteration::returns_made(0).number(), 1);
    }

    /// The off-by-one the type exists to own: one return, second pass.
    #[test]
    fn one_return_is_the_second_pass() {
        assert_eq!(Iteration::returns_made(1).number(), 2);
        assert_eq!(Iteration::returns_made(4).number(), 5);
    }

    #[test]
    fn a_stored_zero_is_refused_rather_than_read_as_a_first_pass() {
        assert_eq!(Iteration::stored(0), None);
        assert_eq!(Iteration::stored(3), Some(Iteration::returns_made(2)));
    }

    #[test]
    fn a_count_that_could_not_come_off_a_log_saturates_rather_than_wrapping() {
        assert_eq!(Iteration::returns_made(u32::MAX).number(), u32::MAX);
    }

    /// The order is what a cap is asked against.
    #[test]
    fn passes_order_by_their_number() {
        assert!(Iteration::returns_made(0) < Iteration::returns_made(1));
    }
}

/// How many of a step's runs the pass it is on has spent. One-based.
///
/// **The count `retry_limit` is asked against, and the only one.** It is not
/// [`Attempt`], which keys every per-run record and therefore has to climb
/// across a loop return so a second pass's verdicts do not overwrite a first
/// pass's. This resets on that return, because `retry_limit`'s registry row
/// says a re-entry as designed is a fresh budget — and it is a separate type
/// precisely so that the two cannot be handed to the same call.
///
/// **Identical to [`Attempt`] on every step of every linear workflow**, which
/// is why the mistake went unnoticed: nothing had ever returned, so the two
/// readings of the log agreed.
///
/// Constructed one way only, from the step's own log, for [`Attempt`]'s reason.
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct Spent(NonZeroU32);

impl Spent {
    /// The run a pass is on before anything has been handed back inside it.
    pub const FIRST: Spent = Spent(NonZeroU32::MIN);

    /// The run this pass is on, given how many times the step has entered
    /// `running` since the last verdict routed back to it.
    ///
    /// **Zero runs since the return is still the first of the pass**, for
    /// [`Attempt::runs_begun`]'s reason: the return itself writes the row that
    /// makes it one, and a record written between the two belongs to the run
    /// it is part of.
    pub fn runs_this_pass(entries_into_running: u32) -> Spent {
        match NonZeroU32::new(entries_into_running) {
            Some(count) => Spent(count),
            None => Spent::FIRST,
        }
    }

    /// The ordinal, for the arithmetic `ResolvedStep::may_hand_back` owns.
    /// One-based.
    pub fn number(self) -> u32 {
        self.0.get()
    }
}

impl fmt::Display for Spent {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}", self.0.get())
    }
}

#[cfg(test)]
mod spent_tests {
    use super::{Attempt, Spent};

    #[test]
    fn a_pass_that_has_not_begun_a_run_is_still_on_its_first() {
        assert_eq!(Spent::runs_this_pass(0), Spent::FIRST);
        assert_eq!(Spent::runs_this_pass(0).number(), 1);
    }

    /// The reading that makes it a different number from [`Attempt`]: a step
    /// worked four times across two passes is on the second run of the second
    /// pass, and a budget of two still has room.
    #[test]
    fn the_budget_resets_where_the_attempt_keeps_climbing() {
        assert_eq!(Attempt::runs_begun(4).number(), 4);
        assert_eq!(Spent::runs_this_pass(2).number(), 2);
    }

    #[test]
    fn runs_of_a_pass_order_by_their_number() {
        assert!(Spent::runs_this_pass(1) < Spent::runs_this_pass(2));
    }
}
