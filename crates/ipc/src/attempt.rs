//! How many times a step was worked, and what each of those runs came to.
//!
//! # The record already held it and nothing served it
//!
//! `job_events` keeps every entry into `running` and every move out of it, and
//! `store::step_attempt` counts the first of those to key the per-attempt
//! tables. So a Job that worked one step four times has four runs written down
//! — and the wire carried the last verdict alone, which is why a step that
//! passed on its third try read exactly like one that passed on its first.
//!
//! # Folded on this side of the store, and never replayed
//!
//! `crates/store/src/fold.rs` owns the machine. What [`StepAttempt::over`] does
//! is read the same rows [`JobHistory`](crate::JobHistory) serves and group
//! them by step — no move goes back through `Job::transition` here, and nothing
//! below can produce a state the fold refused.
//!
//! # It rides on `get_job` and the history does not
//!
//! `detail.rs` and `history.rs` both argue that an unbounded list has no place
//! on a call made at every open of a Job. **This list is bounded by the
//! workflow**: one entry per run of one step, and a run costs a retry budget, so
//! a four-step Job that fought its way through is a dozen entries. What a
//! history has that this does not is every status move, every Drone arriving,
//! and a row per move rather than per run.

use serde::{Deserialize, Serialize};

use crate::enums::StepState;
use crate::ids::Instant;

/// One run of one step: when it began, and what it came to.
///
/// **The outcome is a [`StepState`] rather than a word of its own.** The inner
/// machine already names the six places a run can be, and a second vocabulary
/// here would be a set nothing declares — `advanced` is the run that passed,
/// `retrying` is the run that failed inside its budget, `stopped` is the one
/// that spent it, and `awaiting_human` is the one holding for a person. A run
/// still going is [`ended_at`](StepAttempt::ended_at) absent.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct StepAttempt {
    /// Which run this was, counted from one — the same ordinal
    /// `store::step_attempt` keys the per-attempt tables under, derived the same
    /// way, from entries into `running`.
    pub attempt: u32,
    /// Where the run got to.
    ///
    /// **`running` while it is still going**, which is the one value that is
    /// also readable off the step itself; every other value here is a run that
    /// is over, and this is the only place an earlier run's outcome survives.
    pub outcome: StepState,
    /// The escalation trigger the run carried out of `running`, where it
    /// carried one.
    ///
    /// A string rather than a mirrored enum, for the reason
    /// [`StepMoved::why`](crate::StepMoved) is one: the spelling belongs to
    /// `escalation-triggers.toml` and a closed set restated here would be a
    /// second authority for a list that already has one. **Absent on a run that
    /// advanced**, which is what makes `Attempt 2 advanced` and `Attempt 1
    /// refused` different sentences rather than one with a blank in it.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub why: Option<String>,
    /// When the run entered `running`.
    pub started_at: Instant,
    /// When it left. **Absent is the run that is still going**, and there is at
    /// most one of those per step.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub ended_at: Option<Instant>,
}

