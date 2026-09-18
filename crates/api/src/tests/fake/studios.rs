//! Studios, as the fake holds them: whole `ipc::Studio`s in a `Vec`, changed in
//! place. **It proves the routes and the door carry what they are handed**;
//! the record and its refusals are `store`'s and `fleet`'s, tested there.

use std::sync::Arc;

use ipc::{
    AddStudioNode, AskScout, CaptureStudioNote, CheckoutRunUnderway, ContradictionSettled,
    CreateStudio, DecideStudioEdge, DeferOnStudio, DispatchStudioDraft, EditStudioDraft,
    EditStudioLink, GroupStudioNodes, Instant, ManifestId, MoveStudioNode, ProposeStudioEdge,
    RemoveStudioNodes, RenameStudio, SettleContradiction, StartScout, StartStudioRun,
    StartStudioServer, StopScout, Studio, StudioDeleted, StudioEdge, StudioEdgeId, StudioEdgeKind,
    StudioEdgeStanding, StudioId, StudioList, StudioNode, StudioNodeContent, StudioNodeId,
    StudioNodeState, StudioPosition, StudioRunStarted, StudioServerStarted, StudioSummary,
    WireError, WriteUpStudioNode,
};

use super::FakeDaemon;
use crate::tests::shapes::{run_id, THE_MANIFEST};
use crate::{Redirector, Refusal, Studios};

/// The one Studio every fake starts holding, so a route naming a member
/// answers rather than 404ing.
pub const THE_STUDIO: &str = "01STUDIO";

const AT: &str = "2026-09-17T09:00:00.000Z";

/// The run a Studio's own start answers with.
pub const THE_RUN: &str = "01STUDIORUN";

/// The server instance a Studio's own start answers with. `#1345`.
pub const THE_SERVER: &str = "01STUDIOSERVER";

/// What a frame's bytes are here. A PNG's own first eight bytes and nothing
/// after them: the route answers what it was given, and a whole image would
/// only make the fixture longer.
pub const THE_FRAME: &[u8] = &[0x89, b'P', b'N', b'G', 0x0d, 0x0a, 0x1a, 0x0a];

pub fn the_studio() -> Studio {
    Studio {
        id: StudioId::carried(THE_STUDIO),
        manifest_id: ManifestId::carried(THE_MANIFEST),
        name: None,
        named_by: None,
        created_at: Instant::carried(AT),
        touched_at: Instant::carried(AT),
        nodes: Vec::new(),
        edges: Vec::new(),
    }
}

fn standing(spelled: &str) -> StudioEdgeStanding {
    StudioEdgeStanding::from_wire(spelled).expect("a standing the wire has")
}

fn refused(code: &str, message: String) -> WireError {
    WireError::raised(code, message, run_id())
}

impl FakeDaemon {
    /// The Studio `studio_id` names, changed by `change` and answered whole.
    fn changing<T>(
        &self,
        studio_id: &StudioId,
        within: Option<ManifestId>,
        change: impl FnOnce(&mut Studio) -> Result<T, Refusal>,
    ) -> Result<T, Refusal> {
        let mut studios = self.studios.lock().expect("not poisoned");
        let studio = studios
            .iter_mut()
            .find(|studio| &studio.id == studio_id)
            .filter(|studio| within.map_or(true, |named| studio.manifest_id == named))
            .ok_or_else(|| {
                Refusal::NoSuchJob(refused(
                    "fake.no_such_studio",
                    format!("no Studio is `{}`", studio_id.as_str()),
                ))
            })?;
        change(studio)
    }
}

impl Studios for FakeDaemon {
    async fn list_studios(&self, manifest_id: Option<ManifestId>) -> Result<StudioList, Refusal> {
        let manifest_id = manifest_id.unwrap_or_else(|| ManifestId::carried(THE_MANIFEST));
        let studios = self.studios.lock().expect("not poisoned");
        Ok(StudioList {
            studios: studios
                .iter()
                .filter(|studio| studio.manifest_id == manifest_id)
                .map(|studio| StudioSummary {
                    id: studio.id.clone(),
                    manifest_id: studio.manifest_id.clone(),
                    name: studio.name.clone(),
                    created_at: studio.created_at.clone(),
                    touched_at: studio.touched_at.clone(),
                })
                .collect(),
        })
    }

