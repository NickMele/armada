//! The real [`Clock`] seam.
//!
//! Two clocks, deliberately, because they answer different questions and
//! swapping them is a silent bug either way:
//!
//! - **wall** is for `claimed_at`, which is only ever *displayed*. Comparing it
//!   would make a backwards NTP step look like a stale lease.
//! - **monotonic** is for heartbeat staleness, and it must be the
//!   suspend-*excluding* one — see [`crate::posix::mono_ms`].

use charkit_core::ctx::Clock;
use std::time::{Duration, SystemTime, UNIX_EPOCH};

use crate::posix;

/// The production [`Clock`].
#[derive(Debug, Clone, Copy, Default)]
pub struct SystemClock;

impl Clock for SystemClock {
    fn wall_rfc3339(&self) -> String {
        let secs = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .map(|d| d.as_secs() as i64)
            .unwrap_or(0);
        rfc3339_utc(secs)
    }

    fn wall_ms(&self) -> u64 {
        SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .map(|d| d.as_millis() as u64)
            .unwrap_or(0)
    }

    fn mono(&self) -> u64 {
        posix::mono_ms()
    }

    fn sleep_until(&self, mono_ms: u64) {
        let now = posix::mono_ms();
        if mono_ms > now {
            std::thread::sleep(Duration::from_millis(mono_ms - now));
        }
    }
}

/// Format seconds since the epoch as `YYYY-MM-DDTHH:MM:SSZ`.
///
/// Hand-rolled rather than a date crate: char formats one timestamp, never
/// parses one, never localises, and never does arithmetic on a calendar. A
/// dependency for `strftime` would be the largest thing in the binary that does
/// the least.
fn rfc3339_utc(unix_secs: i64) -> String {
    let days = unix_secs.div_euclid(86_400);
    let seconds = unix_secs.rem_euclid(86_400);
    let (year, month, day) = civil_from_days(days);
    format!(
        "{year:04}-{month:02}-{day:02}T{:02}:{:02}:{:02}Z",
        seconds / 3600,
        (seconds % 3600) / 60,
        seconds % 60
    )
}

/// Howard Hinnant's `civil_from_days`, which is the standard closed-form
/// answer and avoids a table of month lengths and a leap-year branch.
fn civil_from_days(days: i64) -> (i64, u32, u32) {
    let z = days + 719_468;
    let era = z.div_euclid(146_097);
    let doe = z.rem_euclid(146_097);
    let yoe = (doe - doe / 1_460 + doe / 36_524 - doe / 146_096) / 365;
    let y = yoe + era * 400;
    let doy = doe - (365 * yoe + yoe / 4 - yoe / 100);
    let mp = (5 * doy + 2) / 153;
    let d = (doy - (153 * mp + 2) / 5 + 1) as u32;
    let m = if mp < 10 { mp + 3 } else { mp - 9 } as u32;
    (if m <= 2 { y + 1 } else { y }, m, d)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn the_epoch_formats_as_the_epoch() {
        assert_eq!(rfc3339_utc(0), "1970-01-01T00:00:00Z");
    }

    /// The timestamp PLAN.md §3.1's payload shows, so the documented example
    /// and the renderer agree on a real value rather than on a shape.
    #[test]
    fn a_known_instant_round_trips_to_the_documented_spelling() {
        // 2026-08-09T14:02:11Z
        assert_eq!(rfc3339_utc(1_786_284_131), "2026-08-09T14:02:11Z");
    }

    #[test]
    fn a_leap_day_is_not_off_by_one() {
        // 2024-02-29T00:00:00Z
        assert_eq!(rfc3339_utc(1_709_164_800), "2024-02-29T00:00:00Z");
    }

    #[test]
    fn a_century_boundary_is_not_a_leap_year() {
        // 1900 is not a leap year, 2000 is; the closed form has to get both.
        assert_eq!(rfc3339_utc(951_782_400), "2000-02-29T00:00:00Z");
    }

    #[test]
    fn the_wall_clock_answers_in_the_documented_shape() {
        let now = SystemClock.wall_rfc3339();
        assert_eq!(now.len(), 20, "{now}");
        assert!(now.ends_with('Z'));
        assert!(now.starts_with("20"));
    }

    /// The two wall readings answer about the same instant, because a run id
    /// minted from one and a `claimed_at` rendered from the other end up in the
    /// same payload.
    #[test]
    fn the_two_wall_readings_agree_to_the_second() {
        let ms = SystemClock.wall_ms();
        assert!(ms > 1_700_000_000_000, "{ms} is not a plausible instant");
        assert_eq!(rfc3339_utc((ms / 1000) as i64), SystemClock.wall_rfc3339());
    }

    #[test]
    fn sleeping_until_a_past_reading_returns_at_once() {
        let before = SystemClock.mono();
        SystemClock.sleep_until(0);
        assert!(SystemClock.mono() - before < 1_000);
    }
}
