//! What became of a pull request after Armada opened it, and what the
//! repository it merges into should do about it.
//!
//! # A different moment from `crate::delivery`, which is why it is a different
//! file
//!
//! That module runs inside a Job's last turn, in the Job's own worktree, with
//! the work in hand. Everything here runs minutes or days later, from the
//! repository every worktree was cut from, about a pull request nobody on this
//! machine is holding — because Armada opens one and a person merges it.
//!
//! # One ask of the forge, answering two questions
//!
//! `#337` asked whether anybody merged it. `#427` asked whether the forge is
//! still comparing it against the right commit. Both are one `gh pr view`, and
//! `#427` says so itself: building them apart would be two processes where one
//! does.
//!
//! # Reading is free and writing is not
//!
//! [`read`] and [`caught_up`] are cheap and repeatable. [`rendered_afresh`]
//! closes and reopens a person's pull request, which is visible on the forge
//! and mails everybody watching it — so it is called once per pull request,
//! only for a base that is provably superseded, and never on a reading this
//! could not make.

use adapter_traits::{Landing, Rendering, Renewed, RepositoryStanding, WhatBecameOfIt};
use git2::Repository;

use crate::delivery::{asked, last_line, run_in, said, FORGE};

/// Ask the forge what became of one pull request, and what it is showing
/// beside it.
pub(crate) fn read(in_repo: &str, pull_request: &str) -> WhatBecameOfIt {
    // **Four fields in one call**, which is what `#427` asked for: what
    // became of it and what the forge is rendering beside it are the same
    // `pr view`, and two sweeps asking one thing each would be two
    // processes where one does. The address is not among them — it is the
    // argument, so asking for it back would be asking the forge to confirm
    // what was just handed to it.
    let Some(said) = asked(
        in_repo,
        pull_request,
        "state,baseRefName,baseRefOid,headRefOid",
        "[.state, .baseRefName, .baseRefOid, .headRefOid] | @tsv",
    ) else {
        return WhatBecameOfIt::unknown();
    };
    let Some([state, base, pinned, head]) = four(&said) else {
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
    let after = repo.head().ok().and_then(|head| head.target());
    let moved = match (before, after) {
        (Some(before), Some(after)) if before != after => repo
            .graph_ahead_behind(after, before)
            .map(|(ahead, _)| ahead)
            .unwrap_or(0),
        _ => 0,
    };
    match moved {
        0 => RepositoryStanding::AlreadyHadIt {
            base: base.to_string(),
        },
        commits => RepositoryStanding::MovedOn {
            base: base.to_string(),
            commits,
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

/// The four fields of one tab-separated line, or `None` where it is not four.
///
/// **A split and not a parse.** `--jq` did the reading on the forge's side, so
/// what arrives is one line of text and `store` and `ipc` stay the only two
/// crates that deserialise anything. A field the forge left blank makes the
/// whole reading `None`, because a pull request with no base branch is not a
/// thing this could act on.
pub(crate) fn four(said: &str) -> Option<[&str; 4]> {
    let fields: Vec<&str> = said.split('\t').map(str::trim).collect();
    let read: [&str; 4] = fields.try_into().ok()?;
    read.iter().all(|field| !field.is_empty()).then_some(read)
}