    async fn get_studio(
        &self,
        studio_id: StudioId,
        within: Option<ManifestId>,
    ) -> Result<Studio, Refusal> {
        self.changing(&studio_id, within, |studio| Ok(studio.clone()))
    }

    /// The bytes are [`THE_FRAME`], whatever the node kept: what this proves is
    /// that the route answers the file rather than JSON, and that a node with
    /// no picture is refused apart from a node that is not there.
    async fn get_studio_frame(
        &self,
        studio_id: StudioId,
        node_id: StudioNodeId,
        within: Option<ManifestId>,
    ) -> Result<(String, Vec<u8>), Refusal> {
        self.changing(&studio_id, within, |studio| {
            let node = studio
                .nodes
                .iter()
                .find(|node| node.id == node_id)
                .ok_or_else(|| {
                    Refusal::Unacceptable(refused(
                        "fake.no_such_studio_node",
                        format!("no node of this Studio is `{}`", node_id.as_str()),
                    ))
                })?;
            let kept = match &node.content {
                StudioNodeContent::Note {
                    capture: Some(capture),
                    ..
                } => capture.frame.as_ref(),
                _ => None,
            };
            let frame = kept.ok_or_else(|| {
                Refusal::Unacceptable(refused(
                    "fake.studio_frame_not_kept",
                    format!("node `{}` kept no frame", node_id.as_str()),
                ))
            })?;
            Ok((frame.filename.clone(), THE_FRAME.to_vec()))
        })
    }

    async fn create_studio(
        &self,
        create: CreateStudio,
        manifest_id: Option<ManifestId>,
    ) -> Result<Studio, Refusal> {
        let manifest_id = manifest_id.unwrap_or_else(|| ManifestId::carried(THE_MANIFEST));
        let mut studios = self.studios.lock().expect("not poisoned");
        let studio = Studio {
            id: StudioId::carried(format!("01STUDIO{}", studios.len())),
            manifest_id,
            name: create.name,
            ..the_studio()
        };
        studios.push(studio.clone());
        Ok(studio)
    }

    async fn rename_studio(
        &self,
        studio_id: StudioId,
        rename: RenameStudio,
        by: Redirector,
        within: Option<ManifestId>,
    ) -> Result<Studio, Refusal> {
        self.added_by.lock().expect("not poisoned").push(by);
        self.changing(&studio_id, within, |studio| {
            studio.name = Some(rename.name);
            Ok(studio.clone())
        })
    }

    async fn delete_studio(
        &self,
        studio_id: StudioId,
        within: Option<ManifestId>,
    ) -> Result<StudioDeleted, Refusal> {
        let deleted = self.changing(&studio_id, within, |studio| {
            Ok(StudioDeleted {
                id: studio.id.clone(),
                manifest_id: studio.manifest_id.clone(),
            })
        })?;
        self.studios
            .lock()
            .expect("not poisoned")
            .retain(|studio| studio.id != studio_id);
        Ok(deleted)
    }

    /// Helm is held to a Finding, the one kind that starts proposed.
    async fn add_studio_node(
        &self,
        studio_id: StudioId,
        add: AddStudioNode,
        by: Redirector,
        within: Option<ManifestId>,
    ) -> Result<Studio, Refusal> {
        self.added_by.lock().expect("not poisoned").push(by);
        let helms = matches!(add.content, StudioNodeContent::Finding { .. });
        if by == Redirector::Helm && !helms {
            return Err(Refusal::Unacceptable(refused(
                "fake.studio_node_not_helms",
                "Helm adds only a node that starts proposed".to_string(),
            )));
        }
        self.changing(&studio_id, within, |studio| {
            let id = StudioNodeId::carried(format!("01NODE{}", studio.nodes.len()));
            studio.nodes.push(StudioNode {
                id,
                content: add.content,
                state: None,
                position: add.position,
                created_at: Instant::carried(AT),
                added_by: None,
            });
            Ok(studio.clone())
        })
    }

