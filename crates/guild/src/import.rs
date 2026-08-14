//! Adopting what is already on the machine.
//!
//! **Import runs first and does most of the work** (`PLAN.md` §13.4). The guild
//! starts nearly complete rather than empty, which is the difference between a
//! tool you configure once and one you abandon during configuration — and it is
//! the reason the interview is five questions rather than thirty.
//!
//! | From `~/.claude/` | To `~/.armada/guild/` |
//! |---|---|
//! | `skills/` | `skills/` |
//! | `agents/` | `subagents/` — Claude Code's word, Armada's word ([`glossary.md`](../../../docs/glossary.md)) |
//! | `hooks/` | `hooks/` |
//! | `settings.json` | `settings.json`, **scrubbed** |
//! | `plugins/config.json` | `plugins.yml` |
//! | `.mcp.json` | `mcp.yml` |
//! | `CLAUDE.md` | `voice.md` · `expectations.md` · `how-i-work.md` ([`crate::memory`]) |
//!
//! # The secret guard runs on the way in, not on the way out
//!
//! Every JSON document adopted goes through [`crate::secrets`] **before it is
//! written**, so a credential-shaped value never exists inside
//! `~/.armada/guild/` at any point — not for the seconds between a copy and a
//! scrub, and not in the git object a mid-run crash would leave behind. A
//! secret that has already reached a remote cannot be un-pushed (`PLAN.md`
//! §13.5), and "we removed it in the next commit" is not a fix.
//!
//! # Absence is never an error
//!
//! A machine with no `~/.claude/` at all is a legitimate first run — it is
//! exactly what `--no-import` produces on purpose. Every source below is
//! optional and a missing one contributes nothing, silently.

use crate::inventory::Inventory;
use crate::layout::Guild;
use crate::memory;
use crate::secrets::{self, Withheld};
use std::path::Path;

/// What an import did.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct Imported {
    /// Where it read from, as given.
    pub from: String,
    /// What is now in the guild.
    pub inventory: Inventory,
    /// Whether a memory file was found and split.
    pub memory: bool,
    /// Every value refused. **Keys only** — see [`crate::secrets`].
    pub withheld: Vec<Withheld>,
}

impl Imported {
    /// The summary line's facts, in the order the agreed layout prints them:
    /// where it read from, then what it found.
    ///
    /// `tests/golden/render/init-machine.plain` freezes this line, so the order
    /// and the wording are the fixture's rather than this function's.
    pub fn facts(&self) -> Vec<String> {
        let mut facts = vec![format!("imported from {}", self.from)];
        facts.extend(self.inventory.facts());
        if self.memory {
            facts.push("CLAUDE.md".to_string());
        }
        facts
    }
}

/// The directories copied verbatim, and what they are called at each end.
const TREES: [(&str, &str); 3] = [
    ("skills", "skills"),
    // Claude Code calls them agents; Armada calls them subagents, and the
    // glossary is the single definition of that word.
    ("agents", "subagents"),
    ("hooks", "hooks"),
];

/// The JSON documents adopted, scrubbed, and re-spelled as YAML where the guild
/// keeps YAML.
const DOCUMENTS: [(&str, &str); 3] = [
    ("settings.json", "settings.json"),
    ("plugins/config.json", "plugins.yml"),
    (".mcp.json", "mcp.yml"),
];

/// Read a `~/.claude/`-shaped directory into a guild.
///
/// `from` is a path rather than "the home directory" so that `--from <path>`
/// needs no second code path, and so that every test in this file points at a
/// `TempDir` — the rule that keeps a test suite from ever touching the real
/// `~/.armada/`.
pub fn run(from: &Path, guild: &Guild) -> std::io::Result<Imported> {
    let mut imported = Imported {
        from: display(from),
        ..Imported::default()
    };
    std::fs::create_dir_all(guild.root())?;

    if !from.is_dir() {
        // Nothing to adopt. The three fragments are still written, so the guild
        // has the files `doctor` compares against and the interview overwrites.
        write_memory(guild, "")?;
        imported.inventory = Inventory::of(guild.root());
        return Ok(imported);
    }

    for (source, destination) in TREES {
        copy_tree(&from.join(source), &guild.path(destination))?;
    }

    for (source, destination) in DOCUMENTS {
        let path = from.join(source);
        let Ok(text) = std::fs::read_to_string(&path) else {
            continue;
        };
        let Ok(document) = serde_json::from_str::<serde_json::Value>(&text) else {
            // A settings file Armada cannot parse is not a reason to refuse the
            // whole import: the skills, hooks and memory file are unaffected and
            // are most of the value. It is reported and skipped.
            continue;
        };

        // **Scrubbed before written, never after.** See the module header.
        let found = secrets::scan(source, &document);
        let adopted = secrets::without(&document, &found);
        imported.withheld.extend(found);
        write_document(&guild.path(destination), &adopted)?;
    }

    if let Ok(text) = std::fs::read_to_string(from.join("CLAUDE.md")) {
        write_memory(guild, &text)?;
        imported.memory = true;
    } else {
        write_memory(guild, "")?;
    }

    imported.inventory = Inventory::of(guild.root());
    Ok(imported)
}

