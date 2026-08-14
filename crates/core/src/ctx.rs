//! The three injected seams, and the value they travel on
//! (`ARCHITECTURE.md` §1.1).
//!
//! Every interaction with the outside world that is slow, nondeterministic or
//! external is reached through a function passed in, never imported. There are
//! exactly three — `run`, `now`, `fetch` — and the count is a decision rather
//! than an accident:
//!
//! - **Docker and git are subprocess calls**, so they are ordinary adapter
//!   modules that build argv and call [`Run`]. Giving them their own ports
//!   would mean three different ways to fake a shell command.
//! - **Faking at the `run` level keeps argv assertable, and argv is where the
//!   bugs are.** charkit's central claim is *"`clean` releases exactly what this
//!   workspace owns"*, and when that breaks it looks like `--filter
//!   label=char.workspace` written without its `=<id>`. A test that fakes `run`
//!   catches that; a test that fakes a `DockerPort` catches none of it.
//! - **The filesystem and SQLite are not faked at all.** char depends on real
//!   transaction semantics for port claims and lease acquisition, so a fake
//!   proves things about the fake. Two threads against a real database in a
//!   `TempDir` is both more faithful and less code.
//!
//! Traits rather than boxed closures, so a fake is a zero-cost substitution the
//! compiler checks and the production path pays nothing for.

use crate::error::ArmadaError;
use crate::workspace::Workspace;
use std::collections::BTreeMap;
use std::path::PathBuf;
use std::time::Duration;

/// The seams and the workspace, travelling together as one value passed as the
/// first argument.
///
/// A `Ctx` parameter threaded everywhere is ambient state with better
/// testability. That is the trade, made on purpose (`ARCHITECTURE.md` §1.4),
/// and it is why these four things ride together rather than as four
/// arguments.
pub struct Ctx<R: Run, C: Clock, F: Fetch> {
    /// `None` for the two machine-scoped invocations that run outside any
    /// workspace: `char config scan`, and `char clean --all --orphaned`
    /// (PLAN.md §2.1).
    pub workspace: Option<Workspace>,
    /// Every subprocess.
    pub run: R,
    /// Timeouts, heartbeat staleness (monotonic), `claimed_at` (wall).
    pub now: C,
    /// `http` and `tcp` ready-checks.
    pub fetch: F,
}

impl<R: Run, C: Clock, F: Fetch> Ctx<R, C, F> {
    /// The workspace, or a `armada_bug` — reaching for one inside a verb that
    /// declared it needs none is a programming error, not a user error.
    pub fn workspace(&self) -> Result<&Workspace, ArmadaError> {
        self.workspace.as_ref().ok_or_else(|| ArmadaError {
            class: crate::error::ErrClass::ArmadaBug,
            r#where: "ctx.workspace".to_string(),
            message: "this verb needs a workspace and was given none".to_string(),
            next_action: None,
        })
    }
}

/// Where a child's output goes.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum StdioMode {
    /// char reads the child's stdout and stderr, and can therefore scrub them.
    Capture,
    /// The child keeps char's own descriptors — colours, progress bars and
    /// prompts work, and char sees nothing.
    Inherit,
}

/// One subprocess, fully specified.
///
/// `argv` is a list rather than a string on purpose: the split — including
/// quote handling and `${files}` expansion — is a pure decision made in
/// [`crate::template`], so the seam never re-parses anything and a test can
/// assert the exact vector that would have been executed.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct RunRequest {
    /// Program and arguments. `shell: true` arrives here already wrapped as
    /// `["/bin/sh", "-c", …]`.
    pub argv: Vec<String>,
    /// The working directory. Always explicit — nothing below the entrypoint
    /// may read the process's own cwd (`ARCHITECTURE.md` §1.4).
    pub cwd: PathBuf,
    /// Layered over the inherited environment, never replacing it.
    pub env: BTreeMap<String, String>,
    /// Where the child's output goes.
    pub stdio: StdioMode,
    /// char's own deadline. `None` means char imposes none — which is correct
    /// for a dispatched `commands:` entry, whose runtime is the repo's business.
    pub timeout: Option<Duration>,
    /// Spawn the child in a new **session** via `setsid`, so its whole tree
    /// can be reached with one `killpg`. Measured: this is mutually exclusive
    /// with `process_group(0)` (`docs/traps.md`), so it is one flag and not two.
    pub new_session: bool,
}

impl RunRequest {
    /// A captured, session-detached call with no deadline — the shape every
    /// git and docker call starts from.
    pub fn new(argv: Vec<String>, cwd: PathBuf) -> Self {
        RunRequest {
            argv,
            cwd,
            env: BTreeMap::new(),
            stdio: StdioMode::Capture,
            timeout: None,
            new_session: true,
        }
    }

    /// Set the deadline. Every docker call needs one: measured, the docker CLI
    /// has no client-side timeout at all, and the `docker ps` in `init`'s reap
    /// pass is the one that matters — without a deadline a hung daemon wedges
    /// every new workspace on the machine, including the verb whose job is
    /// recovery (`docs/traps.md`).
    pub fn timeout(mut self, timeout: Duration) -> Self {
        self.timeout = Some(timeout);
        self
    }

    /// Layer environment variables over the inherited ones.
    pub fn env(mut self, env: BTreeMap<String, String>) -> Self {
        self.env = env;
        self
    }

