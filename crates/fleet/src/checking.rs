//! Running one step's Checks, several at a time, bounded — and, before them,
//! what their `requires` names. [`beforehand`] owns that half.
//!
//! # One observation per declared Check, in the step's order
//!
//! `Ran::of` refuses a list shorter than the step's declaration, which is how a
//! vacuous pass is made unconstructible. So nothing here appends: the vector is
//! sized from the declaration before anything is spawned and each Check is
//! written into its own slot, skips included, which makes the order of the
//! report a property of the type rather than of the scheduler.
//!
//! # Each Check keeps its own budget, and only a Drone's run stops early
//!
//! `checks_runner::run` holds the timeout, given whole per call rather than
//! shared over the batch — a false failure moves when the machine is busy. At
//! the gate a failing Check cancels none of the others; the second failure
//! often explains the first. A Drone's own run stops at its first failed
//! command, because the Drone will fix that and ask again (#1062).
//!
//! **A Check declared `runs_at: handoff` starts last**, once every other answer
//! in the batch lets the step through; otherwise it records a skip (#849).
//!
//! **Started fastest first, reported in the step's order.** [`Room`] carries
//! the repository's past durations; each result still lands in its own slot.
//!
//! **Each command holds one or more of the machine's places while it runs** —
//! most take one, a Check may declare more — in one line with every other
//! Job's. `crate::places`. #1063, #1102.
//!
//! **Over 500 lines.** `ports` and `env` thread through both halves already
//! here, for `docs/concepts/manifest.md`'s Ports section: it reaches every
//! Command, and splitting by function would separate a Check from the
//! prerequisite it waits behind.

use std::collections::{BTreeMap, VecDeque};
use std::path::{Path, PathBuf};
use std::time::Duration;

use adapter_traits::Footprint;
use checks_runner::{Attempt as RunAttempt, Narrowed, Output, Writing};
use core_model::{Attempt, Prerequisite, ResolvedCheck, RunsAt, TaskCounts};
use tokio::task::JoinSet;
use tokio::time::Instant;
use verification::{Artifact, Exit, NeverRan, Observed};

use crate::places::{Ask, Place, Room};
use crate::ports::resolve_ports;
use crate::reuse::{self, KeptDryRun};
use crate::underway::Announcing;

/// What ends a batch's commands before their budget does: each one's whole
/// group, through the runner's own stop. The gate's and a proof's never fire.
#[derive(Clone, Debug)]
pub(crate) struct Stop {
    watched: Option<tokio::sync::watch::Receiver<()>>,
    /// Whether the batch's first failed command stops the rest. **Only
    /// [`Stop::when_dropped_or_one_fails`] sets it**, and no gate builds one.
    at_first_failure: bool,
}

/// Holding this keeps a batch going. Dropped, every command in it is stopped.
#[derive(Debug)]
pub(crate) struct Going {
    _going: tokio::sync::watch::Sender<()>,
}

impl Stop {
    pub(crate) fn never() -> Stop {
        Stop {
            watched: None,
            at_first_failure: false,
        }
    }

    /// A stop that fires when the [`Going`] beside it is dropped. #1020.
    pub(crate) fn when_dropped() -> (Going, Stop) {
        let (going, watched) = tokio::sync::watch::channel(());
        let stop = Stop {
            watched: Some(watched),
            at_first_failure: false,
        };
        (Going { _going: going }, stop)
    }

    /// [`Stop::when_dropped`], and the first failed command stops the rest too:
    /// a Drone's own run, which will fix that and ask again. #1062.
    pub(crate) fn when_dropped_or_one_fails() -> (Going, Stop) {
        let (going, stop) = Stop::when_dropped();
        let stop = Stop {
            at_first_failure: true,
            ..stop
        };
        (going, stop)
    }

    async fn stopped(mut self) {
        match &mut self.watched {
            None => std::future::pending().await,
            // Nothing is ever sent, so this returns only once the sender is gone.
            Some(watched) => while watched.changed().await.is_ok() {},
        }
    }
}

