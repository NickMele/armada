//! Merging a moved base into a Job's branch and pushing it, in place of closing
//! and reopening the pull request. `#663`, and a merge since `#1131`.
//!
//! **Its own worktree, or one borrowed for the call.** A finished Job's worktree
//! is often gone by the time its base moves, so where nothing is at the derived
//! path a worktree is attached onto the existing branch — never creating one —
//! and removed before returning. The branch, which is the work, is untouched.
//!
//! **A conflict is put back.** No Drone reads markers left here, so the branch
//! and the worktree stay as they were; Fleet's catch-up meets the same conflict
//! when it puts a Drone on the Job, and leaves the markers there.

use std::path::Path;

use adapter_traits::{KeptCurrent, NotDelivered, Pushed, Worktree, WorktreeSpec};
use git2::{BranchType, Repository, WorktreeAddOptions, WorktreePruneOptions};

use crate::delivery::{a_remote, git, last_line, open, pushed, run_in, said, unmerged_files};
use crate::merging_in::{merge_in_progress, merged_in, rev_parsed, MergedIn, OnConflict};

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
        // tool refusal, because there is equally nothing to merge into.
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

    let outcome = merged(&worktree, base);
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

/// Take what the remote's copy of the branch holds, merge the base in, push.
///
/// `KeptCurrent::Rebased` is the trait's name for a branch brought current,
/// and what it now reports is a merge.
fn merged(worktree: &Worktree, base: &str) -> KeptCurrent {
    let Some(onto) = rev_parsed(worktree, base) else {
        return KeptCurrent::NotDelivered(NotDelivered::of(
            "reading the base",
            format!("`{base}` points at no commit"),
        ));
    };
    // A Drone's merge, part-way through clearing its markers: answered as it
    // stands, and never merged over or pushed.
    if merge_in_progress(worktree) {
        return KeptCurrent::Conflicted {
            onto,
            files: unmerged_files(worktree),
        };
    }
    if let Err(cause) = taken_from_the_remote(worktree) {
        return KeptCurrent::NotDelivered(cause);
    }
    match merged_in(worktree, base, OnConflict::PutItBack) {
        Ok(MergedIn::Clean { commits }) => match pushed(worktree) {
            Ok(_) => KeptCurrent::Rebased { onto, commits },
            Err(cause) => KeptCurrent::NotDelivered(cause),
        },
        Ok(MergedIn::Conflicted { files } | MergedIn::PutBack { files }) => {
            KeptCurrent::Conflicted { onto, files }
        }
        Err(cause) => KeptCurrent::NotDelivered(cause),
    }
}

/// Merge in commits the pull request's branch holds on the remote and this
/// worktree does not — a person's own push — so the push after is not refused.
///
/// **A fetch that fails takes nothing**: a branch the remote never had is
/// ordinary, and a remote that will not answer is what the push then says.
fn taken_from_the_remote(worktree: &Worktree) -> Result<(), NotDelivered> {
    let Some(remote) = a_remote(&open(worktree)?) else {
        return Ok(());
    };
    let branch = worktree.branch();
    let tracking = format!("refs/remotes/{remote}/{branch}");
    let refspec = format!("+refs/heads/{branch}:{tracking}");
    let fetched = git(worktree, &["fetch", "--no-tags", &remote, &refspec])?;
    if !fetched.status.success() {
        return Ok(());
    }
    let held = git(
        worktree,
        &["merge-base", "--is-ancestor", &tracking, "HEAD"],
    )?;
    if held.status.success() {
        return Ok(());
    }
    match merged_in(worktree, &tracking, OnConflict::PutItBack)? {
        MergedIn::Clean { .. } => Ok(()),
        MergedIn::Conflicted { files } | MergedIn::PutBack { files } => Err(NotDelivered::of(
            "taking the commits the pull request's branch holds on the remote",
            format!(
                "they conflict with this branch's own in {}, so a person reconciles the two",
                files.join(", ")
            ),
        )),
    }
}

/// Push with `--force-with-lease`: `Delivery::push_forcing`'s implementation.
/// **Nothing of Fleet's takes it since `#1131`** — a merge rewrites nothing, so
/// the ordinary push carries every branch Fleet moves.
pub(crate) fn pushed_forcing(worktree: &Worktree) -> Result<Pushed, NotDelivered> {
    let repo = open(worktree)?;
    let Some(remote) = a_remote(&repo) else {
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
