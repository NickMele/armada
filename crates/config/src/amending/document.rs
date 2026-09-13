//! Where each key of a Manifest sits in its text.
//!
//! **A reading of lines, not a YAML parser.** A key is a line at a map's
//! column; its value is what follows the colon, or the lines indented under
//! it. That is the whole of the subset a Manifest is written in, and it is
//! enough to say which bytes one key occupies. Anything else on a line this
//! reads is refused by name — a quoted key with escapes, a tag, a value that
//! runs across lines — and the refusal is only reached where an edit touches
//! it.
//!
//! **Comments are never content.** A comment line belongs to no key, so no
//! extent computed here starts or ends on one, and a splice between two keys
//! copies every comment around it through.

use std::ops::Range;

/// Why a place in the text could not be read, in words a person would search
/// the file for.
pub(super) type Shape = &'static str;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(super) enum Kind {
    Blank,
    Comment,
    Content,
}

/// One line, by byte offsets into the text.
#[derive(Debug, Clone)]
pub(super) struct Line {
    pub(super) start: usize,
    /// Before the newline, and before a `\r` ahead of it.
    pub(super) end: usize,
    /// Where the next line starts, which is `end` on a last line with no
    /// newline.
    pub(super) next: usize,
    pub(super) indent: usize,
    pub(super) kind: Kind,
}

/// How a one-line value is written. Kept so a changed value is written the way
/// the old one was.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(super) enum Style {
    Plain,
    Double,
    Single,
    /// `{…}` or `[…]` on one line.
    Flow,
    /// A tag, an anchor, an alias or a block of text — never edited here.
    Other,
}

/// A value written on its key's line.
#[derive(Debug, Clone)]
pub(super) struct Inline {
    pub(super) start: usize,
    pub(super) end: usize,
    pub(super) style: Style,
}

/// One key of a block map.
#[derive(Debug, Clone)]
pub(super) struct Entry {
    pub(super) name: String,
    /// The key's line.
    pub(super) line: usize,
    pub(super) col: usize,
    /// The byte just after the key's colon.
    pub(super) colon: usize,
    pub(super) inline: Option<Inline>,
    /// The last content line that belongs to this key. Comments after it are
    /// not the key's.
    pub(super) last: usize,
    /// Indented lines under a value already written on the key's line: a value
    /// that runs across lines, which nothing here edits.
    pub(super) continued: bool,
}

/// One item of a block list.
#[derive(Debug, Clone)]
pub(super) struct Item {
    pub(super) line: usize,
    pub(super) last: usize,
    /// How a one-line text item is written. `None` for anything else.
    pub(super) style: Option<Style>,
}

pub(super) struct Document<'t> {
    pub(super) text: &'t str,
    pub(super) lines: Vec<Line>,
    pub(super) newline: &'static str,
    /// How far a nested key sits in from its parent, as this file does it.
    pub(super) unit: usize,
}

impl<'t> Document<'t> {
    pub(super) fn read(text: &'t str) -> Result<Document<'t>, Shape> {
        let mut lines = Vec::new();
        let mut start = 0;
        while start < text.len() {
            let (end, next) = match text[start..].find('\n') {
                Some(at) => (start + at, start + at + 1),
                None => (text.len(), text.len()),
            };
            let end = match text[start..end].ends_with('\r') {
                true => end - 1,
                false => end,
            };
            let body = &text[start..end];
            let indent = body.len() - body.trim_start_matches(' ').len();
            let rest = &body[indent..];
            let kind = match rest {
                _ if rest.trim().is_empty() => Kind::Blank,
                _ if rest.starts_with('#') => Kind::Comment,
                _ => Kind::Content,
            };
            if kind == Kind::Content && rest.starts_with('\t') {
                return Err("a line indented with a tab");
            }
            let marker = rest == "---" || rest.starts_with("--- ") || rest == "...";
            if kind == Kind::Content && indent == 0 && (marker || rest.starts_with('%')) {
                return Err("a document marker or a directive");
            }
            lines.push(Line {
                start,
                end,
                next,
                indent,
                kind,
            });
            start = next;
        }
        let unit = lines
            .iter()
            .filter(|line| line.kind == Kind::Content && line.indent > 0)
            .map(|line| line.indent)
            .min()
            .unwrap_or(2);
        let newline = match text.contains("\r\n") {
            true => "\r\n",
            false => "\n",
        };
        Ok(Document {
            text,
            lines,
            newline,
            unit,
        })
    }

    fn body(&self, line: usize) -> &'t str {
        let line = &self.lines[line];
        &self.text[line.start + line.indent..line.end]
    }

