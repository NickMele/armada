//! The frozen workflow: everything a step declared, read back off the column.
//!
//! **The declaration is the record, not the file it was read from.** A Job that
//! came back knowing only which workflow it followed would have to go to
//! `.armada/workflows/` to find out what its steps declare — which is the one
//! thing that could have changed underneath it, and the whole reason the
//! workflow is frozen onto the Job at all.
//!
//! **A key that did not exist when a row was written reads back as an absence,
//! and an absence is a value.** Some of these are a row frozen before a key
//! existed, and they assert that nothing backfills; the rest are malformed
//! rows, which refuse rather than read as a none, because a widened Check and a
//! dropped one are both unrecoverable from the record afterwards.
//!
//! Those go straight at [`read_workflow`](crate::columns::read_workflow) with
//! no store at all. The subject is the dialect, and opening a file would only
//! make the same assertion slower.

use core_model::{
    AdvanceGate, ContextSource, Covers, DeclarePlanAt, EvidenceRef, GamingPattern, ModelName,
    Narrowing, PathPattern, Prerequisite, RepoPath, ResolvedCheck, StepId,
};

use crate::tests::{created_at, job_id, open, top_level, TempDir};

/// The whole declaration, not the id beside it.
///
/// A Job that came back knowing only which workflow it followed would have to
/// read the file to find out what its steps declare, which is the thing that
/// could have changed underneath it.
#[test]
fn the_frozen_workflow_comes_back_with_every_check_its_steps_declared() {
    let dir = TempDir::new();
    let stored = top_level("01FROZEN");
    let mut store = open(&dir);
    store.insert_job(&stored, &created_at()).expect("stored");
    drop(store);

    let reopened = open(&dir);
    let loaded = reopened.load_job(&job_id("01FROZEN")).expect("loads");
    let workflow = loaded.workflow();
    assert_eq!(workflow.id().as_str(), "01WORKFLOW");
    assert_eq!(workflow.name(), "bug");
    assert_eq!(workflow.version(), 1);
    assert_eq!(workflow.steps().len(), 2);

    let fix = workflow.step(&StepId::new("fix")).expect("the gated step");
    assert_eq!(fix.label(), "Fix");
    // The loop the fixture's gated step declares round-trips too, and it is
    // asserted in `tests::iteration` beside the counts it bounds.
    assert_eq!(
        fix.checks(),
        &[
            ResolvedCheck::ManifestCheck {
                name: "build".to_string(),
                run: "cargo build".to_string(),
                expect_exit_code: 0,
                when: Covers::of(vec![PathPattern::parse("crates/**").expect("a pattern")]),
                requires: vec![Prerequisite::resolved(
                    "fmt".to_string(),
                    "cargo fmt --all".to_string(),
                )],
                narrow: Some(Narrowing::declared(
                    "cargo build".to_string(),
                    "-p {}".to_string(),
                    Covers::of(vec![PathPattern::parse("crates/**").expect("a pattern")]),
                    Some("crates".to_string()),
                    vec!["acceptance".to_string()],
                )),
            },
            ResolvedCheck::DiffNonempty,
            ResolvedCheck::ArtifactExists {
                target: ".armada/artifacts/fix.md".to_string(),
            },
        ],
        "the command lifted out of the Manifest, not the name it was written as"
    );
    // **The path is the whole of an artifact check**, so a row that came back
    // without it would be a step whose deliverable nothing could look for —
    // and a check with nothing to assert reads as one that passed.
    assert_eq!(
        fix.checks()[2].name(),
        Some(".armada/artifacts/fix.md"),
        "the file the step was asked to write survives the round trip"
    );
    // **Which paths the Check covers is frozen with the command.** A row that
    // came back without it would leave the gate deciding against the live
    // `armada.yml`, which is the moved-gate failure the workflow is frozen to
    // prevent.
    assert!(fix.checks()[0].covers(&["crates/store/src/read.rs".to_string()]));
    assert!(!fix.checks()[0].covers(&["packages/components/src/Badge.tsx".to_string()]));
    // **And so is what runs before it.** Both halves: a row that lost the name
    // gives a failure nobody can attribute, and one that lost the command line
    // reads as a prerequisite that ran while running nothing.
    let requires = fix.checks()[0].requires();
    assert_eq!(requires.len(), 1);
    assert_eq!(requires[0].name(), "fmt");
    assert_eq!(requires[0].run(), "cargo fmt --all");
    // The bar the Judge measures against is frozen with the rest of the step.
    // A criterion edited in `.armada/workflows/` changes the next Job, not this
    // one, which is the whole reason the declaration is on the record.
    assert_eq!(fix.advance_gate(), AdvanceGate::AutoIfJudgePasses);
    assert_eq!(fix.judge_checks().len(), 1);
    assert_eq!(fix.judge_checks()[0].panel_size(), 2);
    assert_eq!(
        fix.judge_checks()[0].model().map(ModelName::as_str),
        Some("haiku"),
        "the per-step model dial"
    );
    assert_eq!(
        fix.judge_calls(),
        3,
        "one criterion at two judges, plus the one gaming pattern the diff cannot answer"
    );
    // The gaming check survives the column too, baseline and patterns both —
    // a step that came back declaring no second look would read as a step that
    // never asked for one.
    let gaming = fix.judge_checks()[0].gaming().expect("a gaming check");
    assert_eq!(
        gaming.baseline().map(EvidenceRef::as_wire).as_deref(),
        Some("root_cause.evidence")
    );
    assert_eq!(
        gaming.flag_if(),
        [
            GamingPattern::AssertionWeakened,
            GamingPattern::CheckConfigEdited
        ]
    );
    assert_eq!(gaming.calls(), 1, "the diff answers check_config_edited");
    // Absence is a value too: a column reading as an empty scope would put a
    // footprint check on a step that asked for none.
    let scope = fix.evidence_scope().expect("the step declared one");
    assert_eq!(scope.context_source(), ContextSource::DroneDeclared);
    assert_eq!(scope.declare_plan_at(), Some(DeclarePlanAt::StepStart));
    assert_eq!(scope.exclude_paths(), &[RepoPath::new("secrets")]);
    assert!(scope.scope_diff_check());
    let reproduce = workflow
        .step(&StepId::new("reproduce"))
        .expect("the first step");
    assert!(reproduce.evidence_scope().is_none());
    assert!(
        !reproduce.asks_the_judge(),
        "a step that declared no criterion did not gain one"
    );
    assert!(
        workflow
            .step(&StepId::new("reproduce"))
            .expect("the ungated step")
            .checks()
            .is_empty(),
        "and an ungated step comes back declaring nothing, which is not the          same sentence as Fleet being unable to say"
    );
    assert_eq!(
        loaded.workflow_id().as_str(),
        "01WORKFLOW",
        "read off the frozen workflow, so the join key cannot disagree with it"
    );
}

