//! The editing model behind the interview's text area — **and nothing that
//! touches a terminal**.
//!
//! A [`Buffer`] is lines and a cursor, and every key the editor accepts is a
//! method on it. That split is the whole reason this file exists: what a
//! terminal does with raw mode and an alternate viewport cannot be tested
//! anywhere a CI job runs, and what happens when you press left at the start of
//! line two can be. The driver in [`super::editor`] is a loop that translates
//! events into these calls and draws the result; the decisions are all here.

/// Lines and a cursor.
///
/// **Grapheme clusters are not modelled**, and that is the same stated limit
/// `render/term.rs` takes for width: Armada's inputs are prose in the Latin
/// alphabet, and a Unicode segmentation table is a dependency the render layer
/// has already ruled out. Characters are `char`s, and an emoji typed into
/// `voice.md` costs one backspace per code point.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Buffer {
    lines: Vec<Vec<char>>,
    /// Which line the cursor is on.
    row: usize,
    /// How many characters into that line, **not** how many columns.
    col: usize,
}

impl Default for Buffer {
    fn default() -> Buffer {
        Buffer {
            lines: vec![Vec::new()],
            row: 0,
            col: 0,
        }
    }
}

impl Buffer {
    /// An empty one, with the cursor at the start.
    pub fn new() -> Buffer {
        Buffer::default()
    }

    /// One with text already in it, cursor at the end.
    pub fn holding(text: &str) -> Buffer {
        let mut buffer = Buffer::new();
        buffer.paste(text);
        buffer
    }

    /// Type one character.
    pub fn insert(&mut self, c: char) {
        let line = &mut self.lines[self.row];
        let at = self.col.min(line.len());
        line.insert(at, c);
        self.col = at + 1;
    }

    /// Enter: split the line at the cursor.
    pub fn newline(&mut self) {
        let at = self.col.min(self.lines[self.row].len());
        let tail = self.lines[self.row].split_off(at);
        self.lines.insert(self.row + 1, tail);
        self.row += 1;
        self.col = 0;
    }

    /// Backspace. **At the start of a line it joins with the one above**, which
    /// is what every editor does and what its absence makes immediately
    /// obvious.
    pub fn backspace(&mut self) {
        if self.col > 0 {
            self.col -= 1;
            self.lines[self.row].remove(self.col);
            return;
        }
        if self.row == 0 {
            return;
        }
        let tail = self.lines.remove(self.row);
        self.row -= 1;
        self.col = self.lines[self.row].len();
        self.lines[self.row].extend(tail);
    }

    /// Delete forwards.
    pub fn delete(&mut self) {
        if self.col < self.lines[self.row].len() {
            self.lines[self.row].remove(self.col);
            return;
        }
        if self.row + 1 < self.lines.len() {
            let tail = self.lines.remove(self.row + 1);
            self.lines[self.row].extend(tail);
        }
    }

    /// Left, wrapping to the end of the line above.
    pub fn left(&mut self) {
        if self.col > 0 {
            self.col -= 1;
        } else if self.row > 0 {
            self.row -= 1;
            self.col = self.lines[self.row].len();
        }
    }

    /// Right, wrapping to the start of the line below.
    pub fn right(&mut self) {
        if self.col < self.lines[self.row].len() {
            self.col += 1;
        } else if self.row + 1 < self.lines.len() {
            self.row += 1;
            self.col = 0;
        }
    }

    /// Up, keeping as much of the column as the shorter line has.
    pub fn up(&mut self) {
        if self.row > 0 {
            self.row -= 1;
            self.col = self.col.min(self.lines[self.row].len());
        }
    }

    /// Down, same rule.
    pub fn down(&mut self) {
        if self.row + 1 < self.lines.len() {
            self.row += 1;
            self.col = self.col.min(self.lines[self.row].len());
        }
    }

    /// To the start of this line.
    pub fn home(&mut self) {
        self.col = 0;
    }

    /// To the end of this line.
    pub fn end(&mut self) {
        self.col = self.lines[self.row].len();
    }

    /// **Several paragraphs at once, arriving intact.**
    ///
    /// A paste is not typing quickly: it is one event carrying newlines, and a
    /// buffer that treated each `\n` as a keystroke would be right by accident
    /// and wrong the moment the terminal sends `\r\n`. Both line endings are
    /// normalised here, and a trailing newline leaves the cursor on a new empty
    /// line rather than being dropped — because it is in what was pasted.
    pub fn paste(&mut self, text: &str) {
        for c in text.replace("\r\n", "\n").replace('\r', "\n").chars() {
            if c == '\n' {
                self.newline();
            } else {
                self.insert(c);
            }
        }
    }

    /// Everything typed, as one string.
    pub fn text(&self) -> String {
        self.lines
            .iter()
            .map(|line| line.iter().collect::<String>())
            .collect::<Vec<_>>()
            .join("\n")
    }

