//! Jobs whose Checks failed on a test another Job is fixing, pointed at that
//! fix and told how it settles. #1001.
//!
//! **Spotted, never asked for**: a failed Check a claim names whose output
//! carries the claimed test's name. **It informs and passes nothing** — a
//! pointed Job's gate still fails on that test until the fix lands.
//!
//! **A claim waits for the merge.** A fix Job completes before its pull request
//! merges, so its claim and its pointers are given back when the forge settles
//! it, or when the Job ends any other way.

use adapter_traits::{AgentHarness, Delivery, Vcs, WorkProduct};
use checks_runner::Output;
use core_model::{BreakageClaim, FixWaiter, Job, JobId, JobStatus, ManifestId, Ulid};

use crate::daemon::Fleet;
use crate::peers::News;

/// Each failed Check and both streams it printed, from the names that failed
/// and what every Check printed.
pub(crate) fn failures_said<'a>(
    failed: impl IntoIterator<Item = &'a str>,
    printed: &[(&str, &Output)],
) -> Vec<(String, String)> {
    failed
        .into_iter()
        .filter_map(|check| {
            printed
                .iter()
                .find(|(name, _)| *name == check)
                .map(|(_, output)| {
                    (
                        check.to_string(),
                        format!("{}\n{}", output.stdout, output.stderr),
                    )
                })
        })
        .collect()
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
    /// Point this Job at every claimed fix in its repository whose test one of
    /// its failed Checks printed. **Nothing here can fail the gate or the dry
    /// run that reached it**: a read that will not answer points nothing.
    pub(crate) async fn pointed_at_fixes(&self, job: &JobId, failed: &[(String, String)]) {
        if failed.is_empty() {
            return;
        }
        let Some(owner) = self.names().owner_of(job) else {
            return;
        };
        let repository = ManifestId::carried(Ulid::carried(owner));
        let Ok(claims) = self.store().lock().await.breakages_claimed_in(&repository) else {
            return;
        };
        for claim in claims {
            // The fix and the Job that reported the test already know.
            if claim.fix == *job || claim.reported_by == *job {
                continue;
            }
            let named = failed.iter().any(|(check, said)| {
                *check == claim.breakage.check && said.contains(&claim.breakage.test)
            });
            if named {
                self.point_at(job, &claim, true).await;
            }
        }
    }

    /// Record the pointer and, the first time and where `tell` asks, queue the
    /// news for the Job's Drone.
    pub(crate) async fn point_at(&self, job: &JobId, claim: &BreakageClaim, tell: bool) {
        let waiter = FixWaiter {
            waiting: job.clone(),
            fix: claim.fix.clone(),
            repository: claim.repository.clone(),
            check: claim.breakage.check.clone(),
            test: claim.breakage.test.clone(),
        };
        let now = self.now();
        let pointed = self
            .store()
            .lock()
            .await
            .point_at_fix(&waiter, &now)
            .unwrap_or(false);
        if !pointed || !tell {
            return;
        }
        let Ok(fix) = self.load(&claim.fix).await else {
            return;
        };
        self.owe(
            job,
            News::Fixing {
                title: fix.title().as_str().to_string(),
                handle: fix.handle(),
                test: waiter.test,
            },
        )
        .await;
    }

    /// A Job that ended: its claims wait for the merge where its pull request
    /// is still open, and are given back now otherwise.
    pub(crate) async fn fix_ended(&self, job: &Job) {
        let awaiting_merge = job.status() == JobStatus::CompletedSuccess
            && self
                .store()
                .lock()
                .await
                .pull_requests_unsettled()
                .is_ok_and(|open| open.iter().any(|pull| pull.job_id == *job.id()));
        if !awaiting_merge {
            self.fix_settled(job.id(), false).await;
        }
    }

    /// Tell every Job pointed at this fix whether it landed, then give back the
    /// fix's claims and the pointers at it. A Job fixing nothing has neither.
    pub(crate) async fn fix_settled(&self, fix: &JobId, landed: bool) {
        let waiting = self
            .store()
            .lock()
            .await
            .waiting_on_fix(fix)
            .unwrap_or_default();
        if !waiting.is_empty() {
            if let Ok(record) = self.load(fix).await {
                for waiter in waiting {
                    let title = record.title().as_str().to_string();
                    let handle = record.handle();
                    let test = waiter.test;
                    let news = match landed {
                        true => News::FixLanded {
                            title,
                            handle,
                            test,
                        },
                        false => News::FixGone {
                            title,
                            handle,
                            test,
                        },
                    };
                    self.owe(&waiter.waiting, news).await;
                }
            }
        }
        let mut store = self.store().lock().await;
        let _ = store.release_waiters(fix);
        let _ = store.release_breakages(fix);
    }
}
