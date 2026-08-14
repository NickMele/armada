//! What the terminal is: whether each stream is one, and how wide.
//!
//! **Read once, in `main`, and passed down.** `ARCHITECTURE.md` §1.4 keeps every
//! reach for the ambient world in the entrypoint, and a terminal is ambient in
//! exactly the way `$HOME` is: it differs per invocation, it is invisible in a
//! function signature, and a renderer that asks for it directly cannot be tested
//! at a width other than the developer's.
//!
//! **Two streams, asked separately, and the distinction matters.** Colour and
//! table width are stdout's business; progress is stderr's (PLAN.md §3.1.1).
//! `armada manifest check | jq` has a piped stdout and a terminal stderr, and it
//! must produce an unstyled payload *and* a live spinner — one flag could not
//! express that.

use std::io::IsTerminal;

/// The terminal, as one answer.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Terminal {
    /// Whether stdout goes to a terminal. Decides colour and truncation.
    pub stdout_is_tty: bool,
    /// Whether stderr goes to a terminal. Decides progress.
    pub stderr_is_tty: bool,
    /// Columns available for output.
    pub width: usize,
}

impl Terminal {
    /// What a stream that is not a terminal is assumed to be.
    ///
    /// **80, which is a convention rather than a measurement**, and it is the
    /// right one here: it is what every other tool assumes, so a captured log
    /// wraps the same way whoever produced it. The alternative — unbounded, so
    /// nothing ever truncates — makes a piped `status` and a terminal `status`
    /// two different renders, which is exactly what §3.1.1 forbids.
    pub const FALLBACK_WIDTH: usize = 80;

    /// Narrower than this and a table has nothing left to give.
    ///
    /// Below it, truncation would eat the columns that carry the answer, so the
    /// renderer stops shrinking and lets the line overhang instead — an
    /// overhanging line is ugly, a line truncated to `ap…` is wrong.
    pub const MIN_WIDTH: usize = 40;

    /// Ask the operating system. **`main` only.**
    pub fn detect() -> Terminal {
        Terminal {
            stdout_is_tty: std::io::stdout().is_terminal(),
            stderr_is_tty: std::io::stderr().is_terminal(),
            width: terminal_size::terminal_size()
                .map(|(terminal_size::Width(columns), _)| columns as usize)
                .filter(|columns| *columns > 0)
                .unwrap_or(Terminal::FALLBACK_WIDTH),
        }
    }

    /// Neither stream is a terminal — what a pipe, a captured log and an agent
    /// reading stdout all share. The default for a test.
    pub const fn piped() -> Terminal {
        Terminal {
            stdout_is_tty: false,
            stderr_is_tty: false,
            width: Terminal::FALLBACK_WIDTH,
        }
    }

    /// Both streams are a terminal, this many columns wide. For a test that
    /// wants to see what a person sees.
    pub const fn at(width: usize) -> Terminal {
        Terminal {
            stdout_is_tty: true,
            stderr_is_tty: true,
            width,
        }
    }

    /// The width a table may use, never below [`Terminal::MIN_WIDTH`].
    pub const fn usable_width(self) -> usize {
        if self.width < Terminal::MIN_WIDTH {
            Terminal::MIN_WIDTH
        } else {
            self.width
        }
    }
}

/// How many columns a string occupies, for the purpose of aligning a column.
///
/// **Characters, not grapheme clusters and not East Asian width.** Getting that
/// right needs a Unicode width table, which is a dependency
/// `docs/commands/render.md` rules out — and the strings being aligned here are
/// check ids, workspace ids, ports, verdicts and paths, which are ASCII in every
/// case Armada produces. A CJK path in a `status` listing will align one column
/// short per wide character; that is a stated limit rather than a surprise.
///
/// **An escape sequence counts as nothing**, which is what makes it safe to
/// measure a cell after painting it — though the table renderer measures before,
/// so this is a guard rather than a strategy.
pub fn display_width(text: &str) -> usize {
    let mut width = 0;
    let mut chars = text.chars();
    while let Some(c) = chars.next() {
        if c != '\x1b' {
            width += 1;
            continue;
        }
        for c in chars.by_ref() {
            if c == 'm' {
                break;
            }
        }
    }
    width
}

/// `text`, cut to `width` columns with an ellipsis when it does not fit.
///
/// **Truncated, never wrapped** (PHASES.md §8.3.1). A wrapped row turns one
/// answer into two lines that no longer line up with the header, and the reader
/// loses the column that made the table worth having; a truncated cell loses the
/// tail of one value and keeps the shape.
///
/// The ellipsis is one character and is included in `width`, so the result is
/// never wider than asked. A `width` of 0 yields nothing at all.
pub fn truncate(text: &str, width: usize) -> String {
    if display_width(text) <= width {
        return text.to_string();
    }
    match width {
        0 => String::new(),
        // No room for content plus a marker; the marker alone at least says
        // "there was more here".
        1 => "…".to_string(),
        _ => {
            let kept: String = text.chars().take(width - 1).collect();
            format!("{kept}…")
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_piped_stream_is_eighty_columns_and_neither_stream_is_a_terminal() {
        let piped = Terminal::piped();
        assert!(!piped.stdout_is_tty && !piped.stderr_is_tty);
        assert_eq!(piped.width, 80);
    }

    /// A terminal narrower than the floor is treated as the floor. Shrinking
    /// past it would truncate the columns that carry the answer.
    #[test]
    fn a_terminal_narrower_than_the_floor_is_widened_to_it() {
        assert_eq!(Terminal::at(20).usable_width(), Terminal::MIN_WIDTH);
        assert_eq!(Terminal::at(120).usable_width(), 120);
    }

    #[test]
    fn an_escape_sequence_occupies_no_columns() {
        assert_eq!(display_width("PASS"), 4);
        assert_eq!(display_width("\x1b[38;2;61;220;132mPASS\x1b[0m"), 4);
        assert_eq!(display_width(""), 0);
    }

    #[test]
    fn truncation_never_exceeds_the_width_it_was_given() {
        assert_eq!(truncate("api:lint", 20), "api:lint");
        assert_eq!(truncate("api:lint", 8), "api:lint");
        assert_eq!(truncate("api:lint", 5), "api:…");
        assert_eq!(truncate("api:lint", 1), "…");
        assert_eq!(truncate("api:lint", 0), "");
        for width in 0..12 {
            assert!(display_width(&truncate("api:lint", width)) <= width);
        }
    }
}
