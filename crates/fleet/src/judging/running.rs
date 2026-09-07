//! Running one rendered call: a child process, a pipe and a budget.
//!
//! **This half has never heard of a criterion.** It is handed something to say
//! and answers with what came back or with why nothing did, and every question
//! about what the answer means is above it.
//!
//! **Calls run in the process's temporary directory, never the worktree.** A
//! `JudgeCall` carries no directory, so the repository is not something a call
//! declines to open — it is somewhere the call is not. [`started`] is the one
//! place that confinement is applied, so a flag added to one runner is added to
//! both.

use std::process::Stdio;

use adapter_traits::{Ask, CallProgress, Heard, ModelClient};
use tokio::io::{AsyncBufReadExt, AsyncWriteExt};
use tokio::process::Command;

use super::{CallFailed, JudgeBudget};

/// Run one rendered call, reporting on it, and answer with what it said.
///
/// **[`said`] with somebody waiting.** Same confinement, same failures, same
/// budget; the call is rendered to print what it is doing and each line goes to
/// `telling` as it arrives.
///
/// **A second runner, and [`said`]'s note against one is kept rather than
/// broken**: every failure here is one of `said`'s, raised for the same
/// condition. What differs is the shape of the read — `wait_with_output`
/// collects a finished process, so nothing built on it can report on a running
/// one, and folding them would make the unwatched call pay for a line reader it
/// never uses.
///
/// `stopping` resolves when somebody asks the call to stop, and the process is
/// killed there rather than left to run out the budget with nobody waiting.
/// [`CallFailed::Stopped`](super::CallFailed::Stopped) is **not a fault** —
/// somebody decided.
pub(crate) async fn watched(
    client: &(dyn ModelClient + Send + Sync),
    ask: &Ask,
    budget: JudgeBudget,
    telling: &(dyn Fn(CallProgress) + Send + Sync),
    stopping: impl std::future::Future<Output = ()> + Send,
) -> Result<String, CallFailed> {
    let call = client.render_watched(ask);
    let mut child = started(&call)?;
    // Taken before the reader borrows the child, so the wait below owns what it
    // waits on. The pipe is `Stdio::piped()` on both runners; only this one
    // reads it a line at a time.
    let stdout = child.stdout.take();
    if let Some(mut stdin) = child.stdin.take() {
        stdin
            .write_all(call.question().as_bytes())
            .await
            .map_err(|error| CallFailed::NotAsked { kind: error.kind() })?;
        drop(stdin);
    }

    let reading = async {
        let mut said: Option<String> = None;
        if let Some(stdout) = stdout {
            let mut lines = tokio::io::BufReader::new(stdout).lines();
            // A read that fails ends the reading rather than the call: the
            // process is still what decides, and its exit status below is the
            // authority on whether this worked. Losing the tail of a stream
            // costs the progress surface and nothing else.
            while let Ok(Some(line)) = lines.next_line().await {
                match client.heard(&line) {
                    // **Last one wins, and there is only ever one.** A turn
                    // prints its `assistant` line once; taking the last rather
                    // than the first is what keeps a harness that reprints one
                    // from being read as two answers.
                    Heard::Answer(answer) => said = Some(answer),
                    Heard::Moved(progress) => telling(progress),
                    Heard::Nothing => {}
                }
            }
        }
        said
    };

    tokio::pin!(stopping);
    let said = tokio::select! {
        // **The stop wins over the budget and over the reading**, which is the
        // point of it: a person who has decided not to wait is not made to wait
        // out the rest of Fleet's budget. Dropping `child` kills the process,
        // and `kill_on_drop` is what makes that true.
        () = &mut stopping => return Err(CallFailed::Stopped),
        read = tokio::time::timeout(budget.duration(), reading) => match read {
            Ok(said) => said,
            Err(_) => return Err(CallFailed::TimedOut),
        },
    };

    // The stream is closed, so the process is finished or nearly. It is still
    // the exit status that decides, for `said`'s reason: a call that printed an
    // answer and then failed did not answer.
    let status = child
        .wait()
        .await
        .map_err(|error| CallFailed::NotAsked { kind: error.kind() })?;
    if !status.success() {
        return Err(CallFailed::Refused {
            code: status.code(),
        });
    }
    match said {
        Some(said) if !said.trim().is_empty() => Ok(said),
        _ => Err(CallFailed::SaidNothing),
    }
}

/// Start a rendered call. **The confinement, and the one place it is applied**
/// — both runners spawn through here, so a flag added to one is added to both
/// and neither can quietly run somewhere the other does not.
fn started(call: &adapter_traits::JudgeCall) -> Result<tokio::process::Child, CallFailed> {
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
    spawning.spawn().map_err(|error| CallFailed::NotStarted {
        program: call.program().to_string(),
        kind: error.kind(),
    })
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
    let mut child = started(&call)?;
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
