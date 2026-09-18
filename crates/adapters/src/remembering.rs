//! One permission rule written into a person's own agent settings, because
//! they said to remember it. `#1389`.
//!
//! **Three settings files exist and this writes the third.**
//! `~/.claude/settings.json` is theirs across every repository and every tool
//! they use; `.claude/settings.json` is committed and is everybody's;
//! `.claude/settings.local.json` is this person, this repository, and is the
//! file the CLI's own convention keeps out of a commit. A rule a person allowed
//! in one repository belongs in the third, and Armada writes nowhere else.
//!
//! **Only on `allow_and_remember`.** Nothing else in Armada writes a settings
//! file at all, and a person who answers *allow once* leaves no trace here.
//! `crates/adapters/src/mcp.rs` is the precedent for writing into a document
//! Armada does not own and the merge that does not destroy what is in it.

use std::fs;
use std::io;
use std::path::{Path, PathBuf};

/// The file, relative to a repository's root.
pub const PERSONAL_SETTINGS: &str = ".claude/settings.local.json";

/// The object a permission rule goes under, and the list inside it — the agent
/// CLI's own schema.
const PERMISSIONS: &str = "permissions";
const ALLOW: &str = "allow";

/// What remembering did.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Remembered {
    /// The rule was not there. The file was written.
    Written,
    /// The file already allows exactly this. Nothing was touched.
    AlreadyThere,
}

/// Where a repository keeps this person's own settings.
pub fn personal_settings(root: &str) -> PathBuf {
    Path::new(root).join(PERSONAL_SETTINGS)
}

/// Add `rule` to the allow list in `at`, keeping every other key.
///
/// **A failure leaves the file alone and is not a reason to refuse the call.**
/// The person already said yes; a settings file that will not parse is
/// somebody's half-finished edit, and Fleet says so rather than turning their
/// answer into a refusal. `fleet::helm` records that as
/// `allowed_but_not_remembered`.
pub fn remember_the_rule(at: &Path, rule: &str) -> Result<Remembered, io::Error> {
    remember_the_rules(at, std::slice::from_ref(&rule))
}

/// Add every rule in `rules` to the allow list in `at`, keeping every other
/// key, in one read and one write.
///
/// **One write and not one per rule.** A person allowing every read is
/// answering once, and a loop over [`remember_the_rule`] would re-read and
/// rewrite their settings forty times over one press — forty chances for
/// another editor to land between two of them, and forty files on disk that
/// each say something different.
///
/// [`Remembered::AlreadyThere`] is every rule already present. One rule missing
/// out of forty is [`Remembered::Written`], the same as forty missing: what the
/// caller acts on is whether the file changed.
pub fn remember_the_rules(at: &Path, rules: &[&str]) -> Result<Remembered, io::Error> {
    let existing = match fs::read(at) {
        Ok(bytes) => Some(bytes),
        Err(cause) if cause.kind() == io::ErrorKind::NotFound => None,
        Err(cause) => return Err(cause),
    };
    let complained = |why: String| {
        io::Error::new(
            io::ErrorKind::InvalidData,
            format!("{} was left alone: {why}", at.display()),
        )
    };
    // Each rule is appended to what the one before it produced, so the whole
    // set lands or none of it does — the file is not touched until the loop is
    // through.
    let mut carried = existing.clone();
    let mut changed = false;
    for rule in rules {
        let appended = ipc::document::appended_to(carried.as_deref(), PERMISSIONS, ALLOW, rule)
            .map_err(|why| complained(why.to_string()))?;
        if let Some(appended) = appended {
            carried = Some(appended.into_bytes());
            changed = true;
        }
    }
    let written = match changed {
        true => carried,
        false => None,
    };
    let Some(written) = written else {
        return Ok(Remembered::AlreadyThere);
    };
    if let Some(parent) = at.parent().filter(|parent| !parent.as_os_str().is_empty()) {
        fs::create_dir_all(parent)?;
    }
    fs::write(at, written)?;
    Ok(Remembered::Written)
}
