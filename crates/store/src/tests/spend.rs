//! What a Job's Drones spent adds up across Drones, and does not add up twice.
//!
//! **The double count is the defect this table's key exists to prevent.** A
//! Drone's exit is folded at two sites in `fleet`, and a per-Job counter
//! incremented at both would bill one Drone twice. Every test below is either
//! about the sum being right across Drones, or about it not moving when one
//! Drone is recorded again.
//!
//! The last few are about the other half of the comparison — what one Job *may*
//! spend, which is a nullable column where **null and zero must never collapse
//! into one answer**: null defers to `armada.yml` and then to the composition
//! root, and zero is a Job that starts nothing.

use crate::tests::{created_at, job_id, open, top_level, TempDir};
use crate::{DroneSpend, Spend, Store, WriteError};

fn a_job(store: &mut Store, id: &str) {
    let job = top_level(id);
    store
        .insert_job(&job, &crate::tests::created_at())
        .expect("the job is stored");
}

fn drone(value: &str) -> core_model::DroneId {
    core_model::DroneId::carried(crate::tests::ulid(value))
}

fn spent(cost_micros: u64, turns: u64, ran_ms: u64) -> DroneSpend {
    DroneSpend {
        cost_micros,
        turns,
        ran_ms,
    }
}

/// A Job nothing has run for reads zero of everything, and **no Drones** —
/// which is the field that tells it apart from a Job whose Drone was free.
#[test]
fn a_job_that_has_not_run_has_spent_nothing() {
    let dir = TempDir::new();
    let mut store = open(&dir);
    a_job(&mut store, "01SPEND00000000000000001");
    let read = store
        .spend_for(&job_id("01SPEND00000000000000001"))
        .expect("the spend is read");
    assert_eq!(
        read,
        Spend::default(),
        "nothing has run, so nothing is spent"
    );
}

/// **The join the cap needs.** Four Drones of one Job, none of which knows
/// about the others, and the Job's figure is the sum of all four.
#[test]
fn four_drones_of_one_job_add_up_to_the_job() {
    let dir = TempDir::new();
    let mut store = open(&dir);
    a_job(&mut store, "01SPEND00000000000000002");
    let job = job_id("01SPEND00000000000000002");
    for (n, cost) in [63_366u64, 87_344, 146_473, 99_000].into_iter().enumerate() {
        store
            .record_drone_spend(
                &job,
                &drone(&format!("01DRONE0000000000000000{n}")),
                &spent(cost, 7, 22_000),
            )
            .expect("the spend is recorded");
    }
    let read = store.spend_for(&job).expect("the spend is read");
    assert_eq!(
        read,
        Spend {
            cost_micros: 396_183,
            turns: 28,
            ran_ms: 88_000,
            drones: 4,
        },
        "the Job's figure is every Drone's, added up"
    );
}

/// **Recording one Drone twice is recording it once.** Both `dispatch::reap`
/// and `boundary::stood_down` can fold the same Drone's stream, and the key is
/// what makes calling both harmless rather than a convention somebody has to
/// remember.
#[test]
fn recording_one_drone_twice_does_not_bill_it_twice() {
    let dir = TempDir::new();
    let mut store = open(&dir);
    a_job(&mut store, "01SPEND00000000000000003");
    let job = job_id("01SPEND00000000000000003");
    let one = drone("01DRONE00000000000000001");
    store
        .record_drone_spend(&job, &one, &spent(120_000, 9, 40_000))
        .expect("the spend is recorded");
    store
        .record_drone_spend(&job, &one, &spent(120_000, 9, 40_000))
        .expect("the same spend is recorded again");
    let read = store.spend_for(&job).expect("the spend is read");
    assert_eq!(
        read,
        Spend {
            cost_micros: 120_000,
            turns: 9,
            ran_ms: 40_000,
            drones: 1,
        },
        "one Drone, recorded twice, is one Drone's spend"
    );
}

