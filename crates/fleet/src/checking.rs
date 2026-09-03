//! Running one step's Checks, several at a time, bounded — and, before them,
//! what their `requires` names. [`beforehand`] owns that half.
//!
//! # One observation per declared Check, in the step's order
//!
//! `Ran::of` refuses a list shorter than the step's declaration, which is how a
//! vacuous pass is made unconstructible. Appending each result as it finished
//! would satisfy the count and lose the order, and a Job that reads differently
//! on two runs is that defect wearing a better disguise.
//!
//! So nothing here appends. The vector is sized from the declaration before
//! anything is spawned and each Check is written into its own slot, skips
//! included, which makes the order of the report a property of the type rather
//! than of the scheduler.
//!
//! # Each Check keeps its own budget, and nothing stops early
//!
//! `checks_runner::run` holds the timeout and is given the whole budget per
//! call. A batch-wide deadline would let the slowest Check fail the others by
//! spending their time — a false failure, and the worst kind, because it moves
//! when the machine is busy. The clock starts when a Check starts: one waiting
//! for a slot spends nothing, since its future is not polled until spawned.
//!
//! A failing Check cancels none of the others. Someone reading a failed step
//! wants every result, and the second failure often explains the first.

use std::path::{Path, PathBuf};
use std::time::Duration;

use checks_runner::{Attempt, Output};
use core_model::{Prerequisite, ResolvedCheck};
use tokio::task::JoinSet;
use tokio::time::Instant;
use verification::{Artifact, Exit, NeverRan, Observed};

/// How many of a step's Checks may run at once.
///
/// **Four, and the number is about the machine rather than about the step.**
/// It was measured when Fleet worked one Job at a time, so it was the whole of
/// Armada's concurrency; `#50` made it a share of it, bounded by
/// `Concurrency` — with the cap at two, two gates running at once is eight
/// Checks and two Drones on one machine.
///
/// **Nothing has re-measured it under two**, and the number is left where the
/// measurement put it rather than halved on an argument. **`#44` landed and
/// did not answer it**: the headroom read is pre-spawn, and two gates already
/// running are past the point anything is asked.
///
/// Measured on this repository's own six Checks, ten cores, warm target
/// directory: 28.5s one at a time against 16.5s at four. Bounds of two, three,
/// four and six were within noise of each other, because the floor is set by
/// the slowest single Check and by `build` and `test` contending for one Cargo
/// target lock however many slots exist. Four rather than two because that
/// floor is this repository's and not every step's, and four rather than six
/// because six leaves nothing for the Drone the step belongs to.
///
/// **A constant rather than a dial.** `CheckBudget` and `DryRuns` have no
/// default because what they bound is policy a person owns; this bounds how
/// many processes one machine should host, which nobody has asked to set. It
/// becomes a `Fittings` field the first time a machine disagrees with it.
const AT_ONCE: usize = 4;

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

/// What was observed of one declared Check, and what it printed.
pub(crate) struct Completed {
    pub observed: Observed,
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
    Command { name: String, run: String },
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
async fn beforehand(
    needed: &[&Prerequisite],
    worktree: &Path,
    budget: Duration,
) -> (Vec<String>, Option<NotMet>) {
    let mut met = Vec::new();
    for prerequisite in needed {
        if met.iter().any(|had: &String| had == prerequisite.name()) {
            continue;
        }
        let attempt = checks_runner::run(prerequisite.run(), worktree, budget).await;
        // **Nothing but zero passes**, for `prepare`'s reason: `expect_exit_code`
        // is a Check's field, and there is no reading of *the fix failed and
        // that was expected* that leaves a worktree the Check can measure.
        if attempt.exit != Exit::Code(0) {
            return (
                met,
                Some(NotMet {
                    command: prerequisite.name().to_string(),
                    run: prerequisite.run().to_string(),
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
pub(crate) async fn ran(
    checks: &[ResolvedCheck],
    touched: &[String],
    moved: bool,
    worktree: &Path,
    budget: Duration,
) -> Vec<Completed> {
    let mut planned: Vec<Planned> = checks
        .iter()
        .map(|check| match not_covered(check, touched) {
            Some(skipped) => Planned::Already(skipped),
            None => match check {
                ResolvedCheck::ManifestCheck { name, run, .. } => Planned::Command {
                    name: name.clone(),
                    run: run.clone(),
                },
                ResolvedCheck::DiffNonempty => Planned::Already(Observed::Diff { moved }),
                ResolvedCheck::ArtifactExists { target } => {
                    Planned::Already(Observed::Artifact(looked_for(worktree, target)))
                }
            },
        })
        .collect();

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
        false => beforehand(&needed, worktree, budget).await,
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
                *plan = Planned::Blocked {
                    name: name.clone(),
                    observed: failed.blocked(),
                };
            }
        }
    }

    let mut done: Vec<Option<(Attempt, Duration)>> = planned.iter().map(|_| None).collect();
    let mut queued = planned
        .iter()
        .enumerate()
        .filter_map(|(at, plan)| match plan {
            Planned::Command { run, .. } => Some((at, run.clone())),
            Planned::Already(_) | Planned::Blocked { .. } => None,
        });
    let worktree = worktree.to_path_buf();
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
            running.spawn(async move {
                let began = Instant::now();
                let attempt = checks_runner::run(&run, &worktree, budget).await;
                (at, attempt, began.elapsed())
            });
        }
        let Some(joined) = running.join_next().await else {
            break;
        };
        if let Ok((at, attempt, took)) = joined {
            done[at] = Some((attempt, took));
        }
    }

    let mut completed = Vec::with_capacity(planned.len());
    for (at, plan) in planned.into_iter().enumerate() {
        completed.push(match plan {
            Planned::Already(observed) => Completed {
                observed,
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
                printed: not_met.as_ref().map(|failed| (name, failed.output.clone())),
                took: Duration::ZERO,
            },
            Planned::Command { name, run } => match done[at].take() {
                Some((attempt, took)) => Completed {
                    observed: Observed::Command(attempt.exit),
                    printed: Some((name, attempt.output)),
                    took,
                },
                // The task was cancelled or it panicked, and neither is a
                // reachable state for a runner that returns an `Exit` for every
                // way a process can fail. It is filled in rather than dropped
                // because dropping it is the short list `Ran::of` refuses, and
                // a Check nobody can account for must not read as one that
                // passed.
                None => Completed {
                    observed: Observed::Command(Exit::NeverRan(NeverRan::NotSpawned {
                        program: run,
                        kind: std::io::ErrorKind::Interrupted,
                    })),
                    printed: Some((name, Output::default())),
                    took: Duration::ZERO,
                },
            },
        });
    }
    completed
}
