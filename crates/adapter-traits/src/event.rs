//! What a Drone said, after somebody typed it.
//!
//! One value per line of the Drone's output stream. The decoding is
//! `adapters`' — the stream's shape belongs to whoever wrote the CLI — and this
//! is the vocabulary the rest of Armada reads, which is why it names
//! capabilities and conditions rather than a vendor's keys.
//!
//! **There is no variant that says the run succeeded.** Measured, spike 3: a
//! run denied every tool it needed terminated with exit code 0, `is_error`
//! false, a success subtype and a polite final message, having accomplished
//! nothing. Four signals, all present, all agreeing, all wrong. So
//! [`Ended`](DroneEvent::Ended) carries the turn count, the cost and how many
//! calls were refused, and **there is no field on it a gate could read as a
//! verdict**. Evidence submitted through the tool is the only proof, and
//! nothing cheap goes beside it.

use alloc::string::String;

/// How much of a call's detail a row carries.
///
/// One terminal line, near enough. The reason a call was worth seeing is at the
/// front of it — `rm -rf` is the first eight characters — and what follows is
/// the tail of a heredoc.
const DETAIL: usize = 200;

/// What a tool call was on, as a person reads it: a path, a command, a pattern.
///
/// **Bounded at construction and nowhere else.** The field is private and
/// [`CallDetail::of`] is the only way to fill it, so there is no call site at
/// which an unbounded argument can be put on a row — a `Write` argument is a
/// whole file.
///
/// Whitespace collapses to single spaces, because a row is one line and a
/// heredoc is not.
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct CallDetail {
    text: String,
    truncated: bool,
}

impl CallDetail {
    /// Bound one: whitespace collapses to single spaces, and what is left is
    /// cut at [`DETAIL`] characters.
    ///
    /// The cut happens word by word rather than on the whole string, so a
    /// single argument the size of a file is never assembled to be thrown away.
    pub fn of(text: &str) -> CallDetail {
        let mut collapsed = String::new();
        let mut truncated = false;
        for word in text.split_whitespace() {
            let separator = usize::from(!collapsed.is_empty());
            let room = DETAIL.saturating_sub(collapsed.chars().count() + separator);
            if room == 0 {
                truncated = true;
                break;
            }
            if separator == 1 {
                collapsed.push(' ');
            }
            match word.char_indices().nth(room) {
                Some((cut, _)) => {
                    collapsed.push_str(&word[..cut]);
                    truncated = true;
                    break;
                }
                None => collapsed.push_str(word),
            }
        }
        CallDetail {
            text: collapsed,
            truncated,
        }
    }

    /// Nothing worth showing. A tool whose arguments this vocabulary has no
    /// name for is still a call, and an empty detail says so without guessing.
    pub fn none() -> CallDetail {
        CallDetail::default()
    }

    pub fn text(&self) -> &str {
        &self.text
    }

    /// Whether what a person is shown is less than what the Drone sent. **A row
    /// says it was cut**, rather than leaving a reader to infer it from a
    /// trailing character a command could legitimately end with.
    pub fn truncated(&self) -> bool {
        self.truncated
    }
}

