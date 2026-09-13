//! The document as a value: what an edit should leave, for the splice to be
//! checked against.
//!
//! **Order is compared, not only content.** `serde_yaml_ng`'s map equality
//! ignores order, and the order of `checks:` is the order the gate starts
//! them in — so [`same`] walks both sides pair by pair.

use serde_yaml_ng::{Mapping, Value};

use super::edits::{Node, Op};

/// `text` as a value. An empty document is an empty map; anything that is not
/// YAML, or not a map at the top, is `None`.
pub(super) fn read(text: &str) -> Option<Value> {
    match serde_yaml_ng::from_str::<Value>(text).ok()? {
        Value::Null => Some(Value::Mapping(Mapping::new())),
        map @ Value::Mapping(_) => Some(map),
        _ => None,
    }
}

/// Equal, and in the same order at every level.
pub(super) fn same(left: &Value, right: &Value) -> bool {
    match (left, right) {
        (Value::Mapping(left), Value::Mapping(right)) => {
            left.len() == right.len()
                && left
                    .iter()
                    .zip(right.iter())
                    .all(|((lk, lv), (rk, rv))| lk == rk && same(lv, rv))
        }
        (Value::Sequence(left), Value::Sequence(right)) => {
            left.len() == right.len() && left.iter().zip(right).all(|(l, r)| same(l, r))
        }
        _ => left == right,
    }
}

/// `doc` with `to` at `path`, or with `path` removed. **A map set over a map
/// keeps the old order** for the keys it keeps and appends the new ones, which
/// is what the splice does to the text. `None` where a key above the last is
/// not a map.
pub(super) fn at(doc: &Value, path: &[String], to: Option<&Node>) -> Option<Value> {
    let Some((key, rest)) = path.split_first() else {
        return Some(match to {
            Some(node) => merged(doc, node),
            None => Value::Null,
        });
    };
    let Value::Mapping(map) = doc else {
        return None;
    };
    let mut map = map.clone();
    let name = Value::String(key.clone());
    match (rest.is_empty(), to) {
        (true, None) => {
            map.shift_remove(&name);
        }
        (true, Some(node)) => {
            let next = match map.get(&name) {
                Some(old) => merged(old, node),
                None => value(node),
            };
            set(&mut map, name, next);
        }
        (false, _) => {
            let below = match map.get(&name) {
                Some(old) => at(old, rest, to)?,
                None if to.is_none() => return Some(Value::Mapping(map)),
                None => at(&Value::Mapping(Mapping::new()), rest, to)?,
            };
            set(&mut map, name, below);
        }
    }
    Some(Value::Mapping(map))
}

/// Replace in place, or append — never reorder.
fn set(map: &mut Mapping, key: Value, to: Value) {
    match map.get_mut(&key) {
        Some(slot) => *slot = to,
        None => {
            map.insert(key, to);
        }
    }
}

fn merged(old: &Value, node: &Node) -> Value {
    match (old, node) {
        (Value::Mapping(old), Node::Map(entries)) => {
            let mut map = Mapping::new();
            for (key, was) in old {
                let Value::String(name) = key else { continue };
                if let Some((_, now)) = entries.iter().find(|(k, _)| k == name) {
                    map.insert(key.clone(), merged(was, now));
                }
            }
            for (name, now) in entries {
                let key = Value::String(name.clone());
                if !map.contains_key(&key) {
                    map.insert(key, value(now));
                }
            }
            Value::Mapping(map)
        }
        _ => value(node),
    }
}

/// A node as the value it reads back as.
pub(super) fn value(node: &Node) -> Value {
    match node {
        Node::Text(text) => Value::String(text.clone()),
        Node::Flag(flag) => Value::Bool(*flag),
        Node::Number(number) => Value::Number(number.clone()),
        Node::List(items) => Value::Sequence(items.iter().map(value).collect()),
        Node::Map(entries) => Value::Mapping(
            entries
                .iter()
                .map(|(key, node)| (Value::String(key.clone()), value(node)))
                .collect(),
        ),
    }
}

/// A value as a node, for a place the splice rewrites whole. `None` for what a
/// Manifest never holds — a null, a tag, a key that is not text.
pub(super) fn node(value: &Value) -> Option<Node> {
    Some(match value {
        Value::String(text) => Node::Text(text.clone()),
        Value::Bool(flag) => Node::Flag(*flag),
        Value::Number(number) => Node::Number(number.clone()),
        Value::Sequence(items) => Node::List(items.iter().map(node).collect::<Option<_>>()?),
        Value::Mapping(map) => Node::Map(
            map.iter()
                .map(|(key, value)| Some((key.as_str()?.to_string(), node(value)?)))
                .collect::<Option<_>>()?,
        ),
        Value::Null | Value::Tagged(_) => return None,
    })
}

/// A map set over an existing map becomes one op per key — **sets first, then
/// removals** — so every key the edit leaves alone keeps its lines, comments
/// inside it included. A map never passes through empty on the way.
pub(super) fn expand(doc: &Value, op: Op) -> Vec<Op> {
    let old = op
        .path
        .iter()
        .try_fold(doc, |value, key| value.as_mapping()?.get(key.as_str()));
    let (Some(Value::Mapping(old)), Some(Node::Map(entries))) = (old, &op.to) else {
        return vec![op];
    };
    if entries.is_empty() || old.is_empty() {
        return vec![op];
    }
    let within = |key: &str| {
        let mut path = op.path.clone();
        path.push(key.to_string());
        path
    };
    let mut ops = Vec::new();
    for (key, node) in entries {
        let step = Op {
            path: within(key),
            to: Some(node.clone()),
        };
        ops.extend(expand(doc, step));
    }
    for key in old.keys().filter_map(Value::as_str) {
        if !entries.iter().any(|(name, _)| name == key) {
            ops.push(Op {
                path: within(key),
                to: None,
            });
        }
    }
    ops
}
