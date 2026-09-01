//! **A step that runs twice keeps both runs.**
//!
//! The defect this file is the test of: all four per-step tables were keyed by
//! step alone and replaced whole on a second visit, so iteration two erased
//! iteration one in every one of them. A step that failed the same criterion
//! four times and a step that failed it once were the same record.
//!
//! Every run here is reached by transitioning, never by writing a state into a
//! row: the second run exists because the step went `running -> stopped ->
//! running` through the machine, which is what makes the attempt ordinal a fact
//! about the log rather than a number this test chose.
//!
//! # Two things are asserted, and only one of them is "the rows are there"
//!
//! That both runs survive is the first. The second is that the **latest-run**
//! reads still answer where the step stands — a baseline that started returning
//! two submissions, or a Board that started rendering a superseded verdict,
//! would be this fix breaking the thing it was protecting.

use core_model::{
    Actor, CheckOutcome, CriterionId, EscalationTrigger, EvidenceType, GamingFlag, GamingPattern,
    Job, JudgeVerdict, Judgment, StepCheck, StepEvidence, StepId, StepLevelTrigger, StepTarget,
    Target,
};

use crate::tests::{at, created_at, job_id, open, top_level, TempDir};
use crate::Store;

pub(super) fn step_id() -> StepId {
    StepId::new("fix")
}

/// A stored Job at `running`, with its first step already entered — one run on
/// the record.
pub(super) fn on_its_first_run(store: &mut Store, id: &str) -> Job {
    let created = top_level(id);
    store
        .insert_job(&created, &created_at())
        .expect("the job is stored");
    let mut job = created;
    for (target, when) in [
        (Target::Queued, at("2026-08-26T10:00:00.000Z")),
        (Target::Running, at("2026-08-26T10:01:00.000Z")),
    ] {
        let moved = job
            .transition(target, Actor::Fleet, when)
            .expect("a legal move");
        store.record_transition(&moved).expect("recorded");
        job = moved.job;
    }
    moved_step(store, &job, StepTarget::Running, "2026-08-26T10:02:00.000Z")
}

/// Stop the step and start it again — the two moves that make a second run, and
/// the only shape `STEP_EDGES` admits for one.
pub(super) fn run_it_again(
    store: &mut Store,
    job: &Job,
    stopped_at: &str,
    started_at: &str,
) -> Job {
    let why = StepLevelTrigger::of(EscalationTrigger::GateFailure)
        .expect("a gate failure is a step-level trigger");
    let job = moved_step(store, job, StepTarget::Stopped(why), stopped_at);
    moved_step(store, &job, StepTarget::Running, started_at)
}

fn moved_step(store: &mut Store, job: &Job, to: StepTarget, when: &str) -> Job {
    let moved = job
        .transition_step(&step_id(), to, Actor::Fleet, at(when))
        .unwrap_or_else(|cause| panic!("moving the step: {cause}"));
    store
        .record_step_transition(&moved)
        .expect("the step move is recorded");
    moved.job
}

/// One run's worth of every kind of record, said in a way the run can be read
/// back out of.
///
/// `pub(super)` because `forget` needs a Job with rows in all four per-step
/// tables, and the two tests should be asking about the same four: a fifth
/// table added here reaches the forget test without anybody remembering to
/// carry it across.
pub(super) fn record_a_whole_run(store: &mut Store, id: &str, saying: &str, when: &str) {
    let job = job_id(id);
    let step = step_id();
    store
        .record_step_checks(
            &job,
            &step,
            &[StepCheck {
                name: "build".to_string(),
                outcome: CheckOutcome::Failed,
                expected: Some("exit 0".to_string()),
                produced: Some(saying.to_string()),
                output_path: Some(format!(".armada/checks/{id}/fix.0.log")),
            }],
            &at(when),
        )
        .expect("checks recorded");
    store
        .record_step_judgments(
            &job,
            &step,
            &[Judgment {
                criterion_id: CriterionId::new("c1"),
                verdict: JudgeVerdict::NotMet,
                expected: Some("the cause is addressed".to_string()),
                produced: Some(saying.to_string()),
                consequence: Some("the symptom returns".to_string()),
                brief_path: Some(format!(".armada/briefs/{id}/fix.1.c1.txt")),
            }],
            &at(when),
        )
        .expect("judgments recorded");
    store
        .record_step_gaming_flags(
            &job,
            &step,
            &[GamingFlag {
                pattern: GamingPattern::AssertionWeakened,
                cited: saying.to_string(),
            }],
            &at(when),
        )
        .expect("flags recorded");
    store
        .record_step_evidence(
            &job,
            &step,
            &StepEvidence {
                evidence_type: EvidenceType::Diff,
                claimed: saying.to_string(),
                shown_by: "the patch".to_string(),
                not_claimed: "nothing else".to_string(),
            },
            &at(when),
        )
        .expect("evidence recorded");
}

