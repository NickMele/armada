//! `~/.armada/`, and **the line between what syncs and what never does**
//! (`PLAN.md` §13.1).
//!
//! ```text
//! ~/.armada/
//! ├── guild/                 # SYNCS — a git repo Armada manages
//! │   ├── voice.md
//! │   ├── expectations.md
//! │   ├── how-i-work.md
//! │   ├── skills/
//! │   ├── hooks/
//! │   ├── subagents/
//! │   ├── workflows/
//! │   ├── plugins.yml
//! │   ├── mcp.yml
//! │   └── permissions.yml     # what a Drone may do unattended
//! ├── manifest.db            # NEVER SYNCS — ports, containers, leases, here
//! ├── jobs/                  # NEVER SYNCS — the Job index
//! ├── workspaces/            # NEVER SYNCS — the git worktrees themselves
//! ├── machine.yml            # NEVER SYNCS — paths, capacity, what was withheld
//! └── projection.json        # NEVER SYNCS — what was placed in THIS ~/.claude/
//! ```
//!
//! **The line is not "content syncs, state does not".** The guild *is* state and
//! it *does* sync. The line is: what describes **you** syncs; what describes
//! **this machine and its running processes** never does. Syncing `manifest.db`
//! to another machine would claim ports that do not exist there and record
//! containers that were never started.
//!
//! # Why the rule is a value here rather than a `.gitignore` line
//!
//! Because the git repository is `guild/` and nothing else, the four
//! never-syncing entries are outside it by construction — they cannot be
//! committed even by accident, because they are not in the working tree.
//! [`NEVER_SYNCS`] therefore exists to be *checked*, not to be enforced:
//! `armada doctor` reads it, and the test at the bottom of this file holds it
//! against the layout so that a future entry added to `~/.armada/` inside
//! `guild/` fails here rather than on somebody's remote.
//!
//! # Guild does not know where `~/.armada` is
//!
//! It is told. `armada-manifest` has an `armada_home`, and Guild may not name
//! Manifest (`ARCHITECTURE.md` §1.9) — so the caller, which is Helm and may
//! name both, passes the path in. That is the sibling rule doing its job rather
//! than getting in the way: a Guild that could resolve `~/.armada` for itself
//! would be a Guild that had opinions about `manifest.db`.

use std::path::{Path, PathBuf};

/// The directory names `armada init` creates under `~/.armada/`.
///
/// In the order `armada init` reports them, which is the order they appear in
/// `PLAN.md` §13.1's tree.
pub const DIRECTORIES: [&str; 3] = ["guild", "jobs", "workspaces"];

/// What each of [`DIRECTORIES`] is for, in the words a reader needs to decide
/// whether its absence matters.
///
/// **`armada doctor` used to report `missing layout — no jobs, workspaces`**,
/// which names an implementation and a set difference and tells a reader
/// nothing. A directory is worth restoring because something writes to it, so
/// the check says what.
/// **Kept short on purpose.** It is read into one line of a `doctor` row beside
/// the paths themselves, and a detail that truncates has lost the half a reader
/// came for.
pub fn holds(directory: &str) -> &'static str {
    match directory {
        "guild" => "your guild",
        "jobs" => "Jobs",
        "workspaces" => "worktrees",
        _ => "nothing Armada knows about",
    }
}

/// Everything under `~/.armada/` that describes **this machine** and therefore
/// never leaves it.
///
/// Read by `armada doctor`, and by the test below that holds it against
/// [`Guild::root`].
///
/// `projection.json` is here because it records what was placed in **this**
/// machine's `~/.claude/` ([`crate::project::MANIFEST`]). Syncing it would tell
/// a second machine that files it has never written are Armada's to overwrite,
/// which is the one mistake projection may not make.
pub const NEVER_SYNCS: [&str; 5] = [
    "manifest.db",
    "jobs",
    "workspaces",
    "machine.yml",
    crate::project::MANIFEST,
];

/// The directories a guild is made of, under `guild/`.
pub const GUILD_DIRECTORIES: [&str; 4] = ["hooks", "skills", "subagents", "workflows"];

/// A tree the guild and `~/.claude/` both hold, named at each end.
///
/// The two names differ once: Claude Code calls them `agents` and Armada calls
/// them `subagents`, and [`glossary.md`](../../../docs/glossary.md) is the
/// single definition of that word.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Tree {
    /// What it is called under `~/.claude/`.
    pub claude: &'static str,
    /// What it is called under `~/.armada/guild/`.
    pub guild: &'static str,
}