    /// Choose where the child's output goes.
    pub fn stdio(mut self, stdio: StdioMode) -> Self {
        self.stdio = stdio;
        self
    }
}

/// What a finished child reported.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct RunOutput {
    /// The exit code, or `None` when a signal ended the child.
    pub code: Option<i32>,
    /// The signal that ended the child, when one did.
    pub signal: Option<i32>,
    /// Empty under [`StdioMode::Inherit`], because char never saw it.
    pub stdout: String,
    /// Empty under [`StdioMode::Inherit`].
    pub stderr: String,
    /// char's own deadline elapsed and char killed the group.
    pub timed_out: bool,
}

impl RunOutput {
    /// Whether the child exited zero.
    pub fn ok(&self) -> bool {
        self.code == Some(0)
    }
}

/// Why a child never started.
///
/// Typed rather than pre-classified, because **the same failure is a different
/// class depending on who asked**: `docker` missing from `PATH` is
/// `environment` — the machine is broken and the repo is fine — while a
/// `commands:` entry whose `cmd:` is not on `PATH` is `bad_config`, and the
/// caller is the only one that knows which it is.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SpawnError {
    /// `argv[0]`.
    pub program: String,
    /// What went wrong.
    pub kind: SpawnErrorKind,
    /// The operating system's account of it.
    pub message: String,
}

/// The distinctions a caller classifies on.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SpawnErrorKind {
    /// `argv[0]` is not on `PATH`.
    NotFound,
    /// It is there and not executable.
    PermissionDenied,
    /// Anything else: out of file descriptors, out of memory, a bad cwd.
    Other,
}

/// Every subprocess.
pub trait Run {
    /// Run to completion and report what happened.
    ///
    /// A child that could not be spawned at all is an `Err`; a child that ran
    /// and failed is an `Ok` carrying its code, because that is a result rather
    /// than an error — the distinction is the whole of PLAN.md §4.5's
    /// `data.dispatched`.
    fn call(&self, request: &RunRequest) -> Result<RunOutput, SpawnError>;

    /// Run to completion, calling `tick` every so often while waiting.
    ///
    /// This is how a **lease heartbeat survives a long child** without a
    /// background timer. PLAN.md §4.3 puts renewal in the loop that waits,
    /// precisely so that a wedged loop stops renewing and the existing
    /// cold-heartbeat path reclaims it — a background timer keeps ticking while
    /// the work is wedged, so the lease looks healthy forever and you need a
    /// TTL to catch it. `bundle install` takes minutes and a lease goes cold in
    /// one, so without this a second agent could take the run lease out from
    /// under a healthy `char init`.
    ///
    /// The default ignores the tick, which is right for a fake: nothing in a
    /// unit test is waiting on anything.
    fn call_with_tick(
        &self,
        request: &RunRequest,
        tick: &mut dyn FnMut(),
    ) -> Result<RunOutput, SpawnError> {
        let _ = tick;
        self.call(request)
    }
}

/// Time, in the three shapes char needs.
pub trait Clock {
    /// Wall clock, RFC 3339. Used for `claimed_at`, which is only ever
    /// displayed — never compared, because a backwards NTP step would make a
    /// live holder look stale (PLAN.md §4.3).
    fn wall_rfc3339(&self) -> String;

    /// Wall clock, milliseconds since the epoch.
    ///
    /// The one thing char needs the wall clock as a *number* for: a run id is
    /// time-ordered so that lexicographic order is chronological order, which is
    /// what makes run retention a sort of directory names rather than a stat of
    /// every one of them (PLAN.md §4.2). Monotonic will not do — those readings
    /// are meaningless across a reboot, and a run id has to sort against the ones
    /// already on disk from last week.
    ///
    /// Like [`Clock::wall_rfc3339`] it is never *compared* to decide whether
    /// something is stale; a backwards NTP step can reorder two run ids, which
    /// costs a retention decision and nothing else.
    fn wall_ms(&self) -> u64;

    /// Milliseconds on a **suspend-excluding** monotonic clock: `Instant`
    /// semantics, which pick `CLOCK_UPTIME_RAW` on darwin and
    /// `CLOCK_MONOTONIC` on Linux.
    ///
    /// The distinction is not academic. Measured, darwin's `CLOCK_MONOTONIC`
    /// counted 4.4 days of sleep on the machine this was written on — and the
    /// clock char wants is the one that does *not* advance while the machine
    /// is suspended, because the lease holder was not running either and its
    /// heartbeat should not age. Getting it backwards makes a live holder look
    /// arbitrarily cold after a laptop resumes, which is the
    /// two-workspaces-one-mutex outcome the lease design exists to prevent.
    fn mono(&self) -> u64;

    /// Sleep until the given monotonic reading. A no-op if it has passed.
    fn sleep_until(&self, mono_ms: u64);
}

/// `http` and `tcp` ready-checks (phase 4). Present on `Ctx` from the start so
/// the shape of the value does not change under every verb when they land.
pub trait Fetch {
    /// GET, and report the status code.
    fn http_status(&self, url: &str, timeout: Duration) -> Result<u16, ArmadaError>;
    /// Connect, and report whether anything answered.
    fn tcp_connect(&self, host: &str, port: u16, timeout: Duration) -> Result<bool, ArmadaError>;
}
