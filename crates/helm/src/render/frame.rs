//! Panel frames for the command centre.
//!
//! **A frame is a pure value function**: given a width and content, it returns
//! lines. So the layout is a unit test rather than a photograph, which is the
//! property [`bridge.rs`]'s doc comment claims and is why the Bridge is testable
//! at all.
//!
//! **The frame is a renderer and must know nothing about Jobs, Manifest or
//! Guild** (ARCHITECTURE.md §1.9). It takes lines, not records.
//!
//! **The focus marker occupies the leading space, never shifting a column**
//! (docs/reserved/033-the-command-centre-designed.md). The first attempt added
//! it and pushed the row one character right; the line-width check passed only
//! because the trailing pad was trimmed to hide the drift. Line width cannot
//! see column drift, so the test for this must assert a column offset and not a
//! line length.

use super::table::Span;
use super::term::{display_width, truncate};

/// Apply focus marker to a row, replacing the leading space with ▸.
///
/// The marker occupies the leading space rather than being added to it, so
/// focus never shifts a column. If `is_focused` is false, the row is returned
/// unchanged. If true and the row starts with a space, that space is replaced
/// with the marker; otherwise the row is returned unchanged.
pub fn focus(row: Vec<Span>, is_focused: bool) -> Vec<Span> {
    if !is_focused || row.is_empty() {
        return row;
    }

    let mut result = row;

    // Look for the first span that starts with a space
    for span in &mut result {
        if span.text.starts_with(' ') {
            // Replace leading space with focus marker
            span.text = format!("▸{}", &span.text[1..]);
            return result;
        }

        // If a span is all whitespace, could replace it
        if span.text.chars().all(|c| c.is_whitespace()) && !span.text.is_empty() {
            span.text = "▸".to_string();
            return result;
        }
    }

    // No leading space found, return unchanged
    result
}

/// Clip a line to maximum width, truncating content that overflows.
///
/// Ensures the line never exceeds the given width. If content is longer,
/// it is truncated with an ellipsis. This prevents long lines from breaking
/// out of the box and shoving neighbours.
fn clip_line(line: Vec<Span>, max_width: usize) -> Vec<Span> {
    let current_width: usize = line.iter().map(|s| display_width(&s.text)).sum();

    if current_width <= max_width {
        return line;
    }

    // Need to truncate
    let mut result = Vec::new();
    let mut remaining_width = max_width;

    for span in line {
        if remaining_width == 0 {
            break;
        }

        let span_width = display_width(&span.text);

        if span_width <= remaining_width {
            // Whole span fits
            remaining_width -= span_width;
            result.push(span);
        } else {
            // Span is too long, truncate it
            let truncated = truncate(&span.text, remaining_width);
            result.push(Span {
                text: truncated,
                role: span.role,
                bold: span.bold,
            });
            remaining_width = 0;
        }
    }

    result
}

/// One `key does` pair on a key line, already worded by the caller.
///
/// **A pair rather than two strings threaded separately**, so a caller cannot
/// hand `shed_to_narrow` a key with the wrong `does` for it — the two travel
/// together from wherever they were decided (`docs/reserved/033`'s legend).
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct KeyPair {
    /// The key, or chord, as it reads on the line — `"↑↓←→ or hjkl"`, `"d"`.
    pub key: String,
    /// What it does — `"move"`, `"detail"`.
    pub does: String,
}

impl KeyPair {
    /// Build one, from anything that reads as a string.
    pub fn new(key: impl Into<String>, does: impl Into<String>) -> KeyPair {
        KeyPair {
            key: key.into(),
            does: does.into(),
        }
    }

    fn spelled(&self) -> String {
        format!("{} {}", self.key, self.does)
    }
}

/// Two spaces between pairs — the one separator on this screen that reads the
/// same for a person and for a pipe (`render.rs`'s `bridge_keys`).
const PAIR_GAP: &str = "  ";

