//! A file a person attached to the brief, from staged path to worktree.
//!
//! Three points, three tests, matching the three points `drafting.rs` and
//! `dispatch.rs` name: promotion at creation, refusal where a staged path
//! cannot be read, and the worktree copy plus the brief line dispatch adds
//! before a Drone's first turn is assembled.

use adapter_traits::WorktreeSpec;
use core_model::JobStatus;

use crate::adrift::Adrift;
use crate::briefing;
use crate::tests::daemon::{a_fleet, a_proposal, worktree_directory};
use crate::tests::tmp::TempDir;
use testkit::FakeWorkProduct;

/// A file on disk at `dir`, standing in for what Bridge's staging step wrote
/// before `propose_job` was ever called.
fn staged(dir: &std::path::Path, filename: &str, bytes: &[u8]) -> String {
    std::fs::create_dir_all(dir).expect("a staging directory");
    let path = dir.join(filename);
    std::fs::write(&path, bytes).expect("a staged file");
    path.to_string_lossy().to_string()
}

fn proposal_with(staged_path: String, filename: &str) -> ipc::ProposeJob {
    let mut proposal = a_proposal("a Job with a screenshot attached");
    proposal.attachments = vec![ipc::AttachmentRef {
        staged_path,
        filename: filename.to_string(),
        mime_type: "image/png".to_string(),
    }];
    proposal
}

/// **(a)** A proposal naming a real staged file produces a Job whose
/// `attachments()` holds it, and the promoted copy is really on disk, keyed
/// by the Job id `drafted()` minted.
#[tokio::test]
async fn a_staged_attachment_is_promoted_into_the_job() {
    let home = TempDir::new();
    let fleet = a_fleet(&home, FakeWorkProduct::changed(&["src/log.rs"]));

    let staged_path = staged(
        &home.path().join("staged"),
        "before.png",
        b"not really a png",
    );
    let job = fleet
        .propose(proposal_with(staged_path, "before.png"))
        .await
        .expect("a real staged file is promoted");

    assert_eq!(job.attachments().len(), 1);
    assert_eq!(job.attachments()[0].filename, "before.png");
    assert_eq!(
        job.attachments()[0].byte_size,
        "not really a png".len() as u64
    );

    let promoted = std::path::Path::new(job.attachments()[0].storage_ref.as_str());
    assert!(
        promoted.exists(),
        "the promoted copy is on disk at {}",
        promoted.display()
    );
    assert_eq!(std::fs::read(promoted).unwrap(), b"not really a png");
    assert!(
        promoted.starts_with(home.path().join("attachments").join(job.id().as_str())),
        "kept under the Job's own directory, outside any worktree: {}",
        promoted.display()
    );
}

/// **(b)** A staged path that does not exist is refused — the fifth named
/// refusal `drafting.rs` documents — and not silently dropped from the Job.
#[tokio::test]
async fn a_missing_staged_path_is_refused_rather_than_dropped() {
    let home = TempDir::new();
    let fleet = a_fleet(&home, FakeWorkProduct::changed(&["src/log.rs"]));

    let missing = home
        .path()
        .join("staged")
        .join("never-written.png")
        .to_string_lossy()
        .to_string();
    let refused = fleet
        .propose(proposal_with(missing, "never-written.png"))
        .await
        .expect_err("a staged path that was never written cannot be promoted");

    match refused {
        Adrift::AttachmentUnreadable { filename, .. } => {
            assert_eq!(filename, "never-written.png");
        }
        other => panic!("expected AttachmentUnreadable, got {other:?}"),
    }
}

/// **(c)** After dispatch, the attachment is copied into the worktree and the
/// first-turn prompt names its worktree-relative path — the same function
/// `spawn_config` calls to brief the Drone, called here directly against the
/// dispatched Job so the test asserts the same string a Drone is given.
#[tokio::test]
async fn dispatch_copies_the_attachment_and_the_brief_names_it() {
    let home = TempDir::new();
    let fleet = a_fleet(&home, FakeWorkProduct::changed(&["src/log.rs"]));

    let staged_path = staged(&home.path().join("staged"), "repro.png", b"a screenshot");
    let job = fleet
        .propose(proposal_with(staged_path, "repro.png"))
        .await
        .expect("promoted");
    worktree_directory(&home, job.id());

    let approved = fleet.approve(job.id()).await.expect("dispatch runs");
    assert_eq!(approved.status(), JobStatus::Running);

    let spec = WorktreeSpec::for_job(&home.path().to_string_lossy(), job.id().as_str())
        .expect("a legal spec");
    let worktree_path = spec.worktree_path();
    let copied = std::path::Path::new(&worktree_path)
        .join(".armada")
        .join("attachments")
        .join("repro.png");
    assert!(
        copied.exists(),
        "the attachment is copied into the fresh worktree at {}",
        copied.display()
    );
    assert_eq!(std::fs::read(&copied).unwrap(), b"a screenshot");

    let step = approved
        .current_step_id()
        .cloned()
        .expect("dispatch moved the cursor onto the first step");
    let prompt = briefing::first_turn(&approved, approved.workflow(), &step)
        .expect("a Job with a brief assembles a prompt")
        .as_str()
        .to_string();
    assert!(
        prompt.contains(".armada/attachments/repro.png"),
        "the brief names the worktree-relative path: {prompt}"
    );
}
