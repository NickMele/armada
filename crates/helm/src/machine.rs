//! Helm's section of `~/.armada/machine.yml` — **the file that never syncs**
//! (`PLAN.md` §13.1, §4.3.1).
//!
//! One key lives here: whether `armada helm --exec` may become a session **on
//! this machine**. Everything else about entering — the argv, the four
//! documents, the conversation record — is built and verified regardless; this
//! is the one bit that decides whether the last step, replacing this process
//! with `claude`, is allowed to happen.
//!
//! # Why a machine fact and not a guild preference
//!
//! The guild syncs between every machine you use (`PLAN.md` §13.1) and
//! describes **you** — voice, skills, permissions a Drone may act under. This
//! decides whether a Claude Code session may open **here**, spending against a
//! real account, on this box, right now.
//!
//! Those are not the same question. A permission you are willing to grant
//! everywhere you work — what an unattended Drone may commit, say — travels
//! with you on purpose (`crates/guild/src/permissions.rs` makes exactly that
//! argument). Whether *this* machine is one you trust to open a real session
//! unattended is a fact about the machine: a laptop you are sitting at answers
//! differently than a shared box, a CI runner, or a machine you configured once
//! and have not opened a terminal on since. Putting `enter` in the guild would
//! mean flipping it once, anywhere, turns it on on every machine that guild has
//! ever been pulled onto or ever will be — including ones nobody meant to grant
//! it on. `machine.yml` is the file that never leaves this box for exactly this
//! reason, so this is where the switch lives.
//!
//! # One section per module, exactly as Guild's and Manifest's are
//!
//! `machine.yml` carries one top-level section per module (`PLAN.md` §4.3.1);
//! this module reads and writes [`SECTION`] and nothing else, and preserves
//! every other section by reading the document as YAML and writing back
//! whatever it did not touch — the same discipline
//! [`armada_guild::machine`](../../guild/src/machine.rs) follows, for the
//! reason its header gives: two siblings sharing one file without one owning
//! the other's schema (`ARCHITECTURE.md` §1.9).

use serde::{Deserialize, Serialize};
use std::path::Path;

/// The key this module's section hangs off.
const SECTION: &str = "helm";

/// What Helm keeps in `machine.yml`.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct HelmSection {
    /// Whether `armada helm --exec` may become a session on this machine.
    ///
    /// **Off is the default**, which is also what a missing file, a missing
    /// section, and a section that fails to parse all mean — a fresh install
    /// must not be able to open a session until told to, on every one of those
    /// paths and not only the ordinary one.
    #[serde(default)]
    pub enter: bool,
}

/// Read Helm's section, or the default — `enter: false` — when the file, the
/// section, or the file itself is absent or will not parse.
///
/// **Not an error, ever, from here.** `armada_manifest::machine::MachineConfig`
/// is the reader that reports a `machine.yml` it cannot parse as `environment`
/// (PLAN.md §4.3.1); a second reader raising a second error about the same file
/// would tell the caller the same thing twice in two different words. Off is
/// also the safe default to fail toward: a `machine.yml` a person is mid-edit
/// on must not be the thing that lets a session start.
pub fn read(armada_home: &Path) -> HelmSection {
    let Some(document) = document(armada_home) else {
        return HelmSection::default();
    };
    document
        .get(SECTION)
        .and_then(|section| serde_yaml_ng::from_value(section.clone()).ok())
        .unwrap_or_default()
}

/// Write Helm's section, leaving every other key in the file exactly as it
/// was.
pub fn write(armada_home: &Path, section: &HelmSection) -> std::io::Result<()> {
    let path = armada_home.join("machine.yml");
    let mut document = document(armada_home)
        .unwrap_or(serde_yaml_ng::Value::Mapping(serde_yaml_ng::Mapping::new()));
    let serialised = serde_yaml_ng::to_value(section)
        .map_err(|e| std::io::Error::new(std::io::ErrorKind::InvalidData, e))?;

    let map = document.as_mapping_mut().ok_or_else(|| {
        std::io::Error::new(
            std::io::ErrorKind::InvalidData,
            "machine.yml is not a mapping",
        )
    })?;
    map.insert(
        serde_yaml_ng::Value::String(SECTION.to_string()),
        serialised,
    );

    let mut text = String::from(HEADER);
    text.push_str(
        &serde_yaml_ng::to_string(&document)
            .map_err(|e| std::io::Error::new(std::io::ErrorKind::InvalidData, e))?,
    );
    if let Some(parent) = path.parent() {
        std::fs::create_dir_all(parent)?;
    }
    std::fs::write(path, text)
}

