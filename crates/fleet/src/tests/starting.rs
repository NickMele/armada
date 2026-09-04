//! A Job that never started, and the trigger that says who fixes it.
//!
//! Eight sites across `dispatch`, `readmitting` and `spawning` escalate before
//! any process exists, and every one said `interrupted` until 2026-08-31 —
//! "a Job marked running has no matching OS process", which sent whoever read
//! the badge hunting for a Drone that had never been spawned. They are grouped
//! now by who fixes the failure rather than by which line raised it:
//! `no_worktree` is the disk or the repository, `not_configurable` is the
//! Manifest or the model roster, `would_not_start` is the environment the
//! daemon runs in.
//!
//! **The assertion in every case is the trigger and not the `Adrift`.** The
//! error was always right; what was wrong was the one word a person sees before
//! they open anything, and no test held it.
//!
//! # What is not driven from here
//!
//! Two of the eight sites have no seam to fail at. `FakeVcs::bring_up_to_date`
//! cannot refuse, so the catch-up that will not run is asserted only by the
//! arm's own construction, and a transcript that will not open needs an
//! unwritable directory under a `TempDir` this suite owns. Both share a trigger
//! with a site that *is* driven here — `no_worktree` and `would_not_start` —
//! which is the argument for grouping by remedy rather than by call site: a
//! path with no seam still gets the right word.

use core_model::{EscalationTrigger, JobStatus, TransitionReason, TriggerLevel};
use testkit::{FakeHarness, FakeWorkProduct};

use crate::adrift::Adrift;
use crate::daemon::Fleet;
use crate::tests::admitted::{admit, dispatched};
use crate::tests::daemon::{a_fleet, a_proposal, fitted_with, fittings, worktree_directory};
use crate::tests::reviewing::{a_fleet_reviewing_the_first_step, at_the_gate};
use crate::tests::tmp::TempDir;

/// The trigger the Job's last transition names.
async fn stopped_by<H, V, W>(fleet: &Fleet<H, V, W>, job: &core_model::JobId) -> EscalationTrigger
where
    H: adapter_traits::AgentHarness + Send + Sync + 'static,
    H::Error: std::error::Error + Send + Sync + 'static,
    V: adapter_traits::Vcs + adapter_traits::Delivery + Send + Sync + 'static,
    V::Error: std::error::Error + Send + Sync + 'static,
    V::CommitError: std::error::Error + Send + Sync + 'static,
    W: adapter_traits::WorkProduct + Send + Sync + 'static,
    W::Error: std::error::Error + Send + Sync + 'static,
{
    assert_eq!(
        fleet.load(job).await.expect("the Job is readable").status(),
        JobStatus::Escalated,
        "a Job that could not be started is stopped for a person, not left in the queue"
    );
    match fleet.last_reason(job).await.expect("a reason was written") {
        Some(TransitionReason::Escalation(trigger)) => trigger,
        other => panic!("the last transition names no escalation: {other:?}"),
    }
}

/// **(a)** git would not make the worktree. The disk, a permission, a
/// repository that is not one — the fake stands in for all three, because the
/// Job stops the same way on each and the same person answers for it.
#[tokio::test]
async fn a_worktree_git_would_not_make_stops_the_job_as_no_worktree() {
    let home = TempDir::new();
    let fleet = a_fleet(&home, FakeWorkProduct::changed(&["src/log.rs"]));
    let job = fleet
        .propose(a_proposal("a Job with nowhere to work"))
        .await
        .expect("proposed");
    worktree_directory(&home, job.id());
    fleet.vcs().refuse_next("a full disk");

    let refused = dispatched(&fleet, job.id())
        .await
        .expect_err("a Job with no worktree is not dispatched");
    assert!(
        matches!(refused, Adrift::NoWorktree { .. }),
        "expected NoWorktree, got {refused:?}"
    );

    assert_eq!(
        stopped_by(&fleet, job.id()).await,
        EscalationTrigger::NoWorktree,
        "not `interrupted` — nothing was spawned, so there is no process to be missing"
    );
    assert_eq!(
        EscalationTrigger::NoWorktree.level(),
        TriggerLevel::Job,
        "no step is the reason: the worktree is a fact about the Job"
    );
}

