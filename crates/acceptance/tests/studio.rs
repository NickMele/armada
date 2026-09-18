//! Studio's claim: **I can work something out on a Studio — notes from using
//! the app, what a scout read, what I pasted in — turn what holds up into a
//! Job, and come back later to see how I got there.** The apparatus is
//! [`bench::studio`].
//!
//! **Every step of the claim is carried.** A change that would add one adds its
//! assertion to this file, made against a value that has been through
//! [`ipc::encode`] and back as `board.rs`'s are.
//!
//! | Not proved here | Why not |
//! |---|---|
//! | That a proposer reading the draft chooses well | Choosing is a model's, and this file calls none. The answer below is written by the test |
//! | That the proposed Job is created at `awaiting_approval` | `fleet::drafting`'s conversion from a proposal to a Job is `pub(crate)`, so building one here would assert what the test built. `fleet`'s own tests create one |
//! | That a Studio survives a restart on disk | It touches a file. `store` and `fleet` reopen one in their own tests; what is asserted here is the record reading back through the wire |
//! | That a sweep past retention is what fills a Run node in | It deletes a directory. `fleet`'s `studio_runs` drives a real run past a real sweep; what is asserted here is the tail that sweep takes and the node carrying it over the wire |
//! | That a reopened Studio is read-only until Continue | A Bridge state; #1287's mock browser test proves it |
//! | That a Link naming an issue, a pull request or a milestone dispatches its address, and that Dispatch is offered on those and no other Link | The route is `fleet`'s `promoting`, which drives a real dispatch to a Job at the gate for each; what Bridge offers off `forge` is #1379's mock browser test. What is asserted here is the field a Link carries on the wire |
//! | That the proposer picks the right workflow for each of the three | A model's, and this file calls none. `crates/config/tests/shipped.rs` holds the `for_requests` line each has to match on, and `fleet`'s `tests::proposing` measures the request and the catalogue the call is handed |
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
    a_capture_sent, a_failed_run, a_long_log, a_studio_promoted_to_a_job,
    a_studio_with_a_captured_note, a_studio_with_a_frozen_finding,
    a_studio_with_a_run_started_from_a_note, a_studio_with_sources_read_in,
    a_studio_with_two_notes, an_issue_draft, held, helms_manifest, one_job_under, received_event,
    received_request, received_studio, ASKED, COMMIT, COMPONENT, COST, CUT, DRAFT_TITLE,
    FIRST_NOTE, FRAME_BYTES, FRAME_FILE, LEFT_AT, MARKUP, OWNERS, READ, READ_IN_NOTE, REPOSITORY,
    SCREEN, SECOND_NOTE, SELECTOR, SIDES, SOURCES, STYLES, THE_COMMAND, THE_FAILURE, THE_RUN,
};

