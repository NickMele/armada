//! Running one step's Checks, several at a time, bounded — and, before them,
//! what their `requires` names. [`beforehand`] owns that half.
//!
//! **One observation per declared Check, in the step's order.** `Ran::of` refuses a list
//! shorter than the step's declaration, so a vacuous pass is unconstructible: the vector is
//! sized from the declaration before anything spawns and each Check gets its own slot, skips
//! included — order is a property of the type, not the scheduler.
//!
//! **Each Check keeps its own budget, and nothing stops early.** `checks_runner::run` holds the
//! timeout whole per call rather than shared over the batch, so a false failure does not move
//! when the machine is busy. A failing Check cancels none of the others.
//!
//! Over 500 lines: `ports` and `env` thread through both halves already, for
//! `docs/concepts/manifest.md`'s Ports section — splitting by function would separate a Check
//! from the prerequisite it waits behind.

use std::collections::BTreeMap;
use std::path::{Path, PathBuf};
use std::time::Duration;

use checks_runner::{Attempt, Narrowed, Output};
use core_model::{Prerequisite, ResolvedCheck};
use tokio::task::JoinSet;
use tokio::time::Instant;
use verification::{Artifact, Exit, NeverRan, Observed};

use crate::ports::resolve_ports;
use crate::underway::Announcing;

