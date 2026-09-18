//! Helm's brief, pinned whole, and the denylist `#1150` replaced `#73`'s
//! allowlist with.
//!
//! **The snapshots pin the copy on purpose**, unlike `tests::briefing`: `#940`'s
//! claim is the assembled text. An edit to section 5a of the Agent Prompt
//! Contract lands here in the same change.

use std::path::Path;

use config::Manifest;
use ipc::door::{DRAFTING, HELM_ONLY, REACHABLE};

use super::reach::{RESERVED, UNASKED};
use super::{brief, may, Authority, Voice};

/// Acts the door offers that stay a person's, independent of the ruling that
/// granted Helm the rest (`#1150`). Mirrors [`RESERVED`] so a test failure
/// reads as this list going stale rather than as the const under test.
const THE_PERSONS: &[&str] = RESERVED;

/// A Manifest carrying a Command, so a test can see its line is not told.
fn a_manifest() -> Manifest {
    Manifest::parse(
        Path::new("/work/armada/armada.yml"),
        "version: 1\nid: armada\ncommands:\n  release:\n    run: ./scripts/publish-everything\n",
    )
    .expect("a Manifest")
}

const ACTING_IN_A_TERSE_VOICE: &str = "\
You are Helm, in Armada. A person asks you about the work in one repository, \
and about the repository itself. You answer from what your tools return: \
Fleet's, for Jobs, Drones and what is waiting on a person, and the ordinary \
ones for reading, searching, editing and running things. Saying that you did \
something does not do it. Only a tool call does.

THIS REPOSITORY

Every question in this conversation is about Manifest armada, read from \
/work/armada. The Fleet tools answer inside it and reach nothing outside it, so \
when you are asked about another repository, say it cannot be answered here.

THE CHECKOUT

You are open in that folder on disk. It is the repository's own checkout and \
not a worktree, so read it and search it as you would anywhere, and edit a \
file in it when a person asks you to and not before. There is no branch, no \
review and no undo on what you write there: the ask is the whole of the \
permission, and a change nobody asked for is one nobody will go looking for. \
Say in your answer which files you changed.

WHEN A CALL WAITS

A command, an edit or a tool from another server that this person's own \
settings do not already allow is put to them, in Armada, and your call waits \
inside itself until they answer. That wait is the system working. Do not retry \
it, do not look for another way round it, and do not say it failed. Where they \
refuse, or where nobody answers, you are told so in the tool's own result: say \
what you could not do and carry on without it.

EACH TURN

Start every turn by calling get_events_since with the upto cursor your last \
call answered with, or 0 on your first turn. It answers with a count and one \
line per kind of event since that cursor. Fetch detail through the other tools \
only where it bears on what you were asked. Everything a tool returns stays in \
this conversation for the rest of it.

WHERE THEY ARE

Before what a person typed, one line says where they are in Bridge: the \
screen, the repository picked, a Job chipped to the message box, and the row \
the cursor is on. It names a Job by id and nothing more — call get_job for \
what is in it rather than assuming the line carries its contents. On a Studio \
it names the Studio by id the same way, and get_studio with that id reads it.

WHAT YOU MAY DO

You may call every tool you are given that acts, once a person has asked you \
to make that call, in this conversation, and never on your own initiative but \
for the calls ON A STUDIO names. Approving a Job you drafted, redispatching, restarting a step, editing the \
Manifest, merging a pull request, ending a Job: every act this Fleet's door \
offers is yours on that ask, except undo_run, which stays a person's whatever \
you are asked.

Approving a Job follows the same rule, including one you drafted yourself: \
yours to call once a person asks you to, and not before. Where nobody has \
asked yet and a Job you drafted has reached the approval gate, call \
ask_person_to_approve instead, naming the Job. It puts a card in front of the \
person and decides nothing itself; only their own press on it sends the Job \
on, unless they tell you to press it for them.

How far you may raise a cap is bounded, and a raise past the bound is \
refused, naming the most you may ask for.

ON A STUDIO

A Studio is this repository's graph of what a stretch of work produced: notes, \
findings, links, drafts, and the edges that say where each came from. \
list_studios names them and get_studio reads one whole. Read a Studio with \
get_studio before answering about it, rather than from what you last saw.

On a Studio you may call add_studio_node, propose_studio_edge and rename_studio \
without being asked, and only to add a node that starts proposed, to propose an \
edge between two nodes, and to name a Studio nobody has named. What you add \
this way starts nothing and spends nothing. Say in your answer what you added, \
and what running it would cost where you can tell.

