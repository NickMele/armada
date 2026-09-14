//! Rebasing a Job's branch onto a base that moved, and pushing it — in place
//! of closing and reopening the pull request. `#663`.
//!
//! # A worktree Fleet already has, or one this borrows for the length of a call
//!
//! A finished Job's worktree is very often gone by the time its base moves: a
//! terminal Job's worktree is only kept while its branch is unmerged, and a
//! person may still give the disk back by hand. [`kept_current`] does not
//! require one to exist. It derives the same path [`WorktreeSpec`] always
//! derives, and where nothing is there, attaches a worktree onto the branch
//! that already exists — never creating one — runs the rebase in it, and
//! removes the checkout again before returning. The branch, which is the
//! work, is never touched by the removal.
//!
//! # Every conflict is undone, not left for a Drone
//!
//! `crate::delivery::bring_up_to_date` leaves conflict markers in a worktree a
//! Drone is about to read, because resolving them is the opening work of the
//! turn that follows. Nothing follows this call — the Job is done — so a
//! conflict here is undone completely: a rebase that fails outright is
//! aborted, which git already restores from the autostash, and a rebase that
//! succeeds but leaves the *reapplied* stash conflicted is undone by hand,
//! resetting the branch to where it stood and popping the stash again onto a
//! tree that now matches what it was taken from. Either way the branch a
//! person is watching is exactly the branch they had.

use std::path::Path;

use adapter_traits::{KeptCurrent, NotDelivered, Pushed, Worktree, WorktreeSpec};
use git2::{BranchType, Repository, WorktreeAddOptions, WorktreePruneOptions};

use crate::delivery::{
    a_remote, git, last_line, open, rebase_in_progress, run_in, said, unmerged_files,
};

/// The local commit a base branch is on, asked from the repository every
/// worktree is cut from — never a worktree, so a Job whose worktree is gone
/// is asked exactly as cheaply as one whose worktree is still there.
pub(crate) fn base_tip(in_repo: &str, base: &str) -> Option<String> {
    let run = run_in(in_repo, "git", &["rev-parse", base]).ok()?;
    let line = last_line(&run);
    (run.status.success() && !line.is_empty()).then_some(line)
}

/// See the module.
pub(crate) fn kept_current(in_repo: &str, handle: &str, base: &str) -> KeptCurrent {
    let Ok(spec) = WorktreeSpec::for_job(in_repo, handle) else {
        // Unreachable in practice: `handle` is read off a Job the store
        // already holds, which passed this exact derivation to create its
        // worktree in the first place. Treated as `NoBranch` rather than a
        // tool refusal, because there is equally nothing to rebase.
        return KeptCurrent::NoBranch;
    };
    let repo = match Repository::open(in_repo) {
        Ok(repo) => repo,
        Err(cause) => {
            return KeptCurrent::NotDelivered(NotDelivered::of(
                "opening the repository",
                cause.message(),
            ))
        }
    };
    if repo.find_branch(&spec.branch(), BranchType::Local).is_err() {
        return KeptCurrent::NoBranch;
    }

    let already_there = Path::new(&spec.worktree_path()).exists();
    let worktree = match already_there {
        true => Worktree::at(spec.worktree_path(), spec.branch()),
        false => match attached(&repo, &spec) {
            Ok(worktree) => worktree,
            Err(cause) => return KeptCurrent::NotDelivered(cause),
        },
    };

    let outcome = rebased(&worktree, base);
    if !already_there {
        // Best-effort, and after the answer is already decided: the checkout
        // was this call's own scaffolding, not a fact anybody reads. Leaving
        // it behind on a failed detach costs disk and nothing else — the
        // branch, which is the work, is untouched either way.
        detached(&repo, &spec);
    }
    outcome
}

/// Attach a worktree onto a branch that already exists, never creating one.
///
/// **The opposite refusal from `crate::worktree`'s `create_worktree`.** That
/// one exists to stop two Jobs sharing a line of history and refuses a branch
/// that is already there; this is reached only because [`kept_current`]
/// already found the branch, checked out nowhere on disk, and wants a working
/// tree for it rather than a second one.
fn attached(repo: &Repository, spec: &WorktreeSpec) -> Result<Worktree, NotDelivered> {
    // A directory that failed to clean up after an earlier attempt — the
    // process died mid-call, say — leaves a registration `repo.worktree`
    // below refuses to reuse. Cleared the way `crate::worktree` clears the
    // same shape of leftover, and best-effort: a registration that will not
    // prune surfaces as the `git2` error the add below raises on its own.
    if let Ok(registered) = repo.find_worktree(spec.registration_name()) {
        if registered.is_prunable(None).unwrap_or(false) {
            let _ = registered.prune(None);
        }
    }
    let branch = repo
        .find_branch(&spec.branch(), BranchType::Local)
        .map_err(|cause| NotDelivered::of("finding the pull request's branch", cause.message()))?;
    let reference = branch.into_reference();
    let path = spec.worktree_path();
    std::fs::create_dir_all(spec.worktree_parent())
        .map_err(|cause| NotDelivered::of("preparing a scratch worktree", cause.to_string()))?;
    let mut options = WorktreeAddOptions::new();
    options.reference(Some(&reference));
    repo.worktree(spec.registration_name(), Path::new(&path), Some(&options))
        .map_err(|cause| {
            NotDelivered::of(
                "attaching a scratch worktree onto the pull request's branch",
                cause.message(),
            )
        })?;
    Ok(Worktree::at(path, spec.branch()))
}