    /// The frame is not kept here: what a fake proves is that the route
    /// carries the body, and where a frame goes is `fleet`'s.
    async fn capture_studio_note(
        &self,
        studio_id: StudioId,
        capture: CaptureStudioNote,
        within: Option<ManifestId>,
    ) -> Result<Studio, Refusal> {
        self.changing(&studio_id, within, |studio| {
            let id = StudioNodeId::carried(format!("01NODE{}", studio.nodes.len()));
            // What Fleet does with the staged PNG, in the one way a client can
            // see: the Note names the file kept for it, never where it was
            // staged. `get_studio_frame` reads that name back.
            let mut pointed = capture.capture.clone();
            pointed.frame = capture.frame.as_ref().map(|staged| ipc::CaptureFrame {
                filename: format!("{}.png", id.as_str()),
                byte_size: THE_FRAME.len() as u64,
                width: staged.width,
                height: staged.height,
            });
            studio.nodes.push(StudioNode {
                id,
                content: StudioNodeContent::Note {
                    said: capture.said.clone(),
                    capture: Some(pointed),
                },
                state: None,
                position: capture.position,
                created_at: Instant::carried(AT),
                added_by: None,
            });
            Ok(studio.clone())
        })
    }

    async fn move_studio_node(
        &self,
        studio_id: StudioId,
        moving: MoveStudioNode,
        within: Option<ManifestId>,
    ) -> Result<Studio, Refusal> {
        self.changing(&studio_id, within, |studio| {
            for node in studio
                .nodes
                .iter_mut()
                .filter(|node| node.id == moving.node_id)
            {
                node.position = moving.position;
            }
            Ok(studio.clone())
        })
    }

    /// **All of them or none**, as Fleet's is: every name is checked before
    /// anything is dropped, so a name this Studio does not carry leaves it
    /// whole.
    async fn remove_studio_nodes(
        &self,
        studio_id: StudioId,
        removing: RemoveStudioNodes,
        within: Option<ManifestId>,
    ) -> Result<Studio, Refusal> {
        self.changing(&studio_id, within, |studio| {
            if removing.node_ids.is_empty() {
                return Err(Refusal::Unacceptable(refused(
                    "fake.studio_no_nodes_named",
                    "a delete names at least one node".to_string(),
                )));
            }
            for wanted in &removing.node_ids {
                if !studio.nodes.iter().any(|node| &node.id == wanted) {
                    return Err(Refusal::Unacceptable(refused(
                        "fake.no_such_studio_node",
                        format!("no node on this Studio is `{}`", wanted.as_str()),
                    )));
                }
            }
            let going = |id: &ipc::StudioNodeId| removing.node_ids.iter().any(|one| one == id);
            studio.nodes.retain(|node| !going(&node.id));
            studio
                .edges
                .retain(|edge| !going(&edge.from) && !going(&edge.to));
            Ok(studio.clone())
        })
    }

    async fn propose_studio_edge(
        &self,
        studio_id: StudioId,
        proposal: ProposeStudioEdge,
        by: Redirector,
        within: Option<ManifestId>,
    ) -> Result<Studio, Refusal> {
        self.added_by.lock().expect("not poisoned").push(by);
        self.changing(&studio_id, within, |studio| {
            studio.edges.push(StudioEdge {
                id: StudioEdgeId::carried(format!("01EDGE{}", studio.edges.len())),
                from: proposal.from,
                to: proposal.to,
                kind: StudioEdgeKind::from_wire(proposal.kind.as_wire())
                    .expect("every relation is an edge kind"),
                standing: standing("proposed"),
                created_at: Instant::carried(AT),
                added_by: None,
            });
            Ok(studio.clone())
        })
    }

