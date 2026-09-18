//! A node's content as the one JSON object its `content` column holds.
//!
//! **Keyed by the row's `kind`, never by a tag inside the object.** The kind is
//! a column with a `CHECK` over it, so the object carries only its fields and
//! a kind cannot be said twice in two places that disagree.

use core_model::{
    CaptureBounds, CaptureElement, CaptureFrame, CaptureWindow, EpicRead, EpicTake, ForgeFacts,
    ForgeState, JobId, ScoutCheckout, ScoutEnded, ScoutOutcome, ScoutSource, ScoutSourceKind,
    StudioCapture, StudioFinding, StudioNodeContent, StudioNodeKind, StudioPosition, StudioRunKept,
    Ulid,
};
use serde_json::{json, Map, Value};
use std::collections::BTreeMap;

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
        StudioNodeContent::Note { said, capture } => match capture {
            None => json!({ "said": said }),
            Some(capture) => json!({ "said": said, "capture": capture_written(capture) }),
        },
        StudioNodeContent::Cluster { title } => json!({ "title": title }),
        StudioNodeContent::Finding(finding) => finding_written(finding),
        StudioNodeContent::Contradiction {
            first,
            second,
            answer,
        } => match answer {
            // Absent until a person settles it here, which is the shape every
            // row written before `#1291` already has.
            None => json!({ "first": first, "second": second }),
            Some(answer) => json!({ "first": first, "second": second, "answer": answer }),
        },
        StudioNodeContent::Sketch { body } | StudioNodeContent::Outline { body } => {
            json!({ "body": body })
        }
        // Each is left out where there is none, which is what every row
        // written before `#1378` and `#1293` already looks like.
        StudioNodeContent::Link {
            address,
            said,
            named,
        } => {
            let mut link = Map::new();
            link.insert("address".into(), json!(address));
            if let Some(said) = said {
                link.insert("said".into(), json!(said));
            }
            if let Some(named) = named {
                link.insert("named".into(), json!(named));
            }
            Value::Object(link)
        }
        // **Each of the three writes what it holds and nothing more**, so a
        // node whose title and state no read-in has resolved reads back as one
        // that has not, rather than as one whose forge said nothing.
        StudioNodeContent::Issue {
            address,
            number,
            said,
            title,
            state,
        }
        | StudioNodeContent::PullRequest {
            address,
            number,
            said,
            title,
            state,
        } => {
            let mut node = forge_written(address, number, said, title);
            if let Some(state) = state {
                node.insert("state".into(), json!(state.as_wire()));
            }
            Value::Object(node)
        }
        StudioNodeContent::Epic {
            address,
            number,
            said,
            title,
            read_in,
        } => {
            let mut node = forge_written(address, number, said, title);
            if let Some(read_in) = read_in {
                // **What the answer left out and what it kept are written as
                // numbers**, so an Epic read back says the same thing it said
                // on the board — `#1405`.
                let mut read = serde_json::Map::new();
                read.insert("issues".into(), json!(read_in.issues));
                read.insert("total".into(), json!(read_in.total));
                read.insert("left_out".into(), json!(read_in.left_out));
                read.insert("kept".into(), json!(read_in.kept));
                if let Some(took) = read_in.took {
                    read.insert("took".into(), json!(took.as_wire()));
                }
                if let Some(from) = read_in.laid_out_from {
                    read.insert("laid_out_from".into(), json!({ "x": from.x, "y": from.y }));
                }
                node.insert("read_in".into(), Value::Object(read));
            }
            Value::Object(node)
        }
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
            capture: match object.get("capture") {
                None => None,
                Some(capture) => Some(capture_read(capture)?),
            },
        },
        StudioNodeKind::Cluster => StudioNodeContent::Cluster {
            title: text("title")?,
        },
        StudioNodeKind::Finding => StudioNodeContent::Finding(finding_read(&object)?),
        StudioNodeKind::Contradiction => StudioNodeContent::Contradiction {
            first: text("first")?,
            second: text("second")?,
            answer: object
                .get("answer")
                .and_then(Value::as_str)
                .map(str::to_string),
        },
        StudioNodeKind::Sketch => StudioNodeContent::Sketch {
            body: text("body")?,
        },
        StudioNodeKind::Link => StudioNodeContent::Link {
            address: text("address")?,
            said: object
                .get("said")
                .and_then(Value::as_str)
                .map(str::to_string),
            named: object
                .get("named")
                .and_then(Value::as_str)
                .map(str::to_string),
        },
        // **Read through `on_the_forge` and `resolved`**, the domain's own two
        // constructors, so this module never spells one of the three kinds'
        // fields itself and a row cannot read back as a shape they do not make.
        kind @ (StudioNodeKind::Issue | StudioNodeKind::PullRequest | StudioNodeKind::Epic) => {
            let made = StudioNodeContent::on_the_forge(
                kind,
                text("address")?,
                text("number")?,
                object
                    .get("said")
                    .and_then(Value::as_str)
                    .map(str::to_string),
            )
            .expect("one of the three forge kinds");
            let state = match object.get("state").and_then(Value::as_str) {
                None => None,
                Some(spelled) => Some(ForgeState::from_wire(spelled).ok_or_else(|| {
                    UnreadableContent::UnknownValue {
                        field: "state",
                        value: spelled.to_string(),
                    }
                })?),
            };
            let read_in = match object.get("read_in") {
                None => None,
                Some(read) => Some(EpicRead {
                    issues: read.get("issues").and_then(Value::as_u64).ok_or(
                        UnreadableContent::MissingField {
                            field: "read_in.issues",
                        },
                    )?,
                    total: read.get("total").and_then(Value::as_u64).ok_or(
                        UnreadableContent::MissingField {
                            field: "read_in.total",
                        },
                    )?,
                    // **An Epic read in before `#1405` reads back saying
                    // nothing about which issues it took**, rather than
                    // claiming an answer nobody gave it. The three that follow
                    // are absent the same way, and a count nobody wrote is nought.
                    took: match read.get("took").and_then(Value::as_str) {
                        None => None,
                        Some(spelled) => Some(EpicTake::from_wire(spelled).ok_or_else(|| {
                            UnreadableContent::UnknownValue {
                                field: "read_in.took",
                                value: spelled.to_string(),
                            }
                        })?),
                    },
                    left_out: read.get("left_out").and_then(Value::as_u64).unwrap_or(0),
                    kept: read.get("kept").and_then(Value::as_u64).unwrap_or(0),
                    laid_out_from: read.get("laid_out_from").and_then(a_position),
                }),
            };
            made.resolved(&ForgeFacts {
                title: object
                    .get("title")
                    .and_then(Value::as_str)
                    .map(str::to_string),
                state,
                read_in,
            })
            .expect("one of the three forge kinds")
        }
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