    /// Whether anything at all was typed. **Whitespace does not count**: an
    /// answer of three spaces is an accepted default, exactly as a bare newline
    /// is at a single-line prompt.
    pub fn is_empty(&self) -> bool {
        self.text().trim().is_empty()
    }

    /// The lines, for the draw.
    pub fn lines(&self) -> Vec<String> {
        self.lines
            .iter()
            .map(|line| line.iter().collect())
            .collect()
    }

    /// Where the cursor is, as `(row, column)` — both zero-based, and the
    /// column is a display column rather than a character index.
    pub fn cursor(&self) -> (usize, usize) {
        (self.row, self.col)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn typed(text: &str) -> Buffer {
        let mut buffer = Buffer::new();
        for c in text.chars() {
            if c == '\n' {
                buffer.newline();
            } else {
                buffer.insert(c);
            }
        }
        buffer
    }

    #[test]
    fn typing_and_reading_back_round_trips() {
        let buffer = typed("Lead with the answer.\nTables for comparisons.");
        assert_eq!(
            buffer.text(),
            "Lead with the answer.\nTables for comparisons."
        );
        assert_eq!(buffer.cursor(), (1, 23));
    }

    /// **A paste of several paragraphs arrives intact**, whichever line ending
    /// the terminal sends. This is the case the whole editor exists for: he
    /// pastes his voice from somewhere else rather than typing it.
    #[test]
    fn a_paste_of_several_paragraphs_arrives_whole() {
        let prose = "First paragraph.\n\nSecond paragraph, longer.\n\nThird.";
        let mut buffer = Buffer::new();
        buffer.paste(prose);
        assert_eq!(buffer.text(), prose);

        let mut crlf = Buffer::new();
        crlf.paste("one\r\ntwo\rthree");
        assert_eq!(crlf.text(), "one\ntwo\nthree");
    }

    /// A paste lands where the cursor is, not at the end.
    #[test]
    fn a_paste_lands_at_the_cursor() {
        let mut buffer = typed("start end");
        for _ in 0..3 {
            buffer.left();
        }
        buffer.paste("middle ");
        assert_eq!(buffer.text(), "start middle end");
    }

    /// **Backspace at the start of a line joins it to the one above.** Every
    /// editor does it and its absence is the first thing anyone notices.
    #[test]
    fn backspace_at_the_start_of_a_line_joins_the_one_above() {
        let mut buffer = typed("one\ntwo");
        buffer.home();
        buffer.backspace();
        assert_eq!(buffer.text(), "onetwo");
        assert_eq!(buffer.cursor(), (0, 3));
    }

    /// Backspace at the very start does nothing rather than panicking.
    #[test]
    fn backspace_at_the_very_start_does_nothing() {
        let mut buffer = Buffer::new();
        buffer.backspace();
        assert_eq!(buffer.text(), "");
        assert_eq!(buffer.cursor(), (0, 0));
    }

    /// Delete forwards is backspace's mirror, joining with the line below.
    #[test]
    fn delete_at_the_end_of_a_line_pulls_the_next_one_up() {
        let mut buffer = typed("one\ntwo");
        buffer.up();
        buffer.end();
        buffer.delete();
        assert_eq!(buffer.text(), "onetwo");
    }

    /// **Arrows wrap between lines**, which is the difference between a text
    /// area and four keys that do nothing at the edges.
    #[test]
    fn the_arrows_move_between_lines_as_well_as_within_them() {
        let mut buffer = typed("one\ntwo");
        buffer.home();
        buffer.left();
        assert_eq!(buffer.cursor(), (0, 3), "left wrapped to the line above");
        buffer.right();
        assert_eq!(buffer.cursor(), (1, 0), "right wrapped back down");

        buffer.up();
        assert_eq!(buffer.cursor(), (0, 0));
        buffer.down();
        assert_eq!(buffer.cursor(), (1, 0));
    }

    /// Up from a long line onto a short one lands at the end of the short one
    /// rather than past it.
    #[test]
    fn moving_onto_a_shorter_line_lands_at_its_end() {
        let mut buffer = typed("ab\nlonger line");
        buffer.end();
        buffer.up();
        assert_eq!(buffer.cursor(), (0, 2));
    }

    /// Nothing typed is an accepted default, and so is whitespace — the same
    /// rule a bare newline follows at a single-line prompt.
    #[test]
    fn whitespace_alone_counts_as_nothing_typed() {
        assert!(Buffer::new().is_empty());
        assert!(typed("   \n  ").is_empty());
        assert!(!typed(" a ").is_empty());
    }

    /// A buffer can start with something already in it, cursor at the end.
    #[test]
    fn a_buffer_can_open_holding_text() {
        let buffer = Buffer::holding("one\ntwo");
        assert_eq!(buffer.text(), "one\ntwo");
        assert_eq!(buffer.cursor(), (1, 3));
    }
}
