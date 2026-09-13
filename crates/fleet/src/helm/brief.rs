//! What a Helm session is told when it starts.
//!
//! The wording is `docs/contracts/agent-prompt.md` section 5a, transcribed. A
//! change to it belongs in the contract first, then here and in the snapshot.

use std::path::Path;

use config::Manifest;
use ipc::door::REACHABLE;

use super::reach::{may, Authority};

/// The Machine Voice setting as it reads. It tunes length and formality and
/// nothing else, and a blank one is no Voice, so no block is rendered empty.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Voice(String);

impl Voice {
    /// `None` for blank text.
    pub fn said(text: &str) -> Option<Voice> {
        let text = text.trim();
        (!text.is_empty()).then(|| Voice(text.to_string()))
    }
}

/// A Helm session's opening brief, whole.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Brief(String);

impl Brief {
    pub fn as_str(&self) -> &str {
        &self.0
    }
}

const OPENING: &str = "\
You are Helm, in Armada. A person asks you about the work in one repository, \
and you answer from what the Fleet tools you have been given return. Saying \
that you did something does not do it. Only a tool call does.";

const EACH_TURN: &str = "\
EACH TURN

Start every turn by calling get_events_since with the upto cursor your last \
call answered with, or 0 on your first turn. It answers with a count and one \
line per kind of event since that cursor. Fetch detail through the other tools \
only where it bears on what you were asked. Everything a tool returns stays in \
this conversation for the rest of it.";

const MAY_ACT: &str = "\
WHAT YOU MAY DO

You may call every tool that reads. Of the tools that act, you may call these \
and no others:
";

const LEFT_TO_THE_PERSON: &str = "\
Any other act is the person's, including a tool you have been given that is \
not listed here. Where one would help, say which and why, and leave it to \
them. How far you may raise a cap is bounded, and a raise past the bound is \
refused, naming the most you may ask for.";

const READ_ONLY: &str = "\
WHAT YOU MAY DO

You may call every tool that reads, and no tool that acts, including any you \
have been given. This machine is set so that Helm only reads. Where an act \
would help, say which and why, and leave it to the person.";

const HOW_YOU_ANSWER: &str = "\
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
covering the error path\". A cause you are supposing, say you are supposing it.";

/// Assemble the brief for one repository's Helm session.
///
/// **The Manifest is named, not quoted.** Everything in it past its id and
/// folder is `get_manifest`'s to answer, and a copy here goes stale over a long
/// conversation. Voice is last, so it adjusts what is above it.
pub fn brief(manifest: &Manifest, authority: Authority, voice: Option<&Voice>) -> Brief {
    let mut blocks = vec![
        OPENING.to_string(),
        this_repository(manifest),
        EACH_TURN.to_string(),
        what_you_may_do(authority),
        HOW_YOU_ANSWER.to_string(),
    ];
    if let Some(Voice(voice)) = voice {
        blocks.push(format!(
            "VOICE\n\n{voice}\n\nThis sets how long and how formal your answers are. It \
             changes nothing else in this brief."
        ));
    }
    Brief(blocks.join("\n\n"))
}

fn this_repository(manifest: &Manifest) -> String {
    let file = manifest.path();
    let folder = file
        .parent()
        .filter(|folder| folder != &Path::new(""))
        .unwrap_or(file);
    format!(
        "THIS REPOSITORY\n\nEvery question in this conversation is about Manifest {}, read \
         from {}. The Fleet tools answer inside it and reach nothing outside it, so when you \
         are asked about another repository, say it cannot be answered here.",
        manifest.id().as_str(),
        folder.display()
    )
}

/// Listed from [`may`] over what the door offers, so the brief and the door's
/// refusal cannot name different acts.
fn what_you_may_do(authority: Authority) -> String {
    match authority {
        Authority::ReadOnly => READ_ONLY.to_string(),
        Authority::Acting => {
            let mut block = MAY_ACT.to_string();
            for row in REACHABLE
                .iter()
                .filter(|row| row.kind == "command" && may(authority, row))
            {
                block.push_str("\n  ");
                block.push_str(row.operation);
            }
            block.push_str("\n\n");
            block.push_str(LEFT_TO_THE_PERSON);
            block
        }
    }
}
