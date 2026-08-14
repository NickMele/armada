//! What a guild holds, counted.
//!
//! One value, produced by reading a directory, and read back by three callers
//! that would otherwise each count for themselves: `guild init`'s import
//! summary, `guild export`'s manifest, and `armada doctor`.
//!
//! **Counting is not a decision, so the counts are facts and nothing here
//! interprets them.** Whether `0 skills` is a problem is `doctor`'s judgement,
//! made against the guild it is looking at; this module only says what is
//! there.

use serde::{Deserialize, Serialize};
use std::path::Path;

/// What a directory of guild content contains.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct Inventory {
    /// Directories under `skills/`, each a skill.
    pub skills: usize,
    /// Files under `hooks/`.
    pub hooks: usize,
    /// Files under `subagents/`.
    pub subagents: usize,
    /// `*.yml` under `workflows/` — the schema is not one.
    pub workflows: usize,
    /// Plugins registered in `plugins.yml`.
    pub plugins: usize,
    /// MCP servers in `mcp.yml`.
    pub mcp_servers: usize,
    /// How many of the three memory fragments exist.
    pub fragments: usize,
}

impl Inventory {
    /// Count what is in a guild, or in a `~/.claude/` about to be imported.
    ///
    /// **A missing directory counts zero rather than failing.** A machine with
    /// no `~/.claude/hooks/` is ordinary, and an import that refused it would
    /// refuse most first runs.
    pub fn of(root: &Path) -> Inventory {
        Inventory {
            skills: entries(&root.join("skills"), Kind::Directory),
            hooks: entries(&root.join("hooks"), Kind::File),
            subagents: entries(&root.join("subagents"), Kind::File),
            workflows: yaml_files(&root.join("workflows")),
            plugins: listed(&root.join("plugins.yml")),
            mcp_servers: listed(&root.join("mcp.yml")),
            fragments: crate::memory::FRAGMENTS
                .iter()
                .filter(|name| root.join(name).is_file())
                .count(),
        }
    }

    /// The facts, in the order the summary line prints them, leaving out
    /// whatever is zero.
    ///
    /// **A zero is left out rather than printed.** `0 hooks` is a fact nobody
    /// was asking about, on the line whose job is to say the guild is not
    /// empty — the same rule `armada manifest clean`'s summary follows.
    pub fn facts(&self) -> Vec<String> {
        let mut out = Vec::new();
        for (count, singular) in [
            (self.skills, "skill"),
            (self.hooks, "hook"),
            (self.subagents, "subagent"),
            (self.workflows, "workflow"),
            (self.plugins, "plugin"),
            (self.mcp_servers, "MCP server"),
        ] {
            if count > 0 {
                out.push(plural(count, singular));
            }
        }
        out
    }

    /// Whether there is anything here at all.
    pub fn is_empty(&self) -> bool {
        self.facts().is_empty() && self.fragments == 0
    }
}

/// `1 skill`, `19 skills`. English pluralisation of the six nouns above, which
/// all take a plain `s`.
fn plural(count: usize, singular: &str) -> String {
    if count == 1 {
        format!("{count} {singular}")
    } else {
        format!("{count} {singular}s")
    }
}

enum Kind {
    File,
    Directory,
}

fn entries(directory: &Path, kind: Kind) -> usize {
    let Ok(listing) = std::fs::read_dir(directory) else {
        return 0;
    };
    listing
        .filter_map(Result::ok)
        .filter(|entry| {
            // A dotfile is the editor's or the shell's, never content.
            if entry.file_name().to_string_lossy().starts_with('.') {
                return false;
            }
            match kind {
                Kind::File => entry.path().is_file(),
                Kind::Directory => entry.path().is_dir(),
            }
        })
        .count()
}

/// `*.yml` under a directory, **excluding the schema**. `workflow.schema.json`
/// is not a workflow, and counting it would report five starters where four
/// were copied.
fn yaml_files(directory: &Path) -> usize {
    let Ok(listing) = std::fs::read_dir(directory) else {
        return 0;
    };
    listing
        .filter_map(Result::ok)
        .filter(|entry| {
            entry
                .path()
                .extension()
                .is_some_and(|extension| extension == "yml" || extension == "yaml")
        })
        .count()
}

