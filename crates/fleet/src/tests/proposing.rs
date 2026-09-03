//! The Job proposer, end to end through Fleet's own runner.
//!
//! The fake renders a shell rather than a model, so what runs is Fleet's spawn,
//! Fleet's stdin write, its budget and this crate's own answer parser. Only the
//! model is faked.
//!
//! The cases the capability turns on are here — a request that resolves, one
//! that resolves to nothing and comes back unchanged, a call that could not be
//! made, and a plan whose second Job does not start before its first has
//! landed. So is the one they all exist to prevent: a workflow assigned because
//! it was the nearest thing on the list.
//!
//! What a request's own link resolves to before any of this runs is
//! [`crate::tests::linking`]'s — a different claim about a different call.

use std::collections::BTreeMap;

use config::ResolvedWorkflow;
use core_model::{Actor, WorkflowId};
use testkit::{FakeJudge, FakeWorkProduct};

use crate::adrift::Adrift;
use crate::proposing::{Brief, NotProposed, Proposal, Unresolved};
use crate::tests::daemon::{a_fleet, a_fleet_proposing_through, a_proposal, workflow_named};
use crate::tests::tmp::TempDir;

pub(crate) const A_REQUEST: &str = "the log reader drops the last line of every file";

/// The catalogue a request is chosen from. Three, because choosing correctly
/// from one proves nothing.
pub(crate) fn a_catalogue() -> Vec<ResolvedWorkflow> {
    vec![
        workflow_named("bug"),
        workflow_named("feature"),
        workflow_named("revert"),
    ]
}

fn held(workflows: Vec<ResolvedWorkflow>) -> BTreeMap<WorkflowId, ResolvedWorkflow> {
    workflows
        .into_iter()
        .map(|workflow| (workflow.id().clone(), workflow))
        .collect()
}

pub(crate) fn read(answer: &str) -> Result<Proposal, NotProposed> {
    let held = held(a_catalogue());
    Brief::about(A_REQUEST, &held).read(answer, &held)
}

/// A request that fits one workflow becomes a Job at the gate, under that one.
#[tokio::test]
async fn a_request_that_fits_one_workflow_reaches_the_approval_gate_under_it() {
    let home = TempDir::new();
    let fleet = a_fleet_proposing_through(
        &home,
        FakeWorkProduct::changed(&["src/log.rs"]),
        a_catalogue(),
        FakeJudge::saying(
            "workflow: bug\ntitle: The log reader drops the last line\n\
             because: a defect with a reproducible symptom\nwrites: src/log.rs",
        ),
    );

    let made = fleet
        .propose_from(A_REQUEST)
        .await
        .expect("a request that fits one workflow");

    let [job] = &made[..] else {
        panic!("one Job, not {}", made.len())
    };
    assert_eq!(job.workflow_id().as_str(), "bug");
    assert_eq!(job.title().as_str(), "The log reader drops the last line");
    assert_eq!(
        job.status().as_wire(),
        "awaiting_approval",
        "the proposer dispatches nothing — the gate is where it stops"
    );
    // The system chose the workflow, and the record says so. `manual` stays
    // what a hand-entered `propose_job` writes, so which of the two happened is
    // answerable rather than inferred.
    assert_eq!(job.origin().as_wire(), "auto_detected");
    assert_eq!(
        job.facts().as_str(),
        A_REQUEST,
        "the request survives onto the Job — a Drone briefed from a title alone \
         is one the description was thrown away for"
    );
    assert!(
        job.write_targets().is_none(),
        "scope is the first step's. Absent says it is not yet worked out, where \
         an empty list would claim the Job writes nothing"
    );
    assert!(
        !job.atomic(),
        "and coupling follows from paths there are none of yet"
    );
}

