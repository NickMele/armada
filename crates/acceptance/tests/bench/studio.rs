//! Studio's apparatus: a Studio with two Notes on it, the text an Issue draft
//! would hold, the catalogue a proposer reads it against, and the round trip a
//! request and a record make. It asserts nothing.
//!
//! **The record is `core_model`'s and its wire form is `ipc`'s.** What this
//! file adds is the assembly — which Notes, where a person left them, and the
//! Same as edge proposed between them — built through the constructors `store`
//! reads a row back with, so nothing here is a second vocabulary for a node.
//!
//! **The draft is read off an Issue draft node, never written out beside
//! one.** `#1291` builds the Studio a person promoted — the Cluster, the draft
//! and the Job — and the text a dispatch sends is that node's own, through
//! `StudioNodeContent::dispatched_as`. A constant here would be a second
//! statement of what Fleet composes.

use std::collections::BTreeMap;

use config::ResolvedWorkflow;
use core_model::{
    ManifestId, ScoutCheckout, ScoutEnded, ScoutLook, ScoutOutcome, ScoutSource, ScoutSourceKind,
    Studio, StudioAuthor, StudioEdge, StudioEdgeId, StudioFinding, StudioGraph, StudioId,
    StudioName, StudioNode, StudioNodeContent, StudioNodeId, StudioPosition, StudioRelation,
    Timestamp, Ulid, WorkflowId,
};
use ipc::JobRequest;

use super::bug_workflow_with_the_fix_judged;

/// What a person pointed at on Bridge and said, the first time.
pub const FIRST_NOTE: &str = "The Board's filter chip keeps its count after the filter is cleared";

/// The second Note, captured later on another screen, about the same fault.
pub const SECOND_NOTE: &str = "Overview still says three waiting after I answered one";

/// The Issue draft's title, as a person would write it up.
pub const DRAFT_TITLE: &str = "Counts go stale after the thing they count changes";

/// The Issue draft's body, as a person edited it before dispatching.
///
/// **Both Notes' words are in it**, because a write-up is made from the Notes
/// feeding it, and what the proposer and then the Drone are told has to be
/// what the person actually saw rather than a summary of it.
pub const DRAFT_BODY: &str = concat!(
    "Two places show a count that does not move when what it counts does.\n\n",
    "- The Board's filter chip keeps its count after the filter is cleared\n",
    "- Overview still says three waiting after I answered one\n"
);

/// The text a dispatch sends: **read off the Issue draft node on the Studio a
/// person promoted**, not written out beside it. `#1291` composes it in one
/// place and this is that place's answer.
pub fn an_issue_draft() -> String {
    let promoted = a_studio_promoted_to_a_job();
    promoted
        .nodes
        .iter()
        .find_map(|node| node.content().dispatched_as())
        .expect("the Issue draft the Cluster was written up as")
}

/// The request as Fleet receives it: encoded, sent, and read back.
///
/// **No `client_ref` and no attachments.** A draft is dispatched from its text
/// alone, and nothing here names a filed issue — filing one is optional
/// and a person's own act.
pub fn received_request(draft: &str) -> JobRequest {
    let sent = JobRequest {
        request: draft.to_string(),
        client_ref: None,
        attachments: Vec::new(),
    };
    let body = ipc::encode(&sent).expect("a request that serialises");
    ipc::decode("a Job request", body.as_bytes()).expect("a request that reads back")
}

/// The workflows the proposer is offered: one, the bench's Bug.
pub fn held() -> BTreeMap<WorkflowId, ResolvedWorkflow> {
    let bug = bug_workflow_with_the_fix_judged();
    BTreeMap::from([(bug.id().clone(), bug)])
}

/// A proposer's answer naming one Job under `workflow`, titled as the draft is.
pub fn one_job_under(workflow: &WorkflowId) -> String {
    format!(
        "workflow: {}\ntitle: {DRAFT_TITLE}\nbecause: a fault in what two screens draw",
        workflow.as_str()
    )
}