/// A frozen workflow as a store written before `when` existed holds one: one
/// Manifest Check, and no key saying which paths it covers.
const WITHOUT_WHEN: &str = r#"{
  "workflow_id": "01J000000000000000000WF01",
  "name": "bug",
  "version": 1,
  "steps": [{
    "id": "fix",
    "label": "Fix",
    "evidence_type": "diff",
    "advance_gate": "auto",
    "retry_limit": 0,
    "evidence_scope": null,
    "judge_checks": [],
    "checks": [{
      "type": "manifest_check",
      "check": "build",
      "run": "cargo build",
      "expect_exit_code": 0
    }]
  }]
}"#;

#[test]
fn a_workflow_frozen_before_when_existed_reads_back_as_a_check_that_always_runs() {
    // **The whole of what makes this additive.** Nothing backfills, because
    // there is nothing to backfill: an absent key and a Check that declares no
    // `when` are the same sentence, and it is "always".
    let workflow = crate::columns::read_workflow(WITHOUT_WHEN).expect("a pre-`when` row");
    let check = &workflow
        .step(&StepId::new("fix"))
        .expect("the step")
        .checks()[0];
    assert_eq!(check.when(), None);
    assert!(!check.needs_changed_paths());
    assert!(check.covers(&[]), "a Check with no `when` runs on any step");
}

#[test]
fn a_stored_pattern_the_dialect_cannot_read_is_malformed_rather_than_dropped() {
    // Dropping it would widen the Check to everything and dropping the Check
    // would narrow it to nothing. Neither is recoverable by reading the record,
    // so the row refuses instead.
    let stored = WITHOUT_WHEN.replace(
        r#""expect_exit_code": 0"#,
        r#""expect_exit_code": 0, "when": ["src/[ab].rs"]"#,
    );
    let refused = crate::columns::read_workflow(&stored).expect_err("an unreadable pattern");
    assert!(refused.contains("src/[ab].rs"), "{refused}");
}

/// **A step's own model survives the column, and so does its absence.**
///
/// The two are different sentences: a name is what this step asked to be run
/// as, and none is the step deferring to the Job's. A writer and a reader that
/// disagreed about the key would collapse the first into the second, and every
/// step would quietly go back to being spawned on the Job's model — which is
/// exactly the state this field was added to leave.
#[test]
fn a_steps_own_model_and_its_absence_both_survive_the_column() {
    let workflow =
        crate::columns::read_workflow(&crate::columns::write_workflow(&crate::tests::workflow()))
            .expect("a workflow that was just written");
    assert_eq!(
        workflow
            .step(&StepId::new("fix"))
            .expect("the step")
            .model()
            .map(|model| model.as_str()),
        Some("the-steps-own-model")
    );
    assert_eq!(
        workflow
            .step(&StepId::new("reproduce"))
            .expect("the step")
            .model(),
        None
    );
}

/// A row frozen before a step could name a model reads back as one that names
/// none — which is what every such step meant, since one process spanned the
/// whole Job and could not have changed model partway.
#[test]
fn a_workflow_frozen_before_a_step_could_name_a_model_reads_back_as_naming_none() {
    let workflow = crate::columns::read_workflow(WITHOUT_WHEN).expect("a pre-`model` row");
    assert_eq!(
        workflow
            .step(&StepId::new("fix"))
            .expect("the step")
            .model(),
        None
    );
}

