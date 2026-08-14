//! `armada manifest up` and `armada manifest down`, as a reducer
//! (`ARCHITECTURE.md` §1.2).
//!
//! ```text
//! step(State, Event) -> (State, Vec<Action>)
//! ```
//!
//! **`Event` and `Action` are matched exhaustively and there is never a `_ =>`
//! arm.** Adding a variant without handling it is `error[E0004]`, which is the
//! whole benefit; a catch-all converts that compile error into silence.
//! [`crate::schedule`] and [`crate::lease`] are the worked precedents, and the
//! membership below extends theirs rather than reinterpreting it.
//!
//! **The order is record → start → wait, and the first two may not swap.** This
//! is the inverse of `clean`'s order and both follow one rule: *the failure mode
//! must be a stale row, never an untracked resource* (PLAN.md §2.3.1).
//! Spawn-then-record leaks a pgid if Armada dies in between, and a leaked pgid
//! is the unreclaimable state the ownership store exists to prevent;
//! record-then-spawn leaves a row pointing at nothing, which the next `init`
//! reaps for free. So [`Action::Record`] is issued before [`Action::Start`] for
//! every service, unconditionally — including one whose ready-check is about to
//! fail, which is precisely the resource most likely to be broken and least
//! likely to be noticed.
//!
//! **A service is not up because it started; it is up when its ready-check
//! passes** ([`commands/manifest/up.md`]). Waiting is a poll loop the shell
//! performs and this module decides: [`Action::Probe`] then [`Action::Sleep`],
//! until the answer is yes or the service's own `timeout:` elapses — and a
//! ready-check that runs out of time reports `TIMEOUT`, never `FAILED`, because
//! a gate reading 1 goes looking for a broken service while reading 4 it raises
//! a deadline (PLAN.md §3.1).
//!
//! **Services are started one at a time, and that is a decision rather than an
//! oversight.** `needs:` already forces a dependency to be *ready* before its
//! dependent starts, so the only thing concurrency would buy is overlapping two
//! independent ready-checks. `check` is the verb with a cost budget, machine-wide
//! exclusives and a fifteen-minute spread; `up` has none of those, and a second
//! concurrent scheduler is a second place the deadlock in PHASES.md §11 can
//! hide. The reducer's shape admits it later without changing the events.
//!
//! [`commands/manifest/up.md`]: ../../../docs/commands/manifest/up.md

use crate::envelope::ResultRow;
use crate::error::{ArmadaError, ErrClass, Status};
use std::collections::BTreeMap;

/// How long the shell sleeps between two ready probes.
///
/// **A fixed interval rather than a backoff.** The thing being waited on is a
/// service coming up in seconds, and a backoff's whole value — being kind to a
/// remote that is rate-limiting you — does not apply to a `connect()` against
/// loopback. A fixed interval also makes the number of probes a function of the
/// elapsed time alone, which is what lets a test assert one.
pub const PROBE_INTERVAL_MS: u64 = 250;

/// What a row says when a signal reached the run before it did.
const INTERRUPTED: &str = "interrupted before it was started";

/// Whether a `log:` ready-check has matched yet.
///
/// **Unanchored, over the whole file, and a `bad_config` when the pattern will
/// not compile** (PLAN.md §6.0). The pattern is the repo's, so a broken one is
/// the repo's to fix — and a probe that silently never matches would burn the
/// whole `ready.timeout:` and then report `TIMEOUT`, sending the reader to look
/// at a service that came up fine.
pub fn log_matched(
    text: &str,
    pattern: &str,
    at: crate::error::ConfigWhere,
) -> Result<bool, ArmadaError> {
    let regex = regex::Regex::new(pattern).map_err(|e| {
        ArmadaError::bad_config(
            at,
            format!("`ready.log:` is not a regex: {e}"),
            "correct the pattern, or use `ready: {tcp: <port-name>}`",
        )
    })?;
    Ok(regex.is_match(text))
}

/// Which verb is driving.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Direction {
    /// Start, and wait on the ready-check.
    Up,
    /// Stop, and keep the port block.
    Down,
}

impl Direction {
    /// The state a row reaches when it worked.
    const fn success(self) -> Status {
        match self {
            Direction::Up => Status::Up,
            Direction::Down => Status::Down,
        }
    }

    /// What `results[]` holds, for the aggregate's message.
    pub const fn subject(self) -> &'static str {
        "services"
    }
}

/// Where one service has got to.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Phase {
    /// Not touched yet.
    Pending,
    /// [`Action::Record`] is out; the store has not answered.
    Recording,
    /// [`Action::Start`] or [`Action::Stop`] is out.
    Working,
    /// Started, and the ready-check has not passed yet.
    Waiting,
    /// Terminal.
    Settled,
}

/// One service's row, as the reducer holds it.
#[derive(Debug, Clone, PartialEq)]
pub struct Row {
    /// Where it has got to.
    pub phase: Phase,
    /// Its state once [`Phase::Settled`].
    pub status: Status,
    /// When work on it began, on the monotonic clock.
    pub began_mono: Option<u64>,
    /// When its ready-check started being asked.
    pub waiting_since: Option<u64>,
    /// Its ready-check's own deadline, in seconds.
    pub ready_timeout: u32,
    /// The ids Armada holds for it — `container:…`, `pgid:…`.
    pub owns: Vec<String>,
    /// Its own failure.
    pub error: Option<ArmadaError>,
    /// Prose the status alone does not carry.
    pub reason: Option<String>,
    /// What Armada executed for it.
    pub argv: Vec<String>,
    /// Where its output went.
    pub log: Option<String>,
}

