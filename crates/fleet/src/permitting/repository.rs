//! Commands a person always-allowed for the whole repository, kept by Fleet
//! itself — `#836`, reversing `#643`'s *"an always allow lands on the Job's
//! branch"*. `crate::permitting::holding` is where a Drone's call and a
//! person's answer meet; this is where the repository-wide half of that
//! answer is kept and read back, for both a live permission question and the
//! ipc surface that lists and removes one.

use adapter_traits::{AgentHarness, Delivery, Vcs, WorkProduct};
use api::Refusal;
use core_model::{Actor, AllowedCommand, Reach};
use ipc::WireError;

use crate::daemon::Fleet;
use crate::permitting::{covers, NotPermitted};

/// Not job-scoped, so it earns its own code rather than joining
/// `crate::adrift::Adrift`'s job-scoped table — `Fleet::server_refusal` is the
/// same call: a small refusal built beside the surface it answers rather than
/// grown into that enum.
const NOT_PERMITTED_IN_REPOSITORY: &str = "fleet.not_permitted_in_repository";

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
    /// Every rule always-allowed for this Fleet's Manifest, **grantable right
    /// now** — a rule the Manifest has since declared destructive is filtered
    /// out here rather than trusted from when it was allowed, so a person
    /// tightening `armada.yml` narrows what an old always-allow still reaches.
    ///
    /// Read by [`holding::first_answer`](super::holding) beside a Job's own
    /// row, and by [`crate::spawning`]'s toolbelt, which is why this lives
    /// apart from [`repository_allowed`](Fleet::repository_allowed): a
    /// listing for a person shows every row it ever wrote, and a grant does
    /// not.
    pub(crate) async fn repository_allowed_commands(&self) -> Vec<AllowedCommand> {
        let stored = self.repository_allowed().await;
        let destructive = self.destructive_commands();
        stored
            .into_iter()
            .filter(|allowed| !destructive.iter().any(|(_, run)| covers(run, &allowed.run)))
            .collect()
    }

    /// Write down that a person always-allowed this rule for the repository.
    /// **Commits nothing** — see the module doc.
    pub(crate) async fn allow_in_repository(&self, run: &str) -> Result<(), NotPermitted> {
        let allowed = AllowedCommand {
            run: run.to_string(),
            reach: Reach::Repository,
            allowed_at: self.now(),
            by: Actor::Human,
        };
        self.store()
            .lock()
            .await
            .allow_repository_command(self.manifest().id(), &allowed)
            .map_err(|cause| NotPermitted::NotRecorded {
                cause: cause.to_string(),
            })
    }

    /// `get_repository_allowed_commands` — every rule a person always-allowed
    /// for this Manifest, oldest first, whether or not `armada.yml` still
    /// leaves it grantable. **Empty where the store will not read**, which is
    /// what [`repository_allowed_commands`](Fleet::repository_allowed_commands)
    /// reads too.
    pub async fn repository_allowed(&self) -> Vec<AllowedCommand> {
        self.store()
            .lock()
            .await
            .repository_allowed_commands(self.manifest().id())
            .unwrap_or_default()
    }

    /// `remove_repository_allowed_command` — take back a rule a person
    /// always-allowed for this Manifest. **Read by the next permission
    /// question and the next spawn**, so nothing respawns and no Job is
    /// touched: a Job already carrying a Drone spawned under the old grant
    /// keeps it for the step it is on.
    ///
    /// Refused where nothing a person allowed this Manifest is spelled `run`.
    pub async fn remove_repository_allowed_command(&self, run: &str) -> Result<(), NotPermitted> {
        let removed = self
            .store()
            .lock()
            .await
            .remove_repository_allowed_command(self.manifest().id(), run)
            .map_err(|cause| NotPermitted::NotChanged {
                cause: cause.to_string(),
            })?;
        if !removed {
            return Err(NotPermitted::NothingAllowed {
                run: run.to_string(),
            });
        }
        Ok(())
    }

    /// [`NotPermitted`] from this file, as a 409 with no Job to name.
    pub(crate) fn repository_allow_refusal(&self, why: NotPermitted) -> Refusal {
        Refusal::Unacceptable(WireError::raised(
            NOT_PERMITTED_IN_REPOSITORY,
            why.to_string(),
            self.run_id(),
        ))
    }
}
