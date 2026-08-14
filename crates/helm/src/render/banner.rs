//! The wordmark, and the two places it is allowed to appear.
//!
//! ANSI Shadow block letters spelling ARMADA, six lines and 51 columns, in
//! signal amber. The glyphs are copied verbatim from the fenced block in
//! `docs/commands/render.md` — that page is the source, and a regenerated
//! wordmark is a different wordmark.
//!
//! **Two places, both moments of orientation rather than work**: bare `armada`,
//! and `armada init`. Not on a verb — a banner on a verb is a banner in the way
//! — and not on `--help`, which is the page you reach for when you are in a
//! hurry.
//!
//! **Never on non-TTY stdout, and that is the rule that matters.** An agent
//! running `armada init` in a worktree reads stdout, and six lines of block
//! characters at the top of it is noise it has to learn to skip. Same reasoning
//! as progress-on-stderr: anything decorative is for the person, not the parser.
//!
//! **Suppressed below 51 columns rather than wrapped.** A wrapped wordmark is
//! worse than no wordmark, and a terminal that narrow has more pressing
//! problems.
//!
//! **`armada init` is M2 and does not exist yet** (`args.rs` reserves the name),
//! so today the wordmark has exactly one live site: bare `armada`. Every
//! suppression rule lives in [`banner`] rather than at that call site, so the
//! second site inherits them instead of re-deciding them.

use super::palette::Role;
use super::style::Style;
use super::term::Terminal;

/// The columns the wordmark needs. Narrower than this and it is not drawn.
pub const WIDTH: usize = 51;

/// ANSI Shadow, spelling ARMADA. Verbatim from `docs/commands/render.md`.
const ARMADA: [&str; 6] = [
    " █████╗ ██████╗ ███╗   ███╗ █████╗ ██████╗  █████╗",
    "██╔══██╗██╔══██╗████╗ ████║██╔══██╗██╔══██╗██╔══██╗",
    "███████║██████╔╝██╔████╔██║███████║██║  ██║███████║",
    "██╔══██║██╔══██╗██║╚██╔╝██║██╔══██║██║  ██║██╔══██║",
    "██║  ██║██║  ██║██║ ╚═╝ ██║██║  ██║██████╔╝██║  ██║",
    "╚═╝  ╚═╝╚═╝  ╚═╝╚═╝     ╚═╝╚═╝  ╚═╝╚═════╝ ╚═╝  ╚═╝",
];

/// The wordmark, or nothing at all.
///
/// **Every suppression rule is here, in one function**, so a second caller
/// cannot draw it under conditions the first refuses. `style.enabled()` covers
/// `--color never` and `NO_COLOR` at once — a reader who turned colour off did
/// not ask for an amber picture, and one who set `NO_COLOR` may not be able to
/// render it at all.
pub fn banner(style: Style, terminal: Terminal) -> String {
    if !terminal.stdout_is_tty || !style.enabled() || terminal.width < WIDTH {
        return String::new();
    }
    let mut out = String::new();
    for line in ARMADA {
        out.push_str(&style.paint(Role::SignalAmber, line));
        out.push('\n');
    }
    out.push('\n');
    out
}

#[cfg(test)]
mod tests {
    use super::super::term::display_width;
    use super::*;

    #[test]
    fn the_wordmark_is_six_lines_of_fifty_one_columns() {
        assert_eq!(ARMADA.len(), 6);
        assert_eq!(
            ARMADA.iter().map(|l| display_width(l)).max(),
            Some(WIDTH),
            "the wordmark is not {WIDTH} columns wide"
        );
    }

    #[test]
    fn a_terminal_wide_enough_gets_it_in_signal_amber() {
        let drawn = banner(Style::painted(), Terminal::at(120));
        let lines: Vec<&str> = drawn.lines().collect();
        assert_eq!(lines.len(), 7, "six glyph lines and one blank separator");
        assert!(lines[6].is_empty(), "the page below it is not crowded");
        assert!(drawn.contains(Role::SignalAmber.fg()));
    }

    /// **The three suppressions, each on its own.** Any one of them is enough,
    /// because each describes a reader who would not benefit from a picture.
    #[test]
    fn it_is_suppressed_by_a_pipe_by_no_colour_and_by_a_narrow_terminal() {
        assert_eq!(
            banner(Style::painted(), Terminal::piped()),
            "",
            "a captured stdout is an agent's, and this is noise to it"
        );
        assert_eq!(
            banner(Style::plain(), Terminal::at(120)),
            "",
            "--color never and NO_COLOR both mean no amber picture"
        );
        assert_eq!(
            banner(Style::painted(), Terminal::at(WIDTH - 1)),
            "",
            "below {WIDTH} columns it would wrap, which is worse than absent"
        );
        assert_ne!(banner(Style::painted(), Terminal::at(WIDTH)), "");
    }
}