impl Row {
    fn new(ready_timeout: u32) -> Self {
        Row {
            phase: Phase::Pending,
            status: Status::Skipped,
            began_mono: None,
            waiting_since: None,
            ready_timeout,
            owns: Vec::new(),
            error: None,
            reason: None,
            argv: Vec::new(),
            log: None,
        }
    }
}

/// What the shell learned when it performed an [`Action::Start`] or an
/// [`Action::Stop`]: the handles, the vector, and where the output went.
///
/// **One value rather than three fields on two events**, because every one of
/// them is known at the same moment and none of them can be re-derived
/// afterwards — `${port.NAME}` has already been substituted by then, so the
/// config no longer says what ran.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct Attempt {
    /// `<kind>:<reference>`, the grammar `results[].owns[]` uses.
    pub owns: Vec<String>,
    /// The exact vector, post-substitution.
    pub argv: Vec<String>,
    /// Workspace-relative path to the service's log, when it has one.
    pub log: Option<String>,
    /// The ready-check this service is about to be waited on, in words —
    /// `http http://127.0.0.1:5460/healthz`.
    ///
    /// **The row's `reason` for a service that worked**, because
    /// [`commands/manifest/up.md`] asks the payload to name *"the ready-check
    /// that was waited on"* and a row that says only `UP` cannot answer *what
    /// was it waiting for* — which is the first question asked of a `TIMEOUT`.
    ///
    /// [`commands/manifest/up.md`]: ../../../docs/commands/manifest/up.md
    pub ready: Option<String>,
}

/// One settled service, in the shape `data.results[]` wants.
#[derive(Debug, Clone, PartialEq)]
pub struct ServiceResult {
    /// The component name — `results[].id`'s grammar for `up` and `down`.
    pub id: String,
    /// Terminal.
    pub status: Status,
    /// How long Armada spent on it, ready-wait included.
    pub duration_ms: Option<u64>,
    /// What Armada holds for it, by id.
    pub owns: Vec<String>,
    /// Its own failure.
    pub error: Option<ArmadaError>,
    /// Prose the status alone does not carry — for `up`, the ready-check that
    /// was waited on.
    pub reason: Option<String>,
    /// What Armada actually executed for it, post-substitution.
    pub argv: Vec<String>,
    /// Where its output went, for a service the shell gave a log file.
    pub log: Option<String>,
}

impl From<&ServiceResult> for ResultRow {
    fn from(result: &ServiceResult) -> Self {
        let mut row = ResultRow::new(result.id.clone(), result.status);
        row.duration_ms = result.duration_ms;
        row.owns = result.owns.clone();
        row.error = result.error.clone();
        row.reason = result.reason.clone();
        row.argv = result.argv.clone();
        row.log = result.log.clone();
        row
    }
}

/// The run: the services in order, where each has got to, and the clock.
///
/// **Owned and returned, never mutated through a `&mut`** — a reducer that can
/// be called for its side effects is how the pure core starts leaking into the
/// shell (`ARCHITECTURE.md` §1.2).
#[derive(Debug, Clone, PartialEq)]
pub struct State {
    /// Which verb.
    pub direction: Direction,
    /// The services, in the order they are acted on.
    pub order: Vec<String>,
    /// Each service's component dependencies, for the cascade.
    pub needs: BTreeMap<String, Vec<String>>,
    /// Each service's row.
    pub rows: BTreeMap<String, Row>,
    /// How far along [`State::order`] the run is.
    pub cursor: usize,
    /// The last monotonic reading the shell reported.
    pub now_mono: u64,
    /// Whether a signal ended the run early.
    pub interrupted: bool,
    /// Whether [`Action::Finish`] has been issued.
    pub finished: bool,
}

impl State {
    /// A run over these services, in this order.
    ///
    /// `ready_timeouts` is per service and in seconds; a service missing from it
    /// takes `ready: {none: true}`'s zero, which settles on the first probe.
    pub fn new(
        direction: Direction,
        order: Vec<String>,
        needs: BTreeMap<String, Vec<String>>,
        ready_timeouts: &BTreeMap<String, u32>,
    ) -> Self {
        let rows = order
            .iter()
            .map(|name| {
                (
                    name.clone(),
                    Row::new(ready_timeouts.get(name).copied().unwrap_or_default()),
                )
            })
            .collect();
        State {
            direction,
            order,
            needs,
            rows,
            cursor: 0,
            now_mono: 0,
            interrupted: false,
            finished: false,
        }
    }

    /// The service the run is on, if any remain.
    fn current(&self) -> Option<String> {
        self.order.get(self.cursor).cloned()
    }

    /// Whether every dependency of `name` reached the success state.
    ///
    /// **The cascade, and it is the reason `needs:` is held in state at all.**
    /// Starting `api` against a `postgres` that never came up produces a service
    /// that fails for a reason two levels away from its own logs.
    fn blocked_by(&self, name: &str) -> Option<String> {
        self.needs.get(name)?.iter().find_map(|need| {
            let row = self.rows.get(need)?;
            (row.phase == Phase::Settled && row.status != self.direction.success())
                .then(|| need.clone())
        })
    }
}

