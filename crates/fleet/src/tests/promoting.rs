//! Promotion through Fleet: two Notes clustered, the Cluster written up, the
//! draft edited and dispatched, and a Contradiction ended four ways. `#1291`.
//!
//! **What is proved here is the record, not the rendering.** Every rung
//! answers with the Studio whole, so each assertion reads the nodes and edges
//! that came back rather than a return value of its own.
//!
//! **Nothing here reaches a forge**, and nothing could: no seam in this crate
//! offers one to a Studio, which is the point — `docs/concepts/studio.md`
//! makes filing a person's own act.
//!
//! **Over 500 lines, and staying one file**, `crate::tests::proposing`'s
//! reason: the rungs share a Studio and two of them end a Contradiction as
//! they go, so a change to one has to be checked against the others it runs
//! beside. Split by rung, the four-outcomes test would have nowhere to live.

use api::{Redirector, Refusal, Studios};
use core_model::StudioNodeKind;
use ipc::{
    AddStudioNode, ContradictionSettled, CreateStudio, DeferOnStudio, DispatchStudioDraft,
    EditStudioDraft, GroupStudioNodes, SettleContradiction, Studio, StudioNode, StudioNodeContent,
    StudioNodeId, StudioPosition, WriteUpStudioNode,
};
use testkit::{FakeJudge, FakeWorkProduct};

use crate::tests::daemon::{a_fleet_proposing_through, fittings};
use crate::tests::proposing::a_catalogue;
use crate::tests::tmp::TempDir;

type Fixture = crate::daemon::Fleet<testkit::FakeHarness, testkit::FakeVcs, FakeWorkProduct>;

const FIRST_NOTE: &str = "The chip keeps its count after the filter is cleared";
const SECOND_NOTE: &str = "Overview still says three waiting after I answered one";
const DRAFT_TITLE: &str = "Counts go stale after the thing they count changes";
const DRAFT_BODY: &str = "Two places show a count that does not move.";

fn a_fleet(home: &TempDir) -> Fixture {
    crate::daemon::Fleet::assembled(fittings(home, FakeWorkProduct::changed(&[])))
}

/// A Fleet whose proposer answers with one Bug, so a dispatch reaches the gate.
fn a_fleet_that_proposes(home: &TempDir) -> Fixture {
    a_fleet_proposing_through(
        home,
        FakeWorkProduct::changed(&[]),
        a_catalogue(),
        FakeJudge::saying(&format!(
            "workflow: bug\ntitle: {DRAFT_TITLE}\nbecause: a fault in what two screens draw"
        )),
    )
}

fn code(refusal: &Refusal) -> &str {
    match refusal {
        Refusal::NoSuchJob(e)
        | Refusal::IllegalMove(e)
        | Refusal::Unacceptable(e)
        | Refusal::Fault(e) => &e.code,
    }
}

fn at(x: i64, y: i64) -> StudioPosition {
    StudioPosition { x, y }
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
        .expect("a Studio in the repository Fleet starts in")
}

/// A node no route mints, written straight through the store.
///
/// **A Contradiction is the one kind this test needs and nothing can add**:
/// reading in is what makes one (`#1293`), and a person adds a Note, a Link or
/// a Sketch by hand and nothing else (`#1364`). So the four outcomes are
/// exercised against a Contradiction the record holds, rather than through a
/// door that would have to be opened for a test.
async fn seeded(fleet: &Fixture, studio: &Studio, content: StudioNodeContent) -> StudioNodeId {
    let id = studio.id.to_domain();
    let at = fleet.now();
    let node = core_model::StudioNode::added(
        core_model::StudioNodeId::carried(fleet.mint().ulid()),
        content.to_domain(),
        core_model::StudioPosition { x: 0, y: 0 },
        at.clone(),
        core_model::StudioAuthor::Person,
    );
    let mut store = fleet.store().lock().await;
    store.add_studio_node(&id, &node, None, &at).expect("added");
    StudioNodeId::from(node.id())
}

async fn added(fleet: &Fixture, studio: &Studio, content: StudioNodeContent) -> StudioNodeId {
    let studio = fleet
        .add_studio_node(
            studio.id.clone(),
            AddStudioNode {
                content,
                position: at(0, 0),
                produced_by: None,
            },
            Redirector::Person,
            None,
        )
        .await
        .expect("a person's node");
    studio.nodes.last().expect("the node just added").id.clone()
}

