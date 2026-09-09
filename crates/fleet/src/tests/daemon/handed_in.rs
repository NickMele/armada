//! What a case hands Fleet: the proposal that starts a Job, the worktree its
//! Drone runs in, and the evidence a step is advanced on.
//!
//! # The one thing a test has to do that Fleet does not
//!
//! `FakeVcs` creates nothing on disk, deliberately — and a detached child needs
//! a working directory that is really there. So each case makes the derived
//! directory itself, from the same `WorktreeSpec` Fleet will derive, between
//! proposing and approving. Deriving it a second way would be the
//! second-vocabulary defect in a test.

use adapter_traits::WorktreeSpec;
use config::EvidenceType;
use verification::{Claimed, NotClaimed, ShownBy};

use crate::evidence::Call;
use crate::tests::tmp::TempDir;

/// [`a_proposal`], naming a workflow other than the fixture's own.
pub fn a_proposal_for(title: &str, workflow_id: &str) -> ipc::ProposeJob {
    let mut proposal = a_proposal(title);
    proposal.workflow_id = ipc::WorkflowId::carried(workflow_id);
    proposal
}

pub fn a_proposal(title: &str) -> ipc::ProposeJob {
    ipc::ProposeJob {
        title: title.to_string(),
        // The values the fixture Fleet actually holds. Naming anything else is
        // refused at creation now, which is the point.
        workflow_id: ipc::WorkflowId::carried("fixture-workflow"),
        owner_manifest_id: ipc::ManifestId::carried("01FIXTUREMANIFEST"),
        origin: ipc::TopLevelOrigin::from_wire("manual").expect("an origin"),
        urgency: ipc::Urgency::from_wire("normal").expect("an urgency"),
        atomic: false,
        model: Some("a-model".to_string()),
        acceptance_criteria: vec![ipc::ProposedCriterion {
            text: "the symptom is gone".to_string(),
            source: ipc::CriterionSource::from_wire("check").expect("a source"),
        }],
        subject: None,
        dependencies: Vec::new(),
        facts: "the reader is off by one".to_string(),
        write_targets: None,
        attachments: Vec::new(),
    }
}

/// Make the directory `FakeVcs` says it made. See this module's comment.
/// **Takes the Job and not its id**, because a worktree is named by the handle
/// now and the handle is derived from the number and the title. A fixture that
/// took an id would make a directory Fleet does not look in.
pub fn worktree_directory(home: &TempDir, job: &core_model::Job) {
    worktree_directory_named(home, &job.handle());
}

/// The same directory, for a caller holding the handle and not the Job — an
/// HTTP test reads it off the `JobSummary` it was answered with.
pub fn worktree_directory_named(home: &TempDir, handle: &str) {
    let spec = WorktreeSpec::for_job(&home.path().to_string_lossy(), handle).expect("a legal spec");
    std::fs::create_dir_all(spec.worktree_path()).expect("a directory for the Drone to run in");
}

pub fn diff_evidence() -> Call<'static> {
    Call {
        evidence_type: EvidenceType::Diff,
        claimed: Claimed("The reader stops one line later."),
        shown_by: ShownBy("src/log.rs, six lines"),
        not_claimed: NotClaimed("The writer has the same bug and is untouched."),
    }
}

pub fn note_evidence() -> Call<'static> {
    Call {
        evidence_type: EvidenceType::FactsNote,
        claimed: Claimed("The cause was an inclusive bound."),
        shown_by: ShownBy("`.armada/root-cause.md`, written this step"),
        not_claimed: NotClaimed(""),
    }
}