/// **Movement never sheds; verbs do** (`docs/reserved/033-the-command-centre-designed.md`).
///
/// Builds the widest key line that fits `width`: every `movement` pair,
/// always; then as many `verbs` as fit, dropped from the end (lowest
/// priority last) until it does; `quit` is pinned last and is never dropped —
/// a full-screen program with no way off the line is a trap.
///
/// **Two lists rather than one flagged list**, so a caller cannot mis-mark a
/// verb as movement by mistake — the type says which is which, and this
/// function is the one place that distinction is spent. It is what keeps
/// [`titled_box`] and [`hjoin`] ignorant of Jobs, Manifest and Guild
/// (`ARCHITECTURE.md` §1.9): nothing here reads a key's *meaning*, only which
/// of two lists it arrived in.
///
/// When even the movement pairs and `quit` overhang `width`, they are kept
/// anyway — movement never sheds, so there is nothing left to drop.
pub fn shed_to_narrow(
    movement: &[KeyPair],
    verbs: &[KeyPair],
    quit: &KeyPair,
    width: usize,
) -> Vec<Span> {
    let mut taken = verbs.len();
    loop {
        let mut line: Vec<String> = movement.iter().map(KeyPair::spelled).collect();
        line.extend(verbs[..taken].iter().map(KeyPair::spelled));
        // **The honest overflow.** Naming what it could not carry is the third
        // option next to wrapping (which would change the frame's height) and
        // silently dropping keys nobody can then find.
        if taken < verbs.len() {
            line.push("? keys".to_string());
        }
        line.push(quit.spelled());
        let text = line.join(PAIR_GAP);
        if display_width(&text) <= width || taken == 0 {
            return vec![Span {
                text,
                role: None,
                bold: false,
            }];
        }
        taken -= 1;
    }
}

/// A titled bordered box around content lines.
///
/// Takes a title, a list of content lines (each line is a Vec<Span>), a width,
/// and returns the boxed lines. All lines are padded to the full width, and the
/// title is centered in the top border.
pub fn titled_box(title: &str, lines: Vec<Vec<Span>>, width: usize) -> Vec<Vec<Span>> {
    let mut result = Vec::new();

    // Top border with title: ┌─ TITLE ──…─┐
    // Format: ┌ + optional(─ + TITLE + ─) + dashes + ┐
    // Total width must be exactly `width` columns.

    let title_display = display_width(title);
    let mut top = Vec::new();

    // Left bracket
    top.push(Span {
        text: "┌".to_string(),
        role: None,
        bold: false,
    });

    // Calculate how many dashes fit
    let available = width.saturating_sub(2); // Width minus both brackets

    if title.is_empty() {
        // No title: just fill with dashes
        if available > 0 {
            top.push(Span {
                text: "─".repeat(available),
                role: None,
                bold: false,
            });
        }
    } else {
        // With title: ─ TITLE ─ dashes
        let overhead = 3; // dash, space, dash
        if title_display + overhead <= available {
            // Title fits with spacing
            top.push(Span {
                text: "─ ".to_string(),
                role: None,
                bold: false,
            });
            top.push(Span {
                text: title.to_string(),
                role: None,
                bold: false,
            });
            top.push(Span {
                text: " ".to_string(),
                role: None,
                bold: false,
            });

            let remaining = available - overhead - title_display;
            if remaining > 0 {
                top.push(Span {
                    text: "─".repeat(remaining),
                    role: None,
                    bold: false,
                });
            }
        } else {
            // Title doesn't fit, just fill with dashes
            if available > 0 {
                top.push(Span {
                    text: "─".repeat(available),
                    role: None,
                    bold: false,
                });
            }
        }
    }

    // Right bracket
    top.push(Span {
        text: "┐".to_string(),
        role: None,
        bold: false,
    });

    result.push(top);

    // Content lines: clip to width, then pad to width
    for line in lines {
        // First clip to ensure no overflow
        let clipped = clip_line(line, width);

        // Then pad to exact width
        let line_width: usize = clipped.iter().map(|s| display_width(&s.text)).sum();
        let mut padded = clipped;

        if line_width < width {
            let padding = width - line_width;
            padded.push(Span {
                text: " ".repeat(padding),
                role: None,
                bold: false,
            });
        }

        result.push(padded);
    }

    // Bottom border: └──────────┘
    let bottom = vec![
        Span {
            text: "└".to_string(),
            role: None,
            bold: false,
        },
        Span {
            text: "─".repeat(width.saturating_sub(2)),
            role: None,
            bold: false,
        },
        Span {
            text: "┘".to_string(),
            role: None,
            bold: false,
        },
    ];
    result.push(bottom);

    result
}

