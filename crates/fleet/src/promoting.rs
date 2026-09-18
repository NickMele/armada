//! Promotion: turning what is on a Studio into work. `#1291`,
//! `docs/concepts/studio.md`, *Promotion*.
//!
//! **Nothing promotes itself.** Every rung here is a call somebody made — a
//! person for four of them, and Helm for the two it may take on a person's
//! ask. A Note nobody wrote up stays a Note.
//!
//! **Nothing here reaches outside Armada.** Writing up makes a node; a forge
//! is a person's own act afterwards. Dispatching is the one call that leaves
//! the Studio, and it goes to the Job proposer with the draft's own text —
//! `crate::proposal`'s ordinary path, at the ordinary gate.
//!
//! **The Studio is read, then written, then read back whole**, through
//! [`Fleet::written`]: an act's answer is the Studio a client replaces what it
//! holds with.

use adapter_traits::{AgentHarness, Delivery, Vcs, WorkProduct};
use api::{Redirector, Refusal};
use core_model::{
    ContradictionOutcome, NotRewritable, StudioEdge, StudioEdgeId, StudioGraph, StudioNode,
    StudioNodeContent, StudioNodeId, StudioNodeKind, StudioPosition, StudioRelation, ToItself,
    TopLevelOrigin,
};
use ipc::{
    ContradictionSettled, DeferOnStudio, DispatchStudioDraft, EditStudioDraft, EditStudioLink,
    GroupStudioNodes, HelmStudioAct, ManifestId, SettleContradiction, WriteUpStudioNode,
};

use crate::daemon::Fleet;
use crate::studios::{author, EDGE_TO_ITSELF, NODE_BLANK, NO_SUCH_NODE};

/// A group of a kind that is not a Cluster or an Outline. A 422.
const NOT_A_GROUP: &str = "fleet.studio_group_not_a_group";
/// A Cluster of something that is not a Note. A 422.
const CLUSTER_IS_OF_NOTES: &str = "fleet.studio_cluster_is_of_notes";
/// A group of nothing, or of one node named twice. A 422.
const GROUP_IS_OF_SEVERAL: &str = "fleet.studio_group_is_of_several";
/// A write-up of a kind no rung writes up. A 422.
const NOT_WRITABLE_UP: &str = "fleet.studio_not_writable_up";
/// An edit or a dispatch of a node that is not an Issue draft. A 422.
const NOT_A_DRAFT: &str = "fleet.studio_not_a_draft";
/// A line written on, or a read-in asked of, a node that is not a Link. A 422.
pub(crate) const NOT_A_LINK: &str = "fleet.studio_not_a_link";
/// An outcome asked of a node that is not a Contradiction. A 422.
const NOT_A_CONTRADICTION: &str = "fleet.studio_not_a_contradiction";

/// The request a node dispatches as, and `None` on a node that dispatches
/// nothing — `#1379`, `#1394`.
///
/// **An Issue draft sends its own words and the three forge kinds send their
/// address**, whole either way: the first is text nobody has filed, the second
/// names something already on the forge, and the [Job
/// proposer](../../../docs/concepts/job-proposer.md) has taken such a link as a
/// request since it shipped. A Link is an address nothing recognised — a board,
/// a page, a session — so there is nothing filed to dispatch against.
///
/// **Read off the kind; no address is read here.** Which host is the forge was
/// `crates/adapters`' answer when the node was made, and asking again would be
/// a second answer the day the first changed. Which workflow each of the three
/// runs under is the proposer's, off each definition's `for_requests` line.
fn dispatched_as(content: &StudioNodeContent) -> Option<String> {
    if let Some(request) = content.dispatched_as() {
        return Some(request);
    }
    matches!(
        content.kind(),
        StudioNodeKind::Issue | StudioNodeKind::PullRequest | StudioNodeKind::Epic
    )
    .then(|| content.address().map(String::from))
    .flatten()
}
/// A Contradiction ended twice. A 409.
const CONTRADICTION_SETTLED: &str = "fleet.studio_contradiction_settled";
/// The kinds a rung writes up: what a person said, what they grouped, and
/// what disagreed. `docs/concepts/studio.md`, *Promotion*, plus the Outline,
/// which that page says can itself be written up.
const WRITABLE_UP: &[StudioNodeKind] = &[
    StudioNodeKind::Note,
    StudioNodeKind::Cluster,
    StudioNodeKind::Contradiction,
    StudioNodeKind::Outline,
];

