//! The interview's text area — **inline, in the terminal you are already in**.
//!
//! Questions 1 to 3 want paragraphs. A single-line prompt that scrolls sideways
//! is not somewhere anyone writes one, and that is what the first real run of
//! this interview produced: three of the five answers are prose and all three
//! were typed into a box one line high.
//!
//! # Why not `$EDITOR`
//!
//! Considered and rejected. Leaving the terminal to answer question two and
//! coming back for question three is a worse interview than a box that scrolls —
//! it puts a modal application between you and the next question, and on a
//! machine whose `$EDITOR` is unset or is `vi` it puts a puzzle there. The
//! interview is five questions long; it should be five questions long from where
//! you started it.
//!
//! # Why `ratatui`
//!
//! It is already the decided crate for the Bridge (`PHASES.md` §8.5), so this is
//! one dependency doing two jobs rather than a second one. `Viewport::Inline`
//! is the reason it fits here at all: the box is drawn in the last few lines of
//! the terminal and everything above it — the preflight table, the questions
//! already answered — stays in the scrollback where it belongs.
//!
//! # Two things this file must get right
//!
//! **A paste of several paragraphs arrives intact.** Bracketed paste is enabled
//! so the terminal sends it as one event rather than as a burst of keystrokes
//! with carriage returns in it; [`Buffer::paste`] takes it whole. Without that,
//! a pasted blank line between paragraphs is a `\r` that some terminals send as
//! a submit.
//!
//! **The terminal is restored whatever happens.** Raw mode and bracketed paste
//! are process-wide switches, and a panic that left them on would leave the
//! person with a shell that does not echo. [`Restore`] puts them back on drop,
//! and a panic hook puts them back before the message is printed — because the
//! message is unreadable otherwise, which is exactly when you need it.

use ratatui::crossterm::event::{
    self, DisableBracketedPaste, EnableBracketedPaste, Event, KeyCode, KeyEventKind, KeyModifiers,
};
use ratatui::crossterm::execute;
use ratatui::crossterm::terminal::{disable_raw_mode, enable_raw_mode};
use ratatui::layout::{Constraint, Layout};
use ratatui::style::Style as RataStyle;
use ratatui::text::Line;
use ratatui::widgets::{Block, Borders, Paragraph, Wrap};
use ratatui::{Terminal, TerminalOptions, Viewport};

use super::buffer::Buffer;
use crate::render::palette::Role;
use crate::render::style::Style;

/// How many lines of the terminal the box occupies.
///
/// **Fixed rather than grown to fit.** An inline viewport that changed height
/// as you typed would redraw the lines above it, and the lines above it are the
/// question. Eight lines of text is four times what the old single line held and
/// enough for any answer anyone gives at an interview; longer answers scroll,
/// which is what the file is for afterwards.
const HEIGHT: u16 = 11;

/// What came back.
pub enum Answer {
    /// Typed, and not blank.
    Given(String),
    /// `esc`, or nothing typed. The documented default stands.
    Kept,
}

/// Put a text area under the prompt and read paragraphs out of it.
///
/// Returns [`Answer::Kept`] for `esc`, for `ctrl-d` over an empty box, and for
/// any terminal that will not go into raw mode — the last of which is a machine
/// that cannot hold an interview, and taking the documented default is what
/// every other question does there too.
pub fn read(style: Style, standing: Option<&str>) -> Answer {
    let Ok(restore) = Restore::install() else {
        return Answer::Kept;
    };

    let backend = ratatui::backend::CrosstermBackend::new(std::io::stderr());
    let Ok(mut terminal) = Terminal::with_options(
        backend,
        TerminalOptions {
            viewport: Viewport::Inline(HEIGHT),
        },
    ) else {
        return Answer::Kept;
    };

    let mut buffer = Buffer::new();
    let answer = loop {
        if terminal
            .draw(|frame| draw(frame, &buffer, style, standing))
            .is_err()
        {
            break Answer::Kept;
        }
        match event::read() {
            Err(_) => break Answer::Kept,
            Ok(Event::Paste(text)) => buffer.paste(&text),
            Ok(Event::Key(key)) if key.kind != KeyEventKind::Release => {
                let control = key.modifiers.contains(KeyModifiers::CONTROL);
                match key.code {
                    // **`ctrl-d` finishes and `ctrl-c` keeps the default.** Two
                    // ways out, because a person who wants to stop typing and a
                    // person who wants to undo having started are asking for
                    // different things, and `ctrl-c` means the second
                    // everywhere else.
                    KeyCode::Char('d') if control => break done(&buffer),
                    KeyCode::Char('c') if control => break Answer::Kept,
                    KeyCode::Esc => break Answer::Kept,
                    KeyCode::Char(c) if !control => buffer.insert(c),
                    KeyCode::Enter => buffer.newline(),
                    KeyCode::Backspace => buffer.backspace(),
                    KeyCode::Delete => buffer.delete(),
                    KeyCode::Left => buffer.left(),
                    KeyCode::Right => buffer.right(),
                    KeyCode::Up => buffer.up(),
                    KeyCode::Down => buffer.down(),
                    KeyCode::Home => buffer.home(),
                    KeyCode::End => buffer.end(),
                    _ => {}
                }
            }
            Ok(_) => {}
        }
    };

    // The box goes and the answer is left in the scrollback by the caller, so
    // the transcript reads as one conversation rather than as a form.
    let _ = terminal.clear();
    drop(restore);
    answer
}