/// Step 6's near half: **two Notes are clustered, the Cluster is written up as
/// an Issue draft, and the Job it was dispatched to stands on the Studio,
/// linked back to both Notes through `Produced` edges.**
/// `docs/concepts/studio.md`, *Promotion*.
///
/// **The failure this is against is work that arrives with no way back.** A
/// Job on the Board says nothing about what was worked out to reach it, so
/// what is held here is the chain: every rung draws the edge that says where
/// its node came from, and walking those edges from the Job reaches the words
/// a person captured. A rung that added a node and no edge would render the
/// same and record nothing.
#[test]
fn two_notes_clustered_and_written_up_reach_a_job_node_that_walks_back_to_both() {
    let studio = received_studio(&a_studio_promoted_to_a_job());
    let kind = |node: &ipc::StudioNode| match &node.content {
        StudioNodeContent::Note { .. } => "note",
        StudioNodeContent::Cluster { .. } => "cluster",
        StudioNodeContent::IssueDraft { .. } => "issue_draft",
        StudioNodeContent::Job { .. } => "job",
        other => panic!("nothing else was promoted onto it: {other:?}"),
    };
    let kinds: Vec<_> = studio.nodes.iter().map(kind).collect();
    assert_eq!(
        kinds,
        ["note", "note", "cluster", "issue_draft", "job"],
        "each rung's node, oldest first"
    );

    // Every edge a rung drew is the Studio's own, accepted as drawn: a person
    // accepting a relation is a different act, and none of these waits on one.
    let produced: Vec<_> = studio
        .edges
        .iter()
        .filter(|edge| edge.kind.as_wire() == "produced")
        .collect();
    assert!(
        produced
            .iter()
            .all(|edge| edge.standing.as_wire() == "accepted"),
        "the Studio draws a Produced edge and nobody accepts one"
    );

    // Walk back from the Job: the draft, the Cluster, then both Notes.
    let made = |to: &ipc::StudioNodeId| -> Vec<ipc::StudioNodeId> {
        produced
            .iter()
            .filter(|edge| &edge.to == to)
            .map(|edge| edge.from.clone())
            .collect()
    };
    let job = studio.nodes.last().expect("the Job node");
    let [draft] = &made(&job.id)[..] else {
        panic!("one Issue draft made the Job");
    };
    let [cluster] = &made(draft)[..] else {
        panic!("one Cluster was written up as the draft");
    };
    let said: Vec<_> = made(cluster)
        .iter()
        .map(
            |note| match &studio.nodes.iter().find(|one| &one.id == note) {
                Some(ipc::StudioNode {
                    content: StudioNodeContent::Note { said, .. },
                    ..
                }) => said.clone(),
                other => panic!("a Cluster is of Notes: {other:?}"),
            },
        )
        .collect();
    assert_eq!(
        said,
        [FIRST_NOTE, SECOND_NOTE],
        "the Job walks back to both Notes' own words, in the order they were clustered"
    );

    // A Job node holds a reference and no status: its state is the Board's.
    assert!(
        job.state.is_none(),
        "a Job node's status is read off the Job, never copied here"
    );
    let StudioNodeContent::IssueDraft { title, body } = &studio.nodes[3].content else {
        panic!("the draft");
    };
    assert_eq!(title, DRAFT_TITLE);
    for note in [FIRST_NOTE, SECOND_NOTE] {
        assert!(
            body.contains(note),
            "the draft was written up from the Notes, and says so: {note}"
        );
    }
}

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
    assert_eq!(
        studio,
        ipc::Studio::of(&graph, &adapters::forge_named),
        "nothing lost on the wire"
    );
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
    assert_eq!(
        studio,
        ipc::Studio::of(&graph, &adapters::forge_named),
        "nothing lost on the wire"
    );

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
            "group_studio_nodes",
            "move_studio_node",
            "remove_studio_node",
            "write_up_studio_node",
        ],
        "a node is added, captured, grouped into a new one, moved, removed or written up into a \
         new one — and not one of these writes over a node already there"
    );

    // **Every route that does write a node's content names the kind it may
    // reach, and a Note is never one of them** (`#1291`, `#1378`). What holds
    // them to it is `core_model`'s own transitions: `StudioNode::edited` takes
    // an Issue draft, `settled` a Contradiction and `relabelled` a Link, and
    // the store's rewrite takes what only those make.
    // `fleet::tests::promoting` drives each against a Note and reads back
    // `fleet.studio_not_a_draft`, `fleet.studio_not_a_contradiction` and
    // `fleet.studio_not_a_link`.
    for rewrites in [
        "edit_studio_draft",
        "edit_studio_link",
        "settle_contradiction",
    ] {
        let route = api::SERVED
            .iter()
            .find(|route| route.operation == rewrites)
            .unwrap_or_else(|| panic!("`{rewrites}` is served"));
        assert!(
            !on_a_node(&route),
            "`{rewrites}` writes a node's content, so it is not one of the routes above — and \
             the list above stays the set that only ever adds, moves or removes"
        );
    }
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
        // #1291: grouping is accepting, deferring is a person's only, editing
        // a draft is what a person read and changed, and ending a
        // Contradiction is their judgement.
        "group_studio_nodes",
        "defer_on_studio",
        "edit_studio_draft",
        "settle_contradiction",
        // #1378: a Link's line is the person's own words, and Helm rewriting
        // them is the act `studio.md` guards against.
        "edit_studio_link",
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
    for asked in [
        "start_scout",
        "start_studio_run",
        "write_up_studio_node",
        "dispatch_studio_draft",
    ] {
        assert!(
            on_an_ask.contains(asked),
            "`{asked}` waits for a person's ask: {studio}"
        );
    }
    for helms in [
        "start_studio_run",
        "write_up_studio_node",
        "dispatch_studio_draft",
    ] {
        assert!(
            HELM_ONLY
                .iter()
                .any(|row| row.operation == helms && row.kind == "command")
                && !REACHABLE.iter().any(|row| row.operation == helms),
            "`{helms}` is offered to Helm alone"
        );
    }
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

    // #1291: **every** act Helm takes on a Studio is its own event, not only
    // the unasked ones — a write-up and a dispatch it was asked for included.
    for asked in [
        HelmStudioAct::WroteUp {
            from: ipc::StudioNodeId::carried("01CLUSTER"),
            node_id: ipc::StudioNodeId::carried("01DRAFT"),
        },
        HelmStudioAct::Dispatched {
            from: ipc::StudioNodeId::carried("01DRAFT"),
            node_ids: vec![ipc::StudioNodeId::carried("01JOBNODE")],
        },
    ] {
        let acted = ipc::Event::StudioHelmActed(ipc::StudioHelmActed {
            studio_id: ipc::StudioId::carried("01STUDIO"),
            manifest_id: ipc::ManifestId::carried(REPOSITORY),
            act: asked,
            at: ipc::Instant::carried("2026-09-17T09:00:00.000Z"),
        });
        assert_eq!(received_event(&acted), acted, "nothing lost on the wire");
    }
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
    assert_eq!(
        studio,
        ipc::Studio::of(&graph, &adapters::forge_named),
        "nothing lost on the wire"
    );

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

