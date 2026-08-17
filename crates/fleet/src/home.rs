//! Where Fleet's three directories are, and how a path is written for a person.
//!
//! `~/.armada/jobs/`, `~/.armada/workspaces/` and `~/.armada/inbox.jsonl` all
//! **never sync** (`armada_guild::layout`): they describe this machine and the
//! processes on it, and a Job index copied to another machine would name
//! worktrees that do not exist there.
//!
//! **Worktrees live outside the repository**, under `~/.armada/workspaces/`, so
//! a stray delete in the parent cannot take out live work (PLAN.md §14.1).

use std::path::{Path, PathBuf};

/// The Job index — one file per Job, named for its uuid.
///
/// **A directory of small files rather than one index**, because the failure
/// mode of a single file is that a crash mid-write loses every Job rather than
/// one, and because two `spawn`s racing would have to agree on a lock. The same
/// reasoning that put Manifest's ownership store in SQLite applies inverted
/// here: there is no query to run across Jobs that a directory listing cannot
/// answer.
pub fn jobs(armada_home: &Path) -> PathBuf {
    armada_home.join("jobs")
}

/// A Job's transcript: `~/.armada/jobs/<uuid>.stream.jsonl`.
///
/// **Beside the record and outside the worktree**, for two reasons that both
/// matter. `armada fleet kill` drops the worktree and keeps the transcript —
/// that is the verb's stated contract, because the transcript is the record of
/// what happened. And a detached Drone reports to nobody, so this file *is* the
/// ledger: `armada fleet ls` reads it to answer what a Job has spent, and a Job
/// whose ledger lived in the tree being torn down would lose its own history at
/// the moment somebody wanted it most.
pub fn stream(armada_home: &Path, uuid: &str) -> PathBuf {
    jobs(armada_home).join(format!("{uuid}.stream.jsonl"))
}

/// A Job's `Stop` hook: `~/.armada/jobs/<uuid>.stop.sh`.
///
/// **Beside the transcript rather than inside the worktree**, for the same
/// reason: `armada fleet kill` drops the tree, and a hook that lived there
/// would be pulled out from under a Drone that is still exiting.
///
/// **Per Job, not per machine.** The hook has to name the Job it relays for,
/// and a single shared file would be rewritten under every running Drone every
/// time another one spawned.
pub fn stop_hook(armada_home: &Path, uuid: &str) -> PathBuf {
    jobs(armada_home).join(format!("{uuid}.stop.sh"))
}

/// The `--settings` document that registers it:
/// `~/.armada/jobs/<uuid>.settings.json`.
pub fn drone_settings(armada_home: &Path, uuid: &str) -> PathBuf {
    jobs(armada_home).join(format!("{uuid}.settings.json"))
}

/// The `--mcp-config` document that attaches Armada's own server:
/// `~/.armada/jobs/<uuid>.mcp.json`.
///
/// **Per Job rather than one shared file**, for the same reason the settings
/// document is: the two are written together at spawn, live and die with the
/// Job, and are reclaimed by the same pass. A shared file would be a thing
/// every Job depends on and no Job owns.
pub fn drone_mcp(armada_home: &Path, uuid: &str) -> PathBuf {
    jobs(armada_home).join(format!("{uuid}.mcp.json"))
}

/// The lock one `armada fleet tick` pass holds: `~/.armada/tick.lock`.
///
/// **One pass at a time, machine-wide.** Every Drone's `Stop` hook sweeps the
/// whole fleet (`020` §2), so on a machine with five Jobs five passes can start
/// within the same second — and two passes gating one step would both call
/// `--resume` on one Claude Code session. The lock is what makes the sweep
/// affordable.
pub fn tick_lock(armada_home: &Path) -> PathBuf {
    armada_home.join("tick.lock")
}

/// Where a Job's git worktree goes: `~/.armada/workspaces/<repo>/<name>`.
///
/// The repository's name is a level of the path because two repositories may
/// each have a Job called `rate-limit`, and a flat directory would make the
/// second one collide with the first.
pub fn worktree(armada_home: &Path, repo: &str, name: &str) -> PathBuf {
    armada_home.join("workspaces").join(repo).join(name)
}

/// `~/.armada/inbox.jsonl` — **append-only, so it survives every kind of
/// crash** (PLAN.md §15.3). The same reasoning that put the ownership store on
/// disk rather than in a process.
pub fn inbox(armada_home: &Path) -> PathBuf {
    armada_home.join("inbox.jsonl")
}