fn done(buffer: &Buffer) -> Answer {
    if buffer.is_empty() {
        Answer::Kept
    } else {
        Answer::Given(buffer.text())
    }
}

/// The box, the text in it, and the cursor.
fn draw(frame: &mut ratatui::Frame, buffer: &Buffer, style: Style, standing: Option<&str>) {
    let [area, footer] =
        Layout::vertical([Constraint::Min(3), Constraint::Length(1)]).areas(frame.area());

    let block = Block::default()
        .borders(Borders::ALL)
        .border_style(painted(style, Role::SteelGrey));
    let inner = block.inner(area);
    frame.render_widget(block, area);

    let lines: Vec<Line> = buffer.lines().into_iter().map(Line::from).collect();
    // **Wrapped, not truncated.** This is the one surface in Armada that wraps
    // by design: a table truncates because a row must stay one line, and a
    // paragraph you are writing must not lose its tail as you type it.
    frame.render_widget(Paragraph::new(lines).wrap(Wrap { trim: false }), inner);

    // The standing value under the box rather than in it — it is what enter
    // keeps, not a draft to edit (`PLAN.md` §13.4).
    if let Some(standing) = standing {
        frame.render_widget(
            Paragraph::new(Line::from(format!("now  {standing}")))
                .style(painted(style, Role::SteelGrey)),
            footer,
        );
    }

    let (row, column) = buffer.cursor();
    let width = inner.width.max(1) as usize;
    // The cursor follows the wrap: a line longer than the box occupies more than
    // one row of it, and putting the caret at `row` would leave it behind.
    let wrapped_row = wrapped_rows_before(buffer, row, width) + column / width;
    let position = (
        inner.x + (column % width) as u16,
        inner.y + wrapped_row.min(inner.height.saturating_sub(1) as usize) as u16,
    );
    frame.set_cursor_position(position);
}

/// How many drawn rows the lines before `row` occupy once wrapped.
fn wrapped_rows_before(buffer: &Buffer, row: usize, width: usize) -> usize {
    buffer
        .lines()
        .iter()
        .take(row)
        .map(|line| (line.chars().count() / width) + 1)
        .sum()
}

/// One of Armada's roles as a `ratatui` style, or unstyled when colour is off.
///
/// **The palette is not duplicated.** `render/palette.rs` is the one place a
/// colour is chosen (`docs/commands/render.md`), and this reads its RGB rather
/// than naming a second set here.
fn painted(style: Style, role: Role) -> RataStyle {
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
struct Restore;

impl Restore {
    fn install() -> Result<Restore, std::io::Error> {
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

#[cfg(test)]
mod tests {
    use super::*;

    /// **A palette colour, not a second palette.** `render/palette.rs` is the
    /// one place a colour is chosen, and colour off means no styling at all.
    #[test]
    fn the_editor_paints_from_the_one_palette() {
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

    /// The cursor follows the wrap. A line twice the width of the box occupies
    /// two rows of it, and the row after that starts on the third.
    #[test]
    fn the_cursor_row_counts_wrapped_rows_and_not_lines() {
        let mut buffer = Buffer::new();
        buffer.paste(&format!("{}\nsecond", "x".repeat(25)));
        assert_eq!(wrapped_rows_before(&buffer, 1, 10), 3);
        assert_eq!(wrapped_rows_before(&buffer, 0, 10), 0);
    }
}
