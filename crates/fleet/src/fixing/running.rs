//! Running the named test on main, and reading what came of it. Split out of
//! `fixing` to keep that file under the line ask (`4aed6e69` did the same for
//! its refusals and claim reads).

use std::path::Path;

use adapter_traits::{AgentHarness, Delivery, Vcs, WorkProduct};
use verification::{Exit, Observed};

use super::{NotFixed, Request};
use crate::checking::Stop;
use crate::daemon::Fleet;

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
    /// What running the test on main came to.
    pub(super) async fn run_on_main(
        &self,
        request: &Request,
        stop: &Stop,
    ) -> Result<checks_runner::OneTestRan, NotFixed> {
        // Matched here rather than read off `NoBase::said`, whose sentences are
        // about photographing a before; these say what running a test there met.
        let (served, checkout) = self
            .base_to_show_from(&request.record)
            .await
            .map_err(|why| NotFixed::NoMain {
                why: match why {
                    crate::basing::NoBase::Unnamed => String::from(
                        "this repository names no base branch, so there is no main to run it against",
                    ),
                    crate::basing::NoBase::NotCheckedOut { why } => {
                        format!("the checkout of main could not be made — {why}")
                    }
                    crate::basing::NoBase::NotPrepared { command, why } => format!(
                        "`setup.requires` did not finish in the checkout of main: `{command}` — {why}"
                    ),
                },
            })?;
        let ports = self.main_checkout_ports(&served).await;
        let env = self.main_checkout_port_env(&served).await;
        let completed = crate::checking::ran(
            std::slice::from_ref(&request.run),
            &[],
            false,
            false,
            Path::new(checkout.path()),
            self.budget().duration(),
            &self.room(crate::places::Asking::FixDraft),
            &crate::underway::Announcing::nowhere(),
            &ports,
            &env,
            None,
            stop,
            // No Drone and no dry run: proving a fix against main is not
            // anything a Drone asked about first.
            None,
            core_model::Attempt::FIRST,
            None,
        )
        .await;
        // Kept apart from a whole run of this Check — the test alone is
        // seconds where the Check is minutes. #1072.
        if let Some(took) = completed.iter().find_map(|done| match &done.observed {
            Observed::Command(Exit::Code(_)) => Some(done.took),
            _ => None,
        }) {
            self.kept_one_test_timing(&request.repository, request.run.label(), took)
                .await;
        }
        let last = completed
            .iter()
            .rev()
            .find_map(|done| match &done.observed {
                Observed::Command(exit) => Some((exit, done.printed.as_ref().map(|(_, out)| out))),
                _ => None,
            });
        match last {
            Some((Exit::NeverRan(_), _)) | None => Err(NotFixed::NeverRan),
            // The exit code alone cannot tell a pass apart from a name that
            // matched nothing at all — both nextest and vitest can report
            // either the same way — so the runner's own summary decides.
            // `checks_runner::matched` says why.
            Some((exit, output)) => Ok(checks_runner::one_test_ran(
                exit,
                output.unwrap_or(&checks_runner::Output::default()),
                request.expect_exit_code,
            )),
        }
    }
}
