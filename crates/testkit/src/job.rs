//! The Job behind a request, for a test that needs one and is not about one.
//!
//! # Why this exists
//!
//! `verification::Request::of` takes a `&Job` and nothing else — that is its
//! source rule, and it is what stops a Drone's own words reaching a Judge
//! dressed as the thing its work is measured against. The cost of a rule like
//! that is that any test asserting on a brief has to hold a real `Job`, and
//! `NewJob` has twenty fields with no `Default` by design.
//!
//! So the twenty fields are written out once, here, the way [`resolved`] writes
//! a workflow out once. Nothing is faked: `Job::create_top_level` is the real
//! constructor and the record it produces is the record `store` would load.
//!
//! [`resolved`]: crate::resolved

use std::sync::LazyLock;

use core_model::{
    AcceptanceCriterion, CriterionId, CriterionSource, Facts, Job, JobId, ManifestId, ModelName,
    NewJob, StepSeed, Timestamp, TopLevelOrigin, Ulid, Urgency,
};

use crate::workflow::{frozen, Sketch};

/// A Job carrying a request, and nothing else worth asserting on.
///
/// The three arguments are exactly what `Request` reads. Everything else is a
/// value chosen so the record is legal, not one a test should look at.
///
/// The criteria are `check`-sourced and identified by position, which is how
/// `fleet::drafting` mints them — a Judge citation names a criterion by its
/// frozen position, so a fixture inventing ids would be a fixture nothing else
/// produces.
pub fn asking(title: &str, facts: &str, criteria: &[&str]) -> Job {
    let workflow = frozen(&[Sketch {
        id: "the-step",
        label: "The step",
        evidence_type: Some("facts_note"),
        gates: &[],
        judged_on: &[],
        scope: None,
        gaming: None,
    }]);
    Job::create_top_level(
        NewJob {
            id: JobId::carried(Ulid::carried("01J0000000000000000000JOB0")),
            title: core_model::Title::new(title).expect("a title"),
            workflow,
            owner_manifest_id: ManifestId::carried(Ulid::carried("01J0000000000000000000MAN0")),
            urgency: Urgency::Normal,
            atomic: false,
            model: ModelName::new("the-configured-model").expect("a model name"),
            acceptance_criteria: criteria
                .iter()
                .enumerate()
                .map(|(position, text)| AcceptanceCriterion {
                    criterion_id: CriterionId::new(format!("c{}", position + 1)),
                    text: (*text).to_string(),
                    source: CriterionSource::Check,
                })
                .collect(),
            steps: vec![StepSeed {
                step_id: core_model::StepId::new("the-step"),
                ordinal: 0,
            }],
            dependencies: Vec::new(),
            gate_manifests: Vec::new(),
            write_targets: None,
            subject: None,
            redispatched_from: None,
            facts: Facts::new(facts),
            scope_revisions: Vec::new(),
            attachments: Vec::new(),
        },
        TopLevelOrigin::Manual,
        Timestamp::from_rfc3339("2026-08-28T00:00:00Z"),
    )
}

/// One Job, made once, for the many call sites that need a request and assert
/// nothing about it.
///
/// **A `&'static Job` rather than a value**, so a call site can write
/// `Request::of(testkit::asked_for())` inline. A `Request` borrows, which is
/// the shape `verification::Reference` already has, and a fixture returning an
/// owned `Job` would force a `let` binding into every one of forty tests that
/// do not care.
pub fn asked_for() -> &'static Job {
    static ASKED: LazyLock<Job> = LazyLock::new(|| {
        asking(
            "make the suite pass",
            "the reader drops the last line",
            &["the suite passes"],
        )
    });
    &ASKED
}
