//! Every field the Job row holds, written and read back.
//!
//! The fixtures fill every `Option` and leave no array empty, because a
//! round-trip over an empty record exercises almost nothing and passes anyway.

use core_model::{
    Actor, AdvanceGate, Attachment, CheckOutcome, ContextSource, Covers, CriterionId,
    DeclarePlanAt, DroneId, EvidenceRef, EvidenceType, GamingPattern, Job, JobStatus, JobStep,
    JudgeVerdict, Judgment, ModelName, PathPattern, RedirectWaiting, RepoPath, ResolvedCheck,
    StepCheck, StepEvidence, StepId, Target, Ulid, WriteTargets,
};

use crate::tests::{created_at, job_id, open, sub_dispatched, top_level, TempDir};
use crate::{LoadAllError, WriteError};

#[test]
fn a_top_level_job_survives_with_every_field_intact() {
    let dir = TempDir::new();
    let stored = top_level("01FULL");
    let mut store = open(&dir);
    store.insert_job(&stored, &created_at()).expect("stored");
    drop(store);

    let reopened = open(&dir);
    assert_eq!(reopened.load_job(&job_id("01FULL")).expect("loads"), stored);
}

#[test]
fn a_sub_dispatched_job_keeps_the_step_that_dispatched_it() {
    let dir = TempDir::new();
    let stored = sub_dispatched("01SUB");
    let mut store = open(&dir);
    store.insert_job(&stored, &created_at()).expect("stored");
    drop(store);

    let reopened = open(&dir);
    let loaded = reopened.load_job(&job_id("01SUB")).expect("loads");
    assert_eq!(loaded, stored);
    assert_eq!(loaded.status(), JobStatus::Queued, "its entry status");
    assert_eq!(
        loaded.dispatched_by().map(|by| by.step_id.as_str()),
        Some("plan")
    );
}

/// Null is not empty. Zero rows in `job_write_targets` cannot say which of the
/// two a Job means, so the Job row carries the discriminator — and this is what
/// would fail if it stopped doing so.
#[test]
fn undetermined_scope_and_determined_to_write_nothing_stay_apart() {
    let dir = TempDir::new();
    let mut store = open(&dir);

    let undetermined = with_targets("01NULLSCOPE", None);
    let nothing = with_targets("01NOTHING", Some(WriteTargets::nothing()));
    let something = with_targets(
        "01SOMETHING",
        Some(WriteTargets::of(vec![RepoPath::new(
            "crates/store/src/lib.rs",
        )])),
    );
    for job in [&undetermined, &nothing, &something] {
        store.insert_job(job, &created_at()).expect("stored");
    }
    drop(store);

    let reopened = open(&dir);
    assert!(reopened
        .load_job(&job_id("01NULLSCOPE"))
        .expect("loads")
        .write_targets()
        .is_none());
    assert_eq!(
        reopened
            .load_job(&job_id("01NOTHING"))
            .expect("loads")
            .write_targets()
            .map(|targets| targets.paths().len()),
        Some(0)
    );
    assert_eq!(
        reopened
            .load_job(&job_id("01SOMETHING"))
            .expect("loads")
            .write_targets()
            .map(|targets| targets.paths().len()),
        Some(1)
    );
}

fn with_targets(id: &str, targets: Option<WriteTargets>) -> Job {
    let mut new = crate::tests::full_new_job(id);
    new.write_targets = targets;
    Job::create_top_level(new, core_model::TopLevelOrigin::Manual, created_at())
}

