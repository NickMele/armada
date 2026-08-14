//! One aligned-column renderer, used by every verb that lists things.
//!
//! **A table is described, not built.** A caller says what its columns are and
//! hands over cells; nothing outside this file counts spaces, and no verb owns a
//! format string. That is the rule PLAN.md §3.1.1 states as "one renderer, not
//! one per verb" — the moment a verb formats its own output, two verbs disagree
//! about what a column is called and one of them right-aligns a duration.
//!
//! **Cells are measured before they are painted.** An escape sequence is
//! characters a terminal does not display, so padding a painted string to
//! `{:<12}` pads to the wrong width and every column below it shears. Every
//! width here comes from the plain text; colour is applied to the content and
//! the padding is added around it.
//!
//! **Too narrow degrades by truncating a column, never by wrapping a row**
//! (PHASES.md §8.3.1). A wrapped row stops lining up with its header, so the
//! reader loses the alignment that was the only reason to draw a table.

use super::palette::Role;
use super::style::Style;
use super::term::{display_width, truncate};

/// Spaces between two columns. Two rather than one, so a short value and a long
/// header do not read as one word.
const GAP: usize = 2;

/// A flexible column is never squeezed below this. Narrower than this and the
/// column holds an ellipsis and one character, which is noise occupying the
/// space its own header needs.
const MIN_FLEXIBLE: usize = 8;

/// Which edge a column's values line up against.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Align {
    /// Names, ids, paths, states.
    Left,
    /// Numbers — durations, ports, counts. **Right, always**: a column of
    /// right-aligned numbers can be compared by eye without reading any of them.
    Right,
}

/// One column's description.
#[derive(Debug, Clone)]
pub struct Column {
    header: String,
    align: Align,
    flexible: bool,
}

impl Column {
    /// A fixed column: it always gets its natural width.
    ///
    /// For anything short and load-bearing — an id, a verdict, a port. Squeezing
    /// one of these loses the answer rather than a detail.
    pub fn fixed(header: &str) -> Column {
        Column {
            header: header.to_string(),
            align: Align::Left,
            flexible: false,
        }
    }

    /// A column that gives up width first: paths, messages, commands.
    ///
    /// Long, and its tail is the least valuable part of the line.
    pub fn flexible(header: &str) -> Column {
        Column {
            header: header.to_string(),
            align: Align::Left,
            flexible: true,
        }
    }

    /// Line this column up on its right edge.
    pub fn right(mut self) -> Column {
        self.align = Align::Right;
        self
    }
}

/// One value, and the role it is spoken in.
#[derive(Debug, Clone)]
pub struct Cell {
    text: String,
    role: Option<Role>,
}

impl Cell {
    /// Ordinary text, in the terminal's own foreground colour.
    ///
    /// **Unpainted rather than painted `Foreground`**: a terminal's own text
    /// colour is the reader's choice, and overriding it with `#E6E8EB` makes
    /// Armada the one tool that ignores a light theme.
    pub fn plain(text: impl Into<String>) -> Cell {
        Cell {
            text: text.into(),
            role: None,
        }
    }

    /// Text that carries a meaning the palette has a colour for.
    pub fn painted(text: impl Into<String>, role: Role) -> Cell {
        Cell {
            text: text.into(),
            role: Some(role),
        }
    }

    /// Muted: present, but not what the reader is scanning for.
    pub fn muted(text: impl Into<String>) -> Cell {
        Cell::painted(text, Role::SteelGrey)
    }

    /// Nothing to report in this column.
    pub fn empty() -> Cell {
        Cell::plain("")
    }
}

/// A described table, ready to render.
#[derive(Debug, Clone)]
pub struct Table {
    columns: Vec<Column>,
    rows: Vec<Row>,
    indent: usize,
    headers: bool,
}

/// A row, and the continuation line that belongs to it.
#[derive(Debug, Clone)]
struct Row {
    cells: Vec<Cell>,
    note: Option<String>,
}

impl Table {
    /// Describe a table by its columns.
    pub fn new(columns: Vec<Column>) -> Table {
        Table {
            columns,
            rows: Vec::new(),
            indent: 0,
            headers: true,
        }
    }

    /// Indent every line by this many spaces.
    pub fn indent(mut self, spaces: usize) -> Table {
        self.indent = spaces;
        self
    }

    /// Drop the header row — for a two-column layout whose columns need no
    /// naming, such as a flag and its explanation.
    pub fn headerless(mut self) -> Table {
        self.headers = false;
        self
    }

    /// Add a row. A row with fewer cells than there are columns is padded with
    /// empties rather than refused: a missing value is a normal fact about an
    /// envelope, and a renderer is not the place to discover it.
    pub fn row(self, cells: Vec<Cell>) -> Table {
        self.row_with_note(cells, None)
    }

