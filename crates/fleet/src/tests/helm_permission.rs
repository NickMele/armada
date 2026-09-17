//! A call a Helm session's own settings do not cover, put to a person and
//! answered: allowed once, allowed and remembered, refused, and left
//! unanswered. `#1389`.
//!
//! **Driven through the surface and not through a process.** The permission
//! path is a route the agent CLI calls and a route a person answers on; what a
//! session does while it waits is the CLI's own, measured in spike 19. So these
//! cases hold the two halves of Fleet's part: what a person is shown, and what
//! goes back inside the call.
//!
//! The hold here is the two seconds `crate::tests::daemon::fleets` plants, so
//! the unanswered case can outlive it.

use std::sync::Arc;
use std::time::Duration;

use api::{Conversations, Next, Subscription};
use ipc::{AnswerHelmCall, AskingToRun, HelmCallAnswer, HelmCallSettled, RunOrNot};
use testkit::{FakeHarness, FakeVcs, FakeWorkProduct};

use crate::daemon::Fleet;
use crate::tests::daemon::fittings;
use crate::tests::tmp::TempDir;

type Hosted = Fleet<FakeHarness, FakeVcs, FakeWorkProduct>;

fn fleet(home: &TempDir) -> Arc<Hosted> {
    Arc::new(Fleet::assembled(fittings(
        home,
        FakeWorkProduct::changed(&[]),
    )))
}

/// One ask, **read the way the route reads it**: the CLI's own bytes, decoded.
/// `fleet` reads no JSON of its own — the gate scopes that to `store` and `ipc`
/// — so a fixture here is a body rather than a literal, which is what the door
/// would have handed over anyway.
fn asking(body: &str) -> AskingToRun {
    ipc::decode("a permission question", body.as_bytes()).expect("the fixture decodes")
}

fn shell(command: &str) -> AskingToRun {
    asking(&format!(
        "{{\"tool_name\":\"Bash\",\"input\":{{\"command\":\"{command}\",\
         \"description\":\"what it is for\"}},\"tool_use_id\":\"toolu_01\"}}"
    ))
}

/// The call a session is held inside, started and left running. What comes back
/// is the answer the CLI would act on.
fn held(
    fleet: &Arc<Hosted>,
    asking: AskingToRun,
) -> tokio::task::JoinHandle<Result<RunOrNot, api::Refusal>> {
    let fleet = Arc::clone(fleet);
    tokio::spawn(async move { fleet.ask_the_person(asking, None).await })
}

/// The one call waiting, once Fleet has put it on the table. Polled rather than
/// slept on, so a slow machine waits longer rather than failing.
async fn waiting(fleet: &Arc<Hosted>) -> ipc::HelmCallInFlight {
    for _ in 0..200 {
        let listed = fleet.list_helm_calls().await.expect("the list");
        if let Some(first) = listed.waiting.into_iter().next() {
            return first;
        }
        tokio::time::sleep(Duration::from_millis(10)).await;
    }
    panic!("nothing was ever put to a person");
}

async fn answer(fleet: &Arc<Hosted>, call: &str, answer: HelmCallAnswer, note: Option<&str>) {
    fleet
        .answer_helm_call(AnswerHelmCall {
            call: String::from(call),
            answer,
            note: note.map(String::from),
        })
        .await
        .expect("the answer lands");
}

async fn published(watching: &mut Subscription) -> Vec<ipc::Event> {
    let mut seen = Vec::new();
    while let Ok(Some(Next::Send(delivered))) =
        tokio::time::timeout(Duration::from_millis(50), watching.next()).await
    {
        seen.push(delivered.event);
    }
    seen
}

/// The owner's own test, in a case: ask for `gh`, be asked about it, allow it,
/// and the session runs it.
///
/// **`updatedInput` is what the CLI runs**, so it comes back exactly as it was
/// sent — anything else here would be Armada editing a person's command behind
/// them.
#[tokio::test]
async fn a_shell_line_is_put_to_a_person_and_runs_on_their_allow() {
    let home = TempDir::new();
    let fleet = fleet(&home);
    let sent = shell("gh issue list --milestone Helm");
    let call = held(&fleet, sent.clone());

    let asked = waiting(&fleet).await;
    assert_eq!(asked.tool, "Bash");
    assert_eq!(asked.detail, "gh issue list --milestone Helm");
    assert_eq!(asked.rule, "Bash(gh issue list:*)");
    assert_eq!(
        asked.offers,
        vec![
            HelmCallAnswer::AllowOnce,
            HelmCallAnswer::AllowAndRemember,
            HelmCallAnswer::Refuse,
        ]
    );
    answer(&fleet, &asked.call, HelmCallAnswer::AllowOnce, None).await;

    let decided = call.await.expect("the task").expect("an answer");
    assert_eq!(
        decided,
        RunOrNot::Allow {
            updated_input: sent.input,
        }
    );
    assert!(
        fleet
            .list_helm_calls()
            .await
            .expect("the list")
            .waiting
            .is_empty(),
        "an answered call is off the table"
    );
}

