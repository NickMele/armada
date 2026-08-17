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

use armada_core::config::LandMerge;
use armada_core::ctx::{Clock, Run, RunRequest, SpawnErrorKind};
use armada_core::error::{ArmadaError, ErrClass, Status};
use armada_core::fleet::job::{mint_uuid, pr_body, pr_title, DaemonActKind, DaemonOutcome, Job};
use armada_core::fleet::JobState;
use armada_manifest::clock::SystemClock;
use std::fmt;
use std::path::Path;

use crate::jobs::Store;

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

// ============================================================================
// The daemon's own loop, from here down (`034` §3 §6; PLAN.md §6, §7).
//
// Everything above is item 3: push, open, and the trail either leaves. What
// follows is new orchestration, run by `armada daemon run` — never by
// `armada fleet tick`, whose own gate only ever *watches* `pr_open`/
// `pr_merged`. Nothing here decides whether work is good; every step below is
// a mechanical condition (`034` §2's own argument for why the daemon is
// allowed to merge at all).
// ============================================================================

/// Whether `job` already has a pull request the daemon opened successfully.
///
/// **Read off the audit trail, not re-derived from `gh`.** `034` §6.5 built
/// `daemon_acts` to answer exactly this question without another round trip:
/// an `Opened` act that settled `Ok` is the fact that a PR exists, and asking
/// `gh` again would spend a call to re-learn something already written down.
/// A failed `Opened` is not this — the next sweep should try `land` again,
/// the same way a failed `push` is retried rather than remembered forever.
fn opened_pr(job: &Job) -> bool {
    job.daemon_acts.iter().any(|act| {
        act.act == DaemonActKind::Opened && matches!(act.outcome, Some(DaemonOutcome::Ok))
    })
}

/// What `gh pr checks` reported about the work's pull request.
///
/// **Three answers, not two.** A check still running is not the same as one
/// that failed — [`Pending`](ChecksStatus::Pending) is `armada fleet
/// tick`'s own `Outcome::NotYet` restated for this seam: something is still
/// deciding, and treating that as red would refuse a merge CI was about to
/// approve a minute later.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ChecksStatus {
    /// No checks are configured, or every one already read `pass` or
    /// `skipping` — nothing left to wait on before a merge.
    AllGreen,
    /// At least one check has not reached a terminal state yet.
    Pending,
    /// At least one check finished and did not pass, named here so a caller
    /// that wants to say why can.
    AnyRed(Vec<String>),
}

/// `gh pr checks <branch> --json name,bucket` — read, never acted on here.
///
/// **The exit code cannot be trusted to tell green from red.** `gh pr checks`
/// exits non-zero both when a check is still pending and when one has
/// failed — precisely the two answers this function exists to tell apart —
/// so the JSON on stdout is parsed regardless of what the process exited
/// with, and only a stdout that will not parse at all falls back to treating
/// this as an ordinary [`LandError`] (no PR for this branch yet, `gh` not
/// authenticated, and so on).
pub fn checks_status(
    run: &impl Run,
    worktree: &Path,
    branch: &str,
) -> Result<ChecksStatus, LandError> {
    let argv = vec![
        "gh".to_string(),
        "pr".to_string(),
        "checks".to_string(),
        branch.to_string(),
        "--json".to_string(),
        "name,bucket".to_string(),
    ];
    let output = run
        .call(&RunRequest::new(argv, worktree.to_path_buf()))
        .map_err(|e| match e.kind {
            SpawnErrorKind::NotFound => LandError::NotOnPath(
                "`gh` is not on PATH, so this Job's checks cannot be read".to_string(),
            ),
            _ => LandError::Rejected(format!("`gh pr checks` would not start: {}", e.message)),
        })?;

    let Ok(rows) = serde_json::from_str::<Vec<serde_json::Value>>(&output.stdout) else {
        return Err(LandError::Rejected(format!(
            "`gh pr checks` did not answer with the checks this needed: {}",
            first_line(&output.stderr, "no output")
        )));
    };

    if rows.is_empty() {
        // Nothing configured to block a merge is nothing standing in its way.
        return Ok(ChecksStatus::AllGreen);
    }

    let mut red = Vec::new();
    let mut pending = false;
    for row in &rows {
        let bucket = row.get("bucket").and_then(|v| v.as_str()).unwrap_or("");
        match bucket {
            "pass" | "skipping" => {}
            "pending" => pending = true,
            _ => red.push(
                row.get("name")
                    .and_then(|v| v.as_str())
                    .unwrap_or("a check")
                    .to_string(),
            ),
        }
    }
    if !red.is_empty() {
        return Ok(ChecksStatus::AnyRed(red));
    }
    if pending {
        return Ok(ChecksStatus::Pending);
    }
    Ok(ChecksStatus::AllGreen)
}