/// **A step's own patience survives the column, both halves and both
/// absences.** They are four different sentences: how long this step's Drone
/// may be quiet, how many nudges it gets, and — twice — the step deferring to
/// what Fleet is running with. A writer and a reader that disagreed about
/// either key would collapse a declaration into a deferral, and every step
/// would go back to the one constant `#60` was opened about, with nothing
/// saying so.
///
/// The pre-`#60` row is asserted beside it, because that is the same failure
/// arriving by the other road: an absent key is a step that declared nothing,
/// which every row written before today is.
#[test]
fn a_steps_own_patience_and_its_absence_both_survive_the_column() {
    let workflow =
        crate::columns::read_workflow(&crate::columns::write_workflow(&crate::tests::workflow()))
            .expect("a workflow that was just written");
    let declared = workflow.step(&StepId::new("fix")).expect("the step");
    assert_eq!(declared.quiet_after_seconds(), Some(900));
    assert_eq!(declared.poke_limit(), Some(4));

    let deferring = workflow.step(&StepId::new("reproduce")).expect("the step");
    assert_eq!(deferring.quiet_after_seconds(), None);
    assert_eq!(deferring.poke_limit(), None);

    let before = crate::columns::read_workflow(WITHOUT_WHEN).expect("a pre-`#60` row");
    let step = before.step(&StepId::new("fix")).expect("the step");
    assert_eq!(step.quiet_after_seconds(), None);
    assert_eq!(step.poke_limit(), None);
}

/// **Which step sends the work out survives the column, and so does its
/// absence on the steps that do not.** A `false` read back over a `true` is a
/// Job whose branch never goes out and nothing saying why; a `true` read back
/// over a `false` is a design document opened for review.
#[test]
fn which_step_sends_the_work_out_survives_the_column() {
    let workflow =
        crate::columns::read_workflow(&crate::columns::write_workflow(&crate::tests::workflow()))
            .expect("a workflow that was just written");
    assert!(workflow
        .step(&StepId::new("fix"))
        .expect("the step")
        .delivers());
    assert!(!workflow
        .step(&StepId::new("reproduce"))
        .expect("the step")
        .delivers());
}

/// **A row frozen before a workflow could say reads back as the last step
/// delivering**, which is what every such Job was created under: the last
/// step's advance is what landed the work.
///
/// The two other readings are both a Job broken in flight. `false` everywhere
/// stops an approved Job from ever pushing its branch, and refusing the row
/// stops it from loading at all. A record says what it meant when it was
/// written, which is the same rule `when` and `iteration_cap` are read by.
///
/// **Read over the whole step list, not off one step.** A step alone cannot
/// tell "this step does not deliver" from "nothing on this record could say",
/// and the two need opposite answers.
#[test]
fn a_workflow_frozen_before_a_step_could_say_delivers_on_its_last_step() {
    let workflow = crate::columns::read_workflow(WITHOUT_WHEN).expect("a pre-`delivers` row");
    assert!(
        workflow
            .step(&StepId::new("fix"))
            .expect("the step")
            .delivers(),
        "the one step of this row is its last, and the last step is what landed the work"
    );
}

/// And a row where a step *does* say is left exactly as it says, including the
/// last step saying no — which is what four of the eight shipped workflows
/// declare and what the backfill above must never overwrite.
#[test]
fn a_row_that_says_no_step_delivers_is_not_backfilled() {
    let stored = WITHOUT_WHEN.replace(
        r#""retry_limit": 0"#,
        r#""retry_limit": 0, "delivers": false"#,
    );
    let workflow =
        crate::columns::read_workflow(&stored).expect("a workflow that delivers nothing");
    assert!(!workflow
        .step(&StepId::new("fix"))
        .expect("the step")
        .delivers());
}

/// A value that is there and is not a boolean is a refusal rather than a
/// `false`, for the stored retry budget's reason: a Job frozen with a
/// delivering step must not lose it quietly.
#[test]
fn a_delivery_declaration_that_is_not_a_boolean_is_malformed() {
    let stored = WITHOUT_WHEN.replace(
        r#""retry_limit": 0"#,
        r#""retry_limit": 0, "delivers": "yes""#,
    );
    let refused =
        crate::columns::read_workflow(&stored).expect_err("a delivery that is not a flag");
    assert!(refused.contains("delivers"), "{refused}");
}

/// A blank in the column is a refusal rather than a none. `""` is a workflow
/// that meant to say something, and reading it as "use the Job's" would be the
/// dial silently not applying.
#[test]
fn a_blank_model_in_the_column_is_malformed_rather_than_read_as_none() {
    let stored = WITHOUT_WHEN.replace(r#""retry_limit": 0"#, r#""retry_limit": 0, "model": " ""#);
    let refused = crate::columns::read_workflow(&stored).expect_err("a blank model");
    assert!(refused.contains("model"), "{refused}");
}
