//! Three claims a running Job makes about itself.
//!
//! Its workflow does not move under it, it knows which Drone is on it, and a
//! Check that ran left its output somewhere a person can read.
//!
//! The first is tested the way the world produces it: a second Fleet over the
//! same store, holding a workflow that has been edited. Fleet reads
//! `.armada/workflows/` once at assembly, so "somebody edited the file" and "a
//! Fleet restarted holding a different definition" are one event.

use core_model::{
    Actor, DroneId, IllegalDroneMove, JobStatus, JobStep, ResolvedCheck, StepId, StepState,
    Timestamp, Ulid,
};
use testkit::{FakeWorkProduct, Gate, Sketch};

use crate::gate::Ruling;
use crate::tests::daemon::{
    a_fleet, a_fleet_holding, a_fleet_holding_all, a_proposal, a_proposal_for, diff_evidence,
    workflow_named, worktree_directory,
};
use crate::tests::tmp::TempDir;

fn changed() -> FakeWorkProduct {
    FakeWorkProduct::changed(&["src/log.rs"])
}

/// The fixture's two step ids, with `implement` gated on a command that cannot
/// pass instead of on a non-empty diff.
///
/// Same `workflow_id`, because editing a file does not mint a new workflow —
/// which is exactly why comparing ids could never have caught this.
fn two_steps_edited() -> config::ResolvedWorkflow {
    testkit::resolved(&[
        Sketch {
            id: "implement",
            label: "Implement, but differently",
            evidence_type: Some("diff"),
            gates: &[Gate::Check {
                name: "suite",
                run: "/usr/bin/false",
                expect_exit_code: 0,
                when: &[],
            }],
            judged_on: &[],
            scope: None,
            gaming: None,
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
    ])
}

/// The gate a Job runs is the one it froze, not the one on disk.
#[tokio::test]
async fn a_workflow_edited_under_a_job_changes_nothing_about_it() {
    let home = TempDir::new();
    let job_id = {
        let fleet = a_fleet(&home, changed());
        let job = fleet
            .propose(a_proposal("fix the off-by-one"))
            .await
            .unwrap();
        worktree_directory(&home, job.id());
        job.id().clone()
    };

    // The file is edited and Fleet restarts. `implement` is now gated on a
    // command that exits 1, which this Job would fail on if the file won.
    let after = a_fleet_holding(&home, changed(), two_steps_edited(), 100);
    after.approve(&job_id).await.unwrap();
    after.submit_evidence(diff_evidence()).await.unwrap();

    let ruling = after.turn().await.unwrap().ruled.expect("the gate ruled");
    assert!(
        matches!(ruling, Ruling::Advanced { .. }),
        "the frozen gate was `diff_nonempty` and it passed; the edited one \
         could not have. Found {ruling:?}"
    );

    let declared = after
        .load(&job_id)
        .await
        .unwrap()
        .workflow()
        .step(&StepId::new("implement"))
        .expect("the frozen step")
        .clone();
    assert_eq!(
        declared.label(),
        "Implement",
        "the label the Job was approved with, not the one on disk"
    );
    assert_eq!(
        declared.checks(),
        &[ResolvedCheck::DiffNonempty],
        "and the Check it was approved with"
    );
}

/// What `get_job` serves is what the gate ran, because both read the Job.
#[tokio::test]
async fn what_a_person_is_shown_is_what_the_job_froze() {
    let home = TempDir::new();
    let job_id = {
        let fleet = a_fleet(&home, changed());
        let job = fleet
            .propose(a_proposal("fix the off-by-one"))
            .await
            .unwrap();
        job.id().clone()
    };

    let after = a_fleet_holding(&home, changed(), two_steps_edited(), 100);
    let detail = api::Daemon::get_job(&after, ipc::JobId::from(&job_id))
        .await
        .expect("the Job reads");
    let checks = detail.steps[0]
        .checks
        .as_ref()
        .expect("Fleet can always say, now that the Job carries the answer");
    assert_eq!(checks.len(), 1);
    assert_eq!(
        checks[0].kind, "diff_nonempty",
        "the frozen declaration, not the edited file's `suite`"
    );
    assert_eq!(
        detail.steps[0].label, "Implement",
        "the word the Job froze, not the one on disk"
    );
}

/// One step gated on a named Check, so the command has somewhere to come from.
fn gated_on(label: &str, run: &str) -> config::ResolvedWorkflow {
    testkit::resolved(&[Sketch {
        id: "implement",
        label,
        evidence_type: Some("diff"),
        gates: &[Gate::Check {
            name: "build",
            run,
            expect_exit_code: 0,
            when: &[],
        }],
        judged_on: &[],
        scope: None,
        gaming: None,
    }])
}

/// **What the rail draws is what ran.** The label and the command both come off
/// the Job's frozen workflow — never off the live Manifest, which is read at
/// Fleet start and can be edited under a Job that is already running.
#[tokio::test]
async fn a_steps_label_and_its_checks_command_are_the_ones_the_job_froze() {
    let home = TempDir::new();
    let job_id = {
        let fleet = a_fleet_holding(
            &home,
            changed(),
            gated_on("Implement the change", "cargo build --workspace --locked"),
            1,
        );
        let job = fleet
            .propose(a_proposal("fix the off-by-one"))
            .await
            .unwrap();
        job.id().clone()
    };

    let after = a_fleet_holding(
        &home,
        changed(),
        gated_on("Renamed since", "cargo build --some-other-way"),
        100,
    );
    let detail = api::Daemon::get_job(&after, ipc::JobId::from(&job_id))
        .await
        .expect("the Job reads");

    assert_eq!(detail.steps[0].label, "Implement the change");
    let checks = detail.steps[0].checks.as_ref().expect("Fleet can say");
    assert_eq!(checks[0].name.as_deref(), Some("build"));
    assert_eq!(
        checks[0].run.as_deref(),
        Some("cargo build --workspace --locked"),
        "the command the gate ran, not the one the file now holds"
    );
}

/// A Job that has a Drone says so, and still says so in a process that did not
/// spawn it — because the fold rebuilds the pointer from the log.
#[tokio::test]
async fn a_job_folds_its_drone_out_of_the_log() {
    let home = TempDir::new();
    let (job_id, drone) = {
        let fleet = a_fleet(&home, changed());
        let job = fleet
            .propose(a_proposal("fix the off-by-one"))
            .await
            .unwrap();
        worktree_directory(&home, job.id());
        let running = fleet.approve(job.id()).await.unwrap();
        assert_eq!(running.status(), JobStatus::Running);
        let drone = running
            .assigned_drone()
            .expect("a dispatched Job knows its Drone")
            .clone();
        (job.id().clone(), drone)
    };

    let after = a_fleet(&home, changed());
    assert_eq!(
        after.load(&job_id).await.unwrap().assigned_drone(),
        Some(&drone),
        "the column is not read back, so this came off the log"
    );

    // A Drone is held in memory by the Fleet that spawned it, so a restart
    // means it is gone and the record must stop naming it.
    after.reconcile().await.unwrap();
    let reconciled = after.load(&job_id).await.unwrap();
    assert_eq!(reconciled.status(), JobStatus::Escalated);
    assert_eq!(
        reconciled.assigned_drone(),
        None,
        "a Job nobody holds a process for claims no Drone"
    );
}

/// A second spawn onto **the same step** is refused rather than overwriting the
/// id, which is the only thing naming the first Drone's transcript. That is the
/// case `restart_step` reaches, so the refusal is still load-bearing.
#[test]
fn a_second_drone_cannot_be_put_on_a_step_that_already_has_one() {
    let at = |instant: &str| Timestamp::from_rfc3339(instant);
    let drone = |id: &str| DroneId::carried(Ulid::carried(id));
    let job = crate::tests::gate::running_job();
    let implement = StepId::new("implement");

    let first = job
        .drone_spawned(
            &implement,
            drone("01DRONEONE"),
            Actor::Fleet,
            at("2026-08-26T09:00:00Z"),
        )
        .expect("the first arrives");
    assert!(matches!(
        first.job.drone_spawned(
            &implement,
            drone("01DRONETWO"),
            Actor::Fleet,
            at("2026-08-26T09:01:00Z")
        ),
        Err(IllegalDroneMove::AlreadyAssigned { .. })
    ));
    assert!(
        first
            .job
            .drone_exited(&implement, Actor::Fleet, at("2026-08-26T09:02:00Z"))
            .is_ok(),
        "and the one that is there can leave"
    );
    assert!(
        matches!(
            job.drone_exited(&implement, Actor::Fleet, at("2026-08-26T09:02:00Z")),
            Err(IllegalDroneMove::NoneAssigned { .. })
        ),
        "a step that never had one cannot lose one"
    );
    assert!(
        matches!(
            job.drone_exited(
                &StepId::new("no-such-step"),
                Actor::Fleet,
                at("2026-08-26T09:02:00Z")
            ),
            Err(IllegalDroneMove::NoSuchStep { .. })
        ),
        "and a step the Job does not have is its own refusal, not a move that \
         changed nothing"
    );
}

/// **The refusal narrowed with the pointer; it did not go away.** A Drone on
/// the second step is admitted while the first step's row still names the one
/// that worked it — which is the whole reason the column moved, because that
/// name is the only thing that finds the first Drone's transcript afterwards.
#[test]
fn the_step_that_finished_keeps_naming_the_drone_that_worked_it() {
    let at = |instant: &str| Timestamp::from_rfc3339(instant);
    let drone = |id: &str| DroneId::carried(Ulid::carried(id));
    let job = crate::tests::gate::running_job();
    let implement = StepId::new("implement");
    let summarise = StepId::new("summarise");

    let first = drone("01DRONEONE");
    let worked = job
        .drone_spawned(
            &implement,
            first.clone(),
            Actor::Fleet,
            at("2026-08-26T09:00:00Z"),
        )
        .expect("the first arrives")
        .job;
    let left = worked
        .drone_exited(&implement, Actor::Fleet, at("2026-08-26T09:05:00Z"))
        .expect("it goes when its step's work is done")
        .job;
    assert_eq!(
        left.assigned_drone(),
        None,
        "no process is on the Job between one step and the next"
    );

    let second = drone("01DRONETWO");
    let next = left
        .drone_spawned(
            &summarise,
            second.clone(),
            Actor::Fleet,
            at("2026-08-26T09:06:00Z"),
        )
        .expect("a second step gets a second Drone")
        .job;
    assert_eq!(
        next.assigned_drone(),
        Some(&second),
        "the Job holds the Drone of the step being worked"
    );
    assert_eq!(
        next.step(&summarise).and_then(JobStep::assigned_drone),
        Some(&second)
    );
}

/// The whole point: an exit code says a Check failed, and the output says why.
#[tokio::test]
async fn a_failed_checks_output_is_readable_from_its_file_afterwards() {
    let home = TempDir::new();
    let fleet = a_fleet_holding(
        &home,
        changed(),
        testkit::resolved(&[Sketch {
            id: "implement",
            label: "Implement",
            evidence_type: Some("diff"),
            gates: &[Gate::Check {
                name: "suite",
                run: "/bin/sh -c 'echo the suite is unhappy 1>&2; exit 2'",
                expect_exit_code: 0,
                when: &[],
            }],
            judged_on: &[],
            scope: None,
            gaming: None,
        }]),
        1,
    );
    let job = fleet
        .propose(a_proposal("fix the off-by-one"))
        .await
        .unwrap();
    worktree_directory(&home, job.id());
    fleet.approve(job.id()).await.unwrap();
    fleet.submit_evidence(diff_evidence()).await.unwrap();

    let ruling = fleet.turn().await.unwrap().ruled.expect("the gate ruled");
    assert!(matches!(ruling, Ruling::Failed { .. }), "{ruling:?}");

    let ran = step_checks(&fleet, job.id()).await;
    let path = ran[0]
        .output_path
        .clone()
        .expect("a Manifest Check that ran has a file");
    assert_eq!(
        path,
        format!(".armada/checks/{}/implement.1.0.log", job.id().as_str()),
        "the whole of the row's key — job, step, attempt, ordinal — so a second \
         run of the step cannot overwrite the first run's output while `store` \
         keeps the first run's row"
    );

    let read = std::fs::read_to_string(home.path().join(&path)).expect("the file is there");
    assert!(
        read.contains("the suite is unhappy"),
        "what the Check printed outlived the process that ran it: {read}"
    );
    assert!(
        read.contains("--- stderr ---"),
        "and which stream it came out of is not left to be guessed"
    );
}

/// A step that advanced keeps its output too. A failure is not the only run
/// worth reading afterwards.
#[tokio::test]
async fn a_check_that_passed_keeps_its_output_and_a_built_in_has_none() {
    let home = TempDir::new();
    let fleet = a_fleet_holding(
        &home,
        changed(),
        testkit::resolved(&[
            Sketch {
                id: "implement",
                label: "Implement",
                evidence_type: Some("diff"),
                gates: &[
                    Gate::Check {
                        name: "suite",
                        run: "/bin/sh -c 'echo every test passed'",
                        expect_exit_code: 0,
                        when: &[],
                    },
                    Gate::DiffNonempty,
                ],
                judged_on: &[],
                scope: None,
                gaming: None,
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
        1,
    );
    let job = fleet
        .propose(a_proposal("fix the off-by-one"))
        .await
        .unwrap();
    worktree_directory(&home, job.id());
    fleet.approve(job.id()).await.unwrap();
    fleet.submit_evidence(diff_evidence()).await.unwrap();
    fleet.turn().await.unwrap();

    let reloaded = fleet.load(job.id()).await.unwrap();
    assert_eq!(
        reloaded.step(&StepId::new("implement")).unwrap().state(),
        StepState::Advanced
    );

    let ran = step_checks(&fleet, job.id()).await;
    let read = std::fs::read_to_string(
        home.path()
            .join(ran[0].output_path.as_ref().expect("the command's output")),
    )
    .expect("the file is there");
    assert!(read.contains("every test passed"), "{read}");
    assert_eq!(
        ran[1].output_path, None,
        "a built-in assertion runs no command, so there is no file and the row \
         says so by having no path"
    );
}

/// **What would have caught the drafting bug.** A Fleet holding more than one
/// workflow freezes the one the proposal actually named — not whichever one
/// this map happens to iterate to first.
#[tokio::test]
async fn a_job_proposed_against_the_second_workflow_freezes_that_ones_steps() {
    let home = TempDir::new();
    let fleet = a_fleet_holding_all(
        &home,
        changed(),
        vec![workflow_named("alpha"), workflow_named("beta")],
    );

    let job = fleet
        .propose(a_proposal_for("against beta", "beta"))
        .await
        .expect("beta is a workflow this Fleet holds");

    assert_eq!(job.steps().len(), 1);
    assert_eq!(
        job.workflow().steps()[0].id().as_str(),
        "only_in_beta",
        "the frozen step is beta's own, not alpha's — alpha sorts first in the map"
    );
}

/// The recorded Checks of the first step that has any.
async fn step_checks(
    fleet: &crate::daemon::Fleet<testkit::FakeHarness, testkit::FakeVcs, FakeWorkProduct>,
    job: &core_model::JobId,
) -> Vec<core_model::StepCheck> {
    fleet
        .store()
        .lock()
        .await
        .step_checks(job)
        .expect("the rows")
        .into_iter()
        .next()
        .expect("one step ran its checks")
        .1
}
