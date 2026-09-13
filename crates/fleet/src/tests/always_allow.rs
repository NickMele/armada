//! A person answering Always allow picks a rule, and that rule — not the whole
//! command a Drone happened to type — is what `record_answer` keeps.
//!
//! `#834`: a chosen rule is what gets kept and recorded, an unoffered rule is
//! a 409, and a later command the rule covers runs without asking.
//!
//! `#836`: Always allow stops writing to `armada.yml`. The rule is kept in
//! Fleet's own store instead, a new Job against the same Manifest reads it
//! without asking, taking it back makes the next Job ask again, and a rule
//! the Manifest declares destructive stays withheld even where a row exists.
//!
//! `crate::tests::permitting` covers the fold and the refused-row path.

use core_model::{Actor, AllowedCommand, Reach, WhenBlocked};
use ipc::CommandAnswer;

use crate::daemon::Fleet;
use crate::permitting::{Answered, NotPermitted, Refusing};
use crate::slots::Concurrency;
use crate::tests::admitted::dispatched;
use crate::tests::daemon::{a_proposal, worktree_directory};
use crate::tests::permitting::{
    a_drone_that_reached_for, asked, dirty_manifest_job, the_fittings, until_waiting,
};
use crate::tests::tmp::TempDir;

const THE_COMMAND: &str = "gh issue view 792 --repo NickMele/armada 2>&1 | head -100";

/// **The whole of what `#834` asks for**, and — since `#836` — proof that the
/// rule lands in Fleet's own repository-wide table rather than a commit.
#[tokio::test]
async fn always_allow_keeps_the_chosen_rule_and_not_the_whole_command() {
    let home = TempDir::new();
    let (fleet, job, _armada_yml) = dirty_manifest_job(&home).await;
    fleet
        .set_when_blocked(&job, WhenBlocked::AskMe)
        .await
        .unwrap();
    let asking = asked("Bash", THE_COMMAND, "c1");
    let before_the_allow = fleet.vcs().committed().len();

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

    assert_eq!(
        fleet.vcs().committed().len(),
        before_the_allow,
        "an always-allow commits nothing, since #836"
    );
    assert!(
        fleet
            .store()
            .lock()
            .await
            .allowed_commands(&job)
            .unwrap()
            .is_empty(),
        "the row is the repository's, not this Job's own"
    );

    let allowed = fleet.repository_allowed(&fleet.first()).await;
    assert_eq!(allowed.len(), 1);
    assert_eq!(
        allowed[0].run, "gh issue view",
        "the rule was kept, and not the pipe and the redirect beside it"
    );
    assert_eq!(allowed[0].reach, Reach::Repository);
}

