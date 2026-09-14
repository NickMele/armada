//! The gaming check's second reading, through Fleet's own gate: a flag stops a
//! step only once a second reader agrees with it.
//!
//! Every judged case watches for `assertion_weakened` alone, so the calls a
//! judge sees are the first look and then, only where it flags, the second —
//! which is why these judges answer in turn rather than by fragment.

use std::sync::Arc;
use std::time::Duration;

use adapter_traits::{Environment, Footprint, Model};
use config::ResolvedWorkflow;
use core_model::GamingPattern;
use ipc::{JobDetail, RunId};
use testkit::{FakeJudge, FakeWorkProduct, Gaming, Sketch};
use verification::{Lifted, Request};

use crate::asked::Asked;
use crate::at_step::AtStep;
use crate::gate::{rule_on, Ruling};
use crate::judging::{JudgeBudget, Judging, Marking};
use crate::policy::Policies;
use crate::tests::admitted::dispatched;
use crate::tests::daemon::{a_fleet_judged_by, a_proposal, worktree_directory};
use crate::tests::detail::get;
use crate::tests::gate::{budget, diff_evidence, worktree};
use crate::tests::keeping::keeping_nowhere;
use crate::tests::tmp::TempDir;
use crate::tests::tools::submitted_by_the_one;

/// The shape that motivated the second reading: a module doc comment the plan
/// asked to narrow, quoted off its `-` line as an assertion that asserts less.
const A_REWORDED_DOC_COMMENT: &str =
    "diff --git a/crates/fleet/src/permitting.rs b/crates/fleet/src/permitting.rs\n\
     --- a/crates/fleet/src/permitting.rs\n\
     +++ b/crates/fleet/src/permitting.rs\n\
     @@ -3,1 +3,1 @@\n\
     -//! **Two silences are declined outright**: evidence at the gate, and a question\n\
     +//! **One silence is declined outright**: evidence at the gate. A question is held\n";

const FLAGGING_THE_DOC_COMMENT: &str = "flag: yes\ncited: the module doc drops \
     \"Two silences are declined outright: evidence at the gate, and a question\"";

/// An assertion deleted outright, with nothing in the change checking what it did.
const A_DELETED_ASSERTION: &str =
    "diff --git a/crates/store/src/tests/reader.rs b/crates/store/src/tests/reader.rs\n\
     --- a/crates/store/src/tests/reader.rs\n\
     +++ b/crates/store/src/tests/reader.rs\n\
     @@ -41,1 +40,0 @@\n\
     -    assert_eq!(rows.len(), 3);\n";

const FLAGGING_THE_ASSERTION: &str =
    "flag: yes\ncited: the test removes \"assert_eq!(rows.len(), 3);\"";

const AGREEING: &str = "The deleted line was the only check on how many rows the reader \
     returns, and nothing added here checks it.\n\nagree: yes\nwhy: The only check on the row \
     count is deleted and nothing replaces it.";

const DISAGREEING: &str = "The earlier step moved the bound into the reader itself.\n\n\
     agree: no\nwhy: The earlier step's evidence called for this assertion to go, because the \
     reader now enforces the bound it checked.";

const IN_PROSE: &str = "I think this one is probably fine, on balance.";

/// What the kept briefs are filed under.
const HANDLE: &str = "1080-read-a-flag-twice";

fn watching_for(flag_if: &'static [&'static str]) -> ResolvedWorkflow {
    testkit::resolved(&[Sketch {
        id: "implement",
        label: "Implement",
        evidence_type: Some("diff"),
        gates: &[],
        judged_on: &[],
        scope: None,
        gaming: Some(Gaming {
            baseline: None,
            flag_if,
        }),
    }])
}

async fn ruled(
    workflow: ResolvedWorkflow,
    patch: &str,
    judge: &Arc<FakeJudge>,
    asked: Asked,
) -> Ruling {
    let worktree = worktree();
    let at = AtStep::first(workflow.frozen(), &worktree).expect("a first step");
    let work = FakeWorkProduct::changed(&["crates/store/src/tests/reader.rs"]).showing(patch);
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
        &crate::places::Room::ignoring_the_machine(crate::places::ChecksAtOnce::of(4)),
        &Judging {
            client: Arc::clone(judge) as _,
            budget: JudgeBudget::of(Duration::from_secs(20)),
            default_model: Model::named("the-cheap-model").expect("a model name"),
            second_opinion_model: Model::named("the-second-model").expect("a model name"),
            environment: Environment::nothing(),
            marking: Marking::detached(),
            asked,
        },
        &keeping_nowhere(),
        Policies::unstated(),
        &crate::underway::Announcing::nowhere(),
        &std::collections::BTreeMap::new(),
        &[],
        core_model::WhenRefused::default(),
        &[],
        None,
        None,
    )
    .await
}

