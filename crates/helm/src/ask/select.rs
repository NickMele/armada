//! One selector, used by every closed question Armada asks.
//!
//! # What this replaces
//!
//! A printed list of numbers followed by a silent `read_line`:
//!
//! ```text
//!   1 let an agent write it with me  opens claude here
//!   2 print the evidence and stop    I will write armada.yml myself
//! ```
//!
//! No prompt, no cursor, nothing to say the program was waiting — and the
//! person it was put to said so: *"has to have a better UI for selecting options
//! instead of just stopping and me guessing it's asking me to input a number."*
//! Every multiple-choice question in Armada had the same shape, so the fix is
//! one widget rather than a better paragraph at each call site.
//!
//! # Arrow keys, and the number still works
//!
//! `↑`/`↓` or `k`/`j` move, `enter` chooses, `esc` takes the default. A digit
//! **moves** the cursor rather than choosing outright, which is the one decision
//! here worth stating: typing `1` and then `enter` is what everybody who met the
//! old menu already does, and a digit that chose on its own would send that
//! `enter` to whatever the next prompt turned out to be.
//!
//! # It is drawn only for a person, and that is decided elsewhere
//!
//! [`Surface::Widgets`] reaches this file; [`Surface::Lines`] prints the same
//! options and reads a line, which is what an agent reading stdout gets and what
//! a terminal that refuses raw mode falls back to. Which one an invocation gets
//! follows from whether **both** stdin and stdout are a terminal — the same rule
//! `armada_core::scan::handover` already applies to `config scan`, computed at
//! the entrypoint and passed down rather than sniffed inside a widget
//! (`ARCHITECTURE.md` §1.4).
//!
//! [`Surface::Widgets`]: super::Surface::Widgets
//! [`Surface::Lines`]: super::Surface::Lines
//!
//! **An agent must never block on stdin that will never arrive.** That is the
//! whole reason the terminal decides rather than a flag, and it is why nothing
//! in this file is reachable without a person on the other end.

use ratatui::crossterm::event::{self, Event, KeyCode, KeyEventKind, KeyModifiers};
use ratatui::layout::{Constraint, Layout};
use ratatui::text::{Line, Span};
use ratatui::widgets::Paragraph;
use ratatui::{Terminal, TerminalOptions, Viewport};

use super::terminal::{painted, Restore};
use crate::render::palette::Role;
use crate::render::style::Style;

/// One thing that can be chosen.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Choice {
    /// What it is, in the words the answer is about.
    pub label: String,
    /// What it does, or nothing. `config scan` needs it — *opens claude here*
    /// is the difference between the two options — and `armada init`'s three
    /// speak for themselves.
    pub aside: String,
}

impl Choice {
    /// A choice with something to say about itself.
    pub fn new(label: &str, aside: &str) -> Choice {
        Choice {
            label: label.to_string(),
            aside: aside.to_string(),
        }
    }

    /// A choice whose label is the whole of it.
    pub fn bare(label: &str) -> Choice {
        Choice::new(label, "")
    }
}

/// Which option the cursor is on.
///
/// **Wrapping**, because a list of three is a ring and stopping at the ends
/// makes the last option feel further away than the first. Nothing here draws
/// or reads anything, which is what lets the key handling be tested where no
/// terminal exists.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Selection {
    count: usize,
    /// Zero-based.
    at: usize,
}

impl Selection {
    /// Open on `default`, which is **one-based** to match what is printed.
    pub fn new(count: usize, default: usize) -> Selection {
        Selection {
            count: count.max(1),
            at: default.saturating_sub(1).min(count.saturating_sub(1)),
        }
    }

    /// Down one, wrapping.
    pub fn next(&mut self) {
        self.at = (self.at + 1) % self.count;
    }

    /// Up one, wrapping.
    pub fn previous(&mut self) {
        self.at = (self.at + self.count - 1) % self.count;
    }

    /// Move to a one-based position, and say whether there was one.
    pub fn jump(&mut self, one_based: usize) -> bool {
        if one_based == 0 || one_based > self.count {
            return false;
        }
        self.at = one_based - 1;
        true
    }

    /// Where the cursor is, **one-based**.
    pub fn chosen(&self) -> usize {
        self.at + 1
    }

    /// Whether this is the row under the cursor, by zero-based index.
    pub fn is_on(&self, index: usize) -> bool {
        self.at == index
    }
}

/// What one keypress did.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Press {
    /// The cursor moved, or a digit put it somewhere.
    Moved,
    /// Enter: this is the answer.
    Chose,
    /// Esc or ctrl-c: take the documented default.
    Cancelled,
    /// A key this widget has no meaning for.
    Ignored,
}

