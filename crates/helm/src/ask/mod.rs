//! Asking the person a question — the fourth thing the outside world does to
//! Armada, and the only verb that needs it.
//!
//! # Why this is not a fourth seam on `Ctx`
//!
//! `ARCHITECTURE.md` §1.1 fixes the count at three — `run`, `now`, `fetch` —
//! and the count is a decision rather than an accident. An interview is not
//! slow, nondeterministic or external in the way those are: it is one verb's
//! collaborator, needed by `armada init` and by nothing else, ever. So it
//! travels the way `check`'s progress reporter already does, as a `&mut dyn`
//! passed to the one verb that takes it — which is the precedent this follows
//! rather than a new pattern.
//!
//! # Prompts go to stderr, and that is the same rule as the spinner
//!
//! `docs/commands/render.md`: *progress goes to stderr, always*, because a
//! frame of animation on stdout breaks the one consumer the envelope exists
//! for. A prompt is progress — it is the live half of a conversation — and
//! stdout carries the finished transcript once, at the end, in the shape
//! `tests/golden/render/init-machine.plain` froze.
//!
//! That also makes `armada init --json` work without a special case: the
//! questions appear on the terminal, the payload appears on stdout, and neither
//! is in the other's way.

pub mod buffer;
pub mod editor;
pub mod select;
pub mod terminal;

use armada_core::envelope::Asked;
pub use select::Choice;

/// Who answers a question Armada puts.
pub trait Ask {
    /// Put one question. `None` means the default was taken.
    fn question(&mut self, asked: &Asked) -> Option<String>;

    /// Put a closed question, and return the **one-based** index of the answer.
    ///
    /// **Every multiple-choice question in Armada comes through here**, which is
    /// the point: `armada init`'s *do you already have a guild?* and `config
    /// scan`'s hand-over were two printed lists that each waited silently on
    /// stdin, and fixing one of them would have left the other.
    fn choose(&mut self, question: &str, options: &[Choice], default: usize) -> usize;
}

/// Take the default answer to everything.
///
/// **What `--defaults` uses, and what every test uses.** A skipped interview
/// leaves a *working* guild — import has already written the fragments and the
/// starters are already copied — and `armada doctor` reports which fragments
/// are still whatever import produced (`PLAN.md` §13.4).
#[derive(Debug, Clone, Copy, Default)]
pub struct Defaults;

impl Ask for Defaults {
    fn question(&mut self, _: &Asked) -> Option<String> {
        None
    }

    fn choose(&mut self, _: &str, _: &[Choice], default: usize) -> usize {
        default
    }
}

/// Answers decided in advance — for a test that wants the answered path.
#[derive(Debug, Clone, Default)]
pub struct Scripted {
    /// One answer per question, in order. A `None` takes the default.
    pub answers: Vec<Option<String>>,
    /// What the one multiple-choice question is answered with.
    pub choice: Option<usize>,
    /// Every prompt that was put, in order — so a test can assert the interview
    /// asked what it claims to ask.
    pub asked: Vec<Asked>,
}

impl Ask for Scripted {
    fn question(&mut self, asked: &Asked) -> Option<String> {
        self.asked.push(asked.clone());
        if self.answers.is_empty() {
            return None;
        }
        self.answers.remove(0)
    }

    fn choose(&mut self, _: &str, _: &[Choice], default: usize) -> usize {
        self.choice.unwrap_or(default)
    }
}

/// A person, at a terminal.
///
/// Writes the prompt to `stderr` and reads the answer from `stdin`. **An empty
/// line is the default**, which is what every hint on every question promises.
pub struct AtTheTerminal<W: std::io::Write, R: std::io::BufRead> {
    prompt: W,
    input: R,
    style: crate::render::style::Style,
    width: usize,
    surface: Surface,
}

/// Whether this invocation may draw a widget.
///
/// **One flag for both widgets, because they answer one question**: is a person
/// here who can see a box and press a key. The text area and the selector had no
/// business disagreeing about that, and two flags is how they would eventually
/// have.
///
/// **Two, because one of them cannot be tested and the other is most of the
/// behaviour.** [`Surface::Widgets`] takes over the terminal — raw mode, an
/// inline viewport, real key events — and no test that runs without a TTY can
/// drive it. [`Surface::Lines`] reads through the same streams as everything
/// else, which is what the whole `Ask` suite uses, what a terminal that refuses
/// raw mode falls back to, and what a run with no terminal gets.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Surface {
    /// The inline text area of [`editor`] and the selector of [`select`].
    Widgets,
    /// Printed prompts and a line of stdin, like every other question.
    Lines,
}

