//! What a long run says while it is still going.
//!
//! **Progress goes to stderr. Always.** A spinner on stdout means `armada
//! manifest check | jq` receives frames of animation, and the one consumer the
//! envelope exists for is the one that breaks (PLAN.md §3.1.1). Anything that
//! redraws, animates or reports intermediate state is stderr's; stdout carries
//! the result and nothing else.
//!
//! **And only when stderr is a terminal.** The audience for a redraw is a person
//! watching one; a captured stderr is a log file, and a log file full of carriage
//! returns and half-erased lines is worse than a silent one. That is why the
//! decision is made from `stderr_is_tty` rather than from the flag that decides
//! stdout's colour — `armada manifest check | jq` is a piped stdout *and* a
//! terminal stderr, and it wants both.
//!
//! **It leaves nothing behind.** [`Progress::finish`] erases the line it was
//! drawing on, so the run's real output — the envelope on stdout, a failure on
//! stderr — arrives on a clean stream.

use armada_core::error::Status;
use std::collections::BTreeSet;
use std::io::Write;

use super::palette::Role;
use super::style::Style;
use super::term::{truncate, Terminal};

/// A run reporting on itself as it goes.
///
/// **Every method has a do-nothing default**, so the silent case is
/// `impl Progress for Silent {}` and adding a hook later cannot break it. The
/// alternative — a `None` checked at four call sites — is four places to forget.
pub trait Progress {
    /// How many checks this run will report on.
    fn begin(&mut self, _total: usize) {}
    /// A check was spawned.
    fn started(&mut self, _id: &str) {}
    /// A check reached a verdict.
    fn finished(&mut self, _id: &str, _status: Status) {}
    /// A turn of the scheduler's loop went by. Redraw, if anything is animating.
    fn tick(&mut self) {}
    /// The run is over. Leave the stream as it was found.
    fn finish(&mut self) {}
}

/// Reports nothing at all: a pipe, a `--json` run, and every test that is not
/// about progress.
pub struct Silent;

impl Progress for Silent {}

/// The frames, in order. Braille, because every cell is one column wide in every
/// font that has them — an emoji spinner is two columns in some terminals and
/// one in others, and the line jitters.
const FRAMES: [&str; 10] = ["⠋", "⠙", "⠹", "⠸", "⠼", "⠴", "⠦", "⠧", "⠇", "⠏"];

/// Scheduler turns between frames.
///
/// The loop turns roughly every 20ms — it polls children on this loop — so
/// advancing every turn spins far too fast to read. Four turns is about 80ms,
/// which reads as motion rather than as flicker. Counted in turns rather than
/// measured against a clock deliberately: the clock is injected
/// (`ARCHITECTURE.md` §1.1) and a renderer is not a place to start reading one.
const TURNS_PER_FRAME: usize = 4;

/// A single redrawing line on stderr.
pub struct Spinner<W: Write> {
    out: W,
    style: Style,
    width: usize,
    frame: usize,
    turns: usize,
    total: usize,
    done: usize,
    running: BTreeSet<String>,
}

impl<W: Write> Spinner<W> {
    /// A spinner writing to `out`, at `terminal`'s width.
    pub fn new(out: W, style: Style, terminal: Terminal) -> Spinner<W> {
        Spinner {
            out,
            style,
            width: terminal.usable_width(),
            frame: 0,
            turns: 0,
            total: 0,
            done: 0,
            running: BTreeSet::new(),
        }
    }

    /// Redraw the line in place.
    ///
    /// `\r` returns to the start and `\x1b[K` erases what was there, rather than
    /// padding with spaces to the previous length: padding needs the previous
    /// length to be remembered correctly, and it is wrong exactly once — after a
    /// resize — which is when the leftovers are most confusing.
    ///
    /// **A write that fails is dropped.** Progress is not the answer; a run that
    /// cannot draw a spinner has still done its work, and turning that into an
    /// error would fail a check because a terminal went away.
    fn draw(&mut self) {
        let running: Vec<&str> = self.running.iter().map(String::as_str).collect();
        let counts = format!("{}/{}", self.done, self.total);
        let body = if running.is_empty() {
            counts
        } else {
            format!("{counts}  {}", running.join(", "))
        };
        // One column short of the terminal, so a line that exactly fills it does
        // not wrap into a second one the erase will not reach.
        let line = truncate(&body, self.width.saturating_sub(3));
        let painted = format!(
            "\r\x1b[K{} {}",
            self.style.paint(Role::SignalAmber, FRAMES[self.frame]),
            self.style.paint(Role::SteelGrey, &line)
        );
        let _ = self.out.write_all(painted.as_bytes());
        let _ = self.out.flush();
    }
}

impl<W: Write> Progress for Spinner<W> {
    fn begin(&mut self, total: usize) {
        self.total = total;
        self.draw();
    }

    fn started(&mut self, id: &str) {
        self.running.insert(id.to_string());
        self.draw();
    }

