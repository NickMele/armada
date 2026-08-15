//! What a guild holds — counted, and named.
//!
//! One value, produced by reading a directory, and read back by three callers
//! that would otherwise each count for themselves: `guild init`'s import
//! summary, `guild export`'s manifest, and `armada doctor`.
//!
//! **Counting is not a decision, so the counts are facts and nothing here
//! interprets them.** Whether `0 skills` is a problem is `doctor`'s judgement,
//! made against the guild it is looking at; this module only says what is
//! there.
//!
//! # Counting was not enough, and that is the whole of `PLAN.md` §15.3.4
//!
//! [`Inventory`] answers *how many*. Every `armada guild` verb moved the guild
//! somewhere and none of them said what was in it, so `19 skills` was the
//! closest thing to an answer a reader could get — a number he then had to
//! satisfy with `ls`. [`Inventory::items`] answers *which*, in the same pass
//! over the same directory, and [`Item::detail`] is the half `ls` cannot give:
//! what a workflow's steps are, whether a fragment is still Armada's example
//! text, what a skill says it is for.
//!
//! **The counts did not move.** `facts()` and its six nouns are what three
//! callers already print, and a listing is a different question asked of the
//! same tree rather than a replacement for the summary line.

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
            skills: entries(&root.join("skills"), Entry::Directory),
            hooks: entries(&root.join("hooks"), Entry::File),
            subagents: entries(&root.join("subagents"), Entry::File),
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

/// What kind of thing one guild item is.
///
/// **The kind is the word in the STATUS column**, which is why it is an enum
/// with one spelling rather than the directory name it happens to live under.
/// A reader scanning a listing is scanning for one word — the same reason
/// `guild pull` groups its change set by area — and `skills` in the STATUS
/// column beside `skills` in the ITEM column would say nothing twice.
///
/// **Ordered as the listing is ordered.** The three fragments come first
/// because they are the half of a guild that is *you*; the schema comes last
/// because it is the one entry nobody wrote and nobody edits.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum Kind {
    /// `voice.md`, `expectations.md`, `how-i-work.md` — how you want to be
    /// worked with.
    Memory,
    /// A directory under `skills/`, holding a `SKILL.md`.
    Skill,
    /// A file under `subagents/`.
    Subagent,
    /// A `*.yml` under `workflows/`.
    Workflow,
    /// A file under `hooks/`.
    Hook,
    /// `settings.json`.
    Settings,
    /// `plugins.yml`.
    Plugins,
    /// `mcp.yml`.
    Mcp,
    /// `workflows/workflow.schema.json`. **Not a workflow** — the distinction
    /// [`Inventory::of`] already draws, kept here so that a listing which shows
    /// the file does not also miscount it.
    Schema,
}

impl Kind {
    /// The word this kind is printed as, lowercase. The renderer decides the
    /// case; this decides the noun.
    pub const fn word(self) -> &'static str {
        match self {
            Kind::Memory => "memory",
            Kind::Skill => "skill",
            Kind::Subagent => "subagent",
            Kind::Workflow => "workflow",
            Kind::Hook => "hook",
            Kind::Settings => "settings",
            Kind::Plugins => "plugins",
            Kind::Mcp => "mcp",
            Kind::Schema => "schema",
        }
    }

    /// Whether a person is expected to edit this by hand.
    ///
    /// The schema is not: it is the thing workflows are checked *against*, and
    /// editing it to make an invalid workflow pass is the one edit that would
    /// defeat the validation `guild edit` exists for.
    pub const fn editable(self) -> bool {
        !matches!(self, Kind::Schema)
    }
}

/// One thing in a guild, named.
///
/// **Three paths rather than one, because three questions are being asked.**
/// [`Item::name`] is what a person calls it, [`Item::opens`] is what viewing
/// or editing reads, and [`Item::path`] is what deleting removes — and for a
/// skill those last two differ: you open `skills/onboard-repo/SKILL.md` and you
/// delete `skills/onboard-repo`. Collapsing them would give a browser that
/// deletes a skill's prose and leaves its directory, or one that deletes a
/// directory the reader thought was a file.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Item {
    /// Which kind it is.
    pub kind: Kind,
    /// What a person calls it — `voice.md`, `onboard-repo`, `feature.yml`.
    pub name: String,
    /// Guild-relative, and **what a delete removes**.
    pub path: String,
    /// Guild-relative, and **what a view or an edit opens**.
    pub opens: String,
    /// One line saying what it is, read out of the file itself.
    pub detail: String,
    /// How big [`Item::opens`] is.
    pub bytes: u64,
}

