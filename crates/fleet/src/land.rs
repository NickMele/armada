//! Push the branch, open the pull request — the daemon's half of landing
//! (`034` §3, §4; PLAN.md §3).
//!
//! **The daemon pushes, not the Drone.** A Drone is denied `git push` and
//! stays denied (`034` §2); the outward-facing, irreversible step is the one
//! component the owner said he trusts, and pushing here rather than in
//! `crates/fleet/src/drone.rs` is what keeps the deny list meaning something.
//! It is also the only way a pull request's body can be built from the Job's
//! own verified record — [`armada_core::fleet::job::pr_body`] — rather than
//! from a Drone's own summary of itself, which is `034` §4's whole argument
//! for moving the push here at all.
//!
//! **Shells to `git`/`gh` through [`Run`], exactly the seam
//! [`crate::drone`] shells to `claude` through** — never
//! `std::process::Command` directly — so a test asserts the exact argv and
//! feeds back a recorded answer instead of touching a real remote.
//!
//! # Where the wall clock comes from
//!
//! [`push`] and [`open_pr`] stamp [`Job::begin_daemon_act`]/
//! [`Job::settle_daemon_act`] with [`armada_manifest::clock::SystemClock`]
//! directly, rather than threading a `Clock` generic through their
//! signatures. That mirrors the precedent [`crate::daemon::record_started`]
//! already set for the same kind of fact — a daemon-act timestamp is never
//! asserted to the millisecond by a test, only its presence and ordering are,
//! so the seam a `Ctx<R, C, F>` would buy here costs a generic parameter on
//! every caller for nothing a test needs.
//!
//! # Recording the audit trail is not optional, and it is not the caller's
//!
//! `034` §6.5: *"the trail is written before the action where the action is
//! irreversible. Record the intent, act, record the outcome."* [`push`] and
//! [`open_pr`] take `&mut Job` and call
//! [`Job::begin_daemon_act`]/[`Job::settle_daemon_act`] themselves, rather
//! than leaving that to whoever calls them. Item 6 — the daemon's real loop —
//! has not landed yet; a caller that had to remember to wire up the trail is
//! a caller that, on the day it is finally written, might forget. Baking the
//! recording into the primitive means every future caller gets it for free.

use armada_core::ctx::{Clock, Run, RunRequest, SpawnErrorKind};
use armada_core::error::{ArmadaError, ErrClass};
use armada_core::fleet::job::{pr_body, pr_title, DaemonActKind, DaemonOutcome, Job};
use armada_manifest::clock::SystemClock;
use std::fmt;
use std::path::Path;

/// Why [`push`] or [`open_pr`] could not land the work.
///
/// **Three variants a caller can match on, not a free-form message** —
/// PLAN.md §3's own words: *"`LandError` distinguishes no remote configured
/// from push rejected from `gh`/`git` not on `PATH`, because the first is
/// `034`'s 'fails legibly' case and the others are ordinary transient
/// failures."* [`crate::drone`] returns `Result<_, ArmadaError>` throughout,
/// and that is right there because nothing downstream of a Drone call needs
/// to branch on *which* failure it got — every one of them ends the same
/// way, reported and retried unchanged. Here the caller genuinely does need
/// to tell the cases apart: `034` §3 asks for a repository with no remote to
/// "fail legibly at the land step", which only means something if the
/// no-remote case is a distinct, matchable shape rather than one more string
/// inside an `ArmadaError`. So this stays its own enum, and
/// [`From<LandError> for ArmadaError`] is provided for the ordinary case —
/// a caller that has no reason to branch and just wants to report failure
/// the way every other verb does.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum LandError {
    /// No `origin` remote is configured in this worktree, so `git push` has
    /// nowhere to send the branch. `034` §3's "fails legibly at the land
    /// step" case.
    NoRemote(String),
    /// `git push` or `gh pr create` ran and refused, for an ordinary reason:
    /// a non-fast-forward push, an existing PR, an auth failure, or `gh pr
    /// create` succeeding but printing nothing this module can parse as a
    /// PR handle.
    Rejected(String),
    /// `git` or `gh` itself is not on `PATH`.
    NotOnPath(String),
}

