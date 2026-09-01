//! The document a Judge read, still readable after the Job is cleaned.
//!
//! **The worktree is really deleted here.** `armada clean` reclaims a Job's
//! checkout and the deliverable is in no commit, so a test that kept the
//! directory around would prove nothing about the case `#223` was filed for:
//! the file was found missing an hour after a Job was approved, merged and
//! tidied up, with its transcripts, its log, its Checks' output and its diff
//! all still there.
//!
//! Nothing here fakes a filesystem. The whole subject is which directory a byte
//! is in when another directory is removed.

use std::path::{Path, PathBuf};
use std::sync::Arc;

use adapter_traits::{Footprint, Worktree};
use config::ResolvedWorkflow;
use core_model::{Attempt, JobId, StepId, Ulid};
use testkit::{FakeJudge, FakeWorkProduct, Gate, Sketch};
use verification::Request;

use crate::at_step::AtStep;
use crate::gate::{rule_on, Ruling};
use crate::keeping::{deliverables_dir, kept_deliverables, Keeping};
use crate::tests::gate::{budget, judged_by_shared, note_evidence};
use crate::tests::tmp::TempDir;

const TARGET: &str = ".armada/artifacts/plan.md";
const JOB: &str = "01J0000000000000000000JOB0";

fn job_id() -> JobId {
    JobId::carried(Ulid::carried(JOB))
}

/// Somewhere a gate's copy can go, for a test whose subject is not the copy.
///
/// **A real directory, removed with the value.** A path the filesystem would
/// refuse is a case and it is this file's; a test about something else must not
/// be exercising it without saying so. The directory travels with the
/// [`Keeping`] so a call site is one argument rather than a binding and an
/// argument — `crate::tests::gate` is nineteen lines from the cap that refuses
/// a file, and every gate test needs one of these.
pub(super) struct Keeps {
    keeping: Keeping,
    _dir: TempDir,
}

impl std::ops::Deref for Keeps {
    type Target = Keeping;

    fn deref(&self) -> &Keeping {
        &self.keeping
    }
}

pub(super) fn keeping_nowhere() -> Keeps {
    let dir = TempDir::new();
    Keeps {
        keeping: Keeping::of(&dir.path().to_string_lossy(), &job_id()),
        _dir: dir,
    }
}

/// A `facts_note` step gated on the file it was asked to write, and on one
/// question about it — so the Judge is asked and the gate reads the bytes.
fn delivering_workflow() -> ResolvedWorkflow {
    testkit::resolved(&[Sketch {
        id: "plan",
        label: "Plan the change",
        evidence_type: Some("facts_note"),
        gates: &[Gate::ArtifactExists { target: TARGET }],
        judged_on: &[("c1", "Does this plan name a specific root cause?")],
        scope: None,
        gaming: None,
    }])
}

/// A repository laid out the way Fleet's own is, with this Job's worktree
/// inside it at the path `WorktreeSpec` derives.
///
/// The two directories are the whole point: the copy has to land in the first
/// and the deliverable is written in the second.
struct Repo {
    root: TempDir,
}

impl Repo {
    fn new() -> Repo {
        Repo {
            root: TempDir::new(),
        }
    }

    fn root(&self) -> &Path {
        self.root.path()
    }

    fn worktree_path(&self) -> PathBuf {
        self.root().join(".armada").join("worktrees").join(JOB)
    }

    fn worktree(&self) -> Worktree {
        Worktree::at(
            self.worktree_path().to_string_lossy().to_string(),
            format!("armada/{JOB}"),
        )
    }

    /// The step's deliverable, written where the frozen workflow says a Drone
    /// would have written it.
    fn drone_wrote(&self, contents: &str) {
        let at = self.worktree_path().join(TARGET);
        std::fs::create_dir_all(at.parent().expect("a parent")).expect("the directory");
        std::fs::write(at, contents).expect("the file");
    }

    /// What `armada clean` does to a Job: the checkout goes, and nothing under
    /// the repository root does.
    fn cleaned(&self) {
        std::fs::remove_dir_all(self.worktree_path()).expect("the worktree goes");
    }

    /// Every copy kept for this Job, by file name.
    fn kept(&self) -> Vec<String> {
        let dir = deliverables_dir(&self.root().to_string_lossy(), &job_id());
        let Ok(entries) = std::fs::read_dir(dir) else {
            return Vec::new();
        };
        let mut names: Vec<String> = entries
            .flatten()
            .map(|entry| entry.file_name().to_string_lossy().into_owned())
            .collect();
        names.sort();
        names
    }