/// Whether the gate declines to run this Check, and what it writes down when it
/// does.
///
/// **Pure, and the whole of the skip decision.** It reads the Check's own
/// frozen `when` against paths already in hand: no adapter call, no clock, no
/// process. That is what lets the decision be taken while the batch is being
/// built, before anything is spawned and without ordering anything.
///
/// **The kind of change is not consulted.** A file deleted from `packages/` is
/// a change to `packages/`, and a rename arrives as two paths — the old one
/// deleted, the new one added, because the git adapter runs no rename
/// detection — so either side of a rename is enough on its own.
fn not_covered(check: &ResolvedCheck, touched: &[String]) -> Option<Observed> {
    // `ResolvedCheck::covers` answers `true` for a Check with no `when`, which
    // is where "absent means always" is spelled. It is asked rather than
    // re-derived here so there is one place that could ever be wrong about it.
    match check.covers(touched) {
        true => None,
        false => Some(Observed::Skipped {
            covers: check
                .when()
                .map(core_model::Covers::written)
                .unwrap_or_default(),
        }),
    }
}

/// What a Check runs when a Drone asked about its own change.
///
/// **Pure, and settled beside the skip decision** for the same reason: it reads
/// the Check's frozen `narrow` against paths already in hand, so nothing is
/// spawned and nothing is ordered by it.
///
/// **Three answers and each is a different sentence.** The Check runs whole,
/// which is what a Check with no `narrow` does and what every gate run does;
/// it runs a narrower command; or it is not run at all, because nothing the
/// change touched feeds it. The third is a skip and not a pass — a Drone told
/// `test` passed on a change that touched no crate would have been told
/// something false about its own work.
fn narrowed(
    check: &ResolvedCheck,
    name: &str,
    run: &str,
    touched: &[String],
    narrow: bool,
) -> Planned {
    let whole = Planned::Command {
        name: name.to_string(),
        run: run.to_string(),
        narrowed_to: None,
    };
    if !narrow {
        return whole;
    }
    match checks_runner::narrowed(check.narrowing(), touched) {
        Narrowed::Whole => whole,
        Narrowed::To(command) => Planned::Command {
            name: name.to_string(),
            run: run.to_string(),
            narrowed_to: Some(command),
        },
        // The same sentence a `when` skip writes, and it is the true one: a
        // Check that narrows to a directory the change did not touch has
        // nothing to say about the change. `Observed::Skipped` carries what a
        // reader's first question is, so it carries what the narrowing looked
        // at rather than what the Check covers.
        Narrowed::Nothing => Planned::Already(Observed::Skipped {
            covers: check
                .narrowing()
                .and_then(core_model::Narrowing::under)
                .unwrap_or_default()
                .to_string(),
        }),
    }
}

/// What was observed of one declared Check, and what it printed.
pub(crate) struct Completed {
    pub observed: Observed,
    /// The command this Check was narrowed to, where it was narrowed at all.
    /// **`None` on every gate run**, which never narrows, and on a narrowed run
    /// of a Check the Manifest gave no narrower way to run.
    pub narrowed_to: Option<String>,
    /// The Check's name and its output, for a Check that ran a command. `None`
    /// for a skip and for `diff_nonempty`, neither of which prints anything.
    pub printed: Option<(String, Output)>,
    /// How long this Check took on its own — not its share of the batch.
    pub took: Duration,
    /// The command whose failure stopped this Check before it finished, on a
    /// Drone's run. `None` for every Check that ran its course.
    pub stopped: Option<String>,
}

/// What is known about one Check before anything is spawned.
///
/// The two variants are the whole reason the skip decision is pure: everything
/// that does not need a process is settled while the list is being built, and
/// the futures are made only for what is left.
enum Planned {
    /// Already answered — a Check the step's changes do not cover, or the diff
    /// reading the caller took before this was called.
    Already(Observed),
    /// A Check that will not be run, because a Command it requires did not
    /// succeed. **Not a skip** — a skip fails nothing and this must, so it is
    /// its own variant and carries an `Observed::Command` that no expectation
    /// can be compared into a pass.
    ///
    /// The name rides along so the prerequisite's output is filed under the
    /// Check whose row a person will open looking for it.
    Blocked { name: String, observed: Observed },
    /// A command, and the slot it belongs in.
    ///
    /// `narrowed_to` is the command a narrowed run replaced the Check's own
    /// with, kept beside `run` rather than instead of it because the report has
    /// to be able to say the run was narrowed at all — a command line with no
    /// second reading is one a Drone would take for the Check itself.
    Command {
        name: String,
        run: String,
        narrowed_to: Option<String>,
    },
}

