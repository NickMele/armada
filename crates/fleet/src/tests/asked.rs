//! What the Judge was asked, kept beside what it answered.
//!
//! # The assertion is against the bytes that went out, not against a rebuild
//!
//! `FakeJudge::asked` records every question Fleet actually wrote to a child's
//! stdin. Every case here compares the file to *that*, which is the one
//! comparison the two hand reconstructions of a brief could not make — and the
//! whole reason `#224` was filed.

use std::sync::Arc;
use std::time::Duration;

use adapter_traits::{Environment, Footprint, Model, Worktree};
use config::ResolvedWorkflow;
use core_model::{Attempt, CriterionId, StepId};
use testkit::{FakeJudge, FakeWorkProduct, Gate, Sketch};
use verification::{Lifted, Request};

use crate::asked::Asked;
use crate::at_step::AtStep;
use crate::gate::{rule_on, Ruling};
use crate::judging::{JudgeBudget, Judging, Marking};
use crate::policy::Policies;
use crate::tests::gate::{budget, diff_evidence, worktree};
use crate::tests::keeping::keeping_nowhere;
use crate::tests::tmp::TempDir;

const FIRST: &str = "Does the fix address the cause the note names?";
const SECOND: &str = "Does the change stay inside the paths the step declared?";

/// What the Job is called, which is what its brief directory is named after.
/// A separate constant from [`JOB`] because the two are separate values now:
/// the id joins the record and the handle names the paths.
const HANDLE: &str = "12-the-reader-drops-a-line";

/// One step, one passing Check, and two questions — so a test can tell one
/// criterion's file from another's rather than asserting on a single one.
fn two_criteria() -> ResolvedWorkflow {
    testkit::resolved(&[Sketch {
        id: "implement",
        label: "Implement",
        evidence_type: Some("diff"),
        gates: &[
            Gate::Check {
                name: "suite",
                run: "/usr/bin/true",
                expect_exit_code: 0,
                when: &[],
            },
            Gate::DiffNonempty,
        ],
        judged_on: &[("c1", FIRST), ("c2", SECOND)],
        scope: None,
        gaming: None,
    }])
}

fn judging(client: Arc<FakeJudge>, asked: Asked) -> Judging {
    Judging {
        client,
        budget: JudgeBudget::of(Duration::from_secs(20)),
        default_model: Model::named("the-cheap-model").expect("a model name"),
        environment: Environment::nothing(),
        marking: Marking::detached(),
        asked,
    }
}

async fn ruled(judge: Arc<FakeJudge>, asked: Asked, worktree: &Worktree) -> Ruling {
    let workflow = two_criteria();
    let at = AtStep::first(workflow.frozen(), worktree).expect("a first step");
    let work = FakeWorkProduct::changed(&["src/log.rs"]).showing("+    let n = n - 1;\n");
    rule_on(
        at,
        Request::of(testkit::asked_for()),
        &diff_evidence(),
        None,
        &Lifted::default(),
        Some(&Footprint::nothing()),
        &[],
        &work,
        budget(),
        &judging(judge, asked),
        &keeping_nowhere(),
        Policies::unstated(),
        &crate::underway::Announcing::nowhere(),
        &std::collections::BTreeMap::new(),
        &[],
        core_model::WhenRefused::default(),
        &[],
    )
    .await
}

/// The definition of done: a verdict comes back carrying the path to the call
/// that produced it, and the file at that path is what the model was sent.
#[tokio::test]
async fn a_verdict_can_be_read_against_the_question_that_went_out() {
    let dir = TempDir::new();
    let judge = Arc::new(FakeJudge::with_no_objection());
    let root = dir.path().to_string_lossy().to_string();
    let ruling = ruled(
        Arc::clone(&judge),
        Asked::under(root.clone(), HANDLE.to_string()),
        &worktree(),
    )
    .await;

    let judged = ruling.judged();
    assert_eq!(judged.len(), 2, "one verdict per declared criterion");
    let sent = judge.asked();
    assert_eq!(sent.len(), 2, "one call per declared criterion");

    for (judgment, question) in judged.iter().zip(sent.iter()) {
        let path = judgment
            .brief_path
            .as_deref()
            .expect("the verdict names where its brief was kept");
        assert_eq!(
            path,
            format!(
                ".armada/briefs/{HANDLE}/implement.1.{}.txt",
                judgment.criterion_id.as_str()
            ),
            "the path is the row's own key"
        );
        let kept = std::fs::read_to_string(dir.path().join(path)).expect("the brief is on disk");
        assert_eq!(
            &kept, question,
            "the file is the bytes that went to the model, not a rebuild of them"
        );
    }
    assert_ne!(
        judged[0].brief_path, judged[1].brief_path,
        "two criteria are two questions and two files"
    );
}

