//! Studio's claim: **I can work something out on a Studio — notes from using
//! the app, what a scout read, what I pasted in — turn what holds up into a
//! Job, and come back later to see how I got there.** The apparatus is
//! [`bench::studio`].
//!
//! **Most of it is not carried yet, and a green run here says so rather than
//! hiding it.** The record exists; what a person does with it mostly does not,
//! so the rest of the claim is the table below, in the order a person meets it.
//! The change that builds a row adds its assertion to this file, made against a
//! value that has been through [`ipc::encode`] and back as `board.rs`'s are,
//! and takes the row out.
//!
//! | Step of the claim | Carried by |
//! |---|---|
//! | 2. A Run node is started from it and ends failed, and keeps its log's tail and result past retention | #1289. **What it meets:** a checkout run today is `ipc::CheckoutRunRecord`, whose end is an exit code and a sentence documented as unhued — there is no run state for a Run node to alias to a Job status |
//! | 3. A Note is captured, and nothing writes to it afterwards | #1290 |
//! | 4. A scout's Finding arrives Frozen, listing every file and source it read | #1292 |
//! | 5. An issue from the repository's forge is read in, and a Contradiction appears | #1293 |
//! | 6. Two Notes are clustered, written up as an Issue draft, and dispatched, and a Job node stands at the gate | #1291. **The dispatch's far half is asserted below**: an Issue draft's text alone reaches the Job proposer, and the Job it proposes is told all of it |
//! | Helm's unasked proposals, and the acts it takes only on a person's ask | #1288 |
//!
//! | Not proved here | Why not |
//! |---|---|
//! | That a proposer reading the draft chooses well | Choosing is a model's, and this file calls none. The answer below is written by the test |
//! | That the proposed Job is created at `awaiting_approval` | `fleet::drafting`'s conversion from a proposal to a Job is `pub(crate)`, so building one here would assert what the test built. `fleet`'s own tests create one |
//! | That a Studio survives a restart on disk | It touches a file. `store` and `fleet` reopen one in their own tests; what is asserted here is the record reading back through the wire |
//! | That a reopened Studio is read-only until Continue | A Bridge state; #1287's mock browser test proves it |
//! | Anything a person sees | Nothing here renders. The whiteboard is #1286 and #1287 |

// The bench is shared with the other milestones' tests and none of them uses
// all of it.
#[allow(dead_code)]
mod bench;

use fleet::{Brief, Proposal};
use ipc::door::{DRAFTING, HELM_ONLY, REACHABLE};
use ipc::StudioNodeContent;

use bench::studio::{
    a_studio_with_two_notes, an_issue_draft, held, one_job_under, received_request,
    received_studio, DRAFT_TITLE, FIRST_NOTE, LEFT_AT, REPOSITORY, SECOND_NOTE,
};

/// Step 6's far half: **an Issue draft is dispatched from its text, through the
/// Job proposer, and the Job it becomes is told the whole of it.**
/// `docs/concepts/studio.md`, *Promotion*: filing the issue anywhere is
/// optional, so nothing between the draft and the proposer may need it filed.
///
/// **The failure this is against is a lossy hop.** A write-up is made from the
/// Notes feeding it, and a dispatch that summarised, trimmed or re-fetched the
/// draft on the way would hand a Drone something other than what the person
/// saw. So the draft's text goes over the wire as a request, the proposer is
/// asked with it, and a one-Job answer reads back with the draft as that Job's
/// brief, both Notes' words in it.
#[test]
fn an_issue_drafts_text_alone_reaches_the_proposer_and_the_job_is_told_all_of_it() {
    let draft = an_issue_draft();
    let received = received_request(&draft);
    assert_eq!(
        received.request, draft,
        "the draft crosses the wire verbatim, with no issue filed to point at"
    );

    let held = held();
    let brief = Brief::about(&received.request, &held);
    assert!(
        brief.question().contains(&draft),
        "the proposer is asked with the draft as the person wrote it up: {}",
        brief.question()
    );

    let workflow = held.keys().next().expect("the bench holds a workflow");
    let Ok(Proposal::Resolved(jobs)) = brief.read(&one_job_under(workflow), &held) else {
        panic!("an answer naming a held workflow and a title is a proposal");
    };
    assert_eq!(
        jobs.len(),
        1,
        "one draft, written up as one thing, is one Job"
    );
    let job = &jobs[0];
    assert_eq!(job.title, DRAFT_TITLE, "the Job keeps the draft's title");
    assert_eq!(
        job.brief, draft,
        "what the Job's Drone is told is the whole draft, not a part of it"
    );
    for note in [FIRST_NOTE, SECOND_NOTE] {
        assert!(
            job.brief.contains(note),
            "both Notes the draft was written up from reach the Drone: {note}"
        );
    }
}

