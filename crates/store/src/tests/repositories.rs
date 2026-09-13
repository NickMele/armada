//! The repositories a Fleet serves, remembered, and V57's rebuild of
//! `port_claims` around them.

use rusqlite::Connection;

use super::migrate::recorded_version;
use crate::migrations::{MIGRATIONS, SCHEMA_VERSION_KEY};
use crate::tests::{at, job_id, open, TempDir};
use crate::{PortClaimant, Store, KNOWN_SCHEMA_VERSION};

#[test]
fn a_repository_is_remembered_once_in_the_order_it_was_added() {
    let dir = TempDir::new();
    let mut store = open(&dir);
    assert!(store.remembered_repositories().expect("reads").is_empty());
    for (root, when) in [
        ("/repos/storefront", "2026-09-13T09:00:00.000Z"),
        ("/repos/mailer", "2026-09-13T09:01:00.000Z"),
        ("/repos/storefront", "2026-09-13T09:02:00.000Z"),
    ] {
        store
            .remember_repository(root, &at(when))
            .expect("remembered");
    }
    drop(store);
    let reopened = Store::open(&dir.db()).expect("reopens");
    assert_eq!(
        reopened.remembered_repositories().expect("reads"),
        ["/repos/storefront", "/repos/mailer"]
    );
}

/// A file at version 56 holding one claim of each kind.
fn version_fifty_six(dir: &TempDir, job: &str) {
    let conn = Connection::open(dir.db()).expect("a file to put version 56 in");
    for migration in &MIGRATIONS[..56] {
        conn.execute_batch(migration).expect("a migration");
    }
    conn.execute(
        "INSERT INTO armada_meta (key, value) VALUES (?1, '56')",
        (SCHEMA_VERSION_KEY,),
    )
    .expect("recorded as version 56");
    conn.execute(
        "INSERT INTO jobs (
             job_id, title, status, workflow_id, owner_manifest_id, origin, urgency,
             atomic, model, acceptance_criteria, dependencies, facts, scope_revisions,
             write_targets_known, created_at
         ) VALUES (?1, 'a job holding a port span', 'running', '01WORKFLOW',
                   '01OWNERMANIFEST', 'manual', 'normal', 0, 'a-model-name', '[]', '[]',
                   '', '[]', 0, '2026-09-13T09:00:00.000Z')",
        (job,),
    )
    .expect("a Job as version 56 wrote them");
    conn.execute_batch(&format!(
        "INSERT INTO port_claims (job_id, main_checkout, fleet_listener, base, width, claimed_at)
         VALUES ('{job}', NULL, NULL, 41000, 8, '2026-09-13T09:00:00.000Z'),
                (NULL, 1, NULL, 41008, 8, '2026-09-13T09:00:00.000Z'),
                (NULL, NULL, 1, 41016, 1, '2026-09-13T09:00:00.000Z');"
    ))
    .expect("one claim of each kind");
}

/// **V57 carries the Job's and the listener's claims**, releases the main
/// checkout's that named no repository, and remembers repositories after.
#[test]
fn a_store_at_version_fifty_six_is_rebuilt_around_repositories() {
    let dir = TempDir::new();
    version_fifty_six(&dir, "01PORTREPOSITORIES000001");
    let mut store = Store::open(&dir.db()).expect("migrated");
    assert_eq!(recorded_version(&store), KNOWN_SCHEMA_VERSION.to_string());

    let every = store.every_port_claim().expect("read");
    let held: Vec<(PortClaimant, u16)> = every.into_iter().map(|c| (c.claimant, c.base)).collect();
    let job = job_id("01PORTREPOSITORIES000001");
    assert_eq!(
        held,
        [
            (PortClaimant::Job(job), 41_000),
            (PortClaimant::FleetListener, 41_016)
        ]
    );
    store
        .remember_repository("/repos/mailer", &at("2026-09-13T09:05:00.000Z"))
        .expect("the new table is there");
    assert_eq!(
        store.remembered_repositories().expect("reads"),
        ["/repos/mailer"]
    );
}