    async fn decide_studio_edge(
        &self,
        studio_id: StudioId,
        decision: DecideStudioEdge,
        within: Option<ManifestId>,
    ) -> Result<Studio, Refusal> {
        self.changing(&studio_id, within, |studio| {
            let Some(at) = studio
                .edges
                .iter()
                .position(|edge| edge.id == decision.edge_id)
            else {
                return Err(Refusal::Unacceptable(refused(
                    "fake.no_such_studio_edge",
                    format!("no edge is `{}`", decision.edge_id.as_str()),
                )));
            };
            match decision.accepted {
                true => studio.edges[at].standing = standing("accepted"),
                false => {
                    studio.edges.remove(at);
                }
            }
            Ok(studio.clone())
        })
    }

    /// A Finding added Gathering. **No scout runs in the fake**: the route
    /// carrying the ask is what is proved here.
    async fn ask_scout(
        self: Arc<Self>,
        studio_id: StudioId,
        ask: AskScout,
        within: Option<ManifestId>,
    ) -> Result<Studio, Refusal> {
        self.changing(&studio_id, within, |studio| {
            studio.nodes.push(StudioNode {
                id: StudioNodeId::carried(format!("01NODE{}", studio.nodes.len())),
                content: StudioNodeContent::finding_asked(&ask.asked),
                state: StudioNodeState::from_wire("gathering"),
                position: ask.position,
                created_at: Instant::carried(AT),
                added_by: None,
            });
            Ok(studio.clone())
        })
    }

    /// A Finding added Gathering off the Link. **Nothing is fetched in the
    /// fake**: the route and who it records the act as is what is proved here.
    async fn read_in_link(
        self: Arc<Self>,
        studio_id: StudioId,
        read_in: ipc::ReadInLink,
        _by: Redirector,
        within: Option<ManifestId>,
    ) -> Result<Studio, Refusal> {
        self.changing(&studio_id, within, |studio| {
            studio.nodes.push(StudioNode {
                id: StudioNodeId::carried(format!("01NODE{}", studio.nodes.len())),
                content: StudioNodeContent::finding_asked(&format!(
                    "Read in {}",
                    read_in.node_id.as_str()
                )),
                state: StudioNodeState::from_wire("gathering"),
                position: read_in.position,
                created_at: Instant::carried(AT),
                added_by: None,
            });
            Ok(studio.clone())
        })
    }

    async fn start_scout(
        self: Arc<Self>,
        studio_id: StudioId,
        start: StartScout,
        within: Option<ManifestId>,
    ) -> Result<Studio, Refusal> {
        self.changing(&studio_id, within, |studio| {
            for node in studio
                .nodes
                .iter_mut()
                .filter(|node| node.id == start.node_id)
            {
                node.state = StudioNodeState::from_wire("gathering");
            }
            Ok(studio.clone())
        })
    }

    /// Nothing is running to stop, so this answers the Studio as it stands.
    async fn stop_scout(
        &self,
        studio_id: StudioId,
        _stop: StopScout,
        within: Option<ManifestId>,
    ) -> Result<Studio, Refusal> {
        self.changing(&studio_id, within, |studio| Ok(studio.clone()))
    }

    /// The run is `THE_RUN`, started and not finished, and the node references
    /// it. **Who acted is recorded, as on every other write here**, so the
    /// door's word for a Helm call is what the route carries.
    async fn start_studio_run(
        self: std::sync::Arc<Self>,
        studio_id: StudioId,
        run: StartStudioRun,
        by: Redirector,
        within: Option<ManifestId>,
    ) -> Result<StudioRunStarted, Refusal> {
        self.added_by.lock().expect("not poisoned").push(by);
        self.changing(&studio_id, within, |studio| {
            let node_id = StudioNodeId::carried(format!("01NODE{}", studio.nodes.len()));
            studio.nodes.push(StudioNode {
                id: node_id.clone(),
                content: StudioNodeContent::Run {
                    run_id: String::from(THE_RUN),
                    held: None,
                    kept: None,
                },
                state: None,
                position: run.position,
                created_at: Instant::carried(AT),
                added_by: None,
            });
            Ok(StudioRunStarted {
                studio: studio.clone(),
                node_id,
                run: CheckoutRunUnderway {
                    id: String::from(THE_RUN),
                    name: run.name.clone(),
                    command: format!("run {}", run.name),
                    started_at: Instant::carried(AT),
                    workspace: run.workspace.clone(),
                },
            })
        })
    }