/// Remove a worktree this call attached, directory and record together.
///
/// **`working_tree(true)`, unlike the ordinary create path's cleanup.** A
/// worktree Fleet hands a Drone survives every terminal state because nothing
/// here may delete one — `crate::worktree`'s module header says so. This one
/// was never that: it exists for the length of one call, and the branch it
/// was checked out on is what survives, not the directory.
fn detached(repo: &Repository, spec: &WorktreeSpec) {
    let Ok(registered) = repo.find_worktree(spec.registration_name()) else {
        let _ = std::fs::remove_dir_all(spec.worktree_path());
        return;
    };
    let mut options = WorktreePruneOptions::new();
    options.valid(true).working_tree(true);
    let _ = registered.prune(Some(&mut options));
    // Prune refuses a worktree it cannot prove valid — locked, say — and the
    // directory is left rather than fought over: the next sweep finds the
    // path still occupied and reports the true reason through the ordinary
    // worktree, not through this one's best effort.
}

/// Rebase in place, leaving the branch exactly as it was on any conflict.
///
/// **Its own stash, never `--autostash`'s.** `refs/stash` is one list shared
/// by every worktree of the repository. `#1097` found a rebase whose
/// reapplied autostash conflicted, undone by a second `git stash pop` —
/// `pop` always takes whatever is on top of that shared list, which by then
/// may belong to a different Job's [`kept_current`] running at the same
/// moment. What is set aside here carries a tag naming this call, so it is
/// found again by that tag rather than by position — [`set_aside`],
/// [`restored`].
fn rebased(worktree: &Worktree, base: &str) -> KeptCurrent {
    let before = match rev_parsed(worktree, "HEAD") {
        Some(oid) => oid,
        None => {
            return KeptCurrent::NotDelivered(NotDelivered::of(
                "reading the branch",
                "`HEAD` points at no commit",
            ))
        }
    };
    let onto = match rev_parsed(worktree, base) {
        Some(oid) => oid,
        None => {
            return KeptCurrent::NotDelivered(NotDelivered::of(
                "reading the base",
                format!("`{base}` points at no commit"),
            ))
        }
    };
    let commits = commits_between(worktree, &before, &onto);

    let stashed = match set_aside(worktree) {
        Ok(stashed) => stashed,
        Err(cause) => return KeptCurrent::NotDelivered(cause),
    };

    let run = match git(worktree, &["rebase", base]) {
        Ok(run) => run,
        Err(cause) => {
            let _ = restored(worktree, stashed.as_deref());
            return KeptCurrent::NotDelivered(cause);
        }
    };

    if !run.status.success() {
        // Nobody can be handed half a rebase, so a conflicted replay is
        // undone completely: aborted — which restores the tree to `before`
        // on its own — and what this call set aside goes back onto exactly
        // the tree it was taken from.
        let files = unmerged_files(worktree);
        if rebase_in_progress(worktree) {
            let _ = git(worktree, &["rebase", "--abort"]);
            return match restored(worktree, stashed.as_deref()) {
                Ok(_) => KeptCurrent::Conflicted { onto, files },
                Err(cause) => KeptCurrent::NotDelivered(cause),
            };
        }
        let said = said(&run);
        let _ = restored(worktree, stashed.as_deref());
        return KeptCurrent::NotDelivered(NotDelivered::of("the rebase", said));
    }

    match restored(worktree, stashed.as_deref()) {
        Ok(Restored::NothingToRestore | Restored::Clean) => match pushed_forcing(worktree) {
            Ok(_) => KeptCurrent::Rebased { onto, commits },
            Err(cause) => KeptCurrent::NotDelivered(cause),
        },
        // The rebase itself replayed cleanly and putting this call's own
        // change back onto the *new* tip conflicted with what moved on the
        // base. Undone completely: back to `before`, where the very same
        // entry cannot conflict — it is the tree it was taken from — so the
        // worktree ends this holding the change plainly, not stranded in the
        // stash.
        Ok(Restored::Conflicted { files }) => {
            let _ = git(worktree, &["reset", "--hard", &before]);
            let _ = restored(worktree, stashed.as_deref());
            KeptCurrent::Conflicted { onto, files }
        }
        Err(cause) => KeptCurrent::NotDelivered(cause),
    }
}

/// What this branch held uncommitted before the rebase started, once it has
/// been asked for back. `NothingToRestore` is the ordinary case for
/// [`kept_current`] — the Job is done and its worktree has nothing left
/// uncommitted — answered without a stash ever existing.
enum Restored {
    NothingToRestore,
    Clean,
    Conflicted { files: Vec<String> },
}

/// A tag naming this call's own stash entry, so it is found again by what it
/// says rather than by where it sits in the shared list.
fn stash_tag(worktree: &Worktree) -> String {
    format!("armada-kept-current:{}", worktree.branch())
}

