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
/// **Two forms and a true size, because a row and an opened row are different
/// questions.** A row is one line, so [`shown`](CallDetail::shown) collapses
/// whitespace and stops at [`DETAIL`]. Opening that row exists to read the
/// argument, so [`whole`](CallDetail::whole) is what the Drone sent, and
/// [`length`](CallDetail::length) is how much there was — which is what lets a
/// reader be told *showing 200 of 14,320 characters* instead of being told that
/// something was taken away.
///
/// **The whole is present exactly where the shown form is less than all of
/// it.** There is no state in which a row was cut and the rest is unreachable,
/// and none in which a whole is carried beside a shown form that already had
/// everything.
///
/// **Bounded at construction and nowhere else.** The fields are private and
/// [`CallDetail::of`] is the only way to fill them, so no call site can put an
/// unbounded argument on a row, and none can state a length the argument did
/// not have.
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct CallDetail {
    shown: String,
    whole: Option<String>,
    length: usize,
}

impl CallDetail {
    /// Collapse for the row, keep the whole for the payload, count what there
    /// was.
    ///
    /// The collapse runs word by word and stops at [`DETAIL`], so the shown
    /// form never assembles a string the size of a file to throw most of it
    /// away. The whole is copied only where the collapse ran out of room: an
    /// argument that fits is already all of it, and a second copy would travel
    /// the pipe, the file and back for nothing.
    pub fn of(text: &str) -> CallDetail {
        let mut collapsed = String::new();
        let mut ran_out = false;
        for word in text.split_whitespace() {
            let separator = usize::from(!collapsed.is_empty());
            let room = DETAIL.saturating_sub(collapsed.chars().count() + separator);
            if room == 0 {
                ran_out = true;
                break;
            }
            if separator == 1 {
                collapsed.push(' ');
            }
            match word.char_indices().nth(room) {
                Some((cut, _)) => {
                    collapsed.push_str(&word[..cut]);
                    ran_out = true;
                    break;
                }
                None => collapsed.push_str(word),
            }
        }
        CallDetail {
            shown: collapsed,
            whole: match ran_out {
                true => Some(String::from(text)),
                false => None,
            },
            length: text.chars().count(),
        }
    }

    /// Nothing worth showing. A tool whose arguments this vocabulary has no
    /// name for is still a call, and an empty detail says so without guessing.
    pub fn none() -> CallDetail {
        CallDetail::default()
    }

    /// What a row shows: one line, whitespace collapsed, cut at [`DETAIL`].
    pub fn shown(&self) -> &str {
        &self.shown
    }

    /// The argument as the Drone sent it, where that is more than a row shows.
    ///
    /// **`None` is never "nothing was sent"** — it is the shown form already
    /// being all of it, which is why a caller that wants the argument whatever
    /// its size reads this and falls back to [`shown`](CallDetail::shown).
    pub fn whole(&self) -> Option<&str> {
        self.whole.as_deref()
    }

    /// How many characters the argument had, before anything was cut.
    ///
    /// **The size of what there is, never of what is shown.** A reader holding
    /// both learns how much is behind the rest, which is the one thing a
    /// `truncated` flag on its own could not say.
    pub fn length(&self) -> usize {
        self.length
    }

    /// Whether what a person is shown is less than what the Drone sent. **A row
    /// says it was cut**, rather than leaving a reader to infer it from a
    /// trailing character a command could legitimately end with.
    ///
    /// Derived from the whole being kept rather than stored beside it, so "this
    /// was cut" and "the rest is here" cannot disagree.
    pub fn truncated(&self) -> bool {
        self.whole.is_some()
    }
}