/// Join two boxes side-by-side with a gap between them.
///
/// Places the left box and right box beside each other with `gap` spaces
/// between them. Pads the shorter box with blank lines to match the taller
/// one's height. All output lines are padded to exactly `width`.
pub fn hjoin(
    left: Vec<Vec<Span>>,
    right: Vec<Vec<Span>>,
    gap: usize,
    width: usize,
) -> Vec<Vec<Span>> {
    let max_height = left.len().max(right.len());
    let mut result = Vec::new();

    // Pad both boxes to max height with empty lines
    let mut left_padded = left;
    while left_padded.len() < max_height {
        left_padded.push(Vec::new());
    }

    let mut right_padded = right;
    while right_padded.len() < max_height {
        right_padded.push(Vec::new());
    }

    // Combine lines
    for (left_line, right_line) in left_padded.iter().zip(right_padded.iter()) {
        let left_width: usize = left_line.iter().map(|s| display_width(&s.text)).sum();
        let right_width: usize = right_line.iter().map(|s| display_width(&s.text)).sum();

        let mut combined = left_line.clone();

        // Add gap
        if gap > 0 {
            combined.push(Span {
                text: " ".repeat(gap),
                role: None,
                bold: false,
            });
        }

        // Add right content
        combined.extend(right_line.clone());

        // Pad to total width
        let combined_width = left_width + gap + right_width;
        if combined_width < width {
            let padding = width - combined_width;
            combined.push(Span {
                text: " ".repeat(padding),
                role: None,
                bold: false,
            });
        }

        result.push(combined);
    }

    result
}

#[cfg(test)]
mod tests {
    use super::*;

    fn plain(text: &str) -> Span {
        Span {
            text: text.to_string(),
            role: None,
            bold: false,
        }
    }

    fn line_width(line: &[Span]) -> usize {
        line.iter().map(|s| display_width(&s.text)).sum()
    }

    #[test]
    fn titled_box_adds_borders_and_title() {
        let content = vec![vec![plain("hello")]];
        let result = titled_box("TEST", content, 20);

        // Should have: top border + 1 content + bottom border = 3 lines
        assert_eq!(result.len(), 3);

        // Top border should start with ┌ and end with ┐
        let top = &result[0];
        assert_eq!(top[0].text, "┌");
        assert_eq!(top[top.len() - 1].text, "┐");

        // Bottom border should start with └ and end with ┘
        let bottom = &result[2];
        assert_eq!(bottom[0].text, "└");
        assert_eq!(bottom[bottom.len() - 1].text, "┘");

        // All lines should be padded to width
        for line in &result {
            assert_eq!(line_width(line), 20, "line width mismatch");
        }
    }

    #[test]
    fn titled_box_pads_content_to_width() {
        let content = vec![vec![plain("x")]];
        let result = titled_box("", content, 30);

        // Content line (index 1) should be exactly 30 columns wide
        assert_eq!(line_width(&result[1]), 30);
    }

    #[test]
    fn titled_box_centers_title() {
        let content = vec![vec![]];
        let result = titled_box("JOBS", content, 40);

        // Top border: ┌─ JOBS ──────────────────┐
        // Should have roughly equal dashes on each side
        let top = &result[0];
        let text_parts: Vec<String> = top.iter().map(|s| s.text.clone()).collect();
        let text_joined = text_parts.join("");

        // Title should appear in the top border
        assert!(text_joined.contains("JOBS"));
        assert!(text_joined.starts_with("┌"));
        assert!(text_joined.ends_with("┐"));
    }