impl StepAttempt {
    /// The runs of one step, oldest first, folded from that step's moves in
    /// `seq` order.
    ///
    /// **A run opens on an entry into `running` and closes on the next move
    /// away from it.** Counting entries rather than exits is what
    /// `store::step_attempt` counts, so the ordinals here and the ordinals the
    /// per-attempt tables are keyed under cannot disagree.
    ///
    /// A move out of `running` that arrives with no run open is ignored rather
    /// than opening one: the alternative is inventing a `started_at` for a run
    /// whose beginning the log does not hold, and a fabricated instant on a
    /// rail is worse than a run the rail does not draw.
    ///
    /// **A second arrival at `running` closes the run and opens the next**, and
    /// it is the one arrival that changes no state. `running -> running` is the
    /// machine's only self-edge and exists to be a boundary between two passes
    /// over a step that never left: `#263` for a loop coming round, `#418` for a
    /// person sending work back at a gate. Swallowed here, the store keyed
    /// three passes and this served one — the same half of the same defect,
    /// one layer out.
    ///
    /// **The run it closes reads `awaiting_human`.** It ended because a person
    /// was standing at the step and answered, which is what that state says and
    /// the one state nothing else here produces; `running` with an end on it
    /// would say nothing, and `advanced` would say it passed.
    pub fn over<'a>(moves: impl Iterator<Item = Move<'a>>) -> Vec<StepAttempt> {
        let running = StepState::from(core_model::StepState::Running);
        let held = StepState::from(core_model::StepState::AwaitingHuman);
        let mut runs: Vec<StepAttempt> = Vec::new();
        for moved in moves {
            let arriving = moved.to == running.as_wire();
            let open = runs.last().is_some_and(|run| run.ended_at.is_none());
            match (arriving, open) {
                (false, false) => {}
                (true, true) => {
                    // The boundary. It carries no trigger — nothing refused
                    // this — so the run closes with `why` unset, which is what
                    // keeps it out of `verdicts`.
                    if let Some(run) = runs.last_mut() {
                        run.outcome = held;
                        run.ended_at = Some(moved.at.clone());
                    }
                    runs.push(StepAttempt {
                        attempt: runs.len() as u32 + 1,
                        outcome: running,
                        why: None,
                        started_at: moved.at.clone(),
                        ended_at: None,
                    });
                }
                (true, false) => runs.push(StepAttempt {
                    attempt: runs.len() as u32 + 1,
                    outcome: running,
                    why: None,
                    started_at: moved.at.clone(),
                    ended_at: None,
                }),
                (false, true) => {
                    // A spelling this build has no `StepState` for leaves the
                    // run open rather than closing it under a guess. It cannot
                    // arrive through the store, whose own read refuses one.
                    let (Some(run), Some(outcome)) =
                        (runs.last_mut(), StepState::from_wire(moved.to))
                    else {
                        continue;
                    };
                    run.outcome = outcome;
                    run.why = moved.why.map(str::to_string);
                    run.ended_at = Some(moved.at.clone());
                }
            }
        }
        runs
    }

    /// What each closed run in `attempts` came to, oldest first.
    ///
    /// **`why` alone decides it.** A run that closed carrying a trigger is one
    /// a gate refused — `stopped` and `retrying` never close without one, and
    /// `advanced` carries one only on the one move that arrives there without
    /// having passed a gate: [`StepTarget::Overridden`](core_model::StepTarget::Overridden),
    /// where a person advanced the step over the gate's ruling. Reading
    /// `outcome` first and asking "is this `advanced`" would get that one
    /// wrong in the other direction — the state a naive mapping trusts is
    /// exactly the one state this case shares with an ordinary pass. The run
    /// still open — [`ended_at`](StepAttempt::ended_at) absent — has produced
    /// no ruling yet and is not in the list.
    ///
    /// **Neither is the run a person ended by answering.** A run closed as
    /// `awaiting_human` reached no gate ruling at all: the tiers had already
    /// held, and what closed it was somebody asking for the work again. Left in
    /// it would read `passed`, because `why` is empty on that row and empty is
    /// what a pass looks like — a step sent back would report a verdict saying
    /// it was accepted. What that pass came to is on its own attempt row and in
    /// the person's note, which is where a reader is owed it.
    pub fn verdicts(attempts: &[StepAttempt]) -> Vec<crate::Verdict> {
        let held = StepState::from(core_model::StepState::AwaitingHuman);
        attempts
            .iter()
            .filter(|attempt| attempt.ended_at.is_some() && attempt.outcome != held)
            .map(|attempt| crate::Verdict {
                attempt: attempt.attempt,
                named: if attempt.why.is_some() {
                    "failed"
                } else {
                    "passed"
                }
                .to_string(),
                trigger: attempt.why.clone(),
            })
            .collect()
    }
}

/// One recorded move of one step, as the caller hands it over.
///
/// **Borrowed, and it names no step.** The caller has already grouped by step —
/// which it has to, since the rows arrive interleaved with every other step's —
/// so carrying the id again would be a field this type could disagree with the
/// grouping about.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Move<'a> {
    /// Where the step arrived, spelled as `step-states.toml` spells it.
    pub to: &'a str,
    /// The escalation trigger the move carried, where it carried one.
    pub why: Option<&'a str>,
    pub at: &'a Instant,
}

#[cfg(test)]
mod tests {
    use super::{Move, StepAttempt};
    use crate::ids::Instant;