/// **The false positive this was built for.** A reworded doc comment cited as
/// an assertion that asserts less raises nothing, decided off the cited line
/// with no second call. The second answer scripted here would have stopped it.
#[tokio::test]
async fn a_doc_comment_cited_as_a_weakened_assertion_raises_nothing() {
    let judge = Arc::new(FakeJudge::answering_in_turn(&[
        FLAGGING_THE_DOC_COMMENT,
        AGREEING,
    ]));
    let ruling = ruled(
        watching_for(&["assertion_weakened"]),
        A_REWORDED_DOC_COMMENT,
        &judge,
        Asked::nowhere(),
    )
    .await;
    assert!(ruling.advanced(), "{ruling:?}");
    assert!(ruling.gaming_flags().is_empty(), "{ruling:?}");
    assert_eq!(judge.asked().len(), 1, "the first look, and no second");
}

/// **The other half of the definition of done.** An assertion deleted with
/// nothing replacing it is flagged, read again, agreed with, and stops the step.
#[tokio::test]
async fn a_deleted_assertion_a_second_reading_agrees_with_stops_the_step() {
    let judge = Arc::new(FakeJudge::answering_in_turn(&[
        FLAGGING_THE_ASSERTION,
        AGREEING,
    ]));
    let ruling = ruled(
        watching_for(&["assertion_weakened"]),
        A_DELETED_ASSERTION,
        &judge,
        Asked::nowhere(),
    )
    .await;
    let Ruling::Suspect { ref flagged, .. } = ruling else {
        panic!("the flag did not stop the step: {ruling:?}");
    };
    assert_eq!(flagged.patterns(), [GamingPattern::AssertionWeakened]);
    assert!(flagged.cited()[0].stands());
    assert_eq!(
        judge.asked().len(),
        2,
        "one first look and one second reading"
    );
}

/// **A disagreement advances the step, and the flag is kept.** Cleared rather
/// than dropped, with the reason and both calls on disk beside each other.
#[tokio::test]
async fn a_second_reading_that_disagrees_advances_the_step_with_the_flag_cleared() {
    let dir = TempDir::new();
    let root = dir.path().to_string_lossy().to_string();
    let judge = Arc::new(FakeJudge::answering_in_turn(&[
        FLAGGING_THE_ASSERTION,
        DISAGREEING,
    ]));
    let ruling = ruled(
        watching_for(&["assertion_weakened"]),
        A_DELETED_ASSERTION,
        &judge,
        Asked::under(root, HANDLE.to_string()),
    )
    .await;
    assert!(ruling.advanced(), "{ruling:?}");
    let flags = ruling.gaming_flags();
    assert_eq!(flags.len(), 1, "the cleared flag rides on the advance");
    let cleared = flags[0].cleared.as_ref().expect("cleared, with its reason");
    assert!(
        cleared
            .why
            .starts_with("The earlier step's evidence called for"),
        "{}",
        cleared.why
    );
    let second = cleared
        .brief_path
        .as_deref()
        .expect("the second call is kept");
    assert_eq!(
        second,
        format!(".armada/briefs/{HANDLE}/implement.1.gaming.assertion_weakened.second.txt")
    );
    let kept = std::fs::read_to_string(dir.path().join(second)).expect("on disk");
    assert_eq!(
        judge.asked().get(1),
        Some(&kept),
        "the file is what the second call was handed"
    );
    assert_eq!(
        flags[0].brief_path.as_deref(),
        Some(format!(".armada/briefs/{HANDLE}/implement.1.gaming.assertion_weakened.txt").as_str()),
        "and the first look keeps its own"
    );
}

/// **Prose cleared nothing.** A second reading without its two lines checked
/// nothing, so the flag stands and a person decides.
#[tokio::test]
async fn a_second_reading_in_prose_leaves_the_flag_standing() {
    let judge = Arc::new(FakeJudge::answering_in_turn(&[
        FLAGGING_THE_ASSERTION,
        IN_PROSE,
    ]));
    let ruling = ruled(
        watching_for(&["assertion_weakened"]),
        A_DELETED_ASSERTION,
        &judge,
        Asked::nowhere(),
    )
    .await;
    let Ruling::Suspect { ref flagged, .. } = ruling else {
        panic!("prose cleared the flag: {ruling:?}");
    };
    assert!(flagged.cited()[0].stands());
}

