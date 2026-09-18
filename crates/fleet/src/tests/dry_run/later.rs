//! The report as a later turn, and a run no call holds any more. #1020.
//!
//! The call used to be held open for the whole run, and a client that gave up
//! dropped the run with it: the slot stayed `checking`, every later ask was
//! refused "already running", and the clocks stayed suspended.

use std::net::SocketAddr;
use std::os::unix::fs::PermissionsExt;
use std::path::PathBuf;
use std::sync::Arc;
use std::time::{Duration, Instant};

use core_model::{DroneId, JobId};
use tokio::io::AsyncWriteExt;

use crate::tests::dry_run::{
    a_fleet_checking, call, one_step, post, router, started, submit, text_of, the_one_drone,
    told_checks, wait_until_checking, Fixture, Held,
};
use crate::tests::tmp::TempDir;
use crate::tests::tools::checked_by_the_one;

async fn checking_now(fleet: &Fixture) -> bool {
    fleet
        .the_only_slot()
        .await
        .lock()
        .await
        .as_ref()
        .is_some_and(|at_work| at_work.is_checking())
}

/// The reports this Drone has been told, once there is at least one.
async fn told_once(fleet: &Fixture, home: &TempDir, job: &JobId, drone: &DroneId) -> Vec<String> {
    for _ in 0..2_000 {
        let told = told_checks(fleet, home, job, drone).await;
        if !told.is_empty() {
            return told;
        }
        tokio::time::sleep(Duration::from_millis(5)).await;
    }
    panic!("no report arrived as a later turn");
}

/// A Check that sleeps and then leaves a marker, so a stopped one leaves none.
fn slow_then_marks(home: &TempDir) -> (String, PathBuf) {
    let marker = home.path().join("marked");
    let script = home.path().join("slow-check");
    std::fs::write(
        &script,
        format!(
            "#!/bin/sh\n/bin/sleep 2\n/usr/bin/touch {}\n",
            marker.display()
        ),
    )
    .expect("the script writes");
    std::fs::set_permissions(&script, std::fs::Permissions::from_mode(0o755))
        .expect("the script can run");
    (script.display().to_string(), marker)
}

/// **Answered before the Checks finish, and the report follows as a turn.**
/// How long the run takes no longer reaches the call, so a run past the
/// client's five minutes is this same case.
#[tokio::test]
async fn the_call_answers_before_the_checks_finish_and_the_report_follows() {
    let home = TempDir::new();
    let fleet = Arc::new(a_fleet_checking(
        &home,
        one_step("/bin/sleep 3"),
        Arc::new(Held::started()),
        3,
    ));
    let app = router(&fleet);
    started(&fleet, &home).await;
    let (job, drone) = the_one_drone(&fleet).await.expect("a Drone at work");

    let asked = Instant::now();
    let body = post(&app, &call(false)).await;
    assert!(
        asked.elapsed() < Duration::from_secs(3),
        "the call waited for the run: {:?}",
        asked.elapsed()
    );
    assert!(
        checking_now(&fleet).await,
        "the run was still going when the call answered"
    );
    assert!(!body.contains("\"isError\":true"), "{body}");
    assert!(text_of(&body).contains("later turn"), "{body}");

    let told = told_once(&fleet, &home, &job, &drone).await;
    assert_eq!(told.len(), 1, "one run, one report: {told:?}");
    for part in [
        "THE CHECKS YOU ASKED FOR",
        "suite",
        "PASSED",
        "diff_nonempty",
        "implement.1.dry.0.log",
        "not a verdict",
    ] {
        assert!(
            told[0].contains(part),
            "the turn has no {part}: {}",
            told[0]
        );
    }
}

/// The turn carries the whole report, exactly as the held-open answer did.
#[tokio::test]
async fn the_turn_carries_the_whole_report() {
    let home = TempDir::new();
    let fleet = Arc::new(a_fleet_checking(
        &home,
        one_step("/usr/bin/false"),
        Arc::new(Held::started()),
        3,
    ));
    started(&fleet, &home).await;
    let (job, drone) = the_one_drone(&fleet).await.expect("a Drone at work");

    let report = checked_by_the_one(&fleet).await.expect("a report");
    let whole = ipc::encode(&report.to_string()).expect("the report encodes");
    let whole = &whole[1..whole.len() - 1];
    let told = told_once(&fleet, &home, &job, &drone).await;
    assert!(
        told[0].contains(whole),
        "the turn left some of the report out:\n{}\n---\n{whole}",
        told[0]
    );
}

