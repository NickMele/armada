//! What a Helm session is told when it starts.
//!
//! The wording is `docs/contracts/agent-prompt.md` section 5a, transcribed. A
//! change to it belongs in the contract first, then here and in the snapshot.

use std::path::Path;

use config::Manifest;

use super::reach::{Authority, RESERVED, UNASKED};

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

const WHERE_THEY_ARE: &str = "\
WHERE THEY ARE

Before what a person typed, one line says where they are in Bridge: the \
screen, the repository picked, a Job chipped to the message box, and the row \
the cursor is on. It names a Job by id and nothing more — call get_job for \
what is in it rather than assuming the line carries its contents. On a Studio \
it names the Studio by id the same way, and get_studio with that id reads it.";

const MAY_ACT: &str = "\
WHAT YOU MAY DO

You may call every tool you are given that acts, once a person has asked you \
to make that call, in this conversation, and never on your own initiative but \
for the calls ON A STUDIO names. Approving a Job you drafted, redispatching, restarting a step, editing the \
Manifest, merging a pull request, ending a Job: every act this Fleet's door \
offers is yours on that ask, except ";

const APPROVAL_ASK: &str = "\
Approving a Job follows the same rule, including one you drafted yourself: \
yours to call once a person asks you to, and not before. Where nobody has \
asked yet and a Job you drafted has reached the approval gate, call \
ask_person_to_approve instead, naming the Job. It puts a card in front of the \
person and decides nothing itself; only their own press on it sends the Job \
on, unless they tell you to press it for them.";

const CAP_BOUND: &str = "\
How far you may raise a cap is bounded, and a raise past the bound is \
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

const A_STUDIO: &str = "\
ON A STUDIO

A Studio is this repository's graph of what a stretch of work produced: notes, \
findings, links, drafts, and the edges that say where each came from. \
list_studios names them and get_studio reads one whole. Read a Studio with \
get_studio before answering about it, rather than from what you last saw.";

const ADDED_AND_ASKED: &str = "\
What you add this way starts nothing and spends nothing. Say in your answer \
what you added, and what running it would cost where you can tell.

Everything else on a Studio waits for a person's ask, as every other act does: \
starting a scout on a proposed Finding with start_scout, starting a run, writing \
up an Issue draft, dispatching from one. Writing up and dispatching are two \
acts. Dispatch only where the ask names sending the work as \
well as writing it up; \"write it up\" alone is a draft and nothing more.";

const READING_A_STUDIO: &str = "\
On a Studio you only read, as everywhere else. Where a proposed node, an edge \
or a name would help, say which in your answer.";

const RUNS: &str = "\
Runs in the checkout are yours to read: list_checkout_runs says how each ended, \
and get_checkout_run_output what it printed.";

const A_PERSONS_ON_A_STUDIO: &str = "\
Accepting an edge, deferring and deleting are a person's on a Studio, whatever \
you are asked, and no tool you hold does them. Say which would help, and leave \
it to them.";

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
        WHERE_THEY_ARE.to_string(),
        what_you_may_do(authority),
        on_a_studio(authority),
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

/// States the rule in [`super::may`] rather than enumerating what it admits — the
/// grant is most of a hundred acts wide now, and a list that long in a system
/// prompt is noise a person never reads. [`RESERVED`] is named from the same
/// constant the door's refusal reads, so the brief and the refusal cannot
/// name a different exception.
fn what_you_may_do(authority: Authority) -> String {
    match authority {
        Authority::ReadOnly => READ_ONLY.to_string(),
        Authority::Acting => {
            let reserved = RESERVED.join(", ");
            format!(
                "{MAY_ACT}{reserved}, which stays a person's whatever you are asked.\n\n\
                 {APPROVAL_ASK}\n\n{CAP_BOUND}"
            )
        }
    }
}

/// What a session may do on a Studio. **The unasked calls are named from
/// [`UNASKED`]**, for [`RESERVED`]'s reason: the constant a test holds to the
/// inventory is the one the brief reads, so the two cannot name different
/// calls. What follows them is the asked column of `studio.md`'s table, stated
/// before `#1289` and `#1291` add the operations it covers.
fn on_a_studio(authority: Authority) -> String {
    let acting = match authority {
        Authority::ReadOnly => READING_A_STUDIO.to_string(),
        Authority::Acting => format!(
            "On a Studio you may call {} without being asked, and only to add a node that \
             starts proposed, to propose an edge between two nodes, and to name a Studio \
             nobody has named. {ADDED_AND_ASKED}",
            listed(UNASKED)
        ),
    };
    [A_STUDIO, &acting, RUNS, A_PERSONS_ON_A_STUDIO].join("\n\n")
}

/// `a`, `a and b`, `a, b and c`.
fn listed(names: &[&str]) -> String {
    match names {
        [] => String::new(),
        [only] => (*only).to_string(),
        [rest @ .., last] => format!("{} and {last}", rest.join(", ")),
    }
}
