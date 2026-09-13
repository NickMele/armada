//! Verify in the main checkout — Journey 9, *Verify* — driven the way the API
//! drives it: setup and every Check once, one after another, as the
//! checkout's own runs.

use std::path::Path;
use std::process::Command;
use std::sync::Arc;
use std::time::Duration;

use testkit::{FakeHarness, FakeVcs, FakeWorkProduct};

use crate::daemon::Fleet;
use crate::gate::CheckBudget;
use crate::tests::daemon::fittings;
use crate::tests::tmp::TempDir;

type Fixture = Fleet<FakeHarness, FakeVcs, FakeWorkProduct>;

/// `bootstrap` is setup; `broken` exits 3 between two Checks that pass; and
/// `generate` is a Command, which no Verify runs.
const MANIFEST: &str = r#"version: 1
id: 01FIXTUREMANIFEST
checks:
  lint:
    run: "/bin/sh -c 'echo linted'"
  broken:
    run: "/bin/sh -c 'echo breaking; exit 3'"
  after:
    run: "/bin/sh -c 'echo after'"
commands:
  bootstrap:
    run: /usr/bin/true
  generate:
    run: "/bin/sh -c 'echo made > generated.rs'"
setup:
  requires: [bootstrap]
"#;

/// Setup exits 1, so there is an install nothing should be run over.
const SETUP_FAILS: &str = r#"version: 1
id: 01FIXTUREMANIFEST
checks:
  lint:
    run: "/bin/sh -c 'echo linted'"
commands:
  bootstrap:
    run: "/bin/sh -c 'exit 1'"
setup:
  requires: [bootstrap]
"#;

/// A Check that outlives the assertions around it, then one after it.
const SLOW: &str = r#"version: 1
id: 01FIXTUREMANIFEST
checks:
  slow:
    run: "/bin/sh -c 'echo going; sleep 30'"
  after:
    run: "/bin/sh -c 'echo after'"
"#;

fn a_fleet(home: &TempDir, text: &str, events: &api::Broadcaster) -> Arc<Fixture> {
    let manifest = config::Manifest::parse(Path::new("armada.yml"), text)
        .unwrap_or_else(|why| panic!("the fixture manifest did not parse: {why}"));
    let mut fittings = fittings(home, FakeWorkProduct::changed(&[]));
    fittings.starting().manifest = manifest;
    fittings.budget = CheckBudget::of(Duration::from_secs(120));
    fittings.events = events.clone();
    let at = home.path();
    let git = |args: &[&str]| {
        let run = Command::new("git")
            .arg("-C")
            .arg(at)
            .args([
                "-c",
                "user.name=a person",
                "-c",
                "user.email=a@person.invalid",
            ])
            .args(args)
            .output()
            .expect("git on PATH");
        assert!(run.status.success(), "git {args:?} failed");
    };
    git(&["-c", "init.defaultBranch=main", "init", "--quiet"]);
    std::fs::write(at.join("armada.yml"), text).expect("the Manifest");
    git(&["add", "."]);
    git(&["commit", "--quiet", "-m", "the first commit"]);
    Arc::new(Fleet::assembled(fittings))
}

/// The Verify once it has ended.
async fn ended(fleet: &Fixture) -> ipc::CheckoutVerify {
    tokio::time::timeout(Duration::from_secs(60), async {
        loop {
            let sheet = fleet
                .checkout_run_sheet(fleet.first())
                .await
                .expect("a sheet");
            if let Some(verify) = sheet.verify.filter(|one| one.ended_at.is_some()) {
                return verify;
            }
            tokio::time::sleep(Duration::from_millis(20)).await;
        }
    })
    .await
    .expect("the Verify ended")
}

fn names(verify: &ipc::CheckoutVerify) -> Vec<&str> {
    verify.steps.iter().map(|step| step.name.as_str()).collect()
}

/// **What Verify runs is setup, then every Check in written order, and nothing
/// else** — `generate` is declared and is not a step.
#[test]
fn verify_is_setup_then_every_check_and_nothing_else() {
    let manifest = config::Manifest::parse(Path::new("armada.yml"), MANIFEST).expect("parses");
    let steps = crate::verify_steps(&manifest);
    let order: Vec<(ipc::VerifyGroup, &str)> = steps
        .iter()
        .map(|step| (step.group, step.name.as_str()))
        .collect();
    assert_eq!(
        order,
        [
            (ipc::VerifyGroup::Setup, "bootstrap"),
            (ipc::VerifyGroup::Checks, "lint"),
            (ipc::VerifyGroup::Checks, "broken"),
            (ipc::VerifyGroup::Checks, "after"),
        ]
    );
}

/// **Each step runs once, in order, one after another, as an ordinary checkout
/// run** — and a Check that exits other than expected does not stop the ones
/// after it.
#[tokio::test]
async fn each_step_runs_once_in_order_one_after_another() {
    let home = TempDir::new();
    let fleet = a_fleet(&home, MANIFEST, &api::Broadcaster::new());

    let begun = Arc::clone(&fleet)
        .begin_checkout_verify(fleet.first())
        .await
        .expect("underway");
    assert!(begun.ended_at.is_none());
    let verify = ended(&fleet).await;
    assert_eq!(verify.id, begun.id);
    assert_eq!(names(&verify), ["bootstrap", "lint", "broken", "after"]);

    let records: Vec<&ipc::CheckoutRunRecord> = verify
        .steps
        .iter()
        .map(|step| match &step.state {
            ipc::VerifyStepState::Ran { record } => record,
            other => panic!("`{}` did not run: {other:?}", step.name),
        })
        .collect();
    let codes: Vec<Option<i32>> = records.iter().map(|record| record.exit_code).collect();
    assert_eq!(codes, [Some(0), Some(0), Some(3), Some(0)]);
    for pair in records.windows(2) {
        assert!(
            pair[1].started_at.as_str() >= pair[0].ended_at.as_str(),
            "`{}` started before `{}` ended — a burst, not a sequence",
            pair[1].name,
            pair[0].name
        );
    }

    let history = fleet
        .checkout_rehearsal_history(fleet.first())
        .await
        .expect("a history");
    let mut kept: Vec<&str> = history.runs.iter().map(|run| run.name.as_str()).collect();
    kept.reverse();
    assert_eq!(
        kept,
        ["bootstrap", "lint", "broken", "after"],
        "each step is a run the page's own history lists, once, and `generate` never ran"
    );
}

