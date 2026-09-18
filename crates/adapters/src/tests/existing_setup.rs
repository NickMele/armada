//! A person's setup held in memory: what is read is the subject, and nothing
//! is written to read it.
//!
//! The one case that touches a real directory is [`a_home_that_is_not_there`],
//! which needs a path that exists nowhere.

use std::collections::BTreeMap;

use adapter_traits::{
    FileEntry, FileRead, HarnessSetup, SetupFiles, SetupItem, SetupKind, WhatWasRead,
};

use crate::{ExistingSetup, Home};

struct Held(BTreeMap<String, String>);

impl Held {
    fn of(files: &[(&str, &str)]) -> Held {
        Held(
            files
                .iter()
                .map(|(path, text)| (path.to_string(), text.to_string()))
                .collect(),
        )
    }
}

impl SetupFiles for Held {
    fn read(&self, path: &str) -> FileRead {
        match self.0.get(path) {
            Some(text) => FileRead::Bytes(text.as_bytes().to_vec()),
            None => FileRead::Absent,
        }
    }

    fn entries(&self, dir: &str) -> Result<Vec<FileEntry>, String> {
        let prefix = format!("{dir}/");
        let mut found: BTreeMap<String, bool> = BTreeMap::new();
        for path in self.0.keys() {
            if let Some(rest) = path.strip_prefix(&prefix) {
                match rest.split_once('/') {
                    Some((child, _)) => found.insert(child.to_string(), true),
                    None => found.insert(rest.to_string(), false),
                };
            }
        }
        match found.is_empty() {
            true => Err(format!("{dir} is not a directory")),
            false => Ok(found
                .into_iter()
                .map(|(name, is_dir)| FileEntry { name, is_dir })
                .collect()),
        }
    }
}

fn read(files: &[(&str, &str)]) -> adapter_traits::Inventory {
    ExistingSetup::over(Held::of(files), "/home/someone").read()
}

fn items(files: &[(&str, &str)], kind: SetupKind) -> Vec<SetupItem> {
    match read(files).kind(kind).cloned() {
        Some(WhatWasRead::Read { items, .. }) => items,
        other => panic!("{kind:?} was {other:?}"),
    }
}

fn unreadable(files: &[(&str, &str)], kind: SetupKind) -> Vec<String> {
    match read(files).kind(kind).cloned() {
        Some(WhatWasRead::Read { unreadable, .. }) => {
            unreadable.into_iter().map(|one| one.source).collect()
        }
        other => panic!("{kind:?} was {other:?}"),
    }
}

const A_SKILL: &str = "---\nname: humanizer\ndescription: Rewrite text\n---\n\n# Humanizer\n";

#[test]
fn a_skill_is_named_and_described_by_its_own_front_matter() {
    let read = items(
        &[(".claude/skills/humanizer/SKILL.md", A_SKILL)],
        SetupKind::Skills,
    );
    assert_eq!(read.len(), 1);
    assert_eq!(read[0].name, "humanizer");
    assert_eq!(read[0].says.as_deref(), Some("Rewrite text"));
    assert_eq!(read[0].source, "/home/someone/.claude/skills/humanizer");
}

#[test]
fn a_folder_of_skills_is_read_through_rather_than_reported_as_one() {
    let read = items(
        &[
            (".claude/skills/humanizer/SKILL.md", A_SKILL),
            (
                ".claude/skills/synced/from-the-web/SKILL.md",
                "---\nname: docs\ndescription: Read the docs\n---\n",
            ),
        ],
        SetupKind::Skills,
    );
    let named: Vec<&str> = read.iter().map(|item| item.name.as_str()).collect();
    assert_eq!(named, vec!["humanizer", "docs"]);
}

#[test]
fn a_skill_whose_front_matter_will_not_parse_is_named_rather_than_skipped() {
    let files = [
        (".claude/skills/broken/SKILL.md", "---\nname: [oh\n---\n"),
        (".claude/skills/humanizer/SKILL.md", A_SKILL),
    ];
    assert_eq!(items(&files, SetupKind::Skills).len(), 1);
    assert_eq!(
        unreadable(&files, SetupKind::Skills),
        vec!["/home/someone/.claude/skills/broken/SKILL.md"]
    );
}

#[test]
fn front_matter_that_never_closes_is_a_reason_and_not_a_panic() {
    let files = [(".claude/skills/half/SKILL.md", "---\nname: half\n")];
    assert_eq!(items(&files, SetupKind::Skills).len(), 0);
    assert_eq!(unreadable(&files, SetupKind::Skills).len(), 1);
}

#[test]
fn a_skill_without_front_matter_is_named_by_its_folder() {
    let read = items(
        &[(".claude/skills/plain/SKILL.md", "# Plain\n")],
        SetupKind::Skills,
    );
    assert_eq!(read[0].name, "plain");
    assert_eq!(read[0].says, None);
}

