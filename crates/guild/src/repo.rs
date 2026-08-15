//! `~/.armada/guild/` **is** a git repository Armada manages (`PLAN.md` §13.5).
//!
//! Armada commits on change and pushes to a private remote named once during the
//! interview. This module is every git call that makes that true, and it makes
//! all of them through `ctx.run` — `ARCHITECTURE.md` §1.1's one subprocess seam,
//! chosen so that **argv is assertable**, because argv is where this class of
//! bug lives. A test here asserts `["git", "merge", "--ff-only", …]` and would
//! catch the day somebody drops the `--ff-only`.
//!
//! # Conflicts surface as conflicts, never a silent overwrite
//!
//! The rule the whole module is arranged around, and it is not a preference:
//! two machines' guilds merged automatically is how you end up with a hook you
//! did not write.
//!
//! | Situation | What happens |
//! |---|---|
//! | remote is ahead, local is not | **fast-forward**, and report what changed |
//! | histories have diverged | **stop, change nothing**, and report both counts and every file touched on both sides |
//! | local is ahead, remote is not | push |
//! | both, on `push` | refuse, and say to pull first |
//!
//! There is no automatic merge and no automatic rebase anywhere in this file,
//! and `--force` on `push` is accepted **only when the remote is strictly
//! behind** — which is to say, only when it discards nothing.

use armada_core::ctx::{Run, RunRequest, StdioMode};
use armada_core::error::{ArmadaError, ErrClass};
use std::path::Path;
use std::time::Duration;

/// Armada's own deadline on a git call.
///
/// **Every network-touching call needs one** for the same reason every docker
/// call does (`docs/traps.md`): git has no client-side timeout of its own, and
/// a `fetch` against an unreachable host hangs until TCP gives up — which on a
/// laptop that has just changed networks is minutes, on the verb whose whole
/// job is to tell you what is wrong.
const DEADLINE: Duration = Duration::from_secs(120);

/// The branch a fresh guild is created on.
///
/// **Named explicitly rather than inherited from `init.defaultBranch`.** The
/// guild is a repository Armada creates on every machine you own, and a machine
/// whose git is configured for `master` would produce a guild that cannot
/// fast-forward from one that was not.
pub const BRANCH: &str = "main";

/// The remote a guild pushes to. One, always, and named.
pub const REMOTE: &str = "origin";

/// How far apart two histories are.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub struct Divergence {
    /// Commits this machine has that the remote does not.
    pub ahead: usize,
    /// Commits the remote has that this machine does not.
    pub behind: usize,
}

impl Divergence {
    /// Both sides have commits the other does not. **The case that must never
    /// be resolved automatically.**
    pub fn diverged(self) -> bool {
        self.ahead > 0 && self.behind > 0
    }

    /// Nothing to do in either direction.
    pub fn in_step(self) -> bool {
        self.ahead == 0 && self.behind == 0
    }

    /// `local 2 ahead, 1 behind` — the wording `guild/push.md` prints.
    pub fn written(self) -> String {
        format!("local {} ahead, {} behind", self.ahead, self.behind)
    }
}

/// How one path differs between two commits.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Change {
    /// It is on the remote and not here.
    Added,
    /// It is on both and differs.
    Changed,
    /// It is here and not on the remote.
    Removed,
}

impl Change {
    /// The word the agreed layout puts in the STATUS column.
    pub fn word(self) -> &'static str {
        match self {
            Change::Added => "ADDED",
            Change::Changed => "CHANGED",
            Change::Removed => "REMOVED",
        }
    }
}

/// One path, and what happened to it.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Touched {
    /// Guild-relative.
    pub path: String,
    /// What kind of difference.
    pub change: Change,
}

/// Run one git command in the guild, and insist it succeeded.
fn git(run: &impl Run, cwd: &Path, args: &[&str]) -> Result<String, ArmadaError> {
    git_with(run, cwd, args, &[], None)
}

/// The same, with an environment layer and something on stdin.
fn git_with(
    run: &impl Run,
    cwd: &Path,
    args: &[&str],
    env: &[(&str, &str)],
    stdin: Option<&str>,
) -> Result<String, ArmadaError> {
    let output = try_git_with(run, cwd, args, env, stdin)?;
    if !output.0 {
        return Err(ArmadaError {
            class: ErrClass::ToolFailed,
            r#where: format!("git {}", args.join(" ")),
            message: first_line(&output.2).unwrap_or_else(|| "git failed".to_string()),
            next_action: None,
        });
    }
    Ok(output.1)
}

/// Run one git command and report whether it succeeded, rather than insisting.
///
/// Returns `(ok, stdout, stderr)`. For the calls whose failure is an answer:
/// `rev-parse` on a branch that does not exist yet, `diff --quiet` on a clean
/// tree.
fn try_git(
    run: &impl Run,
    cwd: &Path,
    args: &[&str],
) -> Result<(bool, String, String), ArmadaError> {
    try_git_with(run, cwd, args, &[], None)
}

