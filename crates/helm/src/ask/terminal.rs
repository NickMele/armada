//! The two things every widget in this module needs from the terminal, and
//! neither of them is a widget's business.
//!
//! [`Restore`] owns the process-wide switches — raw mode, bracketed paste — and
//! [`painted`] is how a `ratatui` surface reads Armada's one palette. Both were
//! the text area's until the selector needed them too, which is the moment to
//! move something rather than reach across for it.

use ratatui::crossterm::cursor::MoveTo;
use ratatui::crossterm::event::{DisableBracketedPaste, EnableBracketedPaste};
use ratatui::crossterm::execute;
use ratatui::crossterm::terminal::{disable_raw_mode, enable_raw_mode, Clear, ClearType};
use ratatui::layout::Rect;
use ratatui::style::Style as RataStyle;

use crate::render::palette::Role;
use crate::render::style::Style;

/// One of Armada's roles as a `ratatui` style, or unstyled when colour is off.
///
/// **The palette is not duplicated.** `render/palette.rs` is the one place a
/// colour is chosen (`docs/commands/render.md`), and this reads its RGB rather
/// than naming a second set here.
pub fn painted(style: Style, role: Role) -> RataStyle {
    if !style.enabled() {
        return RataStyle::default();
    }
    let (r, g, b) = role.rgb();
    RataStyle::default().fg(ratatui::style::Color::Rgb(r, g, b))
}

/// Raw mode and bracketed paste, put back on drop **and on panic**.
///
/// Both are process-wide switches. A panic that left raw mode on would leave the
/// person with a shell that does not echo what they type — and the panic message
/// itself would be printed without carriage returns, stepping diagonally down
/// the screen. So the hook restores first and prints second: the one moment the
/// message matters is the one moment it would otherwise be unreadable.
pub struct Restore;

impl Restore {
    /// Take the terminal, or say why not. **A failure is not fatal**: every
    /// caller falls back to a form that needs no raw mode, because a terminal
    /// that will not give it up is still a terminal somebody is sitting at.
    pub fn install() -> Result<Restore, std::io::Error> {
        enable_raw_mode()?;
        if execute!(std::io::stderr(), EnableBracketedPaste).is_err() {
            // A terminal that will not bracket pastes still edits fine; a paste
            // arrives as keystrokes instead. Not worth refusing the interview
            // over.
            let _ = disable_raw_mode();
            enable_raw_mode()?;
        }

        let previous = std::panic::take_hook();
        std::panic::set_hook(Box::new(move |info| {
            let _ = execute!(std::io::stderr(), DisableBracketedPaste);
            let _ = disable_raw_mode();
            previous(info);
        }));

        Ok(Restore)
    }
}

impl Drop for Restore {
    fn drop(&mut self) {
        let _ = execute!(std::io::stderr(), DisableBracketedPaste);
        let _ = disable_raw_mode();
        let _ = std::panic::take_hook();
    }
}

/// Give an inline viewport's rows back, **without asking where the cursor is**.
///
/// `ratatui::Terminal::clear()` asks: for an inline viewport it queries the
/// cursor position so it can restore it afterward, on top of the query
/// `Terminal::with_options` already sent to anchor the viewport in the first
/// place. A query is a write to the terminal and a blocking read of its
/// reply, and the reply can arrive late, not at all, or after something else
/// has already moved on — a `script(1)` session with nothing on the other end
/// to answer it, a slow pty. Answered late, the stale reply is read by
/// whatever asks next and is mistaken for *its* answer, which is how a
/// widget's teardown ends up anchored on coordinates that were never its own:
/// blank lines where the reserved rows still sit in the scrollback, and the
/// next line drawn from wherever that stale reply happened to say the cursor
/// was.
///
/// `area` needs no query because it is already known — every widget that
/// calls this already has it, handed back by its own last successful
/// `Terminal::draw`. Clearing from its top-left down and leaving the cursor
/// there is where a query that got answered in time would have landed
/// anyway: the widget's rows erased, ready for whatever gets written next.
pub fn clear_viewport<W: std::io::Write>(mut stream: W, area: Rect) {
    let _ = execute!(
        stream,
        MoveTo(area.x, area.y),
        Clear(ClearType::FromCursorDown)
    );
}

#[cfg(test)]
mod tests {
    use super::*;

    /// **A palette colour, not a second palette.** `render/palette.rs` is the
    /// one place a colour is chosen, and colour off means no styling at all.
    #[test]
    fn a_widget_paints_from_the_one_palette() {
        assert_eq!(
            painted(Style::plain(), Role::SteelGrey),
            RataStyle::default()
        );
        let (r, g, b) = Role::SteelGrey.rgb();
        assert_eq!(
            painted(Style::painted(), Role::SteelGrey),
            RataStyle::default().fg(ratatui::style::Color::Rgb(r, g, b))
        );
    }
}
