//! Walking a YAML document by hand, collecting a refusal per key.
//!
//! **Why this is not `#[derive(Deserialize)]`.**
//! `serde(deny_unknown_fields)` would give the unknown-key hard-fail for free
//! and nothing else this milestone step asks for. It stops at the first fault,
//! so a file with three mistakes takes three edits to load; its message names
//! the field and a line but not the file; and the faults that actually matter
//! here are not field-shaped at all — a name in both `checks` and `commands`, a
//! duplicate step id, a `verdict_routing` that contradicts `structure`. Those
//! are cross-key comparisons a derive cannot express, and a
//! `#[serde(deserialize_with)]` hook can only express by taking the document
//! apart again.
//!
//! So the document is parsed once into an untyped [`Value`] and walked. Every
//! reader here takes the dotted key path it is at and a refusal sink, returns
//! [`None`] on a fault, and **records rather than raises** — which is what lets
//! the walk continue and report the whole file.
//!
//! **`None` means recorded, never means fine.** Every function returning
//! `Option` pushes a [`Refusal`] before returning [`None`], so a caller that
//! sees `None` and carries on is not swallowing an error: it is already in the
//! sink, and the top-level parse refuses if the sink is non-empty. An optional
//! key is read with [`Table::optional`], which returns `None` recording
//! nothing.

use std::collections::BTreeSet;

use serde_yaml_ng::Value;

use crate::error::{Fault, Refusal};

/// What a value is, in the words a message uses. YAML's own type names, so a
/// refusal reads back against the file the author wrote.
pub(crate) fn kind(value: &Value) -> &'static str {
    match value {
        Value::Null => "nothing",
        Value::Bool(_) => "true or false",
        Value::Number(_) => "a number",
        Value::String(_) => "text",
        Value::Sequence(_) => "a list",
        Value::Mapping(_) => "a map",
        Value::Tagged(_) => "a tagged value",
    }
}

/// A mapping being read, and the set of keys taken out of it so far.
///
/// [`Table::close`] is what turns "keys I did not ask for" into refusals, and
/// it consumes the table so it cannot be forgotten after the last read — a
/// forgotten `close` is exactly the unknown-key hole this crate must not have.
pub(crate) struct Table<'a> {
    key: String,
    entries: Vec<(String, &'a Value)>,
    taken: BTreeSet<String>,
}

impl<'a> Table<'a> {
    /// The mapping at `key`, or a refusal naming what was there instead.
    ///
    /// A key that is not text is refused rather than stringified: `1:` and
    /// `"1":` are different keys in YAML and Armada is not going to decide
    /// which one an author meant.
    pub(crate) fn open(key: &str, value: &'a Value, out: &mut Vec<Refusal>) -> Option<Table<'a>> {
        let Value::Mapping(map) = value else {
            out.push(Refusal::new(
                key,
                Fault::WrongType {
                    wanted: "a map",
                    found: kind(value),
                },
            ));
            return None;
        };
        let mut entries = Vec::new();
        for (name, item) in map {
            match name {
                Value::String(name) => entries.push((name.clone(), item)),
                other => out.push(Refusal::new(
                    key,
                    Fault::WrongType {
                        wanted: "text for every name",
                        found: kind(other),
                    },
                )),
            }
        }
        Some(Table {
            key: key.to_string(),
            entries,
            taken: BTreeSet::new(),
        })
    }

    /// The dotted path of a key inside this table.
    pub(crate) fn at(&self, name: &str) -> String {
        if self.key.is_empty() {
            name.to_string()
        } else {
            format!("{}.{name}", self.key)
        }
    }

    /// A key that may be absent. Records nothing when it is.
    pub(crate) fn optional(&mut self, name: &str) -> Option<&'a Value> {
        self.taken.insert(name.to_string());
        self.entries
            .iter()
            .find(|(have, _)| have == name)
            .map(|(_, value)| *value)
    }

    /// A key that must be there. Refuses [`Fault::Missing`] when it is not.
    pub(crate) fn required(&mut self, name: &str, out: &mut Vec<Refusal>) -> Option<&'a Value> {
        match self.optional(name) {
            Some(value) => Some(value),
            None => {
                out.push(Refusal::new(self.at(name), Fault::Missing));
                None
            }
        }
    }

    /// Whether a key is present, without taking it. Used where the key's whole
    /// meaning is a contradiction — `verdict_routing` under `structure: linear`
    /// — so its value is never read and it is still not an unknown key.
    pub(crate) fn present(&self, name: &str) -> bool {
        self.entries.iter().any(|(have, _)| have == name)
    }

    /// Take a key without reading it, so [`Table::close`] does not report it.
    pub(crate) fn ignore(&mut self, name: &str) {
        self.taken.insert(name.to_string());
    }

    /// Whether the mapping held nothing at all. For `verdict_routing`, where
    /// `{}` is a key the author wrote and left blank rather than an absent one
    /// — [`Fault::Empty`]'s distinction, applied to a map instead of a string.
    pub(crate) fn is_empty(&self) -> bool {
        self.entries.is_empty()
    }

    /// Every name in this table, in the order the file wrote them, with its
    /// value. For the open-ended maps — `checks` and `commands` — where the
    /// names are the author's and there is no known set to close against.
    pub(crate) fn into_entries(self) -> Vec<(String, &'a Value)> {
        self.entries
    }

    /// Refuse every key that was never asked for. **Consumes the table.**
    pub(crate) fn close(self, known: &'static [&'static str], out: &mut Vec<Refusal>) {
        for (name, _) in &self.entries {
            if !self.taken.contains(name) {
                out.push(Refusal::new(self.at(name), Fault::Unknown { known }));
            }
        }
    }
}

