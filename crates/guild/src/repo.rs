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
            Change::Added => "added",
            Change::Changed => "changed",
            Change::Removed => "removed",
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
    let output = try_git(run, cwd, args)?;
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
    let mut argv = vec!["git".to_string()];
    argv.extend(args.iter().map(|a| (*a).to_string()));
    let request = RunRequest::new(argv, cwd.to_path_buf())
        .stdio(StdioMode::Capture)
        .timeout(DEADLINE);

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
        assert_eq!(Change::Added.word(), "added");
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
}
