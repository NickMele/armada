//! A finished Job's work, brought up to its base and put where a person
//! reviews it.
//!
//! # git's own rebase, not libgit2's
//!
//! libgit2 has a rebase API and it has no autostash, no conflict driver and no
//! `--abort` that restores what it started from; every one of those would be
//! reimplemented here, against the case where getting it wrong destroys a
//! Drone's uncommitted work. `git` is on the machine already — every Check in
//! this repository shells out to a toolchain — so the rebase is git's.
//!
//! # `--autostash`, which is what makes a mid-Job rebase safe at all
//!
//! Fleet commits only at the last step, so at every earlier boundary the branch
//! has no commits of its own and the worktree is full of uncommitted work. A
//! plain rebase over that either refuses or destroys it. `--autostash` puts the
//! work aside, moves the branch, and puts it back — and where putting it back
//! conflicts, git says so and keeps the stash, so nothing is lost either way.
//!
//! # Nothing here fetches
//!
//! The base is the local branch. A rebase that fetched first would put the
//! network, and a credential, in the middle of a step boundary — and the branch
//! a person merges into is the one on their machine.

use std::path::Path;
use std::process::{Command, Output};

use adapter_traits::{
    Base, BroughtUpToDate, Delivery, Landing, NotDelivered, Opened, Pushed, Review,
};
use adapter_traits::{Standing, Worktree};
use git2::{BranchType, Repository};

use crate::worktree::GitVcs;

/// The command that opens a pull request. **The one vendor name in this file**,
/// and the reason the whole of delivery lives in `adapters`.
const FORGE: &str = "gh";

impl Delivery for GitVcs {
    fn base(
        &self,
        worktree: &Worktree,
        declared: Option<&str>,
    ) -> Result<Option<Base>, NotDelivered> {
        crate::base::resolve(&open(worktree)?, declared)
    }

    fn standing(&self, worktree: &Worktree, base: &Base) -> Result<Standing, NotDelivered> {
        let repo = open(worktree)?;
        let tip = head_of(&repo, worktree)?;
        let base_tip = tip_of(&repo, base.name())?;
        match repo.graph_ahead_behind(tip, base_tip) {
            Ok((_, 0)) => Ok(Standing::UpToDate),
            Ok((_, behind)) => Ok(Standing::Behind { commits: behind }),
            Err(cause) => Err(NotDelivered::of(
                "comparing the branch with its base",
                cause.message(),
            )),
        }
    }

    fn bring_up_to_date(
        &self,
        worktree: &Worktree,
        base: &Base,
    ) -> Result<BroughtUpToDate, NotDelivered> {
        let behind = match self.standing(worktree, base)? {
            Standing::UpToDate => 0,
            Standing::Behind { commits } => commits,
        };
        let run = git(worktree, &["rebase", "--autostash", base.name()])?;
        // **Read whether it succeeded or not**, because a rebase can succeed
        // and still leave conflicts: git fast-forwards the branch, fails to put
        // the autostash back, says so and exits zero. Measured, not assumed.
        let files = unmerged_files(worktree);
        if run.status.success() {
            return Ok(match files.is_empty() {
                true => BroughtUpToDate::Clean {
                    base: base.name().to_string(),
                    commits: behind,
                },
                // The branch moved and the work came back with conflicts in it.
                // The stash git kept is left alone: it says the changes are safe
                // there, and dropping it would remove the only other copy.
                false => BroughtUpToDate::Conflicted {
                    base: base.name().to_string(),
                    files,
                },
            });
        }
        // Nobody can be handed half a rebase — a Drone has no git and a person
        // did not ask for one — so the branch goes back exactly as it was.
        if rebase_in_progress(worktree) {
            let _ = git(worktree, &["rebase", "--abort"]);
            return Ok(BroughtUpToDate::PutBack {
                base: base.name().to_string(),
                files,
            });
        }
        Err(NotDelivered::of("the rebase", said(&run)))
    }

