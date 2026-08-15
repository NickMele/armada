//! **Putting the guild where Claude Code will read it**, reversibly.
//!
//! A guild is not on any tool's load path until something puts it there
//! ([`PHASES.md`](../../../docs/PHASES.md) §8.4). Guild skills live in
//! `~/.armada/guild/skills/` and Claude Code reads `~/.claude/skills/`; until
//! this module ran, nothing copied between them, and a skill `guild init`
//! installed could not be invoked by name in any session.
//!
//! This module holds the decisions. [`crate::projector`] does the writing.
//!
//! # What is projected, and by which mechanism
//!
//! **Direct file placement, into the directories Claude Code already reads.**
//! [`PLAN.md`](../../../docs/PLAN.md) §13.3 expected a Claude Code plugin to
//! carry this half; it was re-decided against three measurements, all recorded
//! in §13.3 and summarised here because this is the file the decision is
//! embodied in:
//!
//! | Measured against `claude` 2.1.233 | Consequence |
//! |---|---|
//! | `claude plugin init` scaffolds into `~/.claude/skills/<name>/`, which auto-loads with no marketplace and no install step | The installer and versioning §13.3 credits a plugin with are **not obtained** in the one plugin form that needs no install step |
//! | A plugin namespaces what it carries — a skill in plugin `p` is `p:skill` | A plugin **renames** `/onboard-repo`, which is the one name §8.4 says has to resolve |
//! | A plugin's hooks are registered by a `hooks/hooks.json` | Armada would have to synthesise one, and it would double-fire every hook the reader's own `settings.json` already registers against `~/.claude/hooks/` |
//!
//! What a plugin is still the right mechanism for is everything direct
//! placement has no load path for — MCP servers, LSP servers, a `bin/` on
//! `PATH` — and it lands when those are projected. §13.3 anticipated both
//! halves; this is which half is which.
//!
//! # The manifest is the whole design
//!
//! §13.2 specifies **a manifest of what was placed and a hash of each file, so
//! re-sync updates only what you have not touched and `--remove` reverses
//! exactly.** Every rule below falls out of holding three hashes against each
//! other, and there is only ever one question:
//!
//! | | is what the guild says | is what is on disk | is what Armada last wrote |
//! |---|---|---|---|
//! | **`desired`** | ✓ | | |
//! | **`present`** | | ✓ | |
//! | **`recorded`** | | | ✓ |
//!
//! **A file whose `present` is not its `recorded` is yours**, whatever the
//! guild says, and it is left exactly as it is and reported. That is the one
//! rule this module exists to hold: getting it wrong destroys work somebody did
//! by hand, silently, on a machine they were not looking at.
//!
//! A file with no `recorded` at all is also yours — Armada never placed it, so
//! a guild skill that happens to share its name does not get to overwrite it.

use armada_core::envelope::Sync;
use serde::{Deserialize, Serialize};
use std::collections::{BTreeMap, BTreeSet};

/// Where the manifest lives, under `~/.armada/`.
///
/// **Beside the guild rather than inside it.** It records what was placed on
/// *this* machine, so it is one of the things `PLAN.md` §13.1 says never syncs
/// — and [`crate::layout::NEVER_SYNCS`] lists it, which is what keeps it
/// outside the git repository by construction.
pub const MANIFEST: &str = "projection.json";

/// What a previous projection placed, and the hash of each file as it placed
/// it.
///
/// Keyed on the path **relative to `~/.claude/`**, so the manifest is the same
/// document on every machine and can be read without knowing where anybody's
/// home directory is.
#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct Placed {
    /// Path under `~/.claude/`, to the hash of the bytes Armada wrote there.
    #[serde(default)]
    pub files: BTreeMap<String, String>,
}

impl Placed {
    /// What Armada last wrote at a path, or `None` if it never wrote there.
    ///
    /// `None` is not "unknown" — it is **the file is not Armada's**, which is
    /// the answer that stops a projection from overwriting somebody's own
    /// `~/.claude/skills/review/SKILL.md` because a guild happens to hold a
    /// skill of that name.
    pub fn hash_of(&self, at: &str) -> Option<&str> {
        self.files.get(at).map(String::as_str)
    }

