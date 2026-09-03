//! What a line looks like on disk.
//!
//! Two shapes, because there are two files. A transcript row is one
//! `DroneEvent` and the instant Fleet saw it, and it is `ipc::TranscriptRow`
//! rather than a type here — the live view sends the same shape, and two
//! declarations would be two vocabularies. What stayed is the mapping: `ipc`
//! depends on `core-model` and nothing else, so the arm that turns a
//! `DroneEvent` into a row lives beside the loop that holds the event.
//!
//! A Job log line is `core_model::Envelope`, and [`Line`] is its wire shape.
//! The derives are here rather than on `Envelope` because `core-model` has no
//! dependencies at all, deliberately, so serde on it would be serde everywhere.

use std::collections::BTreeMap;

use adapter_traits::{DroneEvent, Speaker};
use core_model::{Envelope, FieldValue, StepId, Timestamp};
use ipc::{Instant, Saw, TranscriptRow};
use serde::Serialize;

/// One event, as the transcript holds it.
///
/// The `match` has **no `_` arm, on purpose.** A variant added to `DroneEvent`
/// fails to compile here, which is the whole of what stops a transcript quietly
/// losing a kind — cheaper than a gate rule and impossible to skip.
///
/// **The step is a parameter and never read from anywhere here.** Which step is
/// running is the slot's to say, and a row that asked would be a lookup on
/// every line.
pub(crate) fn seen(at: &Timestamp, step: &StepId, event: &DroneEvent) -> TranscriptRow {
    TranscriptRow {
        ts: Instant::carried(at.as_str()),
        step: Some(ipc::StepId::from(step)),
        by: voice_of(event),
        saw: match event {
            DroneEvent::Started {
                session,
                model,
                mcp_servers,
            } => Saw::Started {
                session: session.clone(),
                model: model.clone(),
                mcp_servers: *mcp_servers,
            },
            // **The file takes the whole argument and the socket does not.**
            // `ipc::Shown` is what drops it on the way out, so this arm writes
            // what the record should hold rather than what a viewer should see
            // — the two are different questions and only one of them is asked
            // here.
            DroneEvent::Called { tool, call, detail } => Saw::Called {
                tool: tool.clone(),
                call: call.clone(),
                detail: String::from(detail.shown()),
                truncated: detail.truncated(),
                detail_length: Some(detail.length()),
                whole: detail.whole().map(String::from),
            },
            DroneEvent::Answered { call, failed } => Saw::Answered {
                call: call.clone(),
                failed: *failed,
            },
            DroneEvent::Said { text, .. } => Saw::Said { text: text.clone() },
            DroneEvent::Refused {
                tool,
                call,
                because,
            } => Saw::Refused {
                tool: tool.clone(),
                call: call.clone(),
                because: because.clone(),
            },
            DroneEvent::QuotaMoved { window, status } => Saw::QuotaMoved {
                window: window.clone(),
                status: status.clone(),
            },
            DroneEvent::Ended {
                turns,
                cost_micros,
                refusals,
            } => Saw::Ended {
                turns: *turns,
                cost_micros: *cost_micros,
                refusals: *refusals,
            },
            DroneEvent::Unrecognised { kind } => Saw::Unrecognised { kind: kind.clone() },
            DroneEvent::Unreadable { line, why } => Saw::Unreadable {
                line: line.clone(),
                why: why.clone(),
            },
        },
    }
}

/// Whose row a decoded event is.
///
/// **Drone unless the stream itself says otherwise**, which it does for exactly
/// one kind: prose that arrived on the session's input channel rather than its
/// output. Everything else in a Drone's stream is the run — a tool result is
/// the Drone's call answering, and an opening or an ending is its process.
///
/// **This is where two values widen onto three.** `adapter_traits::Speaker`
/// names what a decoder can witness and `ipc::Voice` names what the record
/// distinguishes; `Voice::Fleet` has no `Speaker` because Fleet acts on the Job
/// around the Drone and never inside its stream. A `match` rather than a
/// `From`, because the widening belongs beside the mapping it serves and
/// neither crate may depend on the other.
fn voice_of(event: &DroneEvent) -> ipc::Voice {
    match event {
        DroneEvent::Said {
            by: Speaker::Armada,
            ..
        } => ipc::Voice::Armada,
        _ => ipc::Voice::Drone,
    }
}

