//! The shell of projection: reading two directories, writing one of them, and
//! keeping the manifest honest.
//!
//! Every decision is [`crate::project`]'s. What lives here is the walking, the
//! hashing and the four filesystem calls — and the order they go in, which is
//! the one thing this file decides:
//!
//! 1. **Plan first, against what is on disk right now.** Nothing is written
//!    until every file has been hashed, so a projection that is going to leave
//!    something alone knows it before it has written anything.
//! 2. **Write, then record.** The manifest is written last, from the steps that
//!    actually ran. A manifest written first and then abandoned by a failure
//!    would claim files Armada does not own, and the next `--remove` would
//!    delete somebody else's.
//!
//! # Nothing here reads `$HOME`
//!
//! `~/.armada` and `~/.claude` both arrive as paths, which is
//! `ARCHITECTURE.md` §1.4 and also what lets every test in this file — and the
//! whole suite above it — point at a `TempDir` rather than at the reader's real
//! setup. Projection is the one part of Armada that writes into a directory
//! somebody else's tool owns; a test that could reach the real one is not a
//! test anybody should run twice.

use crate::layout::Guild;
use crate::project::{self, Offered, Placed, Step};
use std::collections::BTreeMap;
use std::path::{Path, PathBuf};

/// What a projection did, and what it now claims.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Projected {
    /// One step per path, in path order.
    pub steps: Vec<Step>,
    /// The manifest as it now stands.
    pub placed: Placed,
}

impl Projected {
    /// The files left alone because the reader had edited them.
    pub fn yours(&self) -> Vec<&str> {
        self.steps
            .iter()
            .filter(|step| step.is_yours())
            .map(|step| step.at.as_str())
            .collect()
    }

    /// Whether anything was written or deleted.
    pub fn changed_anything(&self) -> bool {
        self.steps
            .iter()
            .any(|step| step.writes() || step.deletes())
    }
}

/// Put the guild on Claude Code's load path, and record what was put there.
///
/// **The reader's own files are never touched.** A path Armada did not place,
/// or placed and the reader has since edited, is left exactly as it is and
/// comes back as a `CONFLICT` step.
pub fn project(
    guild: &Guild,
    claude_home: &Path,
    armada_home: &Path,
) -> std::io::Result<Projected> {
    let desired = offers(guild);
    let placed = read_manifest(armada_home);
    let steps = project::plan(&desired, &present(claude_home, &desired, &placed), &placed);

    for step in &steps {
        let at = claude_home.join(&step.at);
        if step.writes() {
            let Some(from) = &step.from else { continue };
            if let Some(parent) = at.parent() {
                std::fs::create_dir_all(parent)?;
            }
            std::fs::copy(guild.path(from), &at)?;
        } else if step.deletes() {
            std::fs::remove_file(&at)?;
            prune(claude_home, at.parent());
        }
    }

    let after = project::manifest(&steps, &desired, &placed);
    write_manifest(armada_home, &after)?;
    Ok(Projected {
        steps,
        placed: after,
    })
}

/// Take back exactly what was placed — `armada guild project --remove`.
///
/// Reads the manifest and never the guild, so the reversal is bounded by what
/// was actually written rather than by what the guild happens to hold now.
pub fn remove(claude_home: &Path, armada_home: &Path) -> std::io::Result<Projected> {
    let placed = read_manifest(armada_home);
    let steps = project::reversal(&present(claude_home, &BTreeMap::new(), &placed), &placed);

    for step in &steps {
        if step.deletes() {
            let at = claude_home.join(&step.at);
            std::fs::remove_file(&at)?;
            prune(claude_home, at.parent());
        }
    }

    // **What survived the reversal is what is still claimed**, which is nothing
    // except the files left alone — and those are no longer Armada's either
    // (`project::manifest`), so the manifest ends empty and the next `--remove`
    // deletes nothing.
    let after = Placed::default();
    write_manifest(armada_home, &after)?;
    Ok(Projected {
        steps,
        placed: after,
    })
}