/// A file handed to the Job at proposal time survives a close and reopen —
/// its own table, like `job_write_targets`, and read back the same way.
#[test]
fn an_attachment_survives_the_reopen() {
    let dir = TempDir::new();
    let mut new = crate::tests::full_new_job("01ATTACHED");
    new.attachments = vec![Attachment {
        filename: "repro.png".to_string(),
        mime_type: "image/png".to_string(),
        byte_size: 20480,
        storage_ref: "/var/armada/attachments/01ATTACHED/repro.png".to_string(),
    }];
    let stored = Job::create_top_level(new, core_model::TopLevelOrigin::Manual, created_at());

    let mut store = open(&dir);
    store.insert_job(&stored, &created_at()).expect("stored");
    drop(store);

    let reopened = open(&dir);
    let loaded = reopened.load_job(&job_id("01ATTACHED")).expect("loads");
    assert_eq!(loaded.attachments().len(), 1);
    assert_eq!(loaded.attachments()[0].filename, "repro.png");
    assert_eq!(loaded.attachments()[0].mime_type, "image/png");
    assert_eq!(loaded.attachments()[0].byte_size, 20480);
    assert_eq!(
        loaded.attachments()[0].storage_ref,
        "/var/armada/attachments/01ATTACHED/repro.png"
    );
}

#[test]
fn creation_is_not_an_update() {
    let dir = TempDir::new();
    let mut store = open(&dir);
    let job = top_level("01TWICE");
    store.insert_job(&job, &created_at()).expect("stored");
    match store.insert_job(&job, &created_at()) {
        Err(WriteError::JobAlreadyExists { job_id }) => assert_eq!(job_id.as_str(), "01TWICE"),
        other => panic!("expected a refusal, found {other:?}"),
    }
}

#[test]
fn a_transition_against_a_job_that_was_never_stored_is_refused() {
    let dir = TempDir::new();
    let mut store = open(&dir);
    let moved = top_level("01GHOST")
        .transition(Target::Queued, Actor::Human, created_at())
        .expect("a legal move");
    match store.record_transition(&moved) {
        Err(WriteError::NoSuchJob { job_id }) => assert_eq!(job_id.as_str(), "01GHOST"),
        other => panic!("expected a refusal, found {other:?}"),
    }
}

/// Never edited and never removed — enforced by the database, not by this
/// crate's discipline. There is no method here that would try either; these go
/// straight at the table to show the trigger is what stops them.
#[test]
fn the_log_refuses_to_be_edited_or_removed() {
    let dir = TempDir::new();
    let mut store = open(&dir);
    let job = top_level("01APPENDONLY");
    store.insert_job(&job, &created_at()).expect("stored");
    let moved = job
        .transition(Target::Queued, Actor::Human, created_at())
        .expect("a legal move");
    store.record_transition(&moved).expect("recorded");

    let edit = store
        .conn
        .execute("UPDATE job_events SET status_to = 'killed'", []);
    assert!(edit.is_err(), "a recorded transition is never edited");

    let remove = store.conn.execute("DELETE FROM job_events", []);
    assert!(remove.is_err(), "a recorded transition is never removed");

    assert_eq!(
        store
            .events_for(&job_id("01APPENDONLY"))
            .expect("still there")
            .len(),
        1
    );
}

#[test]
fn the_boot_read_returns_every_job() {
    let dir = TempDir::new();
    let mut store = open(&dir);
    for id in ["01ONE", "01TWO", "01THREE"] {
        store.insert_job(&top_level(id), &created_at()).expect("ok");
    }
    drop(store);

    let mut reopened = open(&dir);
    let loaded = reopened.load_all_jobs().expect("all three rebuild");
    assert_eq!(loaded.jobs.len(), 3);
    assert!(
        loaded.repaired.is_empty(),
        "nothing to repair on a clean file"
    );
}

/// The signature that makes the v1 bug unwritable: a caller cannot end up with
/// a shorter list and no error.
#[test]
fn one_unreadable_job_does_not_hide_and_does_not_take_the_others_down() {
    let dir = TempDir::new();
    let mut store = open(&dir);
    store
        .insert_job(&top_level("01GOOD"), &created_at())
        .expect("ok");
    store
        .insert_job(&top_level("01BAD"), &created_at())
        .expect("ok");
    store
        .conn
        .execute(
            "UPDATE jobs SET status = 'a status nobody has' WHERE job_id = '01BAD'",
            [],
        )
        .expect("scribbled on");

    match store.load_all_jobs() {
        Err(LoadAllError::SomeJobsUnreadable { loaded, failed }) => {
            assert_eq!(loaded.jobs.len(), 1, "the good one still came back");
            assert_eq!(failed.len(), 1, "and the bad one is named, not dropped");
        }
        other => panic!("expected both halves, found {other:?}"),
    }
}