/// One row Armada or Fleet authored. **Not a `DroneEvent` either.**
///
/// It takes the [`Saw`] already built rather than a value to map, which is the
/// opposite of [`seen`] and right for the opposite reason: a Drone event is a
/// vendor's vocabulary being narrowed to Armada's, and this is Armada's own
/// vocabulary already — there is nothing to translate, and a `match` here would
/// be a second place a variant had to be added.
pub(crate) fn authored(at: &Timestamp, step: &StepId, by: ipc::Voice, saw: Saw) -> TranscriptRow {
    TranscriptRow {
        ts: Instant::carried(at.as_str()),
        step: Some(ipc::StepId::from(step)),
        by,
        saw,
    }
}

/// Rows the queue would not take. **Not a `DroneEvent`** — a sink saying what
/// it lost is the one thing the vocabulary cannot say, and a gap left unsaid
/// reads as a Drone that went quiet.
pub(crate) fn missed(at: &Timestamp, step: &StepId, rows: u64) -> TranscriptRow {
    TranscriptRow {
        ts: Instant::carried(at.as_str()),
        step: Some(ipc::StepId::from(step)),
        // Fleet's, and it is withheld from a viewer either way: what a sink
        // lost is the sink's to say and nobody else's.
        by: ipc::Voice::Fleet,
        saw: Saw::Missed { rows },
    }
}

/// One line of `.armada/logs/<job-id>.jsonl`, in the contract's field order.
///
/// **A key is present with a value or absent, never present and null**, which
/// is why every conditional field is skipped rather than written. `target` and
/// `span` are absent from this type entirely: they are supplied by a `tracing`
/// layer, and Fleet has none.
#[derive(Debug, Serialize)]
pub(crate) struct Line<'a> {
    ts: &'a str,
    level: &'static str,
    component: &'static str,
    run_id: &'a str,
    msg: &'a str,
    #[serde(skip_serializing_if = "Option::is_none")]
    job_id: Option<&'a str>,
    #[serde(skip_serializing_if = "Option::is_none")]
    drone_id: Option<&'a str>,
    #[serde(skip_serializing_if = "Option::is_none")]
    step_id: Option<&'a str>,
    #[serde(skip_serializing_if = "Option::is_none")]
    workspace: Option<&'a str>,
    #[serde(skip_serializing_if = "BTreeMap::is_empty")]
    fields: BTreeMap<&'a str, Field>,
}

impl<'a> From<&'a Envelope> for Line<'a> {
    fn from(envelope: &'a Envelope) -> Line<'a> {
        Line {
            ts: envelope.ts().as_str(),
            level: envelope.level().as_wire(),
            component: envelope.component().as_wire(),
            run_id: envelope.run_id().as_str(),
            msg: envelope.msg(),
            job_id: envelope.job_id().map(|id| id.as_str()),
            drone_id: envelope.drone_id().map(|id| id.as_str()),
            step_id: envelope.step_id(),
            workspace: envelope.workspace(),
            fields: envelope
                .fields()
                .iter()
                .map(|(key, value)| (key.as_str(), Field::from(value)))
                .collect(),
        }
    }
}

/// A `FieldValue` on the wire. Untagged, because `fields` is read by a person
/// with `jq` and a tag would put the type of every value in their way.
#[derive(Debug, Serialize)]
#[serde(untagged)]
enum Field {
    Str(String),
    Int(i64),
    Float(f64),
    Bool(bool),
    List(Vec<Field>),
}

impl From<&FieldValue> for Field {
    fn from(value: &FieldValue) -> Field {
        match value {
            FieldValue::Str(text) => Field::Str(text.clone()),
            FieldValue::Int(number) => Field::Int(*number),
            FieldValue::Float(number) => Field::Float(*number),
            FieldValue::Bool(flag) => Field::Bool(*flag),
            FieldValue::List(values) => Field::List(values.iter().map(Field::from).collect()),
        }
    }
}