#[test]
fn a_namespaced_command_carries_the_folder_in_its_name() {
    let read = items(
        &[(".claude/commands/git/sync.md", "do the thing\n")],
        SetupKind::Commands,
    );
    assert_eq!(read[0].name, "git:sync");
}

#[test]
fn plugins_come_off_the_file_the_harness_keeps_them_in() {
    let read = items(
        &[(
            ".claude/plugins/installed_plugins.json",
            r#"{"version":2,"plugins":{"github@official":[{"scope":"user","version":"1.0.0","installPath":"/home/someone/.claude/plugins/cache/github"}]}}"#,
        )],
        SetupKind::Plugins,
    );
    assert_eq!(read[0].name, "github@official");
    assert_eq!(read[0].says.as_deref(), Some("version 1.0.0"));
    assert_eq!(read[0].source, "/home/someone/.claude/plugins/cache/github");
}

#[test]
fn a_plugin_file_that_will_not_parse_is_named_and_takes_nothing_else_down() {
    let files = [
        (".claude/plugins/installed_plugins.json", "{ not json"),
        (".claude/skills/humanizer/SKILL.md", A_SKILL),
    ];
    assert_eq!(unreadable(&files, SetupKind::Plugins).len(), 1);
    assert_eq!(items(&files, SetupKind::Skills).len(), 1);
}

/// **The whole safety of this seam, as a test.** A server a person connected
/// is carried by name and by sort, and the address is not in the answer at all
/// — so nothing downstream can build a `KitServer` out of what was read.
#[test]
fn a_connected_server_crosses_as_a_name_and_never_as_an_address() {
    let read = items(
        &[(
            ".claude.json",
            r#"{"oauthAccount":{"emailAddress":"someone@example.com"},
                "mcpServers":{"tracker":{"command":"npx","args":["-y","@scope/tracker","--key=SECRET"]},
                              "docs":{"type":"http","url":"https://docs.example.com/mcp?token=SECRET"}}}"#,
        )],
        SetupKind::McpServers,
    );
    let said = format!("{read:?}");
    assert!(
        !said.contains("SECRET"),
        "an address survived the read: {said}"
    );
    assert!(!said.contains("npx"), "a command survived the read: {said}");
    assert!(
        !said.contains("example.com"),
        "a host survived the read: {said}"
    );
    let named: Vec<&str> = read.iter().map(|item| item.name.as_str()).collect();
    assert_eq!(named, vec!["docs", "tracker"]);
    assert_eq!(
        read[1].says.as_deref(),
        Some("a program this machine starts")
    );
}

#[test]
fn the_global_agent_file_is_measured_and_never_carried() {
    let read = items(
        &[(".claude/CLAUDE.md", "\n# How I work\n\nBe direct.\n")],
        SetupKind::AgentFile,
    );
    assert_eq!(read.len(), 1);
    assert_eq!(
        read[0].says.as_deref(),
        Some("4 lines, opening # How I work")
    );
    assert!(!format!("{read:?}").contains("Be direct"));
}

/// Two kinds are answered as not read rather than drawn as empty, which is the
/// difference between *you have none* and *nothing looked*.
#[test]
fn the_kinds_nothing_reads_yet_say_so() {
    let read = read(&[]);
    assert_eq!(read.kinds.len(), SetupKind::ALL.len());
    for kind in [SetupKind::Allowlist, SetupKind::Models] {
        match read.kind(kind) {
            Some(WhatWasRead::NotRead { why }) => assert!(why.contains("#41")),
            other => panic!("{kind:?} was {other:?}"),
        }
    }
}

#[test]
fn a_home_with_nothing_in_it_is_read_as_nothing_rather_than_as_a_failure() {
    let read = read(&[]);
    assert!(!read.present);
    assert_eq!(
        read.kind(SetupKind::Skills),
        Some(&WhatWasRead::Read {
            items: Vec::new(),
            unreadable: Vec::new()
        })
    );
}

#[test]
fn a_home_that_is_not_there_is_an_answer() {
    let read = ExistingSetup::over(Home::at("/nowhere/at/all"), "/nowhere/at/all").read();
    assert!(!read.present);
    assert_eq!(read.home, "/nowhere/at/all/.claude");
    assert_eq!(read.kinds.len(), SetupKind::ALL.len());
}

/// A path built above the home reads nothing, whatever a caller asks for.
#[test]
fn nothing_above_the_home_is_readable_through_it() {
    let home = Home::at("/nowhere/at/all");
    assert_eq!(
        home.read("../etc/passwd"),
        FileRead::Unreadable("above the home it is read from".to_string())
    );
    assert!(home.entries("../..").is_err());
}
