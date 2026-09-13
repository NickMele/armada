//! Overview's claim: **everything in flight and everything waiting on me,
//! across my repositories, on one surface — and I can answer it, or ask Helm
//! about it, without leaving.**
//!
//! **The screen half is not a Rust question.** What is reachable is the seam
//! the surface is drawn from: that a received Job names its Manifest, so a
//! client can sort a mixed list without a second query, and that the tiles'
//! readings survive [`ipc::encode`] and back. The apparatus is
//! [`bench::overview`]. **A green run here is not the milestone** — the two
//! tables below name what still is not proved.

// The bench is shared with the other milestones' tests and none of them uses
// all of it. Every item in it is reached from one of the four below.
#[allow(dead_code)]
mod bench;

use core_model::AdmissionHold;
use ipc::{
    Declaration, Drift, FleetCapacity, FleetHealth, JobList, JobSummary, ManifestDrift, Probe,
    Unfollowed, Unprobed,
};

use bench::overview::{created_under, done_under, queued_under, running_under};

// | Not proved here | Why not, and what would prove it |
// |---|---|
// | Answering a question — a Drone's, a held command, a Judge refusal — by id, and a stale id refused | Helm's session host is still being built; #936 |
// | A session taking two turns, the second remembering the first, and a resumed one remembering both | Same reason; #939 |
// | That a person reading the surface learns anything, and Bridge's own pick | Nothing here renders — `needs-you.ts` and `board.ts` own the rule; `ofPicked` is not a Rust question |

// ---------------------------------------------------------------------------
// Every Job carries the Manifest it belongs to
// ---------------------------------------------------------------------------

/// Copied from `domain/job-statuses.toml`'s `who_is_acting`/`mode` columns:
/// `core_model` exposes neither to read, so the lists below are hand kept —
/// `every_status_the_registry_names_is_classified_or_excluded_as_terminal`
/// is what catches them drifting from `JobStatus::ALL`.
const NEEDS_YOU: &[&str] = &[
    "awaiting_approval",
    "awaiting_attestation",
    "awaiting_repair",
    "awaiting_review",
    "escalated",
];
const RUNNING: &[&str] = &["running", "piloted"];
const QUEUED: &[&str] = &["queued"];

/// Overview's three drawn sections — Needs you, Running and Queued — read off
/// one row, with **Other named as the section this build never populates.**
/// `asking` is read first because it is not a status at all — `board.ts`
/// reads it the same way, ahead of every status-derived rule.
fn section_of(row: &JobSummary) -> Option<&'static str> {
    if row.asking {
        return Some("needs-you");
    }
    let status = row.status.as_wire();
    if NEEDS_YOU.contains(&status) {
        Some("needs-you")
    } else if RUNNING.contains(&status) {
        Some("running")
    } else if QUEUED.contains(&status) {
        Some("queued")
    } else {
        None
    }
}

/// A new status, or a misspelling in one of the three lists above, fails this
/// rather than passing silently: every non-terminal status in
/// `core_model::JobStatus::ALL` lands in exactly one of the three, every
/// terminal one lands in none, and every name the lists carry is a real
/// status's wire spelling.
#[test]
fn every_status_the_registry_names_is_classified_or_excluded_as_terminal() {
    for status in core_model::JobStatus::ALL {
        let wire = status.as_wire();
        let placed = [NEEDS_YOU, RUNNING, QUEUED]
            .iter()
            .filter(|list| list.contains(&wire))
            .count();
        if status.is_terminal() {
            assert_eq!(
                placed, 0,
                "{wire} is terminal and belongs in none of the three"
            );
        } else {
            assert_eq!(placed, 1, "{wire} must land in exactly one of the three");
        }
    }
    for name in NEEDS_YOU.iter().chain(RUNNING).chain(QUEUED) {
        assert!(
            core_model::JobStatus::from_wire(name).is_some(),
            "{name} is not a status this build's registry carries"
        );
    }
}