/// The two Notes the bench's Studio is worked out from.
async fn two_notes(fleet: &Fixture, studio: &Studio) -> (StudioNodeId, StudioNodeId) {
    let note = |said: &str| StudioNodeContent::Note {
        said: said.to_string(),
        capture: None,
    };
    (
        added(fleet, studio, note(FIRST_NOTE)).await,
        added(fleet, studio, note(SECOND_NOTE)).await,
    )
}

fn of_kind<'a>(studio: &'a Studio, kind: StudioNodeKind) -> Vec<&'a StudioNode> {
    studio
        .nodes
        .iter()
        .filter(|node| node.content.to_domain().kind() == kind)
        .collect()
}

/// Every `produced` edge into `node`, as the ids that made it.
fn made_by<'a>(studio: &'a Studio, node: &StudioNodeId) -> Vec<&'a StudioNodeId> {
    studio
        .edges
        .iter()
        .filter(|edge| &edge.to == node && edge.kind.as_wire() == "produced")
        .map(|edge| &edge.from)
        .collect()
}

/// The issue's own definition of done, as far as Fleet carries it: two Notes
/// clustered, the Cluster written up, the draft edited, and the draft
/// dispatched to a Job node at the gate, each link a `produced` edge.
#[tokio::test]
async fn two_notes_are_clustered_written_up_edited_and_dispatched_to_a_job_at_the_gate() {
    let home = TempDir::new();
    let fleet = a_fleet_that_proposes(&home);
    let studio = a_studio(&fleet).await;
    let (first, second) = two_notes(&fleet, &studio).await;

    let clustered = fleet
        .group_studio_nodes(
            studio.id.clone(),
            GroupStudioNodes {
                content: StudioNodeContent::Cluster {
                    title: "Counts go stale".to_string(),
                },
                from: vec![first.clone(), second.clone()],
                position: at(240, 0),
            },
            None,
        )
        .await
        .expect("two Notes a person accepted as one thing");
    let [cluster] = of_kind(&clustered, StudioNodeKind::Cluster)[..] else {
        panic!("one Cluster");
    };
    assert_eq!(
        made_by(&clustered, &cluster.id),
        [&first, &second],
        "a Produced edge from each Note, in the order the person picked them"
    );

    let written = fleet
        .write_up_studio_node(
            studio.id.clone(),
            WriteUpStudioNode {
                node_id: cluster.id.clone(),
                title: DRAFT_TITLE.to_string(),
                body: DRAFT_BODY.to_string(),
                position: at(480, 0),
            },
            Redirector::Person,
            None,
        )
        .await
        .expect("a Cluster written up");
    let [draft] = of_kind(&written, StudioNodeKind::IssueDraft)[..] else {
        panic!("one Issue draft");
    };
    let draft = draft.id.clone();
    assert_eq!(
        made_by(&written, &draft),
        [&cluster.id],
        "the draft points back at the Cluster it was written up from"
    );

    let edited = fleet
        .edit_studio_draft(
            studio.id.clone(),
            EditStudioDraft {
                node_id: draft.clone(),
                title: DRAFT_TITLE.to_string(),
                body: format!("{DRAFT_BODY}\n\n- {FIRST_NOTE}\n- {SECOND_NOTE}\n"),
            },
            None,
        )
        .await
        .expect("a draft a person edited");
    let [after] = of_kind(&edited, StudioNodeKind::IssueDraft)[..] else {
        panic!("one Issue draft");
    };
    let StudioNodeContent::IssueDraft { body, .. } = &after.content else {
        panic!("a draft");
    };
    assert!(
        body.contains(FIRST_NOTE) && body.contains(SECOND_NOTE),
        "what the person added is what is kept: {body}"
    );

    let dispatched = std::sync::Arc::new(fleet)
        .dispatch_studio_draft(
            studio.id.clone(),
            DispatchStudioDraft {
                node_id: draft.clone(),
                position: at(720, 0),
            },
            Redirector::Person,
            None,
        )
        .await
        .expect("a draft a person dispatched");
    let [job] = of_kind(&dispatched, StudioNodeKind::Job)[..] else {
        panic!("one Job node");
    };
    assert_eq!(
        made_by(&dispatched, &job.id),
        [&draft],
        "the Job points back at the draft it was dispatched from"
    );
    assert!(
        job.state.is_none(),
        "a Job node holds a reference, and its status is read off the Board"
    );
}

