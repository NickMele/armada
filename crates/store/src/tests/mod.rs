//! What this crate proves about itself.
//!
//! Six files, and two of them are the point of a milestone step: `reconstruct`
//! writes a Job's whole history, drops every in-memory copy, reopens the file
//! and rebuilds the same `Job` from the log alone, and `cursor` does the same
//! for the inner machine — a Job that advanced a step comes back on the step it
//! advanced to. The others exist so those mean something — `roundtrip` shows
//! the row survives, `corrupt` shows a damaged file is refused rather than
//! opened empty, and `migrate` shows what a migration does to a Job an earlier
//! one wrote.
//!
//! Fixtures here fill **every** optional field. A Job built with `None`
//! everywhere round-trips through almost no code, and a round-trip test that
//! passes on an empty record is the kind of green v1 shipped 2,181 of.

pub(crate) mod attempt;
mod corrupt;
mod cursor;
mod delivery;
mod footprint;
mod forget;
mod gaming;
mod iteration;
mod migrate;
mod plan;
mod process;
mod reconstruct;
mod report;
mod roundtrip;
mod spend;
mod tmp;

use core_model::{
    AcceptanceCriterion, Actor, AdvanceGate, Attachment, ContextSource, Covers, CriterionId,
    CriterionSource, DeclarePlanAt, DependencyDirection, DependencyEdge, DispatchOrigin,
    EvidenceRef, EvidenceScope, EvidenceType, Facts, FrozenWorkflow, GamingCheck, GamingPattern,
    GateManifest, GateOutcome, Job, JobId, JudgeCheck, JudgeCriterion, ManifestId, ModelName,
    NewJob, NotRunReason, PathPattern, Prerequisite, RepoPath, ResolvedCheck, ResolvedStep,
    ScopeRevision, ScopeRevisionOutcome, StepId, StepSeed, Subject, Timestamp, Title,
    TopLevelOrigin, Ulid, Urgency, WorkflowId, WriteTargets,
};

use crate::Store;
pub use tmp::TempDir;

pub fn ulid(value: &str) -> Ulid {
    Ulid::carried(value)
}

pub fn job_id(value: &str) -> JobId {
    JobId::carried(ulid(value))
}

/// A title, or a panic. Blank is a case under test in `core-model` and in
/// `migrate`, never something a fixture here is asked to carry.
pub fn title(text: &str) -> Title {
    Title::new(text).expect("a fixture title is never blank")
}

pub fn at(value: &str) -> Timestamp {
    Timestamp::from_rfc3339(value)
}

/// The creation instant every fixture uses, so a test that cares about a
/// timestamp is naming one it set.
pub fn created_at() -> Timestamp {
    at("2026-08-26T09:00:00.000Z")
}

pub fn open(dir: &TempDir) -> Store {
    Store::open(&dir.db()).expect("a fresh store opens")
}

/// The workflow every fixture Job freezes: the two steps its `job_steps` rows
/// name, one of them gated, so a round trip carries a Check declaration and a
/// Judge criterion too.
pub fn workflow() -> FrozenWorkflow {
    FrozenWorkflow::frozen(
        WorkflowId::carried(ulid("01WORKFLOW")),
        "bug".to_string(),
        1,
        vec![
            ResolvedStep::frozen(
                StepId::new("reproduce"),
                "Reproduce".to_string(),
                Some(EvidenceType::FailingTest),
                Vec::new(),
                AdvanceGate::Auto,
                Vec::new(),
                None,
                0,
                None,
            ),
            ResolvedStep::frozen(
                StepId::new("fix"),
                "Fix".to_string(),
                Some(EvidenceType::Diff),
                vec![
                    ResolvedCheck::ManifestCheck {
                        name: "build".to_string(),
                        run: "cargo build".to_string(),
                        expect_exit_code: 0,
                        // Carried on the shared fixture rather than in a test
                        // of its own, so every roundtrip in this crate walks a
                        // Check that declares which paths it covers.
                        when: Covers::of(vec![PathPattern::parse("crates/**").expect("a pattern")]),
                        // Carried here for `when`'s reason, and a list of one
                        // is enough to catch the failure that matters: a
                        // prerequisite dropped on the way to a row is a Job
                        // that stops running its own fix and nothing says so.
                        requires: vec![Prerequisite::resolved(
                            "fmt".to_string(),
                            "cargo fmt --all".to_string(),
                        )],
                    },
                    ResolvedCheck::DiffNonempty,
                    // Carried on the shared fixture for `when`'s reason: every
                    // round trip in this crate then walks a check whose whole
                    // content is a path, which is the one kind that comes back
                    // wrong if the writer and the reader disagree about a key
                    // name.
                    ResolvedCheck::ArtifactExists {
                        target: ".armada/artifacts/fix.md".to_string(),
                    },
                ],
                AdvanceGate::AutoIfJudgePasses,
                vec![JudgeCheck::declared(
                    Some(ModelName::new("haiku").expect("a model name")),
                    2,
                    vec![JudgeCriterion {
                        criterion_id: CriterionId::new("c1"),
                        question: "Does the fix address the cause the note names?".to_string(),
                    }],
                    Some(GamingCheck::declared(
                        EvidenceRef::parse("root_cause.evidence"),
                        vec![
                            GamingPattern::AssertionWeakened,
                            GamingPattern::CheckConfigEdited,
                        ],
                    )),
                )],
                // One step carries a scope and one carries none, so a round
                // trip proves both the value and its absence survive the
                // column rather than only the value.
                Some(EvidenceScope::declared(
                    ContextSource::DroneDeclared,
                    vec![RepoPath::new("secrets")],
                    vec![EvidenceRef::parse("root_cause.evidence").expect("a reference")],
                    true,
                    Some(DeclarePlanAt::StepStart),
                )),
                0,
                // Carried on the shared fixture for `when`'s reason: one step
                // names a model and one names none, so every round trip in
                // this crate walks both the value and its absence. They are not
                // the same sentence — absent is the step deferring to the Job's
                // — and a column that lost the difference would silently spawn
                // every step on the Job's model again.
                Some(ModelName::new("the-steps-own-model").expect("a model name")),
            )
            // Carried on the shared fixture for `when`'s reason again, and
            // with the halves set to different numbers: one step declares both
            // and one declares neither, so every round trip in this crate
            // walks the value and its absence. Absent is the step deferring to
            // what Fleet is running with, which is not the same sentence as a
            // number, and a column that lost the difference would put every
            // step back on one constant.
            .quiet_after(Some(900))
            .poking(Some(4)),
        ],
    )
}

