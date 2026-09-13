//! A value as the lines that write it.
//!
//! **Plain wherever plain reads back as the same text**, and otherwise
//! double-quoted. Whether plain is safe is asked of the parser rather than of a
//! list of special characters here: `true`, `08`, `a #b` and `*.rs` all read
//! back as something other than the text, and a list would miss the next one.
//!
//! A value replacing one that was quoted stays quoted, so an edit to a
//! `when:` list of quoted patterns does not leave one pattern bare.

use serde_yaml_ng::Value;

use super::document::Style;
use super::edits::Node;

/// A value that fits on its key's line: a scalar, `{}` or `[]`.
pub(super) fn inline(node: &Node, style: Option<Style>) -> Option<String> {
    Some(match node {
        Node::Text(text) => written(text, style, false),
        Node::Flag(flag) => flag.to_string(),
        Node::Number(number) => number.to_string(),
        Node::List(items) if items.is_empty() => "[]".to_string(),
        Node::Map(entries) if entries.is_empty() => "{}".to_string(),
        Node::List(_) | Node::Map(_) => return None,
    })
}

/// A list of scalars as `[a, b]`, for a list already written that way. `None`
/// where an item is not a scalar.
pub(super) fn flow_list(items: &[Node]) -> Option<String> {
    let written = items
        .iter()
        .map(|item| match item {
            Node::Text(text) => Some(written(text, None, true)),
            Node::Flag(_) | Node::Number(_) => inline(item, None),
            Node::List(_) | Node::Map(_) => None,
        })
        .collect::<Option<Vec<String>>>()?;
    Some(format!("[{}]", written.join(", ")))
}

/// `key: value`, or `key:` and the lines under it, at `col`.
pub(super) fn entry(key: &str, value: &Node, col: usize, unit: usize) -> Vec<String> {
    let head = format!("{}{}:", pad(col), name(key));
    match inline(value, None) {
        Some(value) => vec![format!("{head} {value}")],
        None => {
            let mut lines = vec![head];
            lines.extend(block(value, col + unit, unit));
            lines
        }
    }
}

/// One list item at `col`. A map item starts on the dash's line.
pub(super) fn item(node: &Node, col: usize, unit: usize, style: Option<Style>) -> Vec<String> {
    if let Some(value) = inline(node, style) {
        return vec![format!("{}- {value}", pad(col))];
    }
    let mut lines = block(node, col + 2, unit);
    if let Some(first) = lines.first_mut() {
        *first = format!("{}- {}", pad(col), &first[col + 2..]);
    }
    lines
}

/// A non-empty map or list as the lines under its key, at `col`.
pub(super) fn block(node: &Node, col: usize, unit: usize) -> Vec<String> {
    match node {
        Node::Map(entries) => entries
            .iter()
            .flat_map(|(key, value)| entry(key, value, col, unit))
            .collect(),
        Node::List(items) => items
            .iter()
            .flat_map(|one| item(one, col, unit, None))
            .collect(),
        scalar => vec![format!(
            "{}{}",
            pad(col),
            inline(scalar, None).unwrap_or_default()
        )],
    }
}

fn pad(col: usize) -> String {
    " ".repeat(col)
}

fn written(text: &str, style: Option<Style>, flow: bool) -> String {
    match style {
        Some(Style::Double) => double(text),
        Some(Style::Single) if !text.chars().any(char::is_control) => {
            format!("'{}'", text.replace('\'', "''"))
        }
        _ if plain(text, flow) => text.to_string(),
        _ => double(text),
    }
}

/// A key, plain where it reads back as itself.
fn name(key: &str) -> String {
    let probe = format!("{key}: v\n");
    let reads = !key.is_empty()
        && key == key.trim()
        && !key.chars().any(char::is_control)
        && matches!(
            serde_yaml_ng::from_str::<Value>(&probe),
            Ok(Value::Mapping(map)) if map.len() == 1 && map.contains_key(key)
        );
    match reads {
        true => key.to_string(),
        false => double(key),
    }
}

/// Whether `text` written bare reads back as exactly `text`.
fn plain(text: &str, flow: bool) -> bool {
    if text.is_empty() || text != text.trim() || text.chars().any(char::is_control) {
        return false;
    }
    if flow && text.contains([',', '[', ']', '{', '}']) {
        return false;
    }
    let wanted = Value::String(text.to_string());
    let (probe, wanted) = match flow {
        true => (format!("k: [{text}]\n"), Value::Sequence(vec![wanted])),
        false => (format!("k: {text}\n"), wanted),
    };
    matches!(
        serde_yaml_ng::from_str::<Value>(&probe),
        Ok(Value::Mapping(map)) if map.len() == 1 && map.get("k") == Some(&wanted)
    )
}

fn double(text: &str) -> String {
    let mut out = String::with_capacity(text.len() + 2);
    out.push('"');
    for found in text.chars() {
        match found {
            '"' => out.push_str("\\\""),
            '\\' => out.push_str("\\\\"),
            '\n' => out.push_str("\\n"),
            '\t' => out.push_str("\\t"),
            '\r' => out.push_str("\\r"),
            control if control.is_control() => {
                out.push_str(&format!("\\u{:04X}", u32::from(control)))
            }
            other => out.push(other),
        }
    }
    out.push('"');
    out
}