impl Inventory {
    /// Everything in a guild, named, in reading order.
    ///
    /// **The same directory read, asked a different question.** [`Inventory::of`]
    /// counts; this names, summarises, and says where each thing is. A guild
    /// that is not there lists nothing rather than failing, for the same reason
    /// a missing directory counts zero.
    pub fn items(root: &Path) -> Vec<Item> {
        let mut items = Vec::new();
        for fragment in crate::memory::FRAGMENTS {
            if root.join(fragment).is_file() {
                items.push(item(root, Kind::Memory, fragment, fragment, fragment));
            }
        }
        for name in listing(&root.join("skills"), Entry::Directory) {
            let path = format!("skills/{name}");
            let opens = format!("{path}/SKILL.md");
            items.push(item(root, Kind::Skill, &name, &path, &opens));
        }
        for name in listing(&root.join("subagents"), Entry::File) {
            let path = format!("subagents/{name}");
            items.push(item(root, Kind::Subagent, &name, &path, &path));
        }
        for name in listing(&root.join("workflows"), Entry::File) {
            let path = format!("workflows/{name}");
            let kind = if is_yaml(&name) {
                Kind::Workflow
            } else if name == SCHEMA {
                Kind::Schema
            } else {
                continue;
            };
            items.push(item(root, kind, &name, &path, &path));
        }
        for name in listing(&root.join("hooks"), Entry::File) {
            let path = format!("hooks/{name}");
            items.push(item(root, Kind::Hook, &name, &path, &path));
        }
        for (file, kind) in [
            ("settings.json", Kind::Settings),
            ("plugins.yml", Kind::Plugins),
            ("mcp.yml", Kind::Mcp),
        ] {
            if root.join(file).is_file() {
                items.push(item(root, kind, file, file, file));
            }
        }
        items
    }

    /// The item a guild-relative path or a bare name refers to, if exactly one
    /// does.
    ///
    /// **A bare name is accepted and an ambiguous one is refused.** `armada
    /// guild edit voice.md` is what a person types after reading the ITEM
    /// column; `skills/onboard-repo` is what `--json` gave an agent. Both
    /// resolve, and a name that two kinds answer to comes back as `Err` with
    /// both paths, because guessing which one was meant is the one mistake a
    /// verb that deletes files may not make.
    pub fn find(items: &[Item], asked: &str) -> Result<Item, Vec<String>> {
        let asked = asked.trim().trim_end_matches('/');
        let mut matched: Vec<&Item> = items
            .iter()
            .filter(|item| item.path == asked || item.opens == asked)
            .collect();
        if matched.is_empty() {
            matched = items.iter().filter(|item| item.name == asked).collect();
        }
        match matched.as_slice() {
            [one] => Ok((*one).clone()),
            _ => Err(matched.iter().map(|item| item.path.clone()).collect()),
        }
    }
}

/// The schema that sits among the workflows and is not one.
pub const SCHEMA: &str = "workflow.schema.json";

/// Everything else in the guild that names this item.
///
/// **In-guild only, and that limit is stated rather than hidden.** A workflow
/// step naming a skill, a workflow naming a sub-workflow, a fragment naming a
/// subagent — those are here, because they are text in files this function can
/// read. A *project's* `armada.yml` naming a workflow is not: those live in
/// repositories Guild has no register of, and Guild may not ask Manifest for one
/// (`ARCHITECTURE.md` §1.9). `guild delete` says so on the row rather than
/// implying it checked.
///
/// The match is on the item's **stem** as a whole word — `plan` for
/// `plan.yml`, `onboard-repo` for `skills/onboard-repo` — because that is how a
/// workflow names one (`workflow: plan`, `skill: onboard-repo`). It over-reports
/// rather than under-reports: `plan` is an ordinary English word, and a
/// mentioned-here row a reader dismisses costs a glance, where a missed one
/// costs a workflow that no longer runs.
pub fn references(root: &Path, item: &Item) -> Vec<String> {
    let stem = stem_of(&item.name);
    if stem.is_empty() {
        return Vec::new();
    }
    let mut found = Vec::new();
    for other in Inventory::items(root) {
        if other.path == item.path {
            continue;
        }
        let Ok(body) = std::fs::read_to_string(root.join(&other.opens)) else {
            continue;
        };
        if mentions(&body, &stem) {
            found.push(other.path);
        }
    }
    found.sort();
    found
}