/// What a projection **would** do, having written nothing.
///
/// `armada doctor` is read-only, and a check that had to project in order to
/// report on projection would be a check nobody could run twice.
pub fn survey(guild: &Guild, claude_home: &Path, armada_home: &Path) -> Vec<Step> {
    let desired = offers(guild);
    let placed = read_manifest(armada_home);
    project::plan(&desired, &present(claude_home, &desired, &placed), &placed)
}

/// Every file the guild offers Claude Code, keyed on where it lands.
///
/// A guild that is not there offers nothing, which is the ordinary state of a
/// machine that has run `armada init` and not yet `guild init`.
fn offers(guild: &Guild) -> BTreeMap<String, Offered> {
    let mut out = BTreeMap::new();
    for tree in crate::layout::TREES {
        for from in walk(guild.root(), &guild.path(tree.guild)) {
            let Some(at) = project::target(&from) else {
                continue;
            };
            let Ok(bytes) = std::fs::read(guild.path(&from)) else {
                continue;
            };
            out.insert(
                at,
                Offered {
                    from,
                    hash: project::hash(&bytes),
                },
            );
        }
    }
    out
}

/// The hash of what is at each candidate path right now.
///
/// **Only the union of the two key sets is hashed.** Projection looks at what
/// it offers and at what it once wrote, and at nothing else in the reader's
/// `~/.claude/` — so a directory of two hundred skills somebody else's tool put
/// there is neither read nor reported.
fn present(
    claude_home: &Path,
    desired: &BTreeMap<String, Offered>,
    placed: &Placed,
) -> BTreeMap<String, String> {
    desired
        .keys()
        .chain(placed.files.keys())
        .filter_map(|at| {
            std::fs::read(claude_home.join(at))
                .ok()
                .map(|bytes| (at.clone(), project::hash(&bytes)))
        })
        .collect()
}

/// Every regular file under a directory, as a path relative to `root`.
fn walk(root: &Path, at: &Path) -> Vec<String> {
    let mut out = Vec::new();
    let Ok(listing) = std::fs::read_dir(at) else {
        return out;
    };
    for entry in listing.filter_map(Result::ok) {
        // A dotfile is the editor's or the shell's, never content — the same
        // rule `inventory` and `import` both apply.
        if entry.file_name().to_string_lossy().starts_with('.') {
            continue;
        }
        let path = entry.path();
        if path.is_dir() {
            out.extend(walk(root, &path));
        } else if let Ok(relative) = path.strip_prefix(root) {
            out.push(relative.display().to_string());
        }
    }
    out.sort();
    out
}

/// Remove the directories a withdrawal emptied, and stop at the first one that
/// still holds something.
///
/// **Never `~/.claude/skills/` itself.** Withdrawing the last projected skill
/// must not take a directory Claude Code expects to exist with it, and must
/// certainly not take one holding the reader's own skills — which the
/// non-empty check already guarantees, and the floor guarantees again.
fn prune(claude_home: &Path, from: Option<&Path>) {
    let mut at = from.map(Path::to_path_buf);
    while let Some(directory) = at {
        if directory
            .parent()
            .is_none_or(|parent| parent == claude_home)
        {
            return;
        }
        if std::fs::remove_dir(&directory).is_err() {
            return;
        }
        at = directory.parent().map(Path::to_path_buf);
    }
}

/// Where the manifest lives.
pub fn manifest_path(armada_home: &Path) -> PathBuf {
    armada_home.join(project::MANIFEST)
}

/// The manifest, or nothing projected.
///
/// **An unreadable or unparseable manifest reads as nothing projected**, which
/// is the safe direction: every file then looks like the reader's, so
/// projection places what is missing and touches nothing it finds. The
/// alternative — refusing to run — would leave a machine with no way to project
/// because of a file nobody reads by hand.
fn read_manifest(armada_home: &Path) -> Placed {
    std::fs::read_to_string(manifest_path(armada_home))
        .ok()
        .and_then(|text| serde_json::from_str(&text).ok())
        .unwrap_or_default()
}

fn write_manifest(armada_home: &Path, placed: &Placed) -> std::io::Result<()> {
    std::fs::create_dir_all(armada_home)?;
    let text = serde_json::to_string_pretty(placed)?;
    std::fs::write(manifest_path(armada_home), format!("{text}\n"))
}

