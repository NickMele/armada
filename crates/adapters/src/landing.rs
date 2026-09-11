//! What became of a pull request after Armada opened it, and what the
//! repository it merges into should do about it.
//!
//! # A different moment from `crate::delivery`, which is why it is a different
//! file
//!
//! That module runs inside a Job's last turn, in the Job's own worktree, with
//! the work in hand. Everything here runs minutes or days later, from the
//! repository every worktree was cut from, about a pull request nobody on this
//! machine is holding. `#337` asked whether anybody merged it and `#427`
//! whether the forge is still comparing it against the right commit; both are
//! one `gh pr view`, because building them apart would be two processes where
//! one does.
//!
//! # Reading is free and writing is not
//!
//! [`read`] and [`caught_up`] are cheap and repeatable. [`rendered_afresh`]
//! closes and reopens a person's pull request, which everybody watching it is
//! mailed about — so it is called once per pull request, only for a base that
//! is provably superseded, and never on a reading this could not make.
//! [`merge`] is louder still and is the one act here nothing decides: a person
//! presses for it, and a refusal is classified from `mergeStateStatus` rather
//! than from the prose `gh` printed. A sentence [`why_not`] has no word for is
//! [`NotMerged::Refused`] carrying it verbatim, never a guess — the kinds exist
//! to send a person somewhere, and the wrong place is worse than the sentence.

use adapter_traits::{
    Landing, Mergeable, Merged, NotMerged, Rendering, Renewed, RepositoryStanding, WhatBecameOfIt,
};
use git2::Repository;

use crate::delivery::{asked, last_line, run_in, said, FORGE};

/// Ask the forge what became of one pull request, what it is showing beside
/// it, and what a person would read about it before either of those answers
/// mattered.
pub(crate) fn read(in_repo: &str, pull_request: &str) -> WhatBecameOfIt {
    // **Seven fields in one call**, `#427`'s argument carried one field
    // further: the number, the title and whether the forge can merge it are
    // three more scalars on the one `pr view` this already runs, and a second
    // call for any of them would be a process spent on an answer this one
    // already has. The address is not among them — it is the argument, so
    // asking for it back would be asking the forge to confirm what was just
    // handed to it.
    let Some(said) = asked(
        in_repo,
        pull_request,
        "number,title,state,mergeable,baseRefName,baseRefOid,headRefOid",
        "[.number, .title, .state, .mergeable, .baseRefName, .baseRefOid, .headRefOid] | @tsv",
    ) else {
        return WhatBecameOfIt::unknown();
    };
    let Some([number, title, state, mergeable, base, pinned, head]) = fields::<7>(&said) else {
        return WhatBecameOfIt::unknown();
    };
    let url = pull_request.to_string();
    let landing = match state {
        "MERGED" => Landing::Merged { url },
        "CLOSED" => Landing::ClosedUnmerged { url },
        "OPEN" => Landing::Open {
            url,
            rendering: rendering(in_repo, base, pinned, head),
        },
        // A word this forge has and Armada has not. Saying nothing is the
        // honest answer, and it costs one more ask later.
        _ => return WhatBecameOfIt::unknown(),
    };
    WhatBecameOfIt {
        landing,
        base: Some(base.to_string()),
        // A number the forge printed that this cannot parse is a forge
        // sending back something other than a number, which this reads the
        // same as not having asked — never a guessed id.
        number: number.parse().ok(),
        title: Some(title.to_string()),
        mergeable: match mergeable {
            "MERGEABLE" => Mergeable::Yes,
            "CONFLICTING" => Mergeable::No,
            // `UNKNOWN` is the forge still computing it, and a word this
            // build has no other name for is the same silence.
            _ => Mergeable::Unreadable,
        },
    }
}

/// Make the forge compare a pull request against the commit its branch sits on.
pub(crate) fn rendered_afresh(in_repo: &str, pull_request: &str) -> Renewed {
    // **The close is allowed to fail and the reopen is not.** A pull
    // request this already closed and could not reopen comes back here on a
    // later sweep, and refusing to run because it is closed already would
    // leave it closed for good.
    let _ = run_in(in_repo, FORGE, &["pr", "close", pull_request]);
    // Twice, because the failure this is guarding against is a blink of
    // network between two calls that run back to back, and a second attempt
    // costs one process where the alternative is a pull request left shut.
    for _ in 0..2 {
        if let Ok(run) = run_in(in_repo, FORGE, &["pr", "reopen", pull_request]) {
            if run.status.success() {
                return Renewed::Renewed;
            }
        }
    }
    Renewed::LeftClosed {
        why: format!("`{FORGE} pr reopen` would not reopen {pull_request}"),
    }
}

