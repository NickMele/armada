//! A frozen repository: nothing new starts there, a running Job waits at its next gate, a
//! person's act is taken and then waits, and lifting the freeze lets all of it carry on.
//!
//! **The file is real and re-read**, because the claim is that a freeze is live: the
//! Manifest Fleet holds is the one `Reloads` moves, with no restart between.

use std::path::PathBuf;

use api::Queries;
use config::{Manifest, Reloads};
use core_model::{Job, JobId, JobStatus, StepState, StepVerdict};
use testkit::{FakeHarness, FakeVcs, FakeWorkProduct};

use crate::daemon::Fleet;
use crate::freezing::frozen_among;
use crate::tests::admitted::dispatched;
use crate::tests::daemon::{
    a_proposal, diff_evidence, fittings, one, two_steps_gated_on_a_person, worktree_directory,
};
use crate::tests::tmp::TempDir;
use crate::tests::tools::submitted_by_the_one;

type Fixture = Fleet<FakeHarness, FakeVcs, FakeWorkProduct>;

const HEAD: &str = "version: 1\nid: 01FIXTUREMANIFEST\n";

/// A Fleet over an `armada.yml` on disk, and the one handle that re-reads it.
struct Frozen {
    fleet: Fixture,
    file: PathBuf,
    reloads: Reloads,
}

impl Frozen {
    fn over(home: &TempDir, says: &str, gated_on_a_person: bool) -> Frozen {
        let dir = home.path().join("manifest");
        std::fs::create_dir_all(&dir).expect("a directory for the file");
        let file = dir.join("armada.yml");
        std::fs::write(&file, format!("{HEAD}{says}")).expect("the file");
        let (manifest, reloads) = Manifest::reloadable(&file).expect("the file loads");
        let mut fittings = fittings(home, FakeWorkProduct::changed(&["src/log.rs"]));
        fittings.starting().manifest = manifest;
        if gated_on_a_person {
            fittings.starting().workflows =
                one(two_steps_gated_on_a_person("implement", None, None));
        }
        Frozen {
            fleet: Fleet::assembled(fittings),
            file,
            reloads,
        }
    }

    /// A person saving the file under the running Fleet, as the watch re-reads it.
    fn saved(&self, says: &str) {
        std::fs::write(&self.file, format!("{HEAD}{says}")).expect("the save");
        self.reloads.reread().expect("the save loads");
    }

    async fn proposed(&self, home: &TempDir, title: &str) -> Job {
        let job = self
            .fleet
            .propose(a_proposal(title))
            .await
            .expect("a proposal");
        worktree_directory(home, &job);
        job
    }

    async fn row(&self, job: &JobId) -> ipc::JobSummary {
        self.fleet
            .get_job(ipc::JobId::from(job))
            .await
            .expect("the Job reads")
            .job
    }

    async fn working(&self, job: &JobId) -> bool {
        self.fleet.working_on().await.contains(job)
    }
}

fn held_by_the_fixture() -> Vec<ipc::ManifestId> {
    vec![ipc::ManifestId::carried("01FIXTUREMANIFEST")]
}

/// **The claim's first half.** A person approves at a frozen repository; the approval is
/// taken, the Job starts nothing, and the Board says why and names the Manifest.
#[tokio::test]
async fn a_frozen_repository_starts_no_new_job_and_lifting_the_freeze_starts_it() {
    let home = TempDir::new();
    let at = Frozen::over(&home, "freeze: true\n", false);
    let job = at.proposed(&home, "a change during a release freeze").await;

    let approved = at
        .fleet
        .approve(job.id())
        .await
        .expect("a person's approval is never refused for a freeze");
    assert_eq!(
        approved.status(),
        JobStatus::Queued,
        "the approval is recorded"
    );
    at.fleet.turn().await.expect("the loop turns");

    assert!(!at.working(job.id()).await, "and no Drone starts on it");
    let row = at.row(job.id()).await;
    assert_eq!(row.queued_reason.map(|why| why.as_wire()), Some("frozen"));
    assert_eq!(row.frozen_by, held_by_the_fixture());

    at.saved("");
    at.fleet
        .turn()
        .await
        .expect("the loop turns after the save");

    assert!(
        at.working(job.id()).await,
        "lifting the freeze starts it, with no restart"
    );
    let row = at.row(job.id()).await;
    assert_eq!(row.queued_reason, None);
    assert!(row.frozen_by.is_empty());
}