/// Entry zero is what says the call ran at all.
#[tokio::test]
async fn entry_zero_carries_the_reason_and_names_the_proposer() {
    let home = TempDir::new();
    let fleet = a_fleet_proposing_through(
        &home,
        FakeWorkProduct::changed(&["src/log.rs"]),
        a_catalogue(),
        FakeJudge::saying(
            "workflow: bug\ntitle: Fix the reader\n\
             because: a defect with a reproducible symptom\nwrites: src/log.rs",
        ),
    );

    let made = fleet.propose_from(A_REQUEST).await.expect("a proposal");

    let [zero] = made[0].scope_revisions() else {
        panic!("exactly one entry at creation")
    };
    assert_eq!(zero.rationale, "a defect with a reproducible symptom");
    assert_eq!(
        zero.approved_by,
        Actor::Fleet,
        "the call stated this scope, which is what makes it evaluable later"
    );
    assert!(
        zero.at_step.is_none(),
        "entry zero is before the first step"
    );
    assert!(
        zero.paths_added.is_empty(),
        "it adds no paths, because nothing has worked out what they are — the \
         scope step's own revision is the entry that will"
    );
}

/// The same record on the hand-entered path, told apart by who stated it.
#[tokio::test]
async fn hand_entry_writes_entry_zero_too_and_names_a_person() {
    let home = TempDir::new();
    let fleet = a_fleet(&home, FakeWorkProduct::changed(&["src/log.rs"]));

    let job = fleet
        .propose(a_proposal("typed in by hand"))
        .await
        .expect("a hand-entered proposal");

    let [zero] = job.scope_revisions() else {
        panic!("a hand-entered Job has an entry zero as well — a revert reads it")
    };
    assert_eq!(zero.approved_by, Actor::Human);
}

/// A request nothing fits is refused, and what comes back is what was sent.
#[tokio::test]
async fn a_request_no_workflow_fits_is_refused_and_returned_unchanged() {
    let home = TempDir::new();
    let fleet = a_fleet_proposing_through(
        &home,
        FakeWorkProduct::changed(&["src/log.rs"]),
        a_catalogue(),
        FakeJudge::saying(
            "workflow: none\nbecause: this asks for a release, and none of these run one",
        ),
    );

    let refused = fleet
        .propose_from(A_REQUEST)
        .await
        .expect_err("nothing on the list covers it");

    match &refused {
        Adrift::NoWorkflowFits { request, why } => {
            assert_eq!(
                request, A_REQUEST,
                "returned unchanged, character for character"
            );
            assert!(matches!(why, Unresolved::NoneFits { because: Some(_) }));
        }
        other => panic!("a refusal, not {other:?}"),
    }
    let (loaded, _) = fleet.every_job().await.expect("the board reads");
    assert!(
        loaded.jobs.is_empty(),
        "no Job was created, so there is nothing carrying a workflow nobody chose"
    );
}

/// A call that could not be made is not that refusal, and creates nothing.
#[tokio::test]
async fn a_call_that_fails_is_not_a_refusal_to_dispatch() {
    let home = TempDir::new();
    let fleet = a_fleet_proposing_through(
        &home,
        FakeWorkProduct::changed(&["src/log.rs"]),
        a_catalogue(),
        FakeJudge::that_fails("the network, the quota, the timeout"),
    );

    let failed = fleet
        .propose_from(A_REQUEST)
        .await
        .expect_err("the call did not come back");

    assert!(
        matches!(&failed, Adrift::NotProposed { request, .. } if request == A_REQUEST),
        "the outage is its own answer and carries the request back: {failed:?}"
    );
    assert!(
        !matches!(failed, Adrift::NoWorkflowFits { .. }),
        "an outage says nothing about the request, and must not read as a refusal"
    );
    let (loaded, _) = fleet.every_job().await.expect("the board reads");
    assert!(loaded.jobs.is_empty());
}