impl LandError {
    /// The one line every variant carries — what
    /// [`Job::settle_daemon_act`]'s [`DaemonOutcome::Failed`] stores, and
    /// what a person reading `armada fleet show <job>` reads.
    pub fn message(&self) -> &str {
        match self {
            LandError::NoRemote(m) | LandError::Rejected(m) | LandError::NotOnPath(m) => m,
        }
    }
}

impl fmt::Display for LandError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(self.message())
    }
}

impl std::error::Error for LandError {}

impl From<LandError> for ArmadaError {
    /// The ordinary conversion, for a caller that has no reason to match on
    /// the variant and just wants to report the failure the way every other
    /// verb in this crate does.
    fn from(error: LandError) -> ArmadaError {
        match error {
            LandError::NoRemote(message) => ArmadaError {
                class: ErrClass::BadConfig,
                r#where: "land".to_string(),
                message,
                next_action: Some(
                    "add a remote named `origin` to this repository, then retry".to_string(),
                ),
            },
            LandError::Rejected(message) => ArmadaError {
                class: ErrClass::ToolFailed,
                r#where: "land".to_string(),
                message,
                next_action: None,
            },
            LandError::NotOnPath(message) => ArmadaError {
                class: ErrClass::Environment,
                r#where: "land".to_string(),
                message,
                next_action: Some("install git and gh, then retry unchanged".to_string()),
            },
        }
    }
}

/// A pull request the daemon opened, as `gh pr create` reported it back.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct PrHandle {
    /// The PR's number, parsed from the URL `gh pr create` printed.
    pub number: u64,
    /// Its full URL, unparsed.
    pub url: String,
}

/// Substrings that mean "there is nowhere to push this to", across the
/// wording both `git` and `gh` use for it.
///
/// **Broad on purpose.** `034` §3 only has to be right about the *shape* of
/// the failure — no remote at all — and a narrower match that missed a
/// phrasing would silently fall back to [`LandError::Rejected`], which is
/// the ordinary-transient-failure bucket rather than the fails-legibly one
/// `034` names.
const NO_REMOTE_PHRASES: [&str; 4] = [
    "does not appear to be a git repository",
    "no such remote",
    "no configured push destination",
    "no git remotes found",
];

fn looks_like_no_remote(stderr: &str) -> bool {
    let lower = stderr.to_ascii_lowercase();
    NO_REMOTE_PHRASES
        .iter()
        .any(|phrase| lower.contains(phrase))
}

/// The first non-empty line, or a fallback when there is none — `gh`
/// sometimes fails with nothing on stderr at all.
fn first_line(text: &str, fallback: &str) -> String {
    text.lines()
        .map(str::trim)
        .find(|line| !line.is_empty())
        .unwrap_or(fallback)
        .to_string()
}

/// Push `branch` to `origin` from `worktree` — `git push -u origin
/// <branch>` — and record the attempt on `job` as it happens.
///
/// **The intent is recorded before the push runs, not after.** `034` §6.5:
/// a crash mid-push must leave a `Pushed` act with no outcome rather than
/// nothing at all, which is the one case an audit trail exists for.
pub fn push(run: &impl Run, worktree: &Path, branch: &str, job: &mut Job) -> Result<(), LandError> {
    let act_id = job.begin_daemon_act(
        SystemClock.wall_rfc3339(),
        SystemClock.wall_ms(),
        DaemonActKind::Pushed,
        branch.to_string(),
    );

    let result = run_push(run, worktree, branch);

    match &result {
        Ok(()) => job.settle_daemon_act(&act_id, SystemClock.wall_rfc3339(), DaemonOutcome::Ok),
        Err(error) => job.settle_daemon_act(
            &act_id,
            SystemClock.wall_rfc3339(),
            DaemonOutcome::Failed(error.message().to_string()),
        ),
    }

    result
}

fn run_push(run: &impl Run, worktree: &Path, branch: &str) -> Result<(), LandError> {
    let argv = vec![
        "git".to_string(),
        "push".to_string(),
        "-u".to_string(),
        "origin".to_string(),
        branch.to_string(),
    ];
    let output = run
        .call(&RunRequest::new(argv, worktree.to_path_buf()))
        .map_err(|e| match e.kind {
            SpawnErrorKind::NotFound => LandError::NotOnPath(
                "`git` is not on PATH, so the branch cannot be pushed".to_string(),
            ),
            _ => LandError::Rejected(format!("`git push` would not start: {}", e.message)),
        })?;

    if output.ok() {
        return Ok(());
    }

    if looks_like_no_remote(&output.stderr) {
        return Err(LandError::NoRemote(format!(
            "no remote configured: {}",
            first_line(
                &output.stderr,
                "no `origin` remote is set on this repository"
            )
        )));
    }

    Err(LandError::Rejected(format!(
        "`git push` was rejected: {}",
        first_line(&output.stderr, "no output")
    )))
}