    /// Whether anything is projected at all.
    pub fn is_empty(&self) -> bool {
        self.files.is_empty()
    }
}

/// What projection does to one path.
///
/// The outcome is spelled in [`Sync`] rather than in a vocabulary of its own,
/// because it is the same five words `guild pull` already prints and a reader
/// scanning a `STATUS` column should not have to learn a second set:
///
/// | [`Sync`] | Means |
/// |---|---|
/// | `ADDED` | it was not there, and now it is |
/// | `CHANGED` | Armada's own copy was out of date, and was rewritten |
/// | `UNCHANGED` | already byte-identical; nothing was written |
/// | `CONFLICT` | **yours** — edited by hand, or never Armada's. Left alone |
/// | `REMOVED` | the guild no longer holds it, so it was taken back |
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Step {
    /// The path under `~/.claude/`.
    pub at: String,
    /// The path under `~/.armada/guild/` it comes from, when it still exists
    /// there. `None` for a withdrawal.
    pub from: Option<String>,
    /// What happens to it.
    pub outcome: Sync,
}

impl Step {
    /// Whether the shell must write this file.
    pub fn writes(&self) -> bool {
        matches!(self.outcome, Sync::Added | Sync::Changed)
    }

    /// Whether the shell must delete this file.
    pub fn deletes(&self) -> bool {
        self.outcome == Sync::Removed
    }

    /// Whether this is a file the reader edited and Armada therefore did not
    /// touch.
    pub fn is_yours(&self) -> bool {
        self.outcome == Sync::Conflict
    }
}

/// What projecting one file does, from the three hashes and nothing else.
///
/// **`recorded` is what makes this safe.** Without it the only comparison
/// available is `desired` against `present`, and every file that differs looks
/// like one to overwrite — including every file the reader wrote himself.
pub fn verdict(desired: &str, present: Option<&str>, recorded: Option<&str>) -> Sync {
    match (present, recorded) {
        // Nothing there. **Placed, even if the manifest remembers it**: a file
        // that is gone has no edit to lose, and the guild is the source of
        // truth for what should exist. Removing a skill is done by removing it
        // from the guild, not by deleting the copy Armada placed.
        (None, _) => Sync::Added,
        // **Already byte-identical, so it is adopted whether or not Armada put
        // it there.** This is the ordinary state of the machine the guild was
        // imported *from*: `guild init` reads `~/.claude/skills/`, and
        // projecting it back finds the same bytes at the same path with no
        // record of having written them.
        //
        // Treating that as the reader's file would be technically true and
        // useless — a fresh `guild init` would report every adopted skill as a
        // conflict, and the guild could never update one of them again. Nothing
        // can be lost by claiming it: the bytes are identical, and the copy in
        // the guild is in a git repository with history.
        (Some(present), _) if present == desired => Sync::Unchanged,
        // There, different, and Armada never put it there. It is the reader's
        // file and it keeps its name.
        (Some(_), None) => Sync::Conflict,
        // There, and changed since Armada wrote it. It is the reader's now.
        (Some(present), Some(recorded)) if present != recorded => Sync::Conflict,
        // There, untouched, and out of date.
        _ => Sync::Changed,
    }
}

/// What withdrawing one file does — a path the manifest holds and the guild no
/// longer offers, or every path at all under `--remove`.
///
/// `None` means there is nothing to do and nothing to say: the file is already
/// gone, and a row reporting the removal of something that was not there is a
/// row about nothing.
pub fn withdrawal(present: Option<&str>, recorded: &str) -> Option<Sync> {
    match present {
        None => None,
        // **Edited since Armada wrote it, so it is not what was placed.**
        // `--remove` reverses exactly what was placed; this is no longer that.
        Some(present) if present != recorded => Some(Sync::Conflict),
        Some(_) => Some(Sync::Removed),
    }
}