/// `plan` for `plan.yml`, `onboard-repo` for `onboard-repo`.
fn stem_of(name: &str) -> String {
    match name.split_once('.') {
        Some((stem, _)) => stem.to_string(),
        None => name.to_string(),
    }
}

/// Whether a body names a stem as a **whole word**.
///
/// Written against `char::is_alphanumeric` rather than a regex because a
/// substring match reports `plan` inside `planning`, which is the noise that
/// makes an over-reporting check the one a reader learns to ignore.
fn mentions(body: &str, stem: &str) -> bool {
    let boundary =
        |c: Option<char>| !c.is_some_and(|c| c.is_alphanumeric() || c == '-' || c == '_');
    let mut rest = body;
    let mut consumed = 0usize;
    while let Some(at) = rest.find(stem) {
        let start = consumed + at;
        let end = start + stem.len();
        if boundary(body[..start].chars().next_back()) && boundary(body[end..].chars().next()) {
            return true;
        }
        let step = at + stem.len();
        consumed += step;
        rest = &rest[step..];
    }
    false
}

/// Read one item off disk: how big it is, and the one line it gets to say what
/// it is.
fn item(root: &Path, kind: Kind, name: &str, path: &str, opens: &str) -> Item {
    let full = root.join(opens);
    let body = std::fs::read_to_string(&full).unwrap_or_default();
    Item {
        kind,
        name: name.to_string(),
        path: path.to_string(),
        opens: opens.to_string(),
        detail: detail(kind, name, &body),
        bytes: std::fs::metadata(&full).map(|m| m.len()).unwrap_or(0),
    }
}

/// What one item says about itself, in one line.
///
/// **Read out of the file, never guessed from its name.** A listing that
/// flattened a workflow and a fragment to a filename would be `ls` with extra
/// steps — which is the objection `PLAN.md` §15.3.4 raises against the obvious
/// version of this verb, and the reason each kind is summarised in its own
/// terms.
fn detail(kind: Kind, name: &str, body: &str) -> String {
    match kind {
        // **Whether it is still Armada's words is the fact.** A guild whose
        // `voice.md` is the example text is a guild that will be spoken to in
        // nobody's voice, and it is exactly what a reader opening this listing
        // is trying to find out.
        Kind::Memory => match crate::memory::state(body) {
            Some(unedited) => unedited.said().to_string(),
            None => collapse(first_prose(body)),
        },
        Kind::Skill | Kind::Subagent => collapse(&described(body)),
        Kind::Workflow => match armada_core::fleet::workflow::parse(body, name) {
            Ok(workflow) => format!(
                "{}, {}",
                plural(workflow.steps.len(), "step"),
                workflow
                    .steps
                    .iter()
                    .map(|step| step.id.clone())
                    .collect::<Vec<_>>()
                    .join(", ")
            ),
            // **A workflow that does not parse still appears in the listing.**
            // It is in the guild, it is the reason something is failing, and a
            // listing that hid it would hide the one row worth reading.
            Err(error) => format!("does not parse: {}", collapse(&error.message)),
        },
        Kind::Hook => match body.lines().next() {
            Some(line) if line.starts_with("#!") => {
                format!(
                    "{}, {}",
                    interpreter(line),
                    plural(body.lines().count(), "line")
                )
            }
            _ => plural(body.lines().count(), "line"),
        },
        Kind::Settings => match serde_json::from_str::<serde_json::Value>(body) {
            Ok(serde_json::Value::Object(map)) => plural(map.len(), "setting"),
            Ok(_) => "not an object".to_string(),
            Err(error) => format!("does not parse: {}", collapse(&error.to_string())),
        },
        Kind::Plugins | Kind::Mcp => plural(counted(body), "entry"),
        Kind::Schema => "what every workflow is checked against".to_string(),
    }
}