/// How many entries a `plugins.yml` or `mcp.yml` lists.
///
/// Both files are a mapping of name to configuration at their top level, so the
/// count is the number of top-level keys. An unreadable or absent file is zero,
/// for the same reason a missing directory is.
fn listed(file: &Path) -> usize {
    let Ok(text) = std::fs::read_to_string(file) else {
        return 0;
    };
    let Ok(parsed) = serde_yaml_ng::from_str::<serde_yaml_ng::Value>(&text) else {
        return 0;
    };
    match parsed {
        serde_yaml_ng::Value::Mapping(map) => map
            .values()
            .map(|value| match value {
                serde_yaml_ng::Value::Mapping(inner) => inner.len(),
                serde_yaml_ng::Value::Sequence(items) => items.len(),
                _ => 1,
            })
            .sum(),
        serde_yaml_ng::Value::Sequence(items) => items.len(),
        _ => 0,
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::fs;

    fn a_guild() -> tempfile::TempDir {
        let root = tempfile::tempdir().unwrap();
        let path = root.path();
        fs::create_dir_all(path.join("skills/add-migration")).unwrap();
        fs::create_dir_all(path.join("skills/triage-flake")).unwrap();
        fs::create_dir_all(path.join("hooks")).unwrap();
        fs::create_dir_all(path.join("subagents")).unwrap();
        fs::create_dir_all(path.join("workflows")).unwrap();
        fs::write(path.join("hooks/stop-notify.sh"), "#!/bin/sh\n").unwrap();
        fs::write(path.join("subagents/helm.md"), "# Helm\n").unwrap();
        for name in ["design", "plan", "feature", "bug"] {
            fs::write(path.join(format!("workflows/{name}.yml")), "name: x\n").unwrap();
        }
        fs::write(path.join("workflows/workflow.schema.json"), "{}").unwrap();
        fs::write(path.join("voice.md"), "brief\n").unwrap();
        fs::write(
            path.join("plugins.yml"),
            "marketplaces:\n  local: {}\nenabled:\n  - a\n  - b\n",
        )
        .unwrap();
        root
    }

    #[test]
    fn a_guild_is_counted_by_what_is_in_it() {
        let root = a_guild();
        let inventory = Inventory::of(root.path());
        assert_eq!(inventory.skills, 2);
        assert_eq!(inventory.hooks, 1);
        assert_eq!(inventory.subagents, 1);
        assert_eq!(inventory.fragments, 1);
    }

    /// `workflow.schema.json` is not a workflow. Counting it would report five
    /// starters where four were copied, on the line whose only job is to say
    /// what was copied.
    #[test]
    fn the_schema_is_not_counted_as_a_workflow() {
        let root = a_guild();
        assert_eq!(Inventory::of(root.path()).workflows, 4);
    }

    /// A machine with no `~/.claude/` at all is ordinary, and import has to
    /// survive it.
    #[test]
    fn a_directory_that_is_not_there_counts_zero_rather_than_failing() {
        let inventory = Inventory::of(Path::new("/nonexistent/nowhere"));
        assert_eq!(inventory, Inventory::default());
        assert!(inventory.is_empty());
        assert!(inventory.facts().is_empty());
    }

    /// The summary line, and the rule that a zero is left off it.
    #[test]
    fn the_facts_read_as_a_summary_line_and_leave_out_the_zeroes() {
        let root = a_guild();
        let facts = Inventory::of(root.path()).facts();
        assert_eq!(
            facts,
            vec![
                "2 skills".to_string(),
                "1 hook".to_string(),
                "1 subagent".to_string(),
                "4 workflows".to_string(),
                "3 plugins".to_string(),
            ]
        );
        assert!(
            !facts.iter().any(|fact| fact.starts_with('0')),
            "a zero reached the summary: {facts:?}"
        );
    }
}
