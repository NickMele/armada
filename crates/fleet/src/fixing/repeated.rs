//! Fleet noticing, with no Drone asking, that a step's Check failed the same
//! way twice in a row — and running the same probe against main `draft_fix`
//! runs when asked, claiming the same way. #828, #999, #1001.
//!
//! **Spotted, never asked for** — `fixing::waiting`'s own words about #1001 —
//! off the same two attempts' recorded Checks, and nothing a Drone said.
//!
//! # How strict "the same failing test" is
//!
//! Only a Check whose failing output names **exactly one** test on **both**
//! attempts fires this. Several failing tests on either attempt cannot say
//! which of the other's several is the same one without guessing, and firing
//! on a wrong guess drafts a fix Job a person has to notice and close by
//! hand — the safer direction to be wrong in, `NotFixed`'s own refusals
//! already read that way.

use std::path::Path;

use adapter_traits::{AgentHarness, Delivery, Vcs, WorkProduct};
use checks_runner::OneTestRan;
use core_model::{Job, JobId, StepCheck, StepId};
use ipc::mcp::DraftFix;

use super::{Drafted, NotFixed};
use crate::checking::Stop;
use crate::daemon::Fleet;
use crate::gate::Ruling;

/// One Check on one step, naming the same one failing test on this attempt
/// and the one before it.
#[derive(Clone, Debug, PartialEq, Eq)]
pub(crate) struct Repeat {
    pub(crate) step: StepId,
    pub(crate) check: String,
    pub(crate) test: String,
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
    /// Every Check this ruling failed that also failed, naming the same one
    /// test, on the attempt before it. Empty on anything but a hand-back:
    /// every other ruling either ends the step or has no "before" worth
    /// reading yet.
    pub(crate) async fn spotted_repeats(
        &self,
        job_id: &JobId,
        step: &StepId,
        ruling: &Ruling,
    ) -> Vec<Repeat> {
        if !matches!(ruling, Ruling::HandedBack { .. }) {
            return Vec::new();
        }
        let Some(before) = self.checks_before_this_attempt(job_id, step).await else {
            return Vec::new();
        };
        let Ok(served) = self.served_by_id(job_id) else {
            return Vec::new();
        };
        let root = served.records_root().to_string();
        ruling
            .checks()
            .iter()
            .filter(|check| !check.outcome.advances())
            .filter_map(|check| {
                let now = ruling
                    .output()
                    .iter()
                    .find(|kept| kept.check == check.name)
                    .map(|kept| format!("{}\n{}", kept.output.stdout, kept.output.stderr))?;
                let earlier = before
                    .iter()
                    .find(|kept| kept.name == check.name && !kept.outcome.advances())
                    .and_then(|kept| read_kept(&root, kept))?;
                same_one_test(&now, &earlier).map(|test| Repeat {
                    step: step.clone(),
                    check: check.name.clone(),
                    test,
                })
            })
            .collect()
    }

    /// This step's recorded Checks from the attempt before this one. `None`
    /// on a step's first attempt, with nothing yet to compare against —
    /// `recorded_checks` has already written this attempt's own group by the
    /// time a ruling reaches here, so that one is the last and never `before`.
    async fn checks_before_this_attempt(
        &self,
        job_id: &JobId,
        step: &StepId,
    ) -> Option<Vec<StepCheck>> {
        let groups = self
            .store()
            .lock()
            .await
            .step_checks_every_attempt(job_id)
            .ok()?;
        let mut of_this_step: Vec<_> = groups.into_iter().filter(|g| &g.step_id == step).collect();
        (of_this_step.len() >= 2).then(|| of_this_step.remove(of_this_step.len() - 2).record)
    }

    /// The same probe `draft_fix` runs for a Drone's own claim, for a repeat
    /// Fleet spotted itself. `None` where nothing could even be asked about
    /// — a Check the workflow no longer declares, say; the tri-state itself,
    /// where it ran, is `draft_fix`'s own three answers.
    pub(crate) async fn probed_repeat(
        &self,
        job: &JobId,
        repeat: &Repeat,
    ) -> Option<Result<Drafted, NotFixed>> {
        let request = self
            .requested(
                job.clone(),
                repeat.step.clone(),
                &repeat.check,
                &repeat.test,
            )
            .await
            .ok()?;
        let claimed = self
            .store()
            .lock()
            .await
            .breakage_claimed(&request.repository, &repeat.check, &repeat.test)
            .ok()
            .flatten();
        if claimed.is_some() {
            // #1001's claim path already answers a `draft_fix` call reaching
            // the same test later; nothing new is spent probing it again.
            return None;
        }
        if !self
            .fixing_on_main()
            .lock()
            .await
            .insert(request.root.clone())
        {
            // Main is already busy for this repository. Left for a later
            // repeat: a failure still there names itself again next attempt.
            return None;
        }
        let ran = self.run_on_main(&request, &Stop::never()).await;
        self.fixing_on_main().lock().await.remove(&request.root);
        Some(match ran {
            Err(why) => Err(why),
            Ok(OneTestRan::Passed) => Err(NotFixed::PassesOnMain {
                test: repeat.test.clone(),
            }),
            Ok(OneTestRan::NoMatch) => Err(NotFixed::NoMatch {
                test: repeat.test.clone(),
            }),
            Ok(OneTestRan::Failed) => {
                let fix = synthesized(&repeat.check, &repeat.test, &request.record);
                self.drafted_fix(job, &fix, &request, &repeat.test).await
            }
        })
    }
}

/// The one test both texts name as failing, where each names exactly one —
/// this module's own doc on how strict that match is.
fn same_one_test(now: &str, before: &str) -> Option<String> {
    let now = checks_runner::failing_tests_in(now);
    let before = checks_runner::failing_tests_in(before);
    match (now.as_slice(), before.as_slice()) {
        ([a], [b]) if a == b => Some(a.clone()),
        _ => None,
    }
}

/// A recorded Check's own output, read back from where `check_output::kept`
/// wrote it.
fn read_kept(root: &str, check: &StepCheck) -> Option<String> {
    let path = check.output_path.as_deref()?;
    std::fs::read_to_string(Path::new(root).join(path)).ok()
}

/// The fix, as Fleet states it for itself. No Drone names a title or a
/// workflow here, so this reuses the retrying Job's own — `DraftFix`'s own
/// doc says the last four are `dispatch_job`'s because a drafted Job is
/// judged against a real workflow, and this one already runs under one.
fn synthesized(check: &str, test: &str, record: &Job) -> DraftFix {
    let handle = record.handle();
    DraftFix {
        check: check.to_string(),
        test: test.to_string(),
        failure: format!("`{test}` fails under `{check}` on main, the same as on {handle}."),
        title: format!("Fix `{test}` failing on main under `{check}`"),
        workflow: record.workflow_id().as_str().to_string(),
        brief: format!(
            "`{check}` failed on `{test}` on two consecutive attempts of {handle}'s own \
             step, with no Drone asking, and the same test fails on main too."
        ),
        acceptance_criteria: vec![format!("`{test}` passes under the `{check}` Check.")],
    }
}