/// Non-empty text at `key`.
pub(crate) fn text(key: &str, value: &Value, out: &mut Vec<Refusal>) -> Option<String> {
    match value {
        Value::String(found) if !found.trim().is_empty() => Some(found.clone()),
        Value::String(_) => {
            out.push(Refusal::new(key, Fault::Empty));
            None
        }
        other => {
            out.push(Refusal::new(
                key,
                Fault::WrongType {
                    wanted: "text",
                    found: kind(other),
                },
            ));
            None
        }
    }
}

/// A whole number at `key`, of either sign. `expect_exit_code` is the only
/// signed reader in the crate and it stays signed: a shell reports 128 + N for
/// a signal and nothing here is going to normalise that.
pub(crate) fn integer(key: &str, value: &Value, out: &mut Vec<Refusal>) -> Option<i64> {
    match value {
        Value::Number(found) if found.is_i64() => found.as_i64(),
        other => {
            out.push(Refusal::new(
                key,
                Fault::WrongType {
                    wanted: "a whole number",
                    found: kind(other),
                },
            ));
            None
        }
    }
}

/// A whole number at `key` that may be zero, for `retry_limit`.
///
/// Separate from [`positive`] rather than a parameter on it, because the two
/// disagree about zero and each is right about its own key. A `version: 0` is
/// a file that was never versioned; a `retry_limit: 0` is a step saying out
/// loud that its first failure is its last, which is a sentence an author is
/// entitled to write.
pub(crate) fn counted(key: &str, value: &Value, out: &mut Vec<Refusal>) -> Option<u32> {
    match value {
        Value::Number(found) => match found.as_u64().filter(|n| *n <= u64::from(u32::MAX)) {
            Some(n) => u32::try_from(n).ok(),
            None => {
                out.push(Refusal::new(
                    key,
                    Fault::WrongType {
                        wanted: "a whole number of zero or more",
                        found: "a number outside that range",
                    },
                ));
                None
            }
        },
        other => {
            out.push(Refusal::new(
                key,
                Fault::WrongType {
                    wanted: "a whole number of zero or more",
                    found: kind(other),
                },
            ));
            None
        }
    }
}

/// A positive whole number at `key`, for `version`.
pub(crate) fn positive(key: &str, value: &Value, out: &mut Vec<Refusal>) -> Option<u32> {
    match value {
        Value::Number(found) => match found
            .as_u64()
            .filter(|n| *n > 0 && *n <= u64::from(u32::MAX))
        {
            Some(n) => u32::try_from(n).ok(),
            None => {
                out.push(Refusal::new(
                    key,
                    Fault::WrongType {
                        wanted: "a positive whole number",
                        found: "a number outside that range",
                    },
                ));
                None
            }
        },
        other => {
            out.push(Refusal::new(
                key,
                Fault::WrongType {
                    wanted: "a positive whole number",
                    found: kind(other),
                },
            ));
            None
        }
    }
}

/// `true` or `false` at `key`.
pub(crate) fn flag(key: &str, value: &Value, out: &mut Vec<Refusal>) -> Option<bool> {
    match value {
        Value::Bool(found) => Some(*found),
        other => {
            out.push(Refusal::new(
                key,
                Fault::WrongType {
                    wanted: "true or false",
                    found: kind(other),
                },
            ));
            None
        }
    }
}

/// A non-empty list at `key`, each item paired with its own indexed key path.
pub(crate) fn list<'a>(
    key: &str,
    value: &'a Value,
    out: &mut Vec<Refusal>,
) -> Option<Vec<(String, &'a Value)>> {
    let Value::Sequence(items) = value else {
        out.push(Refusal::new(
            key,
            Fault::WrongType {
                wanted: "a list",
                found: kind(value),
            },
        ));
        return None;
    };
    if items.is_empty() {
        out.push(Refusal::new(key, Fault::Empty));
        return None;
    }
    Some(
        items
            .iter()
            .enumerate()
            .map(|(n, item)| (format!("{key}[{n}]"), item))
            .collect(),
    )
}

/// One of a closed set of words at `key`, mapped to the value it names.
///
/// `carried` is what M1 accepts and `legal` is the schema's whole set, which is
/// how a value that is merely deferred gets a different refusal from a value
/// that never existed. The two lists are passed separately rather than derived
/// from each other because only the caller knows which is which.
pub(crate) fn word<T: Copy>(
    key: &str,
    value: &Value,
    carried: &'static [(&'static str, T)],
    legal: &'static [&'static str],
    m1: &'static [&'static str],
    out: &mut Vec<Refusal>,
) -> Option<T> {
    let found = text(key, value, out)?;
    if let Some((_, mapped)) = carried.iter().find(|(word, _)| *word == found) {
        return Some(*mapped);
    }
    let fault = if legal.contains(&found.as_str()) {
        Fault::OutsideM1 {
            value: found,
            carried: m1,
        }
    } else {
        Fault::NotInTheSchema {
            value: found,
            legal,
        }
    };
    out.push(Refusal::new(key, fault));
    None
}