/// `sh` from `#!/bin/sh`, `python3` from `#!/usr/bin/env python3`.
fn interpreter(shebang: &str) -> String {
    let words: Vec<&str> = shebang
        .trim_start_matches("#!")
        .split_whitespace()
        .collect();
    let last = words.last().copied().unwrap_or_default();
    last.rsplit('/').next().unwrap_or(last).to_string()
}

/// A markdown file's `description:` front-matter, or its first heading, or its
/// first line of prose.
///
/// **Front matter first, because that is where a skill says what it is for.**
/// Claude Code's own skills carry it, `templates/guild/skills/` carries it, and
/// falling through to the heading is what makes a hand-written file that has
/// none still say something.
fn described(body: &str) -> String {
    let mut in_front_matter = false;
    for (index, line) in body.lines().enumerate() {
        let trimmed = line.trim();
        if trimmed == "---" {
            if index == 0 {
                in_front_matter = true;
                continue;
            }
            if in_front_matter {
                break;
            }
        }
        if in_front_matter {
            if let Some(rest) = trimmed.strip_prefix("description:") {
                return rest.trim().trim_matches('"').trim_matches('\'').to_string();
            }
        }
    }
    for line in body.lines() {
        if let Some(heading) = line.trim().strip_prefix("# ") {
            return heading.trim().to_string();
        }
    }
    first_prose(body).to_string()
}

/// The first line that is neither blank, nor a heading, nor a comment, nor one
/// of the two lines Armada writes into every fragment.
fn first_prose(body: &str) -> &str {
    body.lines()
        .map(str::trim)
        .find(|line| {
            !line.is_empty()
                && !line.starts_with('#')
                && !line.starts_with("<!--")
                && !line.starts_with("---")
        })
        .unwrap_or("")
}

/// Whitespace flattened to single spaces, so a paragraph cannot break a table
/// row into several.
///
/// **The cut is the table's and not this function's.** `render/table.rs` already
/// knows the terminal width and this does not; truncating here would truncate
/// `--json` too, where there is no width at all.
fn collapse(text: &str) -> String {
    text.split_whitespace().collect::<Vec<_>>().join(" ")
}

/// `1 skill`, `19 skills`. English pluralisation of the six nouns above, which
/// all take a plain `s`.
pub(crate) fn plural(count: usize, singular: &str) -> String {
    if count == 1 {
        format!("{count} {singular}")
    } else {
        format!("{count} {singular}s")
    }
}

enum Entry {
    File,
    Directory,
}

fn entries(directory: &Path, kind: Entry) -> usize {
    listing(directory, kind).len()
}

/// The names under a directory, sorted, dotfiles left out.
///
/// **A dotfile is the editor's or the shell's, never content.** `.DS_Store` is
/// not a hook and `.swp` is not a skill, and a listing that offered either as
/// something to delete would be offering to delete somebody's editor state.
fn listing(directory: &Path, kind: Entry) -> Vec<String> {
    let Ok(read) = std::fs::read_dir(directory) else {
        return Vec::new();
    };
    let mut names: Vec<String> = read
        .filter_map(Result::ok)
        .filter(|entry| {
            if entry.file_name().to_string_lossy().starts_with('.') {
                return false;
            }
            match kind {
                Entry::File => entry.path().is_file(),
                Entry::Directory => entry.path().is_dir(),
            }
        })
        .map(|entry| entry.file_name().to_string_lossy().to_string())
        .collect();
    names.sort();
    names
}

/// `*.yml` under a directory, **excluding the schema**. `workflow.schema.json`
/// is not a workflow, and counting it would report five starters where four
/// were copied.
fn yaml_files(directory: &Path) -> usize {
    listing(directory, Entry::File)
        .iter()
        .filter(|name| is_yaml(name))
        .count()
}

/// Whether a filename is a YAML one. **The one test of "is this a workflow"**,
/// shared by the count and by the listing so that the two can never disagree
/// about the schema.
fn is_yaml(name: &str) -> bool {
    name.ends_with(".yml") || name.ends_with(".yaml")
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
    counted(&text)
}

