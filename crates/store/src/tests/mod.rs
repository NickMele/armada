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

mod corrupt;
mod cursor;
mod forget;
mod migrate;
mod reconstruct;
mod roundtrip;
mod tmp;

use core_model::{
    AcceptanceCriterion, Actor, AdvanceGate, ContextSource, CriterionId, CriterionSource,
    DeclarePlanAt, DependencyDirection, DependencyEdge, DispatchOrigin, EvidenceScope,
    EvidenceType, Facts, FrozenWorkflow, GateManifest, GateOutcome, Job, JobId, JudgeCheck,
    JudgeCriterion, ManifestId, ModelName, NewJob, NotRunReason, RepoPath, ResolvedCheck,
    ResolvedStep, ScopeRevision, ScopeRevisionOutcome, StepId, StepSeed, Subject, Timestamp, Title,
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
                    },
                    ResolvedCheck::DiffNonempty,
                ],
                AdvanceGate::AutoIfJudgePasses,
                vec![JudgeCheck::declared(
                    Some(ModelName::new("haiku").expect("a model name")),
                    2,
                    vec![JudgeCriterion {
                        criterion_id: CriterionId::new("c1"),
                        question: "Does the fix address the cause the note names?".to_string(),
                    }],
                )],
                // One step carries a scope and one carries none, so a round
                // trip proves both the value and its absence survive the
                // column rather than only the value.
                Some(EvidenceScope::declared(
                    ContextSource::DroneDeclared,
                    vec![RepoPath::new("secrets")],
                    true,
                    Some(DeclarePlanAt::StepStart),
                )),
            ),
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
