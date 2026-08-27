//! What each verb prints, so the acting half never holds a format string.
//!
//! # A destructive command that prints nothing is one nobody trusts twice
//!
//! `clean` says what it removed, item by item, including the tip of every
//! branch it deleted — a deleted branch is recoverable from its SHA and from
//! nothing else, so the SHA goes on the screen while the person is still
//! looking at it.
//!
//! A branch it kept is said twice on purpose — once beside its Job, and once
//! at the very end, because a clean that left work standing must not read as a
//! clean that finished.
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
    for row in &cleaned.unreadable {
        println!("\n{} — a row that would not rebuild", row.job_id);
        // Before the removals, not after: it is the last time anything can say
        // why, and a person reading this is deciding whether to stop.
        println!("  {}", row.why);
        println!("  {}", worktree(&row.reclaimed.worktree));
        println!("  {}", branch(&row.reclaimed.branch));
        if row.forgotten.existed {
            println!(
                "  row cleared: {} event(s), {} step(s)",
                row.forgotten.events, row.forgotten.steps
            );
        }
    }
    if cleaned.unreadable_elsewhere > 0 {
        println!(
            "\n{} row(s) would not rebuild and belong to another Manifest. \
             Run `armada clean` in that repository to clear them.",
            cleaned.unreadable_elsewhere
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

    if cleaned.jobs.is_empty() && cleaned.unclaimed.is_empty() && cleaned.unreadable.is_empty() {
        println!("\nno Jobs and no worktrees — there was nothing to give back");
    }
    // Last, because it is the only part of this a person still has to act on.
    branches_left(cleaned);
    for fault in &cleaned.faults {
        eprintln!("  {fault}");
    }
}

/// The branches that are still there, and what to do about each.
fn branches_left(cleaned: &Cleaned) {
    let left = cleaned.branches_left();
    if left.is_empty() {
        return;
    }
    println!(
        "\n{} branch(es) still hold work nothing has taken. Their worktrees \
         were removed; the branches were not:",
        left.len()
    );
    for gone in &left {
        println!("  {}", branch(gone));
    }
    println!(
        "Merge one, then `git branch -d <branch>` — git itself refuses that \
         while it is unmerged. `armada clean --force` deletes them instead, \
         and the commits with them."
    );
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
        BranchGone::Kept {
            branch,
            tip,
            base,
            commits,
        } => format!(
            "branch kept, {commits} commit(s) of its own are not on `{base}`: \
             {branch} is at {tip}"
        ),
        BranchGone::KeptUnanswered { branch, tip, why } => {
            format!("branch kept, {why}: {branch} is at {tip}")
        }
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
