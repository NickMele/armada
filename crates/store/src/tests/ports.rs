//! A Job's port span, and the main checkout's, told apart by the type rather
//! than by two nullable columns a test would otherwise have to police.

use core_model::Timestamp;

use crate::tests::{job_id, open, top_level, TempDir};
use crate::{PortClaim, PortClaimant, Store, WriteError};

fn a_job(store: &mut Store, id: &str) {
    let job = top_level(id);
    store
        .insert_job(&job, &crate::tests::created_at())
        .expect("the job is stored");
}

fn claimed_at() -> Timestamp {
    Timestamp::from_rfc3339("2026-09-11T09:00:00.000Z")
}

/// **The whole claim.** A span written for a Job reads back with the same
/// base and width, and names the Job it was claimed for.
#[test]
fn a_jobs_claim_survives_the_read() {
    let dir = TempDir::new();
    let mut store = open(&dir);
    a_job(&mut store, "01PORT000000000000000001");
    let job = job_id("01PORT000000000000000001");
    let claim = PortClaim {
        claimant: PortClaimant::Job(job.clone()),
        base: 41000,
        width: 4,
        claimed_at: claimed_at(),
    };
    store.claim_port_span(&claim).expect("the span is claimed");

    let read = store
        .port_span_for_job(&job)
        .expect("the read succeeds")
        .expect("a claim is there");
    assert_eq!(read, claim);
}

/// A Job with no `ports:` in its Manifest set claims nothing, and asking for
/// its span answers `None` rather than a zero-width claim.
#[test]
fn a_job_with_no_claim_has_none() {
    let dir = TempDir::new();
    let mut store = open(&dir);
    a_job(&mut store, "01PORT000000000000000002");
    assert_eq!(
        store
            .port_span_for_job(&job_id("01PORT000000000000000002"))
            .expect("the read succeeds"),
        None,
        "nothing was claimed, so nothing can be handed back"
    );
}

/// **Two Jobs never share a span.** The whole reason this table exists —
/// `docs/concepts/fleet.md`, *Ports*: "A claim carries a `job_id` ... so who
/// holds which span is a query rather than an inspection of directories."
#[test]
fn two_jobs_get_different_spans() {
    let dir = TempDir::new();
    let mut store = open(&dir);
    a_job(&mut store, "01PORT000000000000000003");
    a_job(&mut store, "01PORT000000000000000004");
    let first = job_id("01PORT000000000000000003");
    let second = job_id("01PORT000000000000000004");
    store
        .claim_port_span(&PortClaim {
            claimant: PortClaimant::Job(first.clone()),
            base: 41000,
            width: 4,
            claimed_at: claimed_at(),
        })
        .expect("the first span is claimed");
    store
        .claim_port_span(&PortClaim {
            claimant: PortClaimant::Job(second.clone()),
            base: 41004,
            width: 4,
            claimed_at: claimed_at(),
        })
        .expect("the second span is claimed");

    let first_read = store
        .port_span_for_job(&first)
        .expect("read")
        .expect("a claim");
    let second_read = store
        .port_span_for_job(&second)
        .expect("read")
        .expect("a claim");
    assert_ne!(
        first_read.base, second_read.base,
        "the two Jobs do not hold the same span"
    );
}

/// The main checkout — the proof run after a merge, and a server started with
/// no Job, both draw from this — gets a claim of its own, keyed by neither a
/// Job nor a run.
#[test]
fn the_main_checkout_gets_a_claim_of_its_own() {
    let dir = TempDir::new();
    let mut store = open(&dir);
    store
        .claim_port_span(&PortClaim {
            claimant: PortClaimant::MainCheckout,
            base: 42000,
            width: 2,
            claimed_at: claimed_at(),
        })
        .expect("the main checkout's claim is recorded");

    let read = store
        .port_span_for_main_checkout()
        .expect("the read succeeds")
        .expect("a claim is there");
    assert_eq!(read.claimant, PortClaimant::MainCheckout);
    assert_eq!(read.base, 42000);

    let all = store.every_port_claim().expect("every claim is read");
    assert_eq!(all.len(), 1, "one claim, naming the main checkout");
    assert_eq!(all[0].claimant, PortClaimant::MainCheckout);

    store
        .release_port_span(&PortClaimant::MainCheckout)
        .expect("the main checkout's claim is released");
    assert!(
        store
            .every_port_claim()
            .expect("every claim is read")
            .is_empty(),
        "the main checkout's own release takes its claim, with no Job involved at all"
    );
}

/// A second claim for the main checkout is refused rather than silently
/// replacing it — the same rule a Job's own second claim is refused by.
#[test]
fn a_second_claim_for_the_main_checkout_is_refused() {
    let dir = TempDir::new();
    let mut store = open(&dir);
    store
        .claim_port_span(&PortClaim {
            claimant: PortClaimant::MainCheckout,
            base: 42000,
            width: 2,
            claimed_at: claimed_at(),
        })
        .expect("the first claim is recorded");
    let refused = store.claim_port_span(&PortClaim {
        claimant: PortClaimant::MainCheckout,
        base: 42010,
        width: 2,
        claimed_at: claimed_at(),
    });
    assert!(
        matches!(refused, Err(WriteError::Database(_))),
        "a second claim for the main checkout is a conflict, not a replacement: {refused:?}"
    );
}