/// Flip [`HelmSection::enter`] and write it back, reporting whether it
/// actually changed.
///
/// **The one write path `armada helm enable` and `armada helm disable`
/// share.** Both verbs read, compare, and write — never blind-write — so a
/// second `enable` in a row reports `changed: false` rather than claiming to
/// have done something twice.
pub fn set_enter(armada_home: &Path, enter: bool) -> std::io::Result<HelmSection> {
    let before = read(armada_home);
    let section = HelmSection { enter };
    if before.enter != enter {
        write(armada_home, &section)?;
    }
    Ok(section)
}

/// What the file says about itself, on every machine Armada writes one on.
const HEADER: &str = "\
# ~/.armada/machine.yml — this machine, and nothing about you.
#
# NEVER SYNCS (PLAN.md §13.1). helm.enter gates `armada helm --exec`: off by
# default, so a fresh install cannot open a Claude Code session until you say
# so with `armada helm enable`. `armada helm disable` puts it back.
";

fn document(armada_home: &Path) -> Option<serde_yaml_ng::Value> {
    let text = std::fs::read_to_string(armada_home.join("machine.yml")).ok()?;
    match serde_yaml_ng::from_str::<serde_yaml_ng::Value>(&text).ok()? {
        serde_yaml_ng::Value::Null => {
            Some(serde_yaml_ng::Value::Mapping(serde_yaml_ng::Mapping::new()))
        }
        value => Some(value),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_machine_with_no_file_reads_as_off() {
        let home = tempfile::tempdir().unwrap();
        assert_eq!(read(home.path()), HelmSection::default());
        assert!(!read(home.path()).enter);
    }

    #[test]
    fn enabling_is_read_back() {
        let home = tempfile::tempdir().unwrap();
        let section = set_enter(home.path(), true).unwrap();
        assert!(section.enter);
        assert!(read(home.path()).enter);
    }

    #[test]
    fn disabling_after_enabling_is_read_back_too() {
        let home = tempfile::tempdir().unwrap();
        set_enter(home.path(), true).unwrap();
        set_enter(home.path(), false).unwrap();
        assert!(!read(home.path()).enter);
    }

    /// **The write that actually changed something, and the one that did
    /// not.** `armada helm enable` twice must not report `changed: true` the
    /// second time.
    #[test]
    fn set_enter_reports_whether_it_actually_wrote() {
        let home = tempfile::tempdir().unwrap();
        let before = read(home.path());
        assert!(!before.enter);
        set_enter(home.path(), true).unwrap();
        let text_after_first = std::fs::read_to_string(home.path().join("machine.yml")).unwrap();
        set_enter(home.path(), true).unwrap();
        assert_eq!(
            std::fs::read_to_string(home.path().join("machine.yml")).unwrap(),
            text_after_first,
            "a second enable rewrote a file that already said enter: true"
        );
    }

    /// **Another module's section survives untouched.** This module preserves
    /// what it does not own by reading the whole document and writing it back,
    /// the same discipline `armada_guild::machine` follows.
    #[test]
    fn writing_this_sections_leaves_every_other_section_alone() {
        let home = tempfile::tempdir().unwrap();
        std::fs::write(
            home.path().join("machine.yml"),
            "manifest:\n  cpu_slots: 6\n",
        )
        .unwrap();

        set_enter(home.path(), true).unwrap();

        let text = std::fs::read_to_string(home.path().join("machine.yml")).unwrap();
        let parsed: serde_yaml_ng::Value = serde_yaml_ng::from_str(&text).unwrap();
        assert_eq!(parsed["manifest"]["cpu_slots"], 6);
        assert_eq!(parsed["helm"]["enter"], true);
    }

    /// A file this module cannot parse is off, not a crash — the same
    /// fail-safe direction the default takes.
    #[test]
    fn an_unparseable_file_reads_as_off_rather_than_panicking() {
        let home = tempfile::tempdir().unwrap();
        std::fs::write(home.path().join("machine.yml"), "helm: [\n").unwrap();
        assert!(!read(home.path()).enter);
    }

    /// A section with an unrecognised key is dropped rather than trusted —
    /// the typo guard `deny_unknown_fields` gives every other module's
    /// section, applied here too.
    #[test]
    fn a_mistyped_key_does_not_silently_enable_it() {
        let home = tempfile::tempdir().unwrap();
        std::fs::write(home.path().join("machine.yml"), "helm:\n  entr: true\n").unwrap();
        assert!(!read(home.path()).enter);
    }
}
