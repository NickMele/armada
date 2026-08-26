//! The one place in the workspace that reads a clock.
//!
//! # Every other crate takes its instant as an argument
//!
//! `core-model` says it, `store` says it, `config` says it and the gate says
//! it: a function that reads its own inputs from the process cannot be tested
//! and cannot be replayed. That rule needs somewhere for the reading to
//! actually happen, and this is it — one trait with one method, held by Fleet,
//! passed down as a [`Timestamp`] to everything below.
//!
//! So [`Clock`] is not a convenience. It is the seam that makes the rule
//! affordable: a test plants a clock that answers a fixed string and the whole
//! system below Fleet becomes deterministic, without a single call site
//! growing a `#[cfg(test)]`.
//!
//! # The format is `Timestamp`'s, and it is spelled once
//!
//! RFC3339, UTC, millisecond precision — `core_model::Timestamp`'s own
//! contract. It is computed here rather than taken from a date library because
//! the whole computation is the civil-date arithmetic below and a dependency
//! for it would be a dependency in the crate that spawns Drones.
//!
//! There is no parsing counterpart and there will not be one here. A stored
//! instant is compared, ordered and displayed as text; the moment something
//! wants arithmetic on one, the type that holds it needs to change rather than
//! this module growing a reader.

use std::sync::Arc;
use std::time::{SystemTime, UNIX_EPOCH};

use core_model::Timestamp;

/// What time it is, asked rather than read.
///
/// **One method, and it cannot fail.** A clock that returned a `Result` would
/// put an error path into every transition Fleet writes, and there is nothing
/// a caller could do about a clock that would not answer except stop — which
/// is what the implementation below does, by falling back to the epoch rather
/// than by taking the daemon down mid-Job.
pub trait Clock: Send + Sync {
    fn now(&self) -> Timestamp;
}

impl<C: Clock + ?Sized> Clock for Arc<C> {
    fn now(&self) -> Timestamp {
        (**self).now()
    }
}

/// The machine's clock.
///
/// **The only type in this workspace that calls [`SystemTime::now`].** A grep
/// for it should find this line and nothing else outside a test.
#[derive(Clone, Copy, Debug, Default)]
pub struct SystemClock;

impl SystemClock {
    pub fn new() -> SystemClock {
        SystemClock
    }
}

impl Clock for SystemClock {
    fn now(&self) -> Timestamp {
        let millis = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            // A clock set before 1970 is a machine that is wrong, not a Job
            // that should stop. The epoch is visibly absurd in a log line,
            // which is the behaviour a silent panic would not have.
            .map(|since| since.as_millis() as i64)
            .unwrap_or(0);
        Timestamp::from_rfc3339(rfc3339_utc(millis))
    }
}

/// Milliseconds since the epoch, as `YYYY-MM-DDTHH:MM:SS.mmmZ`.
///
/// Public to this crate so `mint` can stamp a ULID from the same reading, and
/// so a test can drive the civil-date arithmetic at the boundaries that
/// actually go wrong — a leap day, a century that is not a leap year, the last
/// millisecond of a year.
pub(crate) fn rfc3339_utc(millis_since_epoch: i64) -> String {
    let (days, millis_of_day) = split_day(millis_since_epoch);
    let (year, month, day) = civil_from_days(days);
    let millis = millis_of_day % 1_000;
    let seconds_of_day = millis_of_day / 1_000;
    let (hour, minute, second) = (
        seconds_of_day / 3_600,
        (seconds_of_day / 60) % 60,
        seconds_of_day % 60,
    );
    format!("{year:04}-{month:02}-{day:02}T{hour:02}:{minute:02}:{second:02}.{millis:03}Z")
}

/// Days since the epoch, and the milliseconds inside that day.
///
/// Floor division, not truncation: a negative reading is a machine whose clock
/// is before 1970, and truncating would put it in the wrong day rather than
/// the wrong century — which is the harder mistake to notice.
fn split_day(millis_since_epoch: i64) -> (i64, i64) {
    const DAY: i64 = 86_400_000;
    let days = millis_since_epoch.div_euclid(DAY);
    (days, millis_since_epoch.rem_euclid(DAY))
}

/// Howard Hinnant's `civil_from_days`, which is the algorithm every date
/// library uses for this and is exact for the whole range an `i64` of days can
/// express. Carried rather than depended on: the whole of it is below.
fn civil_from_days(days: i64) -> (i64, i64, i64) {
    // Shift the era so that the leap-day irregularity falls at the end of a
    // 400-year cycle rather than inside one.
    let shifted = days + 719_468;
    let era = if shifted >= 0 {
        shifted
    } else {
        shifted - 146_096
    } / 146_097;
    let day_of_era = shifted - era * 146_097;
    let year_of_era =
        (day_of_era - day_of_era / 1_460 + day_of_era / 36_524 - day_of_era / 146_096) / 365;
    let year = year_of_era + era * 400;
    let day_of_year = day_of_era - (365 * year_of_era + year_of_era / 4 - year_of_era / 100);
    let shifted_month = (5 * day_of_year + 2) / 153;
    let day = day_of_year - (153 * shifted_month + 2) / 5 + 1;
    let month = if shifted_month < 10 {
        shifted_month + 3
    } else {
        shifted_month - 9
    };
    (if month <= 2 { year + 1 } else { year }, month, day)
}
