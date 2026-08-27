//! The graded skew rule, in the four readings Bridge can take.
//!
//! `docs/practices/protocol.md` is the specification and these are its rows.
//! The asymmetric pair is the one worth reading twice: the same minor gap is a
//! banner in one direction and a refusal in the other, because additive-only
//! promises that a *newer writer's* additions are ignorable and promises
//! nothing about what a *newer reader* requires.

use crate::{decode, encode, Cursor, JobList, ProtocolVersion, Resync, Skew, StreamMessage};

/// Bridge's own version, so nothing here reads as a claim about which one
/// this build is at.
const BRIDGE: ProtocolVersion = ProtocolVersion::new(7, 3);

#[test]
fn the_same_version_connects_with_nothing_to_say() {
    assert_eq!(BRIDGE.reading(BRIDGE), Skew::Same);
    assert!(BRIDGE.reading(BRIDGE).connects());
}

#[test]
fn a_fleet_ahead_by_a_minor_connects_because_bridge_ignores_the_additions() {
    let fleet = ProtocolVersion::new(7, 4);
    assert_eq!(BRIDGE.reading(fleet), Skew::FleetAhead);
    assert!(BRIDGE.reading(fleet).connects());
}

/// The direction that looks identical and is not. Fleet was built before the
/// field Bridge now reads, so the hole arrives mid-Job rather than at startup.
#[test]
fn a_fleet_behind_by_a_minor_is_refused() {
    let fleet = ProtocolVersion::new(7, 2);
    assert_eq!(BRIDGE.reading(fleet), Skew::FleetBehind);
    assert!(!BRIDGE.reading(fleet).connects());
}

#[test]
fn a_major_gap_is_refused_whichever_side_is_newer() {
    for fleet in [ProtocolVersion::new(8, 0), ProtocolVersion::new(6, 9)] {
        assert_eq!(BRIDGE.reading(fleet), Skew::Incompatible);
        assert!(!BRIDGE.reading(fleet).connects());
    }
}

/// A minor that is higher under a different major buys nothing. The majors are
/// checked first, so no arithmetic on minors can talk its way past one.
#[test]
fn a_higher_minor_does_not_rescue_a_major_gap() {
    assert_eq!(
        BRIDGE.reading(ProtocolVersion::new(8, 99)),
        Skew::Incompatible
    );
}

#[test]
fn the_pair_crosses_as_one_field_carrying_both_numbers() {
    let resync = Resync {
        protocol_version: BRIDGE,
        cursor: Cursor::at(0),
        jobs: JobList {
            jobs: Vec::new(),
            unreadable: Vec::new(),
        },
    };
    let json = encode(&resync).expect("plain data");
    assert!(
        json.contains(r#""protocol_version":{"major":7,"minor":3}"#),
        "{json}"
    );
    assert_eq!(
        decode::<Resync>("resync", json.as_bytes()).expect("it round-trips"),
        resync
    );
}

/// A peer from before the pair existed sends one integer, which names a major
/// at minor zero. Read rather than refused, so an older Fleet reaches the skew
/// screen instead of coming back as a message nothing wrote.
#[test]
fn a_bare_integer_reads_as_that_major_at_minor_zero() {
    let body = br#"{"message":"resync","protocol_version":4,"cursor":0,
        "jobs":{"jobs":[],"unreadable":[]}}"#;
    let message = decode::<StreamMessage>("stream message", body).expect("one integer still reads");
    let StreamMessage::Resync(resync) = message else {
        panic!("a resync");
    };
    assert_eq!(resync.protocol_version, ProtocolVersion::new(4, 0));
}
