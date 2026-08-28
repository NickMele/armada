//! Making the Judge's calls, and folding the answers into a veto.
//!
//! # Nothing here can advance a step
//!
//! [`judged`] answers `Option<Refusals>` — refused, or declined to refuse. It
//! has no return value meaning "approved", so the only thing the gate can do
//! with it is `Verdict::but_for`, which narrows.
//!
//! # A call that could not be made is not an answer
//!
//! Every failure — a program that is not there, a non-zero exit, a signal, an
//! expired budget, an empty answer, prose instead of a verdict — comes back as
//! [`CallFailed`] rather than as a verdict. A machine that cannot answer must
//! not produce one, in either direction.
//!
//! # Where it runs
//!
//! In the process's temporary directory, never the worktree. A `JudgeCall`
//! carries no directory, so the repository is not something the call declines
//! to open — it is somewhere the call is not.

use std::fmt;
use std::process::Stdio;
use std::sync::Arc;
use std::time::Duration;

use adapter_traits::{Ask, Environment, Model, ModelClient, Patch};
use core_model::{
    DeclaredPaths, GamingFlag, JudgeCheck, Judgment, RepoPath, ResolvedStep, StepCheck,
};
use tokio::io::AsyncWriteExt;
use tokio::process::Command;
use verification::{
    Baseline, Brief, Convergence, ConvergenceBrief, Flagged, GamingBrief, Refusals, Unreadable,
};

/// How long one Judge call may take before it is a failed call.
///
/// A newtype with one constructor and **no `Default`**, for [`CheckBudget`]'s
/// reason: `crates/config/settings.toml` names no Judge latency budget, so
/// there is no value to read and one invented here would be a threshold nobody
/// could find.
///
/// [`CheckBudget`]: crate::CheckBudget
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct JudgeBudget(Duration);

impl JudgeBudget {
    pub fn of(budget: Duration) -> JudgeBudget {
        JudgeBudget(budget)
    }

    pub fn duration(&self) -> Duration {
        self.0
    }
}

/// Everything one pass over a step needs in order to ask.
///
/// **The default model is a value rather than a literal.** Which model is
/// cheap is a vendor's fact and lives in `adapters`; what arrives here is a
/// name the composition root resolved.
#[derive(Clone)]
pub struct Judging {
    /// What renders a call. A pointer rather than a type parameter: the seam
    /// renders and cannot fail, so nothing about it needs to be generic.
    pub client: Arc<dyn ModelClient + Send + Sync>,
    pub budget: JudgeBudget,
    /// What a step naming no model of its own is judged by.
    pub default_model: Model,
    /// What the call's process holds. Fleet's own, because a Judge call
    /// authenticates as Fleet — the one place it differs from a Drone.
    pub environment: Environment,
}

/// Judge one step, and answer with the refusals or with none.
///
/// **Called only after the mechanical tier passed**, and only on a step that
/// declares a criterion. Both of those are the caller's to establish, which is
/// what makes the tier cold: this function costs money every time it is
/// entered.
pub(crate) async fn judged(
    step: &ResolvedStep,
    patch: &Patch,
    checks: &[StepCheck],
    judging: &Judging,
) -> Result<(Vec<Judgment>, Option<Refusals>), CallFailed> {
    let mut judgments = Vec::new();
    for check in step.judge_checks() {
        let model = model_for(check, &judging.default_model)?;
        for criterion in check.criteria() {
            let brief = Brief::about(step, criterion, patch, checks);
            // Every member of a panel answers the same brief and none of them
            // sees another's verdict — there is nothing in this loop that
            // carries one answer into the next call.
            for _ in 0..check.panel_size() {
                let ask = Ask::put(model.clone(), brief.question(), judging.environment.clone())
                    .map_err(|_| CallFailed::NothingToAsk)?;
                let said = said(judging.client.as_ref(), &ask, judging.budget).await?;
                judgments.push(brief.read(&said).map_err(CallFailed::Unreadable)?);
            }
        }
    }
    let refusals = Refusals::among(&judgments);
    Ok((judgments, refusals))
}

/// Look a second time, and answer with what was flagged or with nothing.
///
/// **Called only where the step would otherwise advance.** Gaming is what a
/// Mechanical Check passes by design, so this is the one place it can matter,
/// and a step already stopped by a Check or a refusal spends nothing here.
///
/// The mechanical half runs first and costs nothing. The judged half is one
/// call per declared pattern the diff cannot answer — **and no panel**, because
/// this check has no veto for a panel to make stricter.
pub(crate) async fn gaming(
    step: &ResolvedStep,
    patch: &Patch,
    baseline: Option<Baseline<'_>>,
    judging: &Judging,
) -> Result<Option<Flagged>, CallFailed> {
    let mut flags: Vec<GamingFlag> = Vec::new();
    for check in step.judge_checks() {
        let Some(gaming) = check.gaming().filter(|gaming| gaming.fires()) else {
            continue;
        };
        flags.extend(verification::in_the_diff(patch, gaming.flag_if()));
        let model = model_for(check, &judging.default_model)?;
        for pattern in verification::judged_patterns(gaming.flag_if()) {
            let Some(brief) = GamingBrief::about(step, pattern, patch, baseline) else {
                continue;
            };
            let ask = Ask::put(model.clone(), brief.question(), judging.environment.clone())
                .map_err(|_| CallFailed::NothingToAsk)?;
            let said = said(judging.client.as_ref(), &ask, judging.budget).await?;
            flags.extend(brief.read(&said).map_err(CallFailed::Unreadable)?);
        }
    }
    Ok(Flagged::among(flags))
}

