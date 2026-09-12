//! What a table rebuild has to carry, checked on the one migration that does
//! one.
//!
//! **V50 rebuilds `port_claims` rather than altering it.** SQLite cannot widen
//! a `CHECK` in place, and [`V45`](crate::ports::V45)'s spells "exactly one of
//! a Job or the main checkout" — so admitting Fleet's own listener as a third
//! claimant means building the table beside the old one, carrying every row
//! across and dropping the original.
//!
//! **A rebuild that quietly lost rows would be a Job losing the port a Command
//! is already bound to**, and the main checkout losing the span a server is
//! running on. That is a different question from the ones
//! `crate::tests::migrate` asks — which are about what a *column* means to a
//! row written before it — so it is asked here.

use rusqlite::Connection;

use super::migrate::recorded_version;
use crate::migrations::{MIGRATIONS, SCHEMA_VERSION_KEY};
use crate::tests::{created_at, job_id, TempDir};
use crate::{PortClaim, PortClaimant, Store, KNOWN_SCHEMA_VERSION};

/// A file at version 49 — the schema before Fleet's own listener became a
/// claimant — carrying one of each claim `port_claims` could then hold. The
/// inserts are raw SQL because the column the current build writes does not
/// exist on this file yet.
fn version_forty_nine(dir: &TempDir, job: &str) {
    let conn = Connection::open(dir.db()).expect("a file to put version 49 in");
    for migration in &MIGRATIONS[..49] {
        conn.execute_batch(migration).expect("a migration");
    }
    conn.execute(
        "INSERT INTO armada_meta (key, value) VALUES (?1, '49')",
        (SCHEMA_VERSION_KEY,),
    )
    .expect("recorded as version 49");
    conn.execute(
        "INSERT INTO jobs (
             job_id, title, status, workflow_id, owner_manifest_id, origin, urgency,
             atomic, model, acceptance_criteria, dependencies, facts, scope_revisions,
             write_targets_known, created_at
         ) VALUES (?1, 'a job holding a port span', 'running', '01WORKFLOW',
                   '01OWNERMANIFEST', 'manual', 'normal', 0, 'a-model-name', '[]', '[]',
                   '', '[]', 0, '2026-08-26T09:00:00.000Z')",
        (job,),
    )
    .expect("a Job as version 49 wrote them");
    conn.execute(
        "INSERT INTO port_claims (job_id, main_checkout, base, width, claimed_at)
         VALUES (?1, NULL, 41000, 8, '2026-08-26T09:00:00.000Z')",
        (job,),
    )
    .expect("the Job's span, claimed before the rebuild");
    conn.execute(
        "INSERT INTO port_claims (job_id, main_checkout, base, width, claimed_at)
         VALUES (NULL, 1, 41008, 8, '2026-08-26T09:00:00.000Z')",
        (),
    )
    .expect("the main checkout's span, claimed before the rebuild");
}

/// **The rebuild carries every row across**, at the same base and the same
/// width, and the claimant each row named is still the claimant it reads back
/// as.
#[test]
fn a_store_at_version_forty_nine_keeps_every_claim_through_the_rebuild() {
    let dir = TempDir::new();
    version_forty_nine(&dir, "01PORTREBUILD00000000001");

    let mut store = Store::open(&dir.db()).expect("a version 49 file opens and is migrated");
    assert_eq!(
        recorded_version(&store),
        KNOWN_SCHEMA_VERSION.to_string(),
        "migrated all the way, not stopped at 49"
    );

    let job = job_id("01PORTREBUILD00000000001");
    let carried = store
        .port_span_for_job(&job)
        .expect("the read succeeds")
        .expect("the Job's claim survived the rebuild");
    assert_eq!((carried.base, carried.width), (41_000, 8));
    let main = store
        .port_span_for_main_checkout()
        .expect("the read succeeds")
        .expect("the main checkout's claim survived the rebuild");
    assert_eq!((main.base, main.width), (41_008, 8));

    // And the third claimant the rebuild exists for, which the old `CHECK`
    // could not have held.
    store
        .claim_port_span(&PortClaim {
            claimant: PortClaimant::FleetListener,
            base: 41_016,
            width: 1,
            claimed_at: created_at(),
        })
        .expect("Fleet's own listener can claim once the rebuild has run");
    assert_eq!(store.every_port_claim().expect("read").len(), 3);
}
