//! Values that are one thing in the envelope and another on a screen.
//!
//! **Three of them, and each is agreed rather than invented**
//! (`docs/reference-output/command-output.html`): a duration, a count with its
//! noun, and a file size. Everything else a verb prints is the envelope's own
//! spelling.

/// A duration, for a person and for the agent reading the same line.
///
/// **Humanised — `26.8s`, not `26754ms`.** The agreed layout settles it, and
/// the reason is that the number is read to answer "is this slow", which
/// milliseconds actively obstruct: nobody converts 26754 in their head, and the
/// column of raw figures cannot be scanned. **Milliseconds stay in `--json`**,
/// untouched, because that reader is doing arithmetic rather than reading.
///
/// Three bands, each chosen so the value is two significant figures wide:
///
/// | Range | Form |
/// |---|---|
/// | under a minute | `0.3s`, `26.8s` |
/// | under an hour | `1m 12s` |
/// | beyond | `1h 05m` |
pub fn duration(ms: u64) -> String {
    const SECOND: u64 = 1_000;
    const MINUTE: u64 = 60 * SECOND;
    const HOUR: u64 = 60 * MINUTE;

    if ms < MINUTE {
        // One decimal, so a sub-second check reads `0.3s` rather than `0s`.
        return format!("{:.1}s", ms as f64 / SECOND as f64);
    }
    if ms < HOUR {
        return format!("{}m {:02}s", ms / MINUTE, (ms % MINUTE) / SECOND);
    }
    format!("{}h {:02}m", ms / HOUR, (ms % HOUR) / MINUTE)
}

/// How long a Job has been alive, to one unit.
///
/// **Coarser than [`duration`], deliberately**, and the agreed layout settles it:
/// `armada fleet ls` draws `14m` and `1h` where `armada manifest check` draws
/// `26.8s` and `1m 12s`. The two are answering different questions — a check's
/// number is read to decide whether the suite got slow, and a Job's is read to
/// decide whether it has been going long enough to look at. Seconds of precision
/// on a Job that has been running for an hour is noise in a column being
/// scanned.
///
/// | Range | Form |
/// |---|---|
/// | under a minute | `42s` |
/// | under an hour | `14m` |
/// | beyond | `1h` |
pub fn elapsed(ms: u64) -> String {
    const SECOND: u64 = 1_000;
    const MINUTE: u64 = 60 * SECOND;
    const HOUR: u64 = 60 * MINUTE;

    if ms < MINUTE {
        return format!("{}s", ms / SECOND);
    }
    if ms < HOUR {
        return format!("{}m", ms / MINUTE);
    }
    format!("{}h", ms / HOUR)
}

/// How long until something happens — `2h14m`, `43m`, `50s`.
///
/// **Finer than [`elapsed`] above an hour, and that is the whole difference.**
/// The two read the same clock and answer opposite questions. `elapsed` rounds a
/// Job's age to one unit because *how long has this been going* does not get
/// better with minutes on it; a countdown is read to decide whether to wait, and
/// `1h` for anything between an hour and two is exactly the precision that
/// decision cannot be made at. `020` §4 writes it `2h14m` for that reason.
///
/// **No space, unlike [`duration`]**, because this appears inside a summary line
/// whose separator is a space-padded middle dot: `resets 2h 14m` would read as
/// two facts.
///
/// **Above a day it coarsens back to whole days**, which looks like a
/// contradiction of the paragraph above and is the same argument applied twice.
/// The precision a countdown needs is the precision the *decision* needs, and
/// the decision changes with the scale: at two hours it is *do I wait*, and
/// minutes settle it; at four days it is *do I plan around this*, and
/// `96h00m` answers that in a unit nobody thinks in. The seven-day rate-limit
/// window is what made this reachable — no Job runs long enough to need it.
pub fn countdown(ms: u64) -> String {
    const SECOND: u64 = 1_000;
    const MINUTE: u64 = 60 * SECOND;
    const HOUR: u64 = 60 * MINUTE;
    const DAY: u64 = 24 * HOUR;

    if ms < MINUTE {
        return format!("{}s", ms / SECOND);
    }
    if ms < HOUR {
        return format!("{}m", ms / MINUTE);
    }
    if ms < DAY {
        return format!("{}h{:02}m", ms / HOUR, (ms % HOUR) / MINUTE);
    }
    format!("{}d", ms / DAY)
}

/// Dollars, as the agreed layout writes them: `$2.10`.
///
/// **Two decimal places always**, so a column of them lines up on the point
/// without the renderer having to align anything — `$0.45` and `$12.30` are the
/// same shape. The unrounded figure stays in `--json`, where the reader is doing
/// arithmetic rather than scanning.
///
/// **`+ 0.0` is not noise, and it is here because a real run printed
/// `$-0.00`.** Rust sums floats from `-0.0` rather than `0.0` — deliberately, so
/// that the sign of an empty or all-negative-zero sum survives — and an empty
/// fleet therefore spends negative nothing. Adding positive zero collapses the
/// two zeroes and changes no other value. A golden fixture could never have
/// caught it: every drawn row costs money.
pub fn money(usd: f64) -> String {
    format!("${:.2}", usd + 0.0)
}

/// `n` of something, pluralised by the only rule English is reliable about.
///
/// Present because a summary line saying `1 checks` is the kind of thing a
/// reader notices instead of the number.
pub fn count(n: usize, noun: &str) -> String {
    if n == 1 {
        format!("{n} {noun}")
    } else {
        format!("{n} {noun}s")
    }
}

