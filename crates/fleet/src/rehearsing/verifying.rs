//! Verify — Journey 9's last section: setup and every Check in the main
//! checkout, once each, one after another.
//!
//! **A sequence of ordinary checkout runs, not a new way to run.** Each step
//! goes through `started_at` like a run a person starts by hand, so it keeps
//! its own record, log, snapshot, diff and Undo, and nothing is added to them.
//!
//! **One at a time, handed over before anybody is told.** A step takes the
//! checkout's one run slot as the step before gives it back, and that step's
//! `checkout_run.finished` waits for the hand-over ([`Handed`]) — so a reader
//! re-reading the sheet on the event finds the next step out, or Verify ended.
//!
//! **Never automatic, never a verdict.** Only `start_checkout_verify` begins
//! one; nothing here writes Evidence or reaches Doctor, and what it keeps is in
//! memory, as a run in flight is.

use std::collections::BTreeMap;
use std::sync::{Arc, Mutex, MutexGuard};
use std::time::Duration;

use adapter_traits::{AgentHarness, Delivery, Vcs, WorkProduct};
use api::Refusal;
use config::Manifest;
use ipc::{CheckoutVerify, Instant, VerifyGroup, VerifyStep, VerifyStepState};
use tokio::sync::oneshot;

use super::entries::{self, Entry};
use super::owner::Place;
use super::record::Record;
use super::unrehearsable::{Unrehearsable, Whose};
use crate::daemon::Fleet;
use crate::repositories::Served;

/// How long a finished step waits for its Verify to take the hand-over before
/// it is published anyway: the next step's start is a directory made.
pub(crate) const HANDING_OVER: Duration = Duration::from_secs(30);

/// One step's record, handed back to its Verify before the step is published.
/// `ack` is answered once the next step is out or the Verify has ended.
pub(crate) struct Handed {
    pub(crate) record: Record,
    pub(crate) ack: oneshot::Sender<()>,
}

/// What Verify runs against `manifest`, in order, as the steps of one that has
/// not started. **Setup, then every Check, and nothing else** — no Command
/// `setup.requires` does not name, and no server. A Check's own `requires` is
/// not a step: it runs inside that Check's run, as it always does.
pub fn verify_steps(manifest: &Manifest) -> Vec<VerifyStep> {
    entries::declared(manifest)
        .verified()
        .iter()
        .map(|(group, entry)| wired(*group, entry, &VerifyStepState::Waiting))
        .collect()
}

fn wired(group: VerifyGroup, entry: &Entry, state: &VerifyStepState) -> VerifyStep {
    VerifyStep {
        group,
        name: entry.name.clone(),
        run: entry.run.clone(),
        state: state.clone(),
    }
}

/// Each repository's latest Verify, underway or ended, keyed by its checkout's
/// root — `Owner::Checkout`'s key, since one Verify holds one tree. Taking a
/// [`Served`] leaves no way to ask after a Job's. `std`'s lock, never held
/// across an `.await`.
#[derive(Clone, Default)]
pub(crate) struct Verifies(Arc<Mutex<BTreeMap<String, Verifying>>>);

struct Verifying {
    id: String,
    started_at: Instant,
    ended_at: Option<Instant>,
    steps: Vec<(VerifyGroup, Entry, VerifyStepState)>,
}

impl Verifies {
    pub(crate) fn underway(&self, served: &Served) -> bool {
        self.held()
            .get(served.root())
            .is_some_and(|one| one.ended_at.is_none())
    }

    pub(crate) fn seen(&self, served: &Served) -> Option<CheckoutVerify> {
        self.held().get(served.root()).map(|one| CheckoutVerify {
            id: one.id.clone(),
            started_at: one.started_at.clone(),
            ended_at: one.ended_at.clone(),
            steps: one
                .steps
                .iter()
                .map(|(group, entry, state)| wired(*group, entry, state))
                .collect(),
        })
    }

    /// Hold `verifying` as the latest, or `false` where one is still underway.
    fn begin(&self, served: &Served, verifying: Verifying) -> bool {
        let mut held = self.held();
        if held
            .get(served.root())
            .is_some_and(|one| one.ended_at.is_none())
        {
            return false;
        }
        held.insert(served.root().to_string(), verifying);
        true
    }

    fn step(&self, served: &Served, id: &str, at: usize) -> Option<(VerifyGroup, Entry)> {
        self.with(served, id, |one| {
            one.steps
                .get(at)
                .map(|(group, entry, _)| (*group, entry.clone()))
        })
        .flatten()
    }

    fn moved(&self, served: &Served, id: &str, at: usize, state: VerifyStepState) {
        self.with(served, id, |one| {
            if let Some(step) = one.steps.get_mut(at) {
                step.2 = state;
            }
        });
    }

    /// End it, saying why of every step not reached.
    fn ended(&self, served: &Served, id: &str, why: &str, now: Instant) {
        self.with(served, id, |one| {
            for step in &mut one.steps {
                if matches!(step.2, VerifyStepState::Waiting) {
                    step.2 = VerifyStepState::NotRun {
                        why: why.to_string(),
                    };
                }
            }
            one.ended_at = Some(now);
        });
    }