/// Set aside whatever this worktree holds uncommitted, tagged so it can be
/// found again by name. `None` where the worktree was already clean — no
/// stash is made for nothing to set aside.
fn set_aside(worktree: &Worktree) -> Result<Option<String>, NotDelivered> {
    let status = git(worktree, &["status", "--porcelain"])?;
    if !status.status.success() {
        return Err(NotDelivered::of(
            "reading the worktree's own state",
            said(&status),
        ));
    }
    if status.stdout.is_empty() {
        return Ok(None);
    }
    let tag = stash_tag(worktree);
    let run = git(worktree, &["stash", "push", "-u", "-m", &tag])?;
    if !run.status.success() {
        return Err(NotDelivered::of(
            "setting the branch's own change aside",
            said(&run),
        ));
    }
    match sha_tagged(worktree, &tag) {
        Some(sha) => Ok(Some(sha)),
        None => Err(NotDelivered::of(
            "setting the branch's own change aside",
            "git reported success, but no stash entry carries the name it was given",
        )),
    }
}

/// Put back what [`set_aside`] took, by the exact commit it made. **Never
/// `git stash pop`**, which takes whatever is on top of the list this
/// repository's every worktree shares — this call's own entry may no longer
/// be there by the time it looks.
fn restored(worktree: &Worktree, stashed: Option<&str>) -> Result<Restored, NotDelivered> {
    let Some(sha) = stashed else {
        return Ok(Restored::NothingToRestore);
    };
    let run = git(worktree, &["stash", "apply", sha])?;
    if !run.status.success() {
        let files = unmerged_files(worktree);
        if files.is_empty() {
            // Not a conflict — the entry itself would not apply, which
            // nothing here can recover from silently.
            return Err(NotDelivered::of(
                "putting the branch's own change back",
                said(&run),
            ));
        }
        return Ok(Restored::Conflicted { files });
    }
    // Dropped only once it is confirmed applied clean, and found by the same
    // tag rather than assumed still at the position it was pushed to.
    if let Some(current) = ref_of(worktree, sha) {
        let _ = git(worktree, &["stash", "drop", &current]);
    }
    Ok(Restored::Clean)
}

/// The SHA of the entry whose subject carries this tag, read fresh — never
/// assumed to still be on top, because another worktree's own stash can land
/// above it between [`set_aside`] and [`restored`].
fn sha_tagged(worktree: &Worktree, tag: &str) -> Option<String> {
    let run = git(worktree, &["stash", "list", "--format=%H %gs"]).ok()?;
    String::from_utf8_lossy(&run.stdout)
        .lines()
        .find_map(|line| {
            let (sha, subject) = line.split_once(' ')?;
            subject.contains(tag).then(|| sha.to_string())
        })
}

/// This entry's current position in the shared list — `stash@{N}` — read
/// fresh at drop time by matching the commit [`set_aside`] made, never by the
/// index it was pushed at.
fn ref_of(worktree: &Worktree, sha: &str) -> Option<String> {
    let run = git(worktree, &["stash", "list", "--format=%H %gd"]).ok()?;
    String::from_utf8_lossy(&run.stdout)
        .lines()
        .find_map(|line| {
            let (found, name) = line.split_once(' ')?;
            (found == sha).then(|| name.to_string())
        })
}

/// Push with `--force-with-lease`, the one push here that is not
/// `crate::delivery::push`'s. **A rewritten history is exactly what a rebase
/// produces**, so the ordinary push — which git refuses over one — is not
/// this call's push to reuse. `Delivery::push_forcing`'s implementation, and
/// the one this module's own rebase calls once a rebase is clean.
pub(crate) fn pushed_forcing(worktree: &Worktree) -> Result<Pushed, NotDelivered> {
    let repo = open(worktree)?;
    let Some(remote) = a_remote(&repo) else {
        // No remote is ordinary everywhere else `crate::delivery` reads it,
        // and it is ordinary here too: the branch is the work, and a rebase
        // that moved it is content for a person to read locally.
        return Ok(Pushed::NoRemote);
    };
    let branch = worktree.branch().to_string();
    let run = git(
        worktree,
        &[
            "push",
            "--force-with-lease",
            "--set-upstream",
            &remote,
            &branch,
        ],
    )?;
    match run.status.success() {
        true => Ok(Pushed::ToTheRemote { remote, branch }),
        false => Err(NotDelivered::of("the push", said(&run))),
    }
}

fn rev_parsed(worktree: &Worktree, r#ref: &str) -> Option<String> {
    let run = git(worktree, &["rev-parse", r#ref]).ok()?;
    let line = last_line(&run);
    (run.status.success() && !line.is_empty()).then_some(line)
}

/// How many commits `onto` holds that `before` did not — what the base was
/// ahead by, for a person reading what the rebase carried across.
fn commits_between(worktree: &Worktree, before: &str, onto: &str) -> usize {
    let range = format!("{before}..{onto}");
    let Ok(run) = git(worktree, &["rev-list", "--count", &range]) else {
        return 0;
    };
    last_line(&run).parse().unwrap_or(0)
}