impl<W: std::io::Write, R: std::io::BufRead> AtTheTerminal<W, R> {
    /// Ask through these two streams, in lines.
    pub fn new(prompt: W, input: R, style: crate::render::style::Style, width: usize) -> Self {
        AtTheTerminal {
            prompt,
            input,
            style,
            width,
            surface: Surface::Lines,
        }
    }

    /// Draw widgets: the text area for prose, the selector for a closed
    /// question.
    ///
    /// **Only `main` calls this, and only when both stdin and stdout are a
    /// terminal** — the same rule `armada_core::scan::handover` applies, and for
    /// the same reason. stdin decides whether an answer can arrive and stdout
    /// decides whether the question was seen; an agent that met a widget on
    /// either would block on stdin that never comes.
    pub fn interactive(mut self) -> Self {
        self.surface = Surface::Widgets;
        self
    }

    /// Write a prompt and read one line back. `None` at end of input.
    ///
    /// **Flushed before the read**, because the prompt does not end in a
    /// newline: the caret is where the cursor sits, and an unflushed prompt is a
    /// program that appears to have hung.
    fn put(&mut self, prompt: &str) -> Option<String> {
        let _ = self.prompt.write_all(prompt.as_bytes());
        let _ = self.prompt.flush();
        let mut line = String::new();
        match self.input.read_line(&mut line) {
            Ok(0) | Err(_) => None,
            Ok(_) => Some(line.trim().to_string()),
        }
    }
}

impl<W: std::io::Write, R: std::io::BufRead> Ask for AtTheTerminal<W, R> {
    /// **A blank line above every question.** Five of these with nothing between
    /// them ran together on the one run that mattered — the answers scroll past
    /// as you type them, so the only thing separating one question from the next
    /// is the space you put there.
    fn question(&mut self, asked: &Asked) -> Option<String> {
        let prompt = format!(
            "\n{}",
            crate::render::interview_prompt(asked, self.style, self.width)
        );
        if asked.prose && self.surface == Surface::Widgets {
            let _ = self.prompt.write_all(prompt.as_bytes());
            let _ = self.prompt.flush();
            return match editor::read(self.style, asked.standing.as_deref()) {
                editor::Answer::Given(text) => Some(text),
                editor::Answer::Kept => None,
            };
        }
        self.put(&prompt).filter(|answer| !answer.is_empty())
    }

    /// **The selector when a person is here; the printed list otherwise.**
    ///
    /// The list is not a lesser copy kept around out of caution — it is the form
    /// an agent reading stdout gets, and the form a terminal that will not go
    /// into raw mode falls back to. Both are real audiences, and neither may
    /// block on a widget it cannot see.
    fn choose(&mut self, question: &str, options: &[Choice], default: usize) -> usize {
        if self.surface == Surface::Widgets {
            if let Some(chosen) = select::ask(question, options, default, self.style) {
                self.echo(question, options, chosen);
                return chosen;
            }
        }
        let prompt = crate::render::choice_list(question, options, default, self.style);
        // **An unreadable answer takes the default rather than re-asking.** A
        // loop here is a loop a piped stdin cannot escape, and the default is a
        // documented, working outcome for every question Armada asks.
        let chosen = self
            .put(&prompt)
            .and_then(|answer| answer.parse::<usize>().ok())
            .filter(|chosen| *chosen >= 1 && *chosen <= options.len())
            .unwrap_or(default);
        self.echo(question, options, chosen);
        chosen
    }
}