/// What the shell tells the reducer happened.
#[derive(Debug, Clone, PartialEq)]
pub enum Event {
    /// The run begins.
    Started,
    /// The intent row is in `manifest.db`. **Before anything exists** — that is
    /// the ordering this whole module is written around.
    Recorded {
        /// Whose row.
        service: String,
    },
    /// The store refused. Nothing has been created, so nothing is stranded —
    /// but Armada must not create anything it could not record either.
    RecordFailed {
        /// Whose row.
        service: String,
        /// What the store said.
        error: ArmadaError,
    },
    /// It exists, and this is everything the shell learned starting it.
    Spawned {
        /// Whose service.
        service: String,
        /// The handles, the argv, and the log.
        attempt: Attempt,
    },
    /// It never started.
    ///
    /// **It still carries an [`Attempt`]**, because a driver can create some of
    /// what it was asked for and then fail — a `compose up` that started two
    /// containers and could not start the third. Those two are Armada's, and a
    /// failure that dropped their ids on the floor would strand them.
    SpawnFailed {
        /// Whose service.
        service: String,
        /// Whatever was created before it failed.
        attempt: Attempt,
        /// **The class the shell decided**, because the same failure is a
        /// different class depending on who asked: a missing `docker` is
        /// `environment`, a `cmd:` that is not on `PATH` is `bad_config`.
        error: ArmadaError,
    },
    /// The ready-check answered yes.
    Ready {
        /// Whose check.
        service: String,
    },
    /// The ready-check answered no. Ordinary, and the reason a deadline exists.
    NotReady {
        /// Whose check.
        service: String,
    },
    /// The ready-check itself could not be asked — a malformed URL, a probe
    /// command that is not on `PATH`. Distinct from [`Event::NotReady`],
    /// because waiting out a deadline for a question Armada cannot ask is time
    /// spent learning nothing.
    ReadyFailed {
        /// Whose check.
        service: String,
        /// What went wrong.
        error: ArmadaError,
    },
    /// The service is stopped and **confirmed gone**. Not "the signal was
    /// sent": `down` reports `DOWN` only once the group is confirmed empty.
    Stopped {
        /// Whose service.
        service: String,
        /// What the shell ran to stop it.
        attempt: Attempt,
    },
    /// It would not stop. **A real leak**, so it fails the row.
    StopFailed {
        /// Whose service.
        service: String,
        /// What survived, and how Armada knows.
        error: ArmadaError,
    },
    /// Time passed. The one variant `ARCHITECTURE.md` §1.2 licenses: a pure
    /// reducer cannot call the clock, so `now` is carried on an event.
    Tick {
        /// A suspend-excluding monotonic reading, in milliseconds.
        now_mono: u64,
    },
    /// SIGINT.
    Interrupted,
}

/// What the reducer asks the shell to do.
#[derive(Debug, Clone, PartialEq)]
pub enum Action {
    /// Write the intent row into `manifest.db`, **before the resource exists**.
    Record {
        /// Whose row.
        service: String,
    },
    /// Start it, through whichever driver it declares.
    Start {
        /// Whose service.
        service: String,
    },
    /// Ask its ready-check once. **Once**: the loop is here, so a probe that
    /// blocks is bounded by its own timeout rather than by a nested retry the
    /// reducer cannot see.
    Probe {
        /// Whose check.
        service: String,
    },
    /// Stop it: the driver's `stop:`, or killpg — TERM, grace, KILL.
    Stop {
        /// Whose service.
        service: String,
    },
    /// Nothing can happen before this monotonic reading.
    Sleep {
        /// The wake-up time.
        until_mono: u64,
    },
    /// Put this row into `data.results[]`.
    Emit {
        /// The row.
        result: ServiceResult,
    },
    /// The verb is over.
    Finish {
        /// Its terminal state.
        status: Status,
    },
}

/// The service an event is about, when it is about one.
///
/// Used for one thing: **an event about a row that has already settled is
/// ignored.** The shell may report a `Recorded` for a service the interrupt
/// path settled a moment earlier, and re-settling it would emit a second row
/// for one service.
const fn subject_of(event: &Event) -> Option<&String> {
    match event {
        Event::Recorded { service }
        | Event::RecordFailed { service, .. }
        | Event::Spawned { service, .. }
        | Event::SpawnFailed { service, .. }
        | Event::Ready { service }
        | Event::NotReady { service }
        | Event::ReadyFailed { service, .. }
        | Event::Stopped { service, .. }
        | Event::StopFailed { service, .. } => Some(service),
        Event::Started | Event::Tick { .. } | Event::Interrupted => None,
    }
}