/// Apply one keypress. **The whole of the key handling, and it touches
/// nothing** — `code` and `modifiers` are plain data, so every binding below is
/// asserted by a unit test rather than by someone pressing it.
pub fn apply(selection: &mut Selection, code: KeyCode, modifiers: KeyModifiers) -> Press {
    let control = modifiers.contains(KeyModifiers::CONTROL);
    match code {
        KeyCode::Char('c') if control => Press::Cancelled,
        KeyCode::Esc => Press::Cancelled,
        KeyCode::Enter => Press::Chose,
        KeyCode::Down | KeyCode::Tab | KeyCode::Char('j') => {
            selection.next();
            Press::Moved
        }
        KeyCode::Up | KeyCode::BackTab | KeyCode::Char('k') => {
            selection.previous();
            Press::Moved
        }
        // **A digit moves and does not choose.** `1` then `enter` is what
        // everyone who met the printed menu already types, and a digit that
        // chose on its own would hand that `enter` to the next prompt.
        KeyCode::Char(c) if c.is_ascii_digit() => {
            let wanted = c.to_digit(10).unwrap_or(0) as usize;
            if selection.jump(wanted) {
                Press::Moved
            } else {
                Press::Ignored
            }
        }
        _ => Press::Ignored,
    }
}

/// Put a closed question and read the answer, **one-based**.
///
/// `None` when the terminal would not cooperate — no raw mode, no backend, a
/// stream that ended. The caller falls back to printing the options and reading
/// a line, which is a worse interface and still an interface.
pub fn ask(question: &str, options: &[Choice], default: usize, style: Style) -> Option<usize> {
    if options.is_empty() {
        return None;
    }
    let restore = Restore::install().ok()?;
    let backend = ratatui::backend::CrosstermBackend::new(std::io::stderr());
    let mut terminal = Terminal::with_options(
        backend,
        TerminalOptions {
            // The evidence above is a report the reader has just finished
            // reading. A full-screen takeover would erase it, so the choice
            // appears *below* the output rather than instead of it.
            viewport: Viewport::Inline(height(options.len())),
        },
    )
    .ok()?;

    let mut selection = Selection::new(options.len(), default);
    let answer = loop {
        if terminal
            .draw(|frame| draw(frame, question, options, selection, style))
            .is_err()
        {
            break None;
        }
        match event::read() {
            Err(_) => break None,
            Ok(Event::Key(key)) if key.kind != KeyEventKind::Release => {
                match apply(&mut selection, key.code, key.modifiers) {
                    Press::Chose => break Some(selection.chosen()),
                    Press::Cancelled => break Some(default),
                    Press::Moved | Press::Ignored => {}
                }
            }
            Ok(_) => {}
        }
    };

    let _ = terminal.clear();
    drop(restore);
    answer
}

/// How many lines the widget occupies: the question, a gap, the options, a gap,
/// and the keys.
fn height(options: usize) -> u16 {
    options as u16 + 4
}

fn draw(
    frame: &mut ratatui::Frame,
    question: &str,
    options: &[Choice],
    selection: Selection,
    style: Style,
) {
    let [head, _, rows, _, keys] = Layout::vertical([
        Constraint::Length(1),
        Constraint::Length(1),
        Constraint::Length(options.len() as u16),
        Constraint::Length(1),
        Constraint::Length(1),
    ])
    .areas(frame.area());

    frame.render_widget(
        Paragraph::new(Line::from(Span::styled(
            question,
            painted(style, Role::SignalAmber),
        ))),
        head,
    );

    // The widest label, so the asides line up in a column rather than trailing
    // each option at a different place — the same reason `render/table.rs`
    // exists at all.
    let widest = options
        .iter()
        .map(|choice| choice.label.chars().count())
        .max()
        .unwrap_or(0);

    let lines: Vec<Line> = options
        .iter()
        .enumerate()
        .map(|(index, choice)| row(index, choice, selection, widest, style))
        .collect();
    frame.render_widget(Paragraph::new(lines), rows);

    frame.render_widget(
        Paragraph::new(Line::from(Span::styled(
            format!(
                "  up/down move · 1-{} jump · enter choose · esc keep the default",
                options.len()
            ),
            painted(style, Role::SteelGrey),
        ))),
        keys,
    );
}