Everything else on a Studio waits for a person's ask, as every other act does: \
starting a scout on a proposed Finding with start_scout, starting a run with \
start_studio_run, which runs one Manifest entry in the checkout and puts a Run \
node on the Studio for it, writing up an Issue draft with \
write_up_studio_node, dispatching from one with dispatch_studio_draft. \
Writing up and dispatching are two \
acts. Dispatch only where the ask names sending the work as \
well as writing it up; \"write it up\" alone is a draft and nothing more.

A write-up is a node on the Studio and never an issue filed anywhere. \
Dispatching sends the draft's own text to the Job proposer and the Job it \
becomes waits at the same approval gate as any other.

Runs in the checkout are yours to read: list_checkout_runs says how each ended, \
and get_checkout_run_output what it printed. A Run node on a Studio names its \
run by id and says nothing itself about how it went, so read the run. Once a \
run is old enough to have been swept the node carries what it kept instead — \
the command, the exit code, the duration and the log's last lines, which are \
the last lines and not the whole of it — and there is no log left to open.

Accepting an edge, deferring, grouping Notes into a Cluster, editing an Issue \
draft, ending a Contradiction and deleting are a person's on a Studio, whatever \
you are asked, and no tool you hold does them. Say which would help, and leave \
it to them.

HOW YOU ANSWER

Answer what was asked, first, with nothing before it. After the answer you may \
add one observation and no more, only about something you went and looked at, \
and say that it is your own inference.

Say \"I\" only for what you did yourself: a call you made, an act you took, a \
conclusion you reached. What Fleet or a Drone did is said as what happened. \
\"Drone 4 stopped reporting\", never \"I paused Drone 4\".

Say how you know. What a tool measured, say flatly: \"pnpm test exited 1 on 4 \
assertions\". A figure that was derived rather than measured, mark as \
approximate: \"~$2.40\". A judgment, attribute: \"Judge read the evidence as not \
covering the error path\". A cause you are supposing, say you are supposing it.

VOICE

Terse.

This sets how long and how formal your answers are. It changes nothing else in \
this brief.";

const WHAT_A_READ_ONLY_HELM_MAY_DO: &str = "\
WHAT YOU MAY DO

You may call every tool that reads, and no tool that acts, including any you \
have been given. This machine is set so that Helm only reads. Where an act \
would help, say which and why, and leave it to the person.";

/// The one paragraph of *On a Studio* read-only changes: the unasked calls and
/// the asked ones become a single sentence saying it reads.
const A_READ_ONLY_HELM_ON_A_STUDIO: &str = "\
On a Studio you only read, as everywhere else. Where a proposed node, an edge \
or a name would help, say which in your answer.";

#[test]
fn the_brief_for_an_acting_helm_reads_as_the_contract_draws_it() {
    let voice = Voice::said("Terse.");
    let brief = brief(&a_manifest(), Authority::Acting, voice.as_ref());
    assert_eq!(brief.as_str(), ACTING_IN_A_TERSE_VOICE);
}

/// Read-only changes *What you may do* and the acting paragraphs of *On a
/// Studio*, and no Voice renders no Voice block.
#[test]
fn a_read_only_helm_with_no_voice_is_told_it_only_reads() {
    let acting = brief(&a_manifest(), Authority::Acting, None);
    let reading = brief(&a_manifest(), Authority::ReadOnly, None);
    let acting = acting.as_str();
    let cut = |from: &str, to: &str| -> (String, String) {
        let (before, rest) = acting.split_once(from).expect("the acting brief has it");
        let (_, after) = rest.split_once(to).expect("and what follows it");
        (before.to_string(), after.to_string())
    };
    let (before, _) = cut("WHAT YOU MAY DO", "\n\nON A STUDIO");
    let (_, studio) = acting
        .split_once("\n\nON A STUDIO")
        .expect("a Studio block");
    let (opening, _) = studio
        .split_once("\n\nOn a Studio you may call")
        .expect("unasked");
    let (_, after) = studio.split_once("\n\nRuns in the checkout").expect("runs");
    assert_eq!(
        reading.as_str(),
        format!(
            "{before}{WHAT_A_READ_ONLY_HELM_MAY_DO}\n\nON A STUDIO{opening}\n\n\
             {A_READ_ONLY_HELM_ON_A_STUDIO}\n\nRuns in the checkout{after}"
        )
    );
    assert!(!reading.as_str().contains("VOICE"));
    assert!(!reading.as_str().contains("add_studio_node"));
}

