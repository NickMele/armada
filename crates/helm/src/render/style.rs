//! **The colour decision, made once.**
//!
//! Every other file in this module asks a [`Style`] whether to paint; none of
//! them reads an environment variable, calls `isatty`, or looks at a flag. That
//! is the whole point: the moment a second place decides, one surface comes out
//! coloured under `| less` and another does not.
//!
//! ```text
//! --color auto     colour when stdout is a TTY and NO_COLOR is unset   (default)
//! --color always   colour even when piped — for a pager
//! --color never    never; the same bytes a non-TTY gets
//! NO_COLOR         no colour, whatever its value, whatever --color says
//! ```

use super::palette::{Role, BOLD, RESET};

/// What the caller asked for. `--color auto|always|never`.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum ColorChoice {
    /// Colour when stdout is a TTY and `NO_COLOR` is unset.
    #[default]
    Auto,
    /// Colour regardless of where stdout goes.
    Always,
    /// No colour, ever.
    Never,
}

impl ColorChoice {
    /// Parse the value of `--color`, or `None` for a word that is not one.
    pub fn parse(word: &str) -> Option<ColorChoice> {
        match word {
            "auto" => Some(ColorChoice::Auto),
            "always" => Some(ColorChoice::Always),
            "never" => Some(ColorChoice::Never),
            _ => None,
        }
    }

    /// The three spellings, for an error message that lists them.
    pub const VALUES: [&'static str; 3] = ["auto", "always", "never"];
}

/// The decision, and the only thing that may paint.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Style {
    color: bool,
}

impl Style {
    /// **`NO_COLOR` wins, including over `--color always`.**
    ///
    /// The two rules genuinely conflict and one has to lose, so here is the
    /// argument. `NO_COLOR` is set by a person who has already decided this
    /// question for every tool on their machine — often because a terminal, a
    /// screen reader or a log collector renders escapes as garbage rather than
    /// as colour. `--color always` is a convenience for pushing colour through a
    /// pager. Losing the convenience costs one `NO_COLOR= armada …` on the rare
    /// invocation that wants it; winning it costs escape codes in the one
    /// environment that was explicitly declared unable to render them.
    ///
    /// `docs/commands/render.md` lists `NO_COLOR` as a peer of the three
    /// `--color` values rather than as a footnote to `auto`, and PLAN.md §3.1.1
    /// says arguing with it "costs a bug report from someone who set it
    /// deliberately". This is the reading that makes both sentences true.
    ///
    /// `no_color` is passed in rather than read here, because
    /// `ARCHITECTURE.md` §1.4 keeps every read of the ambient world in `main`.
    pub fn decide(choice: ColorChoice, stdout_is_tty: bool, no_color: bool) -> Style {
        if no_color {
            return Style { color: false };
        }
        Style {
            color: match choice {
                ColorChoice::Always => true,
                ColorChoice::Never => false,
                ColorChoice::Auto => stdout_is_tty,
            },
        }
    }

    /// No colour at all — what a test, a pipe and `--color never` share.
    pub const fn plain() -> Style {
        Style { color: false }
    }

    /// Colour, unconditionally. For a test that wants to see the escapes.
    pub const fn painted() -> Style {
        Style { color: true }
    }

    /// Whether anything will be painted.
    pub const fn enabled(self) -> bool {
        self.color
    }

    /// `text`, in `role`'s colour — or exactly `text` when colour is off.
    ///
    /// **The uncoloured branch returns the same characters, never fewer.** The
    /// non-TTY audience gets the same columns, order and words as the TTY one
    /// (PLAN.md §3.1.1); if styling changed the text, the two would be different
    /// renderers wearing one name.
    pub fn paint(self, role: Role, text: &str) -> String {
        if self.color {
            format!("{}{text}{RESET}", role.fg())
        } else {
            text.to_string()
        }
    }

    /// `text`, bold and in `role`'s colour.
    pub fn strong(self, role: Role, text: &str) -> String {
        if self.color {
            format!("{BOLD}{}{text}{RESET}", role.fg())
        } else {
            text.to_string()
        }
    }