/// Record that this Job's checks read all green, immediately — `034` §6.5's
/// table lists `checks-green` as its own act even though nothing outward
/// happens: it is the fact that decided a merge was about to be attempted,
/// and the trail is written before the irreversible step exactly as
/// [`push`]/[`open_pr`] already do it, not folded silently into [`merge`]'s
/// own row.
fn record_checks_green<C: Clock>(clock: &C, job: &mut Job, branch: &str) {
    let id = job.begin_daemon_act(
        clock.wall_rfc3339(),
        clock.wall_ms(),
        DaemonActKind::ChecksGreen,
        branch.to_string(),
    );
    job.settle_daemon_act(&id, clock.wall_rfc3339(), DaemonOutcome::Ok);
}

/// Record that the daemon declined to merge under `fleet.land.merge: never`.
///
/// **Once, not every sweep.** `034` §6.4: *"`never` is not a degraded
/// mode"* — the daemon pushed and opened and has nothing further to do here
/// for as long as the policy stands, and a fresh `RefusedToMerge` row every
/// time the daemon's loop comes back around would turn one true fact into
/// noise. Guarded by whether one is already on the trail, the same
/// "already opened" discipline [`opened_pr`] uses one step up.
fn refuse_to_merge_never<C: Clock>(clock: &C, job: &mut Job, branch: &str) {
    if job
        .daemon_acts
        .iter()
        .any(|act| act.act == DaemonActKind::RefusedToMerge)
    {
        return;
    }
    let id = job.begin_daemon_act(
        clock.wall_rfc3339(),
        clock.wall_ms(),
        DaemonActKind::RefusedToMerge,
        branch.to_string(),
    );
    job.settle_daemon_act(&id, clock.wall_rfc3339(), DaemonOutcome::Ok);
}