/// Two Manifests, one list, and each Manifest's own sections read back out of
/// it — without a second query and without one Manifest's Jobs leaking into
/// the other's count.
///
/// **The failure this is against is a client that reads the whole list once
/// it has one Manifest's Jobs in hand.** A Board narrowed by `ofPicked`
/// filters on `owner_manifest_id`; a client that instead trusted "the list I
/// have is the Manifest I asked for" would show a second repository's Jobs
/// under the first the moment Fleet served two.
#[test]
fn every_jobs_manifest_survives_the_wire_and_sorts_two_repositories_apart() {
    let storefront = "01MANIFESTOVERVIEWSTOREFRONT";
    let mailer = "01MANIFESTOVERVIEWMAILERXXXX";

    let gate = created_under(storefront, "01JOBOVERVIEWGATE0000000A", "wait for a person");
    let running = running_under(storefront, "01JOBOVERVIEWRUNNING000A", "run the migration");
    let asking = running_under(
        storefront,
        "01JOBOVERVIEWASKING0000A",
        "ask before it writes",
    );
    let queued = queued_under(storefront, "01JOBOVERVIEWQUEUED0000A", "wait its turn");
    let done = done_under(storefront, "01JOBOVERVIEWDONE00000A", "already over");

    let mailer_gate = created_under(
        mailer,
        "01JOBOVERVIEWGATE0000000B",
        "a mailer Job waits too",
    );
    let mailer_queued = queued_under(mailer, "01JOBOVERVIEWQUEUED0000B", "and one is queued");

    let list = JobList {
        jobs: vec![
            JobSummary::of(&gate, None, None, None, false, None),
            JobSummary::of(&running, None, None, None, false, None),
            // The one row `asking` overrides: a running Job whose Drone is
            // waiting on an answer reads as Needs you, not Running.
            JobSummary::of(&asking, None, None, None, true, None),
            JobSummary::of(&queued, None, None, None, false, None),
            JobSummary::of(&done, None, None, None, false, None),
            JobSummary::of(&mailer_gate, None, None, None, false, None),
            JobSummary::of(&mailer_queued, None, None, None, false, None),
        ],
        unreadable: Vec::new(),
    };
    let received: JobList = round_trip_jobs(&list);
    assert_eq!(
        received.jobs.len(),
        7,
        "the whole of what the machine moved"
    );

    let of = |manifest: &str, section: &str| -> Vec<&str> {
        received
            .jobs
            .iter()
            .filter(|row| row.owner_manifest_id.as_str() == manifest)
            .filter(|row| section_of(row) == Some(section))
            .map(|row| row.title.as_str())
            .collect()
    };

    assert_eq!(
        of(storefront, "needs-you"),
        ["wait for a person", "ask before it writes"]
    );
    assert_eq!(of(storefront, "running"), ["run the migration"]);
    assert_eq!(of(storefront, "queued"), ["wait its turn"]);
    assert_eq!(
        of(mailer, "needs-you"),
        ["a mailer Job waits too"],
        "the second Manifest's own Needs you row, not the first's"
    );
    assert_eq!(of(mailer, "queued"), ["and one is queued"]);
    assert!(
        of(mailer, "running").is_empty(),
        "nothing here dispatched a Drone on the mailer"
    );

    // Done, and the fourth section: every non-terminal status in
    // `job-statuses.toml` is `who_is_acting = "Person"` or `"Drone"`, so
    // Other is empty by construction rather than by omission here.
    let placed: Vec<&JobSummary> = received
        .jobs
        .iter()
        .filter(|row| row.title != "already over")
        .collect();
    assert!(placed.iter().all(|row| section_of(row).is_some()));
    let over = received
        .jobs
        .iter()
        .find(|row| row.title == "already over")
        .expect("the finished Job rode with the rest");
    assert!(
        section_of(over).is_none(),
        "a Job that is over is in none of the three sections a person works down"
    );
}

// ---------------------------------------------------------------------------
// The readings the tiles draw
// ---------------------------------------------------------------------------

// | Not proved here | Why not, and what would prove it |
// |---|---|
// | Health, capacity and drift saying anything true about a real machine | Hermetic — every value below is built and round-tripped by hand, not answered by a live Fleet |
// | `?manifest_id=` on `get_manifest_drift` reaching the repository it names | `InManifest` in `crates/api/src/scoped.rs` is `pub(crate)` — `api`'s own router tests own that seam |
// | A model's reply, to anything | This file calls no model and opens no session |

/// Capacity, as the status bar and the tile both read it: the bound, what is
/// occupying it, and — only when something is — what is holding the next
/// Drone back.
///
/// **Fleet-wide**, unlike the drift tile below: `docs/concepts/fleet.md`
/// names capacity as a reading a scoped surface still shows unscoped, because
/// the bound is the machine's and not a repository's.
#[test]
fn capacity_survives_the_wire_and_says_nothing_where_nothing_holds_it_back() {
    let open = FleetCapacity::of(3, 1, None);
    let held = FleetCapacity::of(3, 3, Some(AdmissionHold::Disk));

    let (open, open_body) = round_trip_capacity(&open);
    let (held, _) = round_trip_capacity(&held);

    assert_eq!((open.bound, open.occupied), (3, 1));
    assert!(open.held_by.is_none());
    assert!(
        !open_body.contains("held_by"),
        "nothing holding admission back is absent, not null: {open_body}"
    );
    assert_eq!(
        held.held_by.as_ref().map(ipc::AdmissionHold::as_wire),
        Some("disk")
    );
}

