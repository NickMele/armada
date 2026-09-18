//! Editing `armada.yml` a key at a time, every other byte left where it was —
//! Journey 9, *Editing*, and `#721`.
//!
//! **A splice, never a re-serialisation**, which would delete every comment.
//! [`document`] reads which bytes a key occupies, over the subset a Manifest is
//! written in — block maps and lists, one-line scalars, `{}`, `[]`, comments —
//! and refuses anything else at the key an edit touches.
//!
//! **Checked rather than trusted.** Each splice is re-parsed and compared, in
//! order, with what the edit should have produced; a misplaced byte arrives as
//! [`NotAmended::Unplaceable`]. **What comes back always loads**: [`Amended`] is
//! built only from text [`Manifest::parse`] accepted.

mod document;
mod drafts;
mod edits;
mod emit;
mod merge;
mod splice;

pub use drafts::{NewCheck, NewCommand, NewEvidence, NewLink, NewNarrowing, NewPort, NewRunner};
pub use edits::{CheckEdit, CommandEdit, Edit, EvidenceEdit, PortEdit};

use std::fmt;
use std::path::Path;

use crate::error::LoadError;
use crate::manifest::Manifest;

use edits::Op;

/// A Manifest's text after edits, and the Manifest it loads as.
///
/// **No constructor but [`amend`]**, and [`amend`] builds one only from text
/// [`Manifest::parse`] accepted — so text that would not load is not a value
/// of this type.
#[derive(Debug)]
pub struct Amended {
    text: String,
    manifest: Manifest,
}

impl Amended {
    /// The edited file, whole.
    pub fn text(&self) -> &str {
        &self.text
    }

    /// What the edited file loads as.
    pub fn manifest(&self) -> &Manifest {
        &self.manifest
    }
}

/// Why the edits did not become a file.
#[derive(Debug)]
pub enum NotAmended {
    /// An edit names a Check, Command, port or section the text does not
    /// declare, or adds one it already does. **The form was drawn from another reading**,
    /// and what a person does next is read the file again.
    Misnamed {
        /// `checks.lint`, `ports.web` — where it was looked for.
        key: String,
        /// `true` where the edit added a name already there.
        declared: bool,
    },
    /// The text at `key` is not one this writer edits without touching more
    /// than the key. The file view makes this edit.
    Unplaceable { key: String, why: Unplaceable },
    /// Every edit was placed and the result would not load. **Every fault, key
    /// by key**, from [`Manifest::parse`] — nothing is written.
    Refused(LoadError),
}

/// What stood in the way of placing an edit.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Unplaceable {
    /// The text the edit started from is not YAML, or is not a map at the top.
    NotAMap,
    /// A shape outside the subset, named in the words a person would search
    /// the file for: `a value written across lines`, `a tag`.
    Shape(&'static str),
    /// The splice re-read as something other than the edit. A guard, not an
    /// expected answer: it means [`document`] misread this file.
    Misread,
}

impl fmt::Display for NotAmended {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            NotAmended::Misnamed {
                key,
                declared: true,
            } => write!(f, "`{key}` is already declared"),
            NotAmended::Misnamed {
                key,
                declared: false,
            } => write!(f, "`{key}` is not declared"),
            NotAmended::Unplaceable { key, why } => match why {
                Unplaceable::NotAMap => {
                    write!(
                        f,
                        "the file is not a map of keys, so `{key}` has nowhere to go"
                    )
                }
                Unplaceable::Shape(shape) => write!(
                    f,
                    "`{key}` sits in {shape}, which a form does not edit — edit it in the file"
                ),
                Unplaceable::Misread => write!(
                    f,
                    "`{key}` could not be edited without touching more than the key — edit it in \
                     the file"
                ),
            },
            NotAmended::Refused(why) => write!(f, "the edited file would not load: {why}"),
        }
    }
}

impl std::error::Error for NotAmended {}

/// Apply `edits`, in order, to `text` — the `armada.yml` at `path` as the edit
/// read it — and load the result.
///
/// **Every byte no edit names is copied through**, comments and blank lines
/// included. An edit that sets a value already there changes nothing.
pub fn amend(path: &Path, text: &str, edits: &[Edit]) -> Result<Amended, NotAmended> {
    let mut current = text.to_string();
    for edit in edits {
        let before = merge::read(&current).ok_or_else(|| edit.unplaceable(Unplaceable::NotAMap))?;
        for op in edit.ops(&before)? {
            for step in merge::expand(&before, op) {
                current = apply(&current, &step)?;
            }
        }
    }
    let manifest = Manifest::parse(path, &current).map_err(NotAmended::Refused)?;
    Ok(Amended {
        text: current,
        manifest,
    })
}

/// One set or removal, spliced and then checked against what it should read as.
fn apply(text: &str, op: &Op) -> Result<String, NotAmended> {
    let refused = |why| NotAmended::Unplaceable { key: op.key(), why };
    let before = merge::read(text).ok_or(refused(Unplaceable::NotAMap))?;
    let expected = merge::at(&before, &op.path, op.to.as_ref()).ok_or(refused(
        Unplaceable::Shape("a key whose value is not a map"),
    ))?;
    if merge::same(&before, &expected) {
        return Ok(text.to_string());
    }
    let doc = document::Document::read(text).map_err(|shape| refused(Unplaceable::Shape(shape)))?;
    let after = splice::apply(&doc, &before, &op.path, op.to.as_ref(), op.attached)
        .map_err(|shape| refused(Unplaceable::Shape(shape)))?;
    match merge::read(&after) {
        Some(reread) if merge::same(&reread, &expected) => Ok(after),
        _ => Err(refused(Unplaceable::Misread)),
    }
}
