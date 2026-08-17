//! Fleet's section of `~/.armada/machine.yml` — **the file that never syncs**
//! (`PLAN.md` §13.1, §4.3.1).
//!
//! One fact lives here: `carry`, a repository root — as the person writes it,
//! `~` abbreviated ([`crate::home::tilde`]) — to the untracked paths that
//! repository needs copied into every worktree `armada fleet spawn` creates
//! for it. `.claude/contamination.local` is gitignored on purpose and `git
//! worktree add` never materialises a gitignored file, so a check that needs
//! it fails in every worktree forever unless something puts it there.
//!
//! # Why a machine fact and not a repository one
//!
//! **The identical argument [`armada_helm::machine::HelmSection`] makes for
//! `helm.enter`.** A gitignored file is gitignored precisely because it names
//! something local to *this* checkout on *this* machine — `.claude/
//! contamination.local` names a private repository, and this one is public.
//! Declaring its path in `armada.yml`, a tracked file, would be the first
//! tracked mention of a local path anywhere in the repository — exactly what
//! `cargo xtask privacy` exists to prevent. Which untracked paths exist, and
//! where, is a fact about this machine's checkout, so it lives in the file
//! that never leaves this box.
//!
//! # One section per module, exactly as Helm's and Guild's are
//!
//! `machine.yml` carries one top-level section per module (`PLAN.md` §4.3.1);
//! this module reads and writes [`SECTION`] and nothing else, and preserves
//! every other section by reading the document as YAML and writing back
//! whatever it did not touch — the same discipline
//! [`armada_helm::machine`](../../helm/src/machine.rs) and
//! [`armada_guild::machine`](../../guild/src/machine.rs) both follow, for the
//! reason their headers give: two siblings sharing one file without one
//! owning the other's schema (`ARCHITECTURE.md` §1.9). This module must not
//! read either of theirs, for the same reason.

use armada_core::error::{ArmadaError, ConfigWhere};
use serde::{Deserialize, Serialize};
use std::collections::BTreeMap;
use std::path::{Component, Path};

/// The key this module's section hangs off.
const SECTION: &str = "fleet";

/// `~/.armada/machine.yml`, for the locator on a refused `carry:` entry.
const FILE: &str = "~/.armada/machine.yml";

/// What Fleet keeps in `machine.yml`.
#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct FleetSection {
    /// Repository root (tilde-form, [`crate::home::tilde`]) to the untracked
    /// paths that repository needs carried into every worktree spawned for
    /// it.
    #[serde(skip_serializing_if = "BTreeMap::is_empty", default)]
    pub carry: BTreeMap<String, Vec<String>>,
}

/// Read Fleet's section, or the default — nothing configured — when the
/// file, the section, or the file itself is absent or will not parse.
///
/// **Not an error, ever, from here**, for the reason
/// [`armada_helm::machine::read`] gives: `armada_manifest::machine::MachineConfig`
/// already reports a `machine.yml` it cannot parse as `environment`, and a
/// second reader raising a second error about the same file would say the
/// same thing twice in two different words. An unconfigured machine — no
/// file, no section, no entry for this repository — is the ordinary case for
/// a fresh clone, not a problem to report.
pub fn read(armada_home: &Path) -> FleetSection {
    let Some(document) = document(armada_home) else {
        return FleetSection::default();
    };
    document
        .get(SECTION)
        .and_then(|section| serde_yaml_ng::from_value(section.clone()).ok())
        .unwrap_or_default()
}

