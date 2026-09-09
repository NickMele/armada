//! The apparatus Landing's claim is asserted against, and none of it asserts
//! anything.
//!
//! Separate from `mod.rs` for the reason `board.rs` and `recovery.rs` are: M1's
//! bench answers "did this Job pass its gates", and Landing asks where the work
//! went afterwards and who ended it.
//!
//! # The two workflows, and why they are the whole apparatus
//!
//! A workflow says which of its steps sends the work out —
//! `ResolvedStep::delivers` — and a workflow may say **no** step does.
//! [`sends_it_out`] and [`sends_nothing`] are those two shapes, built through
//! the same two parsers Fleet loads a definition with, so a fixture that an
//! `armada.yml` could not produce is refused here rather than asserted against.
//!
//! **Nothing here performs a delivery.** The commit, the push and the pull
//! request are `Fleet::land_and_deliver`, which needs a store, a repository and
//! a remote; what this file drives is the record either side of it — which step
//! declares the send, and where the two machines leave a Job that reached it.
//! `landing.rs`'s header carries the whole list of what that leaves unproved.

use config::{EvidenceType, ResolvedWorkflow};
use core_model::Job;
use testkit::{handing_off, Gate, Sketch};
use verification::{Claimed, NotClaimed, ShownBy, Submission};

/// The three steps a Job that hands its work over has: work out the cause,
/// change the code, hand it to a person.
///
/// **`bug.json`'s own step ids, and its own shape at the end.** The real
/// definition's `handoff` declares `delivers: true` and `advance_gate:
/// human_always` together, and the pair is what Landing's claim is about: the
/// branch goes out when the step is *entered*, and the step then holds while
/// somebody reads what went out.
fn three_steps() -> [Sketch<'static>; 3] {
    [
        Sketch {
            id: "root_cause",
            label: "Root cause",
            evidence_type: Some("facts_note"),
            gates: &[],
            judged_on: &[],
            scope: None,
            gaming: None,
        },
        Sketch {
            id: "fix",
            label: "Fix",
            evidence_type: Some("diff"),
            gates: &[Gate::DiffNonempty],
            judged_on: &[],
            scope: None,
            gaming: None,
        },
        Sketch {
            id: "handoff",
            label: "Summarise",
            evidence_type: Some("facts_note"),
            gates: &[],
            judged_on: &[],
            scope: None,
            gaming: None,
        },
    ]
}

/// The workflow whose last step sends the work out and holds for a person.
pub fn sends_it_out() -> ResolvedWorkflow {
    handing_off(&three_steps(), "handoff")
}

/// The same three steps with **no** step declaring a send.
///
/// **The shape half the milestone is about**, and the four shipped workflows
/// that have it — `design-plan`, `code-review`, `epic` and `prototype` — are
/// this: a Job that runs every step, passes every Check and pushes nothing,
/// because what it produced is read rather than merged.
///
/// It is a fixture and not those four files, for the hermetic rule: reading
/// `.armada/workflows/` needs the model roster the adapter resolves, and
/// `config`'s own `shipped.rs` asks that question against the real ones.
pub fn sends_nothing() -> ResolvedWorkflow {
    testkit::delivering(&three_steps(), None)
}

/// Which steps of the workflow a Job froze declare that they send the work out.
///
/// **Read off the Job and not off the definition it was resolved from.** The
/// frozen copy is what Fleet reads at every step entry, so a `delivers` that
/// survived `resolve` and not the freeze would be a workflow that delivers by
/// accident.
pub fn declares_delivery(job: &Job) -> Vec<(&str, bool)> {
    job.workflow()
        .steps()
        .iter()
        .map(|step| (step.id().as_str(), step.delivers()))
        .collect()
}

/// What the last step submits: a summary of work that has already gone out.
pub fn a_handoff_note() -> Submission {
    Submission::submitted(
        EvidenceType::FactsNote,
        Claimed("The reader's bound is fixed and the branch is out for review."),
        ShownBy("crates/store/src/read.rs, six lines, on the Job's own branch"),
        NotClaimed("Nothing was merged: a person does that."),
    )
    .expect("a well-formed handoff submission")
}