    fn first_content(&self, region: Range<usize>) -> Option<usize> {
        region
            .into_iter()
            .find(|at| self.lines[*at].kind == Kind::Content)
    }

    /// The keys of the block map whose lines are `region`, in written order.
    pub(super) fn map(&self, region: Range<usize>) -> Result<Vec<Entry>, Shape> {
        let Some(first) = self.first_content(region.clone()) else {
            return Ok(Vec::new());
        };
        let col = self.lines[first].indent;
        let mut entries: Vec<Entry> = Vec::new();
        for at in region {
            let line = &self.lines[at];
            if line.kind != Kind::Content {
                continue;
            }
            if line.indent < col {
                return Err("a key indented less than the keys beside it");
            }
            let item = dash(self.body(at));
            if line.indent == col && !item {
                entries.push(self.entry(at, col)?);
                continue;
            }
            let Some(entry) = entries.last_mut() else {
                return Err("a value with no key above it");
            };
            if entry.inline.is_some() {
                if line.indent == col {
                    return Err("a list item under a key that already has a value");
                }
                entry.continued = true;
            }
            entry.last = at;
        }
        Ok(entries)
    }

    /// The items of the block list whose lines are `region`.
    pub(super) fn list(&self, region: Range<usize>) -> Result<Vec<Item>, Shape> {
        let Some(first) = self.first_content(region.clone()) else {
            return Ok(Vec::new());
        };
        let col = self.lines[first].indent;
        let mut items: Vec<Item> = Vec::new();
        for at in region {
            let line = &self.lines[at];
            if line.kind != Kind::Content {
                continue;
            }
            if line.indent < col {
                return Err("a list item indented less than the items beside it");
            }
            if line.indent > col {
                let Some(item) = items.last_mut() else {
                    return Err("a value with no list item above it");
                };
                item.last = at;
                continue;
            }
            let body = self.body(at);
            if !dash(body) {
                return Err("a key where a list item was expected");
            }
            let after = body[1..].trim_start();
            let style = match after.as_bytes().first() {
                None => None,
                Some(b'"') => Some(Style::Double),
                Some(b'\'') => Some(Style::Single),
                Some(b'{' | b'[' | b'&' | b'*' | b'!' | b'|' | b'>') => None,
                Some(_) if after.contains(": ") || after.ends_with(':') => None,
                Some(_) => Some(Style::Plain),
            };
            items.push(Item {
                line: at,
                last: at,
                style,
            });
        }
        Ok(items)
    }

    fn entry(&self, at: usize, col: usize) -> Result<Entry, Shape> {
        let line = &self.lines[at];
        let from = line.start + col;
        let (name, after) = key(&self.text[from..line.end])?;
        let colon = from + after;
        let rest = &self.text[colon..line.end];
        let value = colon + (rest.len() - rest.trim_start().len());
        let written = &self.text[value..line.end];
        let inline = match written.is_empty() || written.starts_with('#') {
            true => None,
            false => Some(inline(self.text, value, line.end)?),
        };
        Ok(Entry {
            name,
            line: at,
            col,
            colon,
            inline,
            last: at,
            continued: false,
        })
    }

    /// The line after which a new key at `col` goes, for a map whose last
    /// content line is `last`: past any comment still indented as its keys
    /// are, which is a comment inside the map.
    pub(super) fn insertion(&self, last: usize, col: usize) -> usize {
        let mut at = last;
        for next in last + 1..self.lines.len() {
            match self.lines[next].kind {
                Kind::Blank => continue,
                Kind::Comment if self.lines[next].indent >= col => at = next,
                _ => break,
            }
        }
        at
    }

    /// Whether the key on `line` has a blank line above it, past its own
    /// comments — which is whether this map spaces its keys apart.
    pub(super) fn spaced(&self, line: usize, col: usize) -> bool {
        for above in (0..line).rev() {
            match self.lines[above].kind {
                Kind::Comment if self.lines[above].indent >= col => continue,
                Kind::Blank => return true,
                _ => return false,
            }
        }
        false
    }
}