/// **The gate is unchanged.** A Job dispatched from a Studio stands at
/// `awaiting_approval` like any other, and carries the draft's own text — the
/// title, then the body — as what its Drone is told.
///
/// **And its origin is whoever pressed it.** Every other request through the
/// proposer is one Fleet read, which a row draws as *Found by Fleet*; a
/// dispatch from a Studio is a person sending a draft they wrote up, and a row
/// saying Armada found it would be a sentence nobody could act on.
#[tokio::test]
async fn a_job_dispatched_from_a_draft_stands_at_the_gate_with_the_drafts_own_words() {
    let home = TempDir::new();
    let fleet = std::sync::Arc::new(a_fleet_that_proposes(&home));
    let studio = a_studio(&fleet).await;
    let note = added(
        &fleet,
        &studio,
        StudioNodeContent::Note {
            said: FIRST_NOTE.to_string(),
            capture: None,
        },
    )
    .await;
    let written = fleet
        .write_up_studio_node(
            studio.id.clone(),
            WriteUpStudioNode {
                node_id: note,
                title: DRAFT_TITLE.to_string(),
                body: format!("{DRAFT_BODY}\n\n- {FIRST_NOTE}\n"),
                position: at(240, 0),
            },
            Redirector::Person,
            None,
        )
        .await
        .expect("a Note written up");
    let draft = of_kind(&written, StudioNodeKind::IssueDraft)[0].id.clone();

    std::sync::Arc::clone(&fleet)
        .dispatch_studio_draft(
            studio.id.clone(),
            DispatchStudioDraft {
                node_id: draft,
                position: at(480, 0),
            },
            Redirector::Person,
            None,
        )
        .await
        .expect("a draft dispatched");

    let listed = api::Queries::list_jobs(fleet.as_ref(), None)
        .await
        .expect("the Board reads");
    let [job] = &listed.jobs[..] else {
        panic!("one Job, not {}", listed.jobs.len())
    };
    assert_eq!(
        job.status.as_wire(),
        "awaiting_approval",
        "the dispatch gate is the same gate"
    );
    assert_eq!(job.title, DRAFT_TITLE);
    assert_eq!(
        job.origin.as_wire(),
        "manual",
        "a person pressed dispatch, so the row says Dispatched by you"
    );
    let detail = api::Queries::get_job(fleet.as_ref(), job.id.clone())
        .await
        .expect("the Job it drafted");
    let facts = detail.facts.expect("the brief the draft became");
    assert!(
        facts.starts_with(DRAFT_TITLE) && facts.contains(FIRST_NOTE),
        "the draft crosses whole — title, then body: {facts}"
    );
}

/// **Helm dispatching on a person's ask draws as Helm's.** The same draft, the
/// same gate, and the one thing that differs is who a row says sent it.
#[tokio::test]
async fn a_draft_helm_dispatched_on_an_ask_says_it_was_drafted_in_helm() {
    let home = TempDir::new();
    let fleet = std::sync::Arc::new(a_fleet_that_proposes(&home));
    let studio = a_studio(&fleet).await;
    let note = added(
        &fleet,
        &studio,
        StudioNodeContent::Note {
            said: FIRST_NOTE.to_string(),
            capture: None,
        },
    )
    .await;
    let written = fleet
        .write_up_studio_node(
            studio.id.clone(),
            WriteUpStudioNode {
                node_id: note,
                title: DRAFT_TITLE.to_string(),
                body: DRAFT_BODY.to_string(),
                position: at(240, 0),
            },
            Redirector::Helm,
            None,
        )
        .await
        .expect("Helm writing up on an ask");
    let draft = of_kind(&written, StudioNodeKind::IssueDraft)[0].id.clone();

    std::sync::Arc::clone(&fleet)
        .dispatch_studio_draft(
            studio.id.clone(),
            DispatchStudioDraft {
                node_id: draft,
                position: at(480, 0),
            },
            Redirector::Helm,
            None,
        )
        .await
        .expect("Helm dispatching on an ask");

    let listed = api::Queries::list_jobs(fleet.as_ref(), None)
        .await
        .expect("the Board reads");
    let [job] = &listed.jobs[..] else {
        panic!("one Job, not {}", listed.jobs.len())
    };
    assert_eq!(
        job.origin.as_wire(),
        "helm_drafted",
        "Helm sent it, so the row says Drafted in Helm"
    );
    assert_eq!(
        job.status.as_wire(),
        "awaiting_approval",
        "and it still takes the person's approval"
    );
}