/// The second reading runs on its own model whatever the step names, and is
/// shown the flag and its question beside the diff — never the Drone's claim.
#[tokio::test]
async fn the_second_reading_is_on_its_own_model_and_shown_no_claim() {
    let judge = Arc::new(FakeJudge::answering_in_turn(&[
        FLAGGING_THE_ASSERTION,
        AGREEING,
    ]));
    ruled(
        watching_for(&["assertion_weakened"]),
        A_DELETED_ASSERTION,
        &judge,
        Asked::nowhere(),
    )
    .await;
    assert_eq!(judge.models(), ["the-cheap-model", "the-second-model"]);
    let second = &judge.asked()[1];
    let asked = GamingPattern::AssertionWeakened
        .question()
        .expect("a judged pattern is asked something");
    assert!(second.contains(asked), "{second}");
    assert!(second.contains("assert_eq!(rows.len(), 3);"), "{second}");
    assert!(!second.contains("The loop is a fold."), "{second}");
}

/// A flag the patch decided is a fact about the patch, and no model reads it
/// again. The judge here would clear anything it were asked about.
#[tokio::test]
async fn a_flag_the_diff_decided_is_not_read_again() {
    let judge = Arc::new(FakeJudge::saying(
        "agree: no\nwhy: The configuration change was asked for.",
    ));
    let edited = "diff --git a/jest.config.js b/jest.config.js\n\
                  -  testPathIgnorePatterns: [\"/node_modules/\"],\n\
                  +  testPathIgnorePatterns: [\"/node_modules/\", \"/tests/edge-cases/\"],\n";
    let ruling = ruled(
        watching_for(&["check_config_edited"]),
        edited,
        &judge,
        Asked::nowhere(),
    )
    .await;
    assert!(matches!(ruling, Ruling::Suspect { .. }), "{ruling:?}");
    assert!(judge.asked().is_empty(), "{:?}", judge.asked());
}

/// **The seam.** A cleared flag reaches the store and the detail view a person
/// opens, beside the step it was raised on, which advanced.
#[tokio::test]
async fn a_cleared_flag_reaches_the_detail_view_beside_an_advanced_step() {
    let home = TempDir::new();
    let fleet = a_fleet_judged_by(
        &home,
        FakeWorkProduct::changed(&["crates/store/src/tests/reader.rs"])
            .showing(A_DELETED_ASSERTION),
        testkit::resolved(&[
            Sketch {
                id: "implement",
                label: "Implement",
                evidence_type: Some("diff"),
                gates: &[],
                judged_on: &[],
                scope: None,
                gaming: Some(Gaming {
                    baseline: None,
                    flag_if: &["assertion_weakened"],
                }),
            },
            Sketch {
                id: "summarise",
                label: "Summarise",
                evidence_type: Some("facts_note"),
                gates: &[],
                judged_on: &[],
                scope: None,
                gaming: None,
            },
        ]),
        FakeJudge::answering_in_turn(&[FLAGGING_THE_ASSERTION, DISAGREEING]),
    );
    let job = fleet
        .propose(a_proposal("drop the bound the reader now enforces"))
        .await
        .expect("a Job at the gate");
    let job_id = job.id().clone();
    worktree_directory(&home, &job);
    dispatched(&fleet, &job_id).await.expect("released to run");
    submitted_by_the_one(&fleet, crate::tests::daemon::diff_evidence())
        .await
        .expect("the tool took it");
    let turned = fleet.turn().await.expect("the gate ruled");
    assert!(
        matches!(turned.ruled(), Some(Ruling::Advanced { .. })),
        "{:?}",
        turned.ruled()
    );

    let events = fleet.events();
    let app = api::router(api::Served::by(fleet, RunId::carried("01RUN"), events));
    let (_, body) = get(&app, &format!("/jobs/{}", job_id.as_str())).await;
    let detail: JobDetail = ipc::decode("a Job in full", &body).expect("a JobDetail");
    let flagged = &detail.steps[0].flagged;
    assert_eq!(flagged.len(), 1, "{flagged:?}");
    let cleared = flagged[0].cleared.as_ref().expect("cleared on the wire");
    assert!(
        cleared
            .why
            .starts_with("The earlier step's evidence called for"),
        "{cleared:?}"
    );
    assert!(
        cleared
            .brief_path
            .as_deref()
            .is_some_and(|path| path.ends_with(".gaming.assertion_weakened.second.txt")),
        "{cleared:?}"
    );
}