/// The whole of the issue, in one test: two runs, four tables, both survive.
#[test]
fn a_step_that_ran_twice_has_both_runs_in_all_four_tables() {
    let dir = TempDir::new();
    let id = "01TWICE";
    {
        let mut store = open(&dir);
        let job = on_its_first_run(&mut store, id);
        record_a_whole_run(&mut store, id, "the first note", "2026-08-26T10:05:00.000Z");
        run_it_again(
            &mut store,
            &job,
            "2026-08-26T10:06:00.000Z",
            "2026-08-26T10:07:00.000Z",
        );
        record_a_whole_run(
            &mut store,
            id,
            "the same note again",
            "2026-08-26T10:09:00.000Z",
        );
        // Every in-memory copy goes with the connection. Nothing below has seen
        // what it is about to assert on.
    }

    let store = open(&dir);
    let job = job_id(id);

    let checks = store.step_checks_every_attempt(&job).expect("loads");
    assert_eq!(checks.len(), 2, "two runs of Checks");
    assert_eq!(checks[0].attempt.number(), 1);
    assert_eq!(checks[1].attempt.number(), 2);
    assert_eq!(
        checks[0].record[0].produced.as_deref(),
        Some("the first note")
    );
    assert_eq!(
        checks[1].record[0].produced.as_deref(),
        Some("the same note again")
    );
    assert_eq!(
        checks[0].at,
        at("2026-08-26T10:05:00.000Z"),
        "each run keeps the instant it was written at"
    );

    let judged = store.step_judgments_every_attempt(&job).expect("loads");
    assert_eq!(judged.len(), 2, "two runs of verdicts");
    assert_eq!(
        judged
            .iter()
            .map(|run| run.record[0].produced.as_deref().expect("a produced"))
            .collect::<Vec<_>>(),
        vec!["the first note", "the same note again"],
        "which is what shows the same note went unaddressed twice"
    );
    assert_eq!(
        judged
            .iter()
            .map(|run| run.record[0].criterion_id.as_str())
            .collect::<Vec<_>>(),
        vec!["c1", "c1"],
        "the same criterion, refused on both runs"
    );

    let flagged = store.step_gaming_flags_every_attempt(&job).expect("loads");
    assert_eq!(flagged.len(), 2, "two runs of flags");
    assert_eq!(flagged[0].record[0].cited, "the first note");
    assert_eq!(flagged[1].record[0].cited, "the same note again");

    let evidence = store.step_evidence_every_attempt(&job).expect("loads");
    assert_eq!(evidence.len(), 2, "two runs of evidence");
    assert_eq!(evidence[0].record.claimed, "the first note");
    assert_eq!(evidence[1].record.claimed, "the same note again");
}

