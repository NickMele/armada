//! Reading a Link in: the source fetched by Fleet, and what comes back hung
//! off the Link. `#1293`, `docs/concepts/studio.md`, *Promotion*.
//!
//! **Fleet fetches and the scout is handed text.** A scout's launch denies
//! every tool that could reach a source and carries no MCP server — the
//! confinement spike 017 measured — so the fetch happens here, in Fleet's own
//! unsandboxed process, as it already does for a bare issue link in a request.
//!
//! **The fetch is before the write.** A source that will not fetch is a
//! refusal with no node behind it, rather than a Finding that failed at once.
//!
//! **A milestone takes no scout.** It is a list, and a model asked to echo one
//! back is cost spent on a transcription; Fleet mints a Link per issue itself.

use std::sync::Arc;
use std::time::Duration;

use adapter_traits::{AgentHarness, Delivery, LookupCall, Vcs, WorkProduct};
use adapters::{Fetch, MilestoneRead, Source};
use api::{Redirector, Refusal};
use core_model::{
    FrozenFinding, ScoutSource, ScoutSourceKind, StudioAuthor, StudioEdge, StudioEdgeId,
    StudioFinding, StudioId, StudioNode, StudioNodeContent, StudioNodeId, StudioPosition,
    StudioRelation,
};
use ipc::{HelmStudioAct, ManifestId, ReadInLink};
use tokio::process::Command;
use tokio::time::timeout;

use crate::daemon::Fleet;
use crate::promoting::NOT_A_LINK;
use crate::studios::author;

/// A Link to a board, a wiki or anything else no scout reads. A 422.
const STAYS_A_LINK: &str = "fleet.studio_link_stays_a_link";
/// A source Fleet could not fetch. A 422, and no node is written.
const UNREADABLE: &str = "fleet.studio_source_unreadable";

/// The most of a source's text one scout is handed.
///
/// **A bound because a source is somebody else's**: a 114-turn session and a
/// documentation page are both unbounded, and what did not fit is recorded on
/// the Finding rather than dropped quietly.
const MOST_CHARACTERS: usize = 120_000;