    /// A row, plus one continuation line printed under it.
    ///
    /// **For the fact that belongs to a row but not to a column** — a failed
    /// check's log path is the case that earned it. Putting it in `DETAIL` would
    /// widen every row by the length of the longest path; giving it a column of
    /// its own would leave that column empty on every passing check.
    ///
    /// It is indented to the start of the **second** column, so it reads as
    /// hanging off the row's name rather than as a row of its own.
    pub fn row_with_note(mut self, cells: Vec<Cell>, note: Option<String>) -> Table {
        self.rows.push(Row { cells, note });
        self
    }

    /// Whether anything would be drawn.
    pub fn is_empty(&self) -> bool {
        self.rows.is_empty()
    }

    /// The table, as lines, each ending in `\n`.
    ///
    /// `width` is the terminal's usable width. Nothing is emitted at all when
    /// there are no rows — a header with nothing under it is a claim that
    /// something was listed.
    pub fn render(&self, style: Style, width: usize) -> String {
        if self.rows.is_empty() {
            return String::new();
        }

        let widths = self.widths(width);
        let mut out = String::new();

        if self.headers && self.columns.iter().any(|c| !c.header.is_empty()) {
            let header: Vec<Cell> = self
                .columns
                .iter()
                .map(|c| Cell::painted(c.header.to_uppercase(), Role::SteelGrey))
                .collect();
            out.push_str(&self.line(&header, &widths, style, true));
        }
        for row in &self.rows {
            out.push_str(&self.line(&row.cells, &widths, style, false));
            if let Some(note) = &row.note {
                // Under the second column: hanging off the row's name, not a
                // row of its own. With one column there is no second, so it
                // lines up under the first.
                let hang = self.indent + widths.first().map_or(0, |w| w + GAP);
                let room = width.saturating_sub(hang);
                out.push_str(&" ".repeat(hang));
                out.push_str(&style.paint(Role::SteelGrey, &truncate(note, room)));
                out.push('\n');
            }
        }
        out
    }

    /// One rendered line, padded to `widths` and with no trailing spaces.
    ///
    /// Trailing whitespace is not cosmetic here: it is what makes a diff of two
    /// captured outputs unreadable, and what a `grep -c ' $'` in someone's CI
    /// eventually complains about.
    fn line(&self, cells: &[Cell], widths: &[usize], style: Style, bold: bool) -> String {
        let mut line = " ".repeat(self.indent);
        let last = widths.len().saturating_sub(1);

        for (index, width) in widths.iter().enumerate() {
            let blank = Cell::empty();
            let cell = cells.get(index).unwrap_or(&blank);
            let text = truncate(&cell.text, *width);
            let pad = width.saturating_sub(display_width(&text));

            let painted = match (cell.role, bold) {
                (Some(role), true) => style.strong(role, &text),
                (Some(role), false) => style.paint(role, &text),
                (None, true) => style.strong(Role::Foreground, &text),
                (None, false) => text.clone(),
            };

            let align = self.columns.get(index).map_or(Align::Left, |c| c.align);
            match align {
                Align::Right => {
                    line.push_str(&" ".repeat(pad));
                    line.push_str(&painted);
                }
                Align::Left => {
                    line.push_str(&painted);
                    // The last column is never padded on its right; that is only
                    // trailing whitespace with extra steps.
                    if index != last {
                        line.push_str(&" ".repeat(pad));
                    }
                }
            }
            if index != last {
                line.push_str(&" ".repeat(GAP));
            }
        }

        // A right-aligned final column can still have left padding on an
        // otherwise empty line; and a row of empties leaves the indent alone.
        while line.ends_with(' ') {
            line.pop();
        }
        line.push('\n');
        line
    }