#[cfg(test)]
mod tests {
    use super::*;
    use armada_core::envelope::Sync;
    use std::fs;

    struct Machine {
        home: tempfile::TempDir,
    }

    impl Machine {
        /// A scratch machine with a guild holding one of each projected thing.
        ///
        /// **Nothing in this file ever names a real `$HOME`.** Projection is
        /// the one part of Armada that writes into somebody else's tool's
        /// directory, and a test that could reach the real one is not a test
        /// anybody should run twice.
        fn new() -> Machine {
            let machine = Machine {
                home: tempfile::tempdir().unwrap(),
            };
            let guild = machine.guild();
            fs::create_dir_all(guild.path("skills/onboard-repo")).unwrap();
            fs::create_dir_all(guild.path("subagents")).unwrap();
            fs::create_dir_all(guild.path("hooks")).unwrap();
            fs::create_dir_all(guild.path("workflows")).unwrap();
            fs::write(guild.path("skills/onboard-repo/SKILL.md"), "# onboard\n").unwrap();
            fs::write(guild.path("subagents/helm.md"), "# helm\n").unwrap();
            fs::write(guild.path("hooks/stop-notify.sh"), "#!/bin/sh\n").unwrap();
            fs::write(guild.path("workflows/bug.yml"), "name: bug\n").unwrap();
            fs::write(guild.path("voice.md"), "brief\n").unwrap();
            fs::create_dir_all(machine.claude_home()).unwrap();
            machine
        }

        fn armada_home(&self) -> PathBuf {
            self.home.path().join(".armada")
        }

        fn claude_home(&self) -> PathBuf {
            self.home.path().join(".claude")
        }

        fn guild(&self) -> Guild {
            Guild::at(&self.armada_home())
        }

        fn project(&self) -> Projected {
            project(&self.guild(), &self.claude_home(), &self.armada_home()).unwrap()
        }

        fn read(&self, at: &str) -> Option<String> {
            fs::read_to_string(self.claude_home().join(at)).ok()
        }
    }