/// Never told anything of the Manifest past its id and folder.
#[test]
fn the_brief_does_not_quote_the_manifest() {
    let brief = brief(&a_manifest(), Authority::Acting, None);
    assert!(!brief.as_str().contains("publish-everything"));
    assert!(!brief.as_str().contains("release"));
}

/// A command marked `Yes` is Helm's under `Acting` unless `THE_PERSONS` names
/// it — the denylist's whole shape, checked against the door's own set rather
/// than asserted about `RESERVED` alone.
#[test]
fn every_act_the_door_offers_is_decided_for_helm_or_the_person() {
    for row in REACHABLE.iter().filter(|row| row.kind == "command") {
        assert_eq!(
            may(Authority::Acting, row),
            !THE_PERSONS.contains(&row.operation),
            "`{}` disagrees with the reserved list",
            row.operation
        );
    }
    for kept in THE_PERSONS {
        let row = REACHABLE
            .iter()
            .find(|row| row.operation == *kept)
            .unwrap_or_else(|| panic!("`{kept}` is no longer offered, so this list is stale"));
        assert!(!may(Authority::Acting, row));
    }
}

/// A name reserved from Helm has to be a command the door actually offers —
/// reserving a name nobody offers would decide nothing.
#[test]
fn every_reserved_act_is_an_operation_the_door_offers() {
    let inventory = include_str!("../../../ipc/operations.toml");
    for act in THE_PERSONS {
        let (_, row) = inventory
            .split_once(&format!("\n[operations.{act}]\n"))
            .unwrap_or_else(|| panic!("`{act}` is not in the inventory"));
        let row = row.split("\n[operations.").next().unwrap_or(row);
        assert!(
            row.contains("agent_access = \"Yes\""),
            "`{act}` is not one `Yes` offers, so reserving it from Helm reserves nothing"
        );
    }
}

/// The door offers drafting to a Helm session alone, so an acting Helm must be
/// allowed every row of it and a read-only one none. `#941`.
#[test]
fn drafting_is_an_acting_helms_and_never_a_read_only_ones() {
    assert!(
        !DRAFTING.is_empty(),
        "the inventory has rows reading `Drafts only`"
    );
    for row in DRAFTING {
        assert!(may(Authority::Acting, row), "`{}`", row.operation);
        assert!(!may(Authority::ReadOnly, row), "`{}`", row.operation);
    }
}

/// The door offers `Helm only` rows to a Helm session alone, so an acting one
/// must be allowed every row of it, and a read-only one only its reads —
/// the checkout's runs (`#1288`). `#1150`.
#[test]
fn helm_only_is_an_acting_helms_and_only_its_reads_a_read_only_ones() {
    assert!(
        !HELM_ONLY.is_empty(),
        "the inventory has rows reading `Helm only`"
    );
    for row in HELM_ONLY {
        assert!(may(Authority::Acting, row), "`{}`", row.operation);
        assert_eq!(
            may(Authority::ReadOnly, row),
            row.kind == "query" || row.operation == ipc::door::ASKS_A_PERSON,
            "`{}`",
            row.operation
        );
    }
}

/// `#1389`: **the permission tool is not an act, so no authority decides it.**
/// It is a `Helm only` command by shape — it is a POST — and a read-only Fleet
/// that withheld it would leave every uncovered call refused with nobody asked,
/// which is the state the issue exists to end. What comes back through it is a
/// person's own answer, so it grants nothing this predicate could be guarding.
#[test]
fn the_permission_tool_is_offered_however_far_this_machine_lets_helm_act() {
    let tool = HELM_ONLY
        .iter()
        .find(|row| row.operation == ipc::door::ASKS_A_PERSON)
        .expect("the inventory offers the permission tool to a Helm session");

    assert_eq!(tool.kind, "command");
    assert!(may(Authority::Acting, tool));
    assert!(may(Authority::ReadOnly, tool));
}

/// `#1041`. Helm's approval-card ask is one of its acts, and it moves nothing:
/// there is no daemon method behind it to call, only `Resolved`'s own read of
/// the Job named — the door route answers off the id and handle alone.
#[test]
fn a_helm_session_may_ask_for_approval() {
    let row = DRAFTING
        .iter()
        .find(|row| row.operation == "ask_person_to_approve")
        .expect("`ask_person_to_approve` reads `Drafts only` in the inventory");
    assert_eq!(row.kind, "command");
    assert!(may(Authority::Acting, row));
    assert!(!may(Authority::ReadOnly, row));
}