/// The repository the bench's Studio belongs to.
pub const REPOSITORY: &str = "01FIXTUREMANIFEST";

/// Where a person left the second Note, having dragged it off where it landed.
pub const LEFT_AT: StudioPosition = StudioPosition { x: 480, y: -120 };

fn at(minute: u32) -> Timestamp {
    Timestamp::from_rfc3339(format!("2026-09-17T09:{minute:02}:00.000Z"))
}

/// A Studio in the bench's repository holding both Notes, the second moved to
/// [`LEFT_AT`], and a Same as edge Helm proposed between them, not yet accepted.
pub fn a_studio_with_two_notes() -> StudioGraph {
    let note = |id: &str, said: &str, x: i64| {
        StudioNode::added(
            StudioNodeId::carried(Ulid::carried(id)),
            StudioNodeContent::Note {
                said: said.to_string(),
                capture: None,
            },
            StudioPosition { x, y: 0 },
            at(1),
            StudioAuthor::Person,
        )
    };
    let first = note("01NOTEFIRST", FIRST_NOTE, 0);
    let second = note("01NOTESECOND", SECOND_NOTE, 240).moved(LEFT_AT);
    let same_as = StudioEdge::proposed(
        StudioEdgeId::carried(Ulid::carried("01EDGESAMEAS")),
        first.id().clone(),
        second.id().clone(),
        StudioRelation::SameAs,
        at(2),
        StudioAuthor::Helm,
    )
    .expect("two different Notes");
    StudioGraph {
        studio: Studio {
            id: StudioId::carried(Ulid::carried("01STUDIO")),
            manifest_id: ManifestId::carried(Ulid::carried(REPOSITORY)),
            name: StudioName::named(DRAFT_TITLE),
            named_by: Some(StudioAuthor::Person),
            created_at: at(0),
            touched_at: at(3),
        },
        nodes: vec![first, second],
        edges: vec![same_as],
    }
}

/// A page nobody's adapter recognises, which is what keeps a Link a Link.
pub const A_PAGE: &str = "https://react.dev/reference/react/useId";

/// One node of each kind a forge address makes, and a Link beside them.
///
/// **Every node here is built by `adapters::forge_node`**, off an address, so
/// nothing in this bench decides a kind: the fixture is what the adapter
/// answered, which is the claim. The page is there because it is what the
/// adapter answered nothing about.
pub fn a_studio_with_a_node_of_each_forge_kind() -> StudioGraph {
    let addresses = [
        format!(
            "https://{}NickMele/armada/issues/1394",
            adapters::FORGE_HOST
        ),
        format!("https://{}NickMele/armada/pull/1391", adapters::FORGE_HOST),
        format!(
            "https://{}NickMele/armada/milestone/17",
            adapters::FORGE_HOST
        ),
    ];
    let mut nodes: Vec<StudioNode> = Vec::new();
    for (n, address) in addresses.iter().enumerate() {
        let content = adapters::forge_node(address, None)
            .unwrap_or_else(|| panic!("`{address}` names something on the forge"));
        nodes.push(StudioNode::added(
            StudioNodeId::carried(Ulid::carried(format!("01FORGE{n}"))),
            content,
            StudioPosition {
                x: (n as i64) * 340,
                y: 0,
            },
            at(1),
            StudioAuthor::Person,
        ));
    }
    nodes.push(StudioNode::added(
        StudioNodeId::carried(Ulid::carried("01PAGE")),
        adapters::forge_node(A_PAGE, None)
            .unwrap_or_else(|| StudioNodeContent::link(A_PAGE.to_string(), None)),
        StudioPosition { x: 1020, y: 0 },
        at(1),
        StudioAuthor::Person,
    ));
    StudioGraph {
        studio: Studio {
            id: StudioId::carried(Ulid::carried("01STUDIO")),
            manifest_id: ManifestId::carried(Ulid::carried(REPOSITORY)),
            name: StudioName::named(DRAFT_TITLE),
            named_by: Some(StudioAuthor::Person),
            created_at: at(0),
            touched_at: at(2),
        },
        nodes,
        edges: Vec::new(),
    }
}