/// The same, with an environment layer and something on stdin.
///
/// **Two callers and one reason each.** `GIT_INDEX_FILE` is what lets
/// [`commit_files`] build a commit without checking anything out — the guild's
/// working tree holds the user's edits and an upgrade must not disturb them —
/// and `hash-object --stdin` is what lets a template body become a blob without
/// ever being written to a file somebody could be editing at the same moment.
fn try_git_with(
    run: &impl Run,
    cwd: &Path,
    args: &[&str],
    env: &[(&str, &str)],
    stdin: Option<&str>,
) -> Result<(bool, String, String), ArmadaError> {
    let mut argv = vec!["git".to_string()];
    argv.extend(args.iter().map(|a| (*a).to_string()));
    let mut request = RunRequest::new(argv, cwd.to_path_buf())
        .stdio(StdioMode::Capture)
        .timeout(DEADLINE);
    for (key, value) in env {
        request.env.insert((*key).to_string(), (*value).to_string());
    }
    request.stdin = stdin.map(str::to_string);

    match run.call(&request) {
        Ok(output) if output.timed_out => Err(ArmadaError {
            class: ErrClass::Timeout,
            r#where: format!("git {}", args.join(" ")),
            message: format!("git did not finish within {}s", DEADLINE.as_secs()),
            next_action: Some("check the network, then retry unchanged".to_string()),
        }),
        Ok(output) => Ok((output.ok(), output.stdout, output.stderr)),
        // `git` absent is the machine being broken, never the repository —
        // `environment`, whose documented response is "fix the machine, then
        // retry unchanged" (`ARCHITECTURE.md` §1.7).
        Err(spawn) => Err(ArmadaError {
            class: ErrClass::Environment,
            r#where: "git".to_string(),
            message: format!("cannot run git: {}", spawn.message),
            next_action: Some("install git, then retry unchanged".to_string()),
        }),
    }
}

fn first_line(text: &str) -> Option<String> {
    text.lines()
        .find(|l| !l.trim().is_empty())
        .map(str::trim)
        .map(str::to_string)
}

/// Turn `~/.armada/guild/` into a repository with one commit in it.
///
/// **The initial commit is not optional.** A repository with no commits has no
/// `HEAD`, so it cannot be compared with a remote, cannot be fast-forwarded and
/// cannot be pushed — every verb below would have a first-run special case.
pub fn init(run: &impl Run, guild: &Path, message: &str) -> Result<(), ArmadaError> {
    git(run, guild, &["init", "-b", BRANCH])?;
    commit_all(run, guild, message)?;
    Ok(())
}

/// Commit whatever is uncommitted, and say whether there was anything.
///
/// `guild push` calls this first, so **an edit made outside `armada guild edit`
/// is not left behind** — an uncommitted guild edit is the first half of the
/// drift failure `PHASES.md` §11 names.
pub fn commit_all(run: &impl Run, guild: &Path, message: &str) -> Result<bool, ArmadaError> {
    git(run, guild, &["add", "-A"])?;
    // `diff --cached --quiet` exits non-zero when there *is* something staged,
    // which is the question being asked.
    let (clean, _, _) = try_git(run, guild, &["diff", "--cached", "--quiet"])?;
    if clean {
        return Ok(false);
    }
    git(run, guild, &["commit", "-m", message])?;
    Ok(true)
}

/// The configured remote's URL, or `None` when sync is off.
pub fn remote(run: &impl Run, guild: &Path) -> Result<Option<String>, ArmadaError> {
    let (ok, stdout, _) = try_git(run, guild, &["remote", "get-url", REMOTE])?;
    Ok(ok
        .then(|| stdout.trim().to_string())
        .filter(|u| !u.is_empty()))
}

/// Point the guild at a remote, replacing whatever was there.
pub fn set_remote(run: &impl Run, guild: &Path, url: &str) -> Result<(), ArmadaError> {
    if remote(run, guild)?.is_some() {
        git(run, guild, &["remote", "set-url", REMOTE, url])?;
    } else {
        git(run, guild, &["remote", "add", REMOTE, url])?;
    }
    Ok(())
}

/// Clone a guild from a remote — the second-machine path, done in seconds.
pub fn clone(run: &impl Run, url: &str, guild: &Path) -> Result<(), ArmadaError> {
    let parent = guild.parent().unwrap_or(Path::new("."));
    let name = guild
        .file_name()
        .and_then(|n| n.to_str())
        .unwrap_or("guild");
    git(run, parent, &["clone", url, name])?;
    Ok(())
}

/// Fetch, then measure the distance in both directions.
///
/// **A remote with no `main` on it is not a failure**, and getting that wrong
/// broke the one invocation every guild makes exactly once: the *first* push to
/// a freshly created private repository, where `git fetch origin main` answers
/// `couldn't find remote ref main`. An empty remote is zero commits behind, not
/// an error — the distinction between "the remote has nothing" and "the remote
/// is unreachable" is the whole of it, and only the second is worth reporting.
pub fn fetch(run: &impl Run, guild: &Path) -> Result<Divergence, ArmadaError> {
    let (ok, _, stderr) = try_git(run, guild, &["fetch", REMOTE, BRANCH])?;
    if !ok {
        if empty_remote(&stderr) {
            // **Every local commit is ahead of nothing**, and saying so is the
            // difference between a first push that reports `pushed 1 commit`
            // and one that reports `already in step` while pushing a commit.
            return Ok(Divergence {
                ahead: commits(run, guild)?,
                behind: 0,
            });
        }
        return Err(ArmadaError {
            class: ErrClass::ToolFailed,
            r#where: format!("git fetch {REMOTE} {BRANCH}"),
            message: first_line(&stderr).unwrap_or_else(|| "git fetch failed".to_string()),
            next_action: Some("check the remote is reachable, then retry unchanged".to_string()),
        });
    }
    divergence(run, guild)
}

