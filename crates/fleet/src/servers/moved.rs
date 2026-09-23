//! A held server whose checkout moved on under it — `#1564`.
//!
//! **Telling is the floor and restarting is a choice.** A person watching a
//! preview while Jobs land is worse off for a restart they did not ask for
//! than for a row that says what they are looking at is three commits behind.
//! Nothing here starts or stops a process; it moves a number on a row and
//! publishes it.
//!
//! **Only the main checkout.** A Job's worktree is its own branch and does not
//! gain the commits a merge puts on the base, so nothing lands under a Job's
//! server in the sense this module means.

use std::sync::Arc;

use adapter_traits::{AgentHarness, Delivery, RepositoryStanding, Vcs, WorkProduct};
use ipc::{Event, ServerPhase};

use super::Holder;
use crate::daemon::Fleet;
use crate::repositories::Served;

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
    /// The commit this repository's base branch stands at, read without a
    /// worktree — what a server started in the main checkout is serving.
    ///
    /// **Recorded so that later it can be contradicted.** A server carries the
    /// commit it came up on because that is the only thing "behind by three"
    /// can be counted from once the checkout has moved.
    ///
    /// `None` is a base that would not read, which is reported as an absent
    /// commit and never as agreement — `Delivery::base_tip`'s own rule.
    pub(crate) async fn commit_serving_now(&self, served: &Served) -> Option<String> {
        let vcs = Arc::clone(self.vcs());
        let root = served.root().to_string();
        let base = served.manifest().base().map(str::to_string);
        tokio::task::spawn_blocking(move || vcs.base_commit(&root, base.as_deref()))
            .await
            .ok()?
            .ok()?
    }

    /// A merge brought this repository's main checkout forward: every server
    /// held on it is now that far behind what it serves, and says so.
    ///
    /// **Nothing but [`MovedOn`] is news.** A checkout that already had the
    /// merge, or was left alone over somebody's uncommitted work, did not move
    /// under the servers standing in it.
    ///
    /// [`MovedOn`]: RepositoryStanding::MovedOn
    pub(crate) fn told_servers_the_checkout_moved(
        &self,
        root: &str,
        standing: &RepositoryStanding,
    ) {
        let RepositoryStanding::MovedOn { commits, .. } = standing else {
            return;
        };
        let Ok(commits) = u32::try_from(*commits) else {
            return;
        };
        if commits == 0 {
            return;
        }
        let holder = Holder::MainCheckout(root.to_string());
        for state in self.servers().moved_on(&holder, commits) {
            // The whole row in the kind its phase is, because the three
            // `server.*` kinds each carry the whole `ServerState` and a reader
            // replaces a row rather than patching it — `crates/ipc/operations.toml`.
            let event = match state.phase {
                ServerPhase::Serving => Event::ServerServing(state),
                ServerPhase::Starting => Event::ServerStarting(state),
                // Unreachable: `moved_on` walks what is held, and an instance
                // that ended is not held. Skipped rather than published under
                // a kind that would say it had come back up.
                ServerPhase::Exited => continue,
            };
            self.publish(event);
        }
    }
}