/// The Studio as a client reads it: served, encoded, sent, and read back.
pub fn received_studio(graph: &StudioGraph) -> ipc::Studio {
    let body = ipc::encode(&ipc::Studio::of(graph)).expect("a Studio that serialises");
    ipc::decode("a Studio", body.as_bytes()).expect("a Studio that reads back")
}

/// The repository Helm is briefed for, named and never quoted.
pub fn helms_manifest() -> config::Manifest {
    config::Manifest::parse(
        std::path::Path::new("/work/storefront/armada.yml"),
        &format!("version: 1\nid: {REPOSITORY}\n"),
    )
    .expect("a Manifest")
}

/// Helm's act on a Studio as a client reads it off the stream.
pub fn received_event(event: &ipc::Event) -> ipc::Event {
    let body = ipc::encode(event).expect("an event that serialises");
    ipc::decode("an event", body.as_bytes()).expect("an event that reads back")
}

/// What a person asked a scout, on the bench's Studio.
pub const ASKED: &str = "How is a Job's count on the Board decided?";

/// The files the bench's scout read, in the order it read them.
pub const READ: [&str; 2] = [
    "packages/screens/src/Board.tsx",
    "crates/fleet/src/listing.rs",
];

/// The commit the bench's scout read, with a change on top not yet committed.
pub const COMMIT: &str = "4bdb169c0e1d2f3a4b5c6d7e8f9a0b1c2d3e4f5a";

/// What the scout cost, in millionths of a dollar.
pub const COST: u64 = 18_020;

/// The bench's Studio with a Finding on it, made by the first Note, which its
/// scout started, read [`READ`] into and froze as answered — **through the
/// transitions a scout's Fleet makes**, and nothing that sets a state by hand.
pub fn a_studio_with_a_frozen_finding() -> StudioGraph {
    let mut graph = a_studio_with_two_notes();
    let proposed = StudioNode::added(
        StudioNodeId::carried(Ulid::carried("01FINDING")),
        StudioNodeContent::Finding(StudioFinding::asked(ASKED)),
        StudioPosition { x: 0, y: 240 },
        at(4),
        StudioAuthor::Person,
    );
    let mut gathering = proposed
        .scouting(ScoutCheckout {
            commit: COMMIT.to_string(),
            uncommitted: true,
        })
        .expect("a Finding just added is Proposed");
    for file in READ {
        gathering.looked(ScoutLook::File(file.to_string()));
    }
    gathering.looked(ScoutLook::Search("count in packages/screens".to_string()));
    let frozen = gathering.frozen(
        Some("The Board counts what `list_job_board` answers.".to_string()),
        ScoutEnded {
            outcome: ScoutOutcome::Answered,
            cost_micros: Some(COST),
        },
    );
    let produced = StudioEdge::produced(
        StudioEdgeId::carried(Ulid::carried("01EDGEASKED")),
        graph.nodes[0].id().clone(),
        frozen.node().id().clone(),
        at(4),
        StudioAuthor::Person,
    )
    .expect("a Note and a Finding");
    graph.nodes.push(frozen.node().clone());
    graph.edges.push(produced);
    graph
}

/// The four sources a person pasted in and read, one Link each, in the order
/// `docs/concepts/scout.md` lists them. `#1293`.
pub const SOURCES: [(&str, &str, ScoutSourceKind); 4] = [
    (
        "01LINKISSUE",
        "https://example.invalid/o/r/issues/1293",
        ScoutSourceKind::Issue,
    ),
    (
        "01LINKPAGE",
        "https://react.dev/reference/react/useId",
        ScoutSourceKind::Page,
    ),
    (
        "01LINKSESSION",
        "armada:session/b1c9d559",
        ScoutSourceKind::Session,
    ),
    ("01LINKTHREAD", "armada:thread", ScoutSourceKind::Thread),
];

/// What a page did not fit into what its scout was handed.
pub const CUT: u64 = 12_400;

