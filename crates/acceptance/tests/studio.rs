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
//! | 4. A scout's Finding lists the sources it read beyond the checkout — an issue, a page, a session, a Helm thread | #1293. **The checkout half is asserted below** |
//! | 5. An issue from the repository's forge is read in, and a Contradiction appears | #1293 |
//! | 6. Two Notes are clustered, written up as an Issue draft, and dispatched, and a Job node stands at the gate | #1291. **The dispatch's far half is asserted below**: an Issue draft's text alone reaches the Job proposer, and the Job it proposes is told all of it |
//! | Helm starts a Run node, writes up an Issue draft, and dispatches from one, each only on a person's ask | #1289 and #1291. **Helm's rule for them is asserted below**, with what it proposes unasked and the runs it reads |
//!
//! | Not proved here | Why not |
//! |---|---|
//! | That a proposer reading the draft chooses well | Choosing is a model's, and this file calls none. The answer below is written by the test |
//! | That the proposed Job is created at `awaiting_approval` | `fleet::drafting`'s conversion from a proposal to a Job is `pub(crate)`, so building one here would assert what the test built. `fleet`'s own tests create one |
//! | That a Studio survives a restart on disk | It touches a file. `store` and `fleet` reopen one in their own tests; what is asserted here is the record reading back through the wire |
//! | That a sweep past retention is what fills a Run node in | It deletes a directory. `fleet`'s `studio_runs` drives a real run past a real sweep; what is asserted here is the tail that sweep takes and the node carrying it over the wire |
//! | That a reopened Studio is read-only until Continue | A Bridge state; #1287's mock browser test proves it |
//! | Anything a person sees | Nothing here renders. The whiteboard is #1286 and #1287 |
//! | That Helm keeps to what it is told | A model's. `fleet`'s `helm_studio` drives the door through a stand-in agent |
//! | That a scout reads only the checkout, holds no write tool, and shows its cost when stopped | A process's, and nothing here spawns one: `adapters`' tests hold the launch to read tools under `--restricted`, and `fleet`'s run and stop it against a stand-in agent |

// The bench is shared with the other milestones' tests and none of them uses
// all of it.
#[allow(dead_code)]
mod bench;

use fleet::{Brief, Proposal};
use ipc::door::{DRAFTING, HELM_ONLY, REACHABLE};
use ipc::{HelmStudioAct, StudioNodeContent};

use bench::studio::{
    a_capture_sent, a_failed_run, a_long_log, a_studio_with_a_captured_note,
    a_studio_with_a_frozen_finding, a_studio_with_a_run_started_from_a_note,
    a_studio_with_two_notes, an_issue_draft, held, helms_manifest, one_job_under, received_event,
    received_request, received_studio, ASKED, COMMIT, COMPONENT, COST, DRAFT_TITLE, FIRST_NOTE,
    FRAME_BYTES, FRAME_FILE, LEFT_AT, MARKUP, OWNERS, READ, REPOSITORY, SCREEN, SECOND_NOTE,
    SELECTOR, STYLES, THE_COMMAND, THE_FAILURE, THE_RUN,
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
            StudioNodeContent::Note { said, .. } => said.as_str(),
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
    // #1288: who put each thing there reads back off the record, so a client
    // that missed `studio.helm_acted` still tells Helm's proposal from a
    // person's Notes.
    let by = |author: Option<ipc::StudioAuthor>| author.map(|a| a.as_wire());
    assert_eq!(by(edge.added_by), Some("helm"), "Helm proposed the edge");
    assert!(studio
        .nodes
        .iter()
        .all(|node| by(node.added_by) == Some("person")));
    assert_eq!(by(studio.named_by), Some("person"));
}