/// One option's line: the cursor, its number, its label, and what it does.
fn row<'a>(
    index: usize,
    choice: &'a Choice,
    selection: Selection,
    widest: usize,
    style: Style,
) -> Line<'a> {
    let on = selection.is_on(index);
    // **The cursor is a character, not a colour.** A row told apart only by
    // being amber is a row a monochrome terminal cannot tell apart at all —
    // the same rule that spells every status out in words rather than
    // signalling it (`render/palette.rs`).
    let caret = if on { style.caret() } else { " " };
    let label = format!("{:<widest$}", choice.label);
    let mut spans = vec![
        Span::styled(format!("  {caret} "), painted(style, Role::SignalAmber)),
        Span::styled(format!("{} ", index + 1), painted(style, Role::SteelGrey)),
        Span::styled(
            label,
            if on {
                painted(style, Role::SignalAmber)
            } else {
                painted(style, Role::Foreground)
            },
        ),
    ];
    if !choice.aside.is_empty() {
        spans.push(Span::styled(
            format!("  {}", choice.aside),
            painted(style, Role::SteelGrey),
        ));
    }
    Line::from(spans)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn three() -> Selection {
        Selection::new(3, 3)
    }

    fn press(selection: &mut Selection, code: KeyCode) -> Press {
        apply(selection, code, KeyModifiers::NONE)
    }

    /// It opens on the documented default, which is what the old printed menu
    /// took when you pressed enter at it.
    #[test]
    fn it_opens_on_the_default() {
        assert_eq!(three().chosen(), 3);
        assert_eq!(Selection::new(2, 1).chosen(), 1);
    }

    /// **Both spellings of every direction.** Arrow keys for a person who has
    /// never touched vi, `j`/`k` for one who touches nothing else.
    #[test]
    fn arrows_and_jk_both_move_and_both_wrap() {
        for (down, up) in [
            (KeyCode::Down, KeyCode::Up),
            (KeyCode::Char('j'), KeyCode::Char('k')),
        ] {
            let mut selection = Selection::new(3, 1);
            assert_eq!(press(&mut selection, down), Press::Moved);
            assert_eq!(selection.chosen(), 2);
            assert_eq!(press(&mut selection, up), Press::Moved);
            assert_eq!(selection.chosen(), 1);

            // A list of three is a ring: up from the first is the last.
            press(&mut selection, up);
            assert_eq!(selection.chosen(), 3, "{up:?} did not wrap");
            press(&mut selection, down);
            assert_eq!(selection.chosen(), 1, "{down:?} did not wrap");
        }
    }

    /// **A digit moves the cursor and does not choose.** `1` then `enter` is
    /// what everyone who met the printed menu already types, and a digit that
    /// chose on its own would hand that `enter` to the next prompt.
    #[test]
    fn a_digit_moves_and_enter_is_what_chooses() {
        let mut selection = three();
        assert_eq!(press(&mut selection, KeyCode::Char('1')), Press::Moved);
        assert_eq!(selection.chosen(), 1);
        assert_eq!(press(&mut selection, KeyCode::Enter), Press::Chose);
        assert_eq!(selection.chosen(), 1);
    }

    /// A digit past the end of the list does nothing rather than moving
    /// somewhere arbitrary.
    #[test]
    fn a_digit_with_no_option_behind_it_is_ignored() {
        let mut selection = three();
        assert_eq!(press(&mut selection, KeyCode::Char('9')), Press::Ignored);
        assert_eq!(press(&mut selection, KeyCode::Char('0')), Press::Ignored);
        assert_eq!(selection.chosen(), 3, "the cursor moved anyway");
    }

    /// **Two ways out, and both take the documented default.** Every question
    /// this widget puts has one, and it is a working outcome — the same rule
    /// the interview follows.
    #[test]
    fn esc_and_ctrl_c_both_cancel() {
        let mut selection = three();
        assert_eq!(press(&mut selection, KeyCode::Esc), Press::Cancelled);
        assert_eq!(
            apply(&mut selection, KeyCode::Char('c'), KeyModifiers::CONTROL),
            Press::Cancelled
        );
        // A bare `c` is a key with no meaning here, not a cancel.
        assert_eq!(press(&mut selection, KeyCode::Char('c')), Press::Ignored);
    }

    /// Tab and shift-tab move too, because some people reach for those first.
    #[test]
    fn tab_moves_forward_and_back_tab_moves_back() {
        let mut selection = Selection::new(3, 1);
        press(&mut selection, KeyCode::Tab);
        assert_eq!(selection.chosen(), 2);
        press(&mut selection, KeyCode::BackTab);
        assert_eq!(selection.chosen(), 1);
    }

    /// A key with no binding leaves the selection exactly where it was.
    #[test]
    fn an_unbound_key_changes_nothing() {
        let mut selection = three();
        assert_eq!(press(&mut selection, KeyCode::Char('z')), Press::Ignored);
        assert_eq!(press(&mut selection, KeyCode::Home), Press::Ignored);
        assert_eq!(selection.chosen(), 3);
    }

    /// A default outside the list is clamped rather than panicking: the caller
    /// passes a constant and this is the one place a wrong one would be found.
    #[test]
    fn a_default_outside_the_list_is_clamped() {
        assert_eq!(Selection::new(2, 9).chosen(), 2);
        assert_eq!(Selection::new(2, 0).chosen(), 1);
    }

    /// The widget reserves the question, both gaps, the options and the key
    /// line — and nothing more, because everything above it is the report the
    /// reader has just read.
    #[test]
    fn the_viewport_is_exactly_the_lines_it_draws() {
        assert_eq!(height(2), 6);
        assert_eq!(height(3), 7);
    }
}