/// A workflow this repository does not hold is refused rather than
/// nearest-matched.
#[tokio::test]
async fn a_workflow_nothing_holds_does_not_become_the_nearest_one() {
    let home = TempDir::new();
    let fleet = a_fleet_proposing_through(
        &home,
        FakeWorkProduct::changed(&["src/log.rs"]),
        a_catalogue(),
        FakeJudge::saying("workflow: hotfix\ntitle: The log reader drops the last line"),
    );

    let refused = fleet
        .propose_from(A_REQUEST)
        .await
        .expect_err("`hotfix` is not on the list it was given");

    assert!(
        matches!(
            &refused,
            Adrift::NoWorkflowFits { why: Unresolved::NotHeld { named }, .. } if named == "hotfix"
        ),
        "refused by name rather than resolved to `bug`: {refused:?}"
    );
    let (loaded, _) = fleet.every_job().await.expect("the board reads");
    assert!(loaded.jobs.is_empty());
}

/// A blank request is refused before anything is spent on it.
#[tokio::test]
async fn a_blank_request_is_refused_before_the_call() {
    let home = TempDir::new();
    let fleet = a_fleet_proposing_through(
        &home,
        FakeWorkProduct::changed(&["src/log.rs"]),
        a_catalogue(),
        // A judge that would answer, so a call reaching it would succeed and
        // this test would pass for the wrong reason.
        FakeJudge::saying("workflow: bug\ntitle: something"),
    );

    let refused = fleet
        .propose_from("   \n  ")
        .await
        .expect_err("there is nothing in it to read");

    assert!(matches!(refused, Adrift::NothingToPropose), "{refused:?}");
}

/// A Job in the plan with no name proposes nothing, and Fleet writes none.
#[test]
fn a_job_that_names_no_title_proposes_nothing() {
    assert!(matches!(
        read("workflow: bug"),
        Err(NotProposed::NamesNoTitle { at: 1 })
    ));
}

/// An answer with no `workflow:` line at all is a call that did not answer,
/// not a request that was declined.
#[test]
fn an_answer_naming_no_workflow_is_unreadable_rather_than_a_refusal() {
    assert!(matches!(
        read("I think this is probably a bug of some kind."),
        Err(NotProposed::NamesNoWorkflow)
    ));
}

/// `none` is read as declining, whatever case it arrives in.
#[test]
fn declining_is_read_as_declining_and_not_as_a_workflow_named_none() {
    assert_eq!(
        read("workflow: None").expect("declining is a readable answer"),
        Proposal::Unresolved(Unresolved::NoneFits { because: None })
    );
}

/// What the call is told: the request, and every workflow with its steps.
///
/// Asserted on the brief rather than through a model, for `verification`'s
/// reason — what a call is given is built in one place and readable without
/// spending anything.
#[test]
fn the_call_is_told_the_request_and_every_workflow_with_its_steps() {
    let held = held(a_catalogue());

    let question = Brief::about(A_REQUEST, &held).question().to_string();

    assert!(question.contains(A_REQUEST), "the request, verbatim");
    for id in ["bug", "feature", "revert"] {
        assert!(
            question.contains(id),
            "`{id}` is on the list to choose from"
        );
        assert!(
            question.contains(&format!("only_in_{id}")),
            "and its steps, which are what tell one workflow from its neighbours"
        );
    }
    assert!(
        question.contains("Answer `none` rather than the nearest fit"),
        "the one line that stops a list being answered from"
    );
    assert!(
        question.contains("Write one Job unless"),
        "and the one that stops a proposer that can split work splitting it"
    );
}

/// What the call is denied. A Policy reads the request and the catalogue; a
/// thing that could read the repository is a Drone at many times the price.
#[test]
fn the_call_is_told_nothing_about_the_repository_or_the_board() {
    let held = held(a_catalogue());

    let question = Brief::about(A_REQUEST, &held).question().to_string();

    for withheld in ["armada.yml", "mechanical_checks", "advance_gate", "/tmp"] {
        assert!(
            !question.contains(withheld),
            "`{withheld}` reached a call that fires on every dispatch"
        );
    }
}

