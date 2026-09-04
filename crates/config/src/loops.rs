//! `verdict_routing` and `iteration_cap` on a step: the loop edge, and the
//! bound on how many times it may be taken.
//!
//! **These two are the exception to this crate's own rule, for one wave.**
//! Everywhere else a key nothing reads is refused, because a field nothing
//! reads is a promise the file makes and the system does not keep. These are
//! read and carried while nothing routes on them: the step machine has no edge
//! from `advanced` back to `running`, so `core-model` cannot yet express the
//! return, and `iteration_count` is a `job_steps` column the schema records as
//! deliberately absent. The promise is made deliberately and it is stated here
//! rather than left to be discovered — #263's other half is what keeps it.
//!
//! **A loop return is not a retry**, which is why there are two counters in the
//! registry and not one. Nothing went wrong: a plan on its fourth honest draft
//! must not have consumed a retry budget or tripped an escalation. That is the
//! distinction `iteration_cap` exists to hold, and folding it into
//! `retry_limit` would save a field and lose the difference between a Drone
//! that failed four times and a plan that was asked for four drafts.
//!
//! **The cap lives on the step that emits the verdict**, beside the routing it
//! bounds — the registry is explicit that a cap split from the count it bounds
//! never fires, and `docs/journeys/triage-queue.md` settled the matching
//! question about the count: `request_changes` increments the *gate* step's
//! `iteration_count`, not the step it routes back to.

use std::collections::BTreeMap;

use crate::error::{Fault, Refusal};
use crate::yaml::{self, Table};
use core_model::StepId;

/// A gate verdict that neither advances the Job nor ends it.
///
/// **One value, of the human gate's three.** `approve` advances and `reject`
/// ends the Job, so neither has anywhere to be routed to — which is why this is
/// an enum rather than the open string map the schema's JSON looks like. A
/// second non-terminal verdict widens this and every `match` on it at once.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub enum GateVerdict {
    RequestChanges,
}

impl GateVerdict {
    /// The word the file writes.
    pub fn as_wire(self) -> &'static str {
        match self {
            GateVerdict::RequestChanges => "request_changes",
        }
    }
}

const VERDICT_CARRIED: &[(&str, GateVerdict)] = &[("request_changes", GateVerdict::RequestChanges)];
const VERDICT_KEYS: &[&str] = &["request_changes"];

/// What a step declares about looping: where it goes back to, and how often.
#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub(crate) struct Looping {
    pub(crate) routing: BTreeMap<GateVerdict, StepId>,
    pub(crate) iteration_cap: Option<u32>,
}

/// Read both keys off a step, or [`None`] where one of them was refused.
///
/// [`None`] means recorded, the same as everywhere else in this crate: the
/// refusal is already in `out` and the step is dropped with it.
///
/// `linear` is the workflow's declared structure, and it is a parameter rather
/// than something read here because the contradiction it detects is a fact
/// about the file and not about the step. **On a linear workflow the routing
/// map is refused unread**: its value cannot be right, whatever it says, so
/// parsing it would only add refusals underneath the one that matters.
pub(crate) fn looping(
    table: &mut Table<'_>,
    linear: bool,
    out: &mut Vec<Refusal>,
) -> Option<Looping> {
    // Asked before the key is taken, and asked of the file rather than of what
    // parsed: a `verdict_routing` whose own value was refused still means the
    // author wrote a loop edge here, and the cap below must not then be
    // reported as capping nothing.
    let declares_an_edge = table.present("verdict_routing");

    // **The one deferred key with a refusal of its own.** As an unknown key it
    // would read as "M1 does not do that yet", when on a linear workflow it is
    // wrong at every milestone: the declared structure and the wiring disagree,
    // and the file says so about itself. `read` holds the other half of the
    // same rule, where a `loop` declares no edge at all.
    let routing = if linear && declares_an_edge {
        table.ignore("verdict_routing");
        out.push(Refusal::new(
            table.at("verdict_routing"),
            Fault::ContradictsStructure {
                structure: "linear",
            },
        ));
        Some(BTreeMap::new())
    } else {
        verdict_routing(table, out)
    };

    let cap_key = table.at("iteration_cap");
    // **Absent is none, and a malformed one is a refusal rather than none** —
    // `retry_limit`'s reason. A file that writes `iteration_cap: "five"` meant
    // to bound the loop, and silently reading that as unbounded would be the
    // parser deciding how long a Job may go round.
    //
    // Zero is legal, for the reason `retry_limit: 0` is: a step declaring that
    // its first `request_changes` is its last is a sentence an author is
    // entitled to write, and `yaml::counted` is the reader that allows it.
    let iteration_cap = match table.optional("iteration_cap") {
        None => Some(None),
        Some(value) => yaml::counted(&cap_key, value, out).map(Some),
    };

    // **A cap on a step with no edge bounds nothing.** The same shape as
    // `declare_plan_at` with no `evidence_scope`: half a statement, refused
    // where it is written rather than resolved into a number nothing spends.
    // The registry is the authority — the cap and the count it bounds live on
    // one step, because split they never fire.
    if !declares_an_edge && matches!(iteration_cap, Some(Some(_))) {
        out.push(Refusal::new(cap_key, Fault::CapWithoutALoop));
        return None;
    }

    Some(Looping {
        routing: routing?,
        iteration_cap: iteration_cap?,
    })
}

/// The routing map, read as a closed table of verdicts.
///
/// Read with [`Table`] rather than as an open map so that a verdict outside the
/// set is refused by the same machinery every other unknown key is, naming what
/// is read at that position. `{}` is [`Fault::Empty`] rather than "no edge":
/// the author wrote the key, which is a different mistake from never having
/// written it, and it is the mistake that would otherwise satisfy the wiring
/// check with an edge that goes nowhere.
fn verdict_routing(
    table: &mut Table<'_>,
    out: &mut Vec<Refusal>,
) -> Option<BTreeMap<GateVerdict, StepId>> {
    let at = table.at("verdict_routing");
    let Some(value) = table.optional("verdict_routing") else {
        return Some(BTreeMap::new());
    };
    let mut map = Table::open(&at, value, out)?;
    if map.is_empty() {
        out.push(Refusal::new(at, Fault::Empty));
        map.close(VERDICT_KEYS, out);
        return None;
    }

    let mut routing = BTreeMap::new();
    let mut refused = false;
    for (word, verdict) in VERDICT_CARRIED {
        let key = map.at(word);
        let Some(found) = map.optional(word) else {
            continue;
        };
        match yaml::text(&key, found, out) {
            Some(target) => {
                routing.insert(*verdict, StepId::new(target));
            }
            None => refused = true,
        }
    }
    map.close(VERDICT_KEYS, out);
    if refused {
        return None;
    }
    Some(routing)
}