/// **Their words reach the model**, because a session told only "no" says
/// nothing useful about what it could not do.
#[tokio::test]
async fn a_refusal_carries_the_persons_own_words_to_the_session() {
    let home = TempDir::new();
    let fleet = fleet(&home);
    let call = held(&fleet, shell("rm -rf /tmp/everything"));

    let asked = waiting(&fleet).await;
    answer(
        &fleet,
        &asked.call,
        HelmCallAnswer::Refuse,
        Some("  we are not deleting that  "),
    )
    .await;

    let RunOrNot::Deny { message } = call.await.expect("the task").expect("an answer") else {
        panic!("a refusal is a deny");
    };
    assert!(message.contains("we are not deleting that"), "{message}");
    assert!(
        !message.contains("  we"),
        "the whitespace is keystrokes, not their reason: {message}"
    );
}

/// `allow_and_remember` is the only thing in Armada that writes a person's
/// settings, and it writes **one** rule — the one the card showed them.
#[tokio::test]
async fn allow_and_remember_writes_the_rule_a_person_read_and_nothing_else() {
    let home = TempDir::new();
    let fleet = fleet(&home);
    let call = held(&fleet, shell("gh pr view 1389 --json title"));

    let asked = waiting(&fleet).await;
    answer(&fleet, &asked.call, HelmCallAnswer::AllowAndRemember, None).await;
    call.await.expect("the task").expect("an answer");

    let at = home.path().join(adapters::PERSONAL_SETTINGS);
    let written = std::fs::read_to_string(&at).expect("the settings were written");
    assert!(written.contains("\"Bash(gh pr view:*)\""), "{written}");
    assert!(
        !written.contains("--json"),
        "the whole command is a rule a person would add again tomorrow: {written}"
    );
}

/// **Silence is not consent.** Nobody answers, the hold runs out, and the
/// session is told how long it waited — rather than a process held open until
/// the reply budget kills it.
#[tokio::test]
async fn nobody_answering_ends_the_wait_as_a_refusal_and_never_as_an_allow() {
    let home = TempDir::new();
    let fleet = fleet(&home);
    let call = held(&fleet, shell("gh issue list"));

    let asked = waiting(&fleet).await;
    assert_eq!(asked.holding_for_seconds, 2, "the fixture's own bound");

    let decided = tokio::time::timeout(Duration::from_secs(20), call)
        .await
        .expect("the hold ends the wait")
        .expect("the task")
        .expect("an answer");
    let RunOrNot::Deny { message } = decided else {
        panic!("an unanswered call is a deny, not an allow");
    };
    assert!(message.contains("2 seconds"), "{message}");
    assert!(
        fleet
            .list_helm_calls()
            .await
            .expect("the list")
            .waiting
            .is_empty(),
        "a card nobody answered comes down"
    );
}

/// An answer from a window left open across an ask names an id nothing is
/// waiting on. **A 409 and not a second answer to the next call.**
#[tokio::test]
async fn an_answer_naming_nothing_waiting_is_refused() {
    let home = TempDir::new();
    let fleet = fleet(&home);

    match fleet
        .answer_helm_call(AnswerHelmCall {
            call: String::from("helm-404"),
            answer: HelmCallAnswer::AllowOnce,
            note: None,
        })
        .await
    {
        Err(api::Refusal::IllegalMove(error)) => {
            assert_eq!(error.code, "fleet.helm_call_not_waiting")
        }
        other => panic!("refused, not {other:?}"),
    }
}

/// Every ask and every answer is on the record — `docs/concepts/helm.md`,
/// *Audit trail*. A person who finds a command was run reads the second to see
/// that they allowed it.
#[tokio::test]
async fn the_ask_and_the_answer_are_each_helms_own_event() {
    let home = TempDir::new();
    let fleet = fleet(&home);
    let mut watching = fleet.events().subscribe();
    let call = held(&fleet, shell("gh issue list"));

    let asked = waiting(&fleet).await;
    answer(&fleet, &asked.call, HelmCallAnswer::AllowOnce, None).await;
    call.await.expect("the task").expect("an answer");

    let events = published(&mut watching).await;
    let kinds: Vec<String> = events.iter().map(ipc::Event::kind).collect();
    assert!(
        kinds.contains(&String::from("helm.asking_to_run")),
        "{kinds:?}"
    );
    assert!(
        kinds.contains(&String::from("helm.call_answered")),
        "{kinds:?}"
    );
    let answered = events
        .iter()
        .find_map(|event| match event {
            ipc::Event::HelmCallAnswered(answered) => Some(answered),
            _ => None,
        })
        .expect("the answer is published");
    assert_eq!(answered.call, asked.call);
    assert_eq!(answered.settled, HelmCallSettled::AllowedOnce);
    assert_eq!(answered.rule, "Bash(gh issue list:*)");
}

/// A tool that is not a shell line is named by itself, because that is the rule
/// the CLI's settings would carry for it.
#[tokio::test]
async fn a_tool_that_is_not_a_command_is_its_own_rule() {
    let home = TempDir::new();
    let fleet = fleet(&home);
    let call = held(
        &fleet,
        asking("{\"tool_name\":\"mcp__gitnexus__list_repos\",\"input\":{}}"),
    );

    let asked = waiting(&fleet).await;
    assert_eq!(asked.rule, "mcp__gitnexus__list_repos");
    assert_eq!(asked.detail, "", "a tool with no argument this names");

    answer(&fleet, &asked.call, HelmCallAnswer::AllowOnce, None).await;
    call.await.expect("the task").expect("an answer");
}