    fn with<T>(
        &self,
        served: &Served,
        id: &str,
        change: impl FnOnce(&mut Verifying) -> T,
    ) -> Option<T> {
        self.held()
            .get_mut(served.root())
            .filter(|one| one.id == id)
            .map(change)
    }

    fn held(&self) -> MutexGuard<'_, BTreeMap<String, Verifying>> {
        self.0
            .lock()
            .unwrap_or_else(|poisoned| poisoned.into_inner())
    }
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
    /// Begin a Verify in the main checkout, and answer once its first step is
    /// out.
    pub(crate) async fn begin_checkout_verify(
        self: Arc<Self>,
        served: Served,
    ) -> Result<CheckoutVerify, Refusal> {
        let owner = Place::of_checkout(served.clone()).owner;
        let verifies = self.rehearsals().verifies().clone();
        if verifies.underway(&served) {
            return Err(self.refused_run(&owner, Unrehearsable::VerifyUnderway));
        }
        if let Some(out) = self.rehearsals().in_flight(&owner) {
            let why = Unrehearsable::AlreadyRunning {
                name: out.name,
                whose: Whose::Checkout,
            };
            return Err(self.refused_run(&owner, why));
        }
        let manifest = served.manifest().clone();
        let steps = entries::declared(&manifest).verified();
        if steps.is_empty() {
            return Err(self.refused_run(&owner, Unrehearsable::NothingToVerify));
        }
        let id = self.mint().ulid().as_str().to_string();
        let began = verifies.begin(
            &served,
            Verifying {
                id: id.clone(),
                started_at: Instant::from(&self.now()),
                ended_at: None,
                steps: steps
                    .into_iter()
                    .map(|(group, entry)| (group, entry, VerifyStepState::Waiting))
                    .collect(),
            },
        );
        if !began {
            return Err(self.refused_run(&owner, Unrehearsable::VerifyUnderway));
        }
        let (out, first) = oneshot::channel();
        tokio::spawn(Arc::clone(&self).verified(id, out, served.clone()));
        let _ = first.await;
        verifies.seen(&served).ok_or_else(|| {
            let why = Unrehearsable::NotKept {
                why: String::from("the Verify just begun is no longer held"),
            };
            self.refused_run(&owner, why)
        })
    }

    /// The steps, one after another. A step's hand-over is answered only once
    /// the step after it is out, or the Verify has ended.
    async fn verified(self: Arc<Self>, id: String, out: oneshot::Sender<()>, served: Served) {
        let verifies = self.rehearsals().verifies().clone();
        let mut waiting = vec![out];
        let mut at = 0;
        let why = loop {
            let Some((group, entry)) = verifies.step(&served, &id, at) else {
                break String::new();
            };
            let place = Place::of_checkout(served.clone());
            let Some(tree) = self.tree_at(&place) else {
                break String::from("this checkout is not on disk");
            };
            let (handing, handed) = oneshot::channel();
            let command = entry.run.clone();
            let started = Arc::clone(&self)
                .started_at(place, tree, entry, command, false, false, Some(handing))
                .await;
            match started {
                Ok(underway) => verifies.moved(
                    &served,
                    &id,
                    at,
                    VerifyStepState::Running {
                        run_id: underway.id,
                    },
                ),
                Err(why) => {
                    verifies.moved(
                        &served,
                        &id,
                        at,
                        VerifyStepState::NotRun {
                            why: why.to_string(),
                        },
                    );
                    at += 1;
                    continue;
                }
            }
            answered(&mut waiting);
            let Ok(Handed { record, ack }) = handed.await else {
                let why = String::from("its run ended without keeping a record");
                verifies.moved(
                    &served,
                    &id,
                    at,
                    VerifyStepState::NotRun { why: why.clone() },
                );
                break why;
            };
            waiting.push(ack);
            let cut = cut_short(group, &record);
            verifies.moved(
                &served,
                &id,
                at,
                VerifyStepState::Ran {
                    record: record.of_checkout(),
                },
            );
            at += 1;
            if let Some(why) = cut {
                break why;
            }
        };
        verifies.ended(&served, &id, &why, Instant::from(&self.now()));
        answered(&mut waiting);
    }
}

/// Let every step waiting on the hand-over publish its finish.
fn answered(waiting: &mut Vec<oneshot::Sender<()>>) {
    for ack in waiting.drain(..) {
        let _ = ack.send(());
    }
}

/// Why the steps after this one are not run, where they are not.
///
/// **A stop ends it**: Stop was pressed on the Verify, through its step. **A
/// setup step that did not exit as expected ends it too**, the way a Job's
/// worktree stops preparing and never reaches a Check (`crate::preparing`).
/// Journey 9 does not settle this for Verify; #719 carries the question.
fn cut_short(group: VerifyGroup, record: &Record) -> Option<String> {
    if record.stopped {
        return Some(format!("Verify was stopped during `{}`", record.name));
    }
    let as_expected = record.exit_code.map(i64::from) == Some(record.expect_exit_code);
    (group == VerifyGroup::Setup && !as_expected)
        .then(|| format!("setup `{}` {}", record.name, record.ended))
}