/// **(b)** The worktree exists and the attachments would not copy into it.
///
/// **The same trigger as (a), deliberately.** A worktree missing the files the
/// brief tells the Drone to open is not one work can start in, and the person
/// who answers for it is the person who answers for the disk. A second trigger
/// here would split one remedy across two badges.
#[tokio::test]
async fn attachments_that_will_not_copy_stop_the_job_as_no_worktree() {
    let home = TempDir::new();
    let fleet = a_fleet(&home, FakeWorkProduct::changed(&["src/log.rs"]));

    let staged_dir = home.path().join("staged");
    std::fs::create_dir_all(&staged_dir).expect("a staging directory");
    let staged_path = staged_dir.join("repro.png");
    std::fs::write(&staged_path, b"a screenshot").expect("a staged file");
    let mut proposal = a_proposal("a Job whose attachment goes missing");
    proposal.attachments = vec![ipc::AttachmentRef {
        staged_path: staged_path.to_string_lossy().to_string(),
        filename: "repro.png".to_string(),
        mime_type: "image/png".to_string(),
    }];

    let job = fleet.propose(proposal).await.expect("promoted");
    worktree_directory(&home, job.id());
    // Between promotion and dispatch, which is where a reaper or a person with
    // a shell gets to it. The Job still names the attachment; the bytes have
    // gone.
    for attachment in job.attachments() {
        std::fs::remove_file(&attachment.storage_ref).expect("the promoted copy is reclaimed");
    }

    let refused = dispatched(&fleet, job.id())
        .await
        .expect_err("a worktree the brief's files are not in is not dispatched");
    let Adrift::AttachmentUnreadable { filename, .. } = &refused else {
        panic!("expected AttachmentUnreadable, got {refused:?}");
    };
    assert_eq!(filename, "repro.png");

    assert_eq!(
        stopped_by(&fleet, job.id()).await,
        EscalationTrigger::NoWorktree,
        "one remedy, one trigger — the disk answers for this exactly as it answers for (a)"
    );
}

/// **(c)** A Job readmitted after a person's answer, whose worktree has been
/// reclaimed while it waited in the queue.
///
/// One of the four acts `crate::readmitting` serves; all four arrive at the
/// same read and the same arm.
///
/// **This arm could not escalate at all until 2026-08-31.** The Job is `queued`
/// here, `queued -> escalated` is the edge `dependency_failed` owns, and an
/// edge that declares a trigger accepts that trigger and no other — so asking
/// for `interrupted` from here returned the machine's `WrongTrigger`, which the
/// `?` then returned in place of the missing worktree. The Job stayed `queued`
/// and admission failed on it again every turn. The move through `running` is
/// what makes an escalation reachable, and both halves are asserted: the error
/// the admission gets, and the status a person sees.
///
/// **The error is no longer the approving caller's**, which is `#456`. The
/// approval answers `queued` and the disk is met on the turn that tries to put
/// a Drone back on — so what a person pressed comes back clean and the Job
/// escalates a tick later, carrying the same trigger and the same remedy.
#[tokio::test]
async fn a_readmitted_job_whose_worktree_is_gone_stops_as_no_worktree() {
    let home = TempDir::new();
    let fleet = a_fleet_reviewing_the_first_step(&home, FakeWorkProduct::changed(&["src/log.rs"]));
    let job_id = at_the_gate(&fleet, &home).await;

    let job = fleet.load(&job_id).await.expect("the Job is there");
    std::fs::remove_dir_all(
        fleet
            .surviving_worktree(&job)
            .expect("it is still there while the person reads")
            .path(),
    )
    .expect("the worktree is reclaimed");

    fleet
        .approve_review(&job_id)
        .await
        .expect("the person's decision lands whatever the disk holds");
    let refused = admit(&fleet)
        .await
        .expect_err("there is nothing to put a Drone back onto");
    assert!(
        matches!(refused, Adrift::WorktreeGone { .. }),
        "the missing worktree is what comes back, not the machine's refusal of a \
         trigger: {refused:?}"
    );

    assert_eq!(
        stopped_by(&fleet, &job_id).await,
        EscalationTrigger::NoWorktree,
        "the earlier steps' work is not on disk — the badge says so and names who fixes it"
    );
}