/// How far apart two Jobs from one proposal are placed, in canvas units — one
/// node's height and a gap, so a split reads as a column.
const JOB_APART: i64 = 200;

/// The origin a Job dispatched from a Studio carries: **where from, and who
/// pressed it**. `crates/core-model/domain/enum-verbs.toml` renders these as
/// *From a Studio, by you* and *From a Studio, via Helm* —
/// one value carrying both clauses, because one column answers one question
/// and the row has to keep answering both. `#1362`.
///
/// *Which* Studio is not here and is not a column: the `produced` edge from
/// the Issue draft records it, and `store::studio::tracing` reads it back.
fn pressed(by: Redirector) -> TopLevelOrigin {
    match by {
        Redirector::Person => TopLevelOrigin::StudioDispatched,
        Redirector::Helm => TopLevelOrigin::StudioHelmDrafted,
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
    /// A rewrite the node would not take, as the wire says it.
    fn not_rewritable(&self, code: &'static str, fault: NotRewritable) -> Refusal {
        let said = format!("a {} node does not take that", fault.kind.as_wire());
        match fault.state {
            // A Contradiction of the right kind that would not take it has
            // already ended, and ending it again would lose the outcome that
            // was acted on.
            Some(state) if fault.kind == StudioNodeKind::Contradiction => {
                Refusal::IllegalMove(ipc::WireError::raised(
                    CONTRADICTION_SETTLED,
                    format!(
                        "this Contradiction already ended as `{}`, and a Contradiction ends once",
                        state.as_wire()
                    ),
                    self.run_id(),
                ))
            }
            _ => self.studio_unacceptable(code, said),
        }
    }

    /// The node `node_id` names, on this Studio.
    fn node_on<'a>(
        &self,
        graph: &'a StudioGraph,
        node_id: &StudioNodeId,
    ) -> Result<&'a StudioNode, Refusal> {
        graph
            .nodes
            .iter()
            .find(|node| node.id() == node_id)
            .ok_or_else(|| {
                self.studio_unacceptable(
                    NO_SUCH_NODE,
                    format!("no node on this Studio is `{}`", node_id.as_str()),
                )
            })
    }

    /// The Contradiction ended, where this rung was taken on one still
    /// Reported. **Silent on every other kind**: writing a Note up does not
    /// end anything, and a Contradiction already ended is refused rather than
    /// ended twice.
    fn ending_a_contradiction(
        &self,
        node: &StudioNode,
        outcome: ContradictionOutcome,
    ) -> Result<Option<core_model::Rewritten>, Refusal> {
        if node.kind() != StudioNodeKind::Contradiction {
            return Ok(None);
        }
        node.settled(&outcome)
            .map(Some)
            .map_err(|fault| self.not_rewritable(NOT_A_CONTRADICTION, fault))
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
    /// `group_studio_nodes`: a Cluster or an Outline, made of the nodes named,
    /// with a `produced` edge from each in the order given.
    pub(crate) async fn nodes_grouped(
        &self,
        studio_id: ipc::StudioId,
        group: GroupStudioNodes,
        within: Option<ManifestId>,
    ) -> Result<ipc::Studio, Refusal> {
        let content = group.content.to_domain();
        self.said_whole(&content)?;
        let kind = content.kind();
        if !matches!(kind, StudioNodeKind::Cluster | StudioNodeKind::Outline) {
            return Err(self.studio_unacceptable(
                NOT_A_GROUP,
                format!(
                    "a {} is added by the rung that makes it, and grouping makes a Cluster or an \
                     Outline",
                    kind.as_wire()
                ),
            ));
        }
        let from: Vec<StudioNodeId> = group
            .from
            .iter()
            .map(ipc::StudioNodeId::to_domain)
            .collect();
        if from.len() < 2 || (1..from.len()).any(|i| from[i..].contains(&from[i - 1])) {
            return Err(self.studio_unacceptable(
                GROUP_IS_OF_SEVERAL,
                "a group is made of two or more different nodes on this Studio".into(),
            ));
        }
        let at = self.now();
        let node = StudioNode::added(
            StudioNodeId::carried(self.mint().ulid()),
            content,
            group.position.to_domain(),
            at.clone(),
            core_model::StudioAuthor::Person,
        );
        let edges: Vec<StudioEdgeId> = from
            .iter()
            .map(|_| StudioEdgeId::carried(self.mint().ulid()))
            .collect();
        let graph = {
            let store = self.store().lock().await;
            self.studio_held(&store, &studio_id.to_domain(), within.as_ref())?
        };
        // A Cluster is Notes a person accepted as one thing. An Outline is a
        // reading of whatever feeds it, so it takes any kind.
        for id in &from {
            let member = self.node_on(&graph, id)?;
            if kind == StudioNodeKind::Cluster && member.kind() != StudioNodeKind::Note {
                return Err(self.studio_unacceptable(
                    CLUSTER_IS_OF_NOTES,
                    format!(
                        "a Cluster is the Notes a person accepted as one thing, and `{}` is a {}",
                        id.as_str(),
                        member.kind().as_wire()
                    ),
                ));
            }
        }
        self.written(&studio_id, within, |store, id| {
            let made_it: Vec<_> = from.iter().zip(edges).collect();
            store.add_studio_node_produced_by(id, &node, &made_it, &at)
        })
        .await
    }

    /// `defer_on_studio`: a Deferral, against what it blocks.
    pub(crate) async fn deferred(
        &self,
        studio_id: ipc::StudioId,
        deferring: DeferOnStudio,
        within: Option<ManifestId>,
    ) -> Result<ipc::Studio, Refusal> {
        let content = StudioNodeContent::Deferral {
            what: deferring.what,
        };
        self.said_whole(&content)?;
        let raised_on = deferring.raised_on.to_domain();
        let blocks = deferring.blocks.map(|id| id.to_domain());
        if blocks.as_ref() == Some(&raised_on) {
            return Err(self.studio_unacceptable(
                EDGE_TO_ITSELF,
                "a Deferral raised on a node does not also block it".into(),
            ));
        }
        let graph = {
            let store = self.store().lock().await;
            self.studio_held(&store, &studio_id.to_domain(), within.as_ref())?
        };
        let raised = self.node_on(&graph, &raised_on)?;
        let ended = self.ending_a_contradiction(raised, ContradictionOutcome::Deferral)?;
        let at = self.now();
        let node = StudioNode::added(
            StudioNodeId::carried(self.mint().ulid()),
            content,
            deferring.position.to_domain(),
            at.clone(),
            core_model::StudioAuthor::Person,
        );
        let produced = StudioEdgeId::carried(self.mint().ulid());
        // Accepted, not proposed: the person drawing it is the one who accepts
        // a relation, so there is nobody left to accept it afterwards. The
        // Deferral is new, so neither end can be the other.
        let holds_up = blocks
            .map(|to| {
                StudioEdge::accepted(
                    StudioEdgeId::carried(self.mint().ulid()),
                    node.id().clone(),
                    to,
                    StudioRelation::Blocks,
                    at.clone(),
                    core_model::StudioAuthor::Person,
                )
            })
            .transpose()
            .map_err(|ToItself { node }| {
                self.studio_unacceptable(
                    EDGE_TO_ITSELF,
                    format!(
                        "an edge joins two nodes, and both ends are `{}`",
                        node.as_str()
                    ),
                )
            })?;
        self.written(&studio_id, within, |store, id| {
            store.add_studio_node_produced_by(id, &node, &[(&raised_on, produced)], &at)?;
            if let Some(edge) = &holds_up {
                store.add_studio_edge(id, edge, &at)?;
            }
            match ended {
                Some(ended) => store.keep_rewritten(id, &ended, &at),
                None => Ok(()),
            }
        })
        .await
    }

    /// `write_up_studio_node`: an Issue draft, made from one node.
    pub(crate) async fn written_up(
        &self,
        studio_id: ipc::StudioId,
        writing: WriteUpStudioNode,
        by: Redirector,
        within: Option<ManifestId>,
    ) -> Result<ipc::Studio, Refusal> {
        let content = StudioNodeContent::IssueDraft {
            title: writing.title,
            body: writing.body,
        };
        self.said_whole(&content)?;
        let from = writing.node_id.to_domain();
        let graph = {
            let store = self.store().lock().await;
            self.studio_held(&store, &studio_id.to_domain(), within.as_ref())?
        };
        let source = self.node_on(&graph, &from)?;
        if !WRITABLE_UP.contains(&source.kind()) {
            return Err(self.studio_unacceptable(
                NOT_WRITABLE_UP,
                format!(
                    "a {} is not written up: a Note, a Cluster, a Contradiction or an Outline is",
                    source.kind().as_wire()
                ),
            ));
        }
        let ended = self.ending_a_contradiction(source, ContradictionOutcome::IssueDraft)?;
        let at = self.now();
        let node = StudioNode::added(
            StudioNodeId::carried(self.mint().ulid()),
            content,
            writing.position.to_domain(),
            at.clone(),
            author(by),
        );
        let produced = StudioEdgeId::carried(self.mint().ulid());
        let node_id = ipc::StudioNodeId::from(node.id());
        let studio = self
            .written(&studio_id, within, |store, id| {
                store.add_studio_node_produced_by(id, &node, &[(&from, produced)], &at)?;
                match ended {
                    Some(ended) => store.keep_rewritten(id, &ended, &at),
                    None => Ok(()),
                }
            })
            .await?;
        self.published_as_helms(
            by,
            &studio,
            HelmStudioAct::WroteUp {
                from: writing.node_id,
                node_id,
            },
        );
        Ok(studio)
    }

    /// `edit_studio_draft`: the title and body a person left on a draft.
    pub(crate) async fn draft_edited(
        &self,
        studio_id: ipc::StudioId,
        edit: EditStudioDraft,
        within: Option<ManifestId>,
    ) -> Result<ipc::Studio, Refusal> {
        let node_id = edit.node_id.to_domain();
        let graph = {
            let store = self.store().lock().await;
            self.studio_held(&store, &studio_id.to_domain(), within.as_ref())?
        };
        let draft = self.node_on(&graph, &node_id)?;
        let edited = draft
            .edited(edit.title, edit.body)
            .map_err(|fault| self.not_rewritable(NOT_A_DRAFT, fault))?;
        self.said_whole(edited.node().content())?;
        let at = self.now();
        self.written(&studio_id, within, |store, id| {
            store.keep_rewritten(id, &edited, &at)
        })
        .await
    }

    /// `edit_studio_link`: the line a person keeps beside a Link's address —
    /// `#1378`.
    ///
    /// **The address is read off the node and never off the request**, so an
    /// edit cannot move a Link somewhere else. A blank line clears it.
    pub(crate) async fn link_relabelled(
        &self,
        studio_id: ipc::StudioId,
        edit: EditStudioLink,
        within: Option<ManifestId>,
    ) -> Result<ipc::Studio, Refusal> {
        let node_id = edit.node_id.to_domain();
        let graph = {
            let store = self.store().lock().await;
            self.studio_held(&store, &studio_id.to_domain(), within.as_ref())?
        };
        let link = self.node_on(&graph, &node_id)?;
        let relabelled = link
            .relabelled(Some(edit.said))
            .map_err(|fault| self.not_rewritable(NOT_A_LINK, fault))?;
        let at = self.now();
        self.written(&studio_id, within, |store, id| {
            store.keep_rewritten(id, &relabelled, &at)
        })
        .await
    }

    /// `settle_contradiction`: the two outcomes that write nothing else down.
    pub(crate) async fn contradiction_settled(
        &self,
        studio_id: ipc::StudioId,
        settling: SettleContradiction,
        within: Option<ManifestId>,
    ) -> Result<ipc::Studio, Refusal> {
        let outcome = match settling.outcome {
            ContradictionSettled::NotAProblem => ContradictionOutcome::NotAProblem,
            ContradictionSettled::ResolvedHere { answer } => {
                if answer.trim().is_empty() {
                    return Err(self.studio_unacceptable(
                        NODE_BLANK,
                        "a Contradiction resolved here records the answer, and `answer` is blank"
                            .into(),
                    ));
                }
                ContradictionOutcome::ResolvedHere { answer }
            }
        };
        let node_id = settling.node_id.to_domain();
        let graph = {
            let store = self.store().lock().await;
            self.studio_held(&store, &studio_id.to_domain(), within.as_ref())?
        };
        let node = self.node_on(&graph, &node_id)?;
        let settled = node
            .settled(&outcome)
            .map_err(|fault| self.not_rewritable(NOT_A_CONTRADICTION, fault))?;
        let at = self.now();
        self.written(&studio_id, within, |store, id| {
            store.keep_rewritten(id, &settled, &at)
        })
        .await
    }

    /// `dispatch_studio_draft`: what the node dispatches as — an Issue draft's
    /// text, or the address of a Link naming an issue — through the Job
    /// proposer to the dispatch gate, and a Job node for each Job it became.
    ///
    /// **Two kinds and one path.** A draft is Armada's own unfiled words and a
    /// Link naming an issue is one already on the forge, which the proposer
    /// already takes as a request; nothing is filed either way and the gate is
    /// the same. `#1379`.
    ///
    /// **The proposal runs outside the store's lock.** It is a model call, and
    /// holding the Studios' lock across one would stop every other Studio
    /// write for as long as a vendor takes to answer.
    pub(crate) async fn draft_dispatched(
        &self,
        studio_id: ipc::StudioId,
        dispatching: DispatchStudioDraft,
        by: Redirector,
        within: Option<ManifestId>,
    ) -> Result<ipc::Studio, Refusal> {
        let node_id = dispatching.node_id.to_domain();
        let graph = {
            let store = self.store().lock().await;
            self.studio_held(&store, &studio_id.to_domain(), within.as_ref())?
        };
        let draft = self.node_on(&graph, &node_id)?;
        let Some(request) = dispatched_as(draft.content()) else {
            return Err(self.studio_unacceptable(
                NOT_A_DRAFT,
                match draft.content().address() {
                    // A Link is dispatchable or it is not, and which it is is
                    // the address's doing rather than the kind's — so the
                    // refusal names the address rather than saying *a link is
                    // not dispatched*, which is untrue of the next one.
                    Some(address) => format!(
                        "`{address}` names no issue on this repository's forge, so there is \
                         nothing filed to dispatch against. Write the work up as an Issue \
                         draft and dispatch that"
                    ),
                    None => format!(
                        "a {} is not dispatched: an Issue draft is, and so is a Link naming \
                         an issue on this repository's forge",
                        draft.kind().as_wire()
                    ),
                },
            ));
        };
        let served = self.served_named(Some(&ManifestId::from(&graph.studio.manifest_id)))?;
        // The draft's own text, verbatim, with nothing to point at: filing the
        // issue anywhere is optional and a person's own act. A Link sends its
        // address and nothing else, because the issue is already filed and
        // Fleet reads a ticket link in a request the way it always has.
        //
        // **The origin is who pressed it, not the proposer's own.** Every other
        // request through this path is one Fleet read and `auto_detected` says
        // so — *Found by Fleet*, the label for work Armada noticed by itself.
        // A dispatch from a Studio is somebody sending a draft they wrote up,
        // so the row says *Dispatched by you* or *Drafted in Helm*.
        let made = self
            .propose_from_with_attachments(
                &request,
                None,
                Vec::new(),
                &served,
                by,
                Some(pressed(by)),
            )
            .await
            .map_err(|why| self.refusal(why))?;
        let at = self.now();
        let placed = dispatching.position.to_domain();
        let jobs: Vec<(StudioNode, StudioEdgeId)> = made
            .iter()
            .enumerate()
            .map(|(row, job)| {
                let node = StudioNode::added(
                    StudioNodeId::carried(self.mint().ulid()),
                    StudioNodeContent::Job {
                        job_id: job.id().clone(),
                    },
                    StudioPosition {
                        x: placed.x,
                        y: placed.y + JOB_APART * row as i64,
                    },
                    at.clone(),
                    author(by),
                );
                (node, StudioEdgeId::carried(self.mint().ulid()))
            })
            .collect();
        let node_ids = jobs
            .iter()
            .map(|(node, _)| ipc::StudioNodeId::from(node.id()))
            .collect();
        let studio = self
            .written(&studio_id, within, |store, id| {
                for (node, edge) in &jobs {
                    store.add_studio_node_produced_by(
                        id,
                        node,
                        &[(&node_id, edge.clone())],
                        &at,
                    )?;
                }
                Ok(())
            })
            .await?;
        self.published_as_helms(
            by,
            &studio,
            HelmStudioAct::Dispatched {
                from: dispatching.node_id,
                node_ids,
            },
        );
        Ok(studio)
    }

    /// Refuse a field left blank, naming which — `add_studio_node`'s rule, on
    /// every rung that makes a node.
    fn said_whole(&self, content: &StudioNodeContent) -> Result<(), Refusal> {
        match content.blank() {
            None => Ok(()),
            Some(field) => Err(self.studio_unacceptable(
                NODE_BLANK,
                format!(
                    "a {} node's `{field}` cannot be blank",
                    content.kind().as_wire()
                ),
            )),
        }
    }
}