/// What each declared Check did, written against the step and read back.
///
/// **A pass is a row.** Writing only the failures would make a step whose
/// checks all passed indistinguishable from one whose checks never ran, which
/// is the vacuous pass moved from the gate into the record.
#[test]
fn what_a_step_s_checks_did_survives_the_process_that_ran_them() {
    let dir = TempDir::new();
    let stored = top_level("01CHK");
    let step = StepId::new("reproduce");
    let mut store = open(&dir);
    store.insert_job(&stored, &created_at()).expect("stored");
    store
        .record_step_checks(
            &job_id("01CHK"),
            &step,
            &[
                StepCheck {
                    name: "suite".to_string(),
                    outcome: CheckOutcome::NeverRan,
                    expected: Some("`suite` can be run".to_string()),
                    produced: Some("`suite` is not installed".to_string()),
                    output_path: Some(".armada/checks/01CHK/reproduce.0.log".to_string()),
                },
                StepCheck {
                    name: "diff_nonempty".to_string(),
                    outcome: CheckOutcome::Passed,
                    expected: None,
                    produced: None,
                    output_path: None,
                },
            ],
            &created_at(),
        )
        .expect("recorded");
    drop(store);

    let reopened = open(&dir);
    let read = reopened.step_checks(&job_id("01CHK")).expect("loads");
    assert_eq!(read.len(), 1, "one step recorded anything");
    assert_eq!(read[0].0, step);
    assert_eq!(read[0].1.len(), 2, "in the order the step declares them");
    assert_eq!(read[0].1[0].outcome, CheckOutcome::NeverRan);
    assert_eq!(
        read[0].1[0].produced.as_deref(),
        Some("`suite` is not installed"),
        "which of the four not-passes it was, kept in words"
    );
    assert_eq!(
        read[0].1[0].output_path.as_deref(),
        Some(".armada/checks/01CHK/reproduce.0.log"),
        "the row holds where the output was written, not the output"
    );
    assert_eq!(read[0].1[1].outcome, CheckOutcome::Passed);
    assert!(read[0].1[1].expected.is_none(), "a pass measured nothing");
    assert!(
        read[0].1[1].output_path.is_none(),
        "a built-in assertion runs no command and prints nothing"
    );
}

/// What the Judge said survives the process that asked, refusal and
/// no-objection alike.
///
/// **Both are written.** A step the Judge cleared and a step the Judge never
/// ran on are different facts about the same green step, and only the record
/// can tell them apart — which is why a `met` row exists at all.
#[test]
fn what_the_judge_said_survives_the_process_that_asked() {
    let dir = TempDir::new();
    let stored = top_level("01JDG");
    let step = StepId::new("fix");
    let mut store = open(&dir);
    store.insert_job(&stored, &created_at()).expect("stored");
    store
        .record_step_judgments(
            &job_id("01JDG"),
            &step,
            &[
                Judgment {
                    criterion_id: CriterionId::new("c1"),
                    verdict: JudgeVerdict::NotMet,
                    expected: Some("the reader stopping before `end`".to_string()),
                    produced: Some("the bound widened to match the reader".to_string()),
                    consequence: Some("every caller reads one row too many".to_string()),
                    brief_path: Some(".armada/briefs/01JDG/fix.1.c1.txt".to_string()),
                },
                Judgment {
                    criterion_id: CriterionId::new("c2"),
                    verdict: JudgeVerdict::Met,
                    expected: None,
                    produced: None,
                    consequence: None,
                    // A panel of one, and no file was kept for it. The read has
                    // to tell that from the row above rather than filling in a
                    // path the writer never had.
                    brief_path: None,
                },
            ],
            &created_at(),
        )
        .expect("recorded");
    drop(store);

    let reopened = open(&dir);
    let read = reopened.step_judgments(&job_id("01JDG")).expect("loads");
    assert_eq!(read.len(), 1, "one step was judged");
    assert_eq!(read[0].0, step);
    assert_eq!(read[0].1.len(), 2, "in the order the criteria were asked");
    assert_eq!(read[0].1[0].verdict, JudgeVerdict::NotMet);
    assert_eq!(
        read[0].1[0].consequence.as_deref(),
        Some("every caller reads one row too many"),
        "the field a person triages on"
    );
    assert_eq!(
        read[0].1[0].brief_path.as_deref(),
        Some(".armada/briefs/01JDG/fix.1.c1.txt"),
        "the verdict comes back beside the call that produced it"
    );
    assert_eq!(read[0].1[1].verdict, JudgeVerdict::Met);
    assert!(
        read[0].1[1].brief_path.is_none(),
        "a brief nothing kept is absent rather than guessed"
    );
    assert!(
        read[0].1[1].expected.is_none(),
        "there is nothing a no-objection is refusing on"
    );
}