/// **(d)** Everything is on disk and the spawn will not be built.
///
/// The MCP config path stands in for the whole family, `crates/adapters`'
/// `SpawnConfigRefused`: a model name no roster row carries is the case
/// `crates/config/src/roster.rs` expects most, and it arrives at exactly this
/// arm. A person is sent to `armada.yml` or the roster, which is where the
/// answer is, and `interrupted` sent them to a transcript that does not exist.
#[tokio::test]
async fn a_spawn_config_that_is_refused_stops_the_job_as_not_configurable() {
    let home = TempDir::new();
    let mut fittings = fittings(&home, FakeWorkProduct::changed(&["src/log.rs"]));
    fittings.host.mcp_config = String::from("etc/armada/mcp.json");
    let fleet = Fleet::assembled(fittings);

    let job = fleet
        .propose(a_proposal("a Job no spawn can be built for"))
        .await
        .expect("proposed");
    worktree_directory(&home, job.id());

    let refused = dispatched(&fleet, job.id())
        .await
        .expect_err("a Drone that cannot be confined is not spawned");
    assert!(
        matches!(refused, Adrift::NotConfigurable { .. }),
        "expected NotConfigurable, got {refused:?}"
    );

    assert_eq!(
        stopped_by(&fleet, job.id()).await,
        EscalationTrigger::NotConfigurable,
        "a value somebody wrote, not a process that died"
    );
    assert!(
        fleet.harness().configured().is_empty(),
        "nothing was ever handed to the harness"
    );
}

/// **(e)** The configuration was good and the machine still said no.
///
/// The far end of `interrupted`: a process that never was, against a process
/// that was there and is not. A person reading this looks at the machine —
/// disk, permissions, the agent binary — and not at anything in the
/// repository, which is the whole reason it is its own trigger and not
/// `not_configurable`.
#[tokio::test]
async fn a_harness_that_refuses_to_spawn_stops_the_job_as_would_not_start() {
    let home = TempDir::new();
    let fleet = Fleet::assembled(fitted_with(
        &home,
        FakeWorkProduct::changed(&["src/log.rs"]),
        FakeHarness::refusing("no agent binary on this machine"),
    ));

    let job = fleet
        .propose(a_proposal("a Job the machine will not start"))
        .await
        .expect("proposed");
    worktree_directory(&home, job.id());

    let refused = dispatched(&fleet, job.id())
        .await
        .expect_err("a Drone that would not launch leaves the Job stopped");
    assert!(
        matches!(refused, Adrift::NoDrone { .. }),
        "expected NoDrone, got {refused:?}"
    );

    assert_eq!(
        stopped_by(&fleet, job.id()).await,
        EscalationTrigger::WouldNotStart,
        "no Drone ever ran, so there is no transcript for a reader to go through"
    );
    assert!(
        fleet.working_on().await.is_empty(),
        "the slot came free — nothing is being worked"
    );
}

/// **(f)** The three of them are Job-level and none may stand as a step's
/// verdict.
///
/// `last_verdict` admits step-level triggers only, and nothing about the work
/// was weighed at any of these sites — no evidence was submitted and no
/// criterion was read. Asserted over the set rather than one at a time so that
/// a fourth added later is not quietly narrower.
#[test]
fn none_of_the_three_can_stand_as_a_step_s_verdict() {
    for trigger in [
        EscalationTrigger::NoWorktree,
        EscalationTrigger::NotConfigurable,
        EscalationTrigger::WouldNotStart,
    ] {
        assert_eq!(trigger.level(), TriggerLevel::Job);
        assert!(
            core_model::StepLevelTrigger::of(trigger).is_none(),
            "`{}` narrowed to a step-level trigger, and no step is its reason",
            trigger.as_wire()
        );
    }
}