/// **Only a person defers, and the Deferral says what it holds up.** The
/// `blocks` edge lands accepted, since the person drawing it is the one who
/// would have accepted it.
#[tokio::test]
async fn a_deferral_is_produced_by_what_raised_it_and_blocks_what_it_holds_up() {
    let home = TempDir::new();
    let fleet = a_fleet(&home);
    let studio = a_studio(&fleet).await;
    let (first, second) = two_notes(&fleet, &studio).await;

    let deferred = fleet
        .defer_on_studio(
            studio.id.clone(),
            DeferOnStudio {
                what: "Whether a count is the Board's or the row's".to_string(),
                raised_on: first.clone(),
                blocks: Some(second.clone()),
                position: at(0, 240),
            },
            None,
        )
        .await
        .expect("a person's Deferral");
    let [deferral] = of_kind(&deferred, StudioNodeKind::Deferral)[..] else {
        panic!("one Deferral");
    };
    assert_eq!(deferral.state.map(|state| state.as_wire()), Some("open"));
    assert_eq!(made_by(&deferred, &deferral.id), [&first]);
    let blocks = deferred
        .edges
        .iter()
        .find(|edge| edge.kind.as_wire() == "blocks")
        .expect("what it holds up");
    assert_eq!((&blocks.from, &blocks.to), (&deferral.id, &second));
    assert_eq!(
        blocks.standing.as_wire(),
        "accepted",
        "a person drawing a relation is the person who accepts one"
    );
}

/// A Contradiction ends one of four ways, and each of the four leaves the node
/// saying which. **The two that make a node are the rungs that make it** — so
/// there is one way to write up and one way to defer, not two.
#[tokio::test]
async fn a_contradiction_ends_one_of_four_ways_and_never_twice() {
    let home = TempDir::new();
    let fleet = a_fleet(&home);
    let studio = a_studio(&fleet).await;
    let disagreement = || StudioNodeContent::Contradiction {
        first: "The chip reads the Board".to_string(),
        second: "The chip reads its own row".to_string(),
        answer: None,
    };
    let state = |studio: &Studio, node: &StudioNodeId| {
        studio
            .nodes
            .iter()
            .find(|one| &one.id == node)
            .and_then(|one| one.state)
            .map(|state| state.as_wire())
    };

    // Written up: the Contradiction ends as `issue_draft`, and the draft is a
    // node on the far end of a Produced edge.
    let written_up = seeded(&fleet, &studio, disagreement()).await;
    let after = fleet
        .write_up_studio_node(
            studio.id.clone(),
            WriteUpStudioNode {
                node_id: written_up.clone(),
                title: DRAFT_TITLE.to_string(),
                body: DRAFT_BODY.to_string(),
                position: at(240, 0),
            },
            Redirector::Person,
            None,
        )
        .await
        .expect("a Contradiction written up");
    assert_eq!(state(&after, &written_up), Some("issue_draft"));

    // Deferred: it ends as `deferral`.
    let put_off = seeded(&fleet, &studio, disagreement()).await;
    let after = fleet
        .defer_on_studio(
            studio.id.clone(),
            DeferOnStudio {
                what: "Which of the two the chip should read".to_string(),
                raised_on: put_off.clone(),
                blocks: None,
                position: at(0, 480),
            },
            None,
        )
        .await
        .expect("a Contradiction deferred");
    assert_eq!(state(&after, &put_off), Some("deferral"));

    // Not a problem, and Resolved here, which keeps the answer on the node.
    let held = seeded(&fleet, &studio, disagreement()).await;
    let after = fleet
        .settle_contradiction(
            studio.id.clone(),
            SettleContradiction {
                node_id: held.clone(),
                outcome: ContradictionSettled::NotAProblem,
            },
            None,
        )
        .await
        .expect("both statements holding");
    assert_eq!(state(&after, &held), Some("not_a_problem"));

    let settled = seeded(&fleet, &studio, disagreement()).await;
    let after = fleet
        .settle_contradiction(
            studio.id.clone(),
            SettleContradiction {
                node_id: settled.clone(),
                outcome: ContradictionSettled::ResolvedHere {
                    answer: "The chip reads the Board, and the row is stale".to_string(),
                },
            },
            None,
        )
        .await
        .expect("a person settling it");
    assert_eq!(state(&after, &settled), Some("resolved_here"));
    let node = after
        .nodes
        .iter()
        .find(|one| one.id == settled)
        .expect("the Contradiction");
    let StudioNodeContent::Contradiction { answer, .. } = &node.content else {
        panic!("a Contradiction");
    };
    assert_eq!(
        answer.as_deref(),
        Some("The chip reads the Board, and the row is stale"),
        "Resolved here is only a record if the answer is on the node"
    );

    // A Contradiction ends once: the node keeps what was acted on.
    let again = fleet
        .settle_contradiction(
            studio.id.clone(),
            SettleContradiction {
                node_id: settled,
                outcome: ContradictionSettled::NotAProblem,
            },
            None,
        )
        .await
        .expect_err("one that has already ended");
    assert_eq!(code(&again), "fleet.studio_contradiction_settled");
}