/// A prerequisite that did not succeed, and what it printed.
///
/// One per batch at most, because [`beforehand`] stops at the first failure —
/// `[migrate, seed]` is a sequence, and carrying on past the first would
/// produce the second's error about the first's job. That is
/// `crate::preparing::prepare`'s rule, one scope down.
struct NotMet {
    command: String,
    run: String,
    exit: Exit,
    output: Output,
}

impl NotMet {
    /// What a Check blocked by this is told, and what its row records.
    ///
    /// **The Check's name is not in here.** It is the prerequisite that broke,
    /// and the sentence a Drone reads has to name what it should go and fix.
    fn blocked(&self) -> Observed {
        Observed::Command(Exit::NeverRan(NeverRan::PrerequisiteFailed {
            command: self.command.clone(),
            run: self.run.clone(),
            exit: Box::new(self.exit.clone()),
        }))
    }
}

/// Run every prerequisite the batch's runnable Checks name, in order, once each.
///
/// **A context is one call to [`ran`]** — one gate evaluation of one step, or
/// one dry run — and that is what "skipped if already run in the same context"
/// means here. It follows from where a prerequisite's effect lives: in the
/// worktree, over the span nothing else is editing it. A Drone edits between
/// attempts, so the next attempt is a new context and `fmt` runs again, which
/// it must or the second attempt gates on the first one's formatting. A Check
/// in its own container is a third context and finds no hit, which is
/// `docs/concepts/manifest.md`'s own reading.
///
/// **Serial, and before anything spawns.** These mutate the worktree by design;
/// one running beside a Check would rewrite files under a command already
/// reading them. The batch pays the wall clock for the guarantee.
///
/// **First occurrence wins, by name.** Two Checks naming `migrate` run it once.
/// So `requires` guarantees *has run*, not *has just run* — a Check needing
/// genuinely fresh state resets what it needs in its own command.
///
/// **`ports` and `env` are the same pair every Check-running caller hands
/// in.** A prerequisite is a Command, and `docs/concepts/manifest.md`'s Ports
/// section reaches every Command string, not only a `setup.requires` one —
/// see `crate::ports::resolve_ports` for the substitution and
/// `crate::ports::env_vars` for what `env` holds.
async fn beforehand(
    needed: &[&Prerequisite],
    worktree: &Path,
    budget: Duration,
    ports: &BTreeMap<String, u16>,
    env: &[(String, String)],
    stop: &Stop,
) -> (Vec<String>, Option<NotMet>) {
    let mut met = Vec::new();
    for prerequisite in needed {
        if met.iter().any(|had: &String| had == prerequisite.name()) {
            continue;
        }
        let run = resolve_ports(prerequisite.run(), ports);
        let attempt = checks_runner::run_until(
            &run,
            worktree,
            budget,
            Writing::Nowhere,
            env,
            stop.clone().stopped(),
        )
        .await;
        // **Nothing but zero passes**, for `prepare`'s reason: `expect_exit_code`
        // is a Check's field, and there is no reading of *the fix failed and
        // that was expected* that leaves a worktree the Check can measure.
        if attempt.exit != Exit::Code(0) {
            return (
                met,
                Some(NotMet {
                    command: prerequisite.name().to_string(),
                    run,
                    exit: attempt.exit,
                    output: attempt.output,
                }),
            );
        }
        met.push(prerequisite.name().to_string());
    }
    (met, None)
}

/// What is at the path a step's `artifact_exists` names.
///
/// **`join` on a relative path and nothing cleverer.** `config` refused a
/// target that globs, that is absolute, that ends in `/` or that holds `..`
/// where the workflow was parsed, so what arrives here cannot leave the
/// worktree and cannot match two files. A second guard here would be a second
/// rule to keep in step with the first.
///
/// **Settled before anything is spawned**, beside the skip decision and for the
/// same reason: it is one `metadata` call with no command, no budget and no
/// ordering. Settling it first also means no Check's own output can be what
/// satisfies it.
///
/// Every way the filesystem says no reads as [`Artifact::Missing`]: the
/// overwhelmingly common reason is that the Drone did not write it, which is
/// the answer the gate wants and the one the Drone can act on.
fn looked_for(worktree: &Path, target: &str) -> Artifact {
    match std::fs::metadata(worktree.join(target)) {
        Err(_) => Artifact::Missing,
        Ok(found) if !found.is_file() => Artifact::NotAFile,
        Ok(found) if found.len() == 0 => Artifact::Empty,
        Ok(_) => Artifact::Written,
    }
}