/// A step nobody judged has no rows, which is what the cold tier looks like in
/// the record.
#[test]
fn a_job_no_judge_ran_on_carries_no_judgments_at_all() {
    let dir = TempDir::new();
    let stored = top_level("01NOJ");
    let mut store = open(&dir);
    store.insert_job(&stored, &created_at()).expect("stored");
    drop(store);

    let reopened = open(&dir);
    assert!(reopened
        .step_judgments(&job_id("01NOJ"))
        .expect("loads")
        .is_empty());
}

/// A second ruling **on the same run** supersedes the first. A mixture of the
/// two would be a set of results no single ruling ever produced.
///
/// The step never moves here, so both writes land on run one. A second *run* of
/// the step keeps both, which is `tests::attempt`'s subject and was the defect
/// this once read as intended behaviour.
#[test]
fn a_second_ruling_on_one_run_of_a_step_replaces_the_first_rather_than_joining_it() {
    let dir = TempDir::new();
    let stored = top_level("01AGAIN");
    let step = StepId::new("reproduce");
    let mut store = open(&dir);
    store.insert_job(&stored, &created_at()).expect("stored");
    let failed = [StepCheck {
        name: "suite".to_string(),
        outcome: CheckOutcome::Failed,
        expected: Some("`suite` exits 0".to_string()),
        produced: Some("it exited 1".to_string()),
        output_path: Some(".armada/checks/01AGAIN/reproduce.0.log".to_string()),
    }];
    let passed = [StepCheck {
        name: "suite".to_string(),
        outcome: CheckOutcome::Passed,
        expected: None,
        produced: None,
        output_path: Some(".armada/checks/01AGAIN/reproduce.0.log".to_string()),
    }];
    store
        .record_step_checks(&job_id("01AGAIN"), &step, &failed, &created_at())
        .expect("recorded");
    store
        .record_step_checks(&job_id("01AGAIN"), &step, &passed, &created_at())
        .expect("recorded again");

    let read = store.step_checks(&job_id("01AGAIN")).expect("loads");
    assert_eq!(read[0].1.len(), 1, "one run, not two");
    assert_eq!(read[0].1[0].outcome, CheckOutcome::Passed);
}