/// How many commits this guild has, for the case where there is nothing to
/// compare against.
fn commits(run: &impl Run, guild: &Path) -> Result<usize, ArmadaError> {
    let (ok, stdout, _) = try_git(run, guild, &["rev-list", "--count", "HEAD"])?;
    Ok(ok
        .then(|| stdout.trim().parse().ok())
        .flatten()
        .unwrap_or(0))
}

/// Whether a failed fetch means the remote simply has nothing on this branch.
///
/// **Matched on git's own words, and that is a knowing trade.** There is no
/// exit code for it — every fetch failure is `1` — so the alternative is a
/// `git ls-remote` before every fetch, which is a second network round trip on
/// the verb that runs most. A wording change would make a first push report a
/// real error instead of succeeding, which is loud rather than silent.
fn empty_remote(stderr: &str) -> bool {
    let lowered = stderr.to_ascii_lowercase();
    lowered.contains("couldn't find remote ref")
        || lowered.contains("does not appear to be a git") && lowered.contains("empty")
}

/// How far apart local and `origin/main` are, without fetching.
///
/// **`--count --left-right` in one call, not two `rev-list`s.** Two calls can
/// straddle a concurrent fetch and report a pair that never simultaneously
/// held — and the pair is the thing the divergence decision is made on.
pub fn divergence(run: &impl Run, guild: &Path) -> Result<Divergence, ArmadaError> {
    let range = format!("{REMOTE}/{BRANCH}...HEAD");
    let (ok, stdout, _) = try_git(run, guild, &["rev-list", "--left-right", "--count", &range])?;
    if !ok {
        // No remote-tracking branch yet: never fetched, or nothing pushed. That
        // is not a failure, it is a guild that has not synced.
        return Ok(Divergence::default());
    }
    let mut counts = stdout.split_whitespace();
    Ok(Divergence {
        behind: counts.next().and_then(|n| n.parse().ok()).unwrap_or(0),
        ahead: counts.next().and_then(|n| n.parse().ok()).unwrap_or(0),
    })
}

/// Every path that differs between `HEAD` and `origin/main`.
pub fn incoming(run: &impl Run, guild: &Path) -> Result<Vec<Touched>, ArmadaError> {
    let range = format!("HEAD..{REMOTE}/{BRANCH}");
    let (ok, stdout, _) = try_git(run, guild, &["diff", "--name-status", &range])?;
    if !ok {
        return Ok(Vec::new());
    }
    Ok(parse_name_status(&stdout))
}

/// Every path this machine has changed that the remote has not seen.
pub fn outgoing(run: &impl Run, guild: &Path) -> Result<Vec<Touched>, ArmadaError> {
    let range = format!("{REMOTE}/{BRANCH}..HEAD");
    let (ok, stdout, _) = try_git(run, guild, &["diff", "--name-status", &range])?;
    if !ok {
        return Ok(Vec::new());
    }
    Ok(parse_name_status(&stdout))
}

/// `git diff --name-status`, which is one status letter, a tab, and a path.
///
/// **`A` is read as [`Change::Added`] from the reader's point of view, not
/// git's.** In `HEAD..origin/main` a path git calls added is one the remote has
/// and this machine does not, which is exactly what a person pulling wants the
/// word to mean.
fn parse_name_status(text: &str) -> Vec<Touched> {
    text.lines()
        .filter_map(|line| {
            let (status, path) = line.split_once('\t')?;
            let change = match status.chars().next()? {
                'A' => Change::Added,
                'D' => Change::Removed,
                _ => Change::Changed,
            };
            Some(Touched {
                path: path.trim().to_string(),
                change,
            })
        })
        .collect()
}

/// Fast-forward to the remote. **Refuses anything that is not a fast-forward.**
pub fn fast_forward(run: &impl Run, guild: &Path) -> Result<(), ArmadaError> {
    git(
        run,
        guild,
        &["merge", "--ff-only", &format!("{REMOTE}/{BRANCH}")],
    )?;
    Ok(())
}

/// Push. `force` is the caller's decision and is refused upstream of here
/// unless the remote is strictly behind.
pub fn push(run: &impl Run, guild: &Path, force: bool) -> Result<(), ArmadaError> {
    let mut args = vec!["push"];
    if force {
        // `--force-with-lease`, never bare `--force`. The lease is what makes
        // "the remote is strictly behind" still true at the moment of the push
        // rather than at the moment it was checked.
        args.push("--force-with-lease");
    }
    args.extend(["--set-upstream", REMOTE, BRANCH]);
    git(run, guild, &args)?;
    Ok(())
}

// ------------------------------------- the upstream branch, for `guild upgrade`
//
// **Everything below builds a commit without touching the working tree**, and
// that is the constraint the shape follows from. `~/.armada/guild` holds the
// user's edits and may hold work in progress; an upgrade that checked a branch
// out to write four files onto it would be an upgrade that could lose them if
// it were interrupted. `git read-tree` into a private index, `hash-object` from
// stdin, `write-tree`, `commit-tree`, `update-ref` — five plumbing calls, none
// of which reads or writes a single file in the worktree.
//
// The one command that does touch the worktree is [`merge`], which is the whole
// point: it is the only place the user's files and Armada's meet, and git
// arbitrates it rather than Armada.