/// `~/.armada/daemon.jsonl` — what the daemon did that is about no Job (`034`
/// §6.5). The same append-only shape [`inbox`] is, for the same reason: a
/// crash mid-write must leave a torn last line, never a corrupt file.
pub fn daemon_log(armada_home: &Path) -> PathBuf {
    armada_home.join("daemon.jsonl")
}

/// `~/.armada/daemon.pid` — the running daemon's pid, while it is running.
///
/// **A bare number, not a handle.** The daemon is a single process this
/// machine starts and stops through `armada daemon enable`/`disable`, never
/// signalled by anything that has to prove a pgid was not recycled the way a
/// Drone's group does ([`crate::drone::stop`]) — so the file holds exactly
/// what `armada daemon start` wrote and `armada daemon stop` removes it.
pub fn daemon_pidfile(armada_home: &Path) -> PathBuf {
    armada_home.join("daemon.pid")
}

/// A path as a person writes it: `~/…` when it is under `$HOME`.
///
/// **The payload carries this form, not the absolute one**, which is the
/// convention `armada guild init`'s `guild_path` already set. A `~` path is
/// still a path a shell will `cd` to, and `armada fleet board`'s whole output is
/// two lines somebody pastes.
pub fn tilde(path: &Path, home: &Path) -> String {
    match path.strip_prefix(home) {
        Ok(rest) => format!("~/{}", rest.display()),
        Err(_) => path.display().to_string(),
    }
}

/// The repository's name, for the worktree path and the Job record.
///
/// The last component of the root, which is what a person calls the repository
/// whatever the remote is named.
pub fn repo_name(repo_root: &Path) -> String {
    repo_root
        .file_name()
        .map(|name| name.to_string_lossy().to_string())
        .unwrap_or_else(|| "repo".to_string())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn the_three_directories_hang_off_the_armada_home_they_were_given() {
        let home = Path::new("/scratch/.armada");
        assert_eq!(jobs(home), PathBuf::from("/scratch/.armada/jobs"));
        assert_eq!(
            inbox(home),
            PathBuf::from("/scratch/.armada/inbox.jsonl"),
            "the inbox is a file, not a directory"
        );
        assert_eq!(
            daemon_log(home),
            PathBuf::from("/scratch/.armada/daemon.jsonl")
        );
        assert_eq!(
            daemon_pidfile(home),
            PathBuf::from("/scratch/.armada/daemon.pid")
        );
    }

    /// **Two repositories may each have a `rate-limit`.** The repository is a
    /// level of the path so the second one does not land on the first.
    #[test]
    fn a_worktree_is_named_for_its_repository_and_its_job() {
        assert_eq!(
            worktree(Path::new("/scratch/.armada"), "api", "rate-limit"),
            PathBuf::from("/scratch/.armada/workspaces/api/rate-limit")
        );
        assert_ne!(
            worktree(Path::new("/s"), "api", "rate-limit"),
            worktree(Path::new("/s"), "web", "rate-limit")
        );
    }

    /// **The transcript sits beside the record, not inside the worktree.**
    /// `kill` drops the tree and keeps the transcript, and the transcript is
    /// also the ledger — a Job that lost its own history the moment it was
    /// killed would lose it exactly when somebody wanted it.
    #[test]
    fn a_transcript_lives_beside_the_record_and_outside_the_worktree() {
        let home = Path::new("/scratch/.armada");
        let stream = stream(home, "8f2a1c40");
        assert_eq!(
            stream,
            PathBuf::from("/scratch/.armada/jobs/8f2a1c40.stream.jsonl")
        );
        assert!(stream.starts_with(jobs(home)));
        assert!(!stream.starts_with(home.join("workspaces")));
    }

    /// The index lists `*.json` records; a transcript must not be read as one.
    #[test]
    fn a_transcript_is_not_mistaken_for_a_job_record() {
        let stream = stream(Path::new("/scratch/.armada"), "8f2a");
        assert_eq!(
            stream.extension().and_then(|e| e.to_str()),
            Some("jsonl"),
            "a `.json` transcript would be listed as a Job and fail to parse"
        );
    }

    #[test]
    fn a_path_under_home_is_written_the_way_a_person_writes_it() {
        let home = Path::new("/scratch/home");
        assert_eq!(
            tilde(Path::new("/scratch/home/.armada/workspaces/api"), home),
            "~/.armada/workspaces/api"
        );
        // Somewhere else is left alone: abbreviating a path that is not under
        // `$HOME` would produce one that does not exist.
        assert_eq!(tilde(Path::new("/opt/work"), home), "/opt/work");
    }

    #[test]
    fn a_repository_is_named_by_its_last_component() {
        assert_eq!(repo_name(Path::new("/code/api")), "api");
        assert_eq!(repo_name(Path::new("/")), "repo");
    }
}