/// Step 3: **a Note is captured with everything a person pointed at, and
/// nothing writes to it afterwards.** `docs/concepts/studio.md`, *Notes*.
///
/// **The failure this is against is a note that says what is wrong and not
/// where.** "The chip keeps its count" sends whoever reads it looking; the
/// component, the selector, the markup and the styles are what turn it back
/// into the element. So the capture crosses the wire and the Note is held to
/// it field by field, with the frame named as the file Fleet kept rather than
/// the one Bridge staged.
///
/// **Fixed at capture is proved on the seam**, which is where a rewrite would
/// have to arrive: of the routes on a Studio's nodes, one adds, one captures,
/// one moves and one removes, and none of them names a node and new content.
#[test]
fn a_captured_note_keeps_where_it_was_pointed_and_no_route_can_rewrite_it() {
    let graph = a_studio_with_a_captured_note();
    let studio = received_studio(&graph);
    assert_eq!(studio, ipc::Studio::of(&graph), "nothing lost on the wire");

    let note = studio.nodes.first().expect("the Note");
    assert_eq!(note.added_by.map(|by| by.as_wire()), Some("person"));
    assert!(note.state.is_none(), "a Note has no state");
    let StudioNodeContent::Note { said, capture } = &note.content else {
        panic!("a Note: {:?}", note.content);
    };
    assert_eq!(said, FIRST_NOTE, "what the person said, verbatim");
    let capture = capture.as_ref().expect("where they pointed");

    assert_eq!(capture.component.as_deref(), Some(COMPONENT));
    assert_eq!(capture.owners, OWNERS, "the chain above it, nearest first");
    assert_eq!(capture.selector, SELECTOR, "what finds the element again");
    assert_eq!(capture.element.text, "Queued 3", "what a person read");
    assert_eq!(capture.screen.as_deref(), Some(SCREEN));
    assert_eq!(capture.markup, MARKUP, "the markup, trimmed");
    for (property, value) in STYLES {
        assert_eq!(
            capture.styles.get(property).map(String::as_str),
            Some(value),
            "`{property}` is read off the element"
        );
    }
    assert!(
        capture.source.is_none(),
        "React 19 carries no `_debugSource`, so no path is invented for one"
    );

    let frame = capture.frame.as_ref().expect("the frame Fleet kept");
    assert_eq!(frame.filename, FRAME_FILE, "a file name, never a path");
    assert_eq!(frame.byte_size, FRAME_BYTES);
    assert_eq!(
        (frame.width, frame.height),
        (2880, 1800),
        "the image's own pixels, so a reader knows what it is looking at"
    );
    let staged = a_capture_sent().frame.expect("the bench stages one");
    assert!(
        !ipc::encode(&studio)
            .expect("a Studio that serialises")
            .contains(&staged.staged_path),
        "where Bridge staged the PNG reaches no client"
    );

    let on_a_node = |route: &&api::Route| {
        route.operation.contains("studio_node") || route.operation == "capture_studio_note"
    };
    let mut named: Vec<&str> = api::SERVED
        .iter()
        .filter(on_a_node)
        .map(|route| route.operation)
        .collect();
    named.sort_unstable();
    assert_eq!(
        named,
        [
            "add_studio_node",
            "capture_studio_note",
            "move_studio_node",
            "remove_studio_node"
        ],
        "a node is added, captured, moved or removed, and never written again"
    );
}