/// Everything a projection would do, in one pass over what the guild offers and
/// what the manifest remembers.
///
/// `desired` and `present` are both keyed on the path under `~/.claude/`;
/// `present` need only cover the union of the two key sets, because no other
/// path is ever considered — projection looks at what it offers and at what it
/// once wrote, and at nothing else in the reader's `~/.claude/`.
pub fn plan(
    desired: &BTreeMap<String, Offered>,
    present: &BTreeMap<String, String>,
    placed: &Placed,
) -> Vec<Step> {
    let mut steps = Vec::new();
    for (at, offered) in desired {
        steps.push(Step {
            at: at.clone(),
            from: Some(offered.from.clone()),
            outcome: verdict(
                &offered.hash,
                present.get(at).map(String::as_str),
                placed.hash_of(at),
            ),
        });
    }
    // Everything the manifest remembers that the guild no longer offers. A
    // skill deleted from the guild on another machine is a skill this machine
    // stops carrying, which is the half of `guild pull` that would otherwise
    // leave a projection that only ever grows.
    for (at, recorded) in &placed.files {
        if desired.contains_key(at) {
            continue;
        }
        if let Some(outcome) = withdrawal(present.get(at).map(String::as_str), recorded) {
            steps.push(Step {
                at: at.clone(),
                from: None,
                outcome,
            });
        }
    }
    steps.sort_by(|a, b| a.at.cmp(&b.at));
    steps
}

/// Taking back exactly what was placed, and nothing else — `--remove`.
///
/// Reads the manifest and never the guild, which is what makes "exactly" true:
/// a guild that has grown a skill since the last projection does not widen the
/// reversal, and one that has lost a skill does not narrow it.
pub fn reversal(present: &BTreeMap<String, String>, placed: &Placed) -> Vec<Step> {
    let mut steps: Vec<Step> = placed
        .files
        .iter()
        .filter_map(|(at, recorded)| {
            withdrawal(present.get(at).map(String::as_str), recorded).map(|outcome| Step {
                at: at.clone(),
                from: None,
                outcome,
            })
        })
        .collect();
    steps.sort_by(|a, b| a.at.cmp(&b.at));
    steps
}

/// A file the guild offers: where it is, and what it hashes to.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Offered {
    /// The path under `~/.armada/guild/`.
    pub from: String,
    /// The hash of its bytes.
    pub hash: String,
}

/// The manifest a projection leaves behind.
///
/// **A file left alone because the reader edited it is dropped from the
/// manifest**, which is Armada disowning it: it will be reported as yours on
/// every later projection, and `--remove` will not delete it. That is the
/// correct reading of "reverses exactly what was placed" — an edited file is no
/// longer what was placed.
pub fn manifest(steps: &[Step], desired: &BTreeMap<String, Offered>, placed: &Placed) -> Placed {
    let mut files = BTreeMap::new();
    let touched: BTreeSet<&str> = steps.iter().map(|step| step.at.as_str()).collect();
    // Anything the plan did not mention keeps its record. Nothing produces this
    // today — `plan` considers the union of both key sets — and it is here so
    // that a narrower plan can never silently drop the rest of the manifest.
    for (at, hash) in &placed.files {
        if !touched.contains(at.as_str()) {
            files.insert(at.clone(), hash.clone());
        }
    }
    for step in steps {
        match step.outcome {
            Sync::Added | Sync::Changed | Sync::Unchanged => {
                if let Some(offered) = desired.get(&step.at) {
                    files.insert(step.at.clone(), offered.hash.clone());
                }
            }
            Sync::Conflict | Sync::Removed => {}
        }
    }
    Placed { files }
}

/// Where a guild-relative path lands under `~/.claude/`, or `None` for guild
/// content Claude Code has no load path for.
///
/// The three trees are [`crate::layout::TREES`] read right-to-left. Everything
/// else — `workflows/`, `plugins.yml`, `mcp.yml`, and the three memory
/// fragments — is either Armada's own to read or the personal half `PLAN.md`
/// §13.3 says Guild writes by hand, and neither is projected by placing a file.
pub fn target(guild_relative: &str) -> Option<String> {
    let (top, rest) = guild_relative.split_once('/')?;
    if rest.is_empty() {
        return None;
    }
    crate::layout::TREES
        .iter()
        .find(|tree| tree.guild == top)
        .map(|tree| format!("{}/{rest}", tree.claude))
}

/// The hash a manifest records.
///
/// **BLAKE3 of the bytes, hex.** The same hash the workspace already pins for
/// `armada manifest explain`'s failure signature, for the same reason it is
/// pinned there: two versions of one hash is two runs of one file that stop
/// matching across an upgrade, and here that would report every projected file
/// as edited by hand.
pub fn hash(bytes: &[u8]) -> String {
    blake3::hash(bytes).to_hex().to_string()
}