/// One transition.
pub fn step(mut state: State, event: Event) -> (State, Vec<Action>) {
    let mut actions = Vec::new();

    if let Some(service) = subject_of(&event) {
        if state.rows.get(service).map(|row| &row.phase) == Some(&Phase::Settled) {
            advance(&mut state, &mut actions);
            return (state, actions);
        }
    }

    match event {
        Event::Started => {}
        Event::Tick { now_mono } => {
            state.now_mono = now_mono;
            if let Some(service) = state.current() {
                if let Some(row) = state.rows.get(&service) {
                    if row.phase == Phase::Waiting {
                        let waited = now_mono.saturating_sub(row.waiting_since.unwrap_or(now_mono));
                        if waited >= row.ready_timeout as u64 * 1_000 {
                            // **`TIMEOUT`, never `FAILED`.** A gate reading 1
                            // goes looking for a broken service; reading 4 it
                            // raises a deadline or asks why startup got slow.
                            let seconds = row.ready_timeout;
                            settle(
                                &mut state,
                                &service,
                                Status::Timeout,
                                Some(ArmadaError {
                                    class: ErrClass::Timeout,
                                    r#where: service.clone(),
                                    message: format!(
                                        "the ready-check did not pass within {seconds}s"
                                    ),
                                    next_action: Some(
                                        "raise `ready.timeout:`, or read the service's log"
                                            .to_string(),
                                    ),
                                }),
                                None,
                                &mut actions,
                            );
                        }
                    }
                }
            }
        }
        Event::Recorded { service } => {
            if let Some(row) = state.rows.get_mut(&service) {
                row.phase = Phase::Working;
            }
            actions.push(match state.direction {
                Direction::Up => Action::Start {
                    service: service.clone(),
                },
                Direction::Down => Action::Stop {
                    service: service.clone(),
                },
            });
            return (state, actions);
        }
        Event::RecordFailed { service, error } => {
            settle(
                &mut state,
                &service,
                Status::Failed,
                Some(error),
                None,
                &mut actions,
            );
        }
        Event::Spawned { service, attempt } => {
            if let Some(row) = state.rows.get_mut(&service) {
                record_attempt(row, attempt);
                row.phase = Phase::Waiting;
                row.waiting_since = Some(state.now_mono);
            }
            actions.push(Action::Probe {
                service: service.clone(),
            });
            return (state, actions);
        }
        Event::SpawnFailed {
            service,
            attempt,
            error,
        } => {
            if let Some(row) = state.rows.get_mut(&service) {
                record_attempt(row, attempt);
            }
            settle(
                &mut state,
                &service,
                Status::Failed,
                Some(error),
                None,
                &mut actions,
            );
        }
        Event::Ready { service } => {
            let status = state.direction.success();
            settle(&mut state, &service, status, None, None, &mut actions);
        }
        Event::NotReady { service } => {
            // The deadline is checked on `Tick`, so a `NotReady` only ever asks
            // for another turn of the loop. One place decides that a wait is
            // over, which is what stops two of them disagreeing.
            actions.push(Action::Sleep {
                until_mono: state.now_mono.saturating_add(PROBE_INTERVAL_MS),
            });
            let _ = &service;
            return (state, actions);
        }
        Event::ReadyFailed { service, error } => {
            settle(
                &mut state,
                &service,
                Status::Failed,
                Some(error),
                None,
                &mut actions,
            );
        }
        Event::Stopped { service, attempt } => {
            if let Some(row) = state.rows.get_mut(&service) {
                record_attempt(row, attempt);
            }
            let status = state.direction.success();
            settle(&mut state, &service, status, None, None, &mut actions);
        }
        Event::StopFailed { service, error } => {
            settle(
                &mut state,
                &service,
                Status::Failed,
                Some(error),
                None,
                &mut actions,
            );
        }
        Event::Interrupted => {
            state.interrupted = true;
            // **A row whose `Record` is still out settles here, and one whose
            // `Start` is out does not.** Nothing has been created for the
            // first, so the worst it leaves is a row pointing at nothing —
            // which the next `init` reaps for free, and is the direction this
            // module always errs in. The second may already exist, so it is
            // left to the event the shell is about to deliver.
            let recording: Vec<String> = state
                .rows
                .iter()
                .filter(|(_, row)| row.phase == Phase::Recording)
                .map(|(name, _)| name.clone())
                .collect();
            for service in recording {
                settle(
                    &mut state,
                    &service,
                    Status::Skipped,
                    None,
                    Some(INTERRUPTED.to_string()),
                    &mut actions,
                );
            }
        }
    }

    advance(&mut state, &mut actions);
    (state, actions)
}

/// Move to the next service that has work, and issue the action it needs.
fn advance(state: &mut State, actions: &mut Vec<Action>) {
    loop {
        let Some(service) = state.current() else {
            finish(state, actions);
            return;
        };
        let Some(row) = state.rows.get(&service) else {
            state.cursor += 1;
            continue;
        };

        match row.phase {
            // Something is out with the shell; nothing to propose.
            Phase::Recording | Phase::Working => return,
            // The wait continues, and `Tick` is what ends it.
            Phase::Waiting => {
                actions.push(Action::Probe { service });
                return;
            }
            Phase::Settled => {
                state.cursor += 1;
                continue;
            }
            Phase::Pending => {}
        }

        if state.interrupted {
            settle(
                state,
                &service,
                Status::Skipped,
                None,
                Some(INTERRUPTED.to_string()),
                actions,
            );
            state.cursor += 1;
            continue;
        }

        if let Some(blocker) = state.blocked_by(&service) {
            let verb = match state.direction {
                Direction::Up => "start",
                Direction::Down => "stop",
            };
            settle(
                state,
                &service,
                Status::Skipped,
                None,
                Some(format!("`{blocker}` did not {verb}")),
                actions,
            );
            state.cursor += 1;
            continue;
        }

        // **Record first, unconditionally.** The failure mode must be a stale
        // row, never an untracked resource.
        if let Some(row) = state.rows.get_mut(&service) {
            row.phase = Phase::Recording;
            row.began_mono = Some(state.now_mono);
        }
        actions.push(Action::Record { service });
        return;
    }
}

