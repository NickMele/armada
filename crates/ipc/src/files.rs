//! Searching the checkout a request dispatches into, for the `@` mention popup
//! a person opens while typing a request or a brief — `search_files` in
//! `crates/ipc/operations.toml`.
//!
//! **Named by `?manifest_id=`, not in the body.** A Fleet serves several
//! repositories; an absent one reads the checkout Fleet was started in.

use serde::{Deserialize, Serialize};

/// What was found, narrowed against a person's typed text.
///
/// **Capped, not exhaustive.** `crates/fleet/src/files.rs` stops once it has
/// enough, so a repository nobody has sized cannot turn one keystroke into an
/// unbounded walk. Paths are relative to the repository root, forward-slashed,
/// and never `.git`, `node_modules` or `target` — see that module's own notes
/// for what a fuller answer would still have to settle.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct FilesFound {
    pub paths: Vec<String>,
}