/// The Note the issue's read-in produced.
pub const READ_IN_NOTE: &str = "The issue makes reading in a second, refusable step";

/// The two sides of the Contradiction it found: the source's, then the
/// repository's. **Both are text**, because one side is outside the checkout
/// and has no node to point at.
pub const SIDES: (&str, &str) = (
    "The issue says Connections have no home yet",
    "docs/concepts/kit.md already gives them one",
);

/// A Studio where each of the four sources was read in: the Link standing with
/// its address, the Finding saying what it was handed, and — off the issue —
/// the Note and the Contradiction its scout asked for. `#1293`.
pub fn a_studio_with_sources_read_in() -> StudioGraph {
    let mut graph = a_studio_with_two_notes();
    let mut minted = 0;
    let mut produced = |graph: &mut StudioGraph, from: &StudioNodeId, node: StudioNode| {
        minted += 1;
        let edge = StudioEdge::produced(
            StudioEdgeId::carried(Ulid::carried(format!("01EDGEREADIN{minted:02}"))),
            from.clone(),
            node.id().clone(),
            at(6),
            StudioAuthor::Person,
        )
        .expect("a Link and what it produced");
        graph.nodes.push(node);
        graph.edges.push(edge);
    };
    let mut issue_link: Option<StudioNodeId> = None;
    for (n, (id, address, kind)) in SOURCES.iter().enumerate() {
        let link = StudioNode::added(
            StudioNodeId::carried(Ulid::carried(*id)),
            StudioNodeContent::Link {
                address: address.to_string(),
                said: None,
                named: None,
            },
            StudioPosition {
                x: (n as i64) * 340,
                y: 480,
            },
            at(5),
            StudioAuthor::Person,
        );
        let finding = StudioNode::added(
            StudioNodeId::carried(Ulid::carried(format!("01FINDINGREADIN{n}"))),
            StudioNodeContent::Finding(StudioFinding::asked(&format!("Read in {address}"))),
            StudioPosition {
                x: (n as i64) * 340,
                y: 660,
            },
            at(6),
            StudioAuthor::Person,
        )
        .reading_in(
            ScoutCheckout {
                commit: COMMIT.to_string(),
                uncommitted: false,
            },
            vec![ScoutSource {
                address: address.to_string(),
                kind: *kind,
                cut: match kind {
                    ScoutSourceKind::Page => CUT,
                    _ => 0,
                },
            }],
        )
        .expect("a Finding just added is Proposed")
        .frozen(
            Some(format!("What {address} says.")),
            ScoutEnded {
                outcome: ScoutOutcome::Answered,
                cost_micros: Some(COST),
            },
        );
        let link_id = link.id().clone();
        graph.nodes.push(link);
        produced(&mut graph, &link_id, finding.node().clone());
        if n == 0 {
            issue_link = Some(link_id);
        }
    }
    let from = issue_link.expect("the issue's Link");
    let note = StudioNode::added(
        StudioNodeId::carried(Ulid::carried("01NOTEREADIN")),
        StudioNodeContent::Note {
            said: READ_IN_NOTE.to_string(),
            capture: None,
        },
        StudioPosition { x: 0, y: 840 },
        at(7),
        StudioAuthor::Person,
    );
    let contradiction = StudioNode::added(
        StudioNodeId::carried(Ulid::carried("01CONTRAREADIN")),
        StudioNodeContent::Contradiction {
            first: SIDES.0.to_string(),
            second: SIDES.1.to_string(),
            answer: None,
        },
        StudioPosition { x: 0, y: 1020 },
        at(7),
        StudioAuthor::Person,
    );
    let blocks = StudioEdge::proposed(
        StudioEdgeId::carried(Ulid::carried("01EDGEREADINREL")),
        note.id().clone(),
        contradiction.id().clone(),
        StudioRelation::Blocks,
        at(7),
        StudioAuthor::Person,
    )
    .expect("a Note and a Contradiction");
    produced(&mut graph, &from, note);
    produced(&mut graph, &from, contradiction);
    graph.edges.push(blocks);
    graph
}