/// Fold what the shell learned into the row.
///
/// **Handles accumulate rather than replace.** `up` and `down` both touch a row
/// once, but a driver that reports twice must not lose the first set — the ids
/// are the whole reclaimability guarantee.
fn record_attempt(row: &mut Row, attempt: Attempt) {
    for id in attempt.owns {
        if !row.owns.contains(&id) {
            row.owns.push(id);
        }
    }
    if !attempt.argv.is_empty() {
        row.argv = attempt.argv;
    }
    if attempt.log.is_some() {
        row.log = attempt.log;
    }
    if attempt.ready.is_some() {
        row.reason = attempt.ready;
    }
}

/// Settle one row and emit it.
fn settle(
    state: &mut State,
    service: &str,
    status: Status,
    error: Option<ArmadaError>,
    reason: Option<String>,
    actions: &mut Vec<Action>,
) {
    let Some(row) = state.rows.get_mut(service) else {
        return;
    };
    row.phase = Phase::Settled;
    row.status = status;
    row.error = error;
    // A cascade or an interrupt says why this row never ran, and that outranks
    // the ready-check it would have waited on. Anything else leaves the
    // ready-check in place — it is what `up.md` asks the payload to name.
    if reason.is_some() {
        row.reason = reason;
    }

    let duration_ms = row
        .began_mono
        .map(|began| state.now_mono.saturating_sub(began));
    let result = ServiceResult {
        id: service.to_string(),
        status,
        duration_ms,
        owns: row.owns.clone(),
        error: row.error.clone(),
        reason: row.reason.clone(),
        argv: row.argv.clone(),
        log: row.log.clone(),
    };
    actions.push(Action::Emit { result });
}

fn finish(state: &mut State, actions: &mut Vec<Action>) {
    if state.finished {
        return;
    }
    state.finished = true;
    actions.push(Action::Finish {
        status: verdict(state),
    });
}