impl<W: std::io::Write, R: std::io::BufRead> AtTheTerminal<W, R> {
    /// Write what was picked, where the question was put.
    ///
    /// **A selector that clears itself leaves no record of the decision.** The
    /// widget's viewport is gone the moment it closes, so without this the
    /// scrollback shows a report, a gap, and whatever happened next — and the
    /// one line saying which of the options produced it is nowhere.
    fn echo(&mut self, question: &str, options: &[Choice], chosen: usize) {
        let Some(choice) = options.get(chosen.saturating_sub(1)) else {
            return;
        };
        let line = crate::render::chosen_line(question, &choice.label, chosen, self.style);
        let _ = self.prompt.write_all(line.as_bytes());
        let _ = self.prompt.flush();
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::render::style::Style;

    fn asked() -> Asked {
        Asked {
            number: 1,
            of: 5,
            prompt: "How should agents write to you?".to_string(),
            purpose: "Tone, length, and what to lead with.".to_string(),
            writes: "voice.md".to_string(),
            keeps: "what import found".to_string(),
            prose: false,
            standing: Some("Lead with the answer. Keep it short.".to_string()),
        }
    }

    fn three() -> Vec<Choice> {
        vec![
            Choice::new("pull from a remote", "clones it"),
            Choice::bare("import a bundle"),
            Choice::bare("build one now"),
        ]
    }

    /// `--defaults` answers nothing, and that is not a failure.
    #[test]
    fn defaults_takes_every_default() {
        assert_eq!(Defaults.question(&asked()), None);
        assert_eq!(Defaults.choose("q", &three(), 3), 3);
    }

    fn terminal(input: &str) -> AtTheTerminal<Vec<u8>, std::io::BufReader<&[u8]>> {
        AtTheTerminal::new(
            Vec::new(),
            std::io::BufReader::new(input.as_bytes()),
            Style::plain(),
            80,
        )
    }

    /// **An empty line is the default**, which is what every hint promises.
    #[test]
    fn pressing_enter_takes_the_default() {
        assert_eq!(terminal("\n").question(&asked()), None);
        assert_eq!(terminal("   \n").question(&asked()), None);
        assert_eq!(
            terminal("brief, answer first\n").question(&asked()),
            Some("brief, answer first".to_string())
        );
    }

    /// End of input is a default too. A `armada init < /dev/null` is a scripted
    /// setup, not a hang.
    #[test]
    fn end_of_input_takes_the_default_rather_than_hanging() {
        assert_eq!(terminal("").question(&asked()), None);
        assert_eq!(terminal("").choose("q", &three(), 3), 3);
    }

    /// **Everything a question needs, in the order it is read**, and the caret
    /// is the last thing on the line — the cursor sits after it.
    ///
    /// Each of the four parts was asked for by name after a real first run: the
    /// blank line that keeps five questions from running together, the purpose
    /// and the file so the question says what answer it wants, and `now` so the
    /// default is one you can see before you accept it.
    #[test]
    fn a_question_says_what_it_wants_shows_what_enter_keeps_and_stands_apart() {
        let mut ask = terminal("3\n");
        ask.question(&asked());
        let written = String::from_utf8(ask.prompt).unwrap();
        assert_eq!(
            written,
            "\n1/5  How should agents write to you?\n     \
             Tone, length, and what to lead with. Writes voice.md.\n\n     \
             now  Lead with the answer. Keep it short.\n     \
             enter keeps what import found  > "
        );
    }

    /// A standing value too long for the line is cut rather than wrapped: it is
    /// a reminder of what is on disk, not the file.
    #[test]
    fn a_long_standing_value_is_cut_to_one_line() {
        let mut ask = terminal("\n");
        ask.question(&Asked {
            standing: Some("x".repeat(200)),
            ..asked()
        });
        let written = String::from_utf8(ask.prompt).unwrap();
        for line in written.lines() {
            assert!(line.chars().count() <= 80, "{line:?}");
        }
        assert!(written.contains('…'), "the cut is marked: {written}");
    }

    /// An answer outside the three takes the default rather than re-asking: a
    /// loop is a loop a piped stdin cannot escape.
    #[test]
    fn an_unreadable_choice_takes_the_default() {
        for input in ["9\n", "yes\n", "\n"] {
            assert_eq!(terminal(input).choose("q", &three(), 3), 3);
        }
        assert_eq!(terminal("1\n").choose("q", &three(), 1), 1);
    }

    /// **The printed form says what each option does and what enter takes.**
    ///
    /// It replaces one line of numbers ending at a bare caret, which is the
    /// thing a real reader could not interpret — *"just stopping and me guessing
    /// it's asking me to input a number"*. This is what an agent reading stdout
    /// gets and what a terminal that refuses raw mode falls back to, so it has
    /// to stand on its own.
    #[test]
    fn the_printed_list_gives_every_option_a_line_and_says_what_enter_takes() {
        let mut ask = terminal("2\n");
        ask.choose("Do you already have a guild?", &three(), 3);
        let written = String::from_utf8(ask.prompt).unwrap();
        assert_eq!(
            written,
            "Do you already have a guild?\n\
             \x20 1  pull from a remote  clones it\n\
             \x20 2  import a bundle\n\
             \x20 3  build one now\n\
             \x20 a number, or enter for 3  > \
             Do you already have a guild?  > 2 import a bundle\n"
        );
    }

    /// **What was picked is echoed, whichever way it was picked.** A selector
    /// clears its own viewport, so without this the scrollback shows a report, a
    /// gap, and no record of the decision that produced what follows.
    #[test]
    fn the_chosen_option_is_echoed_for_the_scrollback() {
        let mut ask = terminal("\n");
        assert_eq!(ask.choose("q", &three(), 3), 3);
        let written = String::from_utf8(ask.prompt).unwrap();
        assert!(written.ends_with("q  > 3 build one now\n"), "{written:?}");
    }
}