/// A Job with nothing left null: every `Option` filled, every array non-empty.
pub fn full_new_job(id: &str) -> NewJob {
    NewJob {
        id: job_id(id),
        title: title("fix the off-by-one in the log reader"),
        workflow: workflow(),
        owner_manifest_id: ManifestId::carried(ulid("01OWNERMANIFEST")),
        urgency: Urgency::Incident,
        atomic: true,
        model: ModelName::new("a-model-name").expect("a model name"),
        acceptance_criteria: vec![
            AcceptanceCriterion {
                criterion_id: CriterionId::new("c1"),
                text: "the reported symptom no longer occurs".to_string(),
                source: CriterionSource::Check,
            },
            AcceptanceCriterion {
                criterion_id: CriterionId::new("c2"),
                text: "a test covers the reported symptom".to_string(),
                source: CriterionSource::Judge,
            },
        ],
        steps: vec![
            StepSeed {
                step_id: StepId::new("reproduce"),
                ordinal: 0,
            },
            StepSeed {
                step_id: StepId::new("fix"),
                ordinal: 1,
            },
        ],
        dependencies: vec![
            DependencyEdge {
                direction: DependencyDirection::DependsOn,
                peer: job_id("01UPSTREAM"),
            },
            DependencyEdge {
                direction: DependencyDirection::Blocks,
                peer: job_id("01DOWNSTREAM"),
            },
        ],
        gate_manifests: vec![
            GateManifest {
                manifest_id: ManifestId::carried(ulid("01GATEONE")),
                outcome: GateOutcome::RanAndPassed,
            },
            GateManifest {
                manifest_id: ManifestId::carried(ulid("01GATETWO")),
                outcome: GateOutcome::DidNotRun(NotRunReason::ScopeNarrowed),
            },
        ],
        write_targets: Some(WriteTargets::of(vec![
            RepoPath::new("crates/store/src/lib.rs"),
            RepoPath::new("docs/practices/rust.md"),
        ])),
        subject: Some(Subject {
            kind: "issue".to_string(),
            reference: "13".to_string(),
        }),
        redispatched_from: Some(job_id("01REPLACED")),
        facts: Facts::new("the daemon writes its own log line\nand reads it back"),
        scope_revisions: vec![ScopeRevision {
            at_step: Some(StepId::new("fix")),
            paths_added: vec![RepoPath::new("crates/store/src/read.rs")],
            paths_removed: vec![RepoPath::new("crates/store/src/gone.rs")],
            atomic_before: false,
            atomic_after: true,
            rationale: "the fix touches the reader too".to_string(),
            outcome: ScopeRevisionOutcome::recorded("approved"),
            approved_by: Actor::Human,
            at: at("2026-08-26T09:30:00.000Z"),
        }],
        attachments: vec![Attachment {
            filename: "before.png".to_string(),
            mime_type: "image/png".to_string(),
            byte_size: 4096,
            storage_ref: "/var/armada/attachments/01FULL/before.png".to_string(),
        }],
    }
}

/// A top-level Job, at `awaiting_approval`.
pub fn top_level(id: &str) -> Job {
    Job::create_top_level(full_new_job(id), TopLevelOrigin::HelmDrafted, created_at())
}

/// A sub-dispatched Job, at `queued`.
pub fn sub_dispatched(id: &str) -> Job {
    Job::create_sub_dispatched(
        full_new_job(id),
        DispatchOrigin {
            job_id: job_id("01PARENT"),
            step_id: StepId::new("plan"),
        },
        created_at(),
    )
}
