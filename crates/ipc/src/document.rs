//! One entry written into a JSON document somebody else owns.
//!
//! **The file is not Armada's.** A repository's own agent configuration is
//! written by whoever works there, and Armada registers one server in it. Every
//! other key comes back out, because a publisher that rewrote the file from a
//! struct of its own would delete what it could not model. **Their order does
//! not**: `serde_json`'s object is a sorted map without a feature that would
//! change how every message on the wire is written.
//!
//! Here for `codec`'s reason: reading JSON nobody typed is bytes entering the
//! process. What the entry means and what the file is called are the caller's —
//! `adapters` holds both, because the schema is a vendor's.

use serde::Serialize;
use serde_json::{Map, Value};

use crate::codec::{Undecodable, Unencodable};

/// Write `entry` under `under`.`named`, keeping every other byte of meaning.
///
/// `existing` is `None` where there is no file yet, which is the same answer as
/// an empty document. The result is pretty-printed with a trailing newline and
/// its keys in sorted order: it is a file a person reads and a diff a person
/// reviews, not a wire message.
pub fn merged_into<T: Serialize>(
    existing: Option<&[u8]>,
    under: &str,
    named: &str,
    entry: &T,
) -> Result<String, NotMerged> {
    let mut document = read_object(existing)?;
    let entry = serde_json::to_value(entry).map_err(|why| {
        NotMerged::Unencodable(Unencodable {
            why: why.to_string(),
        })
    })?;

    // **Refused rather than replaced, at both levels.** A document whose root
    // is an array, or whose `under` key holds a string, was written by
    // something with a different idea of the file — and overwriting it to make
    // room for one entry is how a publisher destroys a person's configuration.
    let Value::Object(root) = &mut document else {
        return Err(NotMerged::NotAnObject {
            at: String::from("the document"),
        });
    };
    let servers = root
        .entry(under.to_string())
        .or_insert_with(|| Value::Object(Map::new()));
    let Value::Object(servers) = servers else {
        return Err(NotMerged::NotAnObject {
            at: format!("`{under}`"),
        });
    };
    servers.insert(named.to_string(), entry);

    printed(&document)
}

/// Append `said` to the list at `under`.`into`, keeping every other byte of
/// meaning. [`merged_into`]'s rule for a document somebody else owns, where
/// what is added is one line of a list rather than one keyed entry.
///
/// **`None` where it is already there.** A caller can then say nothing was
/// written rather than moving an mtime and offering a person a diff of nothing,
/// which is [`merged_into`]'s `AlreadyThere` reached one level down.
pub fn appended_to(
    existing: Option<&[u8]>,
    under: &str,
    into: &str,
    said: &str,
) -> Result<Option<String>, NotMerged> {
    let mut document = read_object(existing)?;
    let Value::Object(root) = &mut document else {
        return Err(NotMerged::NotAnObject {
            at: String::from("the document"),
        });
    };
    let held = root
        .entry(under.to_string())
        .or_insert_with(|| Value::Object(Map::new()));
    let Value::Object(held) = held else {
        return Err(NotMerged::NotAnObject {
            at: format!("`{under}`"),
        });
    };
    let list = held
        .entry(into.to_string())
        .or_insert_with(|| Value::Array(Vec::new()));
    let Value::Array(list) = list else {
        return Err(NotMerged::NotAnObject {
            at: format!("`{under}.{into}`"),
        });
    };
    if list.iter().any(|held| held.as_str() == Some(said)) {
        return Ok(None);
    }
    list.push(Value::String(said.to_string()));
    Ok(Some(printed(&document)?))
}

fn read_object(existing: Option<&[u8]>) -> Result<Value, NotMerged> {
    match existing {
        None => Ok(Value::Object(Map::new())),
        Some(bytes) if bytes.iter().all(u8::is_ascii_whitespace) => Ok(Value::Object(Map::new())),
        Some(bytes) => serde_json::from_slice(bytes).map_err(|why| {
            NotMerged::Unreadable(Undecodable {
                expected: "JSON document",
                why: why.to_string(),
            })
        }),
    }
}

fn printed(document: &Value) -> Result<String, NotMerged> {
    let mut written = serde_json::to_string_pretty(document).map_err(|why| {
        NotMerged::Unencodable(Unencodable {
            why: why.to_string(),
        })
    })?;
    written.push('\n');
    Ok(written)
}

/// Why an entry was not written.
///
/// **Every variant leaves the file alone.** A document that would not parse is
/// a document somebody is in the middle of editing or a document written by
/// something else, and neither is improved by Armada replacing it.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum NotMerged {
    Unreadable(Undecodable),
    NotAnObject { at: String },
    Unencodable(Unencodable),
}

impl std::fmt::Display for NotMerged {
    fn fmt(&self, out: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            NotMerged::Unreadable(why) => write!(out, "it is {why}"),
            NotMerged::NotAnObject { at } => {
                write!(
                    out,
                    "{at} is not a JSON object, so nothing can be added to it"
                )
            }
            NotMerged::Unencodable(why) => write!(out, "the entry {why}"),
        }
    }
}

impl std::error::Error for NotMerged {
    fn source(&self) -> Option<&(dyn std::error::Error + 'static)> {
        match self {
            NotMerged::Unreadable(why) => Some(why),
            NotMerged::Unencodable(why) => Some(why),
            NotMerged::NotAnObject { .. } => None,
        }
    }
}