/// The three trees that travel between `~/.claude/` and a guild.
///
/// **One mapping, read in both directions.** [`crate::import`] reads it
/// left-to-right to adopt what is already on the machine;
/// [`crate::project`] reads it right-to-left to put the guild back on Claude
/// Code's load path. Two copies of this table would be two chances for
/// projection to write `subagents/` into a directory Claude Code does not read
/// — which is the whole failure `PHASES.md` §8.4 records.
pub const TREES: [Tree; 3] = [
    Tree {
        claude: "skills",
        guild: "skills",
    },
    Tree {
        claude: "agents",
        guild: "subagents",
    },
    Tree {
        claude: "hooks",
        guild: "hooks",
    },
];

/// The starter skill that writes a repository's `armada.yml` with you — the
/// guild's first real content (PLAN.md §13.4), and what `armada manifest config
/// scan` hands over to once it has produced the evidence.
pub const ONBOARD_REPO: &str = "onboard-repo";

/// Where a skill's prose lives inside a guild, written the way a person types
/// it.
///
/// **`~` rather than an absolute path**, because this string is printed and
/// pasted rather than opened: a shell expands it, it is shorter to read, and it
/// keeps somebody's home directory out of a terminal they are about to screen
/// -share. The code that *reads* the file uses [`Guild::skill`], which has the
/// real root.
pub fn skill_home_path(name: &str) -> String {
    format!("~/.armada/guild/skills/{name}/SKILL.md")
}

/// The argv that opens a session **carrying a guild skill's instructions**.
///
/// **The body, not the name, and that is the whole point.** The first version
/// built `claude /onboard-repo`, which invokes the skill as a slash command —
/// and a slash command only exists if Claude Code can find the skill.
/// Guild skills live in `~/.armada/guild/skills/`, Claude Code reads
/// `~/.claude/skills/`, and projection between the two is not built
/// ([`PHASES.md`](../../../docs/PHASES.md) §8.4). So the session opened with
/// `unknown command /onboard-repo`: Armada handed the user to a tool that could
/// not see the skill Armada ships.
///
/// Passing the prose as an appended system prompt needs no projection and no
/// discovery. It works today, and it keeps working if skill loading changes.
///
/// **`--append-system-prompt` rather than `--system-prompt`**: the session
/// keeps everything Claude Code normally is and gains the repository's
/// onboarding instructions, rather than being reduced to them. Verified against
/// the installed CLI's own option list rather than assumed — which is how the
/// Drone once shipped without `--verbose` (`docs/traps.md`).
///
/// **The opening turn is part of the argv, and leaving it out was the second
/// version's bug.** `--append-system-prompt` gives the session instructions; it
/// does not give it anything to do. Armada handed the reader a brand-new
/// session that knew how to onboard a repository and had not been asked to,
/// so it sat at an empty prompt — the same hand-over failing for a second
/// reason, one level further in than `unknown command /onboard-repo`.
///
/// `claude [options] [command] [prompt]` takes the first turn as a positional,
/// which is why [`OPENING`] goes last and unflagged. **Checking that a flag is
/// accepted is not checking that the session does anything**, and that gap is
/// exactly what the `--verbose` trap taught; the assertion that belongs here is
/// on the opening turn being present, which is what the tests below make.
pub fn skill_argv(body: &str) -> Vec<String> {
    vec![
        armada_core::fleet::drone::CLAUDE.to_string(),
        "--append-system-prompt".to_string(),
        body.to_string(),
        OPENING.to_string(),
    ]
}

/// The first turn of a handed-over session — what the reader would have typed.
///
/// **Phrased as the person, not as Armada.** It is delivered as their turn, and
/// the skill's own `description` triggers on *"someone asks to onboard,
/// configure or set up a repo for Armada"* — so this says that, in those words,
/// rather than instructing the agent in a voice the transcript will attribute
/// to the user.
pub const OPENING: &str = "Onboard this repository for Armada.";

/// The same hand-over, written as a line a person can paste.
///
/// **Two renderings of one thing, and both have to work.** [`skill_argv`] is
/// what Armada execs, with the prose already read; this is what it *prints*
/// for a reader who is not at a terminal, and inlining a multi-kilobyte
/// `SKILL.md` into that line would make it unreadable and unpasteable. `$(cat
/// …)` is what a person would type to achieve exactly the argv above.
pub fn skill_command_line(name: &str) -> String {
    format!(
        "{} --append-system-prompt \"$(cat {})\" \"{}\"",
        armada_core::fleet::drone::CLAUDE,
        skill_home_path(name),
        OPENING
    )
}

/// Where a guild lives, and every path inside it.
///
/// A value rather than a set of free functions so that a caller cannot
/// accidentally compose a path from two different roots — which is the bug that
/// writes a test's fixture into the real `~/.armada/`.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Guild {
    root: PathBuf,
}