    fn finished(&mut self, id: &str, _status: Status) {
        // **The count moves only when a row leaves the running set.** A check
        // may be reported `WAITING` and then `RUNNING` before it ends, and
        // counting every verdict-shaped event would show 7/5.
        if self.running.remove(id) {
            self.done += 1;
        }
        self.draw();
    }

    fn tick(&mut self) {
        self.turns += 1;
        if self.turns.is_multiple_of(TURNS_PER_FRAME) {
            self.frame = (self.frame + 1) % FRAMES.len();
            self.draw();
        }
    }

    fn finish(&mut self) {
        let _ = self.out.write_all(b"\r\x1b[K");
        let _ = self.out.flush();
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn drawn(bytes: &[u8]) -> String {
        String::from_utf8_lossy(bytes).into_owned()
    }

    /// The silent reporter is the whole of the non-terminal path, and it writes
    /// nothing anywhere — there is no stream for it to get wrong.
    #[test]
    fn the_silent_reporter_has_nowhere_to_write() {
        let mut silent = Silent;
        silent.begin(3);
        silent.started("api:lint");
        silent.finished("api:lint", Status::Pass);
        silent.tick();
        silent.finish();
    }

    #[test]
    fn a_spinner_reports_the_count_and_what_is_running() {
        let mut buffer = Vec::new();
        let mut spinner = Spinner::new(&mut buffer, Style::plain(), Terminal::at(80));
        spinner.begin(3);
        spinner.started("api:lint");
        spinner.started("web:e2e");
        spinner.finished("api:lint", Status::Pass);

        let text = drawn(&buffer);
        assert!(text.contains("0/3"), "{text}");
        assert!(text.contains("api:lint, web:e2e"), "{text}");
        assert!(text.contains("1/3  web:e2e"), "{text}");
    }

    /// Every frame starts by erasing the line, so two draws never leave the tail
    /// of the longer one behind.
    #[test]
    fn every_frame_returns_to_the_start_and_erases() {
        let mut buffer = Vec::new();
        let mut spinner = Spinner::new(&mut buffer, Style::plain(), Terminal::at(80));
        spinner.begin(1);
        spinner.started("a-very-long-check-id-indeed");
        spinner.finished("a-very-long-check-id-indeed", Status::Pass);
        let text = drawn(&buffer);
        assert_eq!(
            text.matches("\r\x1b[K").count(),
            3,
            "one erase per draw: {text:?}"
        );
    }

    /// **It leaves nothing behind.** The run's real output arrives on a stream
    /// with no half-drawn frame on it.
    #[test]
    fn finishing_erases_the_line_it_was_drawing_on() {
        let mut buffer = Vec::new();
        let mut spinner = Spinner::new(&mut buffer, Style::plain(), Terminal::at(80));
        spinner.begin(1);
        spinner.finish();
        assert!(drawn(&buffer).ends_with("\r\x1b[K"));
    }

    /// A check that never entered the running set — skipped before it was
    /// spawned — must not advance the count past the total.
    #[test]
    fn a_verdict_for_something_never_started_does_not_move_the_count() {
        let mut buffer = Vec::new();
        let mut spinner = Spinner::new(&mut buffer, Style::plain(), Terminal::at(80));
        spinner.begin(1);
        spinner.finished("api:skipped", Status::Skipped);
        spinner.finished("api:skipped", Status::Skipped);
        assert!(drawn(&buffer).contains("0/1"));
    }

    /// The frame advances on a schedule, not on every turn: the loop polls
    /// children every twenty milliseconds, and a spinner at that rate is
    /// flicker.
    #[test]
    fn the_frame_advances_once_every_few_turns() {
        let mut buffer = Vec::new();
        let mut spinner = Spinner::new(&mut buffer, Style::plain(), Terminal::at(80));
        spinner.begin(1);
        for _ in 0..TURNS_PER_FRAME * 2 {
            spinner.tick();
        }
        let text = drawn(&buffer);
        assert!(text.contains(FRAMES[1]), "{text:?}");
        assert!(text.contains(FRAMES[2]), "{text:?}");
        assert!(!text.contains(FRAMES[3]), "it advanced too fast: {text:?}");
    }

    /// A narrow terminal loses the tail of the running list rather than wrapping
    /// into a second line the erase cannot reach.
    #[test]
    fn a_narrow_terminal_truncates_the_line_rather_than_wrapping_it() {
        let mut buffer = Vec::new();
        let mut spinner = Spinner::new(&mut buffer, Style::plain(), Terminal::at(40));
        spinner.begin(9);
        for id in ["alpha:lint", "beta:lint", "gamma:lint", "delta:lint"] {
            spinner.started(id);
        }
        for line in drawn(&buffer).split('\r') {
            let visible = line.replace("\x1b[K", "");
            assert!(visible.chars().count() <= 40, "{visible:?}");
        }
    }
}
