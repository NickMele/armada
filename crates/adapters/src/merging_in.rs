//! Bringing a Job's branch up to a base by merging the base in. `#1131`.
//!
//! **A merge, never a rebase.** One pass meets every conflict at once, the
//! commits a person already reviewed keep their ids, and nothing that holds the
//! branch is rewritten — so a plain push carries the result.
//!
//! **Run as Fleet, with no hooks**, which is what `crate::commit`'s own commit
//! already is: the identity is `WHO`, and a repository's hooks do not run.

use std::path::Path;
use std::process::Output;

use adapter_traits::{NotDelivered, Worktree};
use git2::{Oid, Repository};

use crate::commit::WHO;
use crate::delivery::{git, last_line, said, unmerged_files};

/// What merging a base in came to.
#[derive(Debug)]
pub(crate) enum MergedIn {
    /// The branch holds the base and nothing is left to resolve.
    Clean { commits: usize },
    /// These files hold conflict markers, left for a Drone to clear.
    Conflicted { files: Vec<String> },
    /// The branch could not take the base, and it and the worktree are back to
    /// what they held.
    PutBack { files: Vec<String> },
}

/// What a conflict is answered with.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) enum OnConflict {
    /// A Drone is about to be put on this worktree, and clearing them is its
    /// opening work.
    LeaveTheMarkers,
    /// Nothing follows the call, so nothing is left half-merged.
    PutItBack,
}

/// Merge `onto` into the branch this worktree has checked out.
///
/// **A merge already part-way through is never started over.** It is a Drone's,
/// clearing the markers the last one left, and is answered as it stands.
pub(crate) fn merged_in(
    worktree: &Worktree,
    onto: &str,
    on_conflict: OnConflict,
) -> Result<MergedIn, NotDelivered> {
    if merge_in_progress(worktree) {
        let files = still_marked(worktree);
        return Ok(match files.is_empty() {
            false => MergedIn::Conflicted { files },
            true => MergedIn::PutBack { files },
        });
    }
    let before = rev_parsed(worktree, "HEAD")
        .ok_or_else(|| NotDelivered::of("reading the branch", "`HEAD` points at no commit"))?;
    let commits = commits_between(worktree, &before, onto);

    let first = merge(worktree, onto)?;
    if first.status.success() {
        return Ok(MergedIn::Clean { commits });
    }
    if merge_in_progress(worktree) {
        let files = unmerged_files(worktree);
        return Ok(match on_conflict {
            OnConflict::LeaveTheMarkers => MergedIn::Conflicted { files },
            OnConflict::PutItBack => {
                let _ = git(worktree, &["merge", "--abort"]);
                MergedIn::PutBack { files }
            }
        });
    }

    // Refused before it started, which is uncommitted work standing where the
    // merge writes. Set aside under this call's own tag, merged, put back.
    let Some(stashed) = set_aside(worktree)? else {
        return Err(NotDelivered::of("the merge", said(&first)));
    };
    let run = match merge(worktree, onto) {
        Ok(run) => run,
        Err(cause) => {
            let _ = restored(worktree, Some(&stashed));
            return Err(cause);
        }
    };
    if merge_in_progress(worktree) {
        // Nobody can clear a conflict while the work set aside is out of the
        // tree, so the merge is undone and the work goes back where it was.
        let files = unmerged_files(worktree);
        let _ = git(worktree, &["merge", "--abort"]);
        restored(worktree, Some(&stashed))?;
        return Ok(MergedIn::PutBack { files });
    }
    if !run.status.success() {
        let said = said(&run);
        let _ = restored(worktree, Some(&stashed));
        return Err(NotDelivered::of("the merge", said));
    }
    match restored(worktree, Some(&stashed))? {
        Restored::NothingToRestore | Restored::Clean => Ok(MergedIn::Clean { commits }),
        // The entry stays on the list either way: it is the other copy.
        Restored::Conflicted { files } => match on_conflict {
            OnConflict::LeaveTheMarkers => Ok(MergedIn::Conflicted { files }),
            OnConflict::PutItBack => {
                let _ = git(worktree, &["reset", "--hard", &before]);
                let _ = restored(worktree, Some(&stashed));
                Ok(MergedIn::PutBack { files })
            }
        },
    }
}

/// `git merge`, as Fleet. `--no-autostash` because this module does its own,
/// tagged, and a person's `merge.autoStash` would take the shared list's top.
fn merge(worktree: &Worktree, onto: &str) -> Result<Output, NotDelivered> {
    let name = format!("user.name={}", WHO.0);
    let email = format!("user.email={}", WHO.1);
    git(
        worktree,
        &[
            "-c",
            &name,
            "-c",
            &email,
            "-c",
            "commit.gpgSign=false",
            "merge",
            "--no-edit",
            "--no-verify",
            "--no-autostash",
            "--no-squash",
            "--ff",
            onto,
        ],
    )
}