/// One line of a Drone's output.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum DroneEvent {
    /// The session opened. Carries what the harness says it is running as,
    /// which is how Armada learns the model it *actually* got rather than the
    /// one it asked for.
    Started {
        session: String,
        model: String,
        /// How many MCP servers the session came up with. **Armada injects
        /// exactly one**, so any other number is the confinement having failed
        /// and is worth seeing rather than inferring.
        mcp_servers: usize,
    },
    /// The Drone reached for a tool, and what it reached for it with.
    ///
    /// **A call carries what it did, bounded.** `Bash · toolu_01Haa…`
    /// twenty-two times cannot tell `ls` from `rm -rf`, so this carries a
    /// [`CallDetail`]. The bound is the type's rather than a caller's, because
    /// a `Write` argument is a whole file.
    Called {
        tool: String,
        call: String,
        detail: CallDetail,
    },
    /// A tool answered. `failed` is the tool's own failure and not a verdict on
    /// the step.
    Answered { call: String, failed: bool },
    /// The Drone wrote prose. **Advances nothing**: prose is not a completion
    /// claim, and there is no path from this variant to a transition.
    Said { text: String },
    /// A call was refused. Silent to the Drone, and never silent here.
    Refused {
        tool: String,
        call: String,
        /// The harness's own wording for why. Carried, never matched on.
        because: String,
    },
    /// The quota window moved. Read for dispatch gating, not for this Job.
    QuotaMoved { window: String, status: String },
    /// A turn finished. **Not a verdict and not an exit** — see this module for
    /// the first, and this variant for the second.
    ///
    /// **It is a turn boundary, not a lifetime.** Measured, spike 4: one
    /// process emitted `result`, accepted an injected turn, emitted a second
    /// `init`, did more work and emitted a second `result` — one session id
    /// throughout. A reader treating the first of these as "the Drone has
    /// exited" reaps a live session. Whether the Drone is gone is a question
    /// about the process, answered by `fleet::holder_of` and never by this
    /// stream.
    Ended {
        turns: u32,
        /// What the turn cost, in millionths of a dollar.
        ///
        /// An integer rather than the fraction the harness reports, because a
        /// budget is compared and accumulated, and a float that is compared and
        /// accumulated is a budget that drifts. Six decimal places is what the
        /// harness itself reports, so nothing is lost converting.
        cost_micros: u64,
        /// How many calls were refused across the turn. Non-zero with no
        /// evidence is `blocked_by_policy`; zero with no evidence is `silent`,
        /// and the remedies are opposite.
        refusals: usize,
    },
    /// Something arrived that this vocabulary has no variant for. Carried by
    /// kind so a stream that grew an event is visible as a count rather than as
    /// a gap.
    Unrecognised { kind: String },
    /// A line that did not decode. **Never dropped.**
    ///
    /// It exists so that no decoder anywhere can answer "nothing happened" for
    /// output it did not understand. Same rule as a query function that never
    /// returns pre-filtered results: the caller sees what failed rather than
    /// being told, silently, that there was less of it.
    Unreadable {
        /// What could not be read, as the decoder saw it. Truncated by the
        /// decoder, because a runaway line is still a line.
        line: String,
        why: String,
    },
}

impl DroneEvent {
    /// Whether this is the Drone having reached for something and been stopped.
    pub fn is_a_refusal(&self) -> bool {
        matches!(self, DroneEvent::Refused { .. })
    }

    /// Whether the decoder could not read the line.
    pub fn is_unreadable(&self) -> bool {
        matches!(self, DroneEvent::Unreadable { .. })
    }
}

#[cfg(test)]
mod tests {
    use super::{CallDetail, DETAIL};

    /// The bound belongs to the type, so a caller cannot opt out of it by
    /// building the value another way — there is no other way.
    #[test]
    fn a_detail_longer_than_a_row_is_cut_and_says_so() {
        let bounded = CallDetail::of(&"x".repeat(DETAIL * 3));
        assert!(bounded.truncated());
        assert_eq!(bounded.text().chars().count(), DETAIL);
    }

    #[test]
    fn a_detail_that_fits_is_untouched_and_does_not_claim_to_be_cut() {
        let bounded = CallDetail::of("cargo build --workspace");
        assert_eq!(bounded.text(), "cargo build --workspace");
        assert!(!bounded.truncated());
    }

    /// A row is one line. A word cut mid-way is still cut, and the flag is what
    /// says so — a command can legitimately end in an ellipsis, so a trailing
    /// character could not.
    #[test]
    fn whitespace_collapses_and_the_cut_never_leaves_a_trailing_space() {
        let bounded = CallDetail::of("cat <<EOF\n  one\n  two\nEOF");
        assert_eq!(bounded.text(), "cat <<EOF one two EOF");

        let words = CallDetail::of(&"word ".repeat(DETAIL));
        assert!(bounded.text() == bounded.text().trim());
        assert_eq!(words.text(), words.text().trim_end());
        assert!(words.truncated());
    }

    #[test]
    fn nothing_worth_showing_is_empty_rather_than_a_guess() {
        assert_eq!(CallDetail::none().text(), "");
        assert!(!CallDetail::none().truncated());
    }
}