    /// The instance is `THE_SERVER`, serving, and the node holds it. `#1345`.
    async fn start_studio_server(
        self: std::sync::Arc<Self>,
        studio_id: StudioId,
        server: StartStudioServer,
        by: Redirector,
        within: Option<ManifestId>,
    ) -> Result<StudioServerStarted, Refusal> {
        self.added_by.lock().expect("not poisoned").push(by);
        self.changing(&studio_id, within, |studio| {
            let node_id = StudioNodeId::carried(format!("01NODE{}", studio.nodes.len()));
            studio.nodes.push(StudioNode {
                id: node_id.clone(),
                content: StudioNodeContent::Run {
                    run_id: String::from(THE_SERVER),
                    held: Some(ipc::StudioRunHeld::Server),
                    kept: None,
                },
                state: None,
                position: server.position,
                created_at: Instant::carried(AT),
                added_by: None,
            });
            Ok(StudioServerStarted {
                studio: studio.clone(),
                node_id,
                server: ipc::ServerState {
                    id: String::from(THE_SERVER),
                    name: server.name.clone(),
                    job_id: None,
                    manifest_id: None,
                    phase: ipc::ServerPhase::Serving,
                    serve: format!("serve {}", server.name),
                    ports: Vec::new(),
                    links: Vec::new(),
                    started_by: ipc::StartedBy::Person,
                    started_at: Instant::carried(AT),
                    serving_since: Some(Instant::carried(AT)),
                    ended_at: None,
                    exit_code: None,
                    ended: None,
                    stopped: false,
                    log: String::from(".armada/servers/main/01SERVER/output.log"),
                },
                already_up: false,
            })
        })
    }
    // Promotion — `#1291`. Each one adds what its rung makes and the edges the
    // Studio draws, so the route and the door are proved to carry the body;
    // which kinds each refuses is `fleet`'s, tested there.

    async fn group_studio_nodes(
        &self,
        studio_id: StudioId,
        group: GroupStudioNodes,
        within: Option<ManifestId>,
    ) -> Result<Studio, Refusal> {
        self.changing(&studio_id, within, |studio| {
            let id = added(studio, group.content, group.position);
            for from in group.from {
                produced(studio, from, id.clone());
            }
            Ok(studio.clone())
        })
    }

    async fn defer_on_studio(
        &self,
        studio_id: StudioId,
        deferring: DeferOnStudio,
        within: Option<ManifestId>,
    ) -> Result<Studio, Refusal> {
        self.changing(&studio_id, within, |studio| {
            let what = StudioNodeContent::Deferral {
                what: deferring.what,
            };
            let id = added(studio, what, deferring.position);
            produced(studio, deferring.raised_on, id.clone());
            if let Some(blocks) = deferring.blocks {
                studio.edges.push(StudioEdge {
                    id: StudioEdgeId::carried(format!("01EDGE{}", studio.edges.len())),
                    from: id,
                    to: blocks,
                    kind: StudioEdgeKind::from_wire("blocks").expect("a relation"),
                    standing: standing("accepted"),
                    created_at: Instant::carried(AT),
                    added_by: None,
                });
            }
            Ok(studio.clone())
        })
    }

    async fn write_up_studio_node(
        &self,
        studio_id: StudioId,
        writing: WriteUpStudioNode,
        by: Redirector,
        within: Option<ManifestId>,
    ) -> Result<Studio, Refusal> {
        self.added_by.lock().expect("not poisoned").push(by);
        self.changing(&studio_id, within, |studio| {
            let draft = StudioNodeContent::IssueDraft {
                title: writing.title,
                body: writing.body,
            };
            let id = added(studio, draft, writing.position);
            produced(studio, writing.node_id, id);
            Ok(studio.clone())
        })
    }

