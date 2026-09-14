//! What a review step tells its Drone about the review it hands in. #903.
//!
//! **The tool's schema says what the fields are; this says what they are for**, and what
//! Fleet refuses, so a Drone writes a review a person can act on rather than one that only
//! parses. Beside `crate::terms` rather than in it, which is at its line budget. **Drafted**,
//! like `RecordingThePlan`: `docs/contracts/agent-prompt.md` has no copy for it.

use core_model::{EvidenceType, ResolvedStep};

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Reviewing(String);

impl Reviewing {
    /// `Some` on a step whose evidence is a review.
    pub fn at(step: &ResolvedStep) -> Option<Reviewing> {
        if step.evidence_type() != Some(EvidenceType::Review) {
            return None;
        }
        Some(Reviewing(String::from(
            "WHAT THIS PART DELIVERS\n\nThis part's product is Armada's review of the \
             change, which a person reads before deciding whether to take it. Hand it in \
             with submit_evidence's review field.\n\n\
             Write every sentence for that person, who has not read the code: short \
             and plain, about what the change does and what it means for them. Keep \
             function, type and file names out of reasons, area names and findings; a \
             view is where code is named.\n\n\
             - says: confident or not_confident, with at most three reasons of one \
             sentence each.\n\
             - areas: every changed file belongs to one. Name an area the way a person \
             would call that part of the product, never by a file or a function, and \
             say in one sentence what changed there.\n\
             - tests: what the tests in the change prove, every test it removed or \
             loosened and why, and changed code no test reaches, a sentence each.\n\
             - findings: each needs_you, small_fix or for_context, in one or two \
             sentences, and why it is there. A small fix is one a Drone could make \
             without asking anyone.\n\
             - view: where a finding or an area is about code, the hunks in the order \
             one change forces the next, each named by its @@ header exactly as the \
             diff wrote it.\n\n\
             Fleet checks the review against the diff and refuses one that leaves a \
             changed file out, names a test or a hunk the diff does not hold, or gives \
             a finding no reason. Each refusal names what to fix.",
        )))
    }

    /// The block, exactly as it reaches a Drone.
    pub fn text(&self) -> &str {
        &self.0
    }
}