/// Look part-way through a step, and answer with where the work stands.
///
/// **Called only once a mechanical tripwire fired**, which is the caller's to
/// establish — this function costs money every time it is entered, and a look
/// on a schedule is the design `docs/concepts/judge.md` rules out.
///
/// One call and **no panel**: the answer has no veto for a panel to make
/// stricter, and unanimity over three opinions about "is this going anywhere"
/// would fail loudly on a step that is merely slow.
pub(crate) async fn converging(
    step: &ResolvedStep,
    patch: &Patch,
    declared: Option<&DeclaredPaths>,
    off_plan: &[RepoPath],
    judging: &Judging,
) -> Result<Convergence, CallFailed> {
    let model = mid_step_model(step, &judging.default_model)?;
    let brief = ConvergenceBrief::about(step, patch, declared, off_plan);
    let ask = Ask::put(model, brief.question(), judging.environment.clone())
        .map_err(|_| CallFailed::NothingToAsk)?;
    let said = said(judging.client.as_ref(), &ask, judging.budget).await?;
    brief.read(&said).map_err(CallFailed::Unreadable)
}

/// Which model the mid-step look runs on.
///
/// The step's own dial where it declares one, so a step that pays for a
/// stronger judge at its gate is looked at by the same one part-way through.
/// A step declaring no Judge check at all still gets the look, on the default:
/// converging is Fleet's question rather than something a step opts into.
fn mid_step_model(step: &ResolvedStep, default: &Model) -> Result<Model, CallFailed> {
    match step.judge_checks().first() {
        Some(check) => model_for(check, default),
        None => Ok(default.clone()),
    }
}

/// The step's own model dial, or the fleet default where it names none.
fn model_for(check: &JudgeCheck, default: &Model) -> Result<Model, CallFailed> {
    match check.model() {
        Some(named) => Model::named(named.as_str()).map_err(|_| CallFailed::NothingToAsk),
        None => Ok(default.clone()),
    }
}

/// Run one rendered call and answer with what it printed.
///
/// `pub(crate)` because the Job proposer's call is the same call: one turn, one
/// question on stdin, a directory with no repository under it. A second runner
/// beside this one would be a second answer to what a failed call is.
pub(crate) async fn said(
    client: &(dyn ModelClient + Send + Sync),
    ask: &Ask,
    budget: JudgeBudget,
) -> Result<String, CallFailed> {
    let call = client.render(ask);
    let mut spawning = Command::new(call.program());
    spawning
        .args(call.args())
        .env_clear()
        .envs(call.environment().vars().iter().cloned())
        // Not the worktree, and not Fleet's own directory either.
        .current_dir(std::env::temp_dir())
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .kill_on_drop(true);
    spawning.process_group(0);

    let mut child = spawning.spawn().map_err(|error| CallFailed::NotStarted {
        program: call.program().to_string(),
        kind: error.kind(),
    })?;
    if let Some(mut stdin) = child.stdin.take() {
        // A failed write is a failed call: a model that was never given the
        // question cannot have answered it.
        stdin
            .write_all(call.question().as_bytes())
            .await
            .map_err(|error| CallFailed::NotAsked { kind: error.kind() })?;
        drop(stdin);
    }

    let ended = tokio::time::timeout(budget.duration(), child.wait_with_output()).await;
    let output = match ended {
        Ok(Ok(output)) => output,
        Ok(Err(error)) => return Err(CallFailed::NotAsked { kind: error.kind() }),
        Err(_) => return Err(CallFailed::TimedOut),
    };
    if !output.status.success() {
        return Err(CallFailed::Refused {
            code: output.status.code(),
        });
    }
    let said = String::from_utf8_lossy(&output.stdout).into_owned();
    match said.trim().is_empty() {
        true => Err(CallFailed::SaidNothing),
        false => Ok(said),
    }
}

/// Why the Judge did not answer. **Never a verdict** — see this module's
/// comment.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum CallFailed {
    /// The model name or the question was not one a call could be made from.
    NothingToAsk,
    /// The process would not start.
    NotStarted {
        program: String,
        kind: std::io::ErrorKind,
    },
    /// The question could not be delivered.
    NotAsked { kind: std::io::ErrorKind },
    /// It was still running when the budget expired. **The latency case**, and
    /// the one a person waiting at the gate feels.
    TimedOut,
    /// It ended badly — the network, the quota, an expired credential.
    Refused { code: Option<i32> },
    /// It ended well and printed nothing.
    SaidNothing,
    /// It answered something this cannot act on.
    Unreadable(Unreadable),
}

impl fmt::Display for CallFailed {
    fn fmt(&self, out: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            CallFailed::NothingToAsk => {
                out.write_str("the step names a model or a criterion a call cannot be made from")
            }
            CallFailed::NotStarted { program, kind } => {
                write!(out, "`{program}` would not start: {kind}")
            }
            CallFailed::NotAsked { kind } => {
                write!(out, "the question could not be delivered: {kind}")
            }
            CallFailed::TimedOut => out.write_str("the Judge did not answer inside its budget"),
            CallFailed::Refused { code: Some(code) } => {
                write!(out, "the Judge call ended with code {code}")
            }
            CallFailed::Refused { code: None } => out.write_str("a signal ended the Judge call"),
            CallFailed::SaidNothing => out.write_str("the Judge answered with nothing at all"),
            CallFailed::Unreadable(why) => write!(out, "{why}"),
        }
    }
}

impl std::error::Error for CallFailed {}