    fn push(&self, worktree: &Worktree) -> Result<Pushed, NotDelivered> {
        let Some(remote) = a_remote(&open(worktree)?) else {
            return Ok(Pushed::NoRemote);
        };
        let branch = worktree.branch().to_string();
        let run = git(worktree, &["push", "--set-upstream", &remote, &branch])?;
        match run.status.success() {
            true => Ok(Pushed::ToTheRemote { remote, branch }),
            false => Err(NotDelivered::of("the push", said(&run))),
        }
    }

    fn open_for_review(
        &self,
        worktree: &Worktree,
        base: &Base,
        review: &Review,
    ) -> Result<Opened, NotDelivered> {
        // Asked before it is created, because creating a second one for a
        // branch is refused with a sentence rather than answered with the first.
        if let Some(url) = already_open(worktree) {
            return Ok(Opened::AlreadyOpen { url });
        }
        let run = run(
            worktree,
            FORGE,
            &[
                "pr",
                "create",
                "--base",
                base.name(),
                "--head",
                worktree.branch(),
                "--title",
                review.title(),
                "--body",
                review.body(),
            ],
        );
        let run = match run {
            Ok(run) => run,
            Err(why) => {
                return Ok(Opened::NoTool {
                    why: format!("`{FORGE}` would not run: {why}"),
                })
            }
        };
        match run.status.success() {
            true => Ok(Opened::PullRequest {
                url: last_line(&run),
            }),
            false => Err(NotDelivered::of("the pull request", said(&run))),
        }
    }

    fn landed(&self, in_repo: &str, pull_request: &str) -> Landing {
        // The state alone. The address is the argument, so asking for it back
        // would be asking the forge to confirm what was just handed to it.
        let Some(state) = asked(in_repo, pull_request, "state", ".state") else {
            return Landing::Unknown;
        };
        let url = pull_request.to_string();
        match state.as_str() {
            "MERGED" => Landing::Merged { url },
            "OPEN" => Landing::Open { url },
            "CLOSED" => Landing::ClosedUnmerged { url },
            // A word this forge has and Armada has not. Saying nothing is the
            // honest answer, and it costs one more ask later.
            _ => Landing::Unknown,
        }
    }
}

/// Ask the forge one thing about one pull request, named as the forge names
/// them — a branch, a number or an address, all of which `pr view` takes.
///
/// **Every failure is `None`**, which is the rule [`Landing`] and
/// [`already_open`] share: no tool, not signed in, no such pull request, a
/// network that would not answer. Nothing follows differently from any of them,
/// and where one is followed by a second call, that call is what says which it
/// was — putting the same fault on two lines is what this avoids.
///
/// **`--jq` and not a parse.** The forge does the reading, so bytes enter this
/// process as one line of text and `store` and `ipc` stay the only two places
/// that deserialise anything.
fn asked(in_dir: &str, named: &str, fields: &str, jq: &str) -> Option<String> {
    let run = run_in(
        in_dir,
        FORGE,
        &["pr", "view", named, "--json", fields, "--jq", jq],
    )
    .ok()?;
    run.status
        .success()
        .then(|| last_line(&run))
        .filter(|said| !said.is_empty())
}

/// The pull request already open for this branch, if there is one.
///
/// Every failure is `None` — see [`asked`], which is where that rule lives now
/// that two callers keep it.
fn already_open(worktree: &Worktree) -> Option<String> {
    asked(worktree.path(), worktree.branch(), "url", ".url")
}

/// The remote to push to: `origin` where there is one, otherwise the only other
/// name. **`None` is a repository with no remote**, which is ordinary.
///
/// Where several are configured and none is `origin`, nothing here can say
/// which one a person meant, so nothing is pushed.
fn a_remote(repo: &Repository) -> Option<String> {
    let remotes = repo.remotes().ok()?;
    let named: Vec<String> = remotes.iter().flatten().map(str::to_string).collect();
    match named.iter().any(|name| name == "origin") {
        true => Some(String::from("origin")),
        false => named.first().cloned(),
    }
}