    async fn edit_studio_draft(
        &self,
        studio_id: StudioId,
        edit: EditStudioDraft,
        within: Option<ManifestId>,
    ) -> Result<Studio, Refusal> {
        self.changing(&studio_id, within, |studio| {
            for node in studio
                .nodes
                .iter_mut()
                .filter(|node| node.id == edit.node_id)
            {
                node.content = StudioNodeContent::IssueDraft {
                    title: edit.title.clone(),
                    body: edit.body.clone(),
                };
            }
            Ok(studio.clone())
        })
    }

    async fn edit_studio_link(
        &self,
        studio_id: StudioId,
        edit: EditStudioLink,
        within: Option<ManifestId>,
    ) -> Result<Studio, Refusal> {
        self.changing(&studio_id, within, |studio| {
            for node in studio
                .nodes
                .iter_mut()
                .filter(|node| node.id == edit.node_id)
            {
                if let StudioNodeContent::Link { address, named, .. } = &node.content {
                    node.content = StudioNodeContent::Link {
                        address: address.clone(),
                        said: Some(edit.said.clone()),
                        named: named.clone(),
                    };
                }
            }
            Ok(studio.clone())
        })
    }

    async fn settle_contradiction(
        &self,
        studio_id: StudioId,
        settling: SettleContradiction,
        within: Option<ManifestId>,
    ) -> Result<Studio, Refusal> {
        self.changing(&studio_id, within, |studio| {
            let (state, answer) = match &settling.outcome {
                ContradictionSettled::NotAProblem => ("not_a_problem", None),
                ContradictionSettled::ResolvedHere { answer } => {
                    ("resolved_here", Some(answer.clone()))
                }
            };
            for node in studio
                .nodes
                .iter_mut()
                .filter(|node| node.id == settling.node_id)
            {
                node.state = StudioNodeState::from_wire(state);
                if let StudioNodeContent::Contradiction { answer: kept, .. } = &mut node.content {
                    *kept = answer.clone();
                }
            }
            Ok(studio.clone())
        })
    }

    /// **No proposer runs in the fake**, so one Job node stands for the plan.
    async fn dispatch_studio_draft(
        self: Arc<Self>,
        studio_id: StudioId,
        dispatching: DispatchStudioDraft,
        by: Redirector,
        within: Option<ManifestId>,
    ) -> Result<Studio, Refusal> {
        self.added_by.lock().expect("not poisoned").push(by);
        self.changing(&studio_id, within, |studio| {
            let job = StudioNodeContent::Job {
                job_id: ipc::JobId::carried("01JOB"),
            };
            let id = added(studio, job, dispatching.position);
            produced(studio, dispatching.node_id, id);
            Ok(studio.clone())
        })
    }
}

/// A node on the fake's Studio, by the next id, and what it was given.
fn added(
    studio: &mut Studio,
    content: StudioNodeContent,
    position: StudioPosition,
) -> StudioNodeId {
    let id = StudioNodeId::carried(format!("01NODE{}", studio.nodes.len()));
    studio.nodes.push(StudioNode {
        id: id.clone(),
        content,
        state: None,
        position,
        created_at: Instant::carried(AT),
        added_by: None,
    });
    id
}

/// The Studio's own edge saying `from` made `to`.
fn produced(studio: &mut Studio, from: StudioNodeId, to: StudioNodeId) {
    studio.edges.push(StudioEdge {
        id: StudioEdgeId::carried(format!("01EDGE{}", studio.edges.len())),
        from,
        to,
        kind: StudioEdgeKind::from_wire("produced").expect("the Studio's own edge"),
        standing: standing("accepted"),
        created_at: Instant::carried(AT),
        added_by: None,
    });
}