/// A rule that is not one of the command's own candidates is refused before
/// anything is written — a person can only pick what Fleet offered, never type
/// past it. **The call is still waiting afterwards**, proved by answering it a
/// second time rather than by a timer: `record_answer` fails before the reply
/// is ever sent, so nothing about the held call moved.
#[tokio::test]
async fn a_rule_the_command_does_not_offer_is_refused_and_the_call_still_waits() {
    let home = TempDir::new();
    let (fleet, job, armada_yml) = dirty_manifest_job(&home).await;
    fleet
        .set_when_blocked(&job, WhenBlocked::AskMe)
        .await
        .unwrap();
    let asking = asked("Bash", THE_COMMAND, "c1");
    let before = std::fs::read_to_string(&armada_yml).expect("the file, before");

    let (answer, (bad, good)) = tokio::join!(fleet.permission(&job, &asking), async {
        until_waiting(&fleet, &job).await;
        let bad = fleet
            .answer_command(
                &job,
                "c1",
                Answered::naming(
                    CommandAnswer::AlwaysAllow,
                    None,
                    Some("gh issue view --frobnicate"),
                ),
            )
            .await;
        let good = fleet
            .answer_command(&job, "c1", Answered::of(CommandAnswer::Reject, None))
            .await;
        (bad, good)
    });

    assert!(
        matches!(
            bad,
            Err(NotPermitted::RuleNotOffered { ref rule }) if rule == "gh issue view --frobnicate"
        ),
        "a rule this command never offered is a 409: {bad:?}"
    );
    assert!(
        fleet.repository_allowed(&fleet.first()).await.is_empty(),
        "nothing was recorded on the refused answer"
    );
    assert_eq!(
        std::fs::read_to_string(&armada_yml).expect("the file, after"),
        before,
        "and armada.yml was never touched"
    );

    good.expect("the call was still waiting, and reject is one of its offers");
    assert_eq!(
        answer,
        api::PermissionAnswer::Deny(Refusing::Rejected { note: None }.to_the_drone(THE_COMMAND)),
        "the 409 above never touched the held call, which a second, valid answer still reaches"
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

/// **`#836`'s own claim.** A Job that never itself asked still reads a rule a
/// different Job on the same Manifest always-allowed — no waiting, because
/// the repository's own row answers the permission tool before anyone asks.
#[tokio::test]
async fn a_new_job_on_the_same_manifest_runs_the_rule_without_asking() {
    let home = TempDir::new();
    let mut fittings = the_fittings(&home, a_drone_that_reached_for("c1"));
    fittings.concurrency = Concurrency::of(2);
    let fleet = Fleet::assembled(fittings);

    let first = fleet.propose(a_proposal("first job")).await.unwrap();
    worktree_directory(&home, &first);
    dispatched(&fleet, first.id()).await.unwrap();
    let first = first.id().clone();

    let asking = asked("Bash", THE_COMMAND, "c1");
    let (answer, answered) = tokio::join!(fleet.permission(&first, &asking), async {
        until_waiting(&fleet, &first).await;
        fleet
            .answer_command(
                &first,
                "c1",
                Answered::naming(CommandAnswer::AlwaysAllow, None, Some("gh issue view")),
            )
            .await
    });
    answered.expect("always allow is offered");
    assert!(matches!(answer, api::PermissionAnswer::Allow));

    let second = fleet.propose(a_proposal("second job")).await.unwrap();
    worktree_directory(&home, &second);
    dispatched(&fleet, second.id()).await.unwrap();
    let second = second.id().clone();

    let later = fleet
        .permission(
            &second,
            &asked("Bash", "gh issue view 800 --repo NickMele/armada", "c2"),
        )
        .await;
    assert!(
        matches!(later, api::PermissionAnswer::Allow),
        "a Job that never asked, granted the repository's own rule: {later:?}"
    );
}

/// Taking the rule back is read by the next permission question: the Job
/// asks again, exactly as one that never held the rule would.
#[tokio::test]
async fn removing_the_allow_makes_the_next_question_ask_again() {
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
    answered.expect("always allow is offered");
    assert_eq!(fleet.repository_allowed(&fleet.first()).await.len(), 1);

    fleet
        .remove_repository_allowed_command(&fleet.first(), "gh issue view")
        .await
        .expect("the rule was there to take back");
    assert!(fleet.repository_allowed(&fleet.first()).await.is_empty());

    let next_command = "gh issue view 900 --repo NickMele/armada";
    let asking = asked("Bash", next_command, "c2");
    let (answer, answered) = tokio::join!(fleet.permission(&job, &asking), async {
        until_waiting(&fleet, &job).await;
        fleet
            .answer_command(&job, "c2", Answered::of(CommandAnswer::Reject, None))
            .await
    });
    answered.expect("the call is waiting again, and reject is one of its offers");
    assert_eq!(
        answer,
        api::PermissionAnswer::Deny(Refusing::Rejected { note: None }.to_the_drone(next_command)),
        "asked again, not answered from a row that is gone"
    );
}

/// A rule a Manifest now declares destructive stays withheld even where a row
/// for it exists. The ordinary flow can never produce this shape —
/// `offers_after` refuses to offer Always allow on a destructive command — so
/// the row is planted directly, to prove the runtime read filters it rather
/// than trusting it from when it was kept.
#[tokio::test]
async fn a_destructive_command_stays_withheld_even_with_a_row_for_it() {
    let home = TempDir::new();
    let mut fittings = the_fittings(&home, a_drone_that_reached_for("c1"));
    fittings.manifest = config::Manifest::parse(
        std::path::Path::new("armada.yml"),
        "version: 1\nid: 01FIXTUREMANIFEST\ncommands:\n  publish:\n    run: \"npm publish\"\n    destructive: true\n",
    )
    .expect("a manifest that parses");
    let fleet = Fleet::assembled(fittings);
    let job = fleet
        .propose(a_proposal("publish the package"))
        .await
        .unwrap();
    worktree_directory(&home, &job);
    dispatched(&fleet, job.id()).await.unwrap();
    let job = job.id().clone();

    fleet
        .store()
        .lock()
        .await
        .allow_repository_command(
            fleet.first().manifest().id(),
            &AllowedCommand {
                run: "npm publish".to_string(),
                reach: Reach::Repository,
                allowed_at: fleet.now(),
                by: Actor::Human,
            },
        )
        .expect("the row is written directly, bypassing the ordinary offer");

    let answer = fleet
        .permission(&job, &asked("Bash", "npm publish", "c1"))
        .await;
    assert!(
        matches!(answer, api::PermissionAnswer::Deny(_)),
        "declared destructive, still withheld: {answer:?}"
    );
}