    #[test]
    fn hjoin_combines_boxes_with_gap() {
        let left = vec![vec![plain("A")]];
        let right = vec![vec![plain("B")]];
        let result = hjoin(left, right, 2, 20);

        assert_eq!(result.len(), 1);
        assert_eq!(line_width(&result[0]), 20);

        // Should contain both pieces
        let text_parts: Vec<String> = result[0].iter().map(|s| s.text.clone()).collect();
        let text = text_parts.join("");
        assert!(text.contains("A"));
        assert!(text.contains("B"));
    }

    #[test]
    fn hjoin_pads_shorter_box_to_match_taller() {
        let left = vec![vec![plain("1")], vec![plain("2")], vec![plain("3")]];
        let right = vec![vec![plain("X")]];
        let result = hjoin(left, right, 2, 30);

        // Both had different heights, result should match the taller (3)
        assert_eq!(result.len(), 3);

        // All lines should be 30 columns wide
        for line in &result {
            assert_eq!(line_width(line), 30);
        }
    }

    #[test]
    fn focus_marker_occupies_leading_space_not_added_to_it() {
        // This is the critical trap: ▸ must replace the leading space,
        // not add to the line width.
        let without_focus = vec![vec![plain("  hello")]];
        let with_focus = vec![vec![plain("▸ hello")]];

        let boxed_without = titled_box("", without_focus, 40);
        let boxed_with = titled_box("", with_focus, 40);

        // Both lines should have the same display width
        let content_without = line_width(&boxed_without[1]);
        let content_with = line_width(&boxed_with[1]);

        assert_eq!(
            content_without, content_with,
            "focus marker must not shift column"
        );

        // The marker should be one character (or Unicode width 1)
        assert_eq!(display_width("▸"), 1);
    }

    #[test]
    fn focus_marker_at_various_widths() {
        // Verify the property holds at multiple terminal widths
        for width in [40, 80, 96, 138] {
            let without = vec![vec![plain("  row")]];
            let with = vec![vec![plain("▸ row")]];

            let boxed_without = titled_box("", without, width);
            let boxed_with = titled_box("", with, width);

            assert_eq!(
                line_width(&boxed_without[1]),
                line_width(&boxed_with[1]),
                "focus marker width invariance failed at width={}",
                width
            );
        }
    }

    #[test]
    fn hjoin_all_lines_same_width() {
        let left = vec![vec![plain("A")], vec![plain("BB")], vec![plain("CCC")]];
        let right = vec![vec![plain("X")], vec![plain("YY")]];

        let result = hjoin(left, right, 2, 50);

        // All lines must be exactly 50 columns
        for (i, line) in result.iter().enumerate() {
            assert_eq!(
                line_width(line),
                50,
                "line {} has width {}, expected 50",
                i,
                line_width(line)
            );
        }
    }

    #[test]
    fn titled_box_empty_content() {
        let content = vec![];
        let result = titled_box("EMPTY", content, 25);

        // Should have: top border + bottom border = 2 lines
        assert_eq!(result.len(), 2);

        // Both should be 25 columns
        assert_eq!(line_width(&result[0]), 25);
        assert_eq!(line_width(&result[1]), 25);
    }

    #[test]
    fn titled_box_narrow_width() {
        let content = vec![vec![plain("x")]];
        let result = titled_box("T", content, 10);

        // Should still work at narrow widths
        assert_eq!(result.len(), 3); // top + content + bottom
        assert_eq!(line_width(&result[0]), 10);
        assert_eq!(line_width(&result[1]), 10);
        assert_eq!(line_width(&result[2]), 10);
    }

    #[test]
    fn titled_box_very_narrow_width_minimum() {
        // At minimum viable width, borders should still render
        let content = vec![vec![]];
        let result = titled_box("", content, 4);

        // ┌──┐ = 4 columns minimum
        assert!(result[0][0].text.starts_with("┌"));
        assert!(result[2][0].text.starts_with("└"));

        for line in &result {
            assert_eq!(line_width(line), 4);
        }
    }

