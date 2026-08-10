//! `char.db` under real contention.
//!
//! The filesystem and SQLite are deliberately **not** faked
//! (`ARCHITECTURE.md` §1.1): char depends on real transaction semantics for
//! port claims and lease acquisition, and a fake gives you a green test over
//! your own fake's concurrency model — in the one area where corruption under
//! concurrent claims is a named live risk. Two threads against a real database
//! in a `TempDir` is both more faithful and less code.

use charkit_adapters::db::{AcquireOutcome, ClaimOutcome, Db};
use charkit_core::id::WorkspaceId;
use charkit_core::lease::{LeaseId, LeaseKind};
use std::path::Path;
use std::sync::{Arc, Barrier};

/// **`BEGIN IMMEDIATE` is mandatory, and `busy_timeout` cannot substitute for
/// it** — re-run rather than re-trusted, because the failure it prevents is
/// nondeterministic and only appears under the contention the lease design
/// exists to handle.
///
/// A DEFERRED transaction that reads and *then* writes fails the moment another
/// writer committed in between, in microseconds, with `SQLITE_BUSY_SNAPSHOT`
/// (517). `busy_timeout` never applies: the reader's snapshot is already stale,
/// so no amount of waiting can rescue it.
#[test]
fn a_deferred_read_then_write_fails_where_an_immediate_one_succeeds() {
    use rusqlite::{Connection, TransactionBehavior};

    let home = tempfile::tempdir().unwrap();
    let path = home.path().join("contended.db");

    let setup = Connection::open(&path).unwrap();
    setup.pragma_update(None, "journal_mode", "WAL").unwrap();
    setup
        .execute("CREATE TABLE counter (n INTEGER NOT NULL)", [])
        .unwrap();
    setup.execute("INSERT INTO counter VALUES (0)", []).unwrap();

    let open = |path: &Path| {
        let conn = Connection::open(path).unwrap();
        conn.pragma_update(None, "busy_timeout", 5_000).unwrap();
        conn
    };

    // The obvious lease code: read the row, decide, write it back.
    let mut reader = open(&path);
    let deferred = reader
        .transaction_with_behavior(TransactionBehavior::Deferred)
        .unwrap();
    let _: i64 = deferred
        .query_row("SELECT n FROM counter", [], |row| row.get(0))
        .unwrap();

    // Another writer commits in between.
    let interloper = open(&path);
    interloper
        .execute("UPDATE counter SET n = n + 1", [])
        .unwrap();

    let started = std::time::Instant::now();
    let error = deferred
        .execute("UPDATE counter SET n = n + 1", [])
        .expect_err("the stale snapshot must not be allowed to write");
    let elapsed = started.elapsed();

    let extended = match &error {
        rusqlite::Error::SqliteFailure(inner, _) => inner.extended_code,
        other => panic!("expected a SQLite failure, got {other:?}"),
    };
    assert_eq!(
        extended, 517,
        "expected SQLITE_BUSY_SNAPSHOT, got {extended} ({error})"
    );
    assert!(
        elapsed < std::time::Duration::from_millis(500),
        "it failed in {elapsed:?} — a BUSY_SNAPSHOT arrives in microseconds, and \
         anything that waited the full busy_timeout is a different failure"
    );
    drop(deferred);

    // The same sequence under IMMEDIATE simply works.
    let mut writer = open(&path);
    let immediate = writer
        .transaction_with_behavior(TransactionBehavior::Immediate)
        .unwrap();
    let _: i64 = immediate
        .query_row("SELECT n FROM counter", [], |row| row.get(0))
        .unwrap();
    immediate
        .execute("UPDATE counter SET n = n + 1", [])
        .expect("IMMEDIATE holds the write lock from the start");
    immediate.commit().unwrap();
}

