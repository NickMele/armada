//! What a run was refused, folded out of the stream while it is still in hand.
//!
//! **The twin of [`backfill::refusals`], from the other place the answer
//! lives.** That one reads a stopped Job's transcripts off the disk for a
//! person who opened it; this one folds the events Fleet is holding at the
//! moment it classifies the ending, for the line it writes into the Job's log.
//! One vocabulary — [`Refusals`] — because they are the same answer.
//!
//! [`backfill::refusals`]: super::refusals

use adapter_traits::{CallDetail, DroneEvent};
use core_model::{Refusal, Refusals};

/// How many refused calls the Job log line names.
///
/// **A log line is read in a terminal**, so this is the handful somebody greps
/// rather than the whole list; the count travels beside it and `ipc::Stuck` is
/// where a surface reads the rest. The run this was written against was refused
/// once, so the cap is reached by almost nothing.
const NAMED: usize = 5;

/// The calls this run reached for and was refused, named.
///
/// # The command is on the other event
///
/// A refusal is recorded twice and neither half is the answer.
/// [`DroneEvent::Refused`] carries the tool and the harness's own reason, which
/// was empty on every one observed; the command is on the
/// [`DroneEvent::Called`] that shares its call id. So a line built from the
/// refusal alone says `Bash` and nothing else, which is the defect: a person
/// was told a policy stopped the Drone and never told what it stopped.
///
/// **Nothing is invented where the join misses.** A refusal whose call is not
/// in this run's stream — an adopted Drone whose earlier turns went into a pipe
/// with no reader — keeps an empty detail, and an empty `because` stays empty
/// rather than being filled in from the trigger.
pub fn refused_in(heard: &[DroneEvent]) -> Refusals {
    let mut kept = Vec::new();
    let mut in_all = 0u64;
    for event in heard {
        let DroneEvent::Refused {
            tool,
            call,
            because,
        } = event
        else {
            continue;
        };
        in_all += 1;
        if kept.len() == NAMED {
            continue;
        }
        let against = against(heard, call);
        kept.push(Refusal {
            tool: tool.clone(),
            call: call.clone(),
            detail: String::from(against.map(CallDetail::shown).unwrap_or_default()),
            truncated: against.is_some_and(CallDetail::truncated),
            length: against.map(CallDetail::length),
            because: because.clone(),
        });
    }
    Refusals::of(kept, in_all)
}

/// What one refused call was on, off the `called` event that shares its id.
///
/// **The whole value and not its shown form**, because the row is owed how much
/// there was as well as the line. `None` is a refusal whose call is not in this
/// run's stream, which is a different answer from a call with nothing to show.
///
/// A scan per named refusal, and deliberately not a map over every call the run
/// made: at most [`NAMED`] of these are ever run, and a map would be a second
/// copy of the stream built to answer five questions.
fn against<'a>(heard: &'a [DroneEvent], call: &str) -> Option<&'a CallDetail> {
    heard.iter().find_map(|earlier| match earlier {
        DroneEvent::Called {
            call: id, detail, ..
        } if id == call => Some(detail),
        _ => None,
    })
}