/// How far apart nodes a read-in made are placed, in canvas units.
const ACROSS: i64 = 340;
const DOWN: i64 = 180;
/// How many nodes a read-in puts in one column before starting another.
const DOWN_A_COLUMN: i64 = 6;

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
    /// `read_in_link`. The node survives and keeps its address; what comes
    /// back hangs off it by `produced` edges.
    pub(crate) async fn link_read_in(
        self: Arc<Self>,
        studio_id: ipc::StudioId,
        read_in: ReadInLink,
        by: Redirector,
        within: Option<ManifestId>,
    ) -> Result<ipc::Studio, Refusal> {
        let id = studio_id.to_domain();
        let link = read_in.node_id.to_domain();
        let source_node = self.source_node(&id, &link, within.as_ref()).await?;
        let address = String::from(
            source_node
                .content()
                .address()
                .expect("a node `source_node` answered keeps an address"),
        );
        let (root, home) = self.checkout_and_home(&id, within.as_ref()).await?;
        let source = adapters::source_of(&address, &root, &home).ok_or_else(|| {
            self.studio_unacceptable(
                STAYS_A_LINK,
                adapters::StaysALink {
                    address: address.clone(),
                }
                .to_string(),
            )
        })?;
        let printed = self.fetched(&source, &id, within.as_ref()).await?;
        match &source {
            Source::Milestone { number, .. } => {
                let read = adapters::milestone_read(&printed, number);
                self.milestone_read_in(&studio_id, &source_node, read, read_in.position, by, within)
                    .await
            }
            _ => {
                self.scout_reading_in(
                    &studio_id,
                    &source_node,
                    &address,
                    source,
                    printed,
                    root,
                    read_in.position,
                    by,
                    within,
                )
                .await
            }
        }
    }

    /// An Epic: one Issue node per issue, straight from what the forge
    /// printed. **No scout and no Finding** — nothing was learned, a list was
    /// copied, and the node that would say what it cost would be saying nought.
    ///
    /// **Every field on every Issue node is filled.** A milestone's own read
    /// already answers each issue's number, title and state, so nothing here
    /// is left for a later fetch — `#1394`.
    #[allow(clippy::too_many_arguments)]
    async fn milestone_read_in(
        &self,
        studio_id: &ipc::StudioId,
        source: &StudioNode,
        read: MilestoneRead,
        at_position: ipc::StudioPosition,
        by: Redirector,
        within: Option<ManifestId>,
    ) -> Result<ipc::Studio, Refusal> {
        let at = self.now();
        let author = author(by);
        let from = at_position.to_domain();
        let link = source.id().clone();
        let mut made: Vec<(StudioNode, StudioEdgeId)> = Vec::new();
        for (n, issue) in read.issues.iter().enumerate() {
            let Some(content) = issue.node() else {
                continue;
            };
            made.push((
                StudioNode::added(
                    StudioNodeId::carried(self.mint().ulid()),
                    content,
                    laid_out(from, n),
                    at.clone(),
                    author,
                ),
                StudioEdgeId::carried(self.mint().ulid()),
            ));
        }
        // **What the Epic itself now says**, which is where a capped read is
        // recorded: the node survives with its address and gains its title and
        // how many of how many of its issues are on the board.
        let resolved = source.resolved(&read.facts()).ok();
        let node_ids: Vec<ipc::StudioNodeId> = made
            .iter()
            .map(|(node, _)| ipc::StudioNodeId::from(node.id()))
            .collect();
        let studio = self
            .written(studio_id, within, |store, id| {
                store.keep_read_in(id, &link, resolved.as_ref(), &made, &[], &at)
            })
            .await?;
        self.published_as_helms(
            by,
            &studio,
            HelmStudioAct::ReadIn {
                from: ipc::StudioNodeId::from(&link),
                node_ids,
            },
        );
        Ok(studio)
    }

    /// Every other source: a Finding Gathering, produced by the node it was
    /// read in on, and a scout told the text on its one turn.
    ///
    /// **An Issue or a Pull request also gains what the forge said about it**
    /// — its title and where it stands, off the first line of what was
    /// fetched, in the same write. `#1394`.
    #[allow(clippy::too_many_arguments)]
    async fn scout_reading_in(
        self: &Arc<Self>,
        studio_id: &ipc::StudioId,
        source_node: &StudioNode,
        address: &str,
        source: Source,
        printed: String,
        root: String,
        at_position: ipc::StudioPosition,
        by: Redirector,
        within: Option<ManifestId>,
    ) -> Result<ipc::Studio, Refusal> {
        let id = studio_id.to_domain();
        let link = source_node.id().clone();
        // Only an issue's and a pull request's fetch prints the line this
        // reads; a page, a session and a thread have no forge fields at all.
        let resolved = match &source {
            Source::Issue { .. } | Source::PullRequest { .. } => {
                source_node.resolved(&adapters::forge_facts(&printed)).ok()
            }
            _ => None,
        };
        let text = match &source {
            Source::Page { .. } => adapters::text_of_a_page(&printed),
            Source::Session { .. } => adapters::text_of_a_session(&printed),
            _ => printed,
        };
        let (text, cut) = adapters::bounded(&text, MOST_CHARACTERS);
        if text.trim().is_empty() {
            return Err(self.studio_unacceptable(
                UNREADABLE,
                format!("`{address}` came back with nothing to read"),
            ));
        }
        let checkout = self.checkout_read(&root).await?;
        let at = self.now();
        let proposed = StudioNode::added(
            StudioNodeId::carried(self.mint().ulid()),
            StudioNodeContent::Finding(StudioFinding::asked(&format!("Read in {address}"))),
            at_position.to_domain(),
            at.clone(),
            author(by),
        );
        let gathering = proposed
            .reading_in(
                checkout,
                vec![ScoutSource {
                    address: address.to_string(),
                    kind: kind_of(&source),
                    cut,
                }],
            )
            .expect("a node just added is a Proposed Finding");
        let edge = StudioEdgeId::carried(self.mint().ulid());
        let node_id = ipc::StudioNodeId::from(gathering.node().id());
        let made = [(gathering.node().clone(), edge)];
        let studio = self
            .written(studio_id, within.clone(), |store, id| {
                store.keep_read_in(id, &link, resolved.as_ref(), &made, &[], &at)
            })
            .await?;
        self.published_as_helms(
            by,
            &studio,
            HelmStudioAct::ReadIn {
                from: ipc::StudioNodeId::from(&link),
                node_ids: vec![node_id],
            },
        );
        let told = crate::scout::told_a_read_in(&root, &source.told(), &text);
        Arc::clone(self)
            .scouting(id, gathering, root, told, Some(link))
            .await;
        self.studio_now(studio_id, within).await
    }

    /// What a scout asked the Studio for, read off its answer and written.
    ///
    /// **Nothing here refuses.** The read-in is over by the time this runs and
    /// its Finding is already kept, so an answer that is not the shape leaves
    /// the Finding saying what the scout said and no node beside it.
    pub(crate) async fn what_came_back(
        &self,
        studio: &StudioId,
        link: &StudioNodeId,
        frozen: &FrozenFinding,
    ) {
        let StudioNodeContent::Finding(finding) = frozen.node().content() else {
            return;
        };
        let Some(learned) = finding.learned() else {
            return;
        };
        let Ok(read) = ipc::what_a_scout_read_in(learned) else {
            return;
        };
        if read.empty() {
            return;
        }
        let at = self.now();
        let from = frozen.node().position();
        let mut made: Vec<(StudioNode, StudioEdgeId)> = Vec::new();
        let mut by_handle: Vec<(String, StudioNodeId)> = Vec::new();
        let mut placed = 0usize;
        let mut mint = |content: StudioNodeContent, handle: Option<&str>| {
            let node = StudioNode::added(
                StudioNodeId::carried(self.mint().ulid()),
                content,
                laid_out(across(from), placed),
                at.clone(),
                // A scout runs on a person's ask, and a Studio records a
                // person or Helm: whoever asked owns what came back.
                frozen.node().added_by().unwrap_or(StudioAuthor::Person),
            );
            placed += 1;
            if let Some(handle) = handle {
                by_handle.push((handle.to_string(), node.id().clone()));
            }
            made.push((node, StudioEdgeId::carried(self.mint().ulid())));
        };
        for note in &read.notes {
            mint(
                StudioNodeContent::Note {
                    said: note.said.clone(),
                    capture: None,
                },
                Some(&note.id),
            );
        }
        for cluster in &read.clusters {
            mint(
                StudioNodeContent::Cluster {
                    title: cluster.title.clone(),
                },
                None,
            );
        }
        for one in &read.contradictions {
            mint(
                StudioNodeContent::Contradiction {
                    first: one.first.clone(),
                    second: one.second.clone(),
                    answer: None,
                },
                Some(&one.id),
            );
        }
        let named = |handle: &str| -> Option<StudioNodeId> {
            by_handle
                .iter()
                .find(|(given, _)| given == handle)
                .map(|(_, id)| id.clone())
        };
        let author = frozen.node().added_by().unwrap_or(StudioAuthor::Person);
        let mut edges: Vec<StudioEdge> = Vec::new();
        for relation in &read.relations {
            let (Some(from), Some(to)) = (named(&relation.from), named(&relation.to)) else {
                continue;
            };
            let Some(kind) = StudioRelation::from_wire(&relation.relation) else {
                continue;
            };
            // Proposed, whoever asked for it: only a person accepts a relation.
            if let Ok(edge) = StudioEdge::proposed(
                StudioEdgeId::carried(self.mint().ulid()),
                from,
                to,
                kind,
                at.clone(),
                author,
            ) {
                edges.push(edge);
            }
        }
        let kept = self
            .store()
            .lock()
            .await
            .keep_read_in(studio, link, None, &made, &edges, &at);
        let _ = kept;
    }

    /// The node a read-in was asked on, refused where it keeps no address.
    ///
    /// **Four kinds keep one** — a Link, an Issue, a Pull request and an Epic
    /// (`#1394`) — so reading in is every address node's rung rather than a
    /// Link's alone. The Link a person pasted and the Issue it turned out to
    /// be are the same node, and a rung that reached only one of them would
    /// depend on whether an adapter recognised the address.
    async fn source_node(
        &self,
        studio: &StudioId,
        node_id: &StudioNodeId,
        within: Option<&ManifestId>,
    ) -> Result<StudioNode, Refusal> {
        let store = self.store().lock().await;
        let graph = self.studio_held(&store, studio, within)?;
        let node = graph
            .nodes
            .iter()
            .find(|node| node.id() == node_id)
            .ok_or_else(|| self.no_such_node(node_id))?;
        match node.content().address() {
            Some(_) => Ok(node.clone()),
            None => Err(self.studio_unacceptable(
                NOT_A_LINK,
                format!(
                    "`{}` is a {} and keeps no address, so there is nothing to read in",
                    node_id.as_str(),
                    node.kind().as_wire()
                ),
            )),
        }
    }

    /// The repository's checkout, and the home the agent CLI keeps its
    /// sessions under.
    async fn checkout_and_home(
        &self,
        studio: &StudioId,
        within: Option<&ManifestId>,
    ) -> Result<(String, String), Refusal> {
        let root = self.checkout_of(studio, within).await?;
        Ok((root, self.host().home.clone()))
    }

    /// Run the fetch and answer with what it printed.
    async fn fetched(
        &self,
        source: &Source,
        studio: &StudioId,
        within: Option<&ManifestId>,
    ) -> Result<String, Refusal> {
        let unreadable = |why: String| self.studio_unacceptable(UNREADABLE, why);
        match adapters::fetching(source) {
            Fetch::Calls(calls) => {
                let mut printed = Vec::new();
                for call in &calls {
                    printed.push(ran(call, &self.host().path).await.map_err(&unreadable)?);
                }
                Ok(printed.join("\n"))
            }
            Fetch::File(file) => tokio::fs::read_to_string(&file)
                .await
                .map_err(|why| unreadable(format!("{} would not open: {why}", file.display()))),
            Fetch::Thread => self.helm_thread(studio, within).await,
        }
    }

    /// This repository's Helm thread, as what was said in it.
    ///
    /// **Fleet's own read and not a tool.** `observe_helm` refuses every agent
    /// — `crates/ipc/operations.toml` — so a scout reaching a thread through a
    /// tool would be that door opened; the file is read here instead and its
    /// prose handed over.
    async fn helm_thread(
        &self,
        studio: &StudioId,
        within: Option<&ManifestId>,
    ) -> Result<String, Refusal> {
        let manifest = {
            let store = self.store().lock().await;
            self.studio_held(&store, studio, within)?
                .studio
                .manifest_id
                .clone()
        };
        let served = self.served_named(Some(&ManifestId::from(&manifest)))?;
        let file =
            crate::helm::ConversationKey::of_repository(&manifest).thread_in(served.records_root());
        let read = tokio::fs::read_to_string(&file).await.map_err(|why| {
            self.studio_unacceptable(
                UNREADABLE,
                format!("this repository's Helm thread would not open: {why}"),
            )
        })?;
        Ok(crate::helm::what_was_said(&read))
    }
}

