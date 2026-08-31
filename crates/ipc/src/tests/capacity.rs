//! What `get_capacity` must keep true, and the one row of the skew table it is
//! the only case for.
//!
//! **`held_by` is the one open set on this seam.** Every other closed set here
//! refuses a spelling the domain does not have, and these cases assert that this
//! one does not — a newer Fleet naming a fifth reason is read, and the two
//! numbers beside it survive. A closed set would have failed the message and
//! taken the occupancy with it, which is why `#51`'s budget cap costs no
//! protocol bump.

use crate::{decode, encode, AdmissionHold, FleetCapacity};

/// **Absent is "nothing is holding it", not "unknown".** The key is skipped
/// rather than written `null`, so a reader that tests for the key's presence and
/// one that tests for a null both get the same answer.
#[test]
fn a_fleet_with_room_carries_no_hold_at_all() {
    let capacity = FleetCapacity::of(2, 1, None);
    let json = encode(&capacity).expect("capacity is plain data");
    assert!(!json.contains("held_by"), "{json}");
    assert_eq!(
        decode::<FleetCapacity>("capacity", json.as_bytes()).expect("it round-trips"),
        capacity
    );
}

#[test]
fn a_hold_crosses_as_the_registry_spells_it() {
    let capacity = FleetCapacity::of(2, 2, Some(core_model::AdmissionHold::Disk));
    let json = encode(&capacity).expect("capacity is plain data");
    assert!(json.contains(r#""held_by":"disk""#), "{json}");
}

/// **The one open set on this seam, asserted rather than described.** A newer
/// Fleet naming a reason this build has never heard of is read, and the two
/// numbers beside it survive — which is the whole of why `held_by` is not a
/// `wire_enum!`. A closed set would have failed the message and taken the
/// occupancy with it.
#[test]
fn a_hold_this_build_has_never_heard_of_is_read_and_the_numbers_survive() {
    let body = br#"{"bound":2,"occupied":2,"held_by":"over_budget"}"#;
    let capacity =
        decode::<FleetCapacity>("capacity", body).expect("an unknown hold is not a bad message");
    assert_eq!(capacity.bound, 2);
    assert_eq!(capacity.occupied, 2);
    let held = capacity.held_by.expect("the reason crossed");
    assert_eq!(held.as_wire(), "over_budget");
    // And it is honestly unrenderable rather than guessed at.
    assert_eq!(held.domain(), None);
}

/// Nothing mints a spelling. Both ways in go through the registry, so a word
/// Fleet made up cannot be built in the first place.
#[test]
fn a_spelling_the_registry_does_not_have_cannot_be_constructed() {
    assert_eq!(AdmissionHold::from_wire("over_budget"), None);
    assert_eq!(
        AdmissionHold::from_wire("disk").map(|hold| hold.as_wire().to_string()),
        Some("disk".to_string())
    );
}

/// The minor-skew row again, on the payload this change added.
#[test]
fn a_capacity_from_a_newer_fleet_still_parses() {
    let body = br#"{"bound":2,"occupied":0,"queued_behind":4}"#;
    let capacity = decode::<FleetCapacity>("capacity", body).expect("unknown fields are ignored");
    assert_eq!(capacity.held_by, None);
}

/// **The other axis over `queued`, and it is closed where `held_by` is open.**
/// A spelling the registry does not have is refused rather than defaulted here,
/// because a fourth value would mean the inner step machine grew a state — not
/// that Fleet learned to read another resource.
#[test]
fn a_resumption_the_registry_does_not_have_is_refused() {
    let body = br#"{"id":"01JOB","title":"fix the parser","status":"queued","workflow_id":"01WF",
        "owner_manifest_id":"01MF","origin":"manual","urgency":"normal","atomic":false,
        "model":"a-model","created_at":"2026-08-31T09:00:00.000Z","resumption":"redirected"}"#;
    let refused = decode::<crate::JobSummary>("a summary", body)
        .expect_err("only the registry may widen a closed set");
    assert!(refused.to_string().contains("redirected"), "{refused}");
}

/// Absent is the ordinary row, and it is skipped rather than written `null`.
#[test]
fn a_summary_of_a_job_nobody_put_back_carries_no_resumption() {
    let body = br#"{"id":"01JOB","title":"fix the parser","status":"queued","workflow_id":"01WF",
        "owner_manifest_id":"01MF","origin":"manual","urgency":"normal","atomic":false,
        "model":"a-model","created_at":"2026-08-31T09:00:00.000Z"}"#;
    let summary = decode::<crate::JobSummary>("a summary", body).expect("the field is optional");
    assert_eq!(summary.resumption, None);
    let json = encode(&summary).expect("a summary is plain data");
    assert!(!json.contains("resumption"), "{json}");
}