    /// What a reader of the record can reach for one run of the step, exactly
    /// as `get_job` answers it — relative to the repository root.
    fn reachable(&self, attempt: Attempt) -> Vec<String> {
        kept_deliverables(
            &self.root().to_string_lossy(),
            &job_id(),
            &StepId::new("plan"),
            attempt,
            TARGET,
        )
    }

    fn kept_bytes(&self, name: &str) -> String {
        let dir = deliverables_dir(&self.root().to_string_lossy(), &job_id());
        std::fs::read_to_string(dir.join(name)).expect("a kept deliverable")
    }

    async fn gated(&self, judge: Arc<FakeJudge>) -> Ruling {
        let worktree = self.worktree();
        let workflow = delivering_workflow();
        let at = AtStep::first(workflow.frozen(), &worktree).expect("a first step");
        rule_on(
            at,
            Request::of(testkit::asked_for()),
            &note_evidence(),
            None,
            Some(&Footprint::nothing()),
            &[],
            &FakeWorkProduct::changed(&[]),
            budget(),
            &judged_by_shared(judge),
            &Keeping::of(&self.root().to_string_lossy(), &job_id()),
        )
        .await
    }
}

/// **The bug, and the fix.** The Judge is shown the document, the Job is
/// cleaned, and the document is still there — under the repository root, where
/// `.armada/logs/` and `.armada/transcripts/` already were.
#[tokio::test]
async fn the_document_a_judge_read_outlives_the_worktree() {
    let repo = Repo::new();
    let plan = "The cause is the `-0.0` comparison in spend.rs:88.\n";
    repo.drone_wrote(plan);
    let judge = Arc::new(FakeJudge::with_no_objection());

    let ruling = repo.gated(Arc::clone(&judge)).await;
    assert!(ruling.advanced(), "{ruling:?}");
    assert_eq!(judge.asked().len(), 1, "the Judge read it");

    repo.cleaned();
    assert!(
        !repo.worktree_path().exists(),
        "the checkout the deliverable was written in is gone"
    );
    assert_eq!(repo.kept(), vec!["plan.1.plan.md".to_string()]);
    assert_eq!(
        repo.kept_bytes("plan.1.plan.md"),
        plan,
        "and it is the document, not a summary of one"
    );
}

/// **What the Judge was handed, byte for byte.** The copy is written from the
/// same value the call carries, so a document edited between the gate's read
/// and any later copy cannot be what is kept.
#[tokio::test]
async fn the_copy_is_the_bytes_that_were_in_the_call() {
    let repo = Repo::new();
    repo.drone_wrote("The reader's bound is inclusive.\n");
    let judge = Arc::new(FakeJudge::with_no_objection());

    repo.gated(Arc::clone(&judge)).await;

    let asked = &judge.asked()[0];
    assert!(
        asked.contains(repo.kept_bytes("plan.1.plan.md").trim()),
        "the call carried what was kept: {asked}"
    );
}

/// **A re-gate of one attempt is idempotent.** `crate::regating` asks the same
/// question again over work that has not moved, so the second read is the first
/// one made again — and two files for one reading would say a Job was judged
/// twice on two documents.
#[tokio::test]
async fn asking_the_same_question_again_keeps_one_copy() {
    let repo = Repo::new();
    repo.drone_wrote("One plan.\n");

    repo.gated(Arc::new(FakeJudge::with_no_objection())).await;
    repo.gated(Arc::new(FakeJudge::with_no_objection())).await;

    assert_eq!(repo.kept(), vec!["plan.1.plan.md".to_string()]);
}

/// **A copy is never overwritten.** Where a person edits the deliverable
/// between two gate runs of one attempt, two documents were judged and both are
/// the record of a verdict. Replacing the first would be `#223` again, one
/// scope smaller.
#[tokio::test]
async fn a_second_document_judged_under_one_attempt_is_kept_beside_the_first() {
    let repo = Repo::new();
    repo.drone_wrote("The first plan.\n");
    repo.gated(Arc::new(FakeJudge::with_no_objection())).await;

    repo.drone_wrote("The second plan, after a person edited it.\n");
    repo.gated(Arc::new(FakeJudge::with_no_objection())).await;

    assert_eq!(
        repo.kept(),
        vec!["plan.1.1.plan.md".to_string(), "plan.1.plan.md".to_string()],
    );
    assert_eq!(repo.kept_bytes("plan.1.plan.md"), "The first plan.\n");
    assert_eq!(
        repo.kept_bytes("plan.1.1.plan.md"),
        "The second plan, after a person edited it.\n"
    );
}

