//! One server, on its own task: `run` first, then `serve` held, `ready`
//! polled, and the end.
//!
//! **Output goes to `observe_server`'s channel and never to `/events`**, which
//! carries the three lifecycle facts. `serve` writes straight into the log —
//! the file is the pipe — and a pass over it every [`api::FOLLOW`] offers the
//! whole lines it grew by.

use std::io::Write;
use std::path::{Path, PathBuf};
use std::time::Duration;

use adapter_traits::{AgentHarness, Delivery, Vcs, WorkProduct};
use checks_runner::{Served, Writing};
use core_model::{Component, Envelope, FieldValue, Level};
use ipc::{Event, ServerPhase, ServerState};
use tokio::sync::watch;
use verification::{Exit, NeverRan};

use super::held::Held;
use crate::daemon::Fleet;
use crate::rehearsing::records;

/// How often `ready` is asked — `crate::showing`'s interval for the evidence
/// harness's. **A command polled, never a duration waited out**: nothing here
/// decides a server is up because time passed.
const ASKING_EVERY: Duration = Duration::from_millis(250);

/// The most one ask of `ready` may take. A probe that hangs is ended and asked
/// again, rather than holding the server at `starting` for good.
const ONE_ASK: Duration = Duration::from_secs(30);

/// Everything the task needs, decided before it was spawned.
pub(crate) struct Plan {
    pub(crate) run: Option<String>,
    pub(crate) serve: String,
    pub(crate) ready: Option<String>,
    pub(crate) worktree: PathBuf,
    pub(crate) env: Vec<(String, String)>,
    pub(crate) dir: PathBuf,
    pub(crate) feed: api::RunFeed,
    pub(crate) now: watch::Sender<ServerState>,
}

struct End {
    exit: Exit,
    stopped: bool,
}

impl<H, V, W> Fleet<H, V, W>
where
    H: AgentHarness + Send + Sync + 'static,
    H::Error: std::error::Error + Send + Sync + 'static,
    V: Vcs + Delivery + Send + Sync + 'static,
    V::Error: std::error::Error + Send + Sync + 'static,
    V::CommitError: std::error::Error + Send + Sync + 'static,
    W: WorkProduct + Send + Sync + 'static,
    W::Error: std::error::Error + Send + Sync + 'static,
{
    /// The server, start to end. **Nothing here can fail the Job**: every
    /// failure is a fact on the instance.
    ///
    /// `server.exited` is published **before** the final state is sent, so a
    /// caller waiting on it — a Job's teardown — returns after the event is
    /// out, and the Job's own terminal event follows it rather than racing it.
    pub(crate) async fn served(&self, plan: Plan, held: Held, stopped: watch::Receiver<bool>) {
        let log = plan.dir.join(records::LOG);
        let (finished, finishing) = watch::channel(false);
        let lived = async {
            let end = self.lived(&plan, &log, stopped).await;
            let _ = finished.send(true);
            end
        };
        let (end, ()) = tokio::join!(lived, pumped(&plan.feed, &log, finishing));
        // Its group has been ended by now, on every road out of `lived`.
        super::left::forgotten(&plan.dir);
        let mut state = plan.now.borrow().clone();
        state.phase = ServerPhase::Exited;
        state.ended_at = Some(ipc::Instant::from(&self.now()));
        state.exit_code = match end.exit {
            Exit::Code(code) => Some(code),
            _ => None,
        };
        state.ended = Some(said(&end.exit, end.stopped));
        state.stopped = end.stopped;
        held.ended(state.clone());
        self.noted_server(&state);
        self.publish(Event::ServerExited(state.clone()));
        plan.now.send_replace(state);
    }

    async fn lived(&self, plan: &Plan, log: &Path, stopped: watch::Receiver<bool>) -> End {
        let path = plan.worktree.as_path();
        if let Some(run) = &plan.run {
            marked(log, &format!("--- `run` first: {run} ---"));
            let attempt = checks_runner::run_until(
                run,
                path,
                self.budget().duration(),
                Writing::Appending(log),
                &plan.env,
                until(stopped.clone()),
            )
            .await;
            if *stopped.borrow() {
                return End {
                    exit: attempt.exit,
                    stopped: true,
                };
            }
            if attempt.exit != Exit::Code(0) {
                return End {
                    exit: Exit::NeverRan(NeverRan::PrerequisiteFailed {
                        command: String::from("run"),
                        run: run.clone(),
                        exit: Box::new(attempt.exit),
                    }),
                    stopped: false,
                };
            }
        }
        marked(log, &format!("--- {} ---", plan.serve));
        let mut served = match Served::spawn_logging(&plan.serve, path, &plan.env, log) {
            Ok(served) => served,
            Err(why) => {
                return End {
                    exit: Exit::NeverRan(why),
                    stopped: false,
                }
            }
        };
        self.recorded_server(plan, &served).await;
        if let Some(ready) = &plan.ready {
            marked(log, &format!("--- waiting for `ready`: {ready} ---"));
            loop {
                let asked = async {
                    let answered = checks_runner::run_until(
                        ready,
                        path,
                        ONE_ASK,
                        Writing::Nowhere,
                        &plan.env,
                        std::future::pending(),
                    )
                    .await;
                    let up = answered.exit == Exit::Code(0);
                    if !up {
                        tokio::time::sleep(ASKING_EVERY).await;
                    }
                    up
                };
                // The server is watched while `ready` is asked, not only
                // between asks: one that dies on its first line is the
                // ordinary shape of a port conflict, and reads as a failure
                // at once rather than as a server still starting.
                tokio::select! {
                    exit = served.exited() => {
                        served.end().await;
                        return End { exit, stopped: false };
                    }
                    () = until(stopped.clone()) => {
                        served.end().await;
                        return End { exit: Exit::Signalled { signal: libc::SIGKILL }, stopped: true };
                    }
                    up = asked => if up { break; }
                }
            }
        }
        self.now_serving(plan);
        // The group is ended on both roads out, so a server that fell over
        // leaves no child of its own holding the port.
        tokio::select! {
            exit = served.exited() => {
                served.end().await;
                End { exit, stopped: false }
            }
            () = until(stopped) => {
                served.end().await;
                End { exit: Exit::Signalled { signal: libc::SIGKILL }, stopped: true }
            }
        }
    }

    /// What finds this server again if Fleet crashes before it ends —
    /// [`super::left`]. A start time that will not read leaves no record,
    /// because a record nothing can confirm is one startup must not act on.
    async fn recorded_server(&self, plan: &Plan, served: &Served) {
        let Some(group) = served.group() else {
            return;
        };
        let (id, whose) = {
            let state = plan.now.borrow();
            let whose = match &state.job_id {
                Some(job) => format!("job {}", job.as_str()),
                None => String::from("main-checkout"),
            };
            (state.id.clone(), whose)
        };
        let started = tokio::task::spawn_blocking(move || crate::process::holder_of(group)).await;
        if let Ok(Ok(crate::process::Holder::Held(started))) = started {
            let _ = super::left::recorded(&plan.dir, &id, &whose, group, &started);
        }
    }

    fn now_serving(&self, plan: &Plan) {
        let mut state = plan.now.borrow().clone();
        state.phase = ServerPhase::Serving;
        state.serving_since = Some(ipc::Instant::from(&self.now()));
        plan.now.send_replace(state.clone());
        self.publish(Event::ServerServing(state));
    }

    /// Write a Job's server's end into the Job's own log. **Fields, never an
    /// interpolated message**, for `crate::dry_run`'s reason. A server with no
    /// Job has no log of that kind to go in.
    fn noted_server(&self, state: &ServerState) {
        let Some(job) = state.job_id.as_ref().map(|id| id.to_domain()) else {
            return;
        };
        let level = match state.stopped {
            true => Level::Info,
            false => Level::Warn,
        };
        let mut envelope = Envelope::new(
            self.now(),
            level,
            Component::Fleet,
            self.run().clone(),
            "a server Fleet held for the Job ended",
        )
        .in_job(job.as_ulid().clone())
        .with_field("name", FieldValue::Str(state.name.clone()))
        .with_field("server", FieldValue::Str(state.id.clone()))
        .with_field("stopped", FieldValue::Bool(state.stopped));
        if let Some(code) = state.exit_code {
            envelope = envelope.with_field("exit_code", FieldValue::Int(i64::from(code)));
        }
        self.noted_in_the_log(&job, &envelope);
    }
}

