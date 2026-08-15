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
//! interview is seven questions long; it should be seven questions long from where
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
//! # The box opens holding what you already have
//!
//! There used to be a `now …` line above it previewing the standing value, and
//! it was wrong twice over. It **truncated**, so a long imported fragment could
//! not be read — which is the only reason to show it. And it **rendered twice**:
//! once above the box, cut to the terminal's width, and once again inside this
//! file as a one-line footer that took no account of wrapping and ran off the
//! edge.
//!
//! Pre-filling deletes both. The default is then visible in full, scrollable and
//! directly editable, rather than a truncated echo of text you would have had to
//! retype to change — and there is nothing left to draw above the box.
//!
//! | Key | Does |
//! |---|---|
//! | `ctrl-d` | saves what is in the box, edited or not |
//! | `esc`, `ctrl-c` | keeps it as it was — the file is not touched |
//!
//! Those two are different answers and the difference is load-bearing: `ctrl-d`
//! on an unchanged box means *I have read this and it is mine*, which is what
//! clears `armada doctor`'s still-not-yours row; `esc` means *leave it*, which
//! does not.
//!
//! # Two more things this file must get right
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

use ratatui::crossterm::event::{self, Event, KeyCode, KeyEventKind, KeyModifiers};
use ratatui::text::Line;
use ratatui::widgets::{Block, Borders, Paragraph, Wrap};
use ratatui::{Terminal, TerminalOptions, Viewport};

use super::buffer::Buffer;
use super::terminal::{painted, Restore};
use crate::render::palette::Role;
use crate::render::style::Style;

/// How many lines of the terminal the box occupies, borders included.
///
/// **Fixed rather than grown to fit.** An inline viewport that changed height as
/// you typed would redraw the lines above it, and the lines above it are the
/// question. Ten lines is eight of text, which is enough to see a fragment and
/// work in; anything longer scrolls, and the box says how much is still below.
const HEIGHT: u16 = 10;

/// What came back.
pub enum Answer {
    /// `ctrl-d`: what is in the box, edited or not.
    Given(String),
    /// `esc`, or an empty box. The file is not touched.
    Kept,
    /// **The box never opened**, so nothing was asked and nothing was answered.
    ///
    /// Distinct from [`Answer::Kept`] on purpose. Both leave the file alone, but
    /// only one of them is a decision: a terminal that will not go into raw mode
    /// or will not answer a cursor query has a person sitting at it who has just
    /// watched his question scroll past without a box under it. The caller falls
    /// back to reading a line, which is what `choose` already does with a
    /// selector it cannot draw — found by driving this under a pty that does not
    /// answer `ESC [ 6 n`, where the question silently kept its default.
    Unavailable,
}