fn dash(body: &str) -> bool {
    body == "-" || body.starts_with("- ")
}

/// A key and the offset just past its colon, from a line's text at the map's
/// column.
fn key(body: &str) -> Result<(String, usize), Shape> {
    let refused = "a line that is not a plain key";
    match body.as_bytes().first() {
        Some(quote @ (b'"' | b'\'')) => {
            let close = body[1..]
                .find(*quote as char)
                .ok_or("a quoted key written across lines")?
                + 1;
            let name = &body[1..close];
            if name.contains('\\') || body[close + 1..].starts_with(*quote as char) {
                return Err("a quoted key with an escape in it");
            }
            let after = &body[close + 1..];
            match after.strip_prefix(':') {
                Some(rest) if rest.is_empty() || rest.starts_with(' ') => {
                    Ok((name.to_string(), close + 2))
                }
                _ => Err(refused),
            }
        }
        None | Some(b'?' | b'[' | b'{' | b'&' | b'*' | b'!' | b'|' | b'>' | b'%' | b'@' | b'`') => {
            Err(refused)
        }
        Some(_) => {
            for (at, found) in body.char_indices() {
                if found == '#' && body[..at].ends_with(' ') {
                    break;
                }
                if found != ':' {
                    continue;
                }
                let rest = &body[at + 1..];
                if rest.is_empty() || rest.starts_with(' ') {
                    let name = body[..at].trim_end();
                    return match name.is_empty() {
                        true => Err(refused),
                        false => Ok((name.to_string(), at + 1)),
                    };
                }
            }
            Err(refused)
        }
    }
}

/// The value written from `start` to the end of its line, without a comment
/// after it.
fn inline(text: &str, start: usize, end: usize) -> Result<Inline, Shape> {
    let written = &text[start..end];
    let bytes = written.as_bytes();
    let (length, style) = match bytes[0] {
        b'"' => {
            let mut at = 1;
            loop {
                match bytes.get(at) {
                    None => return Err("a quoted value written across lines"),
                    Some(b'\\') => at += 2,
                    Some(b'"') => break,
                    Some(_) => at += 1,
                }
            }
            (at + 1, Style::Double)
        }
        b'\'' => {
            let mut at = 1;
            loop {
                match (bytes.get(at), bytes.get(at + 1)) {
                    (None, _) => return Err("a quoted value written across lines"),
                    (Some(b'\''), Some(b'\'')) => at += 2,
                    (Some(b'\''), _) => break,
                    _ => at += 1,
                }
            }
            (at + 1, Style::Single)
        }
        b'{' | b'[' => (flow(bytes)?, Style::Flow),
        b'&' | b'*' | b'!' | b'|' | b'>' => {
            return Ok(Inline {
                start,
                end,
                style: Style::Other,
            })
        }
        _ => {
            let stop = written.find(" #").unwrap_or(written.len());
            (written[..stop].trim_end().len(), Style::Plain)
        }
    };
    let tail = &written[length..];
    let trimmed = tail.trim_start();
    if !trimmed.is_empty() && !(trimmed.starts_with('#') && trimmed.len() < tail.len()) {
        return Err("more than one value on a line");
    }
    Ok(Inline {
        start,
        end: start + length,
        style,
    })
}

/// The length of a `{…}` or `[…]` that closes on its own line.
fn flow(bytes: &[u8]) -> Result<usize, Shape> {
    let (mut depth, mut quote) = (0usize, None::<u8>);
    let mut at = 0;
    while at < bytes.len() {
        match (quote, bytes[at]) {
            (Some(b'"'), b'\\') => at += 1,
            (Some(open), found) if found == open => quote = None,
            (Some(_), _) => {}
            (None, found @ (b'"' | b'\'')) => quote = Some(found),
            (None, b'{' | b'[') => depth += 1,
            (None, b'}' | b']') => {
                depth -= 1;
                if depth == 0 {
                    return Ok(at + 1);
                }
            }
            (None, _) => {}
        }
        at += 1;
    }
    Err("a list or map written across lines")
}
