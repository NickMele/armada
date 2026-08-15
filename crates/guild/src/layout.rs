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
//! │   └── mcp.yml
//! ├── manifest.db            # NEVER SYNCS — ports, containers, leases, here
//! ├── jobs/                  # NEVER SYNCS — the Job index
//! ├── workspaces/            # NEVER SYNCS — the git worktrees themselves
//! └── machine.yml            # NEVER SYNCS — paths, capacity, what was withheld
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
pub const NEVER_SYNCS: [&str; 4] = ["manifest.db", "jobs", "workspaces", "machine.yml"];

/// The directories a guild is made of, under `guild/`.
pub const GUILD_DIRECTORIES: [&str; 4] = ["hooks", "skills", "subagents", "workflows"];

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

    /// `guild/` is the one of the three created directories that travels.
    #[test]
    fn only_the_guild_directory_syncs() {
        assert_eq!(sync_of("guild"), Sync::Syncs);
        assert_eq!(sync_of("jobs"), Sync::Never);
        assert_eq!(sync_of("workspaces"), Sync::Never);
        assert_eq!(sync_of("machine.yml"), Sync::Never);
        assert_eq!(sync_of("manifest.db"), Sync::Never);
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