    fn at(second: u32) -> Instant {
        Instant::carried(format!("2026-08-31T10:00:{second:02}.000Z"))
    }

    /// The sentence the run tree draws: `Attempt 1 refused`, `Attempt 2
    /// advanced`, and the trigger beside the first of them.
    #[test]
    fn a_step_worked_twice_reports_both_runs_and_what_each_came_to() {
        let (first, stopped, second, advanced) = (at(0), at(1), at(2), at(3));
        let runs = StepAttempt::over(
            [
                Move {
                    to: "running",
                    why: None,
                    at: &first,
                },
                Move {
                    to: "retrying",
                    why: Some("gate_failure"),
                    at: &stopped,
                },
                Move {
                    to: "running",
                    why: None,
                    at: &second,
                },
                Move {
                    to: "advanced",
                    why: None,
                    at: &advanced,
                },
            ]
            .into_iter(),
        );

        assert_eq!(runs.len(), 2);
        assert_eq!(runs[0].attempt, 1);
        assert_eq!(runs[0].outcome.as_wire(), "retrying");
        assert_eq!(runs[0].why.as_deref(), Some("gate_failure"));
        assert_eq!(runs[0].ended_at.as_ref(), Some(&stopped));
        assert_eq!(runs[1].attempt, 2);
        assert_eq!(runs[1].outcome.as_wire(), "advanced");
        assert_eq!(runs[1].why, None);
    }

    /// **A step sent back at a gate has two runs and one verdict.** The
    /// boundary is `running -> running`, the machine's only self-edge, and the
    /// run it closes was ended by a person rather than by a gate — so it reads
    /// `awaiting_human` and reports no ruling. Reported as `passed`, the pass
    /// somebody refused would say on the wire that it was accepted.
    #[test]
    fn a_pass_a_person_ended_closes_and_claims_no_verdict() {
        let (first, sent_back, advanced) = (at(0), at(1), at(2));
        let runs = StepAttempt::over(
            [
                Move {
                    to: "running",
                    why: None,
                    at: &first,
                },
                Move {
                    to: "running",
                    why: None,
                    at: &sent_back,
                },
                Move {
                    to: "advanced",
                    why: None,
                    at: &advanced,
                },
            ]
            .into_iter(),
        );

        assert_eq!(runs.len(), 2, "the boundary opened a second run");
        assert_eq!(runs[0].outcome.as_wire(), "awaiting_human");
        assert_eq!(runs[0].ended_at.as_ref(), Some(&sent_back));
        assert_eq!(runs[1].attempt, 2);
        assert_eq!(runs[1].started_at, sent_back);
        assert_eq!(runs[1].outcome.as_wire(), "advanced");

        let verdicts = StepAttempt::verdicts(&runs);
        assert_eq!(
            verdicts.len(),
            1,
            "only the pass a gate ruled on has a verdict"
        );
        assert_eq!(verdicts[0].attempt, 2);
        assert_eq!(verdicts[0].named, "passed");
    }

    /// The run that is still going is the one with no end, and it is the only
    /// one a surface may draw a spinner on.
    #[test]
    fn the_run_in_flight_has_no_end_and_reads_as_running() {
        let started = at(0);
        let runs = StepAttempt::over(
            [Move {
                to: "running",
                why: None,
                at: &started,
            }]
            .into_iter(),
        );

        assert_eq!(runs.len(), 1);
        assert_eq!(runs[0].outcome.as_wire(), "running");
        assert_eq!(runs[0].ended_at, None);
    }

    /// A step created and never entered has no runs. **Empty rather than one
    /// run in `not_started`** — the step's own state says that already, and a
    /// run nothing worked is a row the tree would draw an attempt for.
    #[test]
    fn a_step_never_entered_has_no_runs_at_all() {
        assert!(StepAttempt::over([].into_iter()).is_empty());
    }

    /// A move out of `running` with nothing open is dropped rather than
    /// inventing the instant the run began.
    #[test]
    fn a_move_out_of_nothing_opens_no_run() {
        let stopped = at(1);
        let runs = StepAttempt::over(
            [Move {
                to: "stopped",
                why: Some("retries_exhausted"),
                at: &stopped,
            }]
            .into_iter(),
        );

        assert!(runs.is_empty());
    }
}
