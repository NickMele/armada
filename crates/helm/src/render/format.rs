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

    /// The minute and hour boundaries, where an off-by-one shows as `0m 60s`.
    #[test]
    fn the_boundaries_carry_no_sixty() {
        assert_eq!(duration(59_999), "60.0s");
        assert_eq!(duration(60_000), "1m 00s");
        assert_eq!(duration(3_599_000), "59m 59s");
        assert_eq!(duration(3_600_000), "1h 00m");
        assert_eq!(duration(3_900_000), "1h 05m");
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