/// **A call whose client hangs up mid-run.** Before #1020 this left the slot
/// `checking` for good; proven against that code with the same hang-up.
#[tokio::test]
async fn a_run_whose_call_is_abandoned_still_ends() {
    let home = TempDir::new();
    let clock = Arc::new(Held::started());
    let fleet = Arc::new(a_fleet_checking(
        &home,
        one_step("/bin/sleep 2"),
        Arc::clone(&clock),
        3,
    ));
    started(&fleet, &home).await;
    let listener = tokio::net::TcpListener::bind("127.0.0.1:0")
        .await
        .expect("a port");
    let at = listener.local_addr().expect("its address");
    let app = router(&fleet);
    tokio::spawn(async move {
        let served = app.into_make_service_with_connect_info::<SocketAddr>();
        let _ = axum::serve(listener, served).await;
    });

    let body = call(false);
    let mut client = tokio::net::TcpStream::connect(at).await.expect("connects");
    let request = format!(
        "POST {} HTTP/1.1\r\nhost: fleet\r\ncontent-type: application/json\r\n\
         content-length: {}\r\n\r\n{body}",
        api::MCP_PATH,
        body.len()
    );
    client
        .write_all(request.as_bytes())
        .await
        .expect("the call is sent");
    wait_until_checking(&fleet).await;
    drop(client);

    let mut ended = false;
    for _ in 0..400 {
        if !checking_now(&fleet).await {
            ended = true;
            break;
        }
        tokio::time::sleep(Duration::from_millis(25)).await;
    }
    assert!(ended, "the run the abandoned call started never ended");

    // The clocks count again: time pushed now lands on the step's wall clock.
    let running_for = || async {
        let now = fleet.now();
        let slot = fleet.the_only_slot().await;
        let held = slot.lock().await;
        held.as_ref().expect("a Drone at work").running_for(&now)
    };
    let before = running_for().await;
    clock.on(1_000);
    let after = running_for().await;
    assert!(
        after >= before + Duration::from_secs(1_000),
        "the clocks are still suspended: {before:?}, then {after:?}"
    );

    let again = checked_by_the_one(&fleet).await;
    assert!(again.is_ok(), "the next ask was refused: {again:?}");
}

/// **A submission ends the run**: its Checks stop and no report follows.
#[tokio::test]
async fn a_submission_mid_run_stops_the_checks_and_nothing_is_told() {
    let home = TempDir::new();
    let (check, marker) = slow_then_marks(&home);
    let fleet = Arc::new(a_fleet_checking(
        &home,
        one_step(&check),
        Arc::new(Held::started()),
        3,
    ));
    let app = router(&fleet);
    started(&fleet, &home).await;
    let (job, drone) = the_one_drone(&fleet).await.expect("a Drone at work");

    let underway = fleet
        .run_checks(&job, ipc::mcp::ChecksAsk::everything(false))
        .await
        .expect("the run starts");
    wait_until_checking(&fleet).await;
    submit(&app).await;
    assert_eq!(fleet.evidence_waiting(), 1, "the submission was taken");
    assert!(
        !checking_now(&fleet).await,
        "the submission left the mark on"
    );

    assert!(
        underway.finished().await.is_none(),
        "a run the step ended was reported"
    );
    tokio::time::sleep(Duration::from_secs(3)).await;
    assert!(!marker.exists(), "the Check went on after the step ended");
    assert!(told_checks(&fleet, &home, &job, &drone).await.is_empty());
}

/// **A kill ends the run the same way**, through the slot it takes away.
#[tokio::test]
async fn a_drone_killed_mid_run_stops_the_checks_and_nothing_is_told() {
    let home = TempDir::new();
    let (check, marker) = slow_then_marks(&home);
    let fleet = Arc::new(a_fleet_checking(
        &home,
        one_step(&check),
        Arc::new(Held::started()),
        3,
    ));
    started(&fleet, &home).await;
    let (job, drone) = the_one_drone(&fleet).await.expect("a Drone at work");

    let underway = fleet
        .run_checks(&job, ipc::mcp::ChecksAsk::everything(false))
        .await
        .expect("the run starts");
    wait_until_checking(&fleet).await;
    fleet.kill_drone(&job).await.expect("the Drone is killed");

    assert!(
        underway.finished().await.is_none(),
        "a run whose Drone was killed was reported"
    );
    tokio::time::sleep(Duration::from_secs(3)).await;
    assert!(
        !marker.exists(),
        "the Check went on after the Drone was killed"
    );
    assert!(told_checks(&fleet, &home, &job, &drone).await.is_empty());
}