/// Run the step's Checks in `worktree` and say what each one did.
///
/// `moved` is `diff_nonempty`'s answer, decided by the caller: it is a read of
/// the work product, which is fallible and belongs where the caller's error
/// path already is. Reading it before rather than during also means no Check's
/// output can be part of what the diff sees.
///
/// `announcing` is told when the batch begins and as each Check is spawned and
/// joined, so a person sees it waiting, running and finished rather than
/// nothing until the ruling. **Told and never asked**: nothing here reads it
/// back, and what this returns is built exactly as it was before —
/// `crate::underway`.
///
/// `ports` and `env` are the claimed span's name-to-port map and the
/// environment it sets, **handed in and never read here** — for
/// `gate::rule_on`'s own reason beside `policies`: a caller with a Job in hand
/// can resolve a claim and nothing else here can, and every call site is a
/// Fleet method with a store to ask. Both are empty where the Job declared no
/// `ports:`, or where there is no Job at all — `crate::proving`'s own call,
/// proving a merged commit nothing dispatched.
///
/// `plan` is the Job's plan counts off the store, handed in for `ports`' reason;
/// `None` is a Job no plan was recorded for.
/// `room` is where each command waits for a place on the machine — [`Room`].
///
/// `dry_run`, `attempt` and `footprint_now` are what a gate hands in to reuse a
/// Check instead of asking it again; a dry run itself, and `crate::proving`'s
/// commit sweep, hand in `None` and answer every position by running it.
///
/// **Looked up by name, per Check, inside the loop below — never a second
/// sequence.** [`reuse::trusted`] decides once whether `dry_run` is good for
/// anything against `attempt` and `footprint_now`; [`KeptDryRun::passed`] is
/// then asked once per Check, by the name that Check already carries. There is
/// no parallel `Vec` here for that loop's length to disagree with, which is
/// what a `zip` over one used to risk — see this module's own history on
/// `#1014`.
#[allow(clippy::too_many_arguments)]
pub(crate) async fn ran(
    checks: &[ResolvedCheck],
    touched: &[String],
    moved: bool,
    narrow: bool,
    worktree: &Path,
    budget: Duration,
    room: &Room,
    announcing: &Announcing,
    ports: &BTreeMap<String, u16>,
    env: &[(String, String)],
    plan: Option<TaskCounts>,
    stop: &Stop,
    dry_run: Option<&KeptDryRun>,
    attempt: Attempt,
    footprint_now: Option<&Footprint>,
) -> Vec<Completed> {
    let trusted = reuse::trusted(dry_run, attempt, footprint_now);
    let mut planned: Vec<Planned> = checks
        .iter()
        .map(
            |check| match trusted.and_then(|kept| kept.passed(check.label())) {
                Some(row) => Planned::Already(Observed::Reused(row)),
                None => match not_covered(check, touched) {
                    Some(skipped) => Planned::Already(skipped),
                    None => match check {
                        ResolvedCheck::ManifestCheck { name, run, .. } => {
                            narrowed(check, name, run, touched, narrow)
                        }
                        ResolvedCheck::DiffNonempty => Planned::Already(Observed::Diff { moved }),
                        ResolvedCheck::ArtifactExists { target } => {
                            Planned::Already(Observed::Artifact(looked_for(worktree, target)))
                        }
                        ResolvedCheck::PlanRecorded { .. } => Planned::Already(Observed::Plan {
                            tasks: plan.map(|counts| counts.not_dropped()),
                        }),
                    },
                },
            },
        )
        .collect();

    // **Before the prerequisites**, so a Check waiting behind `migrate` reads
    // as waiting rather than as nothing. What is already answered is said now;
    // a Check a prerequisite goes on to block is said below, once it is.
    let settled: Vec<Option<&Observed>> = planned
        .iter()
        .map(|plan| match plan {
            Planned::Already(observed) => Some(observed),
            Planned::Blocked { observed, .. } => Some(observed),
            Planned::Command { .. } => None,
        })
        .collect();
    announcing.began(checks, &settled);

    // Every prerequisite of every Check that is actually going to run, in the
    // order the Manifest named them, before anything is spawned. `beforehand`
    // takes the wall clock of this out of the batch on purpose; see the module
    // header for why it cannot overlap the Checks it prepares for.
    let needed: Vec<&Prerequisite> = planned
        .iter()
        .enumerate()
        .filter(|(_, plan)| matches!(plan, Planned::Command { .. }))
        .flat_map(|(at, _)| checks[at].requires())
        .collect();
    let (met, not_met) = match needed.is_empty() {
        true => (Vec::new(), None),
        // One place for the whole phase, which is serial and as heavy as a Check.
        false => {
            let mut ask = room.ask();
            let place = tokio::select! {
                place = room.granted(&mut ask, |held| announcing.behind(held)) => Some(place),
                () = stop.clone().stopped() => None,
            };
            drop(ask);
            announcing.behind(0);
            let met = beforehand(&needed, worktree, budget, ports, env, stop).await;
            drop(place);
            met
        }
    };
    // A Check whose prerequisites all ran still runs, even where another
    // Check's did not: a broken `migrate` is not a reason to stop asking `lint`.
    // `met` is what succeeded before the phase stopped, so a Check naming
    // something after the failure is blocked by it too — and is told about the
    // command that actually broke rather than the one that never got a turn.
    if let Some(failed) = &not_met {
        for (at, plan) in planned.iter_mut().enumerate() {
            let unmet = checks[at]
                .requires()
                .iter()
                .any(|needed| !met.iter().any(|had| had == needed.name()));
            if let (true, Planned::Command { name, .. }) = (unmet, &*plan) {
                let observed = failed.blocked();
                announcing.finished(at, &checks[at], &observed, Duration::ZERO);
                *plan = Planned::Blocked {
                    name: name.clone(),
                    observed,
                };
            }
        }
    }

    let mut done: Vec<Option<(RunAttempt, Duration)>> = planned.iter().map(|_| None).collect();
    let queued = planned
        .iter()
        .enumerate()
        .filter_map(|(at, plan)| match plan {
            // The narrowed command where there is one: it is what this run is
            // about, and the Check's own line is kept only so the report can
            // say the two differed.
            Planned::Command {
                run, narrowed_to, ..
            } => Some((
                at,
                resolve_ports(&narrowed_to.clone().unwrap_or_else(|| run.clone()), ports),
            )),
            Planned::Already(_) | Planned::Blocked { .. } => None,
        })
        .collect::<Vec<(usize, String)>>();
    let queued: VecDeque<(usize, String)> = room.past().fastest_first(queued, checks);
    let (later, now): (Vec<(usize, String)>, Vec<(usize, String)>) = queued
        .into_iter()
        .partition(|(at, _)| checks[*at].runs_at() == RunsAt::Handoff);
    let mut deferred = vec![false; planned.len()];
    for (at, _) in &later {
        deferred[*at] = true;
    }
    let mut queued: VecDeque<(usize, String)> = now.into();
    let mut later: Option<VecDeque<(usize, String)>> = (!later.is_empty()).then(|| later.into());
    let mut held_back = vec![false; planned.len()];
    let worktree = worktree.to_path_buf();
    // Owned, so each spawned Check can move its own copy — the env slice this
    // function borrows does not outlive the batch, and a spawned future must.
    let env: Vec<(String, String)> = env.to_vec();
    // Refilled as each one finishes rather than run in batches: a batch costs
    // the slowest member of it, and a step whose Checks are 17s and 1s would
    // spend the fast slot idle for sixteen of them. **Each waits for a place on
    // the machine**, so a short or busy machine waits for a finish, not a batch.
    // **A Drone's run drops `halting` at its first failure**, which stops every
    // command still running, and nothing queued starts after it. A
    // prerequisite that broke is that failure before anything spawns.
    let (halting, halted) = Stop::when_dropped();
    let mut halting = Some(halting);
    let mut failed_first = stop
        .at_first_failure
        .then(|| not_met.as_ref().map(|failed| failed.command.clone()))
        .flatten();
    if failed_first.is_some() {
        halting = None;
    }
    let mut stopped = vec![false; planned.len()];
    let mut running: JoinSet<(usize, RunAttempt, Duration)> = JoinSet::new();
    // The batch's turn for its next Check, kept across wakes so it keeps its place in line.
    let mut ask: Option<Ask> = None;
    // Places this batch's own running Checks hold right now — in places, not
    // in how many are running, since a weighted Check holds more than one.
    // #1102.
    let mut own_places: usize = 0;
    let stopping = stop.clone().stopped();
    tokio::pin!(stopping);
    loop {
        // Everything else has answered: start what waits for handoff, or hold it back.
        if queued.is_empty() && running.is_empty() {
            if let Some(held) = later.take() {
                match halting.is_some() && passed_so_far(checks, &planned, &done, &deferred) {
                    true => queued = held,
                    false => {
                        for (at, _) in held {
                            let observed = Observed::HeldBack;
                            announcing.finished(at, &checks[at], &observed, Duration::ZERO);
                            held_back[at] = true;
                        }
                    }
                }
            }
        }
        let wants_a_turn = halting.is_some() && !queued.is_empty();
        if !wants_a_turn {
            ask = None;
            if running.is_empty() {
                break;
            }
        } else if ask.is_none() {
            // Peeked rather than popped: the ask has to be for the Check that
            // will actually be spawned once it is granted. #1102.
            let places = queued
                .front()
                .map_or(std::num::NonZeroU32::MIN, |(at, _)| checks[*at].places());
            ask = Some(room.ask_for(places));
        }
        tokio::select! {
            place = next_place(room, ask.as_mut(), own_places, announcing), if wants_a_turn => {
                ask = None;
                announcing.behind(0);
                let Some((at, run)) = queued.pop_front() else {
                    continue;
                };
                own_places += checks[at].places().get() as usize;
                let worktree: PathBuf = worktree.clone();
                let env = env.clone();
                let log = announcing.log_for(at);
                let writing = log.clone();
                let stop = stop.clone();
                let halted = halted.clone();
                running.spawn(async move {
                    let _held: Place = place;
                    let began = Instant::now();
                    let ended = async move {
                        tokio::select! {
                            () = stop.stopped() => {}
                            () = halted.stopped() => {}
                        }
                    };
                    let attempt = checks_runner::run_until(
                        &run,
                        &worktree,
                        budget,
                        writing.as_deref().map_or(Writing::Nowhere, Writing::Fresh),
                        &env,
                        ended,
                    )
                    .await;
                    (at, attempt, began.elapsed())
                });
                announcing.started(at, log.as_deref());
            }
            // Stopped while waiting for room: what has not started never will.
            () = &mut stopping, if wants_a_turn => {
                ask = None;
                for (at, run) in queued.drain(..) {
                    let attempt = never_started(run);
                    let observed = Observed::Command(attempt.exit.clone());
                    announcing.finished(at, &checks[at], &observed, Duration::ZERO);
                    done[at] = Some((attempt, Duration::ZERO));
                }
            }
            Some(joined) = running.join_next(), if !running.is_empty() => {
                let Ok((at, attempt, took)) = joined else {
                    continue;
                };
                own_places = own_places.saturating_sub(checks[at].places().get() as usize);
                let observed = Observed::Command(attempt.exit.clone());
                // Ended by the halt rather than by its own answer.
                let cut = failed_first
                    .clone()
                    .filter(|_| matches!(attempt.exit, Exit::Signalled { .. }));
                match cut {
                    Some(first) => {
                        stopped[at] = true;
                        announcing.stopped(at, &checks[at], &observed, took, &first);
                    }
                    None => {
                        announcing.finished(at, &checks[at], &observed, took);
                        if stop.at_first_failure
                            && failed_first.is_none()
                            && !advances(&checks[at], &observed)
                        {
                            failed_first = Some(checks[at].label().to_string());
                            halting = None;
                        }
                    }
                }
                done[at] = Some((attempt, took));
                // A result that does not end the run is heard now; the last is the report.
                if failed_first.is_none()
                    && !(queued.is_empty() && running.is_empty() && later.is_none())
                {
                    announcing.landed(at);
                }
            }
        }
    }
    // What a failure stopped before it ever started.
    if let Some(first) = &failed_first {
        for (at, run) in queued.drain(..) {
            let attempt = never_started(run);
            let observed = Observed::Command(attempt.exit.clone());
            announcing.stopped(at, &checks[at], &observed, Duration::ZERO, first);
            stopped[at] = true;
            done[at] = Some((attempt, Duration::ZERO));
        }
    }

    let mut completed = Vec::with_capacity(planned.len());
    for (at, plan) in planned.into_iter().enumerate() {
        if held_back[at] {
            completed.push(Completed {
                observed: Observed::HeldBack,
                narrowed_to: None,
                printed: None,
                took: Duration::ZERO,
                stopped: None,
            });
            continue;
        }
        completed.push(match plan {
            Planned::Already(observed) => Completed {
                observed,
                narrowed_to: None,
                printed: None,
                took: Duration::ZERO,
                stopped: None,
            },
            // **The prerequisite's output, filed under the Check's name.** It
            // is the only output there is — the Check ran nothing — and the
            // Check's row is where a person goes looking for why it did not
            // pass. `took` is zero because this Check took nothing; the
            // prerequisite's own seconds are the batch's and are not one
            // Check's to claim.
            Planned::Blocked { name, observed } => Completed {
                observed,
                narrowed_to: None,
                printed: not_met.as_ref().map(|failed| (name, failed.output.clone())),
                took: Duration::ZERO,
                stopped: None,
            },
            Planned::Command {
                name,
                run,
                narrowed_to,
            } => match done[at].take() {
                Some((attempt, took)) => Completed {
                    observed: Observed::Command(attempt.exit),
                    narrowed_to,
                    printed: Some((name, attempt.output)),
                    took,
                    stopped: stopped[at].then(|| failed_first.clone()).flatten(),
                },
                // The task was cancelled or it panicked, and neither is a
                // reachable state for a runner that returns an `Exit` for every
                // way a process can fail. It is filled in rather than dropped
                // because dropping it is the short list `Ran::of` refuses, and
                // a Check nobody can account for must not read as one that
                // passed.
                None => {
                    let observed = Observed::Command(Exit::NeverRan(NeverRan::NotSpawned {
                        program: run,
                        kind: std::io::ErrorKind::Interrupted,
                    }));
                    // Said as finished too, or a Check nobody can account for
                    // would read as running until the ruling came down.
                    announcing.finished(at, &checks[at], &observed, Duration::ZERO);
                    Completed {
                        observed,
                        narrowed_to,
                        printed: Some((name, Output::default())),
                        took: Duration::ZERO,
                        stopped: None,
                    }
                }
            },
        });
    }
    completed
}

