//! A node's content as the one JSON object its `content` column holds.
//!
//! **Keyed by the row's `kind`, never by a tag inside the object.** The kind is
//! a column with a `CHECK` over it, so the object carries only its fields and
//! a kind cannot be said twice in two places that disagree.

use core_model::{JobId, StudioNodeContent, StudioNodeKind, Ulid};
use serde_json::{json, Map, Value};

/// Why a stored content object does not read back.
#[derive(Debug)]
pub enum UnreadableContent {
    UnknownKind { kind: String },
    NotAnObject { detail: String },
    MissingField { field: &'static str },
}

/// The object written for `content`.
pub(super) fn written(content: &StudioNodeContent) -> String {
    let object = match content {
        StudioNodeContent::Run { run_id } => json!({ "run_id": run_id }),
        StudioNodeContent::Note { said } => json!({ "said": said }),
        StudioNodeContent::Cluster { title } => json!({ "title": title }),
        StudioNodeContent::Finding { asked } => json!({ "asked": asked }),
        StudioNodeContent::Contradiction { first, second } => {
            json!({ "first": first, "second": second })
        }
        StudioNodeContent::Sketch { body } | StudioNodeContent::Outline { body } => {
            json!({ "body": body })
        }
        StudioNodeContent::Link { address } => json!({ "address": address }),
        StudioNodeContent::Deferral { what } => json!({ "what": what }),
        StudioNodeContent::IssueDraft { title, body } => json!({ "title": title, "body": body }),
        StudioNodeContent::Job { job_id } => json!({ "job_id": job_id.as_str() }),
    };
    object.to_string()
}

/// The content a row's `kind` and `content` columns hold.
pub(super) fn read(kind: &str, stored: &str) -> Result<StudioNodeContent, UnreadableContent> {
    let kind = StudioNodeKind::from_wire(kind).ok_or_else(|| UnreadableContent::UnknownKind {
        kind: kind.to_string(),
    })?;
    let object: Map<String, Value> =
        serde_json::from_str(stored).map_err(|why| UnreadableContent::NotAnObject {
            detail: why.to_string(),
        })?;
    let text = |field: &'static str| -> Result<String, UnreadableContent> {
        object
            .get(field)
            .and_then(Value::as_str)
            .map(str::to_string)
            .ok_or(UnreadableContent::MissingField { field })
    };
    Ok(match kind {
        StudioNodeKind::Run => StudioNodeContent::Run {
            run_id: text("run_id")?,
        },
        StudioNodeKind::Note => StudioNodeContent::Note {
            said: text("said")?,
        },
        StudioNodeKind::Cluster => StudioNodeContent::Cluster {
            title: text("title")?,
        },
        StudioNodeKind::Finding => StudioNodeContent::Finding {
            asked: text("asked")?,
        },
        StudioNodeKind::Contradiction => StudioNodeContent::Contradiction {
            first: text("first")?,
            second: text("second")?,
        },
        StudioNodeKind::Sketch => StudioNodeContent::Sketch {
            body: text("body")?,
        },
        StudioNodeKind::Link => StudioNodeContent::Link {
            address: text("address")?,
        },
        StudioNodeKind::Deferral => StudioNodeContent::Deferral {
            what: text("what")?,
        },
        StudioNodeKind::Outline => StudioNodeContent::Outline {
            body: text("body")?,
        },
        StudioNodeKind::IssueDraft => StudioNodeContent::IssueDraft {
            title: text("title")?,
            body: text("body")?,
        },
        StudioNodeKind::Job => StudioNodeContent::Job {
            job_id: JobId::carried(Ulid::carried(text("job_id")?)),
        },
    })
}