/// Health, as far as Fleet can answer for itself: every probe it ran, and
/// everything it could not — named, never silently missing.
///
/// **Not Doctor.** `crates/ipc/src/health.rs` says so in its own header: this
/// is the rows Fleet itself holds, and the rest is `not_probed`.
#[test]
fn health_survives_the_wire_with_the_unprobed_half_still_on_it() {
    let health = FleetHealth {
        probes: vec![Probe {
            module: "Fleet".to_string(),
            outcome: "pass".to_string(),
            detail: "one Drone, one working slot".to_string(),
        }],
        not_probed: vec![Unprobed {
            owner: "adapters".to_string(),
            because: "Doctor's grid is not built".to_string(),
        }],
    };
    let received: FleetHealth = round_trip_health(&health);
    assert_eq!(received.probes.len(), 1);
    assert_eq!(received.probes[0].outcome, "pass");
    assert_eq!(
        received.not_probed[0].because, "Doctor's grid is not built",
        "a report with rows and nothing beside them reads as a healthy \
         machine, which is the failure `not_probed` exists to name"
    );
}

/// Drift, for one repository — never for the machine. Two Manifests read two
/// answers, because a `run` line's target is a fact about a checkout and not
/// about Fleet.
#[test]
fn drift_survives_the_wire_and_is_read_per_repository() {
    let storefront = ManifestDrift {
        path: "storefront/armada.yml".to_string(),
        checkout: "/repos/storefront".to_string(),
        declarations: vec![Declaration {
            section: "checks".to_string(),
            name: "e2e".to_string(),
            key: "run".to_string(),
            run: "pnpm playwright test".to_string(),
            drift: Drift::Current { checked: 1 },
            unfollowed: Vec::new(),
        }],
    };
    let mailer = ManifestDrift {
        path: "mailer/armada.yml".to_string(),
        checkout: "/repos/mailer".to_string(),
        declarations: vec![Declaration {
            section: "checks".to_string(),
            name: "lint".to_string(),
            key: "run".to_string(),
            run: "golangci-lint run".to_string(),
            drift: Drift::Gone {
                missing: vec![".golangci.yml".to_string()],
            },
            unfollowed: vec![Unfollowed {
                word: "golangci-lint".to_string(),
                why: "not a tool this read follows".to_string(),
            }],
        }],
    };

    let storefront: ManifestDrift = round_trip_drift(&storefront);
    let mailer: ManifestDrift = round_trip_drift(&mailer);

    assert!(matches!(
        storefront.declarations[0].drift,
        Drift::Current { checked: 1 }
    ));
    let Drift::Gone { missing } = &mailer.declarations[0].drift else {
        panic!("the mailer's line is missing what it names");
    };
    assert_eq!(missing, &vec![".golangci.yml".to_string()]);
    assert_eq!(mailer.declarations[0].unfollowed[0].word, "golangci-lint");
    assert_ne!(
        storefront.checkout, mailer.checkout,
        "one repository's drift, not the machine's"
    );
}

// ---------------------------------------------------------------------------
// The round trip every assertion above is made through
// ---------------------------------------------------------------------------

/// Written once per type rather than generically: `ipc::encode` and
/// `ipc::decode` already carry the bounds, and naming `serde` here would be a
/// dependency this crate does not otherwise need.
fn round_trip_jobs(value: &JobList) -> JobList {
    let body = ipc::encode(value).expect("a list that serialises");
    ipc::decode("a Job list", body.as_bytes()).expect("a list that reads back")
}

fn round_trip_capacity(value: &FleetCapacity) -> (FleetCapacity, String) {
    let body = ipc::encode(value).expect("a capacity that serialises");
    let read =
        ipc::decode("a capacity reading", body.as_bytes()).expect("a reading that reads back");
    (read, body)
}

fn round_trip_health(value: &FleetHealth) -> FleetHealth {
    let body = ipc::encode(value).expect("a health reading that serialises");
    ipc::decode("a health reading", body.as_bytes()).expect("a reading that reads back")
}

fn round_trip_drift(value: &ManifestDrift) -> ManifestDrift {
    let body = ipc::encode(value).expect("a drift reading that serialises");
    ipc::decode("a drift reading", body.as_bytes()).expect("a reading that reads back")
}