/// Offer what the log has grown by, a pass every [`api::FOLLOW`], until the
/// server is over and its last line has gone out.
async fn pumped(feed: &api::RunFeed, log: &Path, mut finishing: watch::Receiver<bool>) {
    let mut from = 0u64;
    loop {
        let last = *finishing.borrow();
        if let Some((lines, next)) = records::lines_from(log, from, last) {
            from = next;
            feed.offer(api::RunChunk { lines });
        }
        if last {
            return;
        }
        tokio::select! {
            () = tokio::time::sleep(api::FOLLOW) => {}
            _ = finishing.changed() => {}
        }
    }
}

/// Resolves when something asks the server to stop. A stop that can no longer
/// arrive never resolves, so a server is never ended by its bookkeeping going.
async fn until(mut stopped: watch::Receiver<bool>) {
    if stopped.wait_for(|stop| *stop).await.is_err() {
        std::future::pending::<()>().await;
    }
}

/// A line of Fleet's own between commands, so one log reads as the sequence.
fn marked(log: &Path, line: &str) {
    if let Ok(mut file) = std::fs::OpenOptions::new().append(true).open(log) {
        let _ = writeln!(file, "{line}");
    }
}

/// How it ended, in a sentence. **A server that exits on its own has failed,
/// whatever its code**, so a zero is said as plainly as any other.
fn said(exit: &Exit, stopped: bool) -> String {
    if stopped {
        return String::from("was stopped");
    }
    match exit {
        Exit::Code(code) => {
            format!("stopped on its own, exit {code} — a server that exits has failed")
        }
        Exit::Signalled { signal } => format!("was ended by signal {signal} from outside Fleet"),
        Exit::TimedOut { after } => {
            format!(
                "its `run` ran past its {}s budget and was killed",
                after.as_secs()
            )
        }
        Exit::NeverRan(NeverRan::PrerequisiteFailed { run, exit, .. }) => {
            let how = match exit.as_ref() {
                Exit::Code(code) => format!("exited {code}"),
                other => said(other, false),
            };
            format!("never started: its `run`, `{run}`, {how}")
        }
        Exit::NeverRan(NeverRan::NothingToRun) => {
            String::from("declares a `serve` with no program in it")
        }
        Exit::NeverRan(NeverRan::NoSuchCommand { program }) => {
            format!("needs `{program}`, which is not on this machine's PATH")
        }
        Exit::NeverRan(NeverRan::WorktreeGone { worktree }) => {
            format!("could not start: {worktree} is not there")
        }
        Exit::NeverRan(NeverRan::NotSpawned { program, kind }) => {
            format!("could not start `{program}`: {kind}")
        }
    }
}