/// The batch's next place, or never where it has not asked.
async fn next_place(
    room: &Room,
    ask: Option<&mut Ask>,
    own: usize,
    announcing: &Announcing,
) -> Place {
    match ask {
        Some(ask) => {
            room.granted(ask, |held| announcing.behind(held.saturating_sub(own)))
                .await
        }
        None => std::future::pending().await,
    }
}

/// A command a stop or a first failure kept from ever starting.
fn never_started(run: String) -> RunAttempt {
    RunAttempt {
        exit: Exit::NeverRan(NeverRan::NotSpawned {
            program: run,
            kind: std::io::ErrorKind::Interrupted,
        }),
        output: Output::default(),
    }
}

/// Whether every answer not waiting for handoff lets the step through.
fn passed_so_far(
    checks: &[ResolvedCheck],
    planned: &[Planned],
    done: &[Option<(RunAttempt, Duration)>],
    deferred: &[bool],
) -> bool {
    planned.iter().enumerate().all(|(at, plan)| {
        deferred[at]
            || match plan {
                Planned::Already(observed) | Planned::Blocked { observed, .. } => {
                    advances(&checks[at], observed)
                }
                Planned::Command { .. } => done[at].as_ref().is_some_and(|(attempt, _)| {
                    advances(&checks[at], &Observed::Command(attempt.exit.clone()))
                }),
            }
    })
}

/// Whether this answer lets a step through, as the gate would record it.
fn advances(check: &ResolvedCheck, observed: &Observed) -> bool {
    verification::Ran::against(std::slice::from_ref(check), std::slice::from_ref(observed))
        .ok()
        .and_then(|ran| ran.recorded().first().map(|row| row.outcome.advances()))
        .unwrap_or(false)
}
