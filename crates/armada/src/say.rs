//! What each verb prints, so the acting half never holds a format string.
//!
//! # A destructive command that prints nothing is one nobody trusts twice
//!
//! `clean` says what it removed, item by item, including the tip of every
//! branch it deleted — a deleted branch is recoverable from its SHA and from
//! nothing else, so the SHA goes on the screen while the person is still
//! looking at it.
//!
//! Absences are printed too. "There was nothing there" and "it was removed" are
//! different answers, and a command that shows only the second leaves the
//! reader to guess which one they got.

use std::path::Path;

use adapters::{BranchGone, WorktreeGone};

use crate::clean::{Cleaned, FileGone};
use crate::declared::{Ended, Ran};

/// What one Check or Command did: its output, then how it ended.
pub fn ran(ran: &Ran, verb: &str) {
    println!("{verb} {} — {}", ran.name, ran.command);
    if ran.destructive {
        println!("  the Manifest calls this destructive");
    }
    // The whole of what it printed, both streams, before the verdict. Nothing
    // here reads a byte of it — `checks-runner` does not either.
    print!("{}", ran.attempt.output.stdout);
    eprint!("{}", ran.attempt.output.stderr);
    if ran.attempt.output.truncated {
        println!("  (output was longer than the capture limit; this is the tail)");
    }
    println!("{} {}", ran.name, Ended(&ran.attempt.exit));
}

/// Everything a clean removed, and everything it left where it was.
pub fn cleaned(cleaned: &Cleaned) {
    println!(
        "{} — Manifest `{}`",
        cleaned.repository.display(),
        cleaned.manifest_id
    );

    for job in &cleaned.jobs {
        println!("\n{} — {}", job.job_id, job.title);
        println!("  {}", worktree(&job.reclaimed.worktree));
        println!("  {}", branch(&job.reclaimed.branch));
        if job.forgotten.existed {
            println!(
                "  forgotten: {} event(s), {} step(s)",
                job.forgotten.events, job.forgotten.steps
            );
        }
    }

    if !cleaned.unclaimed.is_empty() {
        println!("\nno Job claims these, and they are left alone:");
        for path in &cleaned.unclaimed {
            println!("  {}", path.display());
        }
    }
    if cleaned.unreadable > 0 {
        println!(
            "\n{} row(s) in the store would not rebuild, so nothing here \
             could derive a worktree for them",
            cleaned.unreadable
        );
    }

    if let Some(directory) = cleaned
        .machine
        .first()
        .and_then(|first| held(first).parent())
    {
        // The directory once, then five file names. Five absolute paths under
        // one directory is four repetitions of the directory.
        println!("\nmachine state, in {}:", directory.display());
        for file in &cleaned.machine {
            println!("  {}", machine(file));
        }
    }

    if cleaned.jobs.is_empty() && cleaned.unclaimed.is_empty() {
        println!("\nno Jobs and no worktrees — there was nothing to give back");
    }
    for fault in &cleaned.faults {
        eprintln!("  {fault}");
    }
}

fn worktree(gone: &WorktreeGone) -> String {
    match gone {
        WorktreeGone::Removed { path } => format!("worktree removed: {path}"),
        WorktreeGone::RecordCleared { path } => {
            format!("worktree registration cleared, the checkout was already gone: {path}")
        }
        WorktreeGone::DirectoryRemoved { path } => {
            format!("directory removed, git had no record of it: {path}")
        }
        WorktreeGone::Absent { path } => format!("no worktree: {path}"),
        WorktreeGone::Locked { path, reason } => {
            format!("worktree left alone, it is locked — {reason}: {path}")
        }
        WorktreeGone::NotRemoved { path, why } => {
            format!("worktree NOT removed — {why}: {path}")
        }
    }
}

fn branch(gone: &BranchGone) -> String {
    match gone {
        // The tip, always. Nine branches deleted by hand were recoverable only
        // because their SHAs were still on the screen.
        BranchGone::Deleted { branch, tip } => {
            format!("branch deleted: {branch} was at {tip}")
        }
        BranchGone::Absent { branch } => format!("no branch: {branch}"),
        BranchGone::NotDeleted { branch, why } => {
            format!("branch NOT deleted — {why}: {branch}")
        }
    }
}

fn machine(file: &FileGone) -> String {
    let name = shown(held(file));
    match file {
        FileGone::Removed(_) => format!("removed: {name}"),
        FileGone::Absent(_) => format!("not there: {name}"),
        FileGone::NotRemoved { why, .. } => format!("NOT removed — {why}: {name}"),
    }
}

fn held(file: &FileGone) -> &Path {
    match file {
        FileGone::Removed(path) | FileGone::Absent(path) => path,
        FileGone::NotRemoved { path, .. } => path,
    }
}

fn shown(path: &Path) -> String {
    path.file_name()
        .map(|name| name.to_string_lossy().into_owned())
        .unwrap_or_else(|| path.display().to_string())
}
