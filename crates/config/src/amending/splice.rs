//! One set or removal, as one byte range of the text replaced.
//!
//! **Every edit is a single [`Splice`]**, so what an edit touches is a range
//! that can be named, and everything before and after it is the text it
//! started from.
//!
//! **What a removal takes is the key's own lines** — its key line through its
//! last content line, comments inside it included — **and nothing above it.**
//! A comment written over a Check stays when the Check goes: the owner's rule
//! is that a form leaves every comment it did not touch, and a comment above a
//! key is not the key.

use serde_yaml_ng::{Mapping, Value};

use super::document::{Document, Entry, Kind, Shape, Style};
use super::edits::Node;
use super::{emit, merge};

/// `text[start..end]` becomes `with`.
struct Splice {
    start: usize,
    end: usize,
    with: String,
}

pub(super) fn apply(
    doc: &Document<'_>,
    before: &Value,
    path: &[String],
    to: Option<&Node>,
) -> Result<String, Shape> {
    let map = before.as_mapping().ok_or("a document that is not a map")?;
    let splice = within(doc, 0..doc.lines.len(), None, map, path, to)?;
    Ok(format!(
        "{}{}{}",
        &doc.text[..splice.start],
        splice.with,
        &doc.text[splice.end..]
    ))
}

fn within(
    doc: &Document<'_>,
    region: std::ops::Range<usize>,
    owner: Option<&Entry>,
    map: &Mapping,
    path: &[String],
    to: Option<&Node>,
) -> Result<Splice, Shape> {
    let entries = doc.map(region)?;
    let agrees = entries.len() == map.len()
        && entries
            .iter()
            .zip(map.keys())
            .all(|(entry, key)| key.as_str() == Some(entry.name.as_str()));
    if !agrees {
        return Err("keys that are not written one to a line");
    }
    let (name, rest) = path.split_first().ok_or("an empty key")?;
    let found = entries.iter().position(|entry| &entry.name == name);
    match (found, to) {
        (None, None) => Ok(Splice {
            start: 0,
            end: 0,
            with: String::new(),
        }),
        (None, Some(node)) => {
            let wrapped = rest.iter().rev().fold(node.clone(), |inner, key| {
                Node::Map(vec![(key.clone(), inner)])
            });
            insert(doc, owner, &entries, name, &wrapped)
        }
        (Some(at), _) if rest.is_empty() => match to {
            None => remove(doc, owner, &entries, at),
            Some(node) => set(doc, &entries[at], &map[name.as_str()], node),
        },
        (Some(at), _) => {
            let entry = &entries[at];
            let child = &map[name.as_str()];
            if entry.inline.is_some() {
                let merged = merge::at(child, rest, to).ok_or("a key whose value is not a map")?;
                let node = merge::node(&merged).ok_or("a value a form does not write")?;
                return replace(doc, entry, &node);
            }
            let child = child.as_mapping().ok_or("a key whose value is not a map")?;
            within(
                doc,
                entry.line + 1..entry.last + 1,
                Some(entry),
                child,
                rest,
                to,
            )
        }
    }
}

/// A new key, after the last one in its map and past the comments still inside
/// it — spaced from the key above where this map spaces its keys.
fn insert(
    doc: &Document<'_>,
    owner: Option<&Entry>,
    entries: &[Entry],
    name: &str,
    node: &Node,
) -> Result<Splice, Shape> {
    let nl = doc.newline;
    let (col, last) = match (entries.last(), owner) {
        (Some(sibling), _) => (sibling.col, Some(sibling.last)),
        (None, Some(owner)) => (owner.col + doc.unit, Some(owner.line)),
        (None, None) => (0, None),
    };
    let lines = emit::entry(name, node, col, doc.unit);
    let Some(last) = last else {
        let lead = match doc.text.is_empty() || doc.text.ends_with('\n') {
            true => "",
            false => nl,
        };
        let at = doc.text.len();
        return Ok(Splice {
            start: at,
            end: at,
            with: format!("{lead}{}{nl}", lines.join(nl)),
        });
    };
    let after = &doc.lines[doc.insertion(last, col)];
    let spaced = entries
        .last()
        .is_some_and(|sibling| doc.spaced(sibling.line, col));
    let gap = if spaced { nl } else { "" };
    let with = match after.next == after.end {
        // The last line of a file with no newline at its end, which stays so.
        true => format!("{nl}{gap}{}", lines.join(nl)),
        false => format!("{gap}{}{nl}", lines.join(nl)),
    };
    Ok(Splice {
        start: after.next,
        end: after.next,
        with,
    })
}

/// A key's own lines out. **A map's last key leaves `{}`** rather than a key
/// with no value, which would read as nothing at all.
fn remove(
    doc: &Document<'_>,
    owner: Option<&Entry>,
    entries: &[Entry],
    at: usize,
) -> Result<Splice, Shape> {
    if let (1, Some(owner)) = (entries.len(), owner) {
        return replace(doc, owner, &Node::Map(Vec::new()));
    }
    let entry = &entries[at];
    let mut start = doc.lines[entry.line].start;
    // The blank line that spaced it goes with it, so the keys either side of
    // it are spaced as they were rather than twice.
    let above = entry.line.checked_sub(1).map(|line| doc.lines[line].kind);
    let below = doc.lines.get(entry.last + 1).map(|line| line.kind);
    if above == Some(Kind::Blank) && matches!(below, None | Some(Kind::Blank)) {
        start = doc.lines[entry.line - 1].start;
    }
    // A file with no newline at its end keeps none: the newline before the
    // key's first line goes with the key.
    let last = &doc.lines[entry.last];
    if last.next == last.end && start > 0 {
        let before = &doc.text[..start];
        start -= match before.ends_with("\r\n") {
            true => 2,
            false => usize::from(before.ends_with('\n')),
        };
    }
    Ok(Splice {
        start,
        end: doc.lines[entry.last].next,
        with: String::new(),
    })
}