/// The summary line's facts: how many files landed, and how many were left as
/// yours.
///
/// **A zero is left out**, which is the rule every other summary in Armada
/// follows — `0 kept` is a fact nobody was asking about on a line whose job is
/// to say what happened.
pub fn facts(steps: &[Step]) -> Vec<String> {
    let mut out = Vec::new();
    for (outcome, word) in [
        (Sync::Added, "placed"),
        (Sync::Changed, "updated"),
        (Sync::Removed, "withdrawn"),
        (Sync::Conflict, "left as yours"),
    ] {
        let count = steps.iter().filter(|step| step.outcome == outcome).count();
        if count > 0 {
            out.push(format!("{count} {word}"));
        }
    }
    if out.is_empty() {
        out.push("already current".to_string());
    }
    out
}

#[cfg(test)]
mod tests {
    use super::*;

    const MINE: &str = "hash-of-the-guilds-copy";
    const YOURS: &str = "hash-of-what-you-wrote";
    const OLD: &str = "hash-of-what-armada-wrote-last-time";

    fn offered(from: &str, hash: &str) -> Offered {
        Offered {
            from: from.to_string(),
            hash: hash.to_string(),
        }
    }

    fn desired(pairs: &[(&str, &str, &str)]) -> BTreeMap<String, Offered> {
        pairs
            .iter()
            .map(|(at, from, hash)| (at.to_string(), offered(from, hash)))
            .collect()
    }

    fn present(pairs: &[(&str, &str)]) -> BTreeMap<String, String> {
        pairs
            .iter()
            .map(|(at, hash)| (at.to_string(), hash.to_string()))
            .collect()
    }

    fn placed(pairs: &[(&str, &str)]) -> Placed {
        Placed {
            files: present(pairs),
        }
    }

    /// **The rule this module exists for.** A file whose bytes are not the
    /// bytes Armada wrote is the reader's, and it is left exactly as it is —
    /// whatever the guild says it should hold.
    #[test]
    fn a_file_you_edited_is_never_overwritten() {
        assert_eq!(verdict(MINE, Some(YOURS), Some(OLD)), Sync::Conflict);
    }

    /// **And a file Armada never placed is yours too.** A guild that grows a
    /// skill named `review` does not get to overwrite the `review` skill
    /// somebody already had in `~/.claude/skills/`, because there is no record
    /// saying Armada put it there.
    #[test]
    fn a_file_armada_never_placed_is_yours_even_when_the_guild_holds_its_name() {
        assert_eq!(verdict(MINE, Some(YOURS), None), Sync::Conflict);
    }

    /// **Unless it is already byte-identical**, which is the ordinary state of
    /// the machine the guild was imported from: `guild init` read
    /// `~/.claude/skills/`, and projecting it back finds the same bytes at the
    /// same path with no record of having written them.
    ///
    /// Calling that a conflict would make a fresh `guild init` report every
    /// adopted skill as one, and would mean the guild could never update one of
    /// them again. Nothing can be lost by adopting it — the bytes are the same,
    /// and the guild's copy is in a repository with history.
    #[test]
    fn a_file_that_is_already_what_the_guild_says_is_adopted_rather_than_disputed() {
        assert_eq!(verdict(MINE, Some(MINE), None), Sync::Unchanged);
        let want = desired(&[("skills/adopted/SKILL.md", "skills/adopted/SKILL.md", MINE)]);
        let steps = plan(
            &want,
            &present(&[("skills/adopted/SKILL.md", MINE)]),
            &Placed::default(),
        );
        assert_eq!(
            manifest(&steps, &want, &Placed::default()).hash_of("skills/adopted/SKILL.md"),
            Some(MINE),
            "an identical file was not adopted, so the guild could never update it"
        );
    }

    /// The ordinary first projection: nothing there, so it lands.
    #[test]
    fn a_path_with_nothing_at_it_is_placed() {
        assert_eq!(verdict(MINE, None, None), Sync::Added);
    }