/// The three fields every forge kind keeps, as its object holds them. `said`
/// and `title` are left out where there is none, which is the shape every row
/// a paste wrote has until it is read in.
/// A position a stored object holds, and `None` where it holds none or holds
/// one missing a coordinate. **Absent rather than nought**, because `0, 0` is
/// somewhere a read-in could genuinely have laid its issues out.
fn a_position(stored: &Value) -> Option<StudioPosition> {
    Some(StudioPosition {
        x: stored.get("x")?.as_i64()?,
        y: stored.get("y")?.as_i64()?,
    })
}

fn forge_written(
    address: &str,
    number: &str,
    said: &Option<String>,
    title: &Option<String>,
) -> Map<String, Value> {
    let mut node = Map::new();
    node.insert("address".into(), json!(address));
    node.insert("number".into(), json!(number));
    if let Some(said) = said {
        node.insert("said".into(), json!(said));
    }
    if let Some(title) = title {
        node.insert("title".into(), json!(title));
    }
    node
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
    if !finding.sources().is_empty() {
        let sources: Vec<Value> = finding
            .sources()
            .iter()
            .map(|source| {
                json!({ "address": source.address, "kind": source.kind.as_wire(), "cut": source.cut })
            })
            .collect();
        object.insert("sources".into(), Value::Array(sources));
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
    let sources = match object.get("sources") {
        None => Vec::new(),
        Some(Value::Array(items)) => items
            .iter()
            .map(|item| {
                let spelled = item
                    .get("kind")
                    .and_then(Value::as_str)
                    .ok_or(missing("sources.kind"))?;
                Ok(ScoutSource {
                    address: item
                        .get("address")
                        .and_then(Value::as_str)
                        .ok_or(missing("sources.address"))?
                        .to_string(),
                    kind: ScoutSourceKind::from_wire(spelled).ok_or_else(|| {
                        UnreadableContent::UnknownValue {
                            field: "sources.kind",
                            value: spelled.to_string(),
                        }
                    })?,
                    cut: item.get("cut").and_then(Value::as_u64).unwrap_or(0),
                })
            })
            .collect::<Result<Vec<_>, UnreadableContent>>()?,
        Some(_) => return Err(missing("sources")),
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
        sources,
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
/// A Note's capture. **Only what it holds is written**, so a Note with no
/// styles and no frame reads back the shape it was written in.
fn capture_written(capture: &StudioCapture) -> Value {
    let mut object = Map::new();
    if let Some(component) = &capture.component {
        object.insert("component".into(), json!(component));
    }
    if !capture.owners.is_empty() {
        object.insert("owners".into(), json!(capture.owners));
    }
    object.insert("selector".into(), json!(capture.selector));
    let mut element = Map::new();
    element.insert("tag".into(), json!(capture.element.tag));
    element.insert("text".into(), json!(capture.element.text));
    if let Some(label) = &capture.element.label {
        element.insert("label".into(), json!(label));
    }
    object.insert("element".into(), Value::Object(element));
    for (field, said) in [("screen", &capture.screen), ("layer", &capture.layer)] {
        if let Some(said) = said {
            object.insert(field.into(), json!(said));
        }
    }
    object.insert("location".into(), json!(capture.location));
    let bounds = &capture.bounds;
    object.insert(
        "bounds".into(),
        json!({ "x": bounds.x, "y": bounds.y, "width": bounds.width, "height": bounds.height }),
    );
    object.insert(
        "window".into(),
        json!({ "width": capture.window.width, "height": capture.window.height }),
    );
    if !capture.styles.is_empty() {
        object.insert("styles".into(), json!(capture.styles));
    }
    object.insert("markup".into(), json!(capture.markup));
    if let Some(source) = &capture.source {
        object.insert("source".into(), json!(source));
    }
    if let Some(frame) = &capture.frame {
        object.insert(
            "frame".into(),
            json!({
                "filename": frame.filename,
                "byte_size": frame.byte_size,
                "width": frame.width,
                "height": frame.height,
            }),
        );
    }
    Value::Object(object)
}

fn capture_read(stored: &Value) -> Result<StudioCapture, UnreadableContent> {
    let missing = |field| UnreadableContent::MissingField { field };
    let said = |at: &Value, field: &'static str| -> Result<String, UnreadableContent> {
        at.get(field)
            .and_then(Value::as_str)
            .map(str::to_string)
            .ok_or(missing(field))
    };
    let whole = |at: Option<&Value>, field: &'static str| -> Result<i64, UnreadableContent> {
        at.and_then(|at| at.get(field))
            .and_then(Value::as_i64)
            .ok_or(missing(field))
    };
    let element = stored.get("element").ok_or(missing("capture.element"))?;
    let bounds = stored.get("bounds");
    let window = stored.get("window");
    let maybe = |field: &str| {
        stored
            .get(field)
            .and_then(Value::as_str)
            .map(str::to_string)
    };
    Ok(StudioCapture {
        component: maybe("component"),
        owners: match stored.get("owners") {
            None => Vec::new(),
            Some(Value::Array(items)) => items
                .iter()
                .map(|item| {
                    item.as_str()
                        .map(str::to_string)
                        .ok_or(missing("capture.owners"))
                })
                .collect::<Result<Vec<String>, UnreadableContent>>()?,
            Some(_) => return Err(missing("capture.owners")),
        },
        selector: said(stored, "selector")?,
        element: CaptureElement {
            tag: said(element, "tag")?,
            text: said(element, "text")?,
            label: element
                .get("label")
                .and_then(Value::as_str)
                .map(str::to_string),
        },
        screen: maybe("screen"),
        layer: maybe("layer"),
        location: said(stored, "location")?,
        bounds: CaptureBounds {
            x: whole(bounds, "x")?,
            y: whole(bounds, "y")?,
            width: whole(bounds, "width")?,
            height: whole(bounds, "height")?,
        },
        window: CaptureWindow {
            width: whole(window, "width")?,
            height: whole(window, "height")?,
        },
        styles: match stored.get("styles") {
            None => BTreeMap::new(),
            Some(Value::Object(styles)) => styles
                .iter()
                .map(|(property, value)| {
                    value
                        .as_str()
                        .map(|value| (property.clone(), value.to_string()))
                        .ok_or(missing("capture.styles"))
                })
                .collect::<Result<BTreeMap<String, String>, UnreadableContent>>()?,
            Some(_) => return Err(missing("capture.styles")),
        },
        markup: said(stored, "markup")?,
        source: maybe("source"),
        frame: match stored.get("frame") {
            None => None,
            Some(frame) => Some(CaptureFrame {
                filename: said(frame, "filename")?,
                byte_size: frame
                    .get("byte_size")
                    .and_then(Value::as_u64)
                    .ok_or(missing("capture.frame.byte_size"))?,
                width: whole(Some(frame), "width")?,
                height: whole(Some(frame), "height")?,
            }),
        },
    })
}