/// A file size, for the one verb that writes a file — `armada guild export`.
///
/// **`412 KB`, in the drawing's own spelling**, and powers of 1024 because that
/// is what every other tool on the machine reports and a bundle a person is
/// about to copy somewhere is a bundle they will compare against `ls -lh`.
pub fn bytes(n: u64) -> String {
    const KB: u64 = 1024;
    const MB: u64 = KB * KB;
    if n < KB {
        return format!("{n} B");
    }
    if n < MB {
        return format!("{} KB", n / KB);
    }
    format!("{:.1} MB", n as f64 / MB as f64)
}

#[cfg(test)]
mod tests {
    use super::*;

    /// The values from the agreed layout, which is where these came from.
    #[test]
    fn the_agreed_durations_read_back_the_way_they_were_drawn() {
        assert_eq!(duration(328), "0.3s");
        assert_eq!(duration(458), "0.5s");
        assert_eq!(duration(2_600), "2.6s");
        assert_eq!(duration(26_754), "26.8s");
        assert_eq!(duration(72_000), "1m 12s");
    }

    #[test]
    fn a_duration_never_reads_as_zero_unless_it_is() {
        assert_eq!(duration(0), "0.0s");
        assert_eq!(duration(1), "0.0s");
        assert_eq!(duration(50), "0.1s");
    }

    /// **A countdown keeps its minutes past the hour, and that is why it is not
    /// [`elapsed`].** `020` §4 writes the window reset `2h14m`; `elapsed` would
    /// draw `2h`, which is the same answer for anything between one hour and
    /// two and therefore no answer to *should I wait*.
    #[test]
    fn a_countdown_keeps_the_minutes_that_elapsed_throws_away() {
        assert_eq!(countdown(8_040_000), "2h14m");
        assert_eq!(elapsed(8_040_000), "2h", "the two are the same function");
        assert_eq!(countdown(2_580_000), "43m");
        assert_eq!(countdown(50_000), "50s");
        // No sixty, and no bare `3h` where the minutes happen to be zero: a
        // column of `2h14m` and `3h00m` is one shape.
        assert_eq!(countdown(3_599_000), "59m");
        assert_eq!(countdown(3_600_000), "1h00m");
        assert_eq!(countdown(0), "0s");
    }

    /// The minute and hour boundaries, where an off-by-one shows as `0m 60s`.
    #[test]
    fn the_boundaries_carry_no_sixty() {
        assert_eq!(duration(59_999), "60.0s");
        assert_eq!(duration(60_000), "1m 00s");
        assert_eq!(duration(3_599_000), "59m 59s");
        assert_eq!(duration(3_600_000), "1h 00m");
        assert_eq!(duration(3_900_000), "1h 05m");
    }

    /// The values from the agreed layout for `armada fleet ls`, which is where
    /// these came from (`tests/golden/render/fleet-ls.plain`).
    #[test]
    fn a_jobs_run_time_reads_back_the_way_it_was_drawn() {
        assert_eq!(elapsed(840_000), "14m");
        assert_eq!(elapsed(180_000), "3m");
        assert_eq!(elapsed(1_320_000), "22m");
        assert_eq!(elapsed(3_900_000), "1h");
    }

    /// The boundaries, where an off-by-one shows as `60m` or `0m`.
    #[test]
    fn a_run_time_carries_no_sixty_and_no_zero_of_the_wrong_unit() {
        assert_eq!(elapsed(0), "0s");
        assert_eq!(elapsed(59_999), "59s");
        assert_eq!(elapsed(60_000), "1m");
        assert_eq!(elapsed(3_599_999), "59m");
        assert_eq!(elapsed(3_600_000), "1h");
    }

    #[test]
    fn money_is_always_two_places_so_a_column_of_it_lines_up() {
        assert_eq!(money(2.1), "$2.10");
        assert_eq!(money(0.4499), "$0.45");
        assert_eq!(money(0.0), "$0.00");
        assert_eq!(money(8.4), "$8.40");
    }

    /// **An empty fleet spent nothing, and it says `$0.00`.**
    ///
    /// A real `armada fleet ls` on a machine with no Jobs printed `$-0.00`,
    /// because Rust sums floats from `-0.0` so that the sign of an empty sum
    /// survives. Pinned here rather than in a golden fixture: every drawn row
    /// costs money, so no fixture can reach this value.
    #[test]
    fn an_empty_sum_is_zero_dollars_and_not_negative_zero() {
        let nothing: f64 = [0.0f64; 0].iter().sum();
        assert_eq!(money(nothing), "$0.00");
        assert_eq!(money(-0.0), "$0.00");
    }

    #[test]
    fn one_of_something_is_not_plural() {
        assert_eq!(count(1, "check"), "1 check");
        assert_eq!(count(0, "check"), "0 checks");
        assert_eq!(count(5, "check"), "5 checks");
    }

    /// `412 KB` is the drawing's own spelling, and the units are the ones
    /// `ls -lh` uses — a bundle is a file a person is about to go and look at.
    #[test]
    fn a_bundle_is_sized_the_way_the_rest_of_the_machine_sizes_a_file() {
        assert_eq!(bytes(0), "0 B");
        assert_eq!(bytes(999), "999 B");
        assert_eq!(bytes(421_888), "412 KB");
        assert_eq!(bytes(4_194_304), "4.0 MB");
    }
}