/// **A step's finish is published only once the next step is out**, so a
/// reader re-reading the sheet on `checkout_run.finished` never meets a gap.
///
/// The reader is a thread of its own, reading the instant each finish is
/// published — as Bridge does — so nothing waits on the runtime to schedule it
/// after the next step has already started.
#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn a_steps_finish_is_published_after_the_next_step_is_out() {
    let home = TempDir::new();
    let events = api::Broadcaster::new();
    let fleet = a_fleet(&home, MANIFEST, &events);
    let mut watching = events.subscribe();
    let handle = tokio::runtime::Handle::current();
    let reader = Arc::clone(&fleet);
    let reading = std::thread::spawn(move || {
        let mut read = Vec::new();
        while read.len() < 4 {
            match handle.block_on(watching.next()) {
                Some(api::Next::Send(delivered)) => {
                    if let ipc::Event::CheckoutRunFinished(record) = delivered.event {
                        let sheet = handle
                            .block_on(reader.checkout_run_sheet(reader.first()))
                            .expect("a sheet");
                        read.push((
                            record.name,
                            sheet.verify.expect("the Verify is on the sheet"),
                        ));
                    }
                }
                Some(_) => continue,
                None => break,
            }
        }
        read
    });

    Arc::clone(&fleet)
        .begin_checkout_verify(fleet.first())
        .await
        .expect("underway");
    let read = tokio::time::timeout(
        Duration::from_secs(60),
        tokio::task::spawn_blocking(move || reading.join().expect("the reader")),
    )
    .await
    .expect("every step finished")
    .expect("joined");

    assert_eq!(read.len(), 4);
    for (name, verify) in read {
        let at = verify
            .steps
            .iter()
            .position(|step| step.name == name)
            .expect("a step of the Verify");
        assert!(
            matches!(verify.steps[at].state, ipc::VerifyStepState::Ran { .. }),
            "`{name}` was published before its Verify held the record: {:?}",
            verify.steps[at].state
        );
        match verify.steps.get(at + 1) {
            Some(next) => assert!(
                matches!(next.state, ipc::VerifyStepState::Running { .. }),
                "`{name}` finished and `{}` was not out yet: {:?}",
                next.name,
                next.state
            ),
            None => assert!(verify.ended_at.is_some(), "the last step ended it"),
        }
    }
}

/// **While a Verify is underway nothing else runs in the checkout, and a
/// second Verify is refused.** Stop on the step that is out ends it, and the
/// steps after are not run.
#[tokio::test]
async fn a_verify_holds_the_checkout_and_stop_ends_it() {
    let home = TempDir::new();
    let fleet = a_fleet(&home, SLOW, &api::Broadcaster::new());

    let begun = Arc::clone(&fleet)
        .begin_checkout_verify(fleet.first())
        .await
        .expect("underway");
    let ipc::VerifyStepState::Running { run_id } = &begun.steps[0].state else {
        panic!(
            "the first step is out when the Verify answers: {:?}",
            begun.steps[0]
        );
    };

    let run = Arc::clone(&fleet)
        .start_checkout_rehearsal(
            ipc::StartCheckoutRun {
                name: String::from("after"),
            },
            fleet.first(),
        )
        .await;
    assert!(matches!(run, Err(api::Refusal::IllegalMove(_))));
    let again = Arc::clone(&fleet)
        .begin_checkout_verify(fleet.first())
        .await;
    assert!(matches!(again, Err(api::Refusal::IllegalMove(_))));

    let stopped = fleet
        .stop_checkout_rehearsal(run_id.clone(), fleet.first())
        .await
        .expect("the step stopped");
    assert!(stopped.stopped);
    let verify = ended(&fleet).await;
    let ipc::VerifyStepState::NotRun { why } = &verify.steps[1].state else {
        panic!("`after` ran after a stop: {:?}", verify.steps[1]);
    };
    assert!(why.contains("stopped"), "{why}");
}

/// **A setup step that fails leaves every Check not run, saying why.** A Job's
/// worktree stops preparing the same way; whether Verify should is #719's
/// open question, and this is the behaviour until it is answered.
#[tokio::test]
async fn a_failed_setup_leaves_the_checks_not_run() {
    let home = TempDir::new();
    let fleet = a_fleet(&home, SETUP_FAILS, &api::Broadcaster::new());

    Arc::clone(&fleet)
        .begin_checkout_verify(fleet.first())
        .await
        .expect("underway");
    let verify = ended(&fleet).await;
    assert!(matches!(
        verify.steps[0].state,
        ipc::VerifyStepState::Ran { .. }
    ));
    let ipc::VerifyStepState::NotRun { why } = &verify.steps[1].state else {
        panic!("`lint` ran over a failed install: {:?}", verify.steps[1]);
    };
    assert_eq!(why, "setup `bootstrap` exited 1");
}