/// Open a pull request for `branch` from `worktree` — `gh pr create
/// --title … --body …` — with a title and body built from `job`'s own
/// record ([`pr_title`], [`pr_body`]), and record the attempt on `job` as it
/// happens.
///
/// `plan_md` is the worktree's own `PLAN.md`, already read by the caller:
/// [`pr_body`] is pure and takes no path, so the one piece of I/O beyond the
/// `gh` call itself lives here, next to the rest of this module's shelling
/// out.
pub fn open_pr(
    run: &impl Run,
    worktree: &Path,
    branch: &str,
    job: &mut Job,
    plan_md: Option<&str>,
) -> Result<PrHandle, LandError> {
    let act_id = job.begin_daemon_act(
        SystemClock.wall_rfc3339(),
        SystemClock.wall_ms(),
        DaemonActKind::Opened,
        // The PR's number does not exist until the call below succeeds, so
        // the branch is what this intent names — the same target `Pushed`
        // records, and the only thing known about this PR before it exists.
        branch.to_string(),
    );

    let result = run_open_pr(run, worktree, branch, job, plan_md);

    match &result {
        Ok(_handle) => {
            job.settle_daemon_act(&act_id, SystemClock.wall_rfc3339(), DaemonOutcome::Ok)
        }
        Err(error) => job.settle_daemon_act(
            &act_id,
            SystemClock.wall_rfc3339(),
            DaemonOutcome::Failed(error.message().to_string()),
        ),
    }

    result
}

fn run_open_pr(
    run: &impl Run,
    worktree: &Path,
    branch: &str,
    job: &Job,
    plan_md: Option<&str>,
) -> Result<PrHandle, LandError> {
    let title = pr_title(job);
    let body = pr_body(job, plan_md);
    let argv = vec![
        "gh".to_string(),
        "pr".to_string(),
        "create".to_string(),
        "--title".to_string(),
        title,
        "--body".to_string(),
        body,
        "--head".to_string(),
        branch.to_string(),
    ];
    let output = run
        .call(&RunRequest::new(argv, worktree.to_path_buf()))
        .map_err(|e| match e.kind {
            SpawnErrorKind::NotFound => LandError::NotOnPath(
                "`gh` is not on PATH, so no pull request can be opened".to_string(),
            ),
            _ => LandError::Rejected(format!("`gh pr create` would not start: {}", e.message)),
        })?;

    if !output.ok() {
        if looks_like_no_remote(&output.stderr) {
            return Err(LandError::NoRemote(format!(
                "no remote configured: {}",
                first_line(
                    &output.stderr,
                    "gh found no git remotes for this repository"
                )
            )));
        }
        return Err(LandError::Rejected(format!(
            "`gh pr create` was rejected: {}",
            first_line(&output.stderr, "no output")
        )));
    }

    parse_pr_handle(&output.stdout).ok_or_else(|| {
        LandError::Rejected(format!(
            "`gh pr create` succeeded but printed no pull request URL this could parse: {}",
            first_line(&output.stdout, "no output")
        ))
    })
}

/// Push, then open — the ordinary sequence, for a caller that wants both
/// without watching anything in between.
///
/// **A convenience, not a third primitive with its own rules.** Nothing
/// happens between a push landing and a PR opening from it — the checks
/// [`034` §3](../../../docs/reserved/034-the-job-daemon-lands-the-work.md)
/// watches only start once the PR exists — so [`push`] and [`open_pr`]
/// staying separate functions is for testability and for item 6's daemon
/// loop, which may call them from two different ticks; this is the version
/// that calls both from one.
pub fn land(
    run: &impl Run,
    worktree: &Path,
    branch: &str,
    job: &mut Job,
    plan_md: Option<&str>,
) -> Result<PrHandle, LandError> {
    push(run, worktree, branch, job)?;
    open_pr(run, worktree, branch, job, plan_md)
}