/// The run a person started from the Studio, by id.
pub const THE_RUN: &str = "01RUNTHATFAILED";

/// What the failed run's log ended on, and what a person opens the node for.
pub const THE_FAILURE: &str = "4 failed";

/// The command the Run node was started for.
pub const THE_COMMAND: &str = "pnpm test";

/// A Studio holding one Note and the Run a person started from it, with the
/// Produced edge between them.
///
/// `kept` is what retention left behind: `None` while the run is still there
/// to read, and `Some` once the sweep has been and gone.
pub fn a_studio_with_a_run_started_from_a_note(
    kept: Option<core_model::StudioRunKept>,
) -> StudioGraph {
    let note = StudioNode::added(
        StudioNodeId::carried(Ulid::carried("01NOTEFIRST")),
        StudioNodeContent::Note {
            said: FIRST_NOTE.to_string(),
            capture: None,
        },
        StudioPosition { x: 0, y: 0 },
        at(1),
        StudioAuthor::Person,
    );
    let run = StudioNode::added(
        StudioNodeId::carried(Ulid::carried("01RUNNODE")),
        StudioNodeContent::Run {
            run: core_model::StudioRun::Checkout(THE_RUN.to_string()),
            kept,
        },
        StudioPosition { x: 240, y: 0 },
        at(2),
        StudioAuthor::Person,
    );
    let produced = StudioEdge::produced(
        StudioEdgeId::carried(Ulid::carried("01EDGEPRODUCED")),
        note.id().clone(),
        run.id().clone(),
        at(2),
        StudioAuthor::Person,
    )
    .expect("a Note and the Run it started");
    StudioGraph {
        studio: Studio {
            id: StudioId::carried(Ulid::carried("01STUDIO")),
            manifest_id: ManifestId::carried(Ulid::carried(REPOSITORY)),
            name: StudioName::named(DRAFT_TITLE),
            named_by: Some(StudioAuthor::Person),
            created_at: at(0),
            touched_at: at(3),
        },
        nodes: vec![note, run],
        edges: vec![produced],
    }
}

/// The run's own record, as the checkout wrote it: it failed, and nobody
/// stopped it.
/// The server instance a Studio started, and the id its Run node holds.
pub const THE_SERVER: &str = "01STUDIOSERVER0000000000";

/// The `serve` line as it ran, `${port.NAME}` already resolved.
pub const THE_SERVE: &str =
    "pnpm -C packages/components exec storybook dev -p 41207 --no-open --ci";

/// A Studio holding one Note and the server a person started from it.
///
/// `kept` is what the node holds once the server has ended — `None` while
/// Fleet is still the one to read it from.
pub fn a_studio_with_a_server_started_from_a_note(
    kept: Option<core_model::StudioRunKept>,
) -> StudioGraph {
    let mut graph = a_studio_with_a_run_started_from_a_note(None);
    let server = StudioNode::added(
        StudioNodeId::carried(Ulid::carried("01SERVERNODE")),
        StudioNodeContent::Run {
            run: core_model::StudioRun::Server(THE_SERVER.to_string()),
            kept,
        },
        StudioPosition { x: 480, y: 0 },
        at(3),
        StudioAuthor::Person,
    );
    graph.nodes = vec![server];
    graph.edges = Vec::new();
    graph
}

/// The server as it ended: it exited on its own, with a code, having been up
/// for four minutes and having printed more than a node would carry.
pub fn a_server_that_fell_over() -> ipc::ServerState {
    ipc::ServerState {
        id: THE_SERVER.to_string(),
        name: String::from("storybook_dev"),
        job_id: None,
        manifest_id: None,
        phase: ipc::ServerPhase::Exited,
        serve: THE_SERVE.to_string(),
        ports: vec![ipc::ServerPort {
            name: String::from("storybook"),
            port: 41_207,
        }],
        links: vec![ipc::ServerLink {
            url: String::from("http://localhost:41207"),
            name: Some(String::from("Storybook")),
        }],
        started_by: ipc::StartedBy::Person,
        started_at: ipc::Instant::carried("2026-09-17T09:01:00.000Z"),
        serving_since: Some(ipc::Instant::carried("2026-09-17T09:01:12.000Z")),
        ended_at: Some(ipc::Instant::carried("2026-09-17T09:05:00.000Z")),
        exit_code: Some(0),
        ended: Some(String::from("exited 0")),
        stopped: false,
        log: format!(".armada/servers/main/{THE_SERVER}/output.log"),
    }
}