/// A value over the one a key holds, as narrowly as its shape allows.
fn set(doc: &Document<'_>, entry: &Entry, old: &Value, node: &Node) -> Result<Splice, Shape> {
    let scalar = matches!(node, Node::Text(_) | Node::Flag(_) | Node::Number(_));
    let collection = matches!(old, Value::Sequence(_) | Value::Mapping(_));
    match (&entry.inline, old, node) {
        _ if entry.continued => Err("a value written across lines"),
        (Some(inline), _, _) if inline.style == Style::Other => {
            Err("a tag, an anchor or a block of text")
        }
        (Some(inline), _, _) if scalar && !collection && inline.style != Style::Flow => {
            Ok(Splice {
                start: inline.start,
                end: inline.end,
                with: emit::inline(node, Some(inline.style)).unwrap_or_default(),
            })
        }
        (Some(inline), Value::Sequence(_), Node::List(items)) if inline.style == Style::Flow => {
            match emit::flow_list(items) {
                Some(with) => Ok(Splice {
                    start: inline.start,
                    end: inline.end,
                    with,
                }),
                None => replace(doc, entry, node),
            }
        }
        (None, Value::Sequence(old), Node::List(items)) if !items.is_empty() => {
            list(doc, entry, old, items)
        }
        _ => replace(doc, entry, node),
    }
}

/// A key's whole value rewritten: onto its line where it fits there, or as the
/// lines under it. A comment on the key's line stays.
fn replace(doc: &Document<'_>, entry: &Entry, node: &Node) -> Result<Splice, Shape> {
    if entry.continued {
        return Err("a value written across lines");
    }
    if entry
        .inline
        .as_ref()
        .is_some_and(|inline| inline.style == Style::Other)
    {
        return Err("a tag, an anchor or a block of text");
    }
    let nl = doc.newline;
    let key_line = &doc.lines[entry.line];
    let under = doc.lines[entry.last].end;
    Ok(match (emit::inline(node, None), &entry.inline) {
        (Some(value), Some(inline)) => Splice {
            start: inline.start,
            end: inline.end,
            with: value,
        },
        (Some(value), None) => Splice {
            start: entry.colon,
            end: under,
            with: format!(" {value}{}", &doc.text[entry.colon..key_line.end]),
        },
        (None, Some(inline)) => Splice {
            start: entry.colon,
            end: key_line.end,
            with: format!(
                "{}{nl}{}",
                &doc.text[inline.end..key_line.end],
                emit::block(node, entry.col + doc.unit, doc.unit).join(nl)
            ),
        },
        (None, None) => Splice {
            start: key_line.end,
            end: under,
            with: format!(
                "{nl}{}",
                emit::block(node, entry.col + doc.unit, doc.unit).join(nl)
            ),
        },
    })
}

/// A block list set to `items`. **An item still in the list keeps its own
/// lines**, in written order; a new item is written where it falls among them
/// in the style the list already uses; a dropped item's lines go and the
/// comments between items stay.
fn list(doc: &Document<'_>, entry: &Entry, old: &[Value], items: &[Node]) -> Result<Splice, Shape> {
    let found = doc.list(entry.line + 1..entry.last + 1)?;
    if found.len() != old.len() || found.is_empty() {
        return Err("a list that is not written one item to a line");
    }
    let col = doc.lines[found[0].line].indent;
    let style = found.iter().find_map(|item| item.style);
    let wanted: Vec<Value> = items.iter().map(merge::value).collect();
    let mut kept = vec![None; old.len()];
    let mut from = 0;
    for (new, value) in wanted.iter().enumerate() {
        if let Some(at) = (from..old.len()).find(|at| merge::same(&old[*at], value)) {
            kept[at] = Some(new);
            from = at + 1;
        }
    }
    let nl = doc.newline;
    let start = doc.lines[found[0].line].start;
    let mut out = String::new();
    let mut written = 0;
    let mut end = start;
    let add = |out: &mut String, upto: usize, written: &mut usize| {
        while *written < upto {
            if !out.is_empty() && !out.ends_with('\n') {
                out.push_str(nl);
            }
            for line in emit::item(&items[*written], col, doc.unit, style) {
                out.push_str(&line);
                out.push_str(nl);
            }
            *written += 1;
        }
    };
    for (at, item) in found.iter().enumerate() {
        let item_start = doc.lines[item.line].start;
        out.push_str(&doc.text[end..item_start]);
        end = doc.lines[item.last].next;
        if let Some(new) = kept[at] {
            add(&mut out, new, &mut written);
            out.push_str(&doc.text[item_start..end]);
            written = new + 1;
        }
    }
    add(&mut out, items.len(), &mut written);
    let last = &doc.lines[found[found.len() - 1].last];
    if last.next == last.end && out.ends_with('\n') {
        out.truncate(out.len() - nl.len());
    }
    Ok(Splice {
        start,
        end,
        with: out,
    })
}
