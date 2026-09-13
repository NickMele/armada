//! A person answering Always allow picks a rule, and that rule — not the whole
//! command a Drone happened to type — is what `record_answer` declares.
//!
//! `#834`: `covers` (`crate::permitting::covers`) refuses anything after an
//! allow that carries a shell operator, so declaring
//! `gh issue view 792 --repo X 2>&1 | head -100` whole covered exactly that
//! command and never the next `gh issue view`. What these prove: a chosen rule
//! is what gets declared and recorded, an unoffered rule is a 409 and nothing
//! is written, and a later command the declared rule covers runs without
//! asking.
//!
//! **The `AskMe` path.** Both paths answer through the one `record_answer` this
//! file is about; `crate::tests::permitting` covers the fold and the
//! refused-row path already.

use core_model::{Reach, WhenBlocked};
use ipc::CommandAnswer;

use crate::permitting::{Answered, NotPermitted};
use crate::tests::permitting::{asked, dirty_manifest_job, until_waiting, THE_TIP};
use crate::tests::tmp::TempDir;

const THE_COMMAND: &str = "gh issue view 792 --repo NickMele/armada 2>&1 | head -100";

/// **The whole of what `#834` asks for.** The rule a person picked is what
/// lands in `armada.yml` and on the Job's own allow list — not the command the
/// Drone actually ran, which stays free to carry the pipe and the redirect
/// `covers` would otherwise have to refuse forever.
#[tokio::test]
async fn always_allow_declares_the_chosen_rule_and_not_the_whole_command() {
    let home = TempDir::new();
    let (fleet, job, armada_yml) = dirty_manifest_job(&home).await;
    fleet
        .set_when_blocked(&job, WhenBlocked::AskMe)
        .await
        .unwrap();
    let asking = asked("Bash", THE_COMMAND, "c1");

    let (answer, answered) = tokio::join!(fleet.permission(&job, &asking), async {
        let waiting = until_waiting(&fleet, &job).await;
        assert_eq!(
            waiting.suggested_rule.as_deref(),
            Some("gh issue view"),
            "the pre-selection stops at the first argument: {waiting:?}"
        );
        assert!(
            waiting.rules.contains(&"gh issue view".to_string()),
            "and it is one of the candidates offered: {waiting:?}"
        );
        fleet
            .answer_command(
                &job,
                "c1",
                Answered::naming(CommandAnswer::AlwaysAllow, None, Some("gh issue view")),
            )
            .await
    });

    answered.expect("gh issue view is one of the command's own candidates");
    assert!(matches!(answer, api::PermissionAnswer::Allow));

    let allowed = fleet.store().lock().await.allowed_commands(&job).unwrap();
    assert_eq!(allowed.len(), 1);
    assert_eq!(
        allowed[0].run, "gh issue view",
        "the rule was recorded, and not the pipe and the redirect beside it"
    );
    assert_eq!(allowed[0].reach, Reach::Repository);

    let at_tip = config::Manifest::declaring_command(&armada_yml, THE_TIP, "gh issue view")
        .expect("the tip text takes the rule");
    let committed = fleet.vcs().committed();
    assert_eq!(
        committed.last().expect("the allow's own commit").scope,
        testkit::CommitScope::Content {
            path: "armada.yml".to_string(),
            content: at_tip.text,
        },
        "armada.yml carries the rule, not the command"
    );
}

/// A rule that is not one of the command's own candidates is refused before
/// anything is written — a person can only pick what Fleet offered, never type
/// past it.
#[tokio::test]
async fn a_rule_the_command_does_not_offer_is_refused_and_nothing_is_recorded() {
    let home = TempDir::new();
    let (fleet, job, armada_yml) = dirty_manifest_job(&home).await;
    fleet
        .set_when_blocked(&job, WhenBlocked::AskMe)
        .await
        .unwrap();
    let asking = asked("Bash", THE_COMMAND, "c1");
    let before = std::fs::read_to_string(&armada_yml).expect("the file, before");

    let (answer, answered) = tokio::join!(fleet.permission(&job, &asking), async {
        until_waiting(&fleet, &job).await;
        fleet
            .answer_command(
                &job,
                "c1",
                Answered::naming(
                    CommandAnswer::AlwaysAllow,
                    None,
                    Some("gh issue view --frobnicate"),
                ),
            )
            .await
    });

    assert!(
        matches!(
            answered,
            Err(NotPermitted::RuleNotOffered { ref rule }) if rule == "gh issue view --frobnicate"
        ),
        "a rule this command never offered is a 409: {answered:?}"
    );
    let api::PermissionAnswer::Deny(_) = answer else {
        panic!("the call is still waiting, unanswered: {answer:?}");
    };
    assert!(
        fleet
            .store()
            .lock()
            .await
            .allowed_commands(&job)
            .unwrap()
            .is_empty(),
        "nothing was recorded on the refused answer"
    );
    assert_eq!(
        std::fs::read_to_string(&armada_yml).expect("the file, after"),
        before,
        "and armada.yml was never touched"
    );
}

/// The declared rule covers a later command the way any always-allowed
/// command does — the point of picking a shorter rule in the first place.
#[tokio::test]
async fn a_later_command_the_rule_covers_runs_without_asking() {
    let home = TempDir::new();
    let (fleet, job, _armada_yml) = dirty_manifest_job(&home).await;
    fleet
        .set_when_blocked(&job, WhenBlocked::AskMe)
        .await
        .unwrap();
    let asking = asked("Bash", THE_COMMAND, "c1");
    let (_, answered) = tokio::join!(fleet.permission(&job, &asking), async {
        until_waiting(&fleet, &job).await;
        fleet
            .answer_command(
                &job,
                "c1",
                Answered::naming(CommandAnswer::AlwaysAllow, None, Some("gh issue view")),
            )
            .await
    });
    answered.expect("the first command is always-allowed as `gh issue view`");

    let later = fleet
        .permission(
            &job,
            &asked("Bash", "gh issue view 800 --repo NickMele/armada", "c2"),
        )
        .await;

    assert!(
        matches!(later, api::PermissionAnswer::Allow),
        "a different issue, the same rule: {later:?}"
    );
}