/// Whether a ref resolves, and to what.
///
/// `None` for a branch that does not exist, which is the ordinary state of
/// [`crate::upstream::BRANCH`] on a guild that has never been upgraded.
pub fn rev(run: &impl Run, guild: &Path, reference: &str) -> Result<Option<String>, ArmadaError> {
    let (ok, stdout, _) = try_git(run, guild, &["rev-parse", "--verify", "--quiet", reference])?;
    Ok(ok
        .then(|| stdout.trim().to_string())
        .filter(|sha| !sha.is_empty()))
}

/// `git log --format=%H %s`, newest first — the history
/// [`crate::upstream::base_of`] reads to find a pre-provenance guild's base.
///
/// **The whole log, and the choosing happens in a pure function.** Asking git to
/// `--grep` for the init subjects would put the decision in an argv, where the
/// only test possible is that the argv was built; here the decision is a
/// function over a string and the tests are about histories.
pub fn subjects(run: &impl Run, guild: &Path) -> Result<String, ArmadaError> {
    git(run, guild, &["log", "--format=%H %s"])
}

/// Point a branch at a commit, creating it or moving it.
pub fn set_branch(
    run: &impl Run,
    guild: &Path,
    branch: &str,
    at: &str,
) -> Result<(), ArmadaError> {
    git(
        run,
        guild,
        &["update-ref", &format!("refs/heads/{branch}"), at],
    )?;
    Ok(())
}

/// Whether `ancestor` is reachable from `descendant`.
///
/// Used to decide whether the remote's copy of the upstream branch supersedes
/// this machine's. It is a question with three answers — yes, no, and "one of
/// them does not exist" — and the third is `false`, because a comparison
/// against something that is not there cannot be a fast-forward.
pub fn is_ancestor(
    run: &impl Run,
    guild: &Path,
    ancestor: &str,
    descendant: &str,
) -> Result<bool, ArmadaError> {
    let (ok, _, _) = try_git(
        run,
        guild,
        &["merge-base", "--is-ancestor", ancestor, descendant],
    )?;
    Ok(ok)
}

/// A commit holding `parent`'s tree with these paths written over it, built in
/// a private index so the working tree is never touched.
///
/// **Nothing is ever removed.** The new tree is the parent's with four paths
/// replaced, so the diff a merge sees is exactly the managed files and nothing
/// else — which is what makes "never overwrite something the user wrote" a
/// property of the construction rather than of a check.
///
/// Returns `None` when the result would be identical to the parent, because a
/// commit with an empty diff is a commit that makes every later `merge-base`
/// answer harder to read for no gain.
pub fn commit_files(
    run: &impl Run,
    guild: &Path,
    parent: &str,
    files: &[(&str, String)],
    message: &str,
) -> Result<Option<String>, ArmadaError> {
    // Inside `.git/`, so it is never mistaken for content and never syncs.
    let index = guild.join(".git").join("armada-upgrade-index");
    let index_path = index.display().to_string();
    let env = [("GIT_INDEX_FILE", index_path.as_str())];
    // A stale index from an interrupted run would be read as the starting
    // point, which is the one way this could carry a file nobody asked for.
    let _ = std::fs::remove_file(&index);

    let built = build(run, guild, parent, files, message, &env);
    // The private index is scratch, and leaving it behind would leave a `.git`
    // entry a reader has to wonder about.
    let _ = std::fs::remove_file(&index);
    built
}

fn build(
    run: &impl Run,
    guild: &Path,
    parent: &str,
    files: &[(&str, String)],
    message: &str,
    env: &[(&str, &str)],
) -> Result<Option<String>, ArmadaError> {
    git_with(run, guild, &["read-tree", parent], env, None)?;
    for (path, body) in files {
        // `--path` so git applies the same filters it would for a checkout of
        // that path; `--stdin` so a template body never lands on disk beside a
        // file the reader may be editing.
        let blob = git_with(
            run,
            guild,
            &["hash-object", "-w", "--path", path, "--stdin"],
            env,
            Some(body),
        )?;
        let entry = format!("100644,{},{}", blob.trim(), path);
        git_with(
            run,
            guild,
            &["update-index", "--add", "--cacheinfo", &entry],
            env,
            None,
        )?;
    }
    let tree = git_with(run, guild, &["write-tree"], env, None)?;
    let tree = tree.trim().to_string();

    let parent_tree = git(run, guild, &["rev-parse", &format!("{parent}^{{tree}}")])?;
    if parent_tree.trim() == tree {
        return Ok(None);
    }

    let commit = git(
        run,
        guild,
        &["commit-tree", &tree, "-p", parent, "-m", message],
    )?;
    Ok(Some(commit.trim().to_string()))
}

/// What a merge did.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Merged {
    /// It landed, and the guild is committed.
    Clean,
    /// Both sides changed the same lines. **The merge is left in progress**,
    /// with git's markers in the files it names, for the person to resolve —
    /// which is the one outcome an upgrade may not decide for them.
    Conflicted(Vec<String>),
}