/// **A Job and the main checkout never overlap.** Both are real, independent
/// claims, and the ranges Fleet picks them from never collide by
/// construction — this is the store's own half of that: two rows, two
/// different spans, read back apart.
#[test]
fn a_job_and_the_main_checkout_never_overlap() {
    let dir = TempDir::new();
    let mut store = open(&dir);
    a_job(&mut store, "01PORT000000000000000010");
    let job = job_id("01PORT000000000000000010");
    store
        .claim_port_span(&PortClaim {
            claimant: PortClaimant::Job(job.clone()),
            base: 41000,
            width: 4,
            claimed_at: claimed_at(),
        })
        .expect("the job's span is claimed");
    store
        .claim_port_span(&PortClaim {
            claimant: PortClaimant::MainCheckout,
            base: 41004,
            width: 4,
            claimed_at: claimed_at(),
        })
        .expect("the main checkout's span is claimed");

    let job_read = store
        .port_span_for_job(&job)
        .expect("read")
        .expect("a claim");
    let main_read = store
        .port_span_for_main_checkout()
        .expect("read")
        .expect("a claim");
    assert_ne!(
        job_read.base, main_read.base,
        "the Job and the main checkout do not hold the same span"
    );
    assert_eq!(store.every_port_claim().expect("read").len(), 2);
}

/// **The span is free again once the Job ends.** Release, not a timer —
/// `docs/concepts/fleet.md` rules out anything that ages a claim out.
#[test]
fn releasing_a_jobs_span_frees_it() {
    let dir = TempDir::new();
    let mut store = open(&dir);
    a_job(&mut store, "01PORT000000000000000005");
    let job = job_id("01PORT000000000000000005");
    store
        .claim_port_span(&PortClaim {
            claimant: PortClaimant::Job(job.clone()),
            base: 41000,
            width: 4,
            claimed_at: claimed_at(),
        })
        .expect("the span is claimed");

    store
        .release_port_span(&PortClaimant::Job(job.clone()))
        .expect("the span is released");
    assert_eq!(
        store.port_span_for_job(&job).expect("the read succeeds"),
        None,
        "the span is free again"
    );
    // Releasing again is not a fault: every road that reaches release is a
    // road where the span is already gone, the same answer
    // `forget_drone_process` gives.
    store
        .release_port_span(&PortClaimant::Job(job))
        .expect("releasing nothing is not a fault");
}

/// A claim against a Job that does not exist is refused **by name**, the same
/// shape `record_drone_process` refuses one in.
#[test]
fn a_claim_against_no_job_is_refused_by_name() {
    let dir = TempDir::new();
    let mut store = open(&dir);
    let refused = store
        .claim_port_span(&PortClaim {
            claimant: PortClaimant::Job(job_id("01PORT000000000000000009")),
            base: 41000,
            width: 4,
            claimed_at: claimed_at(),
        })
        .expect_err("there is no such Job");
    assert!(
        matches!(refused, WriteError::NoSuchJob { ref job_id } if job_id.as_str() == "01PORT000000000000000009"),
        "the refusal names the Job: {refused:?}"
    );
}

/// A second claim for a Job already holding one is refused rather than
/// silently replacing it — unlike a Drone's process, a Job claims its span
/// once, at worktree cut.
#[test]
fn a_second_claim_for_the_same_job_is_refused() {
    let dir = TempDir::new();
    let mut store = open(&dir);
    a_job(&mut store, "01PORT000000000000000006");
    let job = job_id("01PORT000000000000000006");
    store
        .claim_port_span(&PortClaim {
            claimant: PortClaimant::Job(job.clone()),
            base: 41000,
            width: 4,
            claimed_at: claimed_at(),
        })
        .expect("the first claim is recorded");
    let refused = store.claim_port_span(&PortClaim {
        claimant: PortClaimant::Job(job),
        base: 41010,
        width: 4,
        claimed_at: claimed_at(),
    });
    assert!(
        matches!(refused, Err(WriteError::Database(_))),
        "a second claim for one Job is a conflict, not a replacement: {refused:?}"
    );
}

/// Forgetting the Job takes its port claim with it, through the catalog walk
/// rather than a hand-kept list — the safety net behind the explicit release
/// at teardown.
#[test]
fn forgetting_a_job_forgets_its_port_claim() {
    let dir = TempDir::new();
    let mut store = open(&dir);
    a_job(&mut store, "01PORT000000000000000007");
    let job = job_id("01PORT000000000000000007");
    store
        .claim_port_span(&PortClaim {
            claimant: PortClaimant::Job(job.clone()),
            base: 41000,
            width: 4,
            claimed_at: claimed_at(),
        })
        .expect("the span is claimed");
    let forgotten = store.forget_job(&job).expect("the job is forgotten");
    assert_eq!(
        forgotten.port_claims, 1,
        "the row is counted by name rather than in the lump"
    );
    assert_eq!(forgotten.other, 0, "no table went uncounted");
}