    /// What stands in a cell that has nothing in it.
    ///
    /// **Typography follows the same decision colour does, and for the same
    /// reason.** Both answer "is a person looking at this": an em dash is
    /// prettier and an ASCII hyphen is what survives a caller who splits the
    /// line and compares the field. The agent audience gets the ASCII form of
    /// every decorative character, which is why this is one method rather than
    /// a literal at forty call sites.
    ///
    /// **Both are one column wide**, so the two renders align identically — the
    /// property `render.rs`'s tests assert.
    pub fn nothing(self) -> &'static str {
        if self.color {
            "—"
        } else {
            "-"
        }
    }

    /// The marker on a line that says what to do about the line above it.
    ///
    /// **A fix line is the point of a report** — a check that names a problem
    /// without the command that fixes it sends the reader to the documentation,
    /// which is most of what reporting exists to save. So the glyph follows the
    /// same decision every other one does: typographic for a person, ASCII for
    /// the agent that might re-parse the line (`render_pending.rs`).
    ///
    /// **Not one column for one column**, unlike [`Style::nothing`] and
    /// [`Style::span`], which is why it may only ever open a line and never sit
    /// in a table cell: a two-column arrow inside a measured cell would shear
    /// every column after it.
    pub fn arrow(self) -> &'static str {
        if self.color {
            "→"
        } else {
            "->"
        }
    }

    /// A span, written the way the reader's audience writes one.
    ///
    /// An en dash between two numbers for a person; a hyphen for anyone who
    /// might feed `5460-5469` back into something.
    pub fn span(self, from: impl std::fmt::Display, to: impl std::fmt::Display) -> String {
        if self.color {
            format!("{from}–{to}")
        } else {
            format!("{from}-{to}")
        }
    }

    /// The separator between two facts on a summary line, spaces included.
    ///
    /// **A summary line is prose, not a column**, so this is the one place the
    /// two renders may differ in width — a middle dot reads better than a comma
    /// and a comma is what an agent's `split(", ")` expects.
    pub fn between(self) -> &'static str {
        if self.color {
            " · "
        } else {
            ", "
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn auto_paints_at_a_terminal_and_not_through_a_pipe() {
        assert!(Style::decide(ColorChoice::Auto, true, false).enabled());
        assert!(!Style::decide(ColorChoice::Auto, false, false).enabled());
    }

    #[test]
    fn always_paints_through_a_pipe_and_never_paints_at_a_terminal() {
        assert!(Style::decide(ColorChoice::Always, false, false).enabled());
        assert!(!Style::decide(ColorChoice::Never, true, false).enabled());
    }

    /// The documented tie-break: `NO_COLOR` beats every `--color`, including
    /// `always`. See [`Style::decide`] for why.
    #[test]
    fn no_color_wins_over_every_color_choice() {
        for choice in [ColorChoice::Auto, ColorChoice::Always, ColorChoice::Never] {
            for tty in [true, false] {
                assert!(
                    !Style::decide(choice, tty, true).enabled(),
                    "{choice:?} on tty={tty} painted despite NO_COLOR"
                );
            }
        }
    }

    /// `NO_COLOR=0` is still `NO_COLOR`. The variable's *presence* is the
    /// signal, per the standard — which is why this function takes a `bool` the
    /// caller computed from presence rather than a string it might parse.
    #[test]
    fn painting_off_emits_the_same_characters_and_no_escapes() {
        let plain = Style::plain();
        assert_eq!(plain.paint(Role::DistressRed, "FAILED"), "FAILED");
        assert_eq!(plain.strong(Role::SignalAmber, "FAILED"), "FAILED");
        assert!(!plain.paint(Role::DistressRed, "FAILED").contains('\x1b'));
    }

    #[test]
    fn painting_on_wraps_the_same_characters_and_always_resets() {
        let painted = Style::painted().paint(Role::BeaconGreen, "PASS");
        assert!(painted.starts_with(Role::BeaconGreen.fg()));
        assert!(painted.ends_with(RESET));
        assert!(painted.contains("PASS"));
    }

    /// **Decoration is ASCII for the audience that might re-parse it**, and the
    /// two forms occupy the same number of columns so a table drawn either way
    /// aligns identically.
    #[test]
    fn a_placeholder_and_a_span_are_typographic_for_a_person_and_ascii_otherwise() {
        let person = Style::painted();
        let agent = Style::plain();

        assert_eq!(person.nothing(), "—");
        assert_eq!(agent.nothing(), "-");
        assert_eq!(
            person.nothing().chars().count(),
            agent.nothing().chars().count()
        );

        assert_eq!(person.span(5460, 5469), "5460–5469");
        assert_eq!(agent.span(5460, 5469), "5460-5469");
        assert_eq!(
            person.span(5460, 5469).chars().count(),
            agent.span(5460, 5469).chars().count()
        );
    }

    #[test]
    fn the_three_spellings_are_the_only_ones() {
        assert_eq!(ColorChoice::parse("auto"), Some(ColorChoice::Auto));
        assert_eq!(ColorChoice::parse("always"), Some(ColorChoice::Always));
        assert_eq!(ColorChoice::parse("never"), Some(ColorChoice::Never));
        assert_eq!(ColorChoice::parse("yes"), None);
        assert_eq!(
            ColorChoice::parse("Always"),
            None,
            "one spelling, lowercase"
        );
    }
}