/// The verb's terminal state, over the rows.
///
/// **The terminal state and the error class are two axes.** This decides *what
/// happened*; [`crate::envelope::aggregate`] decides *why*, and the exit code
/// follows the class and never the state (PLAN.md §3.1).
pub fn verdict(state: &State) -> Status {
    let rows: Vec<&Row> = state.rows.values().collect();
    let success = state.direction.success();

    // `up` in a workspace that declares no services has nothing to do. Exit 0,
    // and **not** `PARTIAL` (PLAN.md §3).
    if rows.is_empty() {
        return match state.direction {
            Direction::Up => Status::Skipped,
            // `down`'s contract has no `SKIPPED`, and stopping nothing is a
            // workspace whose services are all stopped.
            Direction::Down => Status::Down,
        };
    }

    let good = rows.iter().filter(|row| row.status == success).count();
    if good == rows.len() {
        return success;
    }
    // Nothing was attempted — every row cascaded or was interrupted before it
    // began. Claiming a failure for work that never ran is the thing
    // `SKIPPED` exists to avoid.
    if rows.iter().all(|row| row.status == Status::Skipped) {
        return match state.direction {
            Direction::Up => Status::Skipped,
            Direction::Down => Status::Down,
        };
    }
    if good > 0 {
        return Status::Partial;
    }
    // Nothing worked. A run whose only real failures are deadlines says so, so
    // a caller raises a timeout rather than hunting a broken service.
    let timed_out = rows.iter().any(|row| row.status == Status::Timeout);
    let failed = rows.iter().any(|row| row.status == Status::Failed);
    match (timed_out, failed) {
        (true, false) => Status::Timeout,
        _ => Status::Failed,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn needs(pairs: &[(&str, &[&str])]) -> BTreeMap<String, Vec<String>> {
        pairs
            .iter()
            .map(|(name, needs)| {
                (
                    (*name).to_string(),
                    needs.iter().map(|n| (*n).to_string()).collect(),
                )
            })
            .collect()
    }

    fn timeouts(pairs: &[(&str, u32)]) -> BTreeMap<String, u32> {
        pairs
            .iter()
            .map(|(name, seconds)| ((*name).to_string(), *seconds))
            .collect()
    }

    fn order(names: &[&str]) -> Vec<String> {
        names.iter().map(|n| (*n).to_string()).collect()
    }

    fn up(names: &[&str]) -> State {
        State::new(
            Direction::Up,
            order(names),
            needs(&[]),
            &timeouts(&names.iter().map(|n| (*n, 60)).collect::<Vec<_>>()),
        )
    }

    /// Drive one event and hand back what came out.
    fn drive(state: State, event: Event) -> (State, Vec<Action>) {
        step(state, event)
    }

    /// **The one ordering this module exists for.** A container started and not
    /// recorded is unreclaimable, and that is the single failure mode the
    /// design refuses — so `Record` precedes `Start` for every service, before
    /// anything is known about whether it will work.
    #[test]
    fn the_row_is_recorded_before_anything_is_started() {
        let (state, actions) = drive(up(&["db"]), Event::Started);
        assert_eq!(
            actions,
            vec![Action::Record {
                service: "db".to_string()
            }],
            "something was started before it was recorded"
        );

        let (_, actions) = drive(
            state,
            Event::Recorded {
                service: "db".to_string(),
            },
        );
        assert_eq!(
            actions,
            vec![Action::Start {
                service: "db".to_string()
            }]
        );
    }

    /// A store that refused is a service Armada must not create: it could not
    /// record it, so it could never reclaim it.
    #[test]
    fn a_row_that_could_not_be_recorded_is_never_started() {
        let (state, _) = drive(up(&["db"]), Event::Started);
        let (state, actions) = drive(
            state,
            Event::RecordFailed {
                service: "db".to_string(),
                error: ArmadaError {
                    class: ErrClass::Environment,
                    r#where: "manifest.db".to_string(),
                    message: "the store is locked".to_string(),
                    next_action: None,
                },
            },
        );
        assert!(
            !actions
                .iter()
                .any(|action| matches!(action, Action::Start { .. })),
            "{actions:?}"
        );
        assert_eq!(state.rows["db"].status, Status::Failed);
    }

    /// **Started is not ready.** The row settles on the ready-check and not on
    /// the spawn, which is the difference between a caller that races and one
    /// that does not.
    #[test]
    fn a_service_settles_on_its_ready_check_and_not_on_its_spawn() {
        let (state, _) = drive(up(&["db"]), Event::Started);
        let (state, _) = drive(
            state,
            Event::Recorded {
                service: "db".to_string(),
            },
        );
        let (state, actions) = drive(
            state,
            Event::Spawned {
                service: "db".to_string(),
                attempt: Attempt {
                    owns: vec!["container:armada-a3f91c02-db-1".to_string()],
                    ..Attempt::default()
                },
            },
        );
        assert_eq!(
            actions,
            vec![Action::Probe {
                service: "db".to_string()
            }]
        );
        assert_eq!(state.rows["db"].phase, Phase::Waiting);

        let (state, actions) = drive(
            state,
            Event::NotReady {
                service: "db".to_string(),
            },
        );
        assert!(
            matches!(actions.as_slice(), [Action::Sleep { .. }]),
            "{actions:?}"
        );

        let (state, actions) = drive(
            state,
            Event::Ready {
                service: "db".to_string(),
            },
        );
        assert_eq!(state.rows["db"].status, Status::Up);
        assert!(actions.iter().any(|a| matches!(a, Action::Emit { .. })));
        assert!(actions.contains(&Action::Finish { status: Status::Up }));
    }

    /// **The handles are held from the moment it starts**, not from the moment
    /// it works: a container that starts and then fails its ready-check is
    /// still owned, and therefore still reclaimable.
    #[test]
    fn a_service_that_failed_its_ready_check_still_reports_what_it_holds() {
        let mut state = up(&["db"]);
        state = drive(state, Event::Started).0;
        state = drive(
            state,
            Event::Recorded {
                service: "db".to_string(),
            },
        )
        .0;
        state = drive(
            state,
            Event::Spawned {
                service: "db".to_string(),
                attempt: Attempt {
                    owns: vec!["container:armada-a3f91c02-db-1".to_string()],
                    ..Attempt::default()
                },
            },
        )
        .0;
        let (_, actions) = drive(
            state,
            Event::ReadyFailed {
                service: "db".to_string(),
                error: ArmadaError {
                    class: ErrClass::ToolFailed,
                    r#where: "db".to_string(),
                    message: "pg_isready is not on PATH".to_string(),
                    next_action: None,
                },
            },
        );
        let Some(Action::Emit { result }) = actions
            .iter()
            .find(|a| matches!(a, Action::Emit { .. }))
            .cloned()
        else {
            panic!("no row was emitted: {actions:?}")
        };
        assert_eq!(result.status, Status::Failed);
        assert_eq!(result.owns, vec!["container:armada-a3f91c02-db-1"]);
    }

    /// **`TIMEOUT` is not `FAILED`.** A gate reading exit 1 goes looking for a
    /// broken service; reading 4 it raises a deadline.
    #[test]
    fn a_ready_check_that_runs_out_of_time_reports_timeout_and_not_failed() {
        let mut state = State::new(
            Direction::Up,
            order(&["db"]),
            needs(&[]),
            &timeouts(&[("db", 60)]),
        );
        state = drive(state, Event::Started).0;
        state = drive(
            state,
            Event::Recorded {
                service: "db".to_string(),
            },
        )
        .0;
        state = drive(
            state,
            Event::Spawned {
                service: "db".to_string(),
                attempt: Attempt::default(),
            },
        )
        .0;

        // One tick inside the deadline changes nothing.
        let (state, actions) = drive(state, Event::Tick { now_mono: 59_000 });
        assert!(
            !actions.iter().any(|a| matches!(a, Action::Emit { .. })),
            "settled early: {actions:?}"
        );

        let (state, actions) = drive(state, Event::Tick { now_mono: 60_001 });
        assert_eq!(state.rows["db"].status, Status::Timeout);
        assert_eq!(
            state.rows["db"].error.as_ref().unwrap().class,
            ErrClass::Timeout
        );
        assert_eq!(
            state.rows["db"].error.as_ref().unwrap().class.exit_code(),
            4,
            "not 1"
        );
        assert!(actions.contains(&Action::Finish {
            status: Status::Timeout
        }));
    }

    /// A ready-check inherits `ready: {none: true}`'s zero, so a
    /// fire-and-forget service settles on the first turn rather than waiting.
    #[test]
    fn a_service_with_no_ready_check_is_up_as_soon_as_it_starts() {
        let mut state = State::new(
            Direction::Up,
            order(&["fire"]),
            needs(&[]),
            &timeouts(&[("fire", 0)]),
        );
        state = drive(state, Event::Started).0;
        state = drive(
            state,
            Event::Recorded {
                service: "fire".to_string(),
            },
        )
        .0;
        state = drive(
            state,
            Event::Spawned {
                service: "fire".to_string(),
                attempt: Attempt::default(),
            },
        )
        .0;
        let (state, _) = drive(
            state,
            Event::Ready {
                service: "fire".to_string(),
            },
        );
        assert_eq!(state.rows["fire"].status, Status::Up);
    }

    /// **The cascade.** Starting `api` against a `postgres` that never came up
    /// produces a service that fails for a reason two levels from its own logs.
    #[test]
    fn a_service_whose_dependency_failed_is_skipped_and_says_which_one() {
        let mut state = State::new(
            Direction::Up,
            order(&["db", "api"]),
            needs(&[("api", &["db"])]),
            &timeouts(&[("db", 60), ("api", 60)]),
        );
        state = drive(state, Event::Started).0;
        state = drive(
            state,
            Event::Recorded {
                service: "db".to_string(),
            },
        )
        .0;
        let (state, actions) = drive(
            state,
            Event::SpawnFailed {
                service: "db".to_string(),
                attempt: Attempt::default(),
                error: ArmadaError {
                    class: ErrClass::ToolFailed,
                    r#where: "db".to_string(),
                    message: "compose exited 1".to_string(),
                    next_action: None,
                },
            },
        );

        assert_eq!(state.rows["api"].status, Status::Skipped);
        assert_eq!(
            state.rows["api"].reason.as_deref(),
            Some("`db` did not start")
        );
        assert!(
            !actions
                .iter()
                .any(|a| matches!(a, Action::Record { service } if service == "api")),
            "a blocked service was recorded and started anyway: {actions:?}"
        );
        assert!(actions.contains(&Action::Finish {
            status: Status::Failed
        }));
    }

    /// Three of five worked and nothing worked demand different actions, and
    /// would otherwise both read `FAILED` (PLAN.md §3.1).
    #[test]
    fn some_up_and_some_not_is_partial() {
        let mut state = up(&["a", "b"]);
        state = drive(state, Event::Started).0;
        state = drive(
            state,
            Event::Recorded {
                service: "a".to_string(),
            },
        )
        .0;
        state = drive(
            state,
            Event::Spawned {
                service: "a".to_string(),
                attempt: Attempt::default(),
            },
        )
        .0;
        state = drive(
            state,
            Event::Ready {
                service: "a".to_string(),
            },
        )
        .0;
        state = drive(
            state,
            Event::Recorded {
                service: "b".to_string(),
            },
        )
        .0;
        let (state, actions) = drive(
            state,
            Event::SpawnFailed {
                service: "b".to_string(),
                attempt: Attempt::default(),
                error: ArmadaError {
                    class: ErrClass::ToolFailed,
                    r#where: "b".to_string(),
                    message: "no".to_string(),
                    next_action: None,
                },
            },
        );
        assert_eq!(verdict(&state), Status::Partial);
        assert!(actions.contains(&Action::Finish {
            status: Status::Partial
        }));
    }

    /// Nothing to do is exit 0 and **not** `PARTIAL` (PLAN.md §3).
    #[test]
    fn a_workspace_with_no_services_is_skipped_rather_than_failed() {
        let (state, actions) = drive(up(&[]), Event::Started);
        assert_eq!(verdict(&state), Status::Skipped);
        assert_eq!(
            actions,
            vec![Action::Finish {
                status: Status::Skipped
            }]
        );
    }

    /// `down` has no `SKIPPED` in its contract, and a workspace with nothing
    /// running is a workspace that is down.
    #[test]
    fn down_over_no_services_is_down() {
        let state = State::new(Direction::Down, Vec::new(), needs(&[]), &timeouts(&[]));
        let (state, actions) = drive(state, Event::Started);
        assert_eq!(verdict(&state), Status::Down);
        assert_eq!(
            actions,
            vec![Action::Finish {
                status: Status::Down
            }]
        );
    }

    /// `down` records before it stops too: the row is what a later `clean`
    /// reads, and a stop that failed leaves a live process the row still names.
    #[test]
    fn down_stops_rather_than_starts_and_settles_on_the_confirmation() {
        let state = State::new(
            Direction::Down,
            order(&["web"]),
            needs(&[]),
            &timeouts(&[("web", 0)]),
        );
        let (state, actions) = drive(state, Event::Started);
        assert_eq!(
            actions,
            vec![Action::Record {
                service: "web".to_string()
            }]
        );
        let (state, actions) = drive(
            state,
            Event::Recorded {
                service: "web".to_string(),
            },
        );
        assert_eq!(
            actions,
            vec![Action::Stop {
                service: "web".to_string()
            }]
        );
        let (state, _) = drive(
            state,
            Event::Stopped {
                service: "web".to_string(),
                attempt: Attempt::default(),
            },
        );
        assert_eq!(state.rows["web"].status, Status::Down);
        assert_eq!(verdict(&state), Status::Down);
    }

    /// A group still alive after SIGKILL is a real leak, so it fails the row.
    #[test]
    fn a_service_that_would_not_stop_fails_the_row() {
        let state = State::new(
            Direction::Down,
            order(&["web"]),
            needs(&[]),
            &timeouts(&[("web", 0)]),
        );
        let (state, _) = drive(state, Event::Started);
        let (state, _) = drive(
            state,
            Event::Recorded {
                service: "web".to_string(),
            },
        );
        let (state, _) = drive(
            state,
            Event::StopFailed {
                service: "web".to_string(),
                error: ArmadaError {
                    class: ErrClass::ToolFailed,
                    r#where: "web".to_string(),
                    message: "process group 4212 survived SIGKILL".to_string(),
                    next_action: None,
                },
            },
        );
        assert_eq!(state.rows["web"].status, Status::Failed);
        assert_eq!(verdict(&state), Status::Failed);
    }

    /// SIGINT leaves what is already up alone — it is recorded, so it is
    /// reclaimable — and does not pretend the rest failed.
    #[test]
    fn an_interrupt_skips_what_had_not_started_rather_than_failing_it() {
        let mut state = up(&["a", "b"]);
        state = drive(state, Event::Started).0;
        state = drive(
            state,
            Event::Recorded {
                service: "a".to_string(),
            },
        )
        .0;
        state = drive(
            state,
            Event::Spawned {
                service: "a".to_string(),
                attempt: Attempt::default(),
            },
        )
        .0;
        state = drive(
            state,
            Event::Ready {
                service: "a".to_string(),
            },
        )
        .0;
        let (state, _) = drive(state, Event::Interrupted);
        assert_eq!(state.rows["b"].status, Status::Skipped);
        assert_eq!(
            state.rows["b"].reason.as_deref(),
            Some("interrupted before it was started")
        );
        assert_eq!(verdict(&state), Status::Partial);
    }

    /// A duration is measured from the moment work on the row began, so a
    /// ready-wait is inside it — which is the number the caller is asking about.
    #[test]
    fn the_duration_covers_the_ready_wait_and_not_only_the_spawn() {
        let mut state = up(&["db"]);
        state = drive(state, Event::Tick { now_mono: 1_000 }).0;
        state = drive(
            state,
            Event::Recorded {
                service: "db".to_string(),
            },
        )
        .0;
        state = drive(
            state,
            Event::Spawned {
                service: "db".to_string(),
                attempt: Attempt::default(),
            },
        )
        .0;
        state = drive(state, Event::Tick { now_mono: 3_500 }).0;
        let (_, actions) = drive(
            state,
            Event::Ready {
                service: "db".to_string(),
            },
        );
        let Some(Action::Emit { result }) = actions
            .iter()
            .find(|a| matches!(a, Action::Emit { .. }))
            .cloned()
        else {
            panic!("{actions:?}")
        };
        assert_eq!(result.duration_ms, Some(2_500));
    }

    /// **The argv and the ready-check reach the row**, because `up.md`'s
    /// payload asks for both by name and neither can be reconstructed
    /// afterwards: `${port.NAME}` has already been substituted, so the config
    /// no longer says what ran or what was waited on.
    #[test]
    fn a_row_carries_what_ran_and_what_it_waited_for() {
        let mut state = up(&["db"]);
        state = drive(state, Event::Started).0;
        state = drive(
            state,
            Event::Recorded {
                service: "db".to_string(),
            },
        )
        .0;
        state = drive(
            state,
            Event::Spawned {
                service: "db".to_string(),
                attempt: Attempt {
                    owns: vec!["pgid:4212".to_string()],
                    argv: vec!["postgres".to_string(), "-p".to_string(), "5460".to_string()],
                    log: Some(".armada/logs/db.log".to_string()),
                    ready: Some("tcp pg".to_string()),
                },
            },
        )
        .0;
        let (_, actions) = drive(
            state,
            Event::Ready {
                service: "db".to_string(),
            },
        );
        let Some(Action::Emit { result }) = actions
            .iter()
            .find(|a| matches!(a, Action::Emit { .. }))
            .cloned()
        else {
            panic!("{actions:?}")
        };
        assert_eq!(result.argv, vec!["postgres", "-p", "5460"]);
        assert_eq!(result.reason.as_deref(), Some("tcp pg"));
        assert_eq!(result.log.as_deref(), Some(".armada/logs/db.log"));
        assert_eq!(result.owns, vec!["pgid:4212"]);
    }

    /// **A driver that created two of three and then failed keeps the two.**
    /// Dropping their ids on the floor is the strand this whole ordering exists
    /// to prevent.
    #[test]
    fn a_start_that_failed_part_way_keeps_what_it_had_already_created() {
        let mut state = up(&["stack"]);
        state = drive(state, Event::Started).0;
        state = drive(
            state,
            Event::Recorded {
                service: "stack".to_string(),
            },
        )
        .0;
        let (state, _) = drive(
            state,
            Event::SpawnFailed {
                service: "stack".to_string(),
                attempt: Attempt {
                    owns: vec!["container:one".to_string(), "network:two".to_string()],
                    ..Attempt::default()
                },
                error: ArmadaError {
                    class: ErrClass::ToolFailed,
                    r#where: "stack".to_string(),
                    message: "compose exited 1".to_string(),
                    next_action: None,
                },
            },
        );
        assert_eq!(
            state.rows["stack"].owns,
            vec!["container:one", "network:two"]
        );
    }

    /// The run finishes once. A second `Finish` would have the shell write two
    /// envelopes for one invocation.
    #[test]
    fn the_run_finishes_exactly_once() {
        let (state, actions) = drive(up(&[]), Event::Started);
        assert_eq!(actions.len(), 1);
        let (_, again) = drive(state, Event::Tick { now_mono: 10 });
        assert!(again.is_empty(), "{again:?}");
    }
}