/// The same count, over a body already read — which is what the listing has.
fn counted(text: &str) -> usize {
    let Ok(parsed) = serde_yaml_ng::from_str::<serde_yaml_ng::Value>(text) else {
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

    /// **The listing names what the count only counted.** This is the whole of
    /// `PLAN.md` §15.3.4: `2 skills` is a number a reader then has to satisfy
    /// with `ls`, and the fix is the same pass over the same tree answering
    /// *which*.
    #[test]
    fn a_guild_is_listed_by_what_is_in_it_and_not_only_counted() {
        let root = a_guild();
        let items = Inventory::items(root.path());
        let named: Vec<(&str, &str)> = items
            .iter()
            .map(|item| (item.kind.word(), item.name.as_str()))
            .collect();
        assert_eq!(
            named,
            vec![
                ("memory", "voice.md"),
                ("skill", "add-migration"),
                ("skill", "triage-flake"),
                ("subagent", "helm.md"),
                ("workflow", "bug.yml"),
                ("workflow", "design.yml"),
                ("workflow", "feature.yml"),
                ("workflow", "plan.yml"),
                ("schema", "workflow.schema.json"),
                ("hook", "stop-notify.sh"),
                ("plugins", "plugins.yml"),
            ]
        );
    }

    /// **The schema is shown and still not counted as a workflow.** The listing
    /// exists to say what is in the guild, and the file is in the guild — but
    /// the count it is excluded from is the count three other verbs print.
    #[test]
    fn the_schema_is_listed_under_its_own_kind_rather_than_as_a_workflow() {
        let root = a_guild();
        let items = Inventory::items(root.path());
        let schema = items
            .iter()
            .find(|item| item.name == SCHEMA)
            .expect("the schema is in the guild and so it is in the listing");
        assert_eq!(schema.kind, Kind::Schema);
        assert!(!schema.kind.editable(), "the schema is not hand-edited");
        assert_eq!(
            items.iter().filter(|i| i.kind == Kind::Workflow).count(),
            Inventory::of(root.path()).workflows,
            "the listing and the count disagree about what a workflow is"
        );
    }

    /// **A skill is opened at its `SKILL.md` and deleted at its directory.**
    /// Collapsing the two would give a browser that deletes a skill's prose and
    /// leaves the directory behind.
    #[test]
    fn a_skill_opens_at_its_prose_and_deletes_at_its_directory() {
        let root = a_guild();
        let items = Inventory::items(root.path());
        let skill = items
            .iter()
            .find(|item| item.name == "add-migration")
            .unwrap();
        assert_eq!(skill.path, "skills/add-migration");
        assert_eq!(skill.opens, "skills/add-migration/SKILL.md");
    }

    /// **A fragment still holding Armada's example text says so**, which is the
    /// fact a reader opening this listing is trying to establish: a guild whose
    /// `voice.md` is the example will be spoken to in nobody's voice.
    #[test]
    fn a_fragment_says_whether_it_is_yours_yet() {
        let root = a_guild();
        let path = root.path().join("voice.md");
        fs::write(
            &path,
            format!("<!-- {} example -->\n", crate::memory::MARKER),
        )
        .unwrap();
        let unedited = Inventory::items(root.path()).remove(0);
        assert_eq!(unedited.detail, "still Armada's example text");

        fs::write(&path, "# Voice\n\n150 words maximum.\n").unwrap();
        let yours = Inventory::items(root.path()).remove(0);
        assert_eq!(yours.detail, "150 words maximum.");
    }

    /// **A workflow is summarised by its steps, because that is what it is.** A
    /// listing that showed `feature.yml` and stopped would be `ls`.
    #[test]
    fn a_workflow_is_summarised_by_its_steps_and_a_broken_one_says_so() {
        let root = a_guild();
        fs::write(
            root.path().join("workflows/feature.yml"),
            crate::starters::all()
                .into_iter()
                .find(|starter| starter.path.ends_with("feature.yml"))
                .expect("feature is a starter")
                .body,
        )
        .unwrap();
        let items = Inventory::items(root.path());
        let feature = items
            .iter()
            .find(|item| item.name == "feature.yml")
            .unwrap();
        assert!(feature.detail.contains("steps"), "{}", feature.detail);
        assert!(feature.detail.contains("implement"), "{}", feature.detail);

        let broken = items.iter().find(|item| item.name == "bug.yml").unwrap();
        assert!(
            broken.detail.starts_with("does not parse"),
            "a workflow that does not parse is the row worth reading: {}",
            broken.detail
        );
    }

    /// A skill says what it is for, out of its own front matter.
    #[test]
    fn a_skill_is_summarised_by_the_description_it_carries() {
        let root = a_guild();
        fs::write(
            root.path().join("skills/add-migration/SKILL.md"),
            "---\nname: add-migration\ndescription: Write a migration and its rollback.\n---\n\n# Add a migration\n",
        )
        .unwrap();
        let items = Inventory::items(root.path());
        let skill = items
            .iter()
            .find(|item| item.name == "add-migration")
            .unwrap();
        assert_eq!(skill.detail, "Write a migration and its rollback.");
    }

    /// **A bare name resolves and an ambiguous one is refused.** Guessing which
    /// of two things a reader meant is the one mistake a verb that deletes files
    /// may not make.
    #[test]
    fn a_name_resolves_to_one_item_or_to_none_at_all() {
        let root = a_guild();
        // One name, two kinds — a skill and a hook both called `notify`.
        fs::create_dir_all(root.path().join("skills/notify")).unwrap();
        fs::write(root.path().join("hooks/notify"), "#!/bin/sh\n").unwrap();
        let items = Inventory::items(root.path());

        assert_eq!(
            Inventory::find(&items, "skills/add-migration")
                .unwrap()
                .name,
            "add-migration"
        );
        // The openable path resolves too — it is what `--json` handed out.
        assert_eq!(
            Inventory::find(&items, "workflows/plan.yml").unwrap().name,
            "plan.yml"
        );
        assert_eq!(
            Inventory::find(&items, "add-migration").unwrap().path,
            "skills/add-migration"
        );

        // An exact guild-relative path is never ambiguous, even when the name
        // at the end of it is shared.
        assert_eq!(
            Inventory::find(&items, "hooks/notify").unwrap().kind,
            Kind::Hook
        );
        let ambiguous = Inventory::find(&items, "notify").unwrap_err();
        assert_eq!(ambiguous, vec!["skills/notify", "hooks/notify"]);
        assert_eq!(
            Inventory::find(&items, "nothing").unwrap_err(),
            Vec::<String>::new()
        );
    }

    /// **What else names this is reported before it is deleted.** A workflow a
    /// skill is named by, deleted without a word, is a workflow that fails on
    /// its next run for a reason nothing in the guild records.
    #[test]
    fn what_else_names_an_item_is_found_and_a_longer_word_is_not() {
        let root = a_guild();
        fs::write(
            root.path().join("workflows/feature.yml"),
            "name: feature\nsteps:\n  - id: build\n    skill: add-migration\n",
        )
        .unwrap();
        // `plan.yml` mentions `add-migrations`, which is a different name.
        fs::write(
            root.path().join("workflows/plan.yml"),
            "name: plan\n# see add-migrations\n",
        )
        .unwrap();
        let items = Inventory::items(root.path());
        let skill = items
            .iter()
            .find(|item| item.name == "add-migration")
            .unwrap();
        assert_eq!(
            references(root.path(), skill),
            vec!["workflows/feature.yml".to_string()],
            "a substring match would have reported plan.yml too"
        );
    }

    /// Nothing names a thing nothing names, which is the ordinary case and the
    /// one that has to be quiet.
    #[test]
    fn an_item_nothing_names_reports_nothing() {
        let root = a_guild();
        let items = Inventory::items(root.path());
        let hook = items
            .iter()
            .find(|item| item.name == "stop-notify.sh")
            .unwrap();
        assert!(references(root.path(), hook).is_empty());
    }

    /// A guild that is not there lists nothing rather than failing — the same
    /// rule the counts follow, and for the same first run.
    #[test]
    fn a_guild_that_is_not_there_lists_nothing() {
        assert!(Inventory::items(Path::new("/nonexistent/nowhere")).is_empty());
    }
}