/// The later figure wins, because the later fold read further down the stream.
/// A Drone reaped at a turn boundary and then stood down at the step boundary
/// is the ordinary case, and the second reading is the complete one.
#[test]
fn a_second_reading_of_one_drone_replaces_the_first() {
    let dir = TempDir::new();
    let mut store = open(&dir);
    a_job(&mut store, "01SPEND00000000000000004");
    let job = job_id("01SPEND00000000000000004");
    let one = drone("01DRONE00000000000000002");
    store
        .record_drone_spend(&job, &one, &spent(50_000, 3, 9_000))
        .expect("the first reading is recorded");
    store
        .record_drone_spend(&job, &one, &spent(121_961, 5, 11_786))
        .expect("the second reading is recorded");
    let read = store.spend_for(&job).expect("the spend is read");
    assert_eq!(read.cost_micros, 121_961, "the later reading stands");
    assert_eq!(read.turns, 5, "and so does its turn count");
    assert_eq!(read.drones, 1, "it is still one Drone");
}

/// Two Jobs' Drones do not reach each other. A cap is per Job, so a sum that
/// leaked across them would refuse a Job for somebody else's spending.
#[test]
fn one_jobs_spend_is_not_anothers() {
    let dir = TempDir::new();
    let mut store = open(&dir);
    a_job(&mut store, "01SPEND00000000000000005");
    a_job(&mut store, "01SPEND00000000000000006");
    store
        .record_drone_spend(
            &job_id("01SPEND00000000000000005"),
            &drone("01DRONE00000000000000003"),
            &spent(500_000, 40, 600_000),
        )
        .expect("the spend is recorded");
    let other = store
        .spend_for(&job_id("01SPEND00000000000000006"))
        .expect("the spend is read");
    assert_eq!(
        other,
        Spend::default(),
        "the other Job has spent nothing of its own"
    );
}

/// A spend recorded against a Job that is not there is refused by name.
///
/// **`NoSuchJob` and not a database fault**, which is the same distinction
/// `record_delivery` makes: a caller writing against a forgotten Job asked
/// about something that is not there rather than meeting a broken store.
#[test]
fn a_spend_against_no_job_is_refused_by_name() {
    let dir = TempDir::new();
    let mut store = open(&dir);
    let why = store
        .record_drone_spend(
            &job_id("01SPEND00000000000000007"),
            &drone("01DRONE00000000000000004"),
            &spent(1, 1, 1),
        )
        .expect_err("there is no such Job");
    assert!(
        matches!(why, WriteError::NoSuchJob { .. }),
        "a missing Job is named rather than reported as a fault: {why:?}"
    );
}

/// A cap set on a Job survives the process that set it, and the value is the
/// one that was written rather than the machine's.
#[test]
fn a_cap_raised_on_a_job_survives_a_reopen() {
    let dir = TempDir::new();
    let stored = top_level("01CAP0000000000000000001");
    let mut store = open(&dir);
    store.insert_job(&stored, &created_at()).expect("stored");
    assert_eq!(
        stored.cost_cap_micros(),
        None,
        "a Job is created taking whatever the tiers above it say"
    );

    let raised = stored.cost_capped(Some(10_000_000));
    store.record_cost_cap(&raised).expect("the cap is written");
    drop(store);

    let mut reopened = open(&dir);
    let loaded = reopened
        .load_job(&job_id("01CAP0000000000000000001"))
        .expect("loads");
    assert_eq!(loaded.cost_cap_micros(), Some(10_000_000));
    assert_eq!(loaded, raised, "and nothing else about the Job moved");

    // Clearing is the same write with nothing in it, which is why there is one
    // method rather than a setter and a clearer — a person taking a cap off is
    // asking for the repository's number back.
    reopened
        .record_cost_cap(&loaded.cost_capped(None))
        .expect("the cap is cleared");
    drop(reopened);
    assert_eq!(
        open(&dir)
            .load_job(&job_id("01CAP0000000000000000001"))
            .expect("loads"),
        stored,
        "cleared, and the record is back where it started"
    );
}