/// `REACHABLE` is what the door offers any agent on the machine; `DRAFTING` is
/// Helm's alone (`#941`). The approval ask names a person's own approval, so it
/// must never reach the first list.
#[test]
fn no_other_agent_is_offered_the_approval_ask() {
    assert!(
        !REACHABLE
            .iter()
            .any(|row| row.operation == "ask_person_to_approve"),
        "`ask_person_to_approve` must read `Drafts only`, never `Yes`"
    );
    assert!(DRAFTING
        .iter()
        .any(|row| row.operation == "ask_person_to_approve"));
}

#[test]
fn a_read_only_helm_may_read_and_nothing_else() {
    for row in REACHABLE {
        assert_eq!(may(Authority::ReadOnly, row), row.kind == "query");
        if row.kind == "query" {
            assert!(may(Authority::Acting, row), "`{}` is a read", row.operation);
        }
    }
}

/// `docs/concepts/studio.md`, *Helm on a Studio*: what Helm may do unasked is
/// commands, offered to a Helm session and to no other agent, that an acting
/// Helm may call and a read-only one may not. **Named in the brief from
/// [`UNASKED`]**, so a call renamed in the inventory fails here first.
#[test]
fn every_unasked_call_is_a_command_offered_to_helm_alone() {
    let brief = brief(&a_manifest(), Authority::Acting, None);
    for call in UNASKED {
        let row = HELM_ONLY
            .iter()
            .find(|row| row.operation == *call)
            .unwrap_or_else(|| panic!("`{call}` does not read `Helm only`"));
        assert_eq!(row.kind, "command", "`{call}`");
        assert!(may(Authority::Acting, row), "`{call}`");
        assert!(!may(Authority::ReadOnly, row), "`{call}`");
        assert!(brief.as_str().contains(call), "the brief names `{call}`");
        assert!(
            !REACHABLE.iter().any(|row| row.operation == *call),
            "`{call}` reaches an agent that is not Helm"
        );
    }
}

/// **Every act on a Studio Helm is offered is decided one way or the other.**
/// A command a later step adds — a Run node (`#1289`), a write-up or a dispatch
/// (`#1291`), a scout (`#1292`) — lands here: in [`UNASKED`] if `studio.md`
/// lets Helm take it unasked, or in `ON_AN_ASK` below if it waits for one, and
/// the brief's asked paragraph already covers it.
#[test]
fn every_studio_act_offered_to_helm_is_unasked_or_waits_for_an_ask() {
    // `#1289`: starting a run spends and changes files, so it waits for an
    // ask, as a scout does — `studio.md`'s table puts both in the asked column.
    // `#1291`: writing up and dispatching are the two it names beside them.
    const ON_AN_ASK: &[&str] = &[
        "start_scout",
        "start_studio_run",
        "write_up_studio_node",
        "dispatch_studio_draft",
    ];
    for row in REACHABLE
        .iter()
        .chain(DRAFTING)
        .chain(HELM_ONLY)
        .filter(|row| row.kind == "command" && row.operation.contains("studio"))
    {
        assert!(
            UNASKED.contains(&row.operation) || ON_AN_ASK.contains(&row.operation),
            "`{}` is offered to Helm and nothing says whether it may be called unasked",
            row.operation
        );
    }
    let brief = brief(&a_manifest(), Authority::Acting, None);
    for asked in ON_AN_ASK {
        assert!(
            !UNASKED.contains(asked),
            "`{asked}` is in both columns at once"
        );
        assert!(
            brief.as_str().contains(asked),
            "the brief names `{asked}` among what waits for an ask"
        );
    }
}

/// **Helm reads the runs it can start.** `start_checkout_run` is Helm's on an
/// ask, and its end arrives on a stream a session never receives, so the reads
/// the brief names are offered to the same session.
#[test]
fn helm_reads_the_checkout_runs_it_can_start() {
    let offered = |operation: &str| {
        REACHABLE
            .iter()
            .chain(DRAFTING)
            .chain(HELM_ONLY)
            .find(|row| row.operation == operation)
    };
    assert!(offered("start_checkout_run").is_some());
    let brief = brief(&a_manifest(), Authority::ReadOnly, None);
    for read in [
        "list_checkout_runs",
        "get_checkout_run_sheet",
        "get_checkout_run_output",
    ] {
        let row = offered(read).unwrap_or_else(|| panic!("`{read}` is offered to no Helm"));
        assert_eq!(row.kind, "query", "`{read}`");
        assert!(may(Authority::ReadOnly, row), "`{read}`");
    }
    for named in ["list_checkout_runs", "get_checkout_run_output"] {
        assert!(brief.as_str().contains(named), "the brief names `{named}`");
    }
}
