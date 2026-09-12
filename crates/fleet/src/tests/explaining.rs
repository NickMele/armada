//! Asking a model what one blocked command does, for the person deciding.
//!
//! What these prove: a reading comes back naming the model that gave it; a
//! model that will not answer is a refusal in words and leaves the command
//! exactly as it found it; and an id naming nothing a person can still answer
//! about reaches no model at all, so an invented one costs nothing.
//!
//! **Every case asserts the command is undisturbed**, because that is the whole
//! claim of a query that spends money: the offers are still the whole set and
//! the Drone is still held, whatever came back.
//!
//! The fixture is `crate::tests::permitting`'s, which is where a Job waiting on
//! a command is built.

use std::sync::Arc;

use api::Queries;
use core_model::WhenBlocked;
use ipc::CommandAnswer;
use testkit::FakeJudge;

use crate::permitting::Answered;
use crate::tests::permitting::{
    a_drone_that_reached_for, a_fleet_judged_by, asked, started, until_waiting, Fixture,
};
use crate::tests::tmp::TempDir;

/// What the scripted model says about the command. Prose, because prose is what
/// this operation answers with — there is nothing to parse and nothing to get
/// wrong but the absence of it.
const READING: &str = "It publishes the package to the public registry under the `next` tag. \
                       A published version cannot be replaced, so this one is not undoable.";

/// The command the fixture's Drone reaches for.
const REACHED_FOR: &str = "npm publish";

/// **The answer is a claim and says whose it is.** The model is the roster's
/// cheap end, resolved by the composition root and planted by the fixture — and
/// the command it was asked about is still waiting, with all three offers on it.
#[tokio::test]
async fn a_reading_comes_back_naming_the_model_that_gave_it() {
    let home = TempDir::new();
    let judge = Arc::new(FakeJudge::saying(READING));
    let fleet = a_fleet_judged_by(&home, a_drone_that_reached_for("c1"), Arc::clone(&judge));
    let job = started(&fleet, &home).await;
    fleet
        .set_when_blocked(&job, WhenBlocked::AskMe)
        .await
        .unwrap();
    let asking = asked("Bash", REACHED_FOR, "c1");

    let (_, read) = tokio::join!(fleet.permission(&job, &asking), async {
        until_waiting(&fleet, &job).await;
        let read = explained(&fleet, &job, "c1").await;
        let still = fleet
            .command_awaited(&job)
            .await
            .expect("asking is not answering: the command is still waiting");
        assert_eq!(still.offers.len(), 3, "and the offers are the whole set");
        fleet
            .answer_command(&job, "c1", Answered::of(CommandAnswer::Reject, None))
            .await
            .unwrap();
        read
    });

    let read = read.expect("a model that answered");
    assert_eq!(read.explanation, READING);
    assert_eq!(
        read.model, "the-cheap-model",
        "the dial the composition root resolved, named because a reading is a claim"
    );
    let asked_about = judge.asked();
    let [question] = asked_about.as_slice() else {
        panic!("one call, about one command: {asked_about:?}");
    };
    assert!(
        question.contains(REACHED_FOR),
        "the command itself is what was asked about: {question}"
    );
}

/// **A model that will not answer changes nothing.** The refusal is in words,
/// the Drone is still held inside its call, and the three offers are still the
/// whole set — so the person decides on exactly what they had before they
/// asked.
#[tokio::test]
async fn a_model_that_will_not_answer_leaves_the_command_waiting() {
    let home = TempDir::new();
    let judge = Arc::new(FakeJudge::that_fails(
        "the quota, the network, the credential",
    ));
    let fleet = a_fleet_judged_by(&home, a_drone_that_reached_for("c1"), Arc::clone(&judge));
    let job = started(&fleet, &home).await;
    fleet
        .set_when_blocked(&job, WhenBlocked::AskMe)
        .await
        .unwrap();
    let asking = asked("Bash", REACHED_FOR, "c1");

    let (answer, read) = tokio::join!(fleet.permission(&job, &asking), async {
        until_waiting(&fleet, &job).await;
        let read = explained(&fleet, &job, "c1").await;
        let still = fleet
            .command_awaited(&job)
            .await
            .expect("a failed reading is not an answer to the command");
        assert_eq!(still.offers.len(), 3, "and nothing about it is narrowed");
        fleet
            .answer_command(&job, "c1", Answered::of(CommandAnswer::AllowForJob, None))
            .await
            .unwrap();
        read
    });

    let refused = read.expect_err("the call could not be made");
    assert_eq!(refused.status(), 500, "nothing about the request was wrong");
    assert_eq!(refused.error().code, "fleet.not_explained");
    assert_eq!(
        answer,
        api::PermissionAnswer::Allow,
        "and the person's own answer still reaches the Drone inside its call"
    );
}

/// **An id naming nothing open reaches no model at all.** The two places a
/// still-answerable command lives are the only thing that resolves this id, so
/// an invented one is refused before a call is made rather than after.
#[tokio::test]
async fn an_id_naming_nothing_open_costs_no_call() {
    let home = TempDir::new();
    let judge = Arc::new(FakeJudge::saying(READING));
    let fleet = a_fleet_judged_by(&home, a_drone_that_reached_for("c1"), Arc::clone(&judge));
    let job = started(&fleet, &home).await;

    let refused = explained(&fleet, &job, "c9")
        .await
        .expect_err("nothing on this Job is waiting on c9, and nothing was refused on it");

    assert_eq!(refused.status(), 404);
    assert_eq!(refused.error().code, "fleet.nothing_to_explain");
    assert!(
        judge.asked().is_empty(),
        "and no model was asked: {:?}",
        judge.asked()
    );
}

/// The read, through the seam a client reaches it by.
async fn explained(
    fleet: &Fixture,
    job: &core_model::JobId,
    call: &str,
) -> Result<ipc::CommandExplained, api::Refusal> {
    Queries::explain_command(fleet, ipc::JobId::from(job), call.to_string()).await
}