    fn outcome<'a>(projected: &'a Projected, at: &str) -> Option<&'a Step> {
        projected.steps.iter().find(|step| step.at == at)
    }

    /// The whole point: a guild skill is where Claude Code reads skills, a
    /// guild subagent is where it reads agents, and a hook is where the
    /// registrations that fire it point.
    #[test]
    fn a_guild_lands_where_claude_code_reads_it() {
        let machine = Machine::new();
        let projected = machine.project();

        assert_eq!(
            machine.read("skills/onboard-repo/SKILL.md").as_deref(),
            Some("# onboard\n")
        );
        assert_eq!(
            machine.read("agents/helm.md").as_deref(),
            Some("# helm\n"),
            "Claude Code calls them agents"
        );
        assert_eq!(
            machine.read("hooks/stop-notify.sh").as_deref(),
            Some("#!/bin/sh\n")
        );
        assert!(projected.yours().is_empty());
        assert_eq!(projected.steps.len(), 3);
    }

    /// **Workflows and the memory fragments are not placed.** Workflows are
    /// Armada's own to read, and a `voice.md` dropped into `~/.claude/` is read
    /// by nothing — the personal half is written by hand (`PLAN.md` §13.3).
    #[test]
    fn nothing_claude_code_has_no_load_path_for_is_written() {
        let machine = Machine::new();
        machine.project();
        assert_eq!(machine.read("workflows/bug.yml"), None);
        assert_eq!(machine.read("voice.md"), None);
    }

    /// **The rule that must not break.** Place, edit by hand, re-project: the
    /// edit survives, nothing was written over it, and it is reported.
    #[test]
    fn a_file_you_edited_survives_a_re_projection_and_is_reported() {
        let machine = Machine::new();
        machine.project();

        let mine = "# onboard\n\nAnd then ask about the database.\n";
        fs::write(
            machine.claude_home().join("skills/onboard-repo/SKILL.md"),
            mine,
        )
        .unwrap();
        // The guild moves on underneath it, which is what a `guild pull` does.
        fs::write(
            machine.guild().path("skills/onboard-repo/SKILL.md"),
            "# onboard\n\nSomebody else's version.\n",
        )
        .unwrap();

        let again = machine.project();
        assert_eq!(
            machine.read("skills/onboard-repo/SKILL.md").as_deref(),
            Some(mine),
            "the edit was overwritten — this is the failure the manifest exists to prevent"
        );
        assert_eq!(again.yours(), vec!["skills/onboard-repo/SKILL.md"]);
        assert_eq!(
            outcome(&again, "skills/onboard-repo/SKILL.md")
                .unwrap()
                .outcome,
            Sync::Conflict
        );
    }

    /// And a file that was already the reader's before Armada ever ran is
    /// equally untouchable — there is no record saying Armada placed it.
    #[test]
    fn a_file_that_was_already_yours_is_not_taken_over() {
        let machine = Machine::new();
        let mine = "# my own onboarding\n";
        fs::create_dir_all(machine.claude_home().join("skills/onboard-repo")).unwrap();
        fs::write(
            machine.claude_home().join("skills/onboard-repo/SKILL.md"),
            mine,
        )
        .unwrap();

        let projected = machine.project();
        assert_eq!(
            machine.read("skills/onboard-repo/SKILL.md").as_deref(),
            Some(mine)
        );
        assert_eq!(projected.yours(), vec!["skills/onboard-repo/SKILL.md"]);
    }

    /// A guild that moved on is written through — the half of the rule that
    /// makes re-projection worth running at all.
    #[test]
    fn armadas_own_copy_is_brought_up_to_date() {
        let machine = Machine::new();
        machine.project();
        fs::write(
            machine.guild().path("subagents/helm.md"),
            "# helm, revised\n",
        )
        .unwrap();

        let again = machine.project();
        assert_eq!(
            machine.read("agents/helm.md").as_deref(),
            Some("# helm, revised\n")
        );
        assert_eq!(
            outcome(&again, "agents/helm.md").unwrap().outcome,
            Sync::Changed
        );
    }

    /// Projecting twice with nothing changed writes nothing and says so.
    #[test]
    fn a_second_projection_with_nothing_to_do_reports_nothing_to_do() {
        let machine = Machine::new();
        machine.project();
        let again = machine.project();
        assert!(!again.changed_anything());
        assert!(again
            .steps
            .iter()
            .all(|step| step.outcome == Sync::Unchanged));
    }

    /// A skill deleted from the guild on another machine stops being carried
    /// here — otherwise a projection would only ever grow.
    #[test]
    fn a_skill_the_guild_lost_is_withdrawn() {
        let machine = Machine::new();
        machine.project();
        fs::remove_dir_all(machine.guild().path("skills/onboard-repo")).unwrap();

        let again = machine.project();
        assert_eq!(machine.read("skills/onboard-repo/SKILL.md"), None);
        assert!(
            !machine.claude_home().join("skills/onboard-repo").exists(),
            "the emptied directory was left behind"
        );
        assert_eq!(
            outcome(&again, "skills/onboard-repo/SKILL.md")
                .unwrap()
                .outcome,
            Sync::Removed
        );
    }

    /// **`--remove` reverses exactly what was placed, and nothing else.** The
    /// reader's own skill, sitting in the same directory, is still there
    /// afterwards.
    #[test]
    fn remove_takes_back_what_was_placed_and_leaves_everything_else() {
        let machine = Machine::new();
        fs::create_dir_all(machine.claude_home().join("skills/mine")).unwrap();
        fs::write(machine.claude_home().join("skills/mine/SKILL.md"), "mine\n").unwrap();
        machine.project();

        let removed = remove(&machine.claude_home(), &machine.armada_home()).unwrap();
        assert_eq!(machine.read("skills/onboard-repo/SKILL.md"), None);
        assert_eq!(machine.read("agents/helm.md"), None);
        assert_eq!(machine.read("hooks/stop-notify.sh"), None);
        assert_eq!(
            machine.read("skills/mine/SKILL.md").as_deref(),
            Some("mine\n"),
            "--remove reached a file it never placed"
        );
        assert!(removed.placed.is_empty());
        assert!(
            machine.claude_home().join("skills").is_dir(),
            "the directory Claude Code reads was taken with it"
        );
    }

    /// And a projected file the reader has since edited survives `--remove`
    /// too: it is no longer what was placed.
    #[test]
    fn remove_leaves_a_projected_file_you_edited() {
        let machine = Machine::new();
        machine.project();
        fs::write(machine.claude_home().join("agents/helm.md"), "# mine now\n").unwrap();

        let removed = remove(&machine.claude_home(), &machine.armada_home()).unwrap();
        assert_eq!(
            machine.read("agents/helm.md").as_deref(),
            Some("# mine now\n")
        );
        assert_eq!(removed.yours(), vec!["agents/helm.md"]);
    }

    /// `--remove` twice deletes nothing the second time, because the manifest
    /// is empty and the reversal reads only the manifest.
    #[test]
    fn removing_twice_is_not_a_second_deletion() {
        let machine = Machine::new();
        machine.project();
        remove(&machine.claude_home(), &machine.armada_home()).unwrap();
        let again = remove(&machine.claude_home(), &machine.armada_home()).unwrap();
        assert!(again.steps.is_empty(), "{:?}", again.steps);
    }

    /// A machine with no guild projects nothing rather than failing — the
    /// ordinary state between `armada init` and `armada guild init`.
    #[test]
    fn a_machine_with_no_guild_projects_nothing() {
        let home = tempfile::tempdir().unwrap();
        let armada_home = home.path().join(".armada");
        let projected = project(
            &Guild::at(&armada_home),
            &home.path().join(".claude"),
            &armada_home,
        )
        .unwrap();
        assert!(projected.steps.is_empty());
        assert!(projected.placed.is_empty());
    }

    /// A `~/.claude/` that does not exist yet is created, which is what a fresh
    /// machine has.
    #[test]
    fn a_machine_with_no_claude_directory_gets_one() {
        let machine = Machine::new();
        fs::remove_dir_all(machine.claude_home()).unwrap();
        machine.project();
        assert_eq!(
            machine.read("skills/onboard-repo/SKILL.md").as_deref(),
            Some("# onboard\n")
        );
    }

    /// **The survey writes nothing**, which is what makes `armada doctor`
    /// read-only.
    #[test]
    fn a_survey_reports_what_a_projection_would_do_and_writes_nothing() {
        let machine = Machine::new();
        let steps = survey(
            &machine.guild(),
            &machine.claude_home(),
            &machine.armada_home(),
        );
        assert_eq!(steps.len(), 3);
        assert!(steps.iter().all(|step| step.outcome == Sync::Added));
        assert_eq!(machine.read("agents/helm.md"), None);
        assert!(!manifest_path(&machine.armada_home()).exists());
    }

    /// **A manifest nobody can read is nothing projected, not a refusal.** It
    /// fails safe in the one direction that cannot lose work: with no record,
    /// every file already there looks like the reader's, so projection places
    /// what is missing and touches nothing it finds. Refusing to run instead
    /// would leave a machine unable to project because of a file nobody edits
    /// by hand.
    #[test]
    fn an_unreadable_manifest_is_read_as_nothing_projected() {
        let machine = Machine::new();
        machine.project();
        fs::write(manifest_path(&machine.armada_home()), "{ not json").unwrap();

        let mine = "# mine now\n";
        fs::write(machine.claude_home().join("agents/helm.md"), mine).unwrap();
        let again = machine.project();
        assert_eq!(machine.read("agents/helm.md").as_deref(), Some(mine));
        assert_eq!(
            again.yours(),
            vec![
                "agents/helm.md",
                "hooks/stop-notify.sh",
                "skills/onboard-repo/SKILL.md"
            ],
            "with no record, everything already there is the reader's"
        );
        assert!(!again.changed_anything());
    }

    /// The manifest lands outside the git repository that syncs, so it cannot
    /// be committed even by a bug (`layout::NEVER_SYNCS`).
    #[test]
    fn the_manifest_is_outside_the_repository_that_syncs() {
        let machine = Machine::new();
        machine.project();
        let manifest = manifest_path(&machine.armada_home());
        assert!(manifest.is_file());
        assert!(
            !manifest.starts_with(machine.guild().root()),
            "the manifest is inside the guild, which syncs"
        );
    }
}