pub fn a_failed_run() -> ipc::CheckoutRunRecord {
    ipc::CheckoutRunRecord {
        id: THE_RUN.to_string(),
        name: String::from("test"),
        workspace: None,
        command: THE_COMMAND.to_string(),
        required: Vec::new(),
        started_at: ipc::Instant::carried("2026-09-17T09:01:00.000Z"),
        ended_at: ipc::Instant::carried("2026-09-17T09:01:12.400Z"),
        duration_ms: 12_400,
        exit_code: Some(1),
        expect_exit_code: 0,
        ended: String::from("exited 1"),
        stopped: false,
        changed: Vec::new(),
        changed_unreadable: None,
        undoable: false,
        undone_at: None,
        log: format!(".armada/runs/main/{THE_RUN}/output.log"),
    }
}

/// The run's log, longer than any node would carry: the failure is on the last
/// line, where a test runner puts it.
pub fn a_long_log() -> ipc::RunOutput {
    let mut lines: Vec<String> = (1..5_000).map(|at| format!("line {at}")).collect();
    lines.push(THE_FAILURE.to_string());
    ipc::RunOutput {
        id: THE_RUN.to_string(),
        name: String::from("test"),
        path: format!(".armada/runs/main/{THE_RUN}/output.log"),
        from_line: 1,
        total_lines: lines.len() as u32,
        bytes: lines.iter().map(|line| line.len() as u64 + 1).sum(),
        whole: true,
        lines,
    }
}

/// The component the bench's capture landed on, and the chain above it.
pub const COMPONENT: &str = "FilterChip";
pub const OWNERS: [&str; 2] = ["BoardFilters", "Board"];

/// The selector the capture found the chip by, and finds it by again.
pub const SELECTOR: &str = "div.armada-board__filters > button.armada-chip:nth-of-type(2)";

/// The rail item marked current when the capture was made.
pub const SCREEN: &str = "Job Board";

/// The markup the capture trimmed, and the properties it read off the element.
pub const MARKUP: &str = "<button class=\"armada-chip\" aria-pressed=\"true\">Queued 3</button>";
pub const STYLES: [(&str, &str); 2] = [("color", "rgb(232, 232, 237)"), ("font-size", "12px")];

/// The frame Fleet kept beside the Studio's records, and what it weighs.
pub const FRAME_FILE: &str = "01NOTEPOINTED.png";
pub const FRAME_BYTES: u64 = 214_880;

/// The capture as Bridge sends it: the request encoded, sent, and read back.
///
/// **`frame` is what Fleet kept, not what Bridge staged.** The staged path is
/// an input to the command and reaches no client, so what a Studio answers
/// with is the file name and its size.
pub fn a_capture_sent() -> ipc::CaptureStudioNote {
    let sent = ipc::CaptureStudioNote {
        said: FIRST_NOTE.to_string(),
        capture: ipc::StudioCapture {
            component: Some(COMPONENT.to_string()),
            owners: OWNERS.iter().map(|name| name.to_string()).collect(),
            selector: SELECTOR.to_string(),
            element: ipc::CaptureElement {
                tag: "button".to_string(),
                text: "Queued 3".to_string(),
                label: None,
            },
            screen: Some(SCREEN.to_string()),
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
            styles: STYLES
                .iter()
                .map(|(property, value)| (property.to_string(), value.to_string()))
                .collect(),
            markup: MARKUP.to_string(),
            source: None,
            frame: None,
            // Captured on Bridge, which is what carries none — `#1294`.
            served: None,
        },
        position: ipc::StudioPosition { x: 0, y: 0 },
        frame: Some(ipc::StagedFrame {
            staged_path: "/tmp/armada-frames/4f3a/frame.png".to_string(),
            width: 2880,
            height: 1800,
        }),
        produced_by: None,
    };
    let body = ipc::encode(&sent).expect("a capture that serialises");
    ipc::decode("a Note to capture", body.as_bytes()).expect("a capture that reads back")
}