    /// **A projected file the reader deleted is placed again.** Deleting the
    /// copy is not how a skill is removed — removing it from the guild is, and
    /// the guild is the source of truth. There is also no edit to lose: the
    /// file is not there.
    #[test]
    fn a_projected_file_that_was_deleted_is_placed_again() {
        assert_eq!(verdict(MINE, None, Some(OLD)), Sync::Added);
    }

    /// Armada's own copy, out of date, is rewritten — the whole point of
    /// re-projecting after a pull.
    #[test]
    fn armadas_own_copy_is_the_only_thing_it_rewrites() {
        assert_eq!(verdict(MINE, Some(OLD), Some(OLD)), Sync::Changed);
    }

    /// Byte-identical writes nothing and says so, rather than rewriting the
    /// file and reporting a change that did not happen.
    #[test]
    fn a_file_already_identical_is_not_rewritten() {
        assert_eq!(verdict(MINE, Some(MINE), Some(MINE)), Sync::Unchanged);
    }

    /// **`--remove` reverses exactly what was placed.** A file edited since is
    /// not what was placed, so it survives the reversal and is reported.
    #[test]
    fn remove_takes_back_what_it_placed_and_leaves_what_you_changed() {
        let steps = reversal(
            &present(&[
                ("skills/onboard-repo/SKILL.md", OLD),
                ("hooks/stop-notify.sh", YOURS),
            ]),
            &placed(&[
                ("skills/onboard-repo/SKILL.md", OLD),
                ("hooks/stop-notify.sh", OLD),
            ]),
        );
        assert_eq!(steps.len(), 2);
        assert_eq!(steps[0].at, "hooks/stop-notify.sh");
        assert_eq!(
            steps[0].outcome,
            Sync::Conflict,
            "an edited file was deleted"
        );
        assert_eq!(steps[1].outcome, Sync::Removed);
    }

    /// A reversal reads the manifest and never the guild, so a file the guild
    /// holds and Armada never placed is not in it.
    #[test]
    fn a_reversal_never_widens_beyond_the_manifest() {
        let steps = reversal(
            &present(&[("skills/mine/SKILL.md", YOURS)]),
            &Placed::default(),
        );
        assert!(steps.is_empty(), "{steps:?}");
    }

    /// A file the manifest remembers and the guild no longer offers is taken
    /// back — otherwise a skill deleted on another machine would live on this
    /// one for ever.
    #[test]
    fn a_file_the_guild_no_longer_holds_is_withdrawn() {
        let steps = plan(
            &desired(&[]),
            &present(&[("agents/retired.md", OLD)]),
            &placed(&[("agents/retired.md", OLD)]),
        );
        assert_eq!(steps.len(), 1);
        assert_eq!(steps[0].outcome, Sync::Removed);
        assert!(steps[0].deletes());
        assert_eq!(steps[0].from, None);
    }

    /// Withdrawing something already gone is not a row. A reader learns to skip
    /// a table whose rows report things that did not happen.
    #[test]
    fn withdrawing_what_is_already_gone_says_nothing() {
        assert_eq!(withdrawal(None, OLD), None);
    }

    /// The whole plan, over the three cases at once, in path order.
    #[test]
    fn a_plan_covers_what_the_guild_offers_and_what_the_manifest_remembers() {
        let steps = plan(
            &desired(&[
                ("agents/helm.md", "subagents/helm.md", MINE),
                (
                    "skills/onboard-repo/SKILL.md",
                    "skills/onboard-repo/SKILL.md",
                    MINE,
                ),
            ]),
            &present(&[
                ("agents/helm.md", YOURS),
                ("hooks/retired.sh", OLD),
                ("skills/onboard-repo/SKILL.md", OLD),
            ]),
            &placed(&[
                ("agents/helm.md", OLD),
                ("hooks/retired.sh", OLD),
                ("skills/onboard-repo/SKILL.md", OLD),
            ]),
        );
        let outcomes: Vec<(&str, Sync)> = steps
            .iter()
            .map(|step| (step.at.as_str(), step.outcome))
            .collect();
        assert_eq!(
            outcomes,
            vec![
                ("agents/helm.md", Sync::Conflict),
                ("hooks/retired.sh", Sync::Removed),
                ("skills/onboard-repo/SKILL.md", Sync::Changed),
            ]
        );
    }

