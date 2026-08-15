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

use armada_core::envelope::Asked;

/// Who answers the interview.
pub trait Ask {
    /// Put one question. `None` means the default was taken.
    fn question(&mut self, asked: &Asked) -> Option<String>;

    /// Put the one multiple-choice question, and return the **one-based** index
    /// of the answer.
    fn choose(&mut self, question: &str, options: &[&str], default: usize) -> usize;
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

    fn choose(&mut self, _: &str, _: &[&str], default: usize) -> usize {
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

    fn choose(&mut self, _: &str, _: &[&str], default: usize) -> usize {
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
}

impl<W: std::io::Write, R: std::io::BufRead> AtTheTerminal<W, R> {
    /// Ask through these two streams.
    pub fn new(prompt: W, input: R, style: crate::render::style::Style, width: usize) -> Self {
        AtTheTerminal {
            prompt,
            input,
            style,
            width,
        }
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
        self.put(&prompt).filter(|answer| !answer.is_empty())
    }

    fn choose(&mut self, question: &str, options: &[&str], default: usize) -> usize {
        let prompt = crate::render::guild_question(question, options, self.style);
        // **An unreadable answer takes the default rather than re-asking.** A
        // loop here is a loop a piped stdin cannot escape, and the default is a
        // documented, working outcome for every question in this interview.
        self.put(&prompt)
            .and_then(|answer| answer.parse::<usize>().ok())
            .filter(|chosen| *chosen >= 1 && *chosen <= options.len())
            .unwrap_or(default)
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
            hint: "enter keeps what import found".to_string(),
            standing: Some("Lead with the answer. Keep it short.".to_string()),
        }
    }

    /// `--defaults` answers nothing, and that is not a failure.
    #[test]
    fn defaults_takes_every_default() {
        assert_eq!(Defaults.question(&asked()), None);
        assert_eq!(Defaults.choose("q", &["a", "b", "c"], 3), 3);
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
        assert_eq!(terminal("").choose("q", &["a", "b", "c"], 3), 3);
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
            assert_eq!(terminal(input).choose("q", &["a", "b", "c"], 3), 3);
        }
        assert_eq!(terminal("1\n").choose("q", &["a", "b", "c"], 3), 1);
    }
}