/// How many of a step's Checks may run at once.
///
/// **Four, about the machine, not the step.** Measured when Fleet worked one Job at a time, the
/// whole of Armada's concurrency then; `#50` made it a share, bounded by `Concurrency` (at cap
/// two, two gates is eight Checks and two Drones on one machine). Nothing has re-measured it
/// under two, and `#44` landed without answering it: the headroom read is pre-spawn, past the
/// point two running gates ask anything.
///
/// Measured on this repository's own six Checks, ten cores, warm target directory: 28.5s one at
/// a time against 16.5s at four. Two, three, four and six were within noise of each other — the
/// floor is the slowest single Check and `build`/`test` contending for one Cargo target lock.
/// Four over two because that floor is this repository's, not every step's; four over six
/// because six leaves nothing for the Drone the step belongs to.
///
/// **A constant rather than a dial** — this bounds how many processes one machine should host, which nobody has asked to set; it becomes a `Fittings` field the first time one disagrees.
pub(crate) const AT_ONCE: usize = 4;

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
/// **A context is one call to [`ran`]** — one gate evaluation or one dry run — which is what
/// "skipped if already run in the same context" means: a prerequisite's effect lives in the
/// worktree, over the span nothing else edits it. A Drone edits between attempts, so `fmt` runs
/// again next attempt; a Check in its own container is a third context and finds no hit
/// (`docs/concepts/manifest.md`'s own reading).
///
/// **Serial, before anything spawns** — these mutate the worktree by design, so one running
/// beside a Check would rewrite files it is reading, and the batch pays the wall clock for that.
///
/// **First occurrence wins, by name** — two Checks naming `migrate` run it once, so `requires`
/// guarantees *has run*, not *has just run*; a Check needing fresh state resets it itself.
///
/// **`ports` and `env` are the same pair every caller hands in**, since a prerequisite is a Command and `docs/concepts/manifest.md`'s Ports section reaches every Command string.
async fn beforehand(
    needed: &[&Prerequisite],
    worktree: &Path,
    budget: Duration,
    ports: &BTreeMap<String, u16>,
    env: &[(String, String)],
) -> (Vec<String>, Option<NotMet>) {
    let mut met = Vec::new();
    for prerequisite in needed {
        if met.iter().any(|had: &String| had == prerequisite.name()) {
            continue;
        }
        let run = resolve_ports(prerequisite.run(), ports);
        let attempt = checks_runner::run_writing_with_env(&run, worktree, budget, None, env).await;
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
/// **`join` on a relative path, nothing cleverer** — `config` already refused a target that
/// globs, is absolute, ends in `/` or holds `..` when the workflow was parsed, so a second
/// guard here would be a second rule to keep in step with the first.
///
/// **Settled before anything is spawned**, beside the skip decision and for the same reason: one
/// `metadata` call with no command, no budget, no ordering — so no Check's own output can be
/// what satisfies it.
///
/// Every way the filesystem says no reads as [`Artifact::Missing`]: the Drone did not write it,
/// overwhelmingly, which is the answer the gate wants and the Drone can act on.
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
/// `moved` is `diff_nonempty`'s answer, decided by the caller — a fallible read of the work
/// product that belongs where the caller's error path already is, and reading it before rather
/// than during means no Check's output can be part of what the diff sees.
///
/// `announcing` is told when the batch begins and as each Check spawns and joins, so a person
/// sees waiting, running and finished rather than nothing until the ruling. **Told and never
/// asked** — nothing here reads it back.
///
/// `ports` and `env` are the claimed span's name-to-port map and environment, **handed in and
/// never read here** — a caller with a Job in hand can resolve a claim and nothing else here
/// can (`gate::rule_on`'s reason beside `policies`), and every call site is a Fleet method with
/// a store to ask. Both are empty where the Job declared no `ports:` or there is no Job at all
/// (`crate::proving`'s call, proving a merged commit nothing dispatched).
#[allow(clippy::too_many_arguments)]
pub(crate) async fn ran(
    checks: &[ResolvedCheck],
    touched: &[String],
    moved: bool,
    narrow: bool,
    worktree: &Path,
    budget: Duration,
    announcing: &Announcing,
    ports: &BTreeMap<String, u16>,
    env: &[(String, String)],
) -> Vec<Completed> {
    let mut planned: Vec<Planned> = checks
        .iter()
        .map(|check| match not_covered(check, touched) {
            Some(skipped) => Planned::Already(skipped),
            None => match check {
                ResolvedCheck::ManifestCheck { name, run, .. } => {
                    narrowed(check, name, run, touched, narrow)
                }
                ResolvedCheck::DiffNonempty => Planned::Already(Observed::Diff { moved }),
                ResolvedCheck::ArtifactExists { target } => {
                    Planned::Already(Observed::Artifact(looked_for(worktree, target)))
                }
            },
        })
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
        false => beforehand(&needed, worktree, budget, ports, env).await,
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

    let mut done: Vec<Option<(Attempt, Duration)>> = planned.iter().map(|_| None).collect();
    let mut queued = planned
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
        });
    let worktree = worktree.to_path_buf();
    // Owned, so each spawned Check can move its own copy — the env slice this
    // function borrows does not outlive the batch, and a spawned future must.
    let env: Vec<(String, String)> = env.to_vec();
    // Refilled as each one finishes rather than run in batches of four: a batch
    // costs the slowest member of it, and a step whose Checks are 17s and 1s
    // would spend the fast slot idle for sixteen of them.
    let mut running: JoinSet<(usize, Attempt, Duration)> = JoinSet::new();
    loop {
        while running.len() < AT_ONCE {
            let Some((at, run)) = queued.next() else {
                break;
            };
            let worktree: PathBuf = worktree.clone();
            let env = env.clone();
            let log = announcing.log_for(at);
            let writing = log.clone();
            running.spawn(async move {
                let began = Instant::now();
                let attempt = checks_runner::run_writing_with_env(
                    &run,
                    &worktree,
                    budget,
                    writing.as_deref(),
                    &env,
                )
                .await;
                (at, attempt, began.elapsed())
            });
            announcing.started(at, log.as_deref());
        }
        let Some(joined) = running.join_next().await else {
            break;
        };
        if let Ok((at, attempt, took)) = joined {
            announcing.finished(
                at,
                &checks[at],
                &Observed::Command(attempt.exit.clone()),
                took,
            );
            done[at] = Some((attempt, took));
        }
    }

    let mut completed = Vec::with_capacity(planned.len());
    for (at, plan) in planned.into_iter().enumerate() {
        completed.push(match plan {
            Planned::Already(observed) => Completed {
                observed,
                narrowed_to: None,
                printed: None,
                took: Duration::ZERO,
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
                    }
                }
            },
        });
    }
    completed
}
