//! Guild's section of `~/.armada/machine.yml` — **the file that never syncs**.
//!
//! Two things land here, and they are both facts about *this machine* rather
//! than about you (`PLAN.md` §13.1):
//!
//! | Key | Written by | Why it is machine-local |
//! |---|---|---|
//! | `guild.remote` | interview question 5 | the remote is reachable with *this* machine's credentials |
//! | `guild.withheld` | the importer's secret guard | a record of what did not travel, so you can go and look |
//!
//! # `withheld` records the key and never the value
//!
//! See [`crate::secrets`] for the argument. In short: the value never moves, it
//! is still in `~/.claude/` where it always was, and `ARCHITECTURE.md` §1.8
//! forbids Armada writing a resolved secret to anything it writes — `machine.yml`
//! included. What is recorded is *that there was one*, and where.
//!
//! # Manifest owns this file and Guild only adds a section to it
//!
//! `machine.yml` is `armada.yml`'s machine-side twin and its six capacity keys
//! belong to Manifest (`PLAN.md` §4.3.1). Guild may not name Manifest
//! (`ARCHITECTURE.md` §1.9), so this module does not read those keys, does not
//! know what they are, and **preserves them by reading the document as YAML and
//! putting back everything it did not touch**. That is not politeness; it is the
//! only way two siblings can share a file without one of them owning the other's
//! schema.

use crate::interview::Answers;
use crate::secrets::Withheld;
use serde::{Deserialize, Serialize};
use std::path::Path;

/// The key Guild's section hangs off.
const SECTION: &str = "guild";

/// What Guild keeps in `machine.yml`.
#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct GuildSection {
    /// The private remote the guild syncs to, if question 5 was answered.
    ///
    /// **Absent means sync is off and `export` still works** — the documented
    /// default, and not a broken state.
    #[serde(skip_serializing_if = "Option::is_none", default)]
    pub remote: Option<String>,
    /// Every value the importer refused to adopt. Keys only.
    #[serde(skip_serializing_if = "Vec::is_empty", default)]
    pub withheld: Vec<Withheld>,
}

/// Read Guild's section, or the default when the file or the section is absent.
///
/// **A `machine.yml` this cannot parse is not an error here.** Manifest reports
/// that, with the right class and the right remedy; a second reader shouting
/// about the same file would give the user two errors for one problem.
pub fn read(armada_home: &Path) -> GuildSection {
    let Some(document) = document(armada_home) else {
        return GuildSection::default();
    };
    document
        .get(SECTION)
        .and_then(|section| serde_yaml_ng::from_value(section.clone()).ok())
        .unwrap_or_default()
}

/// Write Guild's section, leaving every other key in the file exactly as it was.
pub fn write(armada_home: &Path, section: &GuildSection) -> std::io::Result<()> {
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

/// What the file says about itself, on every machine Armada writes one on.
const HEADER: &str = "\
# ~/.armada/machine.yml — this machine, and nothing about you.
#
# NEVER SYNCS (PLAN.md §13.1). Paths, capacity, the guild's remote, and a record
# of what the importer refused to adopt. `armada guild export` excludes it unless
# you pass --include-secrets, and `armada guild import` ignores a bundle's copy
# unless this machine has none.
#
# `guild.withheld` names keys and never values. The values never moved: they are
# still in the file the import read them from, on this machine, working.
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

/// Record what `guild init` decided: the remote it was told about, and what the
/// importer withheld.
pub fn record(
    armada_home: &Path,
    answers: &Answers,
    withheld: &[Withheld],
) -> std::io::Result<GuildSection> {
    let mut section = read(armada_home);
    if let Some(remote) = &answers.remote {
        section.remote = Some(remote.clone());
    }
    section.withheld = withheld.to_vec();
    write(armada_home, &section)?;
    Ok(section)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::secrets::Because;

    fn withheld() -> Vec<Withheld> {
        vec![Withheld {
            source: "settings.json".to_string(),
            key: "env.GITHUB_TOKEN".to_string(),
            because: Because::KeyName,
        }]
    }

    #[test]
    fn a_machine_with_no_file_reads_as_nothing_configured() {
        let home = tempfile::tempdir().unwrap();
        assert_eq!(read(home.path()), GuildSection::default());
        assert_eq!(read(home.path()).remote, None);
    }

    #[test]
    fn what_is_written_is_what_is_read_back() {
        let home = tempfile::tempdir().unwrap();
        let answers = Answers {
            remote: Some("git@example.com:me/guild.git".to_string()),
            ..Answers::default()
        };
        let recorded = record(home.path(), &answers, &withheld()).unwrap();
        assert_eq!(read(home.path()), recorded);
        assert_eq!(
            read(home.path()).remote.as_deref(),
            Some("git@example.com:me/guild.git")
        );
        assert_eq!(read(home.path()).withheld, withheld());
    }

    /// **Manifest's six keys survive.** Guild may not name Manifest, so it does
    /// not know what those keys are — which means the only safe way to write
    /// this file is to put back everything it did not touch.
    #[test]
    fn writing_guilds_section_leaves_manifests_keys_alone() {
        let home = tempfile::tempdir().unwrap();
        std::fs::write(
            home.path().join("machine.yml"),
            "cpu_slots: 6\nport_block_size: 20\n",
        )
        .unwrap();

        record(home.path(), &Answers::all_defaulted(), &withheld()).unwrap();

        let text = std::fs::read_to_string(home.path().join("machine.yml")).unwrap();
        let parsed: serde_yaml_ng::Value = serde_yaml_ng::from_str(&text).unwrap();
        assert_eq!(parsed["cpu_slots"], 6);
        assert_eq!(parsed["port_block_size"], 20);
        assert_eq!(parsed["guild"]["withheld"][0]["key"], "env.GITHUB_TOKEN");
    }

    /// The invariant this whole path exists for: **no value, ever.**
    #[test]
    fn a_withheld_record_carries_no_value_into_the_file() {
        let home = tempfile::tempdir().unwrap();
        record(home.path(), &Answers::all_defaulted(), &withheld()).unwrap();
        let text = std::fs::read_to_string(home.path().join("machine.yml")).unwrap();
        assert!(text.contains("env.GITHUB_TOKEN"), "{text}");
        assert!(
            text.contains("names keys and never values"),
            "the file does not say what it is: {text}"
        );
    }

    /// Sync off is the documented default, and `export` still works — so an
    /// absent remote is written as absent rather than as an empty string that
    /// a later reader would try to fetch from.
    #[test]
    fn no_remote_is_absent_rather_than_empty() {
        let home = tempfile::tempdir().unwrap();
        record(home.path(), &Answers::all_defaulted(), &[]).unwrap();
        let text = std::fs::read_to_string(home.path().join("machine.yml")).unwrap();
        assert!(!text.contains("remote:"), "{text}");
        assert_eq!(read(home.path()).remote, None);
    }
}
