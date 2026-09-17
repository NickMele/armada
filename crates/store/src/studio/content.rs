//! A node's content as the one JSON object its `content` column holds.
//!
//! **Keyed by the row's `kind`, never by a tag inside the object.** The kind is
//! a column with a `CHECK` over it, so the object carries only its fields and
//! a kind cannot be said twice in two places that disagree.

use core_model::{
    JobId, ScoutCheckout, ScoutEnded, ScoutOutcome, StudioFinding, StudioNodeContent,
    StudioNodeKind, StudioRunKept, Ulid,
};
use serde_json::{json, Map, Value};

/// Why a stored content object does not read back.
#[derive(Debug)]
pub enum UnreadableContent {
    UnknownKind {
        kind: String,
    },
    NotAnObject {
        detail: String,
    },
    MissingField {
        field: &'static str,
    },
    /// A field present with a value that is not one it holds.
    UnknownValue {
        field: &'static str,
        value: String,
    },
}

/// The object written for `content`.
pub(super) fn written(content: &StudioNodeContent) -> String {
    let object = match content {
        StudioNodeContent::Run { run_id, kept } => match kept {
            None => json!({ "run_id": run_id }),
            Some(kept) => json!({
                "run_id": run_id,
                "kept": {
                    "name": kept.name,
                    "command": kept.command,
                    "exit_code": kept.exit_code,
                    "expect_exit_code": kept.expect_exit_code,
                    "stopped": kept.stopped,
                    "duration_ms": kept.duration_ms,
                    "lines": kept.lines,
                    "total_lines": kept.total_lines,
                    "whole": kept.whole,
                },
            }),
        },
        StudioNodeContent::Note { said } => json!({ "said": said }),
        StudioNodeContent::Cluster { title } => json!({ "title": title }),
        StudioNodeContent::Finding(finding) => finding_written(finding),
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
            kept: match object.get("kept") {
                None | Some(Value::Null) => None,
                Some(kept) => Some(run_kept(kept)?),
            },
        },
        StudioNodeKind::Note => StudioNodeContent::Note {
            said: text("said")?,
        },
        StudioNodeKind::Cluster => StudioNodeContent::Cluster {
            title: text("title")?,
        },
        StudioNodeKind::Finding => StudioNodeContent::Finding(finding_read(&object)?),
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

/// A Finding's object. **Only what it holds is written**: a Proposed one is
/// `asked` alone, the shape rows kept before scouts existed already have.
fn finding_written(finding: &StudioFinding) -> Value {
    let mut object = Map::new();
    object.insert("asked".into(), json!(finding.ask()));
    if let Some(checkout) = finding.checkout() {
        object.insert(
            "checkout".into(),
            json!({ "commit": checkout.commit, "uncommitted": checkout.uncommitted }),
        );
    }
    if !finding.read().is_empty() {
        object.insert("read".into(), json!(finding.read()));
    }
    if !finding.searched().is_empty() {
        object.insert("searched".into(), json!(finding.searched()));
    }
    if let Some(learned) = finding.learned() {
        object.insert("learned".into(), json!(learned));
    }
    if let Some(ended) = finding.ended() {
        let mut end = Map::new();
        let (outcome, why) = match &ended.outcome {
            ScoutOutcome::Answered => ("answered", None),
            ScoutOutcome::Stopped => ("stopped", None),
            ScoutOutcome::Failed { why } => ("failed", Some(why)),
        };
        end.insert("outcome".into(), json!(outcome));
        if let Some(why) = why {
            end.insert("why".into(), json!(why));
        }
        if let Some(cost) = ended.cost_micros {
            end.insert("cost_micros".into(), json!(cost));
        }
        object.insert("ended".into(), Value::Object(end));
    }
    Value::Object(object)
}

fn finding_read(object: &Map<String, Value>) -> Result<StudioFinding, UnreadableContent> {
    let missing = |field| UnreadableContent::MissingField { field };
    let asked = object
        .get("asked")
        .and_then(Value::as_str)
        .ok_or(missing("asked"))?;
    let checkout = match object.get("checkout") {
        None => None,
        Some(checkout) => Some(ScoutCheckout {
            commit: checkout
                .get("commit")
                .and_then(Value::as_str)
                .ok_or(missing("checkout.commit"))?
                .to_string(),
            uncommitted: checkout
                .get("uncommitted")
                .and_then(Value::as_bool)
                .ok_or(missing("checkout.uncommitted"))?,
        }),
    };
    let texts = |field: &'static str| -> Result<Vec<String>, UnreadableContent> {
        match object.get(field) {
            None => Ok(Vec::new()),
            Some(Value::Array(items)) => items
                .iter()
                .map(|item| item.as_str().map(str::to_string).ok_or(missing(field)))
                .collect(),
            Some(_) => Err(missing(field)),
        }
    };
    let ended = match object.get("ended") {
        None => None,
        Some(end) => {
            let spelled = end
                .get("outcome")
                .and_then(Value::as_str)
                .ok_or(missing("ended.outcome"))?;
            let outcome = match spelled {
                "answered" => ScoutOutcome::Answered,
                "stopped" => ScoutOutcome::Stopped,
                "failed" => ScoutOutcome::Failed {
                    why: end
                        .get("why")
                        .and_then(Value::as_str)
                        .ok_or(missing("ended.why"))?
                        .to_string(),
                },
                other => {
                    return Err(UnreadableContent::UnknownValue {
                        field: "ended.outcome",
                        value: other.to_string(),
                    })
                }
            };
            Some(ScoutEnded {
                outcome,
                cost_micros: end.get("cost_micros").and_then(Value::as_u64),
            })
        }
    };
    Ok(StudioFinding::recorded(
        asked.to_string(),
        checkout,
        texts("read")?,
        texts("searched")?,
        object
            .get("learned")
            .and_then(Value::as_str)
            .map(str::to_string),
        ended,
    ))
}