/// **The claim's second half.** A Job already running when the freeze lands finishes the
/// step it is on, waits at the gate with that step passed, and resumes on the next step —
/// nothing re-run and nothing lost.
#[tokio::test]
async fn a_running_job_waits_at_its_next_gate_and_resumes_on_the_next_step() {
    let home = TempDir::new();
    let at = Frozen::over(&home, "", false);
    let job = at.proposed(&home, "a change already under way").await;
    dispatched(&at.fleet, job.id()).await.expect("it starts");

    at.saved("freeze: true\n");
    submitted_by_the_one(&at.fleet, diff_evidence())
        .await
        .expect("the Drone's evidence is taken");
    at.fleet.turn().await.expect("the gate rules");

    let standing = at.fleet.load(job.id()).await.expect("the Job reads");
    assert_eq!(
        standing.status(),
        JobStatus::Queued,
        "held at the gate, in the queue"
    );
    assert!(!at.working(job.id()).await, "and holding no Drone");
    let implement = standing
        .steps()
        .iter()
        .find(|row| row.step_id().as_str() == "implement")
        .expect("the first step");
    assert_eq!(
        implement.state(),
        StepState::Advanced,
        "the step it was on still passed"
    );
    assert!(matches!(
        implement.last_verdict(),
        Some(StepVerdict::Passed)
    ));
    let row = at.row(job.id()).await;
    assert_eq!(row.queued_reason.map(|why| why.as_wire()), Some("frozen"));
    assert_eq!(row.frozen_by, held_by_the_fixture());
    assert_eq!(
        row.resumption, None,
        "Fleet stood it down; nobody put it back"
    );

    at.saved("");
    at.fleet
        .turn()
        .await
        .expect("the loop turns after the save");

    let resumed = at.fleet.load(job.id()).await.expect("the Job reads");
    assert_eq!(resumed.status(), JobStatus::Running);
    assert_eq!(
        resumed
            .current_step_id()
            .map(|step| step.as_str().to_string()),
        Some("summarise".to_string()),
        "on the step after the one that passed, not the same one again"
    );
}

/// **The owner's decision.** A person approving at a review gate while the repository is
/// frozen is recorded; the Job then waits at the freeze and carries on once it lifts.
#[tokio::test]
async fn an_approval_at_a_review_gate_is_recorded_and_then_waits_for_the_freeze() {
    let home = TempDir::new();
    let at = Frozen::over(&home, "", true);
    let job = at.proposed(&home, "a change a person reviews").await;
    dispatched(&at.fleet, job.id()).await.expect("it starts");
    submitted_by_the_one(&at.fleet, diff_evidence())
        .await
        .expect("the Drone's evidence is taken");
    at.fleet.turn().await.expect("the gate holds for a person");
    assert_eq!(
        at.fleet.load(job.id()).await.expect("the Job").status(),
        JobStatus::AwaitingReview
    );

    at.saved("freeze: true\n");
    let approved = at
        .fleet
        .approve_review(job.id())
        .await
        .expect("the approval is taken at a frozen repository");
    assert_eq!(approved.status(), JobStatus::Queued);
    at.fleet.turn().await.expect("the loop turns");
    assert!(
        !at.working(job.id()).await,
        "the approval does not start a Drone"
    );
    let row = at.row(job.id()).await;
    assert_eq!(row.queued_reason.map(|why| why.as_wire()), Some("frozen"));

    at.saved("");
    at.fleet
        .turn()
        .await
        .expect("the loop turns after the save");
    assert!(
        at.working(job.id()).await,
        "and it carries on once the freeze lifts"
    );
}

/// **Most-restrictive-wins.** One frozen Manifest among a Job's two freezes the Job, and
/// it is the one named; neither frozen holds nothing.
#[test]
fn one_frozen_manifest_of_two_freezes_the_job_and_is_the_one_named() {
    let parsed = |dir: &str, text: &str| {
        Manifest::parse(std::path::Path::new(dir), text).expect("a manifest that parses")
    };
    let open = parsed("web/armada.yml", "version: 1\nid: 01WEB\n");
    let shut = parsed("api/armada.yml", "version: 1\nid: 01API\nfreeze: true\n");

    let named = |gating: Vec<&Manifest>| -> Vec<String> {
        frozen_among(gating)
            .iter()
            .map(|id| id.as_str().to_string())
            .collect()
    };
    assert_eq!(named(vec![&open, &shut]), ["01API"]);
    assert_eq!(
        named(vec![&shut, &open]),
        ["01API"],
        "whichever order they gate in"
    );
    assert!(named(vec![&open, &open]).is_empty());
}