/// The PR's number and URL, from whatever `gh pr create` printed.
///
/// **The last line that looks like one, scanned from the end.** `gh pr
/// create` prints progress lines before the URL — `"Creating pull request
/// for <branch> into main in OWNER/REPO"` — and the URL itself, `https://
/// github.com/OWNER/REPO/pull/123`, is the thing that actually identifies
/// what was opened. Scanning from the end finds it without depending on how
/// many lines came before it.
fn parse_pr_handle(stdout: &str) -> Option<PrHandle> {
    stdout
        .lines()
        .rev()
        .map(str::trim)
        .find(|line| {
            (line.starts_with("http://") || line.starts_with("https://")) && line.contains("/pull/")
        })
        .and_then(|url| {
            let number = url.rsplit('/').next()?.parse::<u64>().ok()?;
            Some(PrHandle {
                number,
                url: url.to_string(),
            })
        })
}

#[cfg(test)]
mod tests {
    use super::*;
    use armada_core::ctx::{RunOutput, SpawnError};
    use armada_core::fleet::job::Spend;
    use armada_core::fleet::workflow::{Budget, OnExhausted};
    use armada_core::fleet::JobState;
    use std::cell::RefCell;
    use std::path::PathBuf;

    struct FakeRun {
        seen: RefCell<Vec<RunRequest>>,
        output: Result<RunOutput, SpawnError>,
    }

    impl FakeRun {
        fn ok(stdout: &str) -> FakeRun {
            FakeRun {
                seen: RefCell::new(Vec::new()),
                output: Ok(RunOutput {
                    code: Some(0),
                    signal: None,
                    stdout: stdout.to_string(),
                    stderr: String::new(),
                    timed_out: false,
                }),
            }
        }

        fn failing(stderr: &str) -> FakeRun {
            FakeRun {
                seen: RefCell::new(Vec::new()),
                output: Ok(RunOutput {
                    code: Some(1),
                    signal: None,
                    stdout: String::new(),
                    stderr: stderr.to_string(),
                    timed_out: false,
                }),
            }
        }

        fn missing(program: &str) -> FakeRun {
            FakeRun {
                seen: RefCell::new(Vec::new()),
                output: Err(SpawnError {
                    program: program.to_string(),
                    kind: SpawnErrorKind::NotFound,
                    message: "No such file or directory".to_string(),
                }),
            }
        }
    }

    impl Run for FakeRun {
        fn call(&self, request: &RunRequest) -> Result<RunOutput, SpawnError> {
            self.seen.borrow_mut().push(request.clone());
            self.output.clone()
        }
    }

    fn budget() -> Budget {
        Budget {
            attempts: 3,
            cost_usd: 10.0,
            wall_clock_ms: 45 * 60 * 1_000,
            on_exhausted: OnExhausted::NeedsHuman,
        }
    }

    fn job() -> Job {
        Job {
            budget_set: Vec::new(),
            uuid: "8f2a1c40-33b1-4f81-bd7f-688f0f01dbb0".to_string(),
            name: "rate-limit".to_string(),
            workflow: "feature".to_string(),
            confidence: None,
            repo: "api".to_string(),
            repo_root: "~/code/api".to_string(),
            worktree: "~/.armada/workspaces/api/rate-limit".to_string(),
            branch: "armada/rate-limit".to_string(),
            port_block: None,
            budget: budget(),
            state: JobState::Running,
            step: "land".to_string(),
            verdict: None,
            drone: None,
            created_at: "2026-08-17T09:00:00Z".to_string(),
            created_ms: 1_000_000,
            spend: Spend::default(),
            task: "add rate limiting to the API".to_string(),
            progress: Vec::new(),
            attempts: std::collections::BTreeMap::new(),
            waited_ms: 0,
            waiting_from_ms: None,
            transitions: Vec::new(),
            pending: None,
            facts: std::collections::BTreeMap::new(),
            kin: Default::default(),
            ticked_turns: 0,
            doing: None,
            daemon_acts: Vec::new(),
        }
    }

    // -------------------------------------------------------------- push