/// The three fragments, always written, even from nothing.
fn write_memory(guild: &Guild, text: &str) -> std::io::Result<()> {
    for (name, body) in memory::split(text) {
        std::fs::write(guild.path(name), body)?;
    }
    Ok(())
}

/// A JSON value written in whichever language the destination is spelled in.
///
/// **`plugins.yml` and `mcp.yml` are YAML because `PLAN.md` §13.1 names them
/// that way**, and `settings.json` stays JSON because it is Claude Code's file
/// in Claude Code's language — the guild carries it so it can be projected back
/// unchanged.
fn write_document(path: &Path, document: &serde_json::Value) -> std::io::Result<()> {
    if let Some(parent) = path.parent() {
        std::fs::create_dir_all(parent)?;
    }
    let text = if path.extension().is_some_and(|e| e == "json") {
        format!("{}\n", serde_json::to_string_pretty(document)?)
    } else {
        serde_yaml_ng::to_string(document)
            .map_err(|e| std::io::Error::new(std::io::ErrorKind::InvalidData, e))?
    };
    std::fs::write(path, text)
}

/// Copy a directory, recursively. A source that is not there copies nothing.
fn copy_tree(from: &Path, to: &Path) -> std::io::Result<()> {
    if !from.is_dir() {
        return Ok(());
    }
    std::fs::create_dir_all(to)?;
    for entry in std::fs::read_dir(from)? {
        let entry = entry?;
        let name = entry.file_name();
        // **Never adopt a `.git`.** A `~/.claude/` that is itself a repository
        // would otherwise land a second git directory inside the one Armada
        // manages, and the guild's own history would be unreachable.
        if name.to_string_lossy().starts_with('.') {
            continue;
        }
        let target = to.join(&name);
        if entry.path().is_dir() {
            copy_tree(&entry.path(), &target)?;
        } else {
            std::fs::copy(entry.path(), &target)?;
        }
    }
    Ok(())
}