/// The same assertion about the other half of a call's record: what a member
/// was handed is a digest of the bytes that went out, not of anything upstream
/// of them.
///
/// **This is what makes `Judged.given` worth putting on the wire.** A digest
/// taken off the `Brief` would say the members of a panel were handed the same
/// object because the panel loop hands them one — which is the sentence the
/// field exists to replace. Taken off the `Ask`, it is a reading, and a run
/// that did not hand them the same object would come back saying so.
#[tokio::test]
async fn what_a_call_was_handed_is_a_digest_of_the_bytes_that_went_out() {
    let judge = Arc::new(FakeJudge::with_no_objection());
    let ruling = ruled(Arc::clone(&judge), Asked::nowhere(), &worktree()).await;

    let judged = ruling.judged();
    let sent = judge.asked();
    assert_eq!(judged.len(), sent.len(), "one record per call");
    for (judgment, question) in judged.iter().zip(sent.iter()) {
        let given = judgment
            .given
            .as_ref()
            .expect("every call records what it was handed");
        assert_eq!(
            given.digest,
            verification::digest(question),
            "the digest is of what the model was sent"
        );
        assert_eq!(given.size, question.chars().count() as u32);
        assert!(!given.model.is_empty(), "and which model was asked");
    }
    assert_ne!(
        judged[0].given.as_ref().map(|given| &given.digest),
        judged[1].given.as_ref().map(|given| &given.digest),
        "two criteria are two briefs, so two digests — a value that folded \
         across criteria would compare the wrong things"
    );
}

/// The trap the issue names second. A Judge that passes something it should
/// have refused is only visible against its input, so a pass keeps its brief
/// exactly as a refusal does.
#[tokio::test]
async fn a_pass_keeps_its_brief_as_a_refusal_does() {
    let dir = TempDir::new();
    let root = dir.path().to_string_lossy().to_string();
    let refused = ruled(
        Arc::new(FakeJudge::refusing(
            "the loop stops at n",
            "the loop stops at n - 1",
            "the last row is dropped",
        )),
        Asked::under(root.clone(), HANDLE.to_string()),
        &worktree(),
    )
    .await;
    let met = ruled(
        Arc::new(FakeJudge::with_no_objection()),
        Asked::under(root, HANDLE.to_string()),
        &worktree(),
    )
    .await;

    for ruling in [&refused, &met] {
        assert!(
            ruling
                .judged()
                .iter()
                .all(|judgment| judgment.brief_path.is_some()),
            "every verdict names its brief, whichever way it went"
        );
    }
}

/// A brief is written on the way out, so the calls that produced no verdict at
/// all — the timeouts, the refusals from the vendor, the prose — are the ones
/// `#154`'s calibration record can still look at.
#[tokio::test]
async fn a_call_that_never_answered_still_left_what_it_was_asked() {
    let dir = TempDir::new();
    let judge = Arc::new(FakeJudge::that_fails("the quota"));
    let root = dir.path().to_string_lossy().to_string();
    let ruling = ruled(
        Arc::clone(&judge),
        Asked::under(root, HANDLE.to_string()),
        &worktree(),
    )
    .await;

    assert!(
        ruling.judged().is_empty(),
        "a call that could not be made is not a verdict"
    );
    let kept = std::fs::read_to_string(
        dir.path()
            .join(format!(".armada/briefs/{HANDLE}/implement.1.c1.txt")),
    )
    .expect("the brief of a call that failed");
    assert_eq!(
        Some(&kept),
        judge.asked().first(),
        "what the failed call was asked is on disk"
    );
}

/// A panel answers one brief, so its members share one file. Asserted on the
/// writer rather than through a panel, because nothing in `testkit` declares
/// one — what a panel does at the call site is hand the same `Brief` here
/// `panel_size` times, which is this.
#[test]
fn a_panel_shares_one_file_because_it_shares_one_brief() {
    let dir = TempDir::new();
    let keeping = Asked::under(dir.path().to_string_lossy().to_string(), HANDLE.to_string());
    let (step, criterion) = (StepId::new("implement"), CriterionId::new("c1"));

    let paths: Vec<Option<String>> = (0..3)
        .map(|_| keeping.kept(&step, Attempt::FIRST, &criterion, "the whole brief"))
        .collect();

    assert_eq!(paths[0], paths[1]);
    assert_eq!(paths[1], paths[2], "three answers, one question, one file");
    let held = std::fs::read_dir(dir.path().join(".armada/briefs").join(HANDLE))
        .expect("the directory")
        .count();
    assert_eq!(held, 1, "and one file on disk, not three copies of it");
}

/// An id a workflow author typed is not validated anywhere, so one carrying a
/// separator writes nothing rather than writing outside the Job's directory.
/// The verdict still stands; what is lost is the re-read, and the absent path
/// is how a reader is told so.
#[test]
fn an_id_that_is_not_one_path_component_keeps_nothing() {
    let dir = TempDir::new();
    let keeping = Asked::under(dir.path().to_string_lossy().to_string(), HANDLE.to_string());

    for (step, criterion) in [
        ("../implement", "c1"),
        ("implement", "../../etc/c1"),
        ("implement", ".."),
        ("", "c1"),
    ] {
        assert_eq!(
            keeping.kept(
                &StepId::new(step),
                Attempt::FIRST,
                &CriterionId::new(criterion),
                "the whole brief",
            ),
            None,
            "{step} / {criterion} names no file"
        );
    }
    assert!(
        !dir.path().join(".armada").exists(),
        "and nothing was written anywhere"
    );
}

/// The detached state, which the acceptance bench and this crate's own gate
/// tests stand in: a real call goes out and there is no repository under it.
#[tokio::test]
async fn a_gate_with_nowhere_to_write_still_rules() {
    let ruling = ruled(
        Arc::new(FakeJudge::with_no_objection()),
        Asked::nowhere(),
        &worktree(),
    )
    .await;

    assert_eq!(ruling.judged().len(), 2, "the verdicts still arrive");
    assert!(
        ruling
            .judged()
            .iter()
            .all(|judgment| judgment.brief_path.is_none()),
        "and each says plainly that nothing kept its brief"
    );
}