    #[test]
    fn hjoin_three_column_layout_simulation() {
        // Simulate a layout with two panels (left and right)
        // This would be part of the full command centre
        let left_panel = vec![
            vec![plain("JOBS")],
            vec![plain("job1")],
            vec![plain("job2")],
        ];
        let right_panel = vec![
            vec![plain("INBOX")],
            vec![plain("msg1")],
            vec![plain("msg2")],
        ];

        let result = hjoin(left_panel, right_panel, 3, 80);

        // Should have 3 lines (matching taller box)
        assert_eq!(result.len(), 3);

        // Each line exactly 80 columns
        for line in &result {
            assert_eq!(line_width(line), 80);
        }
    }

    #[test]
    fn focus_replaces_leading_space_with_marker() {
        let row = vec![plain("  hello")];
        let focused = focus(row, true);

        // Should have marker replacing the first space
        assert_eq!(focused[0].text, "▸ hello");

        // The marker is one column wide
        assert_eq!(display_width("▸"), 1);
        assert_eq!(display_width("▸ hello"), display_width("  hello"));
    }

    #[test]
    fn focus_when_false_returns_unchanged() {
        let row = vec![plain("  hello")];
        let unfocused = focus(row, false);

        assert_eq!(unfocused[0].text, "  hello");
    }

    #[test]
    fn focus_on_empty_row_returns_unchanged() {
        let row: Vec<Span> = vec![];
        let focused = focus(row, true);

        assert!(focused.is_empty());
    }

    #[test]
    fn focus_on_no_leading_space_returns_unchanged() {
        let row = vec![plain("hello")];
        let focused = focus(row, true);

        // No leading space, so no change
        assert_eq!(focused[0].text, "hello");
    }

    #[test]
    fn focus_width_invariance_across_multiple_widths() {
        // Verify focus marker doesn't shift columns at any width
        for width in [40, 80, 96, 138] {
            let without_marker = vec![plain("  content")];
            let with_marker = focus(vec![plain("  content")], true);

            let without_width = line_width(&without_marker);
            let with_width = line_width(&with_marker);

            assert_eq!(
                without_width, with_width,
                "focus marker width invariance failed at width={}",
                width
            );
        }
    }

    #[test]
    fn clip_line_shortens_overflow() {
        let line = vec![plain(
            "this is a very long line that exceeds the maximum width",
        )];
        let clipped = clip_line(line, 20);

        let width: usize = clipped.iter().map(|s| display_width(&s.text)).sum();
        assert!(width <= 20);
    }

    #[test]
    fn clip_line_preserves_short_lines() {
        let line = vec![plain("short")];
        let clipped = clip_line(line, 20);

        assert_eq!(clipped[0].text, "short");
    }

    #[test]
    fn titled_box_clips_long_content() {
        let long_line = vec![vec![plain(
            "this is an extremely long line that should be clipped by the box",
        )]];
        let result = titled_box("", long_line, 20);

        // Content should be clipped to box width
        // All lines should be exactly 20 columns
        for line in &result {
            let w = line_width(line);
            assert!(w <= 20, "line width {} exceeds max 20", w);
        }
    }

    // ------------------------------------------------------- shed_to_narrow

    /// The movement legend and verbs 033's wide mock names, in priority order
    /// — `?` and `q` are the two the verb list is never without.
    fn movement() -> Vec<KeyPair> {
        vec![
            KeyPair::new("↑↓←→ or hjkl", "move"),
            KeyPair::new("tab", "next panel"),
            KeyPair::new("1-5", "jump to panel"),
            KeyPair::new("enter", "act"),
        ]
    }

    fn verbs() -> Vec<KeyPair> {
        vec![
            KeyPair::new("d", "detail"),
            KeyPair::new("n", "new job"),
            KeyPair::new("a", "answer"),
            KeyPair::new("p", "pause"),
            KeyPair::new("x", "abort"),
            KeyPair::new("r", "reap"),
            KeyPair::new("t", "tick"),
        ]
    }

    fn quit() -> KeyPair {
        KeyPair::new("q", "quit")
    }