/// Who put a line of prose into the session.
///
/// **A Drone's stream is two channels and prose arrives on both.** The output
/// channel is the Drone writing; the input channel is the harness replaying
/// what was typed at it — the brief a step opened with, a person's redirect, a
/// poke. Both used to decode to the same [`DroneEvent::Said`], so the longest
/// thing on a transcript was Armada quoting itself back under the Drone's name.
///
/// **Two values and not `ipc::Voice`'s three.** This crate has no
/// dependencies, deliberately, so importing that one is not available — but it
/// would be the wrong set anyway. `Fleet` names Fleet acting on the Job around
/// the Drone, and Fleet's own acts appear in a Drone's output stream never.
/// These are the two a decoder can witness, and `fleet::transcript::row` is
/// where they widen onto the record's three.
///
/// **The channel and never the wording.** Matching an injected turn's text
/// would break the moment `docs/contracts/agent-prompt.md` revised a word, and
/// the fact is already in hand at the line that carries it.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub enum Speaker {
    /// The Drone's own output. **The default**, because that is what a stream
    /// is mostly made of and what a caller building an event means.
    #[default]
    Drone,
    /// A turn put into the Drone's session, coming back on its input channel.
    ///
    /// **Armada's, or the harness's own.** A continuation nudge the CLI writes
    /// arrives here too and is indistinguishable from a turn Fleet sent — which
    /// costs nothing this variant is for, since the question it answers is
    /// whether the Drone wrote it.
    Armada,
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
    /// **A call carries what it did, bounded for the row and whole behind
    /// it.** `Bash · toolu_01Haa…` twenty-two times cannot tell `ls` from
    /// `rm -rf`, so this carries a [`CallDetail`]. The bound is the type's
    /// rather than a caller's, because a heredoc argument is a whole file — and
    /// the same type keeps that file, so what a row cut is served rather than
    /// apologised for.
    Called {
        tool: String,
        call: String,
        detail: CallDetail,
    },
    /// A tool answered. `failed` is the tool's own failure and not a verdict on
    /// the step.
    Answered { call: String, failed: bool },
    /// Prose crossed the session, and [`Speaker`] says which way.
    ///
    /// **Advances nothing**: prose is not a completion claim, and there is no
    /// path from this variant to a transition.
    ///
    /// **Only the Drone's counts as the Drone having turned.** `fleet::watch`
    /// folds these into a turn count that `Working::turned_since_redirect`
    /// compares against a baseline taken before an instruction is sent — so
    /// with no speaker on the row, the harness echoing the redirect answered
    /// the redirect.
    Said { text: String, by: Speaker },
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
        assert_eq!(bounded.shown().chars().count(), DETAIL);
    }

    /// **The row that was cut is the row that can be opened.** A size and no
    /// remainder is the state this type is built so nobody can construct.
    #[test]
    fn a_cut_detail_states_its_true_size_and_keeps_the_rest() {
        let argument = "x".repeat(DETAIL * 3);
        let bounded = CallDetail::of(&argument);
        assert_eq!(bounded.length(), DETAIL * 3);
        assert_eq!(bounded.whole(), Some(argument.as_str()));
    }

    #[test]
    fn a_detail_that_fits_is_untouched_and_does_not_claim_to_be_cut() {
        let bounded = CallDetail::of("cargo build --workspace");
        assert_eq!(bounded.shown(), "cargo build --workspace");
        assert!(!bounded.truncated());
        assert_eq!(bounded.length(), "cargo build --workspace".chars().count());
        assert_eq!(bounded.whole(), None, "the shown form is already all of it");
    }

    /// A row is one line. A word cut mid-way is still cut, and the flag is what
    /// says so — a command can legitimately end in an ellipsis, so a trailing
    /// character could not.
    #[test]
    fn whitespace_collapses_and_the_cut_never_leaves_a_trailing_space() {
        let bounded = CallDetail::of("cat <<EOF\n  one\n  two\nEOF");
        assert_eq!(bounded.shown(), "cat <<EOF one two EOF");

        let words = CallDetail::of(&"word ".repeat(DETAIL));
        assert!(bounded.shown() == bounded.shown().trim());
        assert_eq!(words.shown(), words.shown().trim_end());
        assert!(words.truncated());
    }

    /// Collapsing is not cutting. A heredoc that fits loses its newlines on the
    /// row and nothing at all behind it, so a reader is not told there is more
    /// where there is not.
    #[test]
    fn collapsed_whitespace_alone_is_not_a_cut() {
        let bounded = CallDetail::of("cat <<EOF\n  one\n  two\nEOF");
        assert!(!bounded.truncated());
        assert_eq!(
            bounded.length(),
            "cat <<EOF\n  one\n  two\nEOF".chars().count()
        );
    }

    #[test]
    fn nothing_worth_showing_is_empty_rather_than_a_guess() {
        assert_eq!(CallDetail::none().shown(), "");
        assert!(!CallDetail::none().truncated());
        assert_eq!(CallDetail::none().length(), 0);
    }
}