/// The other half. Keeping both runs must not change what "where the step
/// stands" answers — a gaming check's baseline is the latest submission, and a
/// Board row is the latest verdict.
#[test]
fn the_latest_run_is_still_what_the_plain_reads_answer() {
    let dir = TempDir::new();
    let id = "01LATEST";
    {
        let mut store = open(&dir);
        let job = on_its_first_run(&mut store, id);
        record_a_whole_run(&mut store, id, "the first note", "2026-08-26T10:05:00.000Z");
        run_it_again(
            &mut store,
            &job,
            "2026-08-26T10:06:00.000Z",
            "2026-08-26T10:07:00.000Z",
        );
        record_a_whole_run(
            &mut store,
            id,
            "the second note",
            "2026-08-26T10:09:00.000Z",
        );
    }

    let store = open(&dir);
    let job = job_id(id);

    let evidence = store.step_evidence(&job).expect("loads");
    assert_eq!(evidence.len(), 1, "one baseline, not two");
    assert_eq!(evidence[0].1.claimed, "the second note");

    let checks = store.step_checks(&job).expect("loads");
    assert_eq!(checks.len(), 1);
    assert_eq!(checks[0].1.len(), 1, "one run's Checks, not both runs'");
    assert_eq!(checks[0].1[0].produced.as_deref(), Some("the second note"));

    let judged = store.step_judgments(&job).expect("loads");
    assert_eq!(judged[0].1.len(), 1, "one pass, never two read as a panel");
    assert_eq!(judged[0].1[0].produced.as_deref(), Some("the second note"));

    let flagged = store.step_gaming_flags(&job).expect("loads");
    assert_eq!(
        flagged[0].1.len(),
        1,
        "one pass, never two read as one that found twice as much"
    );
    assert_eq!(flagged[0].1[0].cited, "the second note");
}

/// Within one run the old rule still holds: a second ruling supersedes the
/// first, because a mixture of two rulings is a result no single ruling
/// produced. The run is the scope of "afresh", not the step.
#[test]
fn a_second_ruling_inside_one_run_still_supersedes_the_first() {
    let dir = TempDir::new();
    let id = "01WITHIN";
    {
        let mut store = open(&dir);
        on_its_first_run(&mut store, id);
        record_a_whole_run(&mut store, id, "ruled once", "2026-08-26T10:05:00.000Z");
        record_a_whole_run(&mut store, id, "ruled again", "2026-08-26T10:06:00.000Z");
    }

    let store = open(&dir);
    let job = job_id(id);
    let judged = store.step_judgments_every_attempt(&job).expect("loads");
    assert_eq!(
        judged.len(),
        1,
        "one run, whatever the store was told twice"
    );
    assert_eq!(judged[0].attempt.number(), 1);
    assert_eq!(judged[0].record[0].produced.as_deref(), Some("ruled again"));

    let evidence = store.step_evidence_every_attempt(&job).expect("loads");
    assert_eq!(evidence.len(), 1);
    assert_eq!(evidence[0].record.claimed, "ruled again");
}

/// The ordinal is the log's answer, not a counter this crate keeps. A step that
/// has never been entered is on its first run, and each entry into `running`
/// moves it on by one.
#[test]
fn the_attempt_is_counted_off_the_log_and_nowhere_else() {
    let dir = TempDir::new();
    let id = "01COUNTED";
    let mut store = open(&dir);
    let job = job_id(id);

    store
        .insert_job(&top_level(id), &created_at())
        .expect("stored");
    assert_eq!(
        store
            .step_attempt(&job, &step_id())
            .expect("counted")
            .number(),
        1,
        "a step that has never run is on the run about to start"
    );

    let running = on_its_first_run(&mut store, "01COUNTEDTWO");
    let second = job_id("01COUNTEDTWO");
    assert_eq!(
        store
            .step_attempt(&second, &step_id())
            .expect("counted")
            .number(),
        1
    );
    let running = run_it_again(
        &mut store,
        &running,
        "2026-08-26T10:06:00.000Z",
        "2026-08-26T10:07:00.000Z",
    );
    assert_eq!(
        store
            .step_attempt(&second, &step_id())
            .expect("counted")
            .number(),
        2
    );
    run_it_again(
        &mut store,
        &running,
        "2026-08-26T10:08:00.000Z",
        "2026-08-26T10:09:00.000Z",
    );
    assert_eq!(
        store
            .step_attempt(&second, &step_id())
            .expect("counted")
            .number(),
        3,
        "and the count is over that step alone"
    );
    assert_eq!(
        store
            .step_attempt(&second, &StepId::new("reproduce"))
            .expect("counted")
            .number(),
        1,
        "the step beside it has run no times"
    );
}