/// What a Run node kept of a swept run, as its `kept` object holds it.
///
/// **Every field or none.** A half-read result would put a wrong exit code or
/// a wrong duration under a run nobody can go back and check, so a missing one
/// fails the row by name the way every other node's does.
fn run_kept(stored: &Value) -> Result<StudioRunKept, UnreadableContent> {
    let object = stored
        .as_object()
        .ok_or_else(|| UnreadableContent::NotAnObject {
            detail: String::from("a Run node's `kept` is not an object"),
        })?;
    let missing = |field: &'static str| UnreadableContent::MissingField { field };
    let text = |field: &'static str| {
        object
            .get(field)
            .and_then(Value::as_str)
            .map(str::to_string)
            .ok_or(missing(field))
    };
    Ok(StudioRunKept {
        name: text("name")?,
        command: text("command")?,
        // Absent and null both read as killed before it exited: `ipc` leaves
        // an absent option out rather than sending null, and this column is
        // written from the same value.
        exit_code: object
            .get("exit_code")
            .and_then(Value::as_i64)
            .map(|code| code as i32),
        expect_exit_code: object
            .get("expect_exit_code")
            .and_then(Value::as_i64)
            .ok_or(missing("expect_exit_code"))?,
        stopped: object
            .get("stopped")
            .and_then(Value::as_bool)
            .ok_or(missing("stopped"))?,
        duration_ms: object
            .get("duration_ms")
            .and_then(Value::as_u64)
            .ok_or(missing("duration_ms"))?,
        lines: object
            .get("lines")
            .and_then(Value::as_array)
            .ok_or(missing("lines"))?
            .iter()
            .map(|line| line.as_str().unwrap_or_default().to_string())
            .collect(),
        total_lines: object
            .get("total_lines")
            .and_then(Value::as_u64)
            .ok_or(missing("total_lines"))? as u32,
        whole: object
            .get("whole")
            .and_then(Value::as_bool)
            .ok_or(missing("whole"))?,
    })
}
