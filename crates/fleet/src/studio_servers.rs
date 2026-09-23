//! Servers a Studio holds: one started from a Studio, and what its node keeps
//! when it ends. `#1345`, `docs/concepts/studio.md`.
//!
//! **A server is a Run node, not a fifteenth kind.** `studio.md` says a Run
//! node holds *a Manifest command started from the Studio*, and a Command with
//! `serve` is one — and Run is one of only two kinds that may take status
//! colour, which *starting* and *serving* need.
//!
//! **The node reads the live holder.** Fleet keeps one instance per holder per
//! name in memory, so a Studio asking for one already serving is answered with
//! that one, and the node holds its id rather than a copy of anything.
//!
//! **Kept when it ends, not when its directory is swept.** A server's record
//! is `crate::servers::held`'s memory, which a restart takes, so the instant
//! it ends is the last moment there is anything to keep.

use std::path::Path;
use std::sync::Arc;

use adapter_traits::{AgentHarness, Delivery, Vcs, WorkProduct};
use api::{Redirector, Refusal};
use core_model::{
    StudioEdgeId, StudioNode, StudioNodeContent, StudioNodeId, StudioRun, StudioRunKept,
};
use ipc::{RunOutput, ServerState, StartStudioServer, StudioServerStarted};

use crate::daemon::Fleet;
use crate::rehearsing::records;

/// What a Run node holding a server keeps as `expect_exit_code`.
///
/// **Nothing reads it, and that is the point.** A server that exits on its own
/// has failed whatever its code — `docs/concepts/manifest.md` — so a reader
/// takes its colour from `stopped` alone.
const A_SERVER_EXPECTS_NOTHING: i64 = 0;

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
    /// Start one server in the checkout the Studio's repository stands in, and
    /// put a Run node on the Studio holding it.
    ///
    /// **The server starts first**, for `started_studio_run`'s reason: a node
    /// written before it would be left naming a start that was refused — for a
    /// name nothing declares, or for a Command that is not a server.
    ///
    /// **The Studio names the repository**, so nothing on this call can reach
    /// another checkout, and the ports come from that checkout's own span.
    pub(crate) async fn started_studio_server(
        self: Arc<Self>,
        studio_id: ipc::StudioId,
        asked: StartStudioServer,
        by: Redirector,
        within: Option<ipc::ManifestId>,
    ) -> Result<StudioServerStarted, Refusal> {
        let repository = {
            let store = self.store().lock().await;
            let graph = self.studio_held(&store, &studio_id.to_domain(), within.as_ref())?;
            ipc::ManifestId::from(&graph.studio.manifest_id)
        };
        let served = self.served_named(Some(&repository))?;
        let refusing = Arc::clone(&self);
        let (server, fresh) = Fleet::hold_server(
            Arc::clone(&self),
            crate::servers::Place::Checkout(crate::checkouts::Checkout::main(served)),
            &asked.name,
            ipc::StartedBy::Person,
        )
        .await
        .map_err(|why| refusing.server_refusal(why, None))?;
        let at = self.now();
        let node = StudioNode::added(
            StudioNodeId::carried(self.mint().ulid()),
            StudioNodeContent::Run {
                run: StudioRun::Server(server.id.clone()),
                kept: None,
            },
            asked.position.to_domain(),
            at.clone(),
            crate::studios::author(by),
        );
        let produced_by = asked
            .produced_by
            .map(|from| (from.to_domain(), StudioEdgeId::carried(self.mint().ulid())));
        let studio = self
            .written(&studio_id, within, |store, id| {
                let produced_by = produced_by
                    .as_ref()
                    .map(|(from, edge)| (from, edge.clone()));
                store.add_studio_node(id, &node, produced_by, &at)
            })
            .await?;
        Ok(StudioServerStarted {
            studio,
            node_id: ipc::StudioNodeId::from(node.id()),
            server,
            already_up: !fresh,
        })
    }

    /// Write what a server that has just ended said onto every Run node holding
    /// it, and publish each Studio that changed.
    ///
    /// **Called from `crate::servers::running`'s own task, as it ends**, so the
    /// log is read while it is complete and the result while it is in memory.
    ///
    /// **A node that could not be written is left as it was.** There is no
    /// directory to hold back — a server's is swept by its age like any other —
    /// so a failure here leaves a node naming an instance nobody can read,
    /// which draws as *not read yet* rather than as a wrong result.
    pub(crate) async fn kept_what_studios_hold_of_server(&self, state: &ServerState, dir: &Path) {
        let printed = records::output_of(
            &dir.join(records::LOG),
            state.id.clone(),
            state.name.clone(),
            state.log.clone(),
        );
        let kept = server_kept(state, printed.as_ref());
        let touched = {
            let mut store = self.store().lock().await;
            store
                .keep_studio_server(&state.id, &kept)
                .unwrap_or_default()
        };
        for studio_id in touched {
            let read = {
                let store = self.store().lock().await;
                store.studio(&studio_id)
            };
            if let Ok(graph) = read {
                self.events()
                    .publish(ipc::Event::StudioChanged(ipc::Studio::of(&graph)));
            }
        }
    }
}

/// What a Run node keeps of a server that has ended: its name, the `serve` line
/// as it ran, how it ended, how long it was up, and the log's tail.
///
/// **`crate::studio_runs::kept`, with the run's record swapped for the
/// instance**, down to the bounds and to `printed` being `None` where the log
/// would not read — the node keeps the result with no lines under it rather
/// than nothing at all.
///
/// **`expect_exit_code` is written and not read.** A server that exits on its
/// own has failed whatever its code — `docs/concepts/manifest.md` — so a reader
/// takes its colour from `stopped` alone.
pub fn server_kept(state: &ServerState, printed: Option<&RunOutput>) -> StudioRunKept {
    let (lines, whole) = match printed {
        None => (Vec::new(), false),
        Some(printed) => {
            let (window, first, _) = crate::check_output::windowed(
                printed.lines.iter().cloned(),
                crate::studio_runs::LAST_LINES,
                crate::studio_runs::LAST_BYTES,
            );
            (Vec::from(window), printed.whole && first == 1)
        }
    };
    let ended = state.ended_at.clone().unwrap_or(state.started_at.clone());
    let up = crate::converging::elapsed(&state.started_at.to_domain(), &ended.to_domain());
    StudioRunKept {
        name: state.name.clone(),
        command: state.serve.clone(),
        exit_code: state.exit_code,
        expect_exit_code: A_SERVER_EXPECTS_NOTHING,
        stopped: state.stopped,
        duration_ms: u64::try_from(up.as_millis()).unwrap_or(u64::MAX),
        lines,
        total_lines: printed
            .map(|printed| printed.total_lines)
            .unwrap_or_default(),
        whole,
    }
}