/// Each rung refuses the kinds it is not for, rather than making a node that
/// claims something nobody said.
#[tokio::test]
async fn each_rung_refuses_the_kinds_it_is_not_for() {
    let home = TempDir::new();
    let fleet = a_fleet(&home);
    let studio = a_studio(&fleet).await;
    let (first, second) = two_notes(&fleet, &studio).await;
    let link = added(
        &fleet,
        &studio,
        StudioNodeContent::Link {
            address: "https://example.invalid/board".to_string(),
        },
    )
    .await;

    let grouped = |content, from| GroupStudioNodes {
        content,
        from,
        position: at(0, 720),
    };
    let refused = fleet
        .group_studio_nodes(
            studio.id.clone(),
            grouped(
                StudioNodeContent::Note {
                    said: "not a group".to_string(),
                    capture: None,
                },
                vec![first.clone(), second.clone()],
            ),
            None,
        )
        .await
        .expect_err("a Note is not a group");
    assert_eq!(code(&refused), "fleet.studio_group_not_a_group");

    let refused = fleet
        .group_studio_nodes(
            studio.id.clone(),
            grouped(
                StudioNodeContent::Cluster {
                    title: "Counts".to_string(),
                },
                vec![first.clone(), link.clone()],
            ),
            None,
        )
        .await
        .expect_err("a Cluster is of Notes");
    assert_eq!(code(&refused), "fleet.studio_cluster_is_of_notes");

    let refused = fleet
        .group_studio_nodes(
            studio.id.clone(),
            grouped(
                StudioNodeContent::Cluster {
                    title: "Counts".to_string(),
                },
                vec![first.clone(), first.clone()],
            ),
            None,
        )
        .await
        .expect_err("one node named twice is not two nodes");
    assert_eq!(code(&refused), "fleet.studio_group_is_of_several");

    let refused = fleet
        .write_up_studio_node(
            studio.id.clone(),
            WriteUpStudioNode {
                node_id: link.clone(),
                title: DRAFT_TITLE.to_string(),
                body: DRAFT_BODY.to_string(),
                position: at(0, 960),
            },
            Redirector::Person,
            None,
        )
        .await
        .expect_err("a Link is read in, not written up");
    assert_eq!(code(&refused), "fleet.studio_not_writable_up");

    let refused = fleet
        .edit_studio_draft(
            studio.id.clone(),
            EditStudioDraft {
                node_id: first.clone(),
                title: DRAFT_TITLE.to_string(),
                body: DRAFT_BODY.to_string(),
            },
            None,
        )
        .await
        .expect_err("a Note is fixed at capture");
    assert_eq!(code(&refused), "fleet.studio_not_a_draft");

    let refused = fleet
        .settle_contradiction(
            studio.id.clone(),
            SettleContradiction {
                node_id: first,
                outcome: ContradictionSettled::NotAProblem,
            },
            None,
        )
        .await
        .expect_err("a Note ends nothing");
    assert_eq!(code(&refused), "fleet.studio_not_a_contradiction");

    let refused = std::sync::Arc::new(fleet)
        .dispatch_studio_draft(
            studio.id.clone(),
            DispatchStudioDraft {
                node_id: link,
                position: at(0, 1200),
            },
            Redirector::Person,
            None,
        )
        .await
        .expect_err("a Link is not dispatched");
    assert_eq!(code(&refused), "fleet.studio_not_a_draft");
}
