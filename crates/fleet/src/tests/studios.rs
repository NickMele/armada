//! A Studio through Fleet: whole after a restart, published after every write,
//! a Helm session held to what starts proposed, and a door session to its own
//! repository. `#1285`.

use std::time::Duration;

use api::{Next, Redirector, Refusal, Studios, Subscription};
use ipc::{
    AddStudioNode, CreateStudio, DecideStudioEdge, ManifestId, MoveStudioNode, ProposeStudioEdge,
    Studio, StudioNodeContent, StudioPosition, StudioRelation,
};
use testkit::FakeWorkProduct;

use crate::daemon::Fleet;
use crate::tests::daemon::fittings;
use crate::tests::tmp::TempDir;

type Fixture = Fleet<testkit::FakeHarness, testkit::FakeVcs, FakeWorkProduct>;

fn a_fleet(home: &TempDir) -> Fixture {
    Fleet::assembled(fittings(home, FakeWorkProduct::changed(&[])))
}

fn code(refusal: &Refusal) -> &str {
    match refusal {
        Refusal::NoSuchJob(e)
        | Refusal::IllegalMove(e)
        | Refusal::Unacceptable(e)
        | Refusal::Fault(e) => &e.code,
    }
}

fn a_note(said: &str, x: i64) -> AddStudioNode {
    AddStudioNode {
        content: StudioNodeContent::Note {
            said: said.to_string(),
            capture: None,
        },
        position: StudioPosition { x, y: 0 },
        produced_by: None,
    }
}

async fn a_studio(fleet: &Fixture) -> Studio {
    fleet
        .create_studio(
            CreateStudio {
                name: Some("Stale counts".to_string()),
            },
            None,
        )
        .await
        .expect("created in the repository Fleet starts in")
}

/// `#1285`'s own claim, through Fleet: two Notes, a proposed Same as edge and a
/// moved node read back identically after Fleet is assembled again.
#[tokio::test]
async fn a_studio_reads_back_whole_after_fleet_restarts() {
    let home = TempDir::new();
    let fleet = a_fleet(&home);
    let studio = a_studio(&fleet).await;
    let mut built = studio.clone();
    for (said, x) in [
        ("The chip keeps its count", 0),
        ("Overview says three", 240),
    ] {
        built = fleet
            .add_studio_node(studio.id.clone(), a_note(said, x), Redirector::Person, None)
            .await
            .expect("a person's Note");
    }
    let (first, second) = (built.nodes[0].id.clone(), built.nodes[1].id.clone());
    fleet
        .propose_studio_edge(
            studio.id.clone(),
            ProposeStudioEdge {
                from: first,
                to: second.clone(),
                kind: StudioRelation::from_wire("same_as").expect("a relation"),
            },
            Redirector::Person,
            None,
        )
        .await
        .expect("proposed");
    let left = fleet
        .move_studio_node(
            studio.id.clone(),
            MoveStudioNode {
                node_id: second,
                position: StudioPosition { x: 480, y: -120 },
            },
            None,
        )
        .await
        .expect("moved");
    drop(fleet);

    let fleet = a_fleet(&home);
    let read = fleet
        .get_studio(studio.id.clone(), None)
        .await
        .expect("still there");
    assert_eq!(read, left);
    assert_eq!(read.nodes[1].position, StudioPosition { x: 480, y: -120 });
    assert_eq!(read.edges[0].standing.as_wire(), "proposed");
    let persons = Some("person");
    assert_eq!(read.edges[0].added_by.map(|by| by.as_wire()), persons);
    assert_eq!(
        read.named_by.map(|by| by.as_wire()),
        persons,
        "named at the start"
    );
    let listed = fleet.list_studios(None).await.expect("lists");
    assert_eq!(listed.studios.len(), 1);
    assert_eq!(listed.studios[0].name.as_deref(), Some("Stale counts"));
}