/// Whether git is part-way through a merge in this worktree.
pub(crate) fn merge_in_progress(worktree: &Worktree) -> bool {
    git(worktree, &["rev-parse", "-q", "--verify", "MERGE_HEAD"])
        .is_ok_and(|run| run.status.success())
}

/// The commit a merge part-way through is merging in, where there is one.
pub(crate) fn merge_head(repo: &Repository) -> Option<Oid> {
    repo.refname_to_id("MERGE_HEAD").ok()
}

/// The unmerged paths whose file still holds a conflict marker. **The marker
/// is the gate**, not a Drone saying it cleared them.
pub(crate) fn still_marked(worktree: &Worktree) -> Vec<String> {
    unmerged_files(worktree)
        .into_iter()
        .filter(|file| {
            std::fs::read(Path::new(worktree.path()).join(file))
                .is_ok_and(|bytes| holds_a_marker(&bytes))
        })
        .collect()
}

/// Whether a line opens or closes a conflict hunk. `=======` is left out: a
/// Markdown heading underline is spelled the same way, and the other two are
/// always there when it is.
pub(crate) fn holds_a_marker(bytes: &[u8]) -> bool {
    bytes
        .split(|byte| *byte == b'\n')
        .any(|line| line.starts_with(b"<<<<<<<") || line.starts_with(b">>>>>>>"))
}

pub(crate) fn rev_parsed(worktree: &Worktree, r#ref: &str) -> Option<String> {
    let run = git(worktree, &["rev-parse", "--verify", "-q", r#ref]).ok()?;
    let line = last_line(&run);
    (run.status.success() && !line.is_empty()).then_some(line)
}

/// How many commits `onto` holds that `before` did not.
fn commits_between(worktree: &Worktree, before: &str, onto: &str) -> usize {
    let range = format!("{before}..{onto}");
    let Ok(run) = git(worktree, &["rev-list", "--count", &range]) else {
        return 0;
    };
    last_line(&run).parse().unwrap_or(0)
}

/// What the worktree held uncommitted, once it has been asked for back.
enum Restored {
    NothingToRestore,
    Clean,
    Conflicted { files: Vec<String> },
}

/// **Its own stash, never `--autostash`'s.** `refs/stash` is one list every
/// worktree of the repository shares, and `#1097` found a `pop` taking another
/// Job's entry off its top. This call's entry is found again by its tag.
fn stash_tag(worktree: &Worktree) -> String {
    format!("armada-merging-in:{}", worktree.branch())
}

/// Set aside whatever this worktree holds uncommitted. `None` where it held
/// nothing, and then no entry is made.
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

/// Put back what [`set_aside`] took, by the commit it made — never `git stash
/// pop`, which takes whatever is on top of the shared list.
fn restored(worktree: &Worktree, stashed: Option<&str>) -> Result<Restored, NotDelivered> {
    let Some(sha) = stashed else {
        return Ok(Restored::NothingToRestore);
    };
    let run = git(worktree, &["stash", "apply", sha])?;
    if !run.status.success() {
        let files = unmerged_files(worktree);
        if files.is_empty() {
            return Err(NotDelivered::of(
                "putting the branch's own change back",
                said(&run),
            ));
        }
        return Ok(Restored::Conflicted { files });
    }
    // Dropped only once it applied clean, and found by the commit rather than
    // by the position it was pushed at.
    if let Some(current) = ref_of(worktree, sha) {
        let _ = git(worktree, &["stash", "drop", &current]);
    }
    Ok(Restored::Clean)
}

fn sha_tagged(worktree: &Worktree, tag: &str) -> Option<String> {
    let run = git(worktree, &["stash", "list", "--format=%H %gs"]).ok()?;
    String::from_utf8_lossy(&run.stdout)
        .lines()
        .find_map(|line| {
            let (sha, subject) = line.split_once(' ')?;
            subject.contains(tag).then(|| sha.to_string())
        })
}

fn ref_of(worktree: &Worktree, sha: &str) -> Option<String> {
    let run = git(worktree, &["stash", "list", "--format=%H %gd"]).ok()?;
    String::from_utf8_lossy(&run.stdout)
        .lines()
        .find_map(|line| {
            let (found, name) = line.split_once(' ')?;
            (found == sha).then(|| name.to_string())
        })
}