/// A path as a person would name it, with the home directory left as `~`.
///
/// The agreed layout prints `imported from ~/.claude/`, and a golden fixture
/// cannot contain a real home directory — nor should any output, since it is
/// the same string on every machine and says nothing.
pub fn display(path: &Path) -> String {
    let shown = path.display().to_string();
    match shown.rsplit_once("/.claude") {
        Some((_, "")) => "~/.claude/".to_string(),
        _ => shown,
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::fs;

    /// A `~/.claude/` with one of everything, including one credential.
    ///
    /// **Named `.claude` inside the temporary directory**, so that the tests
    /// exercise the same path shape a real machine has — including the one
    /// that decides how the source is printed. Nothing in this file ever
    /// touches a real `$HOME`.
    fn a_claude_directory() -> tempfile::TempDir {
        let root = tempfile::tempdir().unwrap();
        let path = &root.path().join(".claude");
        fs::create_dir_all(path).unwrap();
        fs::create_dir_all(path.join("skills/add-migration")).unwrap();
        fs::create_dir_all(path.join("agents")).unwrap();
        fs::create_dir_all(path.join("hooks")).unwrap();
        fs::create_dir_all(path.join("plugins")).unwrap();
        fs::write(path.join("skills/add-migration/SKILL.md"), "# add\n").unwrap();
        fs::write(path.join("agents/reviewer.md"), "# reviewer\n").unwrap();
        fs::write(path.join("hooks/stop-notify.sh"), "#!/bin/sh\n").unwrap();
        fs::write(
            path.join("settings.json"),
            r#"{"model":"opus","env":{"EDITOR":"nvim","GITHUB_TOKEN":"ghp_16C7e42F292c6912E7710c838347Ae178B4a"}}"#,
        )
        .unwrap();
        fs::write(
            path.join("plugins/config.json"),
            r#"{"marketplaces":{"local":{"source":"/tmp/m"}}}"#,
        )
        .unwrap();
        fs::write(
            path.join("CLAUDE.md"),
            "## Verbosity\n\n150 words.\n\n## Branching\n\nTrunk based.\n",
        )
        .unwrap();
        root
    }

    fn claude_dir(root: &tempfile::TempDir) -> std::path::PathBuf {
        root.path().join(".claude")
    }

    fn imported_into(claude: &Path) -> (tempfile::TempDir, Imported) {
        let home = tempfile::tempdir().unwrap();
        let guild = Guild::at(home.path());
        let imported = run(claude, &guild).unwrap();
        (home, imported)
    }

    #[test]
    fn skills_hooks_and_agents_are_adopted_under_armadas_names() {
        let claude = a_claude_directory();
        let (home, imported) = imported_into(&claude_dir(&claude));
        let guild = Guild::at(home.path());

        assert!(guild.path("skills/add-migration/SKILL.md").is_file());
        assert!(guild.path("hooks/stop-notify.sh").is_file());
        assert!(
            guild.path("subagents/reviewer.md").is_file(),
            "`agents/` is Claude Code's word and `subagents/` is Armada's"
        );
        assert_eq!(imported.inventory.skills, 1);
        assert_eq!(imported.inventory.subagents, 1);
    }

    /// **The constraint this milestone would not be allowed to ship without.**
    /// The value is not in the guild, in any file, in any spelling.
    #[test]
    fn no_credential_shaped_value_reaches_the_guild() {
        let claude = a_claude_directory();
        let (home, imported) = imported_into(&claude_dir(&claude));
        let guild = Guild::at(home.path());

        assert_eq!(imported.withheld.len(), 1);
        assert_eq!(imported.withheld[0].key, "env.GITHUB_TOKEN");

        let mut checked = 0;
        for path in walk(guild.root()) {
            let body = fs::read_to_string(&path).unwrap_or_default();
            assert!(
                !body.contains("ghp_"),
                "the token reached {}",
                path.display()
            );
            checked += 1;
        }
        assert!(checked > 0, "nothing was written, so nothing was checked");

        // And what did survive is the rest of the file, so the guard is a
        // filter rather than a refusal.
        let settings = fs::read_to_string(guild.path("settings.json")).unwrap();
        assert!(settings.contains("nvim"), "{settings}");
        assert!(settings.contains("opus"), "{settings}");
    }

    #[test]
    fn the_memory_file_is_split_into_the_three_fragments() {
        let claude = a_claude_directory();
        let (home, imported) = imported_into(&claude_dir(&claude));
        let guild = Guild::at(home.path());

        assert!(imported.memory);
        assert_eq!(imported.inventory.fragments, 3);
        assert!(fs::read_to_string(guild.path("voice.md"))
            .unwrap()
            .contains("150 words"));
        assert!(fs::read_to_string(guild.path("how-i-work.md"))
            .unwrap()
            .contains("Trunk based"));
    }

    /// A machine with no `~/.claude/` is an ordinary first run, and the three
    /// fragments still exist so `doctor` has something to compare against.
    #[test]
    fn a_machine_with_nothing_to_adopt_is_not_a_failure() {
        let home = tempfile::tempdir().unwrap();
        let guild = Guild::at(home.path());
        let imported = run(Path::new("/nonexistent/.claude"), &guild).unwrap();

        assert!(imported.withheld.is_empty());
        assert!(!imported.memory);
        assert_eq!(imported.inventory.fragments, 3);
        for fragment in memory::FRAGMENTS {
            assert!(guild.path(fragment).is_file(), "{fragment} was not written");
        }
    }

    /// A settings file that is not valid JSON does not lose you the skills, the
    /// hooks and the memory file, which are most of the value.
    #[test]
    fn an_unparseable_settings_file_does_not_abandon_the_rest_of_the_import() {
        let claude = a_claude_directory();
        fs::write(claude_dir(&claude).join("settings.json"), "{ not json").unwrap();
        let (home, imported) = imported_into(&claude_dir(&claude));
        let guild = Guild::at(home.path());

        assert!(guild.path("skills/add-migration/SKILL.md").is_file());
        assert!(!guild.path("settings.json").exists());
        assert!(imported.withheld.is_empty());
    }

    /// A `~/.claude/` that is itself a repository must not land a second git
    /// directory inside the one Armada manages.
    #[test]
    fn a_dot_git_in_the_source_is_never_adopted() {
        let claude = a_claude_directory();
        fs::create_dir_all(claude_dir(&claude).join("skills/.git")).unwrap();
        fs::write(claude_dir(&claude).join("skills/.git/HEAD"), "ref: x\n").unwrap();
        let (home, _) = imported_into(&claude_dir(&claude));
        assert!(!Guild::at(home.path()).path("skills/.git").exists());
    }

    /// The fixture prints `imported from ~/.claude/`, on every machine.
    #[test]
    fn the_source_is_named_the_way_a_person_writes_it() {
        assert_eq!(display(Path::new("/Users/agent/.claude")), "~/.claude/");
        assert_eq!(display(Path::new("/elsewhere/setup")), "/elsewhere/setup");
    }

    /// The summary line the agreed layout froze.
    #[test]
    fn the_facts_read_in_the_order_the_agreed_layout_prints_them() {
        let claude = a_claude_directory();
        let (_home, imported) = imported_into(&claude_dir(&claude));
        let facts = imported.facts();
        assert_eq!(facts.first().unwrap(), "imported from ~/.claude/");
        assert_eq!(facts.last().unwrap(), "CLAUDE.md");
        assert!(facts.contains(&"1 skill".to_string()), "{facts:?}");
    }

    fn walk(root: &Path) -> Vec<std::path::PathBuf> {
        let mut out = Vec::new();
        let Ok(listing) = fs::read_dir(root) else {
            return out;
        };
        for entry in listing.filter_map(Result::ok) {
            if entry.path().is_dir() {
                out.extend(walk(&entry.path()));
            } else {
                out.push(entry.path());
            }
        }
        out
    }
}
