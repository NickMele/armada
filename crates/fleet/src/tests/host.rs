//! The two things Fleet reads from the machine, and nothing else does.
//!
//! Both are tested against values rather than against the machine: the clock's
//! arithmetic is driven from a number of milliseconds, and the mint's shape is
//! checked on what it produced. A test that asserted "the clock is roughly now"
//! would be asserting that the operating system works.

use std::collections::BTreeSet;

use crate::clock::{rfc3339_utc, Clock, SystemClock};
use crate::mint::{Mint, UlidMint};

#[test]
fn the_epoch_is_the_epoch() {
    assert_eq!(rfc3339_utc(0), "1970-01-01T00:00:00.000Z");
}

#[test]
fn milliseconds_are_carried_and_not_rounded() {
    assert_eq!(rfc3339_utc(1), "1970-01-01T00:00:00.001Z");
    assert_eq!(rfc3339_utc(999), "1970-01-01T00:00:00.999Z");
}

/// A leap day, a century that is not a leap year, and the last millisecond of
/// a year. The three places civil-date arithmetic goes wrong.
#[test]
fn the_awkward_days_are_right() {
    // 2024-02-29, a leap day in a leap century.
    assert_eq!(rfc3339_utc(1_709_164_800_000), "2024-02-29T00:00:00.000Z");
    // 1900 was not a leap year: the day after 1900-02-28 is 1900-03-01.
    assert_eq!(rfc3339_utc(-2_203_891_200_000), "1900-03-01T00:00:00.000Z");
    // The last millisecond of 1999.
    assert_eq!(rfc3339_utc(946_684_799_999), "1999-12-31T23:59:59.999Z");
}

/// A clock set before 1970 is a machine that is wrong, not a Job that should
/// stop — and the reading it produces has to still be a legal timestamp.
#[test]
fn a_reading_before_the_epoch_is_still_a_timestamp() {
    let said = rfc3339_utc(-1);
    assert_eq!(said, "1969-12-31T23:59:59.999Z");
}

#[test]
fn the_system_clock_answers_in_the_shape_the_timestamp_promises() {
    let now = SystemClock::new().now();
    let said = now.as_str();
    assert_eq!(said.len(), 24, "RFC3339, UTC, milliseconds: {said}");
    assert!(said.ends_with('Z'));
    assert_eq!(&said[4..5], "-");
    assert_eq!(&said[10..11], "T");
    assert_eq!(&said[19..20], ".");
}

#[test]
fn a_minted_id_is_twenty_six_crockford_characters() {
    let minted = UlidMint::new().ulid();
    let said = minted.as_str();
    assert_eq!(said.len(), 26);
    assert!(
        said.chars()
            .all(|c| "0123456789ABCDEFGHJKMNPQRSTVWXYZ".contains(c)),
        "no I, L, O or U, so a transcribed id cannot be misread: {said}"
    );
}

/// **The property the encoding exists for.** A ULID is not a UUIDv4 because a
/// lexicographic sort of these is chronological, and that only holds if two
/// minted in the same millisecond order by the sequence they were minted in.
#[test]
fn ids_minted_in_one_burst_sort_in_the_order_they_were_minted() {
    let mint = UlidMint::new();
    let burst: Vec<String> = (0..64).map(|_| mint.ulid().as_str().to_string()).collect();
    let mut sorted = burst.clone();
    sorted.sort();
    assert_eq!(burst, sorted);
    assert_eq!(
        burst.iter().collect::<BTreeSet<_>>().len(),
        burst.len(),
        "and every one is distinct"
    );
}