/// **Helm adds what starts proposed.** A Note from Helm is refused by name, and
/// a Finding lands `proposed` whoever asked for it.
#[tokio::test]
async fn helm_adds_only_a_node_that_starts_proposed() {
    let home = TempDir::new();
    let fleet = a_fleet(&home);
    let studio = a_studio(&fleet).await;
    let refused = fleet
        .add_studio_node(
            studio.id.clone(),
            a_note("Helm saw this", 0),
            Redirector::Helm,
            None,
        )
        .await
        .expect_err("a Note is a person's");
    assert_eq!(code(&refused), "fleet.studio_node_not_helms");

    let finding = AddStudioNode {
        content: StudioNodeContent::finding_asked("what reads the count"),
        position: StudioPosition { x: 0, y: 0 },
        produced_by: None,
    };
    let added = fleet
        .add_studio_node(studio.id.clone(), finding, Redirector::Helm, None)
        .await
        .expect("a Finding starts proposed");
    assert_eq!(
        added.nodes[0].state.map(|state| state.as_wire()),
        Some("proposed")
    );
}

/// The Studio's own `produced` edge is not a proposal, so there is nothing to
/// accept; and every write is on the stream, whole.
#[tokio::test]
async fn every_write_is_published_and_a_produced_edge_is_not_decided() {
    let home = TempDir::new();
    let fleet = a_fleet(&home);
    let mut watching: Subscription = fleet.events().subscribe();
    let studio = a_studio(&fleet).await;
    let note = fleet
        .add_studio_node(
            studio.id.clone(),
            a_note("the count", 0),
            Redirector::Person,
            None,
        )
        .await
        .expect("added");
    let asked = AddStudioNode {
        content: StudioNodeContent::finding_asked("what reads it"),
        position: StudioPosition { x: 0, y: 160 },
        produced_by: Some(note.nodes[0].id.clone()),
    };
    let asked = fleet
        .add_studio_node(studio.id.clone(), asked, Redirector::Person, None)
        .await
        .expect("added, made by the Note");
    assert_eq!(asked.edges[0].kind.as_wire(), "produced");
    assert_eq!(asked.edges[0].standing.as_wire(), "accepted");

    let refused = fleet
        .decide_studio_edge(
            studio.id.clone(),
            DecideStudioEdge {
                edge_id: asked.edges[0].id.clone(),
                accepted: true,
            },
            None,
        )
        .await
        .expect_err("drawn by the Studio");
    assert_eq!(code(&refused), "fleet.studio_edge_not_proposed");

    fleet
        .delete_studio(studio.id.clone(), None)
        .await
        .expect("deleted");
    let kinds = drained(&mut watching).await;
    assert_eq!(
        kinds,
        vec![
            "studio.changed",
            "studio.changed",
            "studio.changed",
            "studio.deleted"
        ]
    );
    let gone = fleet
        .get_studio(studio.id, None)
        .await
        .expect_err("deleted");
    assert_eq!(code(&gone), "fleet.no_such_studio");
}

/// A door session standing in another repository is answered as if the Studio
/// were not there, and Bridge, which names no scope, reads it.
#[tokio::test]
async fn a_studio_is_refused_to_a_door_session_in_another_repository() {
    let home = TempDir::new();
    let fleet = a_fleet(&home);
    let studio = a_studio(&fleet).await;
    let elsewhere = fleet
        .get_studio(
            studio.id.clone(),
            Some(ManifestId::carried("01ANOTHERMANIFEST")),
        )
        .await
        .expect_err("another repository's");
    assert_eq!(code(&elsewhere), "fleet.studio_in_another_repository");
    let ours = fleet
        .get_studio(studio.id.clone(), Some(studio.manifest_id.clone()))
        .await
        .expect("its own");
    assert_eq!(ours.id, studio.id);
}

async fn drained(watching: &mut Subscription) -> Vec<String> {
    let mut seen = Vec::new();
    while let Ok(Some(Next::Send(delivered))) =
        tokio::time::timeout(Duration::from_millis(30), watching.next()).await
    {
        seen.push(delivered.event.kind());
    }
    seen
}