/// `gh pr merge <branch> --merge` — record the attempt on `job` as it
/// happens, on [`push`]/[`open_pr`]'s own discipline.
///
/// **`--merge`, not `--squash` or `--rebase`.** PLAN.md's own open question
/// 1 leaves the strategy unresolved as a *schema* question — whether
/// `fleet.land.strategy` should exist at all is for the person approving
/// that plan — but stage one still has to call `gh pr merge` with something
/// today. `--merge` is the provisional answer it names: *"a Drone's history
/// was already asked to be clean before landing"*
/// (`land-branch/SKILL.md`), so a plain merge preserves it rather than
/// squashing it away. Revisiting this is a one-line change here and a
/// one-line schema addition elsewhere; it does not change this function's
/// shape.
pub fn merge(
    run: &impl Run,
    worktree: &Path,
    branch: &str,
    job: &mut Job,
) -> Result<(), LandError> {
    let act_id = job.begin_daemon_act(
        SystemClock.wall_rfc3339(),
        SystemClock.wall_ms(),
        DaemonActKind::Merged,
        branch.to_string(),
    );
    let result = run_merge(run, worktree, branch);
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

fn run_merge(run: &impl Run, worktree: &Path, branch: &str) -> Result<(), LandError> {
    let argv = vec![
        "gh".to_string(),
        "pr".to_string(),
        "merge".to_string(),
        branch.to_string(),
        "--merge".to_string(),
    ];
    let output = run
        .call(&RunRequest::new(argv, worktree.to_path_buf()))
        .map_err(|e| match e.kind {
            SpawnErrorKind::NotFound => LandError::NotOnPath(
                "`gh` is not on PATH, so this pull request cannot be merged".to_string(),
            ),
            _ => LandError::Rejected(format!("`gh pr merge` would not start: {}", e.message)),
        })?;
    if output.ok() {
        return Ok(());
    }
    Err(LandError::Rejected(format!(
        "`gh pr merge` was rejected: {}",
        first_line(&output.stderr, "no output")
    )))
}

/// `git fetch origin main:main` — update the *shared repository's* `main`,
/// not the Job's own worktree.
///
/// **PLAN.md's own open question 2, resolved here.** `repo_root` is where
/// this runs, because it is the one place `origin` is configured and the one
/// place every future `git worktree add` reads its starting ref from
/// ([`crate::worktree::add`]'s own `from` parameter) — updating it is what
/// makes the next Job spawned from this repository branch off the merge
/// that just landed. The Job's *own* worktree is the wrong place: it is a
/// `land`ed, about-to-be-reclaimed checkout of the Job's own branch, not a
/// clone with `origin` pointed anywhere useful for this.
///
/// **This alone does not give anything a checked-out tree to run checks
/// against**, which is why [`rerun_on_main`] does not read `main` here —
/// `git fetch` only moves a ref, and re-running checks needs files on disk.
pub fn pull_main(run: &impl Run, repo_root: &Path, job: &mut Job) -> Result<(), LandError> {
    let act_id = job.begin_daemon_act(
        SystemClock.wall_rfc3339(),
        SystemClock.wall_ms(),
        DaemonActKind::Pulled,
        "main".to_string(),
    );
    let result = run_pull_main(run, repo_root);
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

fn run_pull_main(run: &impl Run, repo_root: &Path) -> Result<(), LandError> {
    let argv = vec![
        "git".to_string(),
        "fetch".to_string(),
        "origin".to_string(),
        "main:main".to_string(),
    ];
    let output = run
        .call(&RunRequest::new(argv, repo_root.to_path_buf()))
        .map_err(|e| match e.kind {
            SpawnErrorKind::NotFound => {
                LandError::NotOnPath("`git` is not on PATH, so `main` cannot be pulled".to_string())
            }
            _ => LandError::Rejected(format!(
                "`git fetch origin main:main` would not start: {}",
                e.message
            )),
        })?;
    if output.ok() {
        return Ok(());
    }
    Err(LandError::Rejected(format!(
        "`git fetch origin main:main` was rejected: {}",
        first_line(&output.stderr, "no output")
    )))
}

/// Tell every other `RUNNING`/`PAUSED` Job in the same repository that
/// `main` moved (PLAN.md §7).
///
/// **Two different writes, on two different records, in one pass** — the
/// plan's own words. Each sibling's own `main_moved_at` is set directly on
/// its record; the mover gets one `MarkedMainMoved` act per sibling it
/// marked, because the target a [`DaemonAct`](armada_core::fleet::job::DaemonAct)
/// names is the thing it acted on, and here that is a specific other Job —
/// naming several inside one row would need a second shape this type does
/// not have, for a batch that a reader can already reconstruct from
/// several rows sharing one timestamp.
///
/// **Same repository, by `repo_root` as recorded** — not by resolving the
/// path, since two Jobs whose worktrees came from the same clone always
/// agree on how that clone's own record spells it.
pub fn mark_main_moved<C: Clock>(
    clock: &C,
    store: &Store,
    mover: &mut Job,
) -> Result<usize, ArmadaError> {
    let mut marked = 0usize;
    for mut other in store.all()? {
        if other.uuid == mover.uuid || other.repo_root != mover.repo_root {
            continue;
        }
        if !matches!(other.state, JobState::Running | JobState::Paused) {
            continue;
        }
        other.main_moved_at = Some(clock.wall_rfc3339());
        store.save(&other)?;

        let id = mover.begin_daemon_act(
            clock.wall_rfc3339(),
            clock.wall_ms(),
            DaemonActKind::MarkedMainMoved,
            other.uuid.clone(),
        );
        mover.settle_daemon_act(&id, clock.wall_rfc3339(), DaemonOutcome::Ok);
        marked += 1;
    }
    Ok(marked)
}

/// How often [`rerun_on_main`] polls a running `armada manifest check`.
///
/// **Three seconds.** This is a blocking wait inside the daemon's own loop
/// rather than the detach-and-return shape `armada fleet tick` needs
/// (`crates/helm/src/verbs/fleet.rs`'s own `check` helper) — the daemon has
/// nothing else to do while one Job's re-run finishes, so there is no
/// caller to hand control back to, and a poll interval only has to be short
/// enough that a fast check does not sit idle for it.
const RERUN_POLL_MS: u64 = 3_000;

/// Re-run checks on the updated `main` — the step `034` §3 states is *"not
/// redundant"*: CI passed on the PR's own merge commit, against a `main`
/// that may have moved since it was opened, and this is the only thing that
/// catches two PRs that were each green alone and are not green together.
///
/// **A temporary, detached worktree off `repo_root` at `main`, removed
/// whatever the outcome.** PLAN.md's own open question 2, continued from
/// [`pull_main`]: fetching updates the ref every future `git worktree add`
/// reads, but running checks needs an actual tree on disk, and `repo_root`
/// itself is very likely sitting on whatever branch a person last had it on
/// rather than `main` — nothing in this codebase disciplines it to stay
/// there. So a scratch worktree is checked out at `main`, `armada manifest
/// check` runs inside it, and [`crate::worktree::remove`] takes it away
/// again before this returns, pass or fail, so it can never leak.
///
/// **Blocking, not detach-and-poll-on-the-next-tick.** `armada fleet tick`
/// detaches a check because a person is waiting on the CLI to return; the
/// daemon's own loop is a background process with nothing else to do while
/// this one Job's re-run finishes, so looping here inside one call is
/// simpler and no less correct than making the caller remember to come back.
pub fn rerun_on_main<R: Run, C: Clock>(
    run: &R,
    clock: &C,
    exe: &Path,
    repo_root: &Path,
    tmp_worktree: &Path,
    job: &mut Job,
) -> Result<bool, ArmadaError> {
    let act_id = job.begin_daemon_act(
        clock.wall_rfc3339(),
        clock.wall_ms(),
        DaemonActKind::ReRan,
        "main".to_string(),
    );
    let result = rerun_inner(run, clock, exe, repo_root, tmp_worktree);
    match &result {
        Ok(true) => job.settle_daemon_act(&act_id, clock.wall_rfc3339(), DaemonOutcome::Ok),
        Ok(false) => job.settle_daemon_act(
            &act_id,
            clock.wall_rfc3339(),
            DaemonOutcome::Failed("checks on the updated main are not all green".to_string()),
        ),
        Err(error) => job.settle_daemon_act(
            &act_id,
            clock.wall_rfc3339(),
            DaemonOutcome::Failed(error.message.clone()),
        ),
    }
    result
}

fn rerun_inner<R: Run, C: Clock>(
    run: &R,
    clock: &C,
    exe: &Path,
    repo_root: &Path,
    tmp_worktree: &Path,
) -> Result<bool, ArmadaError> {
    crate::worktree::add_detached(run, repo_root, tmp_worktree, "main")?;
    let outcome = rerun_checks(run, clock, exe, tmp_worktree);
    // **Removed whatever the outcome above was.** This worktree is scratch
    // space for one re-run and nothing else; leaving it behind on a failure
    // path is exactly the leak PLAN.md's open question 2 flags.
    let _ = crate::worktree::remove(run, repo_root, tmp_worktree);
    outcome
}

fn rerun_checks<R: Run, C: Clock>(
    run: &R,
    clock: &C,
    exe: &Path,
    tmp_worktree: &Path,
) -> Result<bool, ArmadaError> {
    let run_id = crate::manifest::check_detach(run, exe, tmp_worktree, None)?;
    loop {
        let (status, _exit, _failed) =
            crate::manifest::check_status(run, exe, tmp_worktree, &run_id)?;
        if status.is_terminal() {
            return Ok(status == Status::Pass);
        }
        clock.sleep_until(clock.mono().saturating_add(RERUN_POLL_MS));
    }
}

/// Report a red re-run to a person — never to a Drone.
///
/// **This is new information a green PR did not carry.** `034` §3's own
/// words, and stage one's whole boundary: no `start_drone`, no
/// `resume_argv`, no `claude` session spawned to "fix and retry" — that is
/// stage two (`034` §6.1's fleet-wide budget). The two writes here are
/// exactly the shape a CI failure already takes elsewhere in this design:
/// [`DaemonActKind::ReportedFailure`] on the Job's own trail, and one
/// [`crate::inbox`] entry, so `armada fleet inbox` and the Bridge's pane
/// both see it without a second mechanism being invented for this one case.
pub fn report_rerun_failure<C: Clock>(
    clock: &C,
    armada_home: &Path,
    job: &mut Job,
) -> Result<(), ArmadaError> {
    let id = job.begin_daemon_act(
        clock.wall_rfc3339(),
        clock.wall_ms(),
        DaemonActKind::ReportedFailure,
        "main".to_string(),
    );
    job.settle_daemon_act(&id, clock.wall_rfc3339(), DaemonOutcome::Ok);

    let at_ms = clock.wall_ms();
    crate::inbox::raise(
        &crate::home::inbox(armada_home),
        &mint_uuid(&format!("{}|{at_ms}|main-rerun-red", job.uuid)),
        &job.uuid,
        &job.name,
        crate::inbox::Kind::Blocked,
        &clock.wall_rfc3339(),
        at_ms,
        &format!(
            "`{}` merged cleanly on its own, but the re-run of checks on the updated `main` \
             came back red — two changes that were each green alone are not green together. \
             Stage one does not resume a Drone to fix this (`034` §6.1); it needs a person.",
            job.branch
        ),
    )?;
    Ok(())
}

/// If `git`/`gh` themselves were unreachable — not a `git`/`gh` failure this
/// codebase already has words for, but the tool missing from `PATH` — say so
/// on the machine's own log rather than the Job's.
///
/// **`034` §6.5's own distinction.** A push rejected for an ordinary reason
/// is about this Job; `git`/`gh` not being installed is a fact about the
/// machine the daemon is running on, and every Job's sweep would otherwise
/// record the identical sentence once each, which buries the one fact that
/// actually explains all of them under a pile of copies.
fn note_if_unreachable<C: Clock>(
    clock: &C,
    armada_home: &Path,
    error: &LandError,
) -> Result<(), ArmadaError> {
    if let LandError::NotOnPath(detail) = error {
        crate::daemon_log::append(
            armada_home,
            &crate::daemon_log::Entry::GhUnreachable {
                at: clock.wall_rfc3339(),
                at_ms: clock.wall_ms(),
                detail: detail.clone(),
            },
        )?;
    }
    Ok(())
}

/// What one [`sweep_one`] pass did about a Job at the `land` step.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Sweep {
    /// `land` ran this pass — either it just pushed and opened, or it
    /// already had, and nothing else was ready.
    Opened,
    /// An open PR exists but its checks have not all read green yet.
    Waiting,
    /// `fleet.land.merge: never` — the daemon has done everything it will
    /// ever do here; a person merges from here on.
    AwaitingAHuman,
    /// Merge, pull and the re-run on `main` all succeeded. **The caller is
    /// responsible for reclaiming this Job's worktree** — `crates/fleet`
    /// sits below `crates/helm` (`ARCHITECTURE.md`'s layering), so the reap
    /// machinery `armada fleet reap` already built lives one crate up and
    /// has to be called from there.
    ReadyToReap,
    /// The re-run on `main` came back red — reported, not reaped.
    ReportedFailure,
    /// Something along the way failed. The reason is on the Job's own
    /// `daemon_acts`, or in `daemon.jsonl` if `git`/`gh` were unreachable.
    Failed,
}

/// One Job at the `land` step, walked exactly as far as it is ready to go.
///
/// **The daemon's own loop calls this once per Job, per pass** — never
/// `armada fleet tick`, whose gate only ever watches `pr_open`/`pr_merged`
/// and takes no action of its own (this module's header, and `034` §3).
/// `worktree` and `repo_root` are already-resolved absolute paths: this
/// crate has no equivalent of `crates/helm`'s `Where::expand`, so the
/// caller — which does — resolves them before calling in.
///
/// # This function never returns an error for an ordinary refusal
///
/// A push rejected, a merge that will not go through, `gh` unreachable —
/// every one of those is recorded on `job.daemon_acts` (or `daemon.jsonl`
/// for the last) by the function that hit it, and reported back here as
/// [`Sweep::Failed`] rather than propagated. What *does* return `Err` is a
/// failure to persist the record itself — `store.save` — because a Job
/// whose trail cannot be written to disk is not a Job this pass can honestly
/// claim to have swept, and the caller (`crates/helm/src/verbs/daemon.rs`)
/// is the one place `034` says a broken machine belongs
/// ([`crate::daemon_log`]).
#[allow(clippy::too_many_arguments)]
pub fn sweep_one<R: Run, C: Clock>(
    run: &R,
    clock: &C,
    store: &Store,
    armada_home: &Path,
    exe: &Path,
    worktree: &Path,
    repo_root: &Path,
    tmp_worktree: &Path,
    job: &mut Job,
) -> Result<Sweep, ArmadaError> {
    if !opened_pr(job) {
        let plan_md = std::fs::read_to_string(worktree.join("PLAN.md")).ok();
        let branch = job.branch.clone();
        let result = land(run, worktree, &branch, job, plan_md.as_deref());
        store.save(job)?;
        return match result {
            Ok(_) => Ok(Sweep::Opened),
            Err(error) => {
                note_if_unreachable(clock, armada_home, &error)?;
                Ok(Sweep::Failed)
            }
        };
    }

    let branch = job.branch.clone();
    let land_merge = crate::manifest::land_merge(worktree);

    let checks = match checks_status(run, worktree, &branch) {
        Ok(checks) => checks,
        Err(error) => {
            note_if_unreachable(clock, armada_home, &error)?;
            return Ok(Sweep::Failed);
        }
    };
    if !matches!(checks, ChecksStatus::AllGreen) {
        return Ok(Sweep::Waiting);
    }

    if land_merge == LandMerge::Never {
        refuse_to_merge_never(clock, job, &branch);
        store.save(job)?;
        return Ok(Sweep::AwaitingAHuman);
    }

    record_checks_green(clock, job, &branch);
    store.save(job)?;

    if let Err(error) = merge(run, worktree, &branch, job) {
        store.save(job)?;
        note_if_unreachable(clock, armada_home, &error)?;
        return Ok(Sweep::Failed);
    }
    store.save(job)?;

    if let Err(error) = pull_main(run, repo_root, job) {
        store.save(job)?;
        note_if_unreachable(clock, armada_home, &error)?;
        return Ok(Sweep::Failed);
    }
    store.save(job)?;

    // **After the pull succeeds, not gated on what the re-run finds.**
    // PLAN.md §7: siblings learn `main` moved because it did — the fact is
    // true whether or not this Job's own re-run turns out green.
    mark_main_moved(clock, store, job)?;
    store.save(job)?;

    let green = match rerun_on_main(run, clock, exe, repo_root, tmp_worktree, job) {
        Ok(green) => green,
        Err(_error) => {
            store.save(job)?;
            return Ok(Sweep::Failed);
        }
    };
    store.save(job)?;

    if green {
        return Ok(Sweep::ReadyToReap);
    }
    report_rerun_failure(clock, armada_home, job)?;
    store.save(job)?;
    Ok(Sweep::ReportedFailure)
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
            main_moved_at: None,
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

    // =================================================== the daemon's own loop

    mod sweep {
        use super::*;

        /// A wall clock that never moves and never really sleeps — every test
        /// below either resolves on its first poll or does not poll at all.
        struct FixedClock;
        impl Clock for FixedClock {
            fn wall_rfc3339(&self) -> String {
                "2026-08-17T09:00:00Z".to_string()
            }
            fn wall_ms(&self) -> u64 {
                1_000
            }
            fn mono(&self) -> u64 {
                0
            }
            fn sleep_until(&self, _: u64) {}
        }

        /// `job()` with a successful `Opened` act already on it, standing in
        /// for a Job whose PR the daemon already pushed and opened on an
        /// earlier pass — so [`sweep_one`] skips straight to watching it.
        fn job_with_open_pr() -> Job {
            let mut record = job();
            let id = record.begin_daemon_act(
                "2026-08-17T08:00:00Z".to_string(),
                500,
                DaemonActKind::Opened,
                record.branch.clone(),
            );
            record.settle_daemon_act(&id, "2026-08-17T08:00:00Z".to_string(), DaemonOutcome::Ok);
            record
        }

        /// A repository whose `armada.yml` opts into (or refuses)
        /// unattended merging.
        fn repo_with_land_merge(merge: &str) -> tempfile::TempDir {
            let repo = tempfile::tempdir().unwrap();
            std::fs::write(
                repo.path().join("armada.yml"),
                format!("manifest:\n  version: 1\nfleet:\n  land:\n    merge: {merge}\n"),
            )
            .unwrap();
            repo
        }

        /// One `Run` that answers each of the several different commands one
        /// `sweep_one` pass may issue, keyed by argv shape rather than by
        /// call order — `checks_status`/`merge`/`pull_main`/`rerun_on_main`
        /// each build their own argv and this is the one fake standing in
        /// for `git`, `gh` and `armada` alike.
        struct ScriptedRun {
            seen: RefCell<Vec<Vec<String>>>,
            checks: String,
            check_status: String,
        }

        impl ScriptedRun {
            fn new(checks_json: &str, check_status_json: &str) -> ScriptedRun {
                ScriptedRun {
                    seen: RefCell::new(Vec::new()),
                    checks: checks_json.to_string(),
                    check_status: check_status_json.to_string(),
                }
            }

            fn saw(&self, word: &str) -> bool {
                self.seen
                    .borrow()
                    .iter()
                    .any(|argv| argv.iter().any(|a| a == word))
            }
        }

        const CHECK_DETACH_STARTED: &str = r#"{"schema_version":2,"verb":"check","status":"RUNNING","error":null,"data":{"run_id":"01M0","results":[]}}"#;

        impl Run for ScriptedRun {
            fn call(&self, request: &RunRequest) -> Result<RunOutput, SpawnError> {
                self.seen.borrow_mut().push(request.argv.clone());
                let argv = &request.argv;
                let stdout = match argv.first().map(String::as_str) {
                    Some("gh") if argv.contains(&"checks".to_string()) => self.checks.clone(),
                    Some("gh") | Some("git") => String::new(),
                    // Everything else is `armada manifest check …`, run from
                    // inside the temporary re-run worktree.
                    _ if argv.contains(&"--detach".to_string()) => CHECK_DETACH_STARTED.to_string(),
                    _ if argv.contains(&"--status".to_string()) => self.check_status.clone(),
                    other => panic!("ScriptedRun has no answer for {other:?}"),
                };
                Ok(RunOutput {
                    code: Some(0),
                    signal: None,
                    stdout,
                    stderr: String::new(),
                    timed_out: false,
                })
            }
        }

        fn all_green_checks() -> &'static str {
            r#"[{"name":"build","bucket":"pass"}]"#
        }

        fn check_status_pass() -> &'static str {
            r#"{"schema_version":2,"verb":"check","status":"PASS","error":null,"data":{"results":[]}}"#
        }

        fn check_status_failed() -> &'static str {
            r#"{"schema_version":2,"verb":"check","status":"FAILED","error":null,"data":{"results":[]}}"#
        }

        /// **The whole green sequence, in order, each act recorded** — merge,
        /// pull, re-run, and a Job the caller is told is ready to reap.
        #[test]
        fn a_green_pr_under_auto_merges_pulls_reruns_and_is_ready_to_reap() {
            let repo = repo_with_land_merge("auto");
            let home = tempfile::tempdir().unwrap();
            let store = Store::at(home.path());
            let run = ScriptedRun::new(all_green_checks(), check_status_pass());
            let clock = FixedClock;
            let mut record = job_with_open_pr();
            store.save(&record).unwrap();

            let outcome = sweep_one(
                &run,
                &clock,
                &store,
                home.path(),
                Path::new("/bin/armada"),
                repo.path(),
                repo.path(),
                &home.path().join("tmp-rerun"),
                &mut record,
            )
            .unwrap();

            assert_eq!(outcome, Sweep::ReadyToReap);
            assert!(run.saw("merge"), "the PR was never merged");
            assert!(run.saw("fetch"), "main was never pulled");

            let acts: Vec<DaemonActKind> = record.daemon_acts.iter().map(|a| a.act).collect();
            assert_eq!(
                acts,
                vec![
                    DaemonActKind::Opened,
                    DaemonActKind::ChecksGreen,
                    DaemonActKind::Merged,
                    DaemonActKind::Pulled,
                    DaemonActKind::ReRan,
                ],
                "{acts:?}"
            );
            for act in &record.daemon_acts {
                assert_eq!(
                    act.outcome,
                    Some(DaemonOutcome::Ok),
                    "{:?} did not settle Ok",
                    act.act
                );
            }
            // The scratch worktree is gone — removed whatever the outcome.
            assert!(run.saw("remove"), "the re-run worktree was never removed");
        }

        /// **A red re-run: reported, not reaped, and nothing resembling a
        /// Drone resume happens.** `034` §3's own case — a green PR that
        /// merged cleanly on its own is not the same as `main` being green
        /// with it.
        #[test]
        fn a_red_rerun_reports_and_raises_one_inbox_entry_without_reaping() {
            let repo = repo_with_land_merge("auto");
            let home = tempfile::tempdir().unwrap();
            let store = Store::at(home.path());
            let run = ScriptedRun::new(all_green_checks(), check_status_failed());
            let clock = FixedClock;
            let mut record = job_with_open_pr();
            store.save(&record).unwrap();

            let outcome = sweep_one(
                &run,
                &clock,
                &store,
                home.path(),
                Path::new("/bin/armada"),
                repo.path(),
                repo.path(),
                &home.path().join("tmp-rerun"),
                &mut record,
            )
            .unwrap();

            assert_eq!(outcome, Sweep::ReportedFailure);

            let acts: Vec<DaemonActKind> = record.daemon_acts.iter().map(|a| a.act).collect();
            assert_eq!(
                acts,
                vec![
                    DaemonActKind::Opened,
                    DaemonActKind::ChecksGreen,
                    DaemonActKind::Merged,
                    DaemonActKind::Pulled,
                    DaemonActKind::ReRan,
                    DaemonActKind::ReportedFailure,
                ],
                "{acts:?}"
            );
            // **Not reaped.** No act here is ever named `Reaped` — that is
            // the caller's own act, recorded by `crates/helm`'s teardown,
            // and it must never be reached on a red re-run.
            assert!(!acts.contains(&DaemonActKind::Reaped));

            let re_ran = record
                .daemon_acts
                .iter()
                .find(|a| a.act == DaemonActKind::ReRan)
                .unwrap();
            assert_eq!(re_ran.outcome.as_ref().unwrap().word(), "failed");

            // One inbox entry, naming this Job, and nothing that looks like
            // a Drone was ever asked to fix anything — stage one raises a
            // question and stops; it does not resume.
            let entries = crate::inbox::read(&crate::home::inbox(home.path())).unwrap();
            assert_eq!(entries.len(), 1, "{entries:#?}");
            assert_eq!(entries[0].job_uuid.as_deref(), Some(record.uuid.as_str()));
            assert!(entries[0].is_open());
            assert!(entries[0].body.contains("main"), "{}", entries[0].body);
        }

        /// **`never` stops after the PR is open — no merge is ever
        /// attempted**, `034` §6.4: *"`never` is not a degraded mode."*
        #[test]
        fn never_policy_watches_checks_but_never_merges() {
            let repo = repo_with_land_merge("never");
            let home = tempfile::tempdir().unwrap();
            let store = Store::at(home.path());
            // Green checks, so the only thing left to prove is that policy —
            // not a red check — is what stopped the merge.
            let run = ScriptedRun::new(all_green_checks(), check_status_pass());
            let clock = FixedClock;
            let mut record = job_with_open_pr();
            store.save(&record).unwrap();

            let outcome = sweep_one(
                &run,
                &clock,
                &store,
                home.path(),
                Path::new("/bin/armada"),
                repo.path(),
                repo.path(),
                &home.path().join("tmp-rerun"),
                &mut record,
            )
            .unwrap();

            assert_eq!(outcome, Sweep::AwaitingAHuman);
            assert!(!run.saw("merge"), "a merge was attempted under `never`");
            assert!(!run.saw("fetch"), "main was pulled with nothing merged");

            let acts: Vec<DaemonActKind> = record.daemon_acts.iter().map(|a| a.act).collect();
            assert_eq!(
                acts,
                vec![DaemonActKind::Opened, DaemonActKind::RefusedToMerge]
            );

            // Idempotent: a second sweep does not add a second refusal.
            sweep_one(
                &run,
                &clock,
                &store,
                home.path(),
                Path::new("/bin/armada"),
                repo.path(),
                repo.path(),
                &home.path().join("tmp-rerun"),
                &mut record,
            )
            .unwrap();
            assert_eq!(
                record
                    .daemon_acts
                    .iter()
                    .filter(|a| a.act == DaemonActKind::RefusedToMerge)
                    .count(),
                1
            );
        }

        // ------------------------------------------------------ main moved

        fn sibling(name: &str, uuid: &str, repo_root: &str, state: JobState) -> Job {
            let mut record = job();
            record.name = name.to_string();
            record.uuid = uuid.to_string();
            record.repo_root = repo_root.to_string();
            record.state = state;
            record
        }

        /// **A sibling in the same repository, still live, is marked; a
        /// sibling in a different repository is not; the mover marks itself
        /// nowhere.** PLAN.md §7's own words, checked directly against
        /// [`mark_main_moved`] rather than through the whole sweep.
        #[test]
        fn main_moved_marks_only_live_siblings_in_the_same_repository() {
            let home = tempfile::tempdir().unwrap();
            let store = Store::at(home.path());

            let mut mover = job();
            mover.uuid = "mover-uuid".to_string();
            mover.repo_root = "~/code/api".to_string();
            store.save(&mover).unwrap();

            let same_repo_running = sibling(
                "same-repo-running",
                "sib-1",
                "~/code/api",
                JobState::Running,
            );
            store.save(&same_repo_running).unwrap();

            let same_repo_paused =
                sibling("same-repo-paused", "sib-2", "~/code/api", JobState::Paused);
            store.save(&same_repo_paused).unwrap();

            let same_repo_done = sibling("same-repo-done", "sib-3", "~/code/api", JobState::Done);
            store.save(&same_repo_done).unwrap();

            let other_repo = sibling("other-repo", "sib-4", "~/code/web", JobState::Running);
            store.save(&other_repo).unwrap();

            let clock = FixedClock;
            let marked = mark_main_moved(&clock, &store, &mut mover).unwrap();

            assert_eq!(marked, 2, "only the two live same-repository siblings");
            assert!(store.load("sib-1").unwrap().main_moved_at.is_some());
            assert!(store.load("sib-2").unwrap().main_moved_at.is_some());
            assert!(
                store.load("sib-3").unwrap().main_moved_at.is_none(),
                "a finished sibling is not mid-turn and has nothing to notice"
            );
            assert!(
                store.load("sib-4").unwrap().main_moved_at.is_none(),
                "a different repository's main did not move"
            );
            assert!(
                store.load("mover-uuid").unwrap().main_moved_at.is_none(),
                "the mover marks siblings, not itself"
            );

            let marked_acts: Vec<&str> = mover
                .daemon_acts
                .iter()
                .filter(|a| a.act == DaemonActKind::MarkedMainMoved)
                .map(|a| a.target.as_str())
                .collect();
            assert_eq!(marked_acts.len(), 2, "{marked_acts:?}");
            assert!(marked_acts.contains(&"sib-1"));
            assert!(marked_acts.contains(&"sib-2"));
        }
    }
}