/// One rendered fetch, run in Fleet's own process. `crate::proposal`'s
/// `resolved`, with this read-in's own budget.
///
/// **`path` is the one Fleet gives every process it spawns**, so a fetch finds
/// the same programs a Drone would and a Fleet configured with a narrow PATH
/// does not quietly reach a wider one.
async fn ran(call: &LookupCall, path: &str) -> Result<String, String> {
    let mut spawning = Command::new(call.program());
    spawning
        .args(call.args())
        .env("PATH", path)
        .stdin(std::process::Stdio::null())
        .kill_on_drop(true);
    let output = timeout(
        Duration::from_secs(adapters::FETCH_SECONDS + 5),
        spawning.output(),
    )
    .await
    .map_err(|_| format!("`{}` took too long", call.program()))?
    .map_err(|error| format!("`{}` would not run: {error}", call.program()))?;
    if !output.status.success() {
        let said = String::from_utf8_lossy(&output.stderr).trim().to_string();
        return Err(format!("`{}` said: {said}", call.program()));
    }
    Ok(String::from_utf8_lossy(&output.stdout).to_string())
}

/// The `n`th node of a read-in, down a column and then across.
fn laid_out(from: StudioPosition, n: usize) -> StudioPosition {
    let n = n as i64;
    StudioPosition {
        x: from.x + (n / DOWN_A_COLUMN) * ACROSS,
        y: from.y + (n % DOWN_A_COLUMN) * DOWN,
    }
}

/// One column to the right of the Finding, where what came back is laid out.
fn across(from: StudioPosition) -> StudioPosition {
    StudioPosition {
        x: from.x + ACROSS,
        y: from.y,
    }
}

fn kind_of(source: &Source) -> ScoutSourceKind {
    ScoutSourceKind::from_wire(source.kind()).expect("adapters spells the domain's own set")
}