    #[test]
    fn a_successful_push_records_an_ok_pushed_act() {
        let run = FakeRun::ok("");
        let mut record = job();
        push(&run, Path::new("/wt"), "armada/rate-limit", &mut record).unwrap();

        let argv = &run.seen.borrow()[0].argv;
        assert_eq!(
            argv,
            &vec!["git", "push", "-u", "origin", "armada/rate-limit"]
        );
        assert_eq!(run.seen.borrow()[0].cwd, PathBuf::from("/wt"));

        assert_eq!(record.daemon_acts.len(), 1);
        assert_eq!(record.daemon_acts[0].act, DaemonActKind::Pushed);
        assert_eq!(record.daemon_acts[0].target, "armada/rate-limit");
        assert_eq!(record.daemon_acts[0].outcome, Some(DaemonOutcome::Ok));
    }

    /// **The defect `034` names by name**: a repository with no remote must
    /// fail legibly at the land step, and the failure has to say "no remote"
    /// in words a person reading `armada fleet show` understands.
    #[test]
    fn a_push_against_no_remote_fails_legibly_and_says_so_on_the_job() {
        let run = FakeRun::failing(
            "fatal: 'origin' does not appear to be a git repository\n\
             fatal: Could not read from remote repository.\n",
        );
        let mut record = job();
        let error = push(&run, Path::new("/wt"), "armada/rate-limit", &mut record).unwrap_err();

        assert!(matches!(error, LandError::NoRemote(_)));
        assert!(error.message().to_lowercase().contains("no remote"));

        assert_eq!(record.daemon_acts.len(), 1);
        assert_eq!(record.daemon_acts[0].act, DaemonActKind::Pushed);
        let outcome = record.daemon_acts[0].outcome.as_ref().unwrap();
        assert_eq!(outcome.word(), "failed");
        let detail = outcome.detail().unwrap().to_lowercase();
        assert!(detail.contains("no remote"), "{detail}");
    }

    /// An ordinary rejection — not a missing remote — is a different variant,
    /// so a caller can tell the two apart without parsing the message.
    #[test]
    fn a_push_rejected_for_an_ordinary_reason_is_a_different_variant() {
        let run = FakeRun::failing(
            "! [rejected]        armada/rate-limit -> armada/rate-limit (non-fast-forward)\n",
        );
        let mut record = job();
        let error = push(&run, Path::new("/wt"), "armada/rate-limit", &mut record).unwrap_err();
        assert!(matches!(error, LandError::Rejected(_)));
        assert_eq!(
            record.daemon_acts[0].outcome.as_ref().unwrap().word(),
            "failed"
        );
    }

    #[test]
    fn a_missing_git_is_reported_as_not_on_path() {
        let run = FakeRun::missing("git");
        let mut record = job();
        let error = push(&run, Path::new("/wt"), "armada/rate-limit", &mut record).unwrap_err();
        assert!(matches!(error, LandError::NotOnPath(_)));
        assert!(error.message().contains("git"));
        assert_eq!(
            record.daemon_acts[0].outcome.as_ref().unwrap().word(),
            "failed"
        );
    }

    // ----------------------------------------------------------- open_pr

    #[test]
    fn opening_a_pr_parses_the_number_and_url_from_ghs_own_output() {
        let run = FakeRun::ok(
            "Creating pull request for armada/rate-limit into main in acme/api\n\
             https://github.com/acme/api/pull/142\n",
        );
        let mut record = job();
        let handle = open_pr(
            &run,
            Path::new("/wt"),
            "armada/rate-limit",
            &mut record,
            None,
        )
        .unwrap();

        assert_eq!(handle.number, 142);
        assert_eq!(handle.url, "https://github.com/acme/api/pull/142");

        let argv = &run.seen.borrow()[0].argv;
        assert_eq!(argv[0], "gh");
        assert_eq!(argv[1], "pr");
        assert_eq!(argv[2], "create");
        assert!(argv.contains(&"--title".to_string()));
        assert!(argv.contains(&"--body".to_string()));
        assert_eq!(argv.last().unwrap(), "armada/rate-limit");

        assert_eq!(record.daemon_acts.len(), 1);
        assert_eq!(record.daemon_acts[0].act, DaemonActKind::Opened);
        assert_eq!(record.daemon_acts[0].outcome, Some(DaemonOutcome::Ok));
    }

