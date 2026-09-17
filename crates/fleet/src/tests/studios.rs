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

/// **A person adds a Note, a Link and a Sketch, and nothing else** — `#1364`.
/// Every other kind is made by the act that earns it, and the door says so by
/// name rather than letting a person mint a Cluster nothing was grouped into.
#[tokio::test]
async fn a_person_adds_only_what_a_person_makes() {
    let home = TempDir::new();
    let fleet = a_fleet(&home);
    let studio = a_studio(&fleet).await;
    let by_hand = [
        StudioNodeContent::Note {
            said: "the legend is unreadable".to_string(),
            capture: None,
        },
        StudioNodeContent::Link {
            address: "docs/contracts/design-system.md".to_string(),
            said: None,
        },
        StudioNodeContent::Sketch {
            body: "legend on its own row".to_string(),
        },
    ];
    for (n, content) in by_hand.into_iter().enumerate() {
        let added = fleet
            .add_studio_node(
                studio.id.clone(),
                AddStudioNode {
                    content,
                    position: StudioPosition {
                        x: n as i64 * 340,
                        y: 0,
                    },
                    produced_by: None,
                },
                Redirector::Person,
                None,
            )
            .await
            .expect("a person's own kind");
        assert_eq!(
            added.nodes[n].added_by.map(|by| by.as_wire()),
            Some("person")
        );
    }

    // A Note typed by hand is fixed the way a captured one is: no act on this
    // seam writes a node's content after it is added.
    let promoted = [
        StudioNodeContent::Cluster {
            title: "the legend cannot be read".to_string(),
        },
        StudioNodeContent::Deferral {
            what: "whether it collapses under 720".to_string(),
        },
        StudioNodeContent::IssueDraft {
            title: "The legend is illegible".to_string(),
            body: "…".to_string(),
        },
        StudioNodeContent::Outline {
            body: "give it its own row".to_string(),
        },
        StudioNodeContent::Contradiction {
            first: "the contract gives it a row".to_string(),
            second: "the Board draws it inside the bar".to_string(),
            answer: None,
        },
        StudioNodeContent::finding_asked("where do the colours come from"),
    ];
    for content in promoted {
        let refused = fleet
            .add_studio_node(
                studio.id.clone(),
                AddStudioNode {
                    content,
                    position: StudioPosition { x: 0, y: 400 },
                    produced_by: None,
                },
                Redirector::Person,
                None,
            )
            .await
            .expect_err("made by the act that earns it");
        assert_eq!(code(&refused), "fleet.studio_node_not_a_persons");
    }

    let read = fleet.get_studio(studio.id, None).await.expect("read back");
    assert_eq!(read.nodes.len(), 3, "nothing refused was written");
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
        content: StudioNodeContent::Link {
            address: "https://example.invalid/counts".to_string(),
            said: None,
        },
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

/// The capture a test sends, with `staged` as its frame's path.
fn a_capture(said: &str, staged: Option<&std::path::Path>) -> ipc::CaptureStudioNote {
    ipc::CaptureStudioNote {
        said: said.to_string(),
        capture: ipc::StudioCapture {
            component: Some("FilterChip".to_string()),
            owners: vec!["Board".to_string()],
            selector: "button.armada-chip".to_string(),
            element: ipc::CaptureElement {
                tag: "button".to_string(),
                text: "Queued 3".to_string(),
                label: None,
            },
            screen: Some("Job Board".to_string()),
            layer: None,
            location: "/".to_string(),
            bounds: ipc::CaptureBounds {
                x: 312,
                y: 148,
                width: 96,
                height: 28,
            },
            window: ipc::CaptureWindow {
                width: 1440,
                height: 900,
            },
            styles: [("color".to_string(), "rgb(232, 232, 237)".to_string())]
                .into_iter()
                .collect(),
            markup: "<button class=\"armada-chip\">Queued 3</button>".to_string(),
            source: None,
            frame: None,
        },
        position: StudioPosition { x: 0, y: 0 },
        frame: staged.map(|path| ipc::StagedFrame {
            staged_path: path.to_string_lossy().to_string(),
            width: 2880,
            height: 1800,
        }),
        produced_by: None,
    }
}

/// **A capture lands as a Note whose frame is a file beside the Studio's
/// records.** `#1290`, `docs/concepts/studio.md`, *Notes*.
///
/// The failure this is against is a frame that only ever existed in Bridge's
/// temporary directory: the Note would name an image nothing can open the day
/// the machine is swept. So the staged PNG is copied into Fleet's own keeping
/// and the Note names what Fleet kept, never where Bridge staged it.
#[tokio::test]
async fn a_capture_keeps_its_frame_beside_the_studios_records_and_names_what_it_kept() {
    let home = TempDir::new();
    let fleet = a_fleet(&home);
    let studio = a_studio(&fleet).await;
    let staged = home.path().join("staged.png");
    std::fs::write(&staged, [0u8; 512]).expect("a frame to stage");

    let captured = fleet
        .capture_studio_note(
            studio.id.clone(),
            a_capture("The chip keeps its count", Some(&staged)),
            None,
        )
        .await
        .expect("a person's capture");

    let node = captured.nodes.first().expect("the Note");
    assert_eq!(node.added_by.map(|by| by.as_wire()), Some("person"));
    let StudioNodeContent::Note { said, capture } = &node.content else {
        panic!("a Note: {:?}", node.content);
    };
    assert_eq!(said, "The chip keeps its count");
    let capture = capture.as_ref().expect("where they pointed");
    assert_eq!(capture.selector, "button.armada-chip");
    let frame = capture.frame.as_ref().expect("the frame Fleet kept");
    assert_eq!(frame.filename, format!("{}.png", node.id.as_str()));
    assert_eq!(frame.byte_size, 512, "what the staged file weighed");

    let kept = std::path::Path::new(&fleet.host().studio_frames_dir)
        .join(studio.id.as_str())
        .join(&frame.filename);
    assert!(kept.exists(), "the frame is a file at {}", kept.display());

    // Deleting a Studio takes its frames with it: nothing else ever read them.
    fleet
        .delete_studio(studio.id.clone(), None)
        .await
        .expect("a person's delete");
    assert!(!kept.exists(), "the frame goes with the Studio");
}

/// A frame Fleet cannot read is refused rather than dropped, and so is one over
/// the cap: a person who saw a frame taken and gets a Note with none would have
/// no way to tell.
#[tokio::test]
async fn a_frame_that_cannot_be_kept_refuses_the_capture_rather_than_dropping_it() {
    let home = TempDir::new();
    let fleet = a_fleet(&home);
    let studio = a_studio(&fleet).await;

    let missing = home.path().join("never-written.png");
    let refused = fleet
        .capture_studio_note(studio.id.clone(), a_capture("said", Some(&missing)), None)
        .await
        .expect_err("a staged frame that is not there");
    assert_eq!(code(&refused), "fleet.studio_frame_unreadable");

    let heavy = home.path().join("heavy.png");
    std::fs::write(&heavy, vec![0u8; 4 * 1024 * 1024 + 1]).expect("a frame over the cap");
    let refused = fleet
        .capture_studio_note(studio.id.clone(), a_capture("said", Some(&heavy)), None)
        .await
        .expect_err("a frame over the cap");
    assert_eq!(code(&refused), "fleet.studio_frame_too_large");

    let blank = fleet
        .capture_studio_note(studio.id.clone(), a_capture("   ", None), None)
        .await
        .expect_err("a Note saying nothing");
    assert_eq!(code(&blank), "fleet.studio_node_blank");

    let held = fleet
        .get_studio(studio.id.clone(), None)
        .await
        .expect("the Studio");
    assert!(held.nodes.is_empty(), "nothing refused was written");
}

/// **What was kept is read back, and the node's own id is the only key.**
/// `#1352`: 14.11 wrote a frame and left every client without a way to see it.
///
/// The three refusals are apart on purpose. A Note captured where no frame
/// could be taken is an ordinary Note and says so; a Studio whose directory
/// went is a file that will not open; a node that is not on the Studio is
/// neither. **Nothing a caller spells reaches a path**: the file name is read
/// off the node's own record, so an id spelling a path traverses nothing.
#[tokio::test]
async fn a_notes_frame_is_read_back_by_its_node_and_a_note_without_one_is_not_a_fault() {
    let home = TempDir::new();
    let fleet = a_fleet(&home);
    let studio = a_studio(&fleet).await;
    let staged = home.path().join("staged.png");
    std::fs::write(&staged, [7u8; 512]).expect("a frame to stage");

    let captured = fleet
        .capture_studio_note(
            studio.id.clone(),
            a_capture("The chip keeps its count", Some(&staged)),
            None,
        )
        .await
        .expect("a person's capture");
    let with = captured.nodes.first().expect("the Note").id.clone();
    let captured = fleet
        .capture_studio_note(
            studio.id.clone(),
            a_capture("No picture was taken", None),
            None,
        )
        .await
        .expect("a person's capture");
    let without = captured.nodes.last().expect("the second Note").id.clone();

    let (name, bytes) = fleet
        .get_studio_frame(studio.id.clone(), with.clone(), None)
        .await
        .expect("the frame that was kept");
    assert_eq!(name, format!("{}.png", with.as_str()), "the kept name");
    assert_eq!(bytes, [7u8; 512], "the file itself");

    let refused = fleet
        .get_studio_frame(studio.id.clone(), without, None)
        .await
        .expect_err("a Note that kept no frame");
    assert_eq!(code(&refused), "fleet.studio_frame_not_kept");

    let refused = fleet
        .get_studio_frame(
            studio.id.clone(),
            ipc::StudioNodeId::carried("01NOSUCHNODE"),
            None,
        )
        .await
        .expect_err("a node that is not on this Studio");
    assert_eq!(code(&refused), "fleet.no_such_studio_node");

    // The Studio's directory, swept off the disk under a record that still
    // names the file: what a client is told is that this one cannot be read.
    std::fs::remove_dir_all(
        std::path::Path::new(&fleet.host().studio_frames_dir).join(studio.id.as_str()),
    )
    .expect("the Studio's own directory");
    let refused = fleet
        .get_studio_frame(studio.id.clone(), with, None)
        .await
        .expect_err("a frame the record names and the disk does not hold");
    assert_eq!(code(&refused), "fleet.studio_frame_unreadable");
}

/// **A Link keeps a line of a person's own beside its address, and the line is
/// theirs to change** — `#1378`. The address is read off the node and never
/// off the request, a blank line clears it, and a kind that is not a Link is
/// refused by name.
#[tokio::test]
async fn a_links_line_is_written_edited_and_cleared_and_its_address_never_moves() {
    const ADDRESS: &str = "https://example.invalid/armada/issues/1378";
    let home = TempDir::new();
    let fleet = a_fleet(&home);
    let studio = a_studio(&fleet).await;
    let pasted = fleet
        .add_studio_node(
            studio.id.clone(),
            AddStudioNode {
                content: StudioNodeContent::Link {
                    address: ADDRESS.to_string(),
                    said: Some("  why the card says nothing  ".to_string()),
                },
                position: StudioPosition { x: 0, y: 0 },
                produced_by: None,
            },
            Redirector::Person,
            None,
        )
        .await
        .expect("a Link a person pasted");
    let link = pasted.nodes[0].id.clone();
    assert_eq!(
        said_on(&pasted, &link),
        (ADDRESS, Some("why the card says nothing")),
        "the line is kept trimmed, beside the address"
    );

    let edited = fleet
        .edit_studio_link(
            studio.id.clone(),
            ipc::EditStudioLink {
                node_id: link.clone(),
                said: "the owner's own report of this defect".to_string(),
            },
            None,
        )
        .await
        .expect("a line a person changed");
    assert_eq!(
        said_on(&edited, &link),
        (ADDRESS, Some("the owner's own report of this defect")),
        "the line changes and the address does not"
    );

    let cleared = fleet
        .edit_studio_link(
            studio.id.clone(),
            ipc::EditStudioLink {
                node_id: link.clone(),
                said: "   ".to_string(),
            },
            None,
        )
        .await
        .expect("a line taken back");
    assert_eq!(
        said_on(&cleared, &link),
        (ADDRESS, None),
        "a blank line leaves the Link as its address alone"
    );

    let note = fleet
        .add_studio_node(
            studio.id.clone(),
            a_note("the legend is unreadable", 340),
            Redirector::Person,
            None,
        )
        .await
        .expect("a Note");
    let refused = fleet
        .edit_studio_link(
            studio.id,
            ipc::EditStudioLink {
                node_id: note.nodes[1].id.clone(),
                said: "not a Link".to_string(),
            },
            None,
        )
        .await
        .expect_err("a Note is fixed at capture");
    assert_eq!(code(&refused), "fleet.studio_not_a_link");
}

/// A Link's address and its line, for an assertion that reads both at once.
fn said_on<'a>(studio: &'a Studio, node_id: &ipc::StudioNodeId) -> (&'a str, Option<&'a str>) {
    let node = studio
        .nodes
        .iter()
        .find(|node| &node.id == node_id)
        .expect("the node");
    let StudioNodeContent::Link { address, said } = &node.content else {
        panic!("a Link");
    };
    (address, said.as_deref())
}
