//! The pid crossing a restart, and the two things that must not be true of it.
//!
//! **A row that outlived its Drone is worse than no row**, because the whole
//! point of it is that a later Fleet trusts what it says: a stale pid probed
//! and found held names whatever process took the number. So the cases below
//! are about the row appearing at the spawn, disappearing at the departure, and
//! being replaced rather than doubled when a Job spawns again.

use core_model::{DroneId, JobId, StepId, Timestamp};

use crate::tests::{job_id, open, top_level, ulid, TempDir};
use crate::{DroneProcess, Store, WriteError};

fn a_job(store: &mut Store, id: &str) {
    let job = top_level(id);
    store
        .insert_job(&job, &crate::tests::created_at())
        .expect("the job is stored");
}

/// One process, with a reading of when it started that is deliberately not a
/// timestamp: what `ps -o lstart=` prints is a format belonging to somebody
/// else's tool, and this crate stores it without parsing it.
fn process(job: &str, drone: &str, pid: u32) -> DroneProcess {
    DroneProcess {
        job_id: JobId::carried(ulid(job)),
        step_id: StepId::new("implement"),
        drone_id: DroneId::carried(ulid(drone)),
        pid,
        started_at: String::from("Wed  3 Sep 01:14:07 2026"),
        spawned_at: Timestamp::from_rfc3339("2026-09-03T01:14:07.000Z"),
    }
}

/// **The whole claim.** What was written at the spawn is what a later process
/// reads back, including the half that is the identity rather than the number.
#[test]
fn the_process_a_drone_runs_as_survives_the_fleet_that_spawned_it() {
    let dir = TempDir::new();
    let mut store = open(&dir);
    a_job(&mut store, "01PROC000000000000000001");
    let recorded = process("01PROC000000000000000001", "01DRONE00000000000000001", 4096);
    store
        .record_drone_process(&recorded)
        .expect("the process is recorded");

    let read = store
        .drone_process(&job_id("01PROC000000000000000001"))
        .expect("the process is read");
    assert_eq!(
        read,
        Some(recorded),
        "the pid and its start time both survive"
    );
}

/// A Job nothing spawned for has no process, which is the answer that makes
/// reconciliation fall back rather than guess.
#[test]
fn a_job_with_no_drone_has_no_process_recorded() {
    let dir = TempDir::new();
    let mut store = open(&dir);
    a_job(&mut store, "01PROC000000000000000002");
    assert_eq!(
        store
            .drone_process(&job_id("01PROC000000000000000002"))
            .expect("the read succeeds"),
        None,
        "nothing was spawned, so nothing can be adopted"
    );
}

/// **The stale row this table must not accumulate.** A Job spawns, its Drone
/// leaves, and it spawns again — one row throughout, naming the process that is
/// there now.
#[test]
fn a_second_spawn_replaces_the_process_rather_than_adding_one() {
    let dir = TempDir::new();
    let mut store = open(&dir);
    a_job(&mut store, "01PROC000000000000000003");
    let job = job_id("01PROC000000000000000003");
    store
        .record_drone_process(&process(
            "01PROC000000000000000003",
            "01DRONE00000000000000001",
            4096,
        ))
        .expect("the first process is recorded");
    store
        .record_drone_process(&process(
            "01PROC000000000000000003",
            "01DRONE00000000000000002",
            8192,
        ))
        .expect("the second process is recorded");

    let read = store.drone_process(&job).expect("the process is read");
    assert_eq!(
        read.map(|held| (held.pid, held.drone_id)),
        Some((8192, DroneId::carried(ulid("01DRONE00000000000000002")))),
        "the row names the process that is there now"
    );
    assert_eq!(
        store
            .drone_processes()
            .expect("every process is read")
            .len(),
        1,
        "a Job holds one Drone, so it holds one row"
    );
}

/// The departure. **`Ok` whether or not there was a row**, because every road
/// that reaches it is a road where the process has gone.
#[test]
fn a_drone_that_left_leaves_no_process_behind() {
    let dir = TempDir::new();
    let mut store = open(&dir);
    a_job(&mut store, "01PROC000000000000000004");
    let job = job_id("01PROC000000000000000004");
    store
        .record_drone_process(&process(
            "01PROC000000000000000004",
            "01DRONE00000000000000001",
            4096,
        ))
        .expect("the process is recorded");
    store
        .forget_drone_process(&job)
        .expect("the process is forgotten");
    assert_eq!(
        store.drone_process(&job).expect("the read succeeds"),
        None,
        "the pointer and the pid go together"
    );
    store
        .forget_drone_process(&job)
        .expect("forgetting nothing is not a fault");
}

/// The reconciliation's own read: every process in the file, in one pass.
#[test]
fn every_recorded_process_comes_back_in_one_read() {
    let dir = TempDir::new();
    let mut store = open(&dir);
    for (n, pid) in [4096u32, 8192, 12288].into_iter().enumerate() {
        let id = format!("01PROC00000000000000010{n}");
        a_job(&mut store, &id);
        store
            .record_drone_process(&process(&id, &format!("01DRONE0000000000000010{n}"), pid))
            .expect("the process is recorded");
    }
    let read = store.drone_processes().expect("every process is read");
    assert_eq!(
        read.iter().map(|held| held.pid).collect::<Vec<_>>(),
        vec![4096, 8192, 12288],
        "one row per Job, in Job order"
    );
}

/// A process against a Job that does not exist is refused **by name**, so a
/// write racing `forget_job` says which Job rather than reporting a constraint.
#[test]
fn a_process_against_no_job_is_refused_by_name() {
    let dir = TempDir::new();
    let mut store = open(&dir);
    let refused = store
        .record_drone_process(&process(
            "01PROC000000000000000009",
            "01DRONE00000000000000001",
            4096,
        ))
        .expect_err("there is no such Job");
    assert!(
        matches!(refused, WriteError::NoSuchJob { ref job_id } if job_id.as_str() == "01PROC000000000000000009"),
        "the refusal names the Job: {refused:?}"
    );
}

/// **A pid the platform cannot express never reaches a probe**, because the
/// database refuses to hold one. `CHECK (pid > 0)` is the guard: pid zero names
/// the caller's own process group to a group-directed call, which is the number
/// that would turn one Drone's ending into a fleet-wide kill.
#[test]
fn a_pid_of_zero_is_refused_by_the_database_itself() {
    let dir = TempDir::new();
    let mut store = open(&dir);
    a_job(&mut store, "01PROC000000000000000005");
    let refused = store
        .record_drone_process(&process(
            "01PROC000000000000000005",
            "01DRONE00000000000000001",
            0,
        ))
        .expect_err("zero is not a pid");
    assert!(
        matches!(refused, WriteError::Database(_)),
        "a refused pid is the database refusing a value, not a missing Job: {refused:?}"
    );
}

/// Forgetting the Job takes the process row with it, through the catalog walk
/// rather than through a list somebody remembered to extend.
#[test]
fn forgetting_a_job_forgets_the_process_it_recorded() {
    let dir = TempDir::new();
    let mut store = open(&dir);
    a_job(&mut store, "01PROC000000000000000006");
    let job = job_id("01PROC000000000000000006");
    store
        .record_drone_process(&process(
            "01PROC000000000000000006",
            "01DRONE00000000000000001",
            4096,
        ))
        .expect("the process is recorded");
    let forgotten = store.forget_job(&job).expect("the job is forgotten");
    assert_eq!(
        forgotten.drone_process, 1,
        "the row is counted by name rather than in the lump"
    );
    assert_eq!(forgotten.other, 0, "no table went uncounted");
}