/// Steps 1 and 7: **a Studio belongs to a repository, and what a client reads
/// back is every node, edge and position as a person left them.**
///
/// **The failure this is against is a whiteboard that reopens rearranged or
/// with a relation silently accepted.** A position dropped on the way out puts
/// a node back where it was added; a standing dropped makes a proposal read as
/// a person's decision. So the Studio crosses the wire, and what arrives is
/// held to the record field by field.
#[test]
fn a_studio_reads_back_with_every_node_where_it_was_left_and_its_proposal_unaccepted() {
    let graph = a_studio_with_two_notes();
    let studio = received_studio(&graph);
    assert_eq!(studio, ipc::Studio::of(&graph), "nothing lost on the wire");
    assert_eq!(
        studio.manifest_id.as_str(),
        REPOSITORY,
        "the repository's own"
    );
    assert_eq!(studio.name.as_deref(), Some(DRAFT_TITLE));

    let said: Vec<_> = studio
        .nodes
        .iter()
        .map(|node| match &node.content {
            StudioNodeContent::Note { said } => said.as_str(),
            other => panic!("only Notes were put on it: {other:?}"),
        })
        .collect();
    assert_eq!(said, [FIRST_NOTE, SECOND_NOTE], "both Notes, word for word");
    assert_eq!(
        (studio.nodes[1].position.x, studio.nodes[1].position.y),
        (LEFT_AT.x, LEFT_AT.y),
        "the moved Note is where it was left, not where it landed"
    );
    assert!(
        studio.nodes.iter().all(|node| node.state.is_none()),
        "a Note has no state"
    );

    let edge = &studio.edges[0];
    assert_eq!(edge.kind.as_wire(), "same_as");
    assert_eq!(
        edge.standing.as_wire(),
        "proposed",
        "a proposal stays one until a person accepts it"
    );
    assert_eq!(
        (&edge.from, &edge.to),
        (&studio.nodes[0].id, &studio.nodes[1].id)
    );
}

/// **Accepting a relation, removing a node and deleting a Studio are a
/// person's acts**, whatever Helm's authority says: no agent is offered one,
/// and Helm alone is offered what it may propose. `docs/concepts/studio.md`.
#[test]
fn no_agent_is_offered_a_persons_act_on_a_studio() {
    let offered = |operation: &str| {
        [REACHABLE, DRAFTING, HELM_ONLY]
            .iter()
            .any(|door| door.iter().any(|row| row.operation == operation))
    };
    for persons in ["decide_studio_edge", "remove_studio_node", "delete_studio"] {
        assert!(!offered(persons), "`{persons}` reaches an agent");
    }
    for helms in ["add_studio_node", "propose_studio_edge"] {
        assert!(
            HELM_ONLY.iter().any(|row| row.operation == helms),
            "`{helms}`"
        );
        assert!(
            !REACHABLE.iter().any(|row| row.operation == helms),
            "`{helms}`"
        );
    }
    assert!(REACHABLE.iter().any(|row| row.operation == "get_studio"));
}