/// A Studio holding one Note, built from [`a_capture_sent`] the way Fleet
/// builds it: the request's capture to the domain, the frame Fleet kept in
/// place of the one Bridge staged, and the node added by a person.
pub fn a_studio_with_a_captured_note() -> StudioGraph {
    let sent = a_capture_sent();
    let staged = sent.frame.as_ref().expect("the bench stages a frame");
    let mut capture = sent.capture.to_domain();
    capture.frame = Some(core_model::CaptureFrame {
        filename: FRAME_FILE.to_string(),
        byte_size: FRAME_BYTES,
        width: staged.width,
        height: staged.height,
    });
    let note = StudioNode::added(
        StudioNodeId::carried(Ulid::carried("01NOTEPOINTED")),
        StudioNodeContent::Note {
            said: sent.said,
            capture: Some(capture),
        },
        sent.position.to_domain(),
        at(5),
        StudioAuthor::Person,
    );
    StudioGraph {
        studio: Studio {
            id: StudioId::carried(Ulid::carried("01STUDIOCAPTURE")),
            manifest_id: ManifestId::carried(Ulid::carried(REPOSITORY)),
            name: None,
            named_by: None,
            created_at: at(5),
            touched_at: at(5),
        },
        nodes: vec![note],
        edges: Vec::new(),
    }
}

/// The Studio after `#1291`'s own rungs: both Notes accepted as one **Cluster**
/// with a `Produced` edge from each, the Cluster written up as an **Issue
/// draft** a person edited, and the draft dispatched to a **Job** node.
///
/// **Built through the constructors alone**, so every edge here is one the
/// Studio draws itself and nothing sets a state by hand.
pub fn a_studio_promoted_to_a_job() -> StudioGraph {
    let mut graph = a_studio_with_two_notes();
    let notes: Vec<StudioNodeId> = graph.nodes.iter().map(|node| node.id().clone()).collect();
    let made = |graph: &mut StudioGraph,
                id: &str,
                edge: &str,
                content: StudioNodeContent,
                from: &[StudioNodeId],
                at_minute: u32,
                x: i64| {
        let node = StudioNode::added(
            StudioNodeId::carried(Ulid::carried(id)),
            content,
            StudioPosition { x, y: 240 },
            at(at_minute),
            StudioAuthor::Person,
        );
        for (nth, source) in from.iter().enumerate() {
            let produced = StudioEdge::produced(
                StudioEdgeId::carried(Ulid::carried(format!("{edge}{nth}"))),
                source.clone(),
                node.id().clone(),
                at(at_minute),
                StudioAuthor::Person,
            )
            .expect("two different nodes");
            graph.edges.push(produced);
        }
        let id = node.id().clone();
        graph.nodes.push(node);
        id
    };

    let cluster = made(
        &mut graph,
        "01CLUSTER",
        "01EDGECLUSTER",
        StudioNodeContent::Cluster {
            title: String::from("Counts go stale"),
        },
        &notes,
        5,
        0,
    );
    let draft = made(
        &mut graph,
        "01DRAFT",
        "01EDGEDRAFT",
        StudioNodeContent::IssueDraft {
            title: DRAFT_TITLE.to_string(),
            body: DRAFT_BODY.to_string(),
        },
        &[cluster],
        6,
        240,
    );
    made(
        &mut graph,
        "01JOBNODE",
        "01EDGEJOB",
        StudioNodeContent::Job {
            job_id: core_model::JobId::carried(Ulid::carried("01JOBATTHEGATE")),
        },
        &[draft],
        7,
        480,
    );
    graph
}