/// Merge the upstream branch into whatever is checked out.
///
/// **`--no-ff`, so the merge is a commit and is legible.** A fast-forward would
/// move `main` onto Armada's branch and leave a history in which the user's
/// guild appears to *be* the template set. `--no-commit` is deliberately not
/// used: on a clean merge the guild should end committed, because a guild that
/// is not committed is one `guild push` will not carry.
pub fn merge(
    run: &impl Run,
    guild: &Path,
    branch: &str,
    message: &str,
) -> Result<Merged, ArmadaError> {
    let (ok, _, stderr) = try_git(run, guild, &["merge", "--no-ff", "-m", message, branch])?;
    if ok {
        return Ok(Merged::Clean);
    }
    let conflicts = unmerged(run, guild)?;
    if conflicts.is_empty() {
        // A merge that failed for a reason that is not a conflict — an
        // in-progress merge, an unborn branch — is the repository being in a
        // state Armada did not put it in, and guessing is worse than saying so.
        return Err(ArmadaError {
            class: ErrClass::ToolFailed,
            r#where: format!("git merge {branch}"),
            message: first_line(&stderr).unwrap_or_else(|| "git merge failed".to_string()),
            next_action: Some(
                "resolve what is in progress in ~/.armada/guild, then retry".to_string(),
            ),
        });
    }
    Ok(Merged::Conflicted(conflicts))
}

/// The paths a merge could not resolve.
pub fn unmerged(run: &impl Run, guild: &Path) -> Result<Vec<String>, ArmadaError> {
    let (ok, stdout, _) = try_git(run, guild, &["diff", "--name-only", "--diff-filter=U"])?;
    if !ok {
        return Ok(Vec::new());
    }
    Ok(stdout
        .lines()
        .map(str::trim)
        .filter(|line| !line.is_empty())
        .map(str::to_string)
        .collect())
}

/// Every path that differs between two commits.
pub fn changed(
    run: &impl Run,
    guild: &Path,
    from: &str,
    to: &str,
) -> Result<Vec<Touched>, ArmadaError> {
    let (ok, stdout, _) = try_git(
        run,
        guild,
        &["diff", "--name-status", &format!("{from}..{to}")],
    )?;
    if !ok {
        return Ok(Vec::new());
    }
    Ok(parse_name_status(&stdout))
}

/// Fetch one Armada-owned branch into its remote-tracking ref, and say whether
/// the remote has it.
///
/// **Forced, and that is safe because the branch is Armada's.** The refspec
/// writes `refs/remotes/origin/<branch>`, which nothing but this module reads;
/// the user's `main` is never a target here.
pub fn fetch_branch(run: &impl Run, guild: &Path, branch: &str) -> Result<bool, ArmadaError> {
    let refspec = format!("+refs/heads/{branch}:refs/remotes/{REMOTE}/{branch}");
    let (ok, _, stderr) = try_git(run, guild, &["fetch", REMOTE, &refspec])?;
    if ok {
        return Ok(true);
    }
    if empty_remote(&stderr) {
        return Ok(false);
    }
    Err(ArmadaError {
        class: ErrClass::ToolFailed,
        r#where: format!("git fetch {REMOTE} {branch}"),
        message: first_line(&stderr).unwrap_or_else(|| "git fetch failed".to_string()),
        next_action: Some("check the remote is reachable, then retry unchanged".to_string()),
    })
}

/// Push one Armada-owned branch, and say whether it went.
///
/// **A rejection is an answer, not an error.** The branch carries nothing of
/// the user's — it is regenerable from the templates — so a machine that could
/// not publish it has still upgraded correctly, and the only cost is that the
/// next machine adopts its base from history instead. The caller reports it on
/// a row rather than failing a verb that did what it was asked.
pub fn push_branch(run: &impl Run, guild: &Path, branch: &str) -> Result<bool, ArmadaError> {
    let refspec = format!("refs/heads/{branch}:refs/heads/{branch}");
    let (ok, _, _) = try_git(run, guild, &["push", REMOTE, &refspec])?;
    Ok(ok)
}

#[cfg(test)]
mod tests {
    use super::*;
    use armada_core::ctx::{RunOutput, SpawnError, SpawnErrorKind};
    use std::cell::RefCell;

    /// A fake that records argv and answers from a script.
    #[derive(Default)]
    struct FakeRun {
        calls: RefCell<Vec<Vec<String>>>,
        answers: RefCell<Vec<(bool, String)>>,
        missing: bool,
    }

    impl FakeRun {
        fn answering(answers: &[(bool, &str)]) -> FakeRun {
            FakeRun {
                answers: RefCell::new(
                    answers
                        .iter()
                        .map(|(ok, out)| (*ok, (*out).to_string()))
                        .collect(),
                ),
                ..FakeRun::default()
            }
        }

        fn argv(&self, index: usize) -> Vec<String> {
            self.calls.borrow()[index].clone()
        }
    }