/// Write Fleet's section, leaving every other key in the file exactly as it
/// was.
///
/// **No verb calls this yet.** `carry:` is configured by hand today — the
/// task this shipped for is explicit that the file is edited directly, not
/// through Armada — but the read-write-preserve discipline is the same
/// discipline every section of this file follows, so this exists for the
/// verb that eventually wants to write one, rather than leaving that verb to
/// invent the preserving write a third time.
pub fn write(armada_home: &Path, section: &FleetSection) -> std::io::Result<()> {
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

/// The `carry:` list for one repository, keyed by its tilde-form root
/// ([`crate::home::tilde`]) — empty when this machine has not configured that
/// repository, refused when it has and one of the declared paths is unsafe.
///
/// **Refused here, when the declaration is read, not when a worktree is
/// copied into.** A carry list that can name `../../.ssh` is a worktree that
/// can be handed anything; checking at the one place every caller reads the
/// list through is what makes the refusal universal rather than a property of
/// whichever call site remembered to ask.
///
/// **No entry for this repository is not an error.** A fresh clone on a
/// machine that has never configured `fleet.carry` for it is the ordinary
/// case, not a problem — the whole reason a missing carried path is not an
/// error either ([`crate::worktree::carry`]).
pub fn carry_for(armada_home: &Path, repo_shown: &str) -> Result<Vec<String>, ArmadaError> {
    let section = read(armada_home);
    let Some(paths) = section.carry.get(repo_shown) else {
        return Ok(Vec::new());
    };
    for path in paths {
        if let Some(reason) = escapes_root(path) {
            return Err(ArmadaError::bad_config(
                ConfigWhere::Path {
                    file: FILE.to_string(),
                    path: format!("{SECTION}.carry.{repo_shown}"),
                },
                format!("`{path}` {reason}"),
                "use a path inside the repository, with no leading `/` and no `..` segment",
            ));
        }
    }
    Ok(paths.clone())
}

/// Why a path is not safely repository-relative, or `None` if it is.
///
/// Component-based rather than a regex: rejecting anything that is not
/// `Component::Normal` catches a leading `/`, a bare `.`, and a `..`
/// anywhere in the path, not only at the front.
fn escapes_root(path: &str) -> Option<String> {
    if path.is_empty() {
        return Some("is empty".to_string());
    }
    for component in Path::new(path).components() {
        match component {
            Component::Normal(_) => {}
            Component::CurDir => return Some("contains a `.` segment".to_string()),
            Component::ParentDir => {
                return Some(
                    "contains `..` — carry paths must stay inside the repository root".to_string(),
                )
            }
            Component::RootDir | Component::Prefix(_) => {
                return Some(
                    "is absolute — carry paths are relative to the repository root".to_string(),
                )
            }
        }
    }
    None
}

/// What the file says about itself, on every machine Armada writes one on.
const HEADER: &str = "\
# ~/.armada/machine.yml — this machine, and nothing about you.
#
# NEVER SYNCS (PLAN.md §13.1). fleet.carry names, per repository (tilde-form
# root as its key), the untracked local paths `armada fleet spawn` copies into
# every worktree it creates for it — gitignored config `git worktree add`
# never materialises, such as a repo-local file naming something private in a
# public repository. A repository not listed here simply carries nothing,
# which is the ordinary case for a fresh clone.
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
    fn a_machine_with_no_file_reads_as_nothing_configured() {
        let home = tempfile::tempdir().unwrap();
        assert_eq!(read(home.path()), FleetSection::default());
        assert!(carry_for(home.path(), "~/code/armada").unwrap().is_empty());
    }

    #[test]
    fn a_repository_with_no_entry_carries_nothing() {
        let home = tempfile::tempdir().unwrap();
        std::fs::write(
            home.path().join("machine.yml"),
            "fleet:\n  carry:\n    ~/code/other:\n      - .env.local\n",
        )
        .unwrap();
        assert!(carry_for(home.path(), "~/code/armada").unwrap().is_empty());
    }

    #[test]
    fn a_configured_repositorys_carry_list_is_read_back() {
        let home = tempfile::tempdir().unwrap();
        std::fs::write(
            home.path().join("machine.yml"),
            "fleet:\n  carry:\n    ~/code/armada:\n      - .claude/contamination.local\n",
        )
        .unwrap();
        assert_eq!(
            carry_for(home.path(), "~/code/armada").unwrap(),
            vec![".claude/contamination.local".to_string()]
        );
    }

    /// **The refusal names the path and says why**, exactly as the task
    /// requires — and it fires when the list is read, before any worktree
    /// exists to copy into.
    #[test]
    fn a_carry_path_with_dotdot_is_refused_and_says_why() {
        let home = tempfile::tempdir().unwrap();
        std::fs::write(
            home.path().join("machine.yml"),
            "fleet:\n  carry:\n    ~/code/armada:\n      - ../../.ssh\n",
        )
        .unwrap();
        let error = carry_for(home.path(), "~/code/armada").unwrap_err();
        assert_eq!(error.class, armada_core::error::ErrClass::BadConfig);
        assert!(error.message.contains("../../.ssh"), "{}", error.message);
        assert!(error.message.contains(".."), "{}", error.message);
        assert!(
            error.r#where.contains("~/.armada/machine.yml"),
            "{}",
            error.r#where
        );
    }

    #[test]
    fn a_carry_path_with_a_leading_slash_is_refused() {
        let home = tempfile::tempdir().unwrap();
        std::fs::write(
            home.path().join("machine.yml"),
            "fleet:\n  carry:\n    ~/code/armada:\n      - /etc/passwd\n",
        )
        .unwrap();
        let error = carry_for(home.path(), "~/code/armada").unwrap_err();
        assert!(error.message.contains("absolute"), "{}", error.message);
    }

    /// **Another module's section survives untouched.** This module preserves
    /// what it does not own by reading the whole document and writing it
    /// back, the same discipline `armada_helm::machine` and
    /// `armada_guild::machine` both follow.
    #[test]
    fn writing_this_sections_leaves_every_other_section_alone() {
        let home = tempfile::tempdir().unwrap();
        std::fs::write(home.path().join("machine.yml"), "helm:\n  enter: true\n").unwrap();

        let mut section = FleetSection::default();
        section
            .carry
            .insert("~/code/armada".to_string(), vec![".env".to_string()]);
        write(home.path(), &section).unwrap();

        let text = std::fs::read_to_string(home.path().join("machine.yml")).unwrap();
        let parsed: serde_yaml_ng::Value = serde_yaml_ng::from_str(&text).unwrap();
        assert_eq!(parsed["helm"]["enter"], true);
        assert_eq!(parsed["fleet"]["carry"]["~/code/armada"][0], ".env");
    }

    /// A file this module cannot parse reads as nothing configured, not a
    /// crash — the same fail-safe direction `armada_helm::machine::read`
    /// takes.
    #[test]
    fn an_unparseable_file_reads_as_nothing_configured() {
        let home = tempfile::tempdir().unwrap();
        std::fs::write(home.path().join("machine.yml"), "fleet: [\n").unwrap();
        assert!(carry_for(home.path(), "~/code/armada").unwrap().is_empty());
    }

    /// A section with an unrecognised key is dropped rather than trusted —
    /// the typo guard `deny_unknown_fields` gives every other module's
    /// section, applied here too.
    #[test]
    fn a_mistyped_key_reads_as_nothing_configured() {
        let home = tempfile::tempdir().unwrap();
        std::fs::write(home.path().join("machine.yml"), "fleet:\n  carr: {}\n").unwrap();
        assert!(carry_for(home.path(), "~/code/armada").unwrap().is_empty());
    }
}
