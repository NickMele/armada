//! Helm's brief, pinned whole, and the line `#73` drew around what Helm may do.
//!
//! **The snapshots pin the copy on purpose**, unlike `tests::briefing`: `#940`'s
//! claim is the assembled text. An edit to section 5a of the Agent Prompt
//! Contract lands here in the same change.

use std::path::Path;

use config::Manifest;
use ipc::door::{DRAFTING, REACHABLE};

use super::reach::ACTS;
use super::{brief, may, Authority, Voice};

/// Acts the door offers that `#73` left with the person.
const THE_PERSONS: &[&str] = &["undo_run"];

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
and you answer from what the Fleet tools you have been given return. Saying \
that you did something does not do it. Only a tool call does.

THIS REPOSITORY

Every question in this conversation is about Manifest armada, read from \
/work/armada. The Fleet tools answer inside it and reach nothing outside it, so \
when you are asked about another repository, say it cannot be answered here.

EACH TURN

Start every turn by calling get_events_since with the upto cursor your last \
call answered with, or 0 on your first turn. It answers with a count and one \
line per kind of event since that cursor. Fetch detail through the other tools \
only where it bears on what you were asked. Everything a tool returns stays in \
this conversation for the rest of it.

WHAT YOU MAY DO

You may call every tool that reads. Of the tools that act, you may call these \
and no others:

  examine_job
  raise_cost_cap
  raise_turn_cap
  redirect_drone
  show_again
  start_run
  stop_run
  start_server
  stop_server
  propose_job
  propose_from_request

Any other act is the person's, including a tool you have been given that is \
not listed here. Where one would help, say which and why, and leave it to \
them. How far you may raise a cap is bounded, and a raise past the bound is \
refused, naming the most you may ask for.

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

#[test]
fn the_brief_for_an_acting_helm_reads_as_the_contract_draws_it() {
    let voice = Voice::said("Terse.");
    let brief = brief(&a_manifest(), Authority::Acting, voice.as_ref());
    assert_eq!(brief.as_str(), ACTING_IN_A_TERSE_VOICE);
}

/// Read-only changes one block, and no Voice renders no Voice block.
#[test]
fn a_read_only_helm_with_no_voice_is_told_it_only_reads() {
    let acting = brief(&a_manifest(), Authority::Acting, None);
    let reading = brief(&a_manifest(), Authority::ReadOnly, None);
    let (before, _) = acting
        .as_str()
        .split_once("WHAT YOU MAY DO")
        .expect("the acting brief has the block");
    let (_, after) = acting
        .as_str()
        .split_once("\n\nHOW YOU ANSWER")
        .expect("and the block after it");
    assert_eq!(
        reading.as_str(),
        format!("{before}{WHAT_A_READ_ONLY_HELM_MAY_DO}\n\nHOW YOU ANSWER{after}")
    );
    assert!(!reading.as_str().contains("VOICE"));
}

/// Never told anything of the Manifest past its id and folder.
#[test]
fn the_brief_does_not_quote_the_manifest() {
    let brief = brief(&a_manifest(), Authority::Acting, None);
    assert!(!brief.as_str().contains("publish-everything"));
    assert!(!brief.as_str().contains("release"));
}

/// A command marked `Yes` later fails here until somebody decides whose it is.
#[test]
fn every_act_the_door_offers_is_decided_for_helm_or_the_person() {
    for row in REACHABLE.iter().filter(|row| row.kind == "command") {
        assert!(
            ACTS.contains(&row.operation) || THE_PERSONS.contains(&row.operation),
            "`{}` reads `Yes` and nobody has said whether it is Helm's",
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

/// Every act names a row agents may reach at all, `Yes` or `Drafts only`.
#[test]
fn every_act_is_an_operation_an_agent_may_reach() {
    let inventory = include_str!("../../../ipc/operations.toml");
    for act in ACTS {
        let (_, row) = inventory
            .split_once(&format!("\n[operations.{act}]\n"))
            .unwrap_or_else(|| panic!("`{act}` is not in the inventory"));
        let row = row.split("\n[operations.").next().unwrap_or(row);
        assert!(
            row.contains("agent_access = \"Yes\"")
                || row.contains("agent_access = \"Drafts only\""),
            "`{act}` is one no agent may reach"
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

#[test]
fn a_read_only_helm_may_read_and_nothing_else() {
    for row in REACHABLE {
        assert_eq!(may(Authority::ReadOnly, row), row.kind == "query");
        if row.kind == "query" {
            assert!(may(Authority::Acting, row), "`{}` is a read", row.operation);
        }
    }
}