/// Merge a pull request Armada opened.
///
/// **Read, then write, and the read is what makes the refusal nameable.** `gh
/// pr merge` prints prose; `gh pr view` answers with the forge's own words for
/// the two states and one flag that decide most of it. So the state is asked
/// first — a pull request somebody already merged is not a failed press, and a
/// closed one is a different sentence from a conflicted one.
///
/// **`--merge`, and no other strategy.** A squash or a rebase rewrites what the
/// Job's Checks passed against, and `crate::proving` runs the repository's
/// after-merge Checks against the commit the base is left on: proving a tree
/// nobody produced is worse than not proving one.
pub(crate) fn merge(in_repo: &str, pull_request: &str) -> Result<Merged, NotMerged> {
    let Some(standing) = asked(
        in_repo,
        pull_request,
        "state,mergeable,mergeStateStatus",
        "[.state, .mergeable, .mergeStateStatus] | @tsv",
    ) else {
        return Err(NotMerged::NoTool {
            said: format!("`{FORGE} pr view {pull_request}` would not answer"),
        });
    };
    let Some([state, mergeable, status]) = fields::<3>(&standing) else {
        return Err(NotMerged::Refused {
            said: format!("`{FORGE}` answered `{standing}`, which has no reading here"),
        });
    };
    match state {
        // **Not a refusal.** The work is where the press was trying to put it.
        "MERGED" => return Ok(Merged::AlreadyMerged),
        "OPEN" => {}
        other => {
            return Err(NotMerged::NotOpen {
                said: format!("the forge says it is {other}"),
            })
        }
    }
    // Asked before the write, because a conflict is the one refusal the forge
    // will not phrase as one: `gh pr merge` on a conflicting branch fails with
    // a sentence about the merge method.
    if mergeable == "CONFLICTING" {
        return Err(NotMerged::Conflicted {
            said: String::from("the forge cannot merge this branch into its base as it stands"),
        });
    }
    let run = match run_in(in_repo, FORGE, &["pr", "merge", pull_request, "--merge"]) {
        Ok(run) => run,
        Err(why) => {
            return Err(NotMerged::NoTool {
                said: format!("`{FORGE}` would not run: {why}"),
            })
        }
    };
    match run.status.success() {
        true => Ok(Merged::Taken),
        false => Err(why_not(status, said(&run))),
    }
}

/// Which kind of refusal a failed merge was, from the forge's own state word.
///
/// **Three words have a reading and everything else is the sentence.** The
/// forge computes `mergeStateStatus` and names it, so a caller acting on one of
/// these three is acting on what the forge decided rather than on how `gh`
/// worded it that release.
fn why_not(status: &str, printed: String) -> NotMerged {
    match status {
        "DIRTY" => NotMerged::Conflicted { said: printed },
        "BLOCKED" => NotMerged::Protected { said: printed },
        "UNSTABLE" => NotMerged::ChecksNotPassed { said: printed },
        _ => NotMerged::Refused { said: printed },
    }
}

