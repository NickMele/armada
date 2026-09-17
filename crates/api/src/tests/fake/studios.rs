//! Studios, as the fake holds them: whole `ipc::Studio`s in a `Vec`, changed in
//! place. **It proves the routes and the door carry what they are handed**;
//! the record and its refusals are `store`'s and `fleet`'s, tested there.

use ipc::{
    AddStudioNode, CreateStudio, DecideStudioEdge, Instant, ManifestId, MoveStudioNode,
    ProposeStudioEdge, RemoveStudioNode, RenameStudio, Studio, StudioDeleted, StudioEdge,
    StudioEdgeId, StudioEdgeKind, StudioEdgeStanding, StudioId, StudioList, StudioNode,
    StudioNodeContent, StudioNodeId, StudioSummary, WireError,
};

use super::FakeDaemon;
use crate::tests::shapes::{run_id, THE_MANIFEST};
use crate::{Redirector, Refusal, Studios};

/// The one Studio every fake starts holding, so a route naming a member
/// answers rather than 404ing.
pub const THE_STUDIO: &str = "01STUDIO";

const AT: &str = "2026-09-17T09:00:00.000Z";

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

    async fn remove_studio_node(
        &self,
        studio_id: StudioId,
        removing: RemoveStudioNode,
        within: Option<ManifestId>,
    ) -> Result<Studio, Refusal> {
        self.changing(&studio_id, within, |studio| {
            studio.nodes.retain(|node| node.id != removing.node_id);
            studio
                .edges
                .retain(|edge| edge.from != removing.node_id && edge.to != removing.node_id);
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
}