/// **Creating the database is itself contended**, and the test above says so by
/// omission: it primes the schema first, so nothing covered the state every
/// machine passes through exactly once — no `char.db` at all, and two `char
/// init`s arriving together.
///
/// Both failures it guards are real and were reproducible about one run in ten
/// on macOS. The `journal_mode = WAL` conversion takes an exclusive lock that
/// `busy_timeout` does not cover, so the loser reported `aborted` for a
/// database nobody was writing to; and `user_version` used to be committed one
/// statement before the namespace, so a sibling that read it in the gap found
/// the row missing and failed `Query returned no rows`.
///
/// Repeated rather than run once: this is a race, and one green pass of a race
/// is not evidence.
#[test]
fn concurrent_first_opens_all_get_a_usable_database_and_one_namespace() {
    for _ in 0..16 {
        let home = Arc::new(tempfile::tempdir().unwrap());
        const OPENERS: usize = 4;
        let barrier = Arc::new(Barrier::new(OPENERS));
        let mut handles = Vec::new();

        for _ in 0..OPENERS {
            let home = Arc::clone(&home);
            let barrier = Arc::clone(&barrier);
            handles.push(std::thread::spawn(move || {
                barrier.wait();
                Db::open(home.path())
                    .expect("a first open must not lose to a sibling first open")
                    .namespace()
                    .expect("a database that exists has its namespace")
            }));
        }

        let namespaces: Vec<String> = handles.into_iter().map(|h| h.join().unwrap()).collect();
        assert!(
            namespaces.windows(2).all(|pair| pair[0] == pair[1]),
            "one database, one namespace: {namespaces:?}"
        );
    }
}

/// **Two directories claim non-overlapping blocks concurrently** — the phase's
/// done-when, against one real database from two real threads.
#[test]
fn concurrent_claimants_never_receive_overlapping_blocks() {
    let home = Arc::new(tempfile::tempdir().unwrap());
    // Prime the schema once, so the test measures claim contention rather than
    // two processes racing to create tables.
    Db::open(home.path()).unwrap();

    const CLAIMANTS: usize = 8;
    let barrier = Arc::new(Barrier::new(CLAIMANTS));
    let mut handles = Vec::new();

    for index in 0..CLAIMANTS {
        let home = Arc::clone(&home);
        let barrier = Arc::clone(&barrier);
        handles.push(std::thread::spawn(move || {
            let mut db = Db::open(home.path()).unwrap();
            let id = WorkspaceId::from_stored(format!("{index:08x}"));
            barrier.wait();
            // Re-decide on a lost race, exactly as `char init` does: losing
            // means another workspace took the block char chose between the
            // read and the write, so the answer has changed.
            for _ in 0..64 {
                match db
                    .claim_block(&id, Path::new(&format!("/w/{index}")), None, 10, "t")
                    .unwrap()
                {
                    ClaimOutcome::Claimed(block) | ClaimOutcome::AlreadyHeld(block) => {
                        return block
                    }
                    ClaimOutcome::Lost => continue,
                    ClaimOutcome::Exhausted => panic!("the machine cannot be out of ports"),
                }
            }
            panic!("lost the race 64 times");
        }));
    }

    let blocks: Vec<_> = handles.into_iter().map(|h| h.join().unwrap()).collect();
    assert_eq!(blocks.len(), CLAIMANTS);
    for (i, a) in blocks.iter().enumerate() {
        for b in blocks.iter().skip(i + 1) {
            assert!(!a.overlaps(b), "{a:?} overlaps {b:?}");
        }
    }

    let db = Db::open(home.path()).unwrap();
    assert_eq!(db.workspaces().unwrap().len(), CLAIMANTS);
}

/// Only one claimant may hold a given lease, however many ask at once.
#[test]
fn a_lease_is_granted_to_exactly_one_of_many_concurrent_claimants() {
    let home = Arc::new(tempfile::tempdir().unwrap());
    Db::open(home.path()).unwrap();

    const CLAIMANTS: usize = 8;
    let barrier = Arc::new(Barrier::new(CLAIMANTS));
    let mut handles = Vec::new();

    for index in 0..CLAIMANTS {
        let home = Arc::clone(&home);
        let barrier = Arc::clone(&barrier);
        handles.push(std::thread::spawn(move || {
            let mut db = Db::open(home.path()).unwrap();
            let lease = LeaseId::exclusive(WorkspaceId::from_stored("a3f91c02"), "browser");
            barrier.wait();
            matches!(
                db.try_acquire(&lease, 1_000, "boot-1", index as i32, None),
                Ok(AcquireOutcome::Granted)
            )
        }));
    }

    let granted = handles
        .into_iter()
        .filter(|_| true)
        .map(|h| h.join().unwrap())
        .filter(|granted| *granted)
        .count();
    assert_eq!(granted, 1, "a mutex granted to {granted} claimants");

    let db = Db::open(home.path()).unwrap();
    let rows = db.leases().unwrap();
    assert_eq!(rows.len(), 1);
    assert_eq!(rows[0].kind, LeaseKind::Exclusive);
}