// ------------------------------------------------------------ the two added

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
    assert_eq!(
        fix.checks(),
        &[
            ResolvedCheck::ManifestCheck {
                name: "build".to_string(),
                run: "cargo build".to_string(),
                expect_exit_code: 0,
                when: Covers::of(vec![PathPattern::parse("crates/**").expect("a pattern")]),
                requires: Vec::new(),
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

/// `assigned_drone` folds, onto the step the Drone was put on. It was refused
/// on read until a Drone arriving was a row in the log, because a rebuild that
/// cannot put a value back must not quietly drop it.
#[test]
fn a_drone_arriving_and_leaving_folds_back_out_of_the_log() {
    let dir = TempDir::new();
    let stored = top_level("01WITHDRONE");
    let drone = DroneId::carried(Ulid::carried("01DRONE"));
    let mut store = open(&dir);
    store.insert_job(&stored, &created_at()).expect("stored");

    let queued = stored
        .transition(Target::Queued, Actor::Human, created_at())
        .expect("a legal move");
    store.record_transition(&queued).expect("recorded");
    let running = queued
        .job
        .transition(Target::Running, Actor::Fleet, created_at())
        .expect("a legal move");
    store.record_transition(&running).expect("recorded");
    let step = StepId::new("reproduce");
    let arrived = running
        .job
        .drone_spawned(&step, drone.clone(), Actor::Fleet, created_at())
        .expect("nothing is on that step yet");
    store.record_drone_move(&arrived).expect("recorded");
    drop(store);

    let reopened = open(&dir);
    let loaded = reopened.load_job(&job_id("01WITHDRONE")).expect("loads");
    assert_eq!(
        loaded.assigned_drone(),
        Some(&drone),
        "the column is not read back; this came off the log"
    );
    assert_eq!(
        loaded.step(&step).and_then(JobStep::assigned_drone),
        Some(&drone),
        "and it folded onto the step it was put on, which is what lets a \
         finished Job name every Drone that worked it"
    );

    let mut store = reopened;
    let left = loaded
        .drone_exited(&step, Actor::Fleet, created_at())
        .expect("one is on that step");
    store.record_drone_move(&left).expect("recorded");
    drop(store);

    let reopened = open(&dir);
    assert_eq!(
        reopened
            .load_job(&job_id("01WITHDRONE"))
            .expect("loads")
            .assigned_drone(),
        None,
        "and a Drone that left is null again, which is what suspends the \
         liveness clock"
    );
}

/// The baseline a later step's gaming check is measured against outlives the
/// process that recorded it. **A baseline held only in the daemon's memory
/// would vanish on restart and take the check quietly with it**, which is the
/// failure this whole capability exists to catch, happening to Armada itself.
#[test]
fn the_evidence_a_step_submitted_survives_the_process_that_gated_it() {
    let dir = TempDir::new();
    let stored = top_level("01EVD");
    let step = StepId::new("root_cause");
    let note = StepEvidence {
        evidence_type: EvidenceType::FactsNote,
        claimed: "the reader stops one row before `end`".to_string(),
        shown_by: "docs/notes/cursor.md".to_string(),
        not_claimed: String::new(),
    };
    let mut store = open(&dir);
    store.insert_job(&stored, &created_at()).expect("stored");
    store
        .record_step_evidence(&job_id("01EVD"), &step, &note, &created_at())
        .expect("recorded");
    // A resubmission replaces rather than appends: a superseded submission is
    // not a baseline.
    store
        .record_step_evidence(
            &job_id("01EVD"),
            &step,
            &StepEvidence {
                claimed: "the reader stops at `end - 1`".to_string(),
                ..note.clone()
            },
            &created_at(),
        )
        .expect("recorded again");
    drop(store);

    let reopened = open(&dir);
    let read = reopened.step_evidence(&job_id("01EVD")).expect("loads");
    assert_eq!(read.len(), 1, "one step, one row");
    assert_eq!(read[0].0, step);
    assert_eq!(read[0].1.claimed, "the reader stops at `end - 1`");
    assert_eq!(read[0].1.evidence_type, EvidenceType::FactsNote);
    assert!(read[0].1.not_claimed.is_empty(), "empty is a legal answer");

    // A step that submitted nothing is absent rather than present and blank.
    assert!(reopened
        .step_evidence(&job_id("01EVD"))
        .expect("loads")
        .iter()
        .all(|(id, _)| id == &step));
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

/// A blank in the column is a refusal rather than a none. `""` is a workflow
/// that meant to say something, and reading it as "use the Job's" would be the
/// dial silently not applying.
#[test]
fn a_blank_model_in_the_column_is_malformed_rather_than_read_as_none() {
    let stored = WITHOUT_WHEN.replace(r#""retry_limit": 0"#, r#""retry_limit": 0, "model": " ""#);
    let refused = crate::columns::read_workflow(&stored).expect_err("a blank model");
    assert!(refused.contains("model"), "{refused}");
}

/// A note with nowhere to go survives the process that took it.
///
/// **The column is the field's authority**, in the sense `branch` is: no event
/// carries a person's words, so the rebuild reads this straight back rather
/// than folding it. A Fleet that restarts between a person writing a note and a
/// Drone opening with it is the ordinary case, not an exotic one — a person may
/// take a day over a review.
#[test]
fn a_note_waiting_for_the_next_drone_survives_a_reopen() {
    let dir = TempDir::new();
    let stored = top_level("01NOTE");
    let mut store = open(&dir);
    store.insert_job(&stored, &created_at()).expect("stored");

    let waiting = stored
        .redirect_waits(RedirectWaiting::saying("name the cause, not the symptom").expect("a note"))
        .expect("nothing was waiting");
    store
        .record_redirect_waiting(&waiting)
        .expect("the note is written down");
    drop(store);

    let mut reopened = open(&dir);
    let loaded = reopened.load_job(&job_id("01NOTE")).expect("loads");
    assert_eq!(
        loaded.redirect_waiting().map(RedirectWaiting::text),
        Some("name the cause, not the symptom")
    );
    assert_eq!(loaded, waiting, "and nothing else about the Job moved");

    // Delivering it is the same write with nothing in it, which is why there is
    // one method rather than a setter and a clearer.
    reopened
        .record_redirect_waiting(&loaded.redirect_delivered())
        .expect("the note is cleared");
    drop(reopened);

    let after = open(&dir);
    assert_eq!(
        after.load_job(&job_id("01NOTE")).expect("loads"),
        stored,
        "cleared on delivery, and the record is back where it started"
    );
}

/// A Job holding an undelivered note, reinserted, still holds it. `insert_job`
/// binds every column for this reason — a rebuild that dropped one would lose
/// what a person typed.
#[test]
fn a_reinserted_job_does_not_lose_the_note_it_was_holding() {
    let dir = TempDir::new();
    let waiting = top_level("01REINSERT")
        .redirect_waits(RedirectWaiting::saying("do the writer too").expect("a note"))
        .expect("nothing was waiting");
    let mut store = open(&dir);
    store.insert_job(&waiting, &created_at()).expect("stored");
    drop(store);

    assert_eq!(
        open(&dir)
            .load_job(&job_id("01REINSERT"))
            .expect("loads")
            .redirect_waiting()
            .map(RedirectWaiting::text),
        Some("do the writer too")
    );
}

/// A note nobody could act on is refused on the way out, not carried into a
/// brief. `RedirectWaiting::saying` is what makes the blank unrepresentable, so
/// a row holding one was written by something that did not share the type.
#[test]
fn a_blank_note_in_the_column_is_refused_rather_than_rendered() {
    let dir = TempDir::new();
    let mut store = open(&dir);
    store
        .insert_job(&top_level("01BLANKNOTE"), &created_at())
        .expect("stored");
    store
        .conn
        .execute(
            "UPDATE jobs SET redirect_waiting = '   ' WHERE job_id = ?1",
            (job_id("01BLANKNOTE").as_str(),),
        )
        .expect("the column is written past the type");

    let refused = store.load_job(&job_id("01BLANKNOTE"));
    assert!(
        matches!(
            refused,
            Err(crate::LoadJobError::Unreadable(
                crate::RowError::MalformedColumn {
                    column: "redirect_waiting",
                    ..
                }
            ))
        ),
        "a blank note is a malformed column, not an empty block in a brief: {refused:?}"
    );
}