/// **Nothing is kept of a document no Judge weighed.** Over the bound the gate
/// reads one byte past the limit and refuses the call, so what it holds is a
/// truncation — and a truncated copy filed as the record of a verdict would say
/// a reading happened that did not.
#[tokio::test]
async fn a_deliverable_too_big_to_judge_is_not_kept() {
    let repo = Repo::new();
    repo.drone_wrote(&"x".repeat(verification::A_DELIVERABLE + 1));

    let judge = Arc::new(FakeJudge::with_no_objection());
    let ruling = repo.gated(Arc::clone(&judge)).await;

    assert!(ruling.undecided().is_some(), "{ruling:?}");
    assert!(judge.asked().is_empty(), "no call was made");
    assert!(repo.kept().is_empty(), "and nothing was filed as one");
}

/// **A step stopped by the mechanical tier keeps nothing**, because there is no
/// document and no verdict. The absence is the same one
/// `crate::tests::delivering` asserts about the call: a missing deliverable is
/// answered by a `stat`, and nothing past it runs.
#[tokio::test]
async fn a_step_that_wrote_nothing_keeps_nothing() {
    let repo = Repo::new();
    std::fs::create_dir_all(repo.worktree_path()).expect("a worktree with no deliverable in it");

    let ruling = repo.gated(Arc::new(FakeJudge::with_no_objection())).await;

    assert!(!ruling.advanced(), "{ruling:?}");
    assert!(repo.kept().is_empty());
}

/// **A step id that is not one path component keeps nothing.** A step id is
/// text a workflow author typed and `config` validates none of it, so one
/// holding a separator would put the copy outside the directory the Job's
/// record is read from. `check_output::file_name` answers the same way for the
/// same reason.
#[test]
fn a_step_id_that_would_leave_the_directory_keeps_nothing() {
    let repo = Repo::new();
    let keeping = Keeping::of(&repo.root().to_string_lossy(), &job_id());

    keeping.kept(
        &StepId::new("../elsewhere"),
        Attempt::FIRST,
        TARGET,
        "a plan",
    );

    assert!(repo.kept().is_empty());
    assert!(
        !repo
            .root()
            .parent()
            .is_some_and(|up| up.join("elsewhere.1.plan.md").exists()),
        "and nothing was written beside the repository either"
    );
}

/// **The reader answers what the writer wrote, in the order it wrote it.** The
/// name is rebuilt from the step, the run and the target rather than parsed out
/// of a listing, so this is the one test holding the two spellings of that
/// arithmetic together — a reader that derived a different name would serve a
/// path to a file nobody has.
#[tokio::test]
async fn the_reader_names_every_copy_of_one_run_oldest_first() {
    let repo = Repo::new();
    repo.drone_wrote("The first plan.\n");
    repo.gated(Arc::new(FakeJudge::with_no_objection())).await;
    repo.drone_wrote("The second plan, after a person edited it.\n");
    repo.gated(Arc::new(FakeJudge::with_no_objection())).await;

    assert_eq!(
        repo.reachable(Attempt::FIRST),
        vec![
            format!(".armada/deliverables/{JOB}/plan.1.plan.md"),
            format!(".armada/deliverables/{JOB}/plan.1.1.plan.md"),
        ],
        "both documents were judged and both are reachable"
    );
}

/// **A path is named only where a file is there.** The whole of `#246` is a
/// record named on a surface that nothing opens, and a reader that derived the
/// name without the check would put that defect one layer down: every step of
/// every Job would carry a path, and the ones with nothing behind them would
/// look exactly like the ones with a document.
#[tokio::test]
async fn a_run_that_kept_nothing_is_named_nowhere() {
    let repo = Repo::new();
    std::fs::create_dir_all(repo.worktree_path()).expect("a worktree with no deliverable in it");

    repo.gated(Arc::new(FakeJudge::with_no_objection())).await;

    assert!(repo.reachable(Attempt::FIRST).is_empty());
    assert!(
        repo.reachable(Attempt::stored(2).expect("a second run"))
            .is_empty(),
        "and neither is a run that never happened"
    );
}