impl Guild {
    /// The guild under a given `~/.armada/`.
    pub fn at(armada_home: &Path) -> Guild {
        Guild {
            root: armada_home.join("guild"),
        }
    }

    /// `~/.armada/guild/`.
    pub fn root(&self) -> &Path {
        &self.root
    }

    /// A path inside the guild.
    pub fn path(&self, relative: &str) -> PathBuf {
        self.root.join(relative)
    }

    /// Whether this guild carries a named skill.
    ///
    /// **The `SKILL.md` is the test, not the directory.** `guild init` creates
    /// `skills/` before anything is in it, and a user who deleted the skill's
    /// body left a directory behind — either way, offering to launch a skill
    /// that is not there produces a failure at the moment the reader was
    /// expecting help.
    pub fn has_skill(&self, name: &str) -> bool {
        self.skill(name).is_file()
    }

    /// Where a skill's prose actually is, for the code that reads it.
    ///
    /// The counterpart of [`skill_home_path`], which is the same location
    /// written for a person to paste rather than for a process to open.
    pub fn skill(&self, name: &str) -> PathBuf {
        self.path(&format!("skills/{name}/SKILL.md"))
    }

    /// Whether this machine has a guild at all.
    ///
    /// **The `.git` directory is the test, not the guild directory.** `armada
    /// init` creates `guild/` as one of the three empty directories long before
    /// anything is in it, so an existence check on the directory would report a
    /// guild on a machine that has none — and `--force`, which exists to refuse
    /// to overwrite one, would then refuse every first run.
    pub fn exists(&self) -> bool {
        self.root.join(".git").is_dir()
    }
}

/// How a value under `~/.armada/` is treated by sync.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Sync {
    /// It describes you.
    Syncs,
    /// It describes this machine.
    Never,
}