    /// **A file left as yours is dropped from the manifest**, which is Armada
    /// disowning it: `--remove` will not delete it, and every later projection
    /// reports it as yours rather than fighting for it.
    #[test]
    fn a_file_left_as_yours_stops_being_armadas() {
        let want = desired(&[("agents/helm.md", "subagents/helm.md", MINE)]);
        let steps = plan(
            &want,
            &present(&[("agents/helm.md", YOURS)]),
            &placed(&[("agents/helm.md", OLD)]),
        );
        let after = manifest(&steps, &want, &placed(&[("agents/helm.md", OLD)]));
        assert!(after.is_empty(), "{after:?}");
        assert_eq!(after.hash_of("agents/helm.md"), None);
    }

    /// What was written is recorded as what the guild said, so the next
    /// projection compares against the right thing.
    #[test]
    fn what_was_written_is_recorded_as_what_the_guild_said() {
        let want = desired(&[("hooks/stop.sh", "hooks/stop.sh", MINE)]);
        let steps = plan(&want, &present(&[]), &Placed::default());
        let after = manifest(&steps, &want, &Placed::default());
        assert_eq!(after.hash_of("hooks/stop.sh"), Some(MINE));
    }

    /// A withdrawal leaves the manifest, so nothing claims a file that is no
    /// longer there.
    #[test]
    fn a_withdrawn_file_leaves_the_manifest() {
        let before = placed(&[("hooks/retired.sh", OLD)]);
        let steps = plan(
            &desired(&[]),
            &present(&[("hooks/retired.sh", OLD)]),
            &before,
        );
        assert!(manifest(&steps, &desired(&[]), &before).is_empty());
    }

    /// The three trees, read right-to-left — and `agents/` is the one name that
    /// changes on the way.
    #[test]
    fn a_guild_path_lands_where_claude_code_reads_it() {
        assert_eq!(
            target("skills/onboard-repo/SKILL.md").as_deref(),
            Some("skills/onboard-repo/SKILL.md")
        );
        assert_eq!(
            target("subagents/helm.md").as_deref(),
            Some("agents/helm.md"),
            "Claude Code calls them agents"
        );
        assert_eq!(target("hooks/stop.sh").as_deref(), Some("hooks/stop.sh"));
    }

    /// **Nothing else is projected by placing a file.** Workflows are Armada's
    /// own to read; the fragments and the settings keys are the personal half
    /// `PLAN.md` §13.3 says Guild writes by hand, and a `voice.md` dropped into
    /// `~/.claude/` is read by nothing.
    #[test]
    fn what_claude_code_has_no_load_path_for_is_not_placed() {
        for path in [
            "workflows/bug.yml",
            "voice.md",
            "plugins.yml",
            "mcp.yml",
            "skills",
        ] {
            assert_eq!(target(path), None, "{path} was offered a load path");
        }
    }

    /// The summary line, and the rule that a zero never reaches it.
    #[test]
    fn the_facts_leave_out_what_did_not_happen() {
        let steps = vec![
            Step {
                at: "a".to_string(),
                from: None,
                outcome: Sync::Added,
            },
            Step {
                at: "b".to_string(),
                from: None,
                outcome: Sync::Conflict,
            },
        ];
        assert_eq!(
            facts(&steps),
            vec!["1 placed".to_string(), "1 left as yours".to_string()]
        );
        assert_eq!(facts(&[]), vec!["already current".to_string()]);
    }

    /// The manifest is a document, and it round-trips — a manifest that could
    /// not be read back would report every projected file as edited by hand.
    #[test]
    fn the_manifest_round_trips_through_its_own_document() {
        let before = placed(&[("skills/onboard-repo/SKILL.md", MINE)]);
        let text = serde_json::to_string_pretty(&before).unwrap();
        assert_eq!(serde_json::from_str::<Placed>(&text).unwrap(), before);
    }

    /// A manifest from a version that wrote nothing but `{}` still reads, so an
    /// upgrade does not report every projected file as yours.
    #[test]
    fn an_empty_manifest_document_reads_as_nothing_projected() {
        assert_eq!(
            serde_json::from_str::<Placed>("{}").unwrap(),
            Placed::default()
        );
    }
}
