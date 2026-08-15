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
    // **Carried out of the loop rather than re-asked for at the end.** Every
    // successful `draw` already hands back exactly the `Rect` teardown needs;
    // asking the terminal again is the query `terminal::clear_viewport` exists
    // to avoid.
    let mut area = None;
    let answer = loop {
        match terminal.draw(|frame| draw(frame, question, options, selection, style)) {
            Ok(completed) => area = Some(completed.area),
            Err(_) => break None,
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

    // **Nothing to give back if nothing was ever drawn.** `area` is `None`
    // only when the very first `draw` failed, and a widget that never
    // appeared leaves nothing on screen to clear.
    if let Some(area) = area {
        super::terminal::clear_viewport(std::io::stderr(), area);
    }
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
    //
    // **Bounded by what the row has left to spend on it.** A caller hands
    // over labels sized for its own guess at the row's furniture; this is the
    // one place that knows the furniture for certain — the indent, the
    // cursor, the option number (a second digit once there are ten options or
    // more), the gap, and the widest aside anybody offered. Padding a label
    // past what that leaves pushes the aside past the edge of the terminal,
    // which is what `armada failures`' own `done` row did the day its list
    // first reached ten entries: `stop looking` came back `stop lookin`.
    let widest = options
        .iter()
        .map(|choice| choice.label.chars().count())
        .max()
        .unwrap_or(0)
        .min(room_for_label(frame.area().width as usize, options));

    // **The aside truncates now that it no longer owns the row.** It used to be
    // reserved whole and the label took what was left; that inverted which half
    // a reader needs (see `room_for_label`). Trimming it here rather than in
    // `row` keeps the width arithmetic in one place.
    let aside = aside_room(frame.area().width as usize, options);
    let lines: Vec<Line> = options
        .iter()
        .enumerate()
        .map(|(index, choice)| row(index, choice, selection, widest, aside, style))
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

/// What a row has left for its label, once its own furniture is spent.
///
/// **Computed from the whole list, not from one row**, because every label is
/// padded to the same width — a room that varied per row would make some rows
/// fit and others not, for a reason nothing on screen would explain. The
/// option number is sized off `options.len()` rather than assumed to be one
/// digit: the tenth option is already two.
/// The label's share of the row, and **the label never loses to the aside.**
///
/// The first version reserved the widest aside outright and gave the label
/// whatever remained. A guild whose skills carry a hundred-and-fifty-character
/// `description:` then drove `furniture` past the terminal width, the
/// subtraction saturated at zero, and every label truncated to nothing — so
/// `armada guild ls` drew a column of descriptions with no names against them.
/// A reader cannot pick a row by a sentence it shares with four others.
///
/// **The label is the identity and the aside is the explanation**, so the aside
/// is what gives way. The label gets what it asks for up to [`LABEL_CAP`], and
/// never less than [`LABEL_FLOOR`] while the row has that much to give; the
/// aside takes the remainder and truncates itself in [`aside_room`].
fn room_for_label(width: usize, options: &[Choice]) -> usize {
    let digits = options.len().to_string().len();
    // "  {caret} " (4) + the number and its space (digits + 1).
    let furniture = 4 + digits + 1;
    let free = width.saturating_sub(furniture);

    let widest_label = options
        .iter()
        .map(|choice| choice.label.chars().count())
        .max()
        .unwrap_or(0);
    let wanted = widest_label.min(LABEL_CAP);

    // The floor only applies while the row can afford it — a very narrow
    // terminal gets the label and nothing else rather than a guaranteed width
    // it cannot honour.
    wanted.max(LABEL_FLOOR.min(free)).min(free)
}

/// What the aside may use once the label has taken its share.
///
/// Zero when nothing is left, which drops the column rather than wrapping it —
/// the same rule `render/table.rs` follows for an empty column.
fn aside_room(width: usize, options: &[Choice]) -> usize {
    let digits = options.len().to_string().len();
    let furniture = 4 + digits + 1 + room_for_label(width, options) + 2;
    width.saturating_sub(furniture)
}

/// The most a label may take, however long the longest one is. A label wider
/// than this is a name that has stopped being scannable, and the aside beside
/// it is worth more than its tail.
const LABEL_CAP: usize = 44;

/// What a label keeps even when every aside is long. Enough for a padded kind
/// and a file name — `SUBAGENT  helm.md` is 18.
const LABEL_FLOOR: usize = 24;

/// One option's line: the cursor, its number, its label, and what it does.
fn row<'a>(
    index: usize,
    choice: &'a Choice,
    selection: Selection,
    widest: usize,
    aside: usize,
    style: Style,
) -> Line<'a> {
    let on = selection.is_on(index);
    // **The cursor is a character, not a colour.** A row told apart only by
    // being amber is a row a monochrome terminal cannot tell apart at all —
    // the same rule that spells every status out in words rather than
    // signalling it (`render/palette.rs`).
    let caret = if on { style.caret() } else { " " };
    // **Truncated to `widest` before it is padded to it.** `widest` is
    // already bounded to what the row has left (see `room_for_label`), so a
    // label longer than that would otherwise be padded past the edge it was
    // just clamped to avoid.
    let label = format!(
        "{:<widest$}",
        crate::render::term::truncate(&choice.label, widest)
    );
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
    if !choice.aside.is_empty() && aside > 0 {
        spans.push(Span::styled(
            format!("  {}", crate::render::term::truncate(&choice.aside, aside)),
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

    /// **The option number is not always one digit.** A list of nine options
    /// or fewer numbers every row with one; the tenth grows a second, and the
    /// room left for the label has to shrink by exactly that much or the row
    /// **A long aside must never cost the label its name.**
    ///
    /// `armada guild ls` drew this exactly wrong the first time it met a real
    /// guild: skills carry a `description:` of a hundred and fifty characters,
    /// the widest aside was reserved outright, the subtraction saturated at
    /// zero, and every row rendered as a description with no name against it.
    /// A reader cannot pick `SKILL  gitnexus-pr-review` out of a column of
    /// sentences that all begin "Use when the user".
    #[test]
    fn a_long_aside_never_starves_the_label() {
        let options = vec![
            Choice::new("MEMORY    voice.md", &"x".repeat(150)),
            Choice::new("SUBAGENT  helm.md", &"y".repeat(150)),
            Choice::new("done", "stop looking"),
        ];
        let width = 120;
        let label = room_for_label(width, &options);
        assert!(
            label >= "SUBAGENT  helm.md".len(),
            "the label was starved to {label} by an aside that should have given way"
        );
        // And the row still fits: the aside is what truncates now.
        let widest = options
            .iter()
            .map(|choice| choice.label.chars().count())
            .max()
            .unwrap_or(0)
            .min(label);
        for (index, choice) in options.iter().enumerate() {
            let line = row(
                index,
                choice,
                Selection::new(options.len(), 1),
                widest,
                aside_room(width, &options),
                Style::plain(),
            );
            assert!(line.width() <= width, "row {index} overflowed");
        }
    }

    /// it belongs to overruns.
    #[test]
    fn the_aside_shrinks_once_the_option_number_grows_a_digit() {
        // **The aside pays for the second digit now, not the label.** It was
        // the label's bill when the label took what the aside left over; the
        // digit still has to come from somewhere, and the aside is the half
        // that gives way (see `room_for_label`).
        let nine = vec![Choice::new("x", "aside"); 9];
        let ten = vec![Choice::new("x", "aside"); 10];
        assert_eq!(aside_room(100, &nine), aside_room(100, &ten) + 1);
    }

    /// **The room accounts for the widest aside, not each row's own.** A row
    /// with a short aside still has to line up under one with a long one, so
    /// the budget is set by whichever option carries the most.
    #[test]
    fn room_for_label_is_not_set_by_the_widest_aside() {
        let short = vec![Choice::new("MEMORY  voice.md", "short")];
        let long = vec![Choice::new("MEMORY  voice.md", &"x".repeat(150))];
        assert_eq!(
            room_for_label(120, &short),
            room_for_label(120, &long),
            "a longer aside changed what the label was allowed, which is the \
             bug that drew `guild ls` as a column of descriptions with no names"
        );
    }

    /// **A row never draws past what `room_for_label` allowed**, whatever a
    /// caller's own label turns out to be. This is what a label padded to a
    /// `widest` computed without this bound used to violate: `armada
    /// failures`' `done` row, the day its list first reached ten entries,
    /// where `stop looking` came back `stop lookin`.
    #[test]
    fn a_row_never_draws_past_the_room_it_was_given() {
        let width = 40;
        let options = vec![
            Choice::new(&"x".repeat(200), "stop looking"),
            Choice::bare("done"),
        ];
        let widest = options
            .iter()
            .map(|choice| choice.label.chars().count())
            .max()
            .unwrap_or(0)
            .min(room_for_label(width, &options));
        for (index, choice) in options.iter().enumerate() {
            let line = row(
                index,
                choice,
                Selection::new(options.len(), 1),
                widest,
                aside_room(width, &options),
                Style::plain(),
            );
            assert!(
                line.width() <= width,
                "row {index} drew {} at width {width}",
                line.width()
            );
        }
    }
}