/// Put a text area under the prompt and read paragraphs out of it.
///
/// `initial` is what the box opens holding — the fragment as it stands, so the
/// default is editable rather than previewed. Empty for a fragment there is
/// nothing to pre-fill.
pub fn read(style: Style, initial: &str) -> Answer {
    let Ok(restore) = Restore::install() else {
        return Answer::Unavailable;
    };

    let backend = ratatui::backend::CrosstermBackend::new(std::io::stderr());
    let Ok(mut terminal) = Terminal::with_options(
        backend,
        TerminalOptions {
            viewport: Viewport::Inline(HEIGHT),
        },
    ) else {
        return Answer::Unavailable;
    };

    // **Cursor at the end, not the start.** Opening on the last character is
    // what every editor does with a file you asked to edit, and it is the
    // position from which adding a line costs one keystroke.
    let mut buffer = Buffer::holding(initial);
    // A draw that fails before the box has ever appeared is a box that never
    // opened; one that fails later is a terminal that went away mid-answer, and
    // there is nothing left to fall back to.
    let mut drawn = false;
    let mut area = None;
    let answer = loop {
        match terminal.draw(|frame| draw(frame, &buffer, style)) {
            Ok(completed) => area = Some(completed.area),
            Err(_) => {
                break if drawn {
                    Answer::Kept
                } else {
                    Answer::Unavailable
                }
            }
        }
        drawn = true;
        match event::read() {
            Err(_) => break Answer::Kept,
            Ok(Event::Paste(text)) => buffer.paste(&text),
            Ok(Event::Key(key)) if key.kind != KeyEventKind::Release => {
                let control = key.modifiers.contains(KeyModifiers::CONTROL);
                match key.code {
                    // **`ctrl-d` saves and `ctrl-c` leaves it as it was.** Two
                    // ways out, and with a pre-filled box the difference is
                    // load-bearing rather than a courtesy: `ctrl-d` on a box
                    // nobody edited says *I have read this and it is mine*,
                    // which is what clears `armada doctor`'s row; `esc` says
                    // *leave it*, which does not.
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
    // the transcript reads as one conversation rather than as a form. Cleared
    // from the area the last draw already handed back, not re-asked for —
    // see `terminal::clear_viewport`.
    if let Some(area) = area {
        super::terminal::clear_viewport(std::io::stderr(), area);
    }
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

/// The box, the text in it, the cursor, and how much is out of sight.
///
/// **It scrolls.** An imported `CLAUDE.md` fragment is easily thirty lines, and
/// a box that silently kept the first eight would be the truncation this widget
/// was pre-filled to remove, moved somewhere new. The view follows the cursor
/// and the bottom border says how many rows are still below it.
fn draw(frame: &mut ratatui::Frame, buffer: &Buffer, style: Style) {
    let area = frame.area();
    let width = area.width.saturating_sub(2).max(1) as usize;
    let visible = area.height.saturating_sub(2).max(1) as usize;

    let (row, column) = buffer.cursor();
    let at = wrapped_rows_before(buffer, row, width) + column / width;
    let rows = wrapped_rows_before(buffer, buffer.lines().len(), width);
    let top = view_top(at, rows, visible);
    let below = rows.saturating_sub(top + visible);

    let mut block = Block::default()
        .borders(Borders::ALL)
        .border_style(painted(style, Role::SteelGrey));
    if below > 0 {
        // **Said out loud rather than left to be discovered.** A box whose tail
        // is out of sight and does not say so is one a reader believes he has
        // read all of.
        block = block.title_bottom(
            Line::from(format!(" {below} more below "))
                .right_aligned()
                .style(painted(style, Role::SteelGrey)),
        );
    }
    let inner = block.inner(area);
    frame.render_widget(block, area);

    let lines: Vec<Line> = buffer.lines().into_iter().map(Line::from).collect();
    // **Wrapped, not truncated.** This is the one surface in Armada that wraps
    // by design: a table truncates because a row must stay one line, and a
    // paragraph you are writing must not lose its tail as you type it.
    frame.render_widget(
        Paragraph::new(lines)
            .wrap(Wrap { trim: false })
            .scroll((top as u16, 0)),
        inner,
    );

    // The cursor follows the wrap *and* the scroll: a line longer than the box
    // occupies more than one row of it, and a view that has scrolled has moved
    // every row under the caret.
    frame.set_cursor_position((
        inner.x + (column % width) as u16,
        inner.y + at.saturating_sub(top) as u16,
    ));
}

/// Which wrapped row the top of the box shows.
///
/// **The smallest scroll that keeps the cursor in view**, so typing at the
/// bottom pushes the view by one line rather than recentring it — a box that
/// jumps under the cursor is one you lose your place in.
fn view_top(cursor: usize, rows: usize, visible: usize) -> usize {
    let last = rows.saturating_sub(visible);
    cursor.saturating_sub(visible.saturating_sub(1)).min(last)
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

#[cfg(test)]
mod tests {
    use super::*;

    /// The cursor follows the wrap. A line twice the width of the box occupies
    /// two rows of it, and the row after that starts on the third.
    #[test]
    fn the_cursor_row_counts_wrapped_rows_and_not_lines() {
        let mut buffer = Buffer::new();
        buffer.paste(&format!("{}\nsecond", "x".repeat(25)));
        assert_eq!(wrapped_rows_before(&buffer, 1, 10), 3);
        assert_eq!(wrapped_rows_before(&buffer, 0, 10), 0);
    }

    /// **The box opens holding what you already have, cursor at the end.** That
    /// is the whole of the fix: the default is editable rather than previewed,
    /// so it can neither truncate nor be drawn twice.
    #[test]
    fn the_box_opens_holding_the_value_with_the_cursor_at_the_end() {
        let buffer = Buffer::holding("first line\nsecond line");
        assert_eq!(buffer.text(), "first line\nsecond line");
        assert_eq!(buffer.cursor(), (1, 11));
    }

    /// **A value that does not fit scrolls rather than being hidden.** Thirty
    /// lines in an eight-line box is the ordinary case for an imported
    /// fragment, and a box that kept the first eight would be the truncation
    /// this widget was pre-filled to remove, in a new place.
    #[test]
    fn the_view_follows_the_cursor_and_never_scrolls_past_the_end() {
        // Nothing to scroll: the whole value fits.
        assert_eq!(view_top(0, 4, 8), 0);
        assert_eq!(view_top(3, 4, 8), 0);

        // Thirty rows in eight: the top stays put until the cursor reaches the
        // bottom row, then moves one for one.
        assert_eq!(view_top(7, 30, 8), 0, "no scroll until it has to");
        assert_eq!(view_top(8, 30, 8), 1, "one line at a time, not a jump");
        assert_eq!(view_top(29, 30, 8), 22, "the last row is the bottom row");

        // And never past the end, whatever the cursor claims.
        assert_eq!(view_top(99, 30, 8), 22);
    }

    /// The count on the border is what is out of sight below, and it is absent
    /// when nothing is.
    #[test]
    fn the_border_counts_the_rows_still_below() {
        assert_eq!(30usize.saturating_sub(view_top(7, 30, 8) + 8), 22);
        assert_eq!(30usize.saturating_sub(view_top(29, 30, 8) + 8), 0);
        assert_eq!(4usize.saturating_sub(view_top(0, 4, 8) + 8), 0);
    }
}