    /// Each column's natural width, then shrunk to fit.
    ///
    /// **Widest flexible column first, one column at a time.** Taking an equal
    /// share from each would squeeze a 9-character column that was already
    /// minimal in order to spare a 60-character path, which is backwards: the
    /// path is where the redundancy is.
    fn widths(&self, available: usize) -> Vec<usize> {
        let mut widths: Vec<usize> = self
            .columns
            .iter()
            .enumerate()
            .map(|(index, column)| {
                let header = if self.headers {
                    display_width(&column.header)
                } else {
                    0
                };
                self.rows
                    .iter()
                    .map(|row| row.cells.get(index).map_or(0, |c| display_width(&c.text)))
                    .chain(std::iter::once(header))
                    .max()
                    .unwrap_or(0)
            })
            .collect();

        let fixed_overhead = self.indent + GAP * widths.len().saturating_sub(1);
        loop {
            let total: usize = fixed_overhead + widths.iter().sum::<usize>();
            if total <= available {
                break;
            }
            // The widest column that is both flexible and above the floor.
            let Some(victim) = widths
                .iter()
                .enumerate()
                .filter(|(index, width)| {
                    self.columns.get(*index).is_some_and(|c| c.flexible) && **width > MIN_FLEXIBLE
                })
                .max_by_key(|(_, width)| **width)
                .map(|(index, _)| index)
            else {
                // Nothing left to give. The line overhangs, which is honest:
                // truncating a fixed column would remove the answer.
                break;
            };
            let over = total - available;
            let give = over.min(widths[victim] - MIN_FLEXIBLE);
            widths[victim] -= give;
        }
        widths
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn ids() -> Table {
        Table::new(vec![
            Column::fixed("check"),
            Column::fixed("status"),
            Column::fixed("time").right(),
        ])
        .row(vec![
            Cell::plain("armada:boundaries"),
            Cell::painted("PASS", Role::BeaconGreen),
            Cell::plain("426ms"),
        ])
        .row(vec![
            Cell::plain("armada:test"),
            Cell::painted("FAILED", Role::DistressRed),
            Cell::plain("25632ms"),
        ])
    }

    #[test]
    fn columns_line_up_and_the_header_is_the_columns_name_upper_cased() {
        let text = ids().render(Style::plain(), 80);
        assert_eq!(
            text,
            "CHECK              STATUS     TIME\n\
             armada:boundaries  PASS      426ms\n\
             armada:test        FAILED  25632ms\n"
        );
    }

    /// A right-aligned column is what makes two durations comparable without
    /// reading either of them.
    #[test]
    fn a_right_aligned_column_lines_up_on_its_last_character() {
        let text = ids().render(Style::plain(), 80);
        let ends: Vec<usize> = text.lines().map(str::len).collect();
        assert!(
            ends.windows(2).all(|w| w[0] == w[1]),
            "right-aligned last column did not line up: {text}"
        );
    }

    #[test]
    fn no_line_ends_in_whitespace() {
        let text = ids().indent(2).render(Style::plain(), 80);
        for line in text.lines() {
            assert!(!line.ends_with(' '), "trailing space: {line:?}");
        }
    }

    /// An empty table draws nothing at all — not even a header. A header with
    /// nothing under it claims something was listed.
    #[test]
    fn a_table_with_no_rows_renders_nothing() {
        let table = Table::new(vec![Column::fixed("check")]);
        assert!(table.is_empty());
        assert_eq!(table.render(Style::plain(), 80), "");
    }

    /// **The degradation rule**: a narrow terminal loses the tail of the
    /// flexible column, and every row stays one line.
    #[test]
    fn a_narrow_terminal_truncates_a_column_rather_than_wrapping_a_row() {
        let table = Table::new(vec![Column::fixed("id"), Column::flexible("log")])
            .row(vec![
                Cell::plain("api:lint"),
                Cell::plain(".armada/run/01J8X2/logs/api.lint.log"),
            ])
            .row(vec![
                Cell::plain("web:e2e"),
                Cell::plain(".armada/run/01J8X2/logs/web.e2e.log"),
            ]);

        let text = table.render(Style::plain(), 30);
        assert_eq!(text.lines().count(), 3, "one line per row plus the header");
        for line in text.lines() {
            assert!(display_width(line) <= 30, "{line:?} overhangs");
        }
        assert!(text.contains('…'), "the cut is marked: {text}");
        assert!(
            text.contains("api:lint"),
            "the fixed column survived: {text}"
        );
    }

    /// A fixed column is never squeezed, so a terminal too narrow for the fixed
    /// columns alone overhangs rather than losing the answer.
    #[test]
    fn a_fixed_column_keeps_its_width_even_when_nothing_fits() {
        let table = Table::new(vec![Column::fixed("workspace"), Column::fixed("status")])
            .row(vec![Cell::plain("a3f91c02"), Cell::plain("FAILED")]);
        let text = table.render(Style::plain(), 10);
        assert_eq!(text.lines().count(), 2);
        assert!(text.contains("a3f91c02   FAILED"), "{text}");
    }

    /// **The measurement rule.** A painted cell and a plain one occupy the same
    /// columns, so the two audiences see the same table with and without colour.
    #[test]
    fn painting_a_cell_does_not_move_the_column_after_it() {
        let plain = ids().render(Style::plain(), 80);
        let painted = ids().render(Style::painted(), 80);
        assert!(painted.contains('\x1b'));
        for (a, b) in plain.lines().zip(painted.lines()) {
            assert_eq!(display_width(a), display_width(b), "{a:?} vs {b:?}");
        }
    }

    /// A row shorter than the column list is padded, not refused: a missing
    /// value is an ordinary fact about an envelope.
    #[test]
    fn a_short_row_is_padded_rather_than_refused() {
        let text = Table::new(vec![
            Column::fixed("id"),
            Column::fixed("status"),
            Column::fixed("note"),
        ])
        .row(vec![Cell::plain("api"), Cell::plain("PASS")])
        .render(Style::plain(), 80);
        assert_eq!(text, "ID   STATUS  NOTE\napi  PASS\n");
    }

    /// A headerless table is how a two-column list — a flag and what it does —
    /// gets alignment without inventing names for its columns.
    #[test]
    fn a_headerless_table_draws_only_its_rows() {
        let text = Table::new(vec![Column::fixed(""), Column::flexible("")])
            .headerless()
            .indent(2)
            .row(vec![Cell::plain("--json"), Cell::plain("the envelope")])
            .row(vec![Cell::plain("--color"), Cell::plain("auto, always")])
            .render(Style::plain(), 80);
        assert_eq!(text, "  --json   the envelope\n  --color  auto, always\n");
    }
}
