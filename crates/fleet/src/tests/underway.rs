//! A gate's Checks, said as each one starts and finishes.
//!
//! **`#628` in two cases.** A step's Checks ran for minutes while the stream
//! said nothing, and every row flipped at once when the Judge answered. The
//! first case reads what one pass over the Checks published, in order, against
//! the bound on how many run at once; the second holds that saying so changed
//! nothing the gate rules on.
//!
//! Real commands, for `crate::tests::checking`'s reason: the claims here are
//! about processes starting and ending, and a fake runner would be asserting
//! this file's guess at both.

use std::sync::Arc;
use std::time::Duration;

use core_model::{Attempt, ResolvedCheck, StepId};

use crate::checking::{ran, AT_ONCE};
use crate::tests::gate::Stopped;
use crate::tests::tmp::TempDir;
use crate::underway::{Announcing, Underway};

const JOB: &str = "01JOB";
const STEP: &str = "implement";

fn named(name: &str, run: &str) -> ResolvedCheck {
    ResolvedCheck::ManifestCheck {
        name: name.to_string(),
        run: run.to_string(),
        expect_exit_code: 0,
        when: None,
        requires: Vec::new(),
        narrow: None,
    }
}

/// Every `job.checking` message one pass over `checks` published, in order,
/// and the slot once the writer was dropped.
///
/// **Drained to its end rather than to a count**, for the Judge's marking
/// test's reason: both senders are dropped before the read, so a third message
/// nobody expected would be counted rather than missed.
async fn heard_over(checks: &[ResolvedCheck], repo: &TempDir) -> (Vec<ipc::JobChecking>, Underway) {
    let underway = Underway::default();
    let events = api::Broadcaster::new();
    let mut heard = events.subscribe();
    let announcing = Announcing::on(
        ipc::JobId::carried(JOB),
        StepId::new(STEP),
        Attempt::FIRST,
        underway.clone(),
        events.clone(),
        Arc::new(Stopped),
        &repo.path().display().to_string(),
        JOB,
    );
    ran(
        checks,
        &[],
        false,
        false,
        repo.path(),
        Duration::from_secs(30),
        &announcing,
        &std::collections::BTreeMap::new(),
        &[],
    )
    .await;
    drop(announcing);
    drop(events);
    let mut said = Vec::new();
    while let Some(api::Next::Send(delivered)) = heard.next().await {
        if let ipc::Event::JobChecking(one) = delivered.event {
            said.push(one);
        }
    }
    (said, underway)
}

/// **The whole claim.** The gate says it has begun with every Check waiting,
/// then one message per start and one per finish — never more running than
/// the gate has slots, the fifth waiting until one frees — and a last message
/// with nothing in it once the writer is dropped.
#[tokio::test]
async fn each_check_is_said_to_start_and_to_finish_in_order_under_the_slot_bound() {
    let repo = TempDir::new();
    let declared = AT_ONCE + 2;
    let checks: Vec<ResolvedCheck> = (0..declared)
        .map(|n| named(&format!("check_{n}"), "/bin/sleep 0.3"))
        .collect();
    let (said, underway) = heard_over(&checks, &repo).await;

    let first = said
        .first()
        .and_then(|one| one.checking.as_ref())
        .expect("the gate says it has begun");
    assert_eq!(
        first.checks.len(),
        declared,
        "every declared Check has a row"
    );
    assert!(
        first
            .checks
            .iter()
            .all(|check| check.started_at.is_none() && check.ran.is_none()),
        "every Check waits before any starts"
    );
    assert!(
        said.last().expect("a last message").checking.is_none(),
        "the last message says the gate's Checks are over"
    );

    let between: Vec<&ipc::ChecksUnderway> = said[1..said.len() - 1]
        .iter()
        .map(|one| one.checking.as_ref().expect("a set while it runs"))
        .collect();
    assert_eq!(
        between.len(),
        declared * 2,
        "a start and a finish per Check, and nothing per second"
    );

    let mut starts: Vec<Option<usize>> = vec![None; declared];
    let mut finishes: Vec<Option<usize>> = vec![None; declared];
    for (n, state) in between.iter().enumerate() {
        let running = state
            .checks
            .iter()
            .filter(|check| check.started_at.is_some() && check.ran.is_none())
            .count();
        assert!(running <= AT_ONCE, "message {n} says {running} are running");
        for (at, check) in state.checks.iter().enumerate() {
            if check.started_at.is_some() && starts[at].is_none() {
                starts[at] = Some(n);
            }
            if check.ran.is_some() && finishes[at].is_none() {
                finishes[at] = Some(n);
            }
        }
    }
    for at in 0..declared {
        let (began, ended) = (
            starts[at].expect("every Check started"),
            finishes[at].expect("every Check finished"),
        );
        assert!(
            began < ended,
            "check_{at} was said to finish before it started"
        );
    }
    let freed = finishes.iter().flatten().min().copied().expect("a finish");
    assert!(
        starts[AT_ONCE].expect("the first Check past the bound started") > freed,
        "a Check past the bound waits until a slot frees"
    );

    let last = between.last().expect("the last set");
    assert!(
        last.checks
            .iter()
            .all(|check| check.ran.as_ref().map(|row| row.outcome)
                == Some(ipc::CheckOutcome::from(core_model::CheckOutcome::Passed))),
        "what each came to is the row the ruling will write"
    );
    assert!(
        last.checks.iter().all(|check| check
            .output_path
            .as_deref()
            .is_some_and(|path| path.starts_with(&format!(".armada/checks/{JOB}/"))
                && path.contains(".live."))),
        "each was writing its log beside the record, under a name of its own"
    );
    assert_eq!(
        underway.on(&ipc::JobId::carried(JOB), &ipc::StepId::carried(STEP)),
        None,
        "the slot is empty once the writer is dropped"
    );
}

/// **Live status is a view of the run.** The same Checks run with a writer
/// and with none come back as the same observations and the same output, so
/// nothing the gate rules on or `job_step_checks` records can depend on
/// whether anybody was told.
#[tokio::test]
async fn saying_each_check_changes_nothing_the_gate_rules_on() {
    let repo = TempDir::new();
    let checks = vec![
        named("passes", "/bin/echo held"),
        named("fails", "/usr/bin/false"),
    ];

    let told = {
        let events = api::Broadcaster::new();
        let announcing = Announcing::on(
            ipc::JobId::carried(JOB),
            StepId::new(STEP),
            Attempt::FIRST,
            Underway::default(),
            events,
            Arc::new(Stopped),
            &repo.path().display().to_string(),
            JOB,
        );
        ran(
            &checks,
            &[],
            false,
            false,
            repo.path(),
            Duration::from_secs(30),
            &announcing,
            &std::collections::BTreeMap::new(),
            &[],
        )
        .await
    };
    let untold = ran(
        &checks,
        &[],
        false,
        false,
        repo.path(),
        Duration::from_secs(30),
        &Announcing::nowhere(),
        &std::collections::BTreeMap::new(),
        &[],
    )
    .await;

    let read = |done: &[crate::checking::Completed]| {
        done.iter()
            .map(|one| (one.observed.clone(), one.printed.clone()))
            .collect::<Vec<_>>()
    };
    assert_eq!(read(&told), read(&untold));
}
