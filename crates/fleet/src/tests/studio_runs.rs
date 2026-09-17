//! A run started from a Studio, and what its node keeps once retention takes
//! the run away. `#1289`.
//!
//! **A real run in a real checkout.** What is under test is the order of two
//! things that touch the same directory — the tail being read and the run
//! being removed — so a fixture that faked either would assert nothing.

use std::path::Path;
use std::process::Command;
use std::sync::Arc;
use std::time::Duration;

use api::{Redirector, Studios};
use ipc::{CreateStudio, StartStudioRun, Studio, StudioNodeContent, StudioPosition};
use testkit::FakeWorkProduct;

use crate::daemon::Fleet;
use crate::gate::CheckBudget;
use crate::tests::daemon::fittings;
use crate::tests::tmp::TempDir;

type Fixture = Fleet<testkit::FakeHarness, testkit::FakeVcs, FakeWorkProduct>;

/// `lint` prints one line and exits zero; `fail` prints two and exits one, so
/// a node that kept a tail has something to have kept and a result to show.
const MANIFEST: &str = r#"version: 1
id: 01FIXTUREMANIFEST
checks:
  lint:
    run: "/bin/sh -c 'echo linted'"
  fail:
    run: "/bin/sh -c 'echo working; echo 4 failed; exit 1'"
"#;

/// A Fleet whose runs are swept the moment they end: retention is the subject
/// here, so the window is the fixture's rather than the shipped thirty days.
fn a_fleet_that_sweeps_at_once(home: &TempDir) -> Arc<Fixture> {
    let manifest = config::Manifest::parse(Path::new("armada.yml"), MANIFEST)
        .unwrap_or_else(|why| panic!("the fixture manifest did not parse: {why}"));
    let mut fittings = fittings(home, FakeWorkProduct::changed(&[]));
    fittings.starting().manifest = manifest;
    fittings.budget = CheckBudget::of(Duration::from_secs(120));
    fittings.run_log_retention = Duration::ZERO;
    Arc::new(Fleet::assembled(fittings))
}

/// The checkout as a repository with one commit, which is what a run's
/// snapshot is taken against.
fn a_repository_at(at: &Path) {
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
    std::fs::write(at.join("armada.yml"), MANIFEST).expect("the manifest is written");
    git(&["add", "."]);
    git(&["commit", "--quiet", "-m", "the checkout"]);
}

async fn finished(fleet: &Fixture, id: &str) {
    tokio::time::timeout(Duration::from_secs(30), async {
        loop {
            let history = fleet
                .checkout_rehearsal_history(fleet.first())
                .await
                .expect("a history");
            if history.runs.iter().any(|run| run.id == id) {
                return;
            }
            tokio::time::sleep(Duration::from_millis(20)).await;
        }
    })
    .await
    .expect("the run wrote its record")
}

async fn a_studio(fleet: &Fixture) -> Studio {
    fleet
        .create_studio(
            CreateStudio {
                name: Some("Stale counts".to_string()),
            },
            None,
        )
        .await
        .expect("created in the repository Fleet starts in")
}

fn asked(name: &str) -> StartStudioRun {
    StartStudioRun {
        name: name.to_string(),
        workspace: None,
        position: StudioPosition { x: 40, y: 80 },
        produced_by: None,
    }
}

fn run_node<'a>(studio: &'a Studio, node_id: &ipc::StudioNodeId) -> &'a StudioNodeContent {
    &studio
        .nodes
        .iter()
        .find(|node| &node.id == node_id)
        .expect("the node that was made")
        .content
}

/// **A Run node started from a Studio outlives its run.** While the run is
/// there the node is a reference and nothing else; once retention sweeps the
/// run, the same node holds the command, the exit code, the duration and the
/// log's last lines, and the directory it referenced is gone.
///
/// The failure this is against is `records::swept` deleting a run a Studio is
/// pointing at, leaving a node naming a run nobody can read. The tail has to
/// be taken before the sweep, so the sweep is what this drives.
#[tokio::test]
async fn a_run_a_studio_holds_keeps_its_tail_and_result_when_retention_sweeps_it() {
    let home = TempDir::new();
    a_repository_at(home.path());
    let fleet = a_fleet_that_sweeps_at_once(&home);
    let studio = a_studio(&fleet).await;

    let started = Arc::clone(&fleet)
        .start_studio_run(studio.id.clone(), asked("fail"), Redirector::Person, None)
        .await
        .expect("underway");
    assert_eq!(
        run_node(&started.studio, &started.node_id),
        &StudioNodeContent::Run {
            run_id: started.run.id.clone(),
            kept: None,
        },
        "a reference, and no status copied off the run"
    );
    finished(&fleet, &started.run.id).await;

    // The second run is what sweeps the first, `rehearsing::shared`.
    let second = Arc::clone(&fleet)
        .start_studio_run(studio.id.clone(), asked("lint"), Redirector::Person, None)
        .await
        .expect("underway");

    let StudioNodeContent::Run { run_id, kept } = run_node(&second.studio, &started.node_id) else {
        panic!("still a Run node");
    };
    assert_eq!(run_id, &started.run.id, "it still says which run it was");
    let kept = kept
        .as_ref()
        .expect("the run was swept, so its node kept it");
    assert_eq!(kept.name, "fail");
    assert_eq!(
        kept.command,
        "/bin/sh -c 'echo working; echo 4 failed; exit 1'"
    );
    assert_eq!(kept.exit_code, Some(1));
    assert_eq!(kept.expect_exit_code, 0, "so the node still reads failed");
    assert!(!kept.stopped);
    assert_eq!(
        kept.lines,
        vec!["working".to_string(), "4 failed".to_string()],
        "the log's last lines, as the log had them"
    );
    assert_eq!(kept.total_lines, 2);
    assert!(
        kept.whole,
        "two lines is under the bound, so it is all of it"
    );
    assert!(
        !home
            .path()
            .join(".armada")
            .join("runs")
            .join("main")
            .join(&started.run.id)
            .exists(),
        "the run itself was swept, which is what the node kept its tail against"
    );
}

/// **A Run node is started, never added.** `add_studio_node` refuses the kind,
/// so no request can name a run that never ran or a result it never had.
#[tokio::test]
async fn a_run_node_cannot_be_added_by_hand() {
    let home = TempDir::new();
    let fleet = a_fleet_that_sweeps_at_once(&home);
    let studio = a_studio(&fleet).await;

    let refused = fleet
        .add_studio_node(
            studio.id.clone(),
            ipc::AddStudioNode {
                content: StudioNodeContent::Run {
                    run_id: String::from("01NEVERRAN"),
                    kept: None,
                },
                position: StudioPosition { x: 0, y: 0 },
                produced_by: None,
            },
            Redirector::Person,
            None,
        )
        .await
        .expect_err("a Run node is made by starting a run");
    let api::Refusal::Unacceptable(why) = refused else {
        panic!("a 422");
    };
    assert_eq!(why.code, "fleet.studio_node_is_a_run");
}