    /// The body handed to `gh pr create` comes from the Job's own record.
    #[test]
    fn opening_a_pr_sends_a_body_built_from_the_jobs_record() {
        let run = FakeRun::ok("https://github.com/acme/api/pull/1\n");
        let mut record = job();
        record.progress.push(armada_core::fleet::job::Note {
            at: "2026-08-17T09:00:00Z".to_string(),
            at_ms: 1_000,
            step: "implement".to_string(),
            body: "trust me, it works".to_string(),
        });
        open_pr(
            &run,
            Path::new("/wt"),
            "armada/rate-limit",
            &mut record,
            Some("the plan"),
        )
        .unwrap();

        let argv = run.seen.borrow()[0].argv.clone();
        let body_index = argv.iter().position(|a| a == "--body").unwrap() + 1;
        assert!(argv[body_index].contains("add rate limiting to the API"));
        assert!(argv[body_index].contains("the plan"));
        assert!(!argv[body_index].contains("trust me"));
    }

    #[test]
    fn a_pr_open_that_fails_records_a_failed_opened_act() {
        let run = FakeRun::failing("HTTP 401: Bad credentials\n");
        let mut record = job();
        let error = open_pr(
            &run,
            Path::new("/wt"),
            "armada/rate-limit",
            &mut record,
            None,
        )
        .unwrap_err();
        assert!(matches!(error, LandError::Rejected(_)));
        assert_eq!(record.daemon_acts.len(), 1);
        assert_eq!(record.daemon_acts[0].act, DaemonActKind::Opened);
        assert_eq!(
            record.daemon_acts[0].outcome.as_ref().unwrap().word(),
            "failed"
        );
    }

    #[test]
    fn a_missing_gh_is_reported_as_not_on_path() {
        let run = FakeRun::missing("gh");
        let mut record = job();
        let error = open_pr(
            &run,
            Path::new("/wt"),
            "armada/rate-limit",
            &mut record,
            None,
        )
        .unwrap_err();
        assert!(matches!(error, LandError::NotOnPath(_)));
        assert!(error.message().contains("gh"));
    }

    /// `gh pr create` exiting zero but printing nothing recognisable is
    /// still a failure, not a `PrHandle` with garbage in it.
    #[test]
    fn success_with_unparseable_output_is_rejected_rather_than_guessed_at() {
        let run = FakeRun::ok("no url here\n");
        let mut record = job();
        let error = open_pr(
            &run,
            Path::new("/wt"),
            "armada/rate-limit",
            &mut record,
            None,
        )
        .unwrap_err();
        assert!(matches!(error, LandError::Rejected(_)));
    }

    // ----------------------------------------------------------------- land

    #[test]
    fn land_pushes_then_opens_and_both_acts_land_on_the_job() {
        let run = FakeRun::ok("https://github.com/acme/api/pull/9\n");
        let mut record = job();
        let handle = land(
            &run,
            Path::new("/wt"),
            "armada/rate-limit",
            &mut record,
            None,
        )
        .unwrap();
        assert_eq!(handle.number, 9);
        assert_eq!(record.daemon_acts.len(), 2);
        assert_eq!(record.daemon_acts[0].act, DaemonActKind::Pushed);
        assert_eq!(record.daemon_acts[1].act, DaemonActKind::Opened);
    }

    /// A push that fails against no remote stops `land` before `gh` is ever
    /// called — opening a PR from a branch that never reached a remote makes
    /// no sense, and only one act is recorded.
    #[test]
    fn land_does_not_open_a_pr_when_the_push_fails() {
        let run = FakeRun::failing("fatal: 'origin' does not appear to be a git repository\n");
        let mut record = job();
        let error = land(
            &run,
            Path::new("/wt"),
            "armada/rate-limit",
            &mut record,
            None,
        )
        .unwrap_err();
        assert!(matches!(error, LandError::NoRemote(_)));
        assert_eq!(
            record.daemon_acts.len(),
            1,
            "gh should never have been called"
        );
        assert_eq!(run.seen.borrow().len(), 1);
    }

    // --------------------------------------------------------- LandError

    #[test]
    fn every_variant_converts_to_an_armada_error_with_its_message_intact() {
        for error in [
            LandError::NoRemote("no remote configured: x".to_string()),
            LandError::Rejected("rejected: x".to_string()),
            LandError::NotOnPath("`git` is not on PATH".to_string()),
        ] {
            let message = error.message().to_string();
            let armada_error: ArmadaError = error.into();
            assert_eq!(armada_error.message, message);
        }
    }
}