/// Steps 4 and 5 of the claim: **a source outside the checkout is read in, the
/// Link stays standing with its address, the Finding says what it was handed
/// and what was cut, and a Contradiction appears with both sides on it.**
/// `docs/concepts/studio.md`, *Promotion*; `docs/concepts/scout.md`.
///
/// **The failure this is against is a decision that has to be retyped.** A
/// Studio that could hold only an address would send a person back to read
/// their own issue again. So what is held is that every node a read-in made
/// walks back to the Link — one that drew no edge would render the same and
/// record nothing about where any of it came from.
///
/// **A Contradiction's two sides are text**, because one side is outside the
/// repository and has no node to point at. Nothing else can mint one, which
/// is what makes `#1291`'s four outcomes reachable.
#[test]
fn four_sources_read_in_leave_their_links_standing_with_what_came_back_hung_off_them() {
    let studio = received_studio(&a_studio_with_sources_read_in());
    let node = |id: &str| {
        studio
            .nodes
            .iter()
            .find(|node| node.id.as_str() == id)
            .unwrap_or_else(|| panic!("`{id}` is on the Studio"))
    };

    // Step 4: each of the four first sources, and the Finding that says which
    // it was handed. The checkout is recorded beside it, so a Finding says
    // both what state of the code it read and what came from outside it.
    for (n, (link, address, kind)) in SOURCES.iter().enumerate() {
        let StudioNodeContent::Link {
            address: kept,
            said,
            named,
            forge,
        } = &node(link).content
        else {
            panic!("`{link}` is a Link");
        };
        assert_eq!(kept, address, "a Link keeps its address whatever came back");
        assert_eq!(*said, None, "nobody wrote a line on these");
        assert_eq!(
            *named, None,
            "nothing a scout read renames the Link it read"
        );
        // `#1379`: what an address names on this repository's forge is read
        // off it by Fleet and said here, so no surface reads an address. None
        // of the four is on it — a page, a session and a thread never are, and
        // the issue here is somebody else's forge — so each says nothing, and
        // a Studio drawing these offers Dispatch on none of them.
        assert_eq!(
            *forge, None,
            "`{address}` names nothing on this repository's forge"
        );

        let StudioNodeContent::Finding {
            sources,
            checkout,
            ended,
            ..
        } = &node(&format!("01FINDINGREADIN{n}")).content
        else {
            panic!("a Finding for `{address}`");
        };
        let [source] = &sources[..] else {
            panic!("one source per read-in");
        };
        assert_eq!(source.address, *address);
        assert_eq!(source.kind.as_wire(), kind.as_wire());
        assert_eq!(
            source.cut,
            match kind.as_wire() {
                // A page did not fit, and the Finding says how much went
                // rather than claiming the whole of it was read.
                "page" => CUT,
                _ => 0,
            }
        );
        assert_eq!(
            checkout
                .as_ref()
                .expect("the commit it read the code at")
                .commit,
            COMMIT
        );
        assert!(ended.is_some(), "a read-in freezes however it ends");
    }

    // Step 5: what came back off the issue — a Note and a Contradiction, each
    // carrying both sides, and a relation nobody has accepted.
    assert!(matches!(
        &node("01NOTEREADIN").content,
        StudioNodeContent::Note { said, .. } if said == READ_IN_NOTE
    ));
    let StudioNodeContent::Contradiction {
        first,
        second,
        answer,
    } = &node("01CONTRAREADIN").content
    else {
        panic!("a Contradiction");
    };
    assert_eq!((first.as_str(), second.as_str()), SIDES);
    assert_eq!(*answer, None, "nobody has settled it");
    assert_eq!(
        node("01CONTRAREADIN").state.map(|state| state.as_wire()),
        Some("reported"),
        "Reported until a person picks one of the four outcomes"
    );

    // Everything the issue's read-in made walks back to its Link.
    let off_the_issue: Vec<_> = studio
        .edges
        .iter()
        .filter(|edge| edge.kind.as_wire() == "produced" && edge.from.as_str() == SOURCES[0].0)
        .map(|edge| edge.to.as_str().to_string())
        .collect();
    assert_eq!(
        off_the_issue,
        ["01FINDINGREADIN0", "01NOTEREADIN", "01CONTRAREADIN"],
        "the Finding, the Note and the Contradiction, oldest first"
    );

    // A relation a scout asked for waits on a person, drawn dashed until then.
    let proposed: Vec<_> = studio
        .edges
        .iter()
        .filter(|edge| {
            edge.standing.as_wire() == "proposed" && edge.from.as_str() == "01NOTEREADIN"
        })
        .collect();
    assert_eq!(proposed.len(), 1);
    assert_eq!(proposed[0].kind.as_wire(), "blocks");
}