/// Whether git is part-way through a rebase in this worktree.
///
/// Both directory names are asked for, because git uses one for the merge
/// backend and the other for the apply backend and which is in play depends on
/// the machine's configuration.
fn rebase_in_progress(worktree: &Worktree) -> bool {
    ["rebase-merge", "rebase-apply"].iter().any(|name| {
        match git(worktree, &["rev-parse", "--git-path", name]) {
            Ok(run) => {
                let path = last_line(&run);
                !path.is_empty() && Path::new(worktree.path()).join(path).exists()
            }
            Err(_) => false,
        }
    })
}

/// The paths git is holding as unresolved, one per line.
fn unmerged_files(worktree: &Worktree) -> Vec<String> {
    let Ok(run) = git(worktree, &["diff", "--name-only", "--diff-filter=U"]) else {
        return Vec::new();
    };
    String::from_utf8_lossy(&run.stdout)
        .lines()
        .map(str::trim)
        .filter(|line| !line.is_empty())
        .map(str::to_string)
        .collect()
}

fn open(worktree: &Worktree) -> Result<Repository, NotDelivered> {
    Repository::open(worktree.path()).map_err(|cause| {
        NotDelivered::of("opening the Job's worktree", cause.message().to_string())
    })
}

fn head_of(repo: &Repository, worktree: &Worktree) -> Result<git2::Oid, NotDelivered> {
    repo.head()
        .ok()
        .and_then(|head| head.target())
        .ok_or_else(|| {
            NotDelivered::of(
                "reading the Job's branch",
                format!("`{}` points at no commit", worktree.branch()),
            )
        })
}

fn tip_of(repo: &Repository, name: &str) -> Result<git2::Oid, NotDelivered> {
    repo.find_branch(name, BranchType::Local)
        .ok()
        .and_then(|found| found.get().target())
        .ok_or_else(|| {
            NotDelivered::of(
                "reading the base branch",
                format!("`{name}` points at no commit"),
            )
        })
}

/// Run `git` in the Job's worktree. **No shell**, like every other command
/// Armada runs, so nothing in an argument can be read as syntax.
fn git(worktree: &Worktree, args: &[&str]) -> Result<Output, NotDelivered> {
    run(worktree, "git", args).map_err(|why| NotDelivered::of("running git", why.to_string()))
}

fn run(worktree: &Worktree, program: &str, args: &[&str]) -> Result<Output, std::io::Error> {
    run_in(worktree.path(), program, args)
}

/// The same, in a directory that is not a Job's worktree. **The one caller is
/// the merge question**, which is asked long after the worktree it was written
/// in has been reclaimed.
fn run_in(dir: &str, program: &str, args: &[&str]) -> Result<Output, std::io::Error> {
    Command::new(program)
        .args(args)
        .current_dir(dir)
        // A tool that stops to ask has nobody to ask: Fleet is a daemon and
        // there is no terminal on the other end of this. Refusing is the
        // answer, and it comes back as the sentence the tool printed.
        .env("GIT_TERMINAL_PROMPT", "0")
        .output()
}

/// What a failed command said, both streams, for a person to read.
fn said(run: &Output) -> String {
    let err = String::from_utf8_lossy(&run.stderr);
    let out = String::from_utf8_lossy(&run.stdout);
    let joined = format!("{}\n{}", err.trim(), out.trim());
    joined.trim().to_string()
}

/// The last non-blank line of stdout. What a tool that prints an address prints
/// it as, after whatever progress it printed first.
fn last_line(run: &Output) -> String {
    String::from_utf8_lossy(&run.stdout)
        .lines()
        .map(str::trim)
        .rfind(|line| !line.is_empty())
        .unwrap_or("")
        .to_string()
}