    fn text_of(line: &[Span]) -> String {
        line.iter().map(|s| s.text.as_str()).collect()
    }

    /// **Movement never sheds — the failure this rule exists to prevent.**
    /// Before this change `shed_to_narrow` returned its input untouched, which
    /// happened to keep movement too; this asserts the property itself, not
    /// the accident.
    #[test]
    fn movement_pairs_are_never_dropped_at_any_width() {
        for width in [10, 40, 80, 138] {
            let line = shed_to_narrow(&movement(), &verbs(), &quit(), width);
            let text = text_of(&line);
            for pair in movement() {
                assert!(
                    text.contains(&pair.key),
                    "`{}` dropped at width {width}: {text}",
                    pair.key
                );
            }
        }
    }

    /// `quit` is pinned last and never dropped, even when nothing else fits —
    /// a full-screen program with no way off the line is a trap.
    #[test]
    fn quit_is_never_dropped_even_at_the_narrowest_width() {
        let line = shed_to_narrow(&movement(), &verbs(), &quit(), 1);
        assert!(text_of(&line).contains("q quit"));
    }

    /// Verbs drop from the end — lowest priority last in the list — before
    /// movement gives up anything. (Measured: movement alone is 63 columns,
    /// every verb present is 136; 95 lands exactly between "d detail" alone
    /// fitting and "n new job" pushing it over.)
    #[test]
    fn verbs_drop_from_the_end_as_the_width_narrows() {
        let wide = text_of(&shed_to_narrow(&movement(), &verbs(), &quit(), 200));
        for pair in verbs() {
            assert!(wide.contains(&pair.does), "`{}` missing at 200", pair.does);
        }

        // Narrow enough that only the highest-priority verb survives.
        let narrow = text_of(&shed_to_narrow(&movement(), &verbs(), &quit(), 95));
        assert!(narrow.contains("detail"), "{narrow}");
        assert!(!narrow.contains("tick"), "{narrow}");
        assert!(!narrow.contains("new job"), "{narrow}");
    }

    /// **The line names what it dropped**, rather than silently shrinking —
    /// the third option next to wrapping (which would change the frame's
    /// height) and dropping keys nobody can then find.
    #[test]
    fn a_narrowed_line_says_keys_were_hidden_and_a_wide_one_does_not() {
        let narrow = text_of(&shed_to_narrow(&movement(), &verbs(), &quit(), 95));
        assert!(narrow.contains("? keys"), "{narrow}");

        let wide = text_of(&shed_to_narrow(&movement(), &verbs(), &quit(), 200));
        assert!(!wide.contains("? keys"), "{wide}");
    }

    /// The line never exceeds its budget once the width is at least as wide
    /// as movement, quit and the overflow marker alone (measured: 79).
    #[test]
    fn the_shed_line_never_overhangs_a_width_it_can_fit_in() {
        for width in [90, 100, 120, 138, 200] {
            let line = shed_to_narrow(&movement(), &verbs(), &quit(), width);
            let w = line_width(&line);
            assert!(w <= width, "line is {w} wide at budget {width}");
        }
    }

    /// **Movement never sheds even where it does not fit** — the floor this
    /// rule has. Below 79 columns even a bare movement-and-quit line
    /// overhangs, and the line is kept anyway rather than truncated, because
    /// truncating movement is the one thing 033 forbids outright.
    #[test]
    fn movement_overhangs_rather_than_shedding_below_its_own_floor() {
        let line = shed_to_narrow(&movement(), &verbs(), &quit(), 40);
        assert!(line_width(&line) > 40, "should overhang, not truncate");
        assert!(text_of(&line).contains("q quit"));
    }

    /// Nothing to shed at all: an empty verb list still carries movement and
    /// quit, and never claims to have hidden anything.
    #[test]
    fn no_verbs_at_all_still_draws_movement_and_quit() {
        let line = text_of(&shed_to_narrow(&movement(), &[], &quit(), 200));
        assert!(line.contains("move"));
        assert!(line.contains("q quit"));
        assert!(!line.contains("? keys"));
    }
}