/// **Accepting a relation, removing a node and deleting a Studio are a
/// person's acts**, whatever Helm's authority says: no agent is offered one,
/// and Helm alone is offered what it may propose. **A scout starts only on a
/// person's ask**: typing one and pressing its stop are Bridge's, and Helm is
/// offered only the start of a Finding already proposed, once asked.
/// `docs/concepts/studio.md`, `docs/concepts/scout.md`.
#[test]
fn no_agent_is_offered_a_persons_act_on_a_studio() {
    let offered = |operation: &str| {
        [REACHABLE, DRAFTING, HELM_ONLY]
            .iter()
            .any(|door| door.iter().any(|row| row.operation == operation))
    };
    for persons in [
        "decide_studio_edge",
        "remove_studio_node",
        "delete_studio",
        "ask_scout",
        "stop_scout",
        // #1290: capture is a person pointing at Bridge, and no agent points.
        "capture_studio_note",
    ] {
        assert!(!offered(persons), "`{persons}` reaches an agent");
    }
    for helms in ["add_studio_node", "propose_studio_edge", "start_scout"] {
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

/// **Helm proposes on a Studio unasked, acts on one only when asked, and what
/// it did is its own event.** `docs/concepts/studio.md`, *Helm on a Studio*.
///
/// **The failure this is against is an agent reorganising a person's work.**
/// The door answers a call the same whether it was asked for, so the line is
/// the brief's: the unasked calls are named, writing up never dispatches on its
/// own, and each unasked call is offered to Helm alone. What Helm did then
/// reaches a client as `studio.helm_acted`, a kind no person's act publishes,
/// and a run it was asked to start is one it can read back.
#[test]
fn helm_proposes_unasked_acts_on_an_ask_and_its_acts_are_its_own_event() {
    let brief = fleet::helm::brief(&helms_manifest(), fleet::helm::Authority::Acting, None);
    let (_, studio) = brief
        .as_str()
        .split_once("ON A STUDIO")
        .expect("Helm is told about Studios");
    for unasked in ["add_studio_node", "propose_studio_edge", "rename_studio"] {
        assert!(studio.contains(unasked), "`{unasked}` is named as unasked");
        assert!(
            HELM_ONLY.iter().any(|row| row.operation == unasked)
                && !REACHABLE.iter().any(|row| row.operation == unasked),
            "`{unasked}` is offered to Helm alone"
        );
    }
    assert!(
        studio.contains("\"write it up\" alone is a draft and nothing more"),
        "writing up never dispatches on its own: {studio}"
    );
    // #1289 and #1292: starting a run and starting a scout both spend, so each
    // is named where the asked acts are named and nowhere above them.
    let (_, on_an_ask) = studio
        .split_once("waits for a person's ask")
        .expect("the asked acts are named");
    for asked in ["start_scout", "start_studio_run"] {
        assert!(
            on_an_ask.contains(asked),
            "`{asked}` waits for a person's ask: {studio}"
        );
    }
    assert!(
        HELM_ONLY
            .iter()
            .any(|row| row.operation == "start_studio_run" && row.kind == "command")
            && !REACHABLE
                .iter()
                .any(|row| row.operation == "start_studio_run"),
        "`start_studio_run` is offered to Helm alone"
    );
    for read in ["list_checkout_runs", "get_checkout_run_output"] {
        assert!(HELM_ONLY
            .iter()
            .any(|row| row.operation == read && row.kind == "query"));
        assert!(studio.contains(read), "Helm is told it reads `{read}`");
    }

    let acted = ipc::Event::StudioHelmActed(ipc::StudioHelmActed {
        studio_id: ipc::StudioId::carried("01STUDIO"),
        manifest_id: ipc::ManifestId::carried(REPOSITORY),
        act: HelmStudioAct::AddedNode {
            node_id: ipc::StudioNodeId::carried("01FINDING"),
        },
        at: ipc::Instant::carried("2026-09-17T09:00:00.000Z"),
    });
    let received = received_event(&acted);
    assert_eq!(received, acted, "nothing lost on the wire");
    assert_eq!(received.kind(), "studio.helm_acted");
    assert_eq!(
        received.about(),
        (None, Some(REPOSITORY.to_string())),
        "a poll's tally names the repository"
    );
}

/// Step 4, the checkout's half: **a scout's Finding arrives Frozen, listing
/// every file it read, the commit it read and that uncommitted changes were
/// there, and what it cost.** `docs/concepts/scout.md`.
///
/// **The failure this is against is a Finding that reads as an answer about
/// code nobody can find again.** A list that dropped a file, a commit dropped
/// on the way out, or a clean checkout reported where there were changes
/// would each send a person to read different code than the scout read.
#[test]
fn a_scouts_finding_arrives_frozen_with_every_file_it_read_its_commit_and_its_cost() {
    let graph = a_studio_with_a_frozen_finding();
    let studio = received_studio(&graph);
    assert_eq!(studio, ipc::Studio::of(&graph), "nothing lost on the wire");

    let finding = studio.nodes.last().expect("the Finding");
    assert_eq!(
        finding.state.map(|state| state.as_wire()),
        Some("frozen"),
        "done reading"
    );
    let StudioNodeContent::Finding {
        asked,
        checkout,
        read,
        learned,
        ended,
        ..
    } = &finding.content
    else {
        panic!("a Finding: {:?}", finding.content);
    };
    assert_eq!(asked, ASKED, "what the person asked, verbatim");
    assert_eq!(read, &READ, "every file it read, in order");
    let checkout = checkout.as_ref().expect("the checkout it read");
    assert_eq!(checkout.commit, COMMIT);
    assert!(checkout.uncommitted, "the change on top of it is said");
    assert!(
        learned.is_some(),
        "what it found is kept beside what it read"
    );
    let ended = ended.as_ref().expect("how it ended");
    assert_eq!(ended.outcome, ipc::ScoutOutcome::Answered);
    assert_eq!(ended.cost_micros, Some(COST), "its cost is shown");

    let asked_from = studio.edges.last().expect("the edge the ask drew");
    assert_eq!(asked_from.kind.as_wire(), "produced");
    assert_eq!(
        (&asked_from.from, &asked_from.to),
        (&studio.nodes[0].id, &finding.id),
        "the Note it was asked from made it"
    );
}

/// Step 2: **a Run started from a Studio is a node that reads its state off
/// the run, and keeps that run's result and its log's last lines once
/// retention sweeps it.** `docs/concepts/studio.md`, *Nodes*.
///
/// **The failure this is against is a Studio that outlives what it points
/// at.** A Studio is kept until a person deletes it and a run's log is not, so
/// a node holding only a reference is a dead end the day retention passes. The
/// node crosses the wire with a reference alone while the run is there; the
/// tail is then taken by the code the sweep calls, from the run's own record
/// and log; and the node carrying it crosses the wire with the command, the
/// exit code, the duration and the last lines a person would have opened it
/// for.
#[test]
fn a_run_node_reads_its_state_off_the_run_and_keeps_its_tail_and_result_once_it_is_swept() {
    let underway = received_studio(&a_studio_with_a_run_started_from_a_note(None));
    let node = &underway.nodes[1];
    assert!(
        node.state.is_none(),
        "a Run node copies no status: its state is the run's"
    );
    assert_eq!(
        node.content,
        StudioNodeContent::Run {
            run_id: THE_RUN.to_string(),
            kept: None,
        },
        "a reference, while the run is still there to read"
    );
    let edge = &underway.edges[0];
    assert_eq!(
        edge.kind.as_wire(),
        "produced",
        "from what it was started on"
    );
    assert_eq!(
        edge.standing.as_wire(),
        "accepted",
        "the Studio draws Produced itself, so nobody accepts it"
    );

    // Retention comes for the run, and what it said is taken before it goes.
    let record = a_failed_run();
    let log = a_long_log();
    let kept = fleet::studio_runs::kept(&record, Some(&log));
    assert_eq!(kept.command, THE_COMMAND, "what was run");
    assert_eq!(kept.exit_code, Some(1));
    assert_eq!(kept.expect_exit_code, 0, "so the node still reads failed");
    assert!(!kept.stopped, "it failed rather than being stopped");
    assert_eq!(kept.duration_ms, record.duration_ms, "how long it took");
    assert_eq!(
        kept.total_lines, log.total_lines,
        "how much there was, whether or not it is here"
    );

    assert!(
        kept.lines.len() < log.lines.len() && !kept.whole,
        "a Studio does not quietly hold a whole log: {} of {} lines",
        kept.lines.len(),
        log.lines.len()
    );
    assert_eq!(
        kept.lines,
        log.lines[log.lines.len() - kept.lines.len()..],
        "the tail, where a runner prints what failed, and not the head"
    );
    assert_eq!(kept.lines.last().map(String::as_str), Some(THE_FAILURE));

    let swept = received_studio(&a_studio_with_a_run_started_from_a_note(Some(kept.clone())));
    assert_eq!(
        swept.nodes[1].content,
        StudioNodeContent::Run {
            run_id: THE_RUN.to_string(),
            kept: Some(ipc::StudioRunKept::of(&kept)),
        },
        "the node carries all of it across the wire, and still says which run"
    );
    assert!(
        swept.nodes[1].state.is_none(),
        "a swept run is still read off what the node kept, never off a status"
    );
}