/// Bring the repository every worktree was cut from up to what just merged.
pub(crate) fn caught_up(in_repo: &str, base: &str) -> RepositoryStanding {
    let left_alone = |why: String| RepositoryStanding::LeftAlone { why };
    let repo = match Repository::open(in_repo) {
        Ok(repo) => repo,
        Err(cause) => return left_alone(cause.message().to_string()),
    };
    if let Some(why) = why_not_to_touch_it(&repo, base) {
        return left_alone(why);
    }
    let before = repo.head().ok().and_then(|head| head.target());
    // `--ff-only`, so a repository that has diverged is refused by git
    // rather than merged by Armada. `pull` and not `fetch` then `merge`:
    // one process, and the refusal comes back as the sentence git printed.
    let run = match run_in(in_repo, "git", &["pull", "--ff-only"]) {
        Ok(run) => run,
        Err(why) => return left_alone(why.to_string()),
    };
    if !run.status.success() {
        return left_alone(said(&run));
    }
    // **Read after the pull and reported, not inferred.** What `#474` runs its
    // Checks against is the tree that is here now, so the commit is the one
    // this repository's HEAD resolves to — never the merge commit the forge
    // named, which may be three commits back by the time anybody looks.
    let Some(after) = repo.head().ok().and_then(|head| head.target()) else {
        // A pull that succeeded over a HEAD nothing can read. Unreachable in
        // practice and reported as a refusal rather than as a standing with no
        // commit, because the alternative is a variant carrying an optional
        // one — and every caller would then have to ask a question that has no
        // answer it could act on.
        return left_alone(format!("git would not say what commit `{base}` is on"));
    };
    let moved = match before {
        Some(before) if before != after => repo
            .graph_ahead_behind(after, before)
            .map(|(ahead, _)| ahead)
            .unwrap_or(0),
        _ => 0,
    };
    match moved {
        0 => RepositoryStanding::AlreadyHadIt {
            base: base.to_string(),
            head: after.to_string(),
        },
        commits => RepositoryStanding::MovedOn {
            base: base.to_string(),
            commits,
            head: after.to_string(),
        },
    }
}

/// Why this repository is not Armada's to move, or `None` where it is.
///
/// **Three refusals and every one of them is a person's, not a fault.** A
/// checkout on some other branch is somebody mid-thought; uncommitted work is
/// somebody's unfinished change, and a fast-forward over it is exactly the
/// thing `bring_up_to_date` uses `--autostash` to avoid doing without asking.
fn why_not_to_touch_it(repo: &Repository, base: &str) -> Option<String> {
    let head = repo.head().ok()?;
    if !head.is_branch() {
        return Some(String::from("the repository is not on a branch"));
    }
    let on = head.shorthand().unwrap_or_default().to_string();
    if on != base {
        return Some(format!("the repository is on `{on}` and not `{base}`"));
    }
    let mut asking = git2::StatusOptions::new();
    asking.include_untracked(false).include_ignored(false);
    match repo.statuses(Some(&mut asking)) {
        Ok(statuses) if statuses.is_empty() => None,
        Ok(statuses) => Some(format!(
            "`{on}` is carrying {} uncommitted change(s)",
            statuses.len()
        )),
        Err(cause) => Some(cause.message().to_string()),
    }
}

/// Whether the forge is comparing this pull request against the commit its
/// branch was written on top of.
///
/// **Read locally, and that is the whole reason it costs nothing.** Every
/// worktree Armada cuts shares one object database with the repository they
/// were cut from, so the branch's own commits are here — and where the branch
/// and the base part company is a question git answers without a network.
///
/// **Silence rather than a guess.** A merge base this cannot compute is
/// [`Rendering::Unreadable`], never `AsWritten`, because the caller's answer to
/// a superseded base is to close and reopen a person's pull request.
pub(crate) fn rendering(in_repo: &str, base: &str, pinned: &str, head: &str) -> Rendering {
    let Ok(run) = run_in(in_repo, "git", &["merge-base", head, base]) else {
        return Rendering::Unreadable;
    };
    let written_on = last_line(&run);
    if !run.status.success() || written_on.is_empty() {
        return Rendering::Unreadable;
    }
    match written_on == pinned {
        true => Rendering::AsWritten,
        false => Rendering::FromASupersededBase {
            pinned: pinned.to_string(),
            written_on,
        },
    }
}

/// The `N` fields of one tab-separated line, or `None` where it is not `N`.
///
/// **A split and not a parse.** `--jq` did the reading on the forge's side, so
/// what arrives is one line of text and `store` and `ipc` stay the only two
/// crates that deserialise anything. A field the forge left blank makes the
/// whole reading `None`, because a pull request with no base branch is not a
/// thing this could act on.
pub(crate) fn fields<const N: usize>(said: &str) -> Option<[&str; N]> {
    let read: [&str; N] = said
        .split('\t')
        .map(str::trim)
        .collect::<Vec<&str>>()
        .try_into()
        .ok()?;
    read.iter().all(|field| !field.is_empty()).then_some(read)
}