    impl Run for FakeRun {
        fn call(&self, request: &RunRequest) -> Result<RunOutput, SpawnError> {
            if self.missing {
                return Err(SpawnError {
                    program: "git".to_string(),
                    kind: SpawnErrorKind::NotFound,
                    message: "No such file or directory".to_string(),
                });
            }
            self.calls.borrow_mut().push(request.argv.clone());
            let mut answers = self.answers.borrow_mut();
            let (ok, stdout) = if answers.is_empty() {
                (true, String::new())
            } else {
                answers.remove(0)
            };
            Ok(RunOutput {
                code: Some(if ok { 0 } else { 1 }),
                signal: None,
                stdout,
                stderr: String::new(),
                timed_out: false,
            })
        }
    }

    fn guild() -> &'static Path {
        Path::new("/scratch/.armada/guild")
    }

    /// **The branch is named, not inherited.** A machine configured for
    /// `master` would otherwise produce a guild that cannot fast-forward from
    /// one that was not.
    #[test]
    fn a_fresh_guild_is_initialised_on_a_named_branch_with_one_commit() {
        let run = FakeRun::answering(&[(true, ""), (true, ""), (false, ""), (true, "")]);
        init(&run, guild(), "guild: initial import").unwrap();

        assert_eq!(run.argv(0), ["git", "init", "-b", "main"]);
        assert_eq!(run.argv(1), ["git", "add", "-A"]);
        assert_eq!(
            run.argv(3),
            ["git", "commit", "-m", "guild: initial import"]
        );
    }

    /// Nothing staged is not a commit, and not an error either.
    #[test]
    fn committing_a_clean_tree_makes_no_commit() {
        let run = FakeRun::answering(&[(true, ""), (true, "")]);
        assert!(!commit_all(&run, guild(), "guild: nothing").unwrap());
        assert_eq!(run.calls.borrow().len(), 2, "a commit was made anyway");
    }

    /// **The rule the module exists for**, asserted on argv: a pull is a
    /// fast-forward or it is nothing.
    #[test]
    fn a_pull_is_a_fast_forward_and_never_a_merge() {
        let run = FakeRun::default();
        fast_forward(&run, guild()).unwrap();
        assert_eq!(
            run.argv(0),
            ["git", "merge", "--ff-only", "origin/main"],
            "without --ff-only this silently merges two machines' guilds"
        );
    }

    /// A forced push uses a lease, so "the remote is strictly behind" is still
    /// true at the moment of the push rather than at the moment it was checked.
    #[test]
    fn a_forced_push_is_leased_rather_than_bare() {
        let run = FakeRun::default();
        push(&run, guild(), true).unwrap();
        assert!(run.argv(0).contains(&"--force-with-lease".to_string()));
        assert!(
            !run.argv(0).iter().any(|a| a == "--force"),
            "a bare --force discards another machine's commits"
        );

        let plain = FakeRun::default();
        push(&plain, guild(), false).unwrap();
        assert_eq!(
            plain.argv(0),
            ["git", "push", "--set-upstream", "origin", "main"]
        );
    }

    /// Both counts from one call, because two calls can straddle a fetch and
    /// report a pair that never simultaneously held.
    #[test]
    fn divergence_reads_both_counts_from_one_call() {
        let run = FakeRun::answering(&[(true, "3\t2\n")]);
        let apart = divergence(&run, guild()).unwrap();
        assert_eq!(
            run.argv(0),
            [
                "git",
                "rev-list",
                "--left-right",
                "--count",
                "origin/main...HEAD"
            ]
        );
        assert_eq!(apart.behind, 3);
        assert_eq!(apart.ahead, 2);
        assert!(apart.diverged());
        assert_eq!(apart.written(), "local 2 ahead, 3 behind");
    }

    /// A guild that has never synced is not diverged from anything.
    #[test]
    fn a_guild_with_no_remote_tracking_branch_is_in_step_rather_than_broken() {
        let run = FakeRun::answering(&[(false, "")]);
        let apart = divergence(&run, guild()).unwrap();
        assert!(apart.in_step());
        assert!(!apart.diverged());
    }

    /// **The first push to a fresh private repository**, which every guild
    /// makes exactly once and which an earlier version of this function broke:
    /// `git fetch origin main` against an empty remote says `couldn't find
    /// remote ref main`, and that is zero commits behind rather than an error.
    #[test]
    fn a_remote_with_nothing_on_it_is_in_step_rather_than_a_failure() {
        struct EmptyRemote;
        impl Run for EmptyRemote {
            fn call(&self, request: &RunRequest) -> Result<RunOutput, SpawnError> {
                let fetching = request.argv.contains(&"fetch".to_string());
                Ok(RunOutput {
                    code: Some(if fetching { 1 } else { 0 }),
                    signal: None,
                    stdout: String::new(),
                    stderr: if fetching {
                        "fatal: couldn't find remote ref main\n".to_string()
                    } else {
                        String::new()
                    },
                    timed_out: false,
                })
            }
        }
        assert!(fetch(&EmptyRemote, guild()).unwrap().in_step());
    }

    /// **A remote that cannot be reached still fails**, which is the whole
    /// point of the distinction: only one of the two is worth reporting.
    #[test]
    fn a_remote_that_cannot_be_reached_is_still_a_failure() {
        struct Unreachable;
        impl Run for Unreachable {
            fn call(&self, _: &RunRequest) -> Result<RunOutput, SpawnError> {
                Ok(RunOutput {
                    code: Some(128),
                    signal: None,
                    stdout: String::new(),
                    stderr: "fatal: Could not read from remote repository.\n".to_string(),
                    timed_out: false,
                })
            }
        }
        let error = fetch(&Unreachable, guild()).unwrap_err();
        assert_eq!(error.class, ErrClass::ToolFailed);
        assert!(error.message.contains("Could not read"), "{error:?}");
    }

    /// The words in the STATUS column, read from git's own status letters.
    #[test]
    fn a_name_status_diff_reads_as_the_words_the_layout_prints() {
        let touched = parse_name_status(
            "A\tskills/add-migration/SKILL.md\nM\thooks/stop-notify.sh\nD\tvoice.md\n",
        );
        assert_eq!(
            touched,
            vec![
                Touched {
                    path: "skills/add-migration/SKILL.md".to_string(),
                    change: Change::Added
                },
                Touched {
                    path: "hooks/stop-notify.sh".to_string(),
                    change: Change::Changed
                },
                Touched {
                    path: "voice.md".to_string(),
                    change: Change::Removed
                },
            ]
        );
        // SCREAMING, like every other word a STATUS column holds.
        assert_eq!(Change::Added.word(), "ADDED");
    }

    /// `git` missing is the **machine** being broken, whose documented response
    /// is "fix the machine, then retry unchanged" — not the repository's fault
    /// and not a `tool_failed` a caller would report as a real result.
    #[test]
    fn git_missing_from_path_is_environment_rather_than_tool_failed() {
        let run = FakeRun {
            missing: true,
            ..FakeRun::default()
        };
        let error = divergence(&run, guild()).unwrap_err();
        assert_eq!(error.class, ErrClass::Environment);
        assert_eq!(error.class.exit_code(), 6);
        assert!(error.next_action.is_some());
    }

    /// Every call carries Armada's own deadline: git has none of its own, and a
    /// fetch against an unreachable host otherwise hangs for minutes.
    #[test]
    fn every_git_call_carries_a_deadline() {
        struct Deadlines(RefCell<Vec<Option<Duration>>>);
        impl Run for Deadlines {
            fn call(&self, request: &RunRequest) -> Result<RunOutput, SpawnError> {
                self.0.borrow_mut().push(request.timeout);
                Ok(RunOutput {
                    code: Some(0),
                    signal: None,
                    stdout: String::new(),
                    stderr: String::new(),
                    timed_out: false,
                })
            }
        }
        let run = Deadlines(RefCell::new(Vec::new()));
        fetch(&run, guild()).unwrap();
        assert!(!run.0.borrow().is_empty());
        for deadline in run.0.borrow().iter() {
            assert_eq!(*deadline, Some(DEADLINE));
        }
    }

    /// A deadline that elapses is `timeout`, whose documented response is
    /// "raise it, or investigate why it is slow" — distinct from a network the
    /// machine does not have.
    #[test]
    fn a_git_call_that_times_out_is_a_timeout() {
        struct TimesOut;
        impl Run for TimesOut {
            fn call(&self, _: &RunRequest) -> Result<RunOutput, SpawnError> {
                Ok(RunOutput {
                    code: None,
                    signal: None,
                    stdout: String::new(),
                    stderr: String::new(),
                    timed_out: true,
                })
            }
        }
        let error = divergence(&TimesOut, guild()).unwrap_err();
        assert_eq!(error.class, ErrClass::Timeout);
        assert_eq!(error.class.exit_code(), 4);
    }

    // ------------------------------------------------ the upstream branch

    /// **The upgrade's commit is built without checking anything out**, and
    /// this is that stated as argv. A `git checkout` anywhere in this sequence
    /// would put the user's working tree — which holds their edits, and may
    /// hold work in progress — at the mercy of an interrupted upgrade.
    #[test]
    fn an_upstream_commit_is_built_in_a_private_index_and_never_checked_out() {
        let run = FakeRun::answering(&[
            (true, ""),         // read-tree
            (true, "b10b\n"),   // hash-object
            (true, ""),         // update-index
            (true, "7433\n"),   // write-tree
            (true, "01dtree\n"), // rev-parse parent^{tree}
            (true, "c0mm17\n"), // commit-tree
        ]);
        let made = commit_files(
            &run,
            guild(),
            "armada",
            &[("subagents/helm.md", "# Helm\n".to_string())],
            "guild: armada templates abc123",
        )
        .unwrap();
        assert_eq!(made.unwrap(), "c0mm17");

        assert_eq!(run.argv(0), ["git", "read-tree", "armada"]);
        assert_eq!(
            run.argv(1),
            [
                "git",
                "hash-object",
                "-w",
                "--path",
                "subagents/helm.md",
                "--stdin"
            ],
            "a template body written to disk is one a reader could be editing"
        );
        assert_eq!(
            run.argv(2),
            [
                "git",
                "update-index",
                "--add",
                "--cacheinfo",
                "100644,b10b,subagents/helm.md"
            ]
        );
        assert_eq!(run.argv(3), ["git", "write-tree"]);
        assert_eq!(
            run.argv(5),
            [
                "git",
                "commit-tree",
                "7433",
                "-p",
                "armada",
                "-m",
                "guild: armada templates abc123"
            ]
        );
        assert!(
            !run.calls
                .borrow()
                .iter()
                .any(|argv| argv.contains(&"checkout".to_string())
                    || argv.contains(&"switch".to_string())
                    || argv.contains(&"stash".to_string())),
            "the working tree was touched: {:?}",
            run.calls.borrow()
        );
    }

    /// A tree identical to its parent's makes no commit. An empty-diff commit
    /// on the upstream branch is one every later `merge-base` has to be read
    /// past, for nothing.
    #[test]
    fn an_unchanged_template_set_makes_no_commit() {
        let run = FakeRun::answering(&[
            (true, ""),
            (true, "b10b\n"),
            (true, ""),
            (true, "5ame\n"),
            (true, "5ame\n"),
        ]);
        let made = commit_files(
            &run,
            guild(),
            "armada",
            &[("subagents/helm.md", "# Helm\n".to_string())],
            "guild: armada templates abc123",
        )
        .unwrap();
        assert!(made.is_none());
        assert!(
            !run.calls
                .borrow()
                .iter()
                .any(|argv| argv.contains(&"commit-tree".to_string()))
        );
    }

    /// **`--no-ff`, asserted.** A fast-forward would move the guild's branch
    /// onto Armada's and leave a history in which the person's guild appears to
    /// *be* the template set.
    #[test]
    fn a_merge_is_never_a_fast_forward() {
        let run = FakeRun::default();
        assert_eq!(
            merge(&run, guild(), "armada", "guild: upgrade").unwrap(),
            Merged::Clean
        );
        assert_eq!(
            run.argv(0),
            ["git", "merge", "--no-ff", "-m", "guild: upgrade", "armada"]
        );
    }

    /// A conflicted merge is **left in progress and reported by name**. The one
    /// outcome an upgrade may not decide is which of two people's words wins.
    #[test]
    fn a_conflicted_merge_is_left_alone_and_names_its_files() {
        let run = FakeRun::answering(&[(false, ""), (true, "subagents/helm.md\n")]);
        let outcome = merge(&run, guild(), "armada", "guild: upgrade").unwrap();
        assert_eq!(
            outcome,
            Merged::Conflicted(vec!["subagents/helm.md".to_string()])
        );
        assert!(
            !run.calls
                .borrow()
                .iter()
                .any(|argv| argv.contains(&"--abort".to_string())
                    || argv.contains(&"reset".to_string())),
            "an abort would throw away the resolution the person is about to make"
        );
    }

    /// A merge that failed for something other than a conflict is not silently
    /// reported as one — there is nothing to resolve, and the message is git's.
    #[test]
    fn a_merge_that_failed_for_another_reason_is_a_failure() {
        struct Broken;
        impl Run for Broken {
            fn call(&self, request: &RunRequest) -> Result<RunOutput, SpawnError> {
                let merging = request.argv.contains(&"merge".to_string());
                Ok(RunOutput {
                    code: Some(if merging { 128 } else { 0 }),
                    signal: None,
                    stdout: String::new(),
                    stderr: if merging {
                        "fatal: You have not concluded your merge\n".to_string()
                    } else {
                        String::new()
                    },
                    timed_out: false,
                })
            }
        }
        let error = merge(&Broken, guild(), "armada", "guild: upgrade").unwrap_err();
        assert_eq!(error.class, ErrClass::ToolFailed);
        assert!(error.message.contains("not concluded"), "{error:?}");
    }

    /// The upstream branch is fetched into its **own** remote-tracking ref, so
    /// a forced refspec can never reach the branch the person's guild is on.
    #[test]
    fn fetching_the_upstream_branch_cannot_touch_main() {
        let run = FakeRun::default();
        assert!(fetch_branch(&run, guild(), "armada").unwrap());
        assert_eq!(
            run.argv(0),
            [
                "git",
                "fetch",
                "origin",
                "+refs/heads/armada:refs/remotes/origin/armada"
            ]
        );
        assert!(
            !run.argv(0).iter().any(|arg| arg.contains(":refs/heads/")),
            "a forced refspec pointed at a local branch: {:?}",
            run.argv(0)
        );
    }

    /// A remote that has never seen the upstream branch is an absence, not a
    /// failure — which is the ordinary state of every guild until one machine
    /// has upgraded once.
    #[test]
    fn a_remote_without_the_upstream_branch_is_an_absence() {
        struct NoBranch;
        impl Run for NoBranch {
            fn call(&self, _: &RunRequest) -> Result<RunOutput, SpawnError> {
                Ok(RunOutput {
                    code: Some(1),
                    signal: None,
                    stdout: String::new(),
                    stderr: "fatal: couldn't find remote ref refs/heads/armada\n".to_string(),
                    timed_out: false,
                })
            }
        }
        assert!(!fetch_branch(&NoBranch, guild(), "armada").unwrap());
    }

    /// A rejected push of the upstream branch is `false` rather than an error:
    /// the branch carries nothing of the person's and the upgrade already
    /// happened.
    #[test]
    fn a_rejected_upstream_push_is_an_answer_rather_than_a_failure() {
        let run = FakeRun::answering(&[(false, "")]);
        assert!(!push_branch(&run, guild(), "armada").unwrap());
        assert_eq!(
            run.argv(0),
            ["git", "push", "origin", "refs/heads/armada:refs/heads/armada"]
        );
        assert!(
            !run.argv(0).iter().any(|arg| arg.contains("force")),
            "the upstream branch is append-only; forcing it would rewrite a base"
        );
    }
}