/// Which side of the line a top-level entry of `~/.armada/` falls.
pub fn sync_of(entry: &str) -> Sync {
    if NEVER_SYNCS.contains(&entry) {
        Sync::Never
    } else {
        Sync::Syncs
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// **The bug, as an assertion.** The first version passed the skill's
    /// *name* as a slash command, and the session it opened answered `Unknown
    /// command: /onboard-repo` — because guild skills live in
    /// `~/.armada/guild/skills/` and Claude Code reads `~/.claude/skills/`, and
    /// nothing projects between them (`PHASES.md` §8.4).
    ///
    /// Asserting on the argv is the same rule Fleet's Drone follows and for the
    /// same reason: **no test spawns a real session or spends a token**
    /// (`PHASES.md` §8), and argv is where this class of bug lives.
    #[test]
    fn the_session_is_handed_the_skills_prose_and_not_its_name() {
        let argv = skill_argv("# Onboard a repository\n\nWrite it with them.\n");
        assert_eq!(argv[0], "claude");
        assert_eq!(
            argv[1], "--append-system-prompt",
            "verified against the installed CLI's own option list — the Drone \
             once shipped a flag that did not exist"
        );
        assert!(
            argv[2].contains("Onboard a repository"),
            "the instructions did not reach the session: {:?}",
            argv[2]
        );
        // **The opening turn, and this assertion is the one that was missing.**
        // The version before this asserted the flag and the prose and stopped
        // at `argv.len() == 3` — so it went green against a hand-over that
        // opened a session with instructions and nothing to act on, and the
        // reader got a brand-new prompt sitting there doing nothing. A flag
        // being accepted is not the session doing something.
        assert_eq!(
            argv[3], OPENING,
            "the session was given instructions but never asked to start: {argv:?}"
        );
        assert_eq!(argv.len(), 4);
        assert!(
            !argv[3].starts_with('-'),
            "the opening turn is positional, and a leading dash makes it a flag: {:?}",
            argv[3]
        );
        assert!(
            !argv.iter().any(|word| word.starts_with('/')),
            "a slash command is what did not work: {argv:?}"
        );
    }

    /// The printed form and the exec'd form are two renderings of one
    /// hand-over, so they have to name the same flag and the same file. A
    /// printed command that does something else is worse than none — it is
    /// wrong in the one place a reader has no way to check.
    #[test]
    fn the_printed_command_and_the_argv_agree() {
        let printed = skill_command_line(ONBOARD_REPO);
        assert!(
            printed.starts_with("claude --append-system-prompt "),
            "{printed}"
        );
        assert!(
            printed.contains(&skill_home_path(ONBOARD_REPO)),
            "{printed}"
        );
        // `$(cat …)` is what produces the argv above when a person pastes it.
        assert!(printed.contains("\"$(cat "), "{printed}");
        // **No absolute home in a printed line**, which is both a privacy
        // matter and a readability one — and `~` is what a person would type.
        assert!(!printed.contains("/Users/"), "{printed}");
        assert!(!printed.contains("/home/"), "{printed}");
        // **The opening turn is in both renderings or in neither.** A reader who
        // pastes the printed line and a reader Armada exec's for must land in
        // the same session; a printed command that opens an idle prompt is the
        // bug this pair exists to keep out of exactly one of the two.
        assert!(
            printed.ends_with(&format!("\"{OPENING}\"")),
            "the printed hand-over opens an idle session: {printed}"
        );
    }

    /// `has_skill` and the path that reads it must never disagree: the check
    /// that decides whether to offer the hand-over is the check that finds the
    /// file it hands over.
    #[test]
    fn the_skill_that_is_tested_for_is_the_skill_that_is_read() {
        let guild = Guild::at(Path::new("/scratch/.armada"));
        assert_eq!(
            guild.skill(ONBOARD_REPO),
            Path::new("/scratch/.armada/guild/skills/onboard-repo/SKILL.md")
        );
        assert!(!guild.has_skill(ONBOARD_REPO), "nothing is on disk there");
    }

    #[test]
    fn the_guild_hangs_off_the_armada_home_it_was_given() {
        let guild = Guild::at(Path::new("/scratch/.armada"));
        assert_eq!(guild.root(), Path::new("/scratch/.armada/guild"));
        assert_eq!(
            guild.path("workflows/bug.yml"),
            PathBuf::from("/scratch/.armada/guild/workflows/bug.yml")
        );
    }

    /// **The load-bearing test of this file.** Everything that never syncs must
    /// sit *outside* the git repository, so that it cannot be committed even by
    /// a bug — and the way to check that is that no never-syncing name is one
    /// of the guild's own directories.
    #[test]
    fn nothing_that_never_syncs_lives_inside_the_repository_that_syncs() {
        for entry in NEVER_SYNCS {
            assert!(
                !GUILD_DIRECTORIES.contains(&entry),
                "`{entry}` never syncs and yet is inside guild/, which is the \
                 directory that does. A secret that has reached a remote cannot \
                 be un-pushed (PLAN.md §13.5)."
            );
            assert_eq!(sync_of(entry), Sync::Never);
        }
    }

    /// Every directory `armada init` creates can say what it is for. A new one
    /// with no answer would report `nothing Armada knows about`, which is a
    /// failing test rather than a row a reader cannot act on.
    #[test]
    fn every_created_directory_says_what_it_holds() {
        for directory in DIRECTORIES {
            assert_ne!(
                holds(directory),
                "nothing Armada knows about",
                "`{directory}` is created and cannot say why it matters"
            );
        }
    }

    /// **Every tree the guild carries is a directory the guild is made of.** A
    /// tree named here but missing from [`GUILD_DIRECTORIES`] would be one
    /// import filled and `guild pull` never reported, or one projection wrote
    /// from a directory nothing ever creates.
    #[test]
    fn every_tree_that_travels_is_one_of_the_guilds_own_directories() {
        for tree in TREES {
            assert!(
                GUILD_DIRECTORIES.contains(&tree.guild),
                "`{}` travels and yet is not one of the guild's directories",
                tree.guild
            );
        }
    }

    /// The one place the two vocabularies differ, asserted so that a rename on
    /// either side fails here rather than in somebody's session.
    #[test]
    fn claude_codes_agents_are_armadas_subagents() {
        let subagents = TREES
            .iter()
            .find(|tree| tree.guild == "subagents")
            .expect("the guild carries subagents");
        assert_eq!(subagents.claude, "agents");
    }

    /// `guild/` is the one of the three created directories that travels.
    #[test]
    fn only_the_guild_directory_syncs() {
        assert_eq!(sync_of("guild"), Sync::Syncs);
        assert_eq!(sync_of("jobs"), Sync::Never);
        assert_eq!(sync_of("workspaces"), Sync::Never);
        assert_eq!(sync_of("machine.yml"), Sync::Never);
        assert_eq!(sync_of("manifest.db"), Sync::Never);
        assert_eq!(sync_of(crate::project::MANIFEST), Sync::Never);
    }

    /// An empty `guild/` is not a guild. `armada init` makes the directory on
    /// every machine; only `guild init`, `pull` or `import` makes a guild.
    #[test]
    fn an_empty_directory_is_not_a_guild() {
        let home = tempfile::tempdir().unwrap();
        let guild = Guild::at(home.path());
        std::fs::create_dir_all(guild.root()).unwrap();
        assert!(!guild.exists(), "an empty guild/ reported as a guild");

        std::fs::create_dir_all(guild.root().join(".git")).unwrap();
        assert!(guild.exists());
    }
}