/// **A cap of zero is not an absence**, and this is the assertion that keeps
/// them apart across the file. A Job capped at zero starts nothing; a Job with
/// no cap starts whatever the repository and the machine allow.
#[test]
fn a_cap_of_zero_reads_back_as_zero_and_not_as_unset() {
    let dir = TempDir::new();
    let mut store = open(&dir);
    let held = top_level("01CAP0000000000000000002").cost_capped(Some(0));
    store.insert_job(&held, &created_at()).expect("stored");
    drop(store);

    assert_eq!(
        open(&dir)
            .load_job(&job_id("01CAP0000000000000000002"))
            .expect("loads")
            .cost_cap_micros(),
        Some(0)
    );
}

/// A Job written before version 32 reads as having no cap of its own, which is
/// the same sentence as the one it ran under: the machine's number.
#[test]
fn a_job_from_before_the_column_existed_reads_as_uncapped() {
    let dir = TempDir::new();
    let mut store = open(&dir);
    store
        .insert_job(&top_level("01CAP0000000000000000003"), &created_at())
        .expect("stored");
    store
        .conn
        .execute(
            "UPDATE jobs SET cost_cap_micros = NULL WHERE job_id = ?1",
            (job_id("01CAP0000000000000000003").as_str(),),
        )
        .expect("the column is nulled, as an older row holds it");
    assert_eq!(
        store
            .load_job(&job_id("01CAP0000000000000000003"))
            .expect("loads")
            .cost_cap_micros(),
        None
    );
}

/// A negative cap is refused by the schema rather than widened into a ceiling
/// nothing could reach. `INTEGER` is signed and the field is not, so a `-1`
/// left to `as u64` would arrive as eighteen quintillion dollars.
#[test]
fn a_negative_cap_is_refused_by_the_row_that_would_hold_it() {
    let dir = TempDir::new();
    let mut store = open(&dir);
    store
        .insert_job(&top_level("01CAP0000000000000000004"), &created_at())
        .expect("stored");
    let refused = store.conn.execute(
        "UPDATE jobs SET cost_cap_micros = -1 WHERE job_id = ?1",
        (job_id("01CAP0000000000000000004").as_str(),),
    );
    assert!(
        refused.is_err(),
        "the trigger refuses it: {refused:?}, and nothing in this crate can write one anyway"
    );
}

/// A cap written against a Job that is not there is refused by name, for
/// `record_drone_spend`'s reason.
#[test]
fn a_cap_against_no_job_is_refused_by_name() {
    let dir = TempDir::new();
    let mut store = open(&dir);
    let why = store
        .record_cost_cap(&top_level("01CAP0000000000000000005").cost_capped(Some(1)))
        .expect_err("there is no such Job");
    assert!(
        matches!(why, WriteError::NoSuchJob { .. }),
        "a missing Job is named rather than reported as a fault: {why:?}"
    );
}

/// Forgetting a Job takes its spend rows with it, because the table points at
/// `jobs` and `forget_job` asks the file which tables do.
#[test]
fn forgetting_a_job_forgets_what_it_spent() {
    let dir = TempDir::new();
    let mut store = open(&dir);
    a_job(&mut store, "01SPEND00000000000000008");
    let job = job_id("01SPEND00000000000000008");
    store
        .record_drone_spend(&job, &drone("01DRONE00000000000000005"), &spent(9, 9, 9))
        .expect("the spend is recorded");
    let held: Vec<String> = store
        .conn
        .prepare(
            "SELECT m.name FROM sqlite_master AS m \
             JOIN pragma_foreign_key_list(m.name) AS fk \
             WHERE m.type = 'table' AND fk.\"table\" = 'jobs' AND m.name = 'job_drone_spend'",
        )
        .expect("the catalog is readable")
        .query_map([], |row| row.get(0))
        .expect("the catalog is readable")
        .collect::<Result<_, _>>()
        .expect("the catalog is readable");
    assert_eq!(
        held,
        vec!["job_drone_spend".to_string()],
        "the table points at jobs, so forget_job's catalog finds it"
    );
}