/// The answers, over the router that ships, against a real Fleet.
///
/// The statuses are the whole point of the split: a person retypes a request
/// that was refused and asks again for one whose call did not come back, and a
/// client that could not tell the two apart would offer the wrong one.
pub(crate) mod over_http {
    use axum::http::StatusCode;
    use ipc::{ProposedPlan, RunId, WireError, WireValue};
    use testkit::{FakeJudge, FakeWorkProduct};

    use super::{a_catalogue, A_REQUEST};
    use crate::tests::daemon::a_fleet_proposing_through;
    use crate::tests::http::call;
    use crate::tests::tmp::TempDir;

    const A_BODY: &str = r#"{"request": "the log reader drops the last line of every file"}"#;

    pub(crate) fn served(home: &TempDir, proposer: FakeJudge) -> axum::Router {
        let fleet = a_fleet_proposing_through(
            home,
            FakeWorkProduct::changed(&["src/log.rs"]),
            a_catalogue(),
            proposer,
        );
        let events = fleet.events();
        api::router(api::Served::by(fleet, RunId::carried("01RUN"), events))
    }

    #[tokio::test]
    async fn a_request_that_resolves_answers_the_job_at_the_gate() {
        let home = TempDir::new();
        let app = served(
            &home,
            FakeJudge::saying("workflow: bug\ntitle: The log reader drops the last line"),
        );

        let (status, body) = call(&app, "POST", "/jobs/from_request", A_BODY).await;

        assert_eq!(
            status,
            StatusCode::CREATED,
            "the same answer propose_job gives"
        );
        let plan: ProposedPlan = ipc::decode("a proposed plan", &body).expect("a ProposedPlan");
        let [job] = &plan.jobs[..] else {
            panic!("one Job")
        };
        assert_eq!(job.status.as_wire(), "awaiting_approval");
        assert_eq!(job.workflow_id.as_str(), "bug");
    }

    #[tokio::test]
    async fn a_request_nothing_fits_answers_422_and_hands_it_back() {
        let home = TempDir::new();
        let app = served(
            &home,
            FakeJudge::saying("workflow: none\nbecause: none of these"),
        );

        let (status, body) = call(&app, "POST", "/jobs/from_request", A_BODY).await;

        assert_eq!(status, StatusCode::UNPROCESSABLE_ENTITY);
        let error: WireError = ipc::decode("a refusal", &body).expect("a WireError");
        assert_eq!(error.code, "fleet.no_workflow_fits");
        assert_eq!(
            error.fields.get("request"),
            Some(&WireValue::Str(A_REQUEST.to_string())),
            "unchanged, so what the person retypes is what they wrote"
        );
    }

    /// A body with no request in it never reaches the daemon, so nothing is
    /// spent on it. 400 is the transport's own refusal.
    #[tokio::test]
    async fn a_body_that_names_no_request_is_refused_by_the_transport() {
        let home = TempDir::new();
        let app = served(&home, FakeJudge::saying("workflow: bug\ntitle: something"));

        let (status, _) = call(&app, "POST", "/jobs/from_request", "{}").await;

        assert_eq!(status, StatusCode::BAD_REQUEST);
    }

    #[tokio::test]
    async fn a_call_that_could_not_be_made_answers_500_and_not_422() {
        let home = TempDir::new();
        let app = served(&home, FakeJudge::that_fails("the network"));

        let (status, body) = call(&app, "POST", "/jobs/from_request", A_BODY).await;

        assert_eq!(
            status,
            StatusCode::INTERNAL_SERVER_ERROR,
            "an outage is not the request being refused"
        );
        let error: WireError = ipc::decode("a fault", &body).expect("a WireError");
        assert_eq!(error.code, "fleet.proposer_unreachable");
        assert_ne!(error.code, "fleet.no_workflow_fits");
    }
}
