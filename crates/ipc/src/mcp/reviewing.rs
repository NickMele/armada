//! The `review` object `submit_evidence` takes on a step that asks for a review. #903.
//!
//! **Read as `core_model`'s review parts**, the ones Fleet checks and keeps, so there is
//! no second shape between the tool and the record: `planning`'s rule.

use std::fmt;

use core_model::{
    Area, Bucket, ChangedTest, Confidence, Finding, Proves, TestChange, TestsInChange, Untested,
};
use serde_json::{json, Map, Value};

use super::tools::{closed, filled, list, text, NotAnArgument, TOOL};

pub const REVIEW_FIELDS: &[&str] = &["says", "reasons", "areas", "tests", "findings"];
pub const AREA_FIELDS: &[&str] = &["name", "what", "files"];
pub const TESTS_FIELDS: &[&str] = &["proves", "changed", "untested"];
pub const PROVES_FIELDS: &[&str] = &["area", "what", "tests"];
pub const CHANGED_FIELDS: &[&str] = &["name", "change", "replaced_by", "why"];
pub const UNTESTED_FIELDS: &[&str] = &["code", "why"];
pub const FINDING_FIELDS: &[&str] = &["bucket", "finding", "why"];

/// A review as the Drone wrote it, in `core_model`'s parts. Fleet checks it against the change.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct SubmittedReview {
    pub says: Confidence,
    pub reasons: Vec<String>,
    pub areas: Vec<Area>,
    pub tests: TestsInChange,
    pub findings: Vec<Finding>,
}

/// Why a `review` object did not read as one. A finding with no reason is not here:
/// Fleet refuses that by name, with every other fault, when it checks the change.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum ReviewArgument {
    NotAnObject { field: &'static str },
    NotAListOfObjects { field: &'static str },
    NotACount { field: &'static str },
    NoSuchVerdict { named: String },
    NoSuchChange { named: String },
    NoSuchBucket { named: String },
}

impl fmt::Display for ReviewArgument {
    fn fmt(&self, out: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            ReviewArgument::NotAnObject { field } => write!(
                out,
                "`{field}` is not an object. The tool's description lists its fields"
            ),
            ReviewArgument::NotAListOfObjects { field } => write!(
                out,
                "`{field}` is not a list of objects. Send [] where it is legitimately empty"
            ),
            ReviewArgument::NotACount { field } => {
                write!(out, "`{field}` is not a whole number")
            }
            ReviewArgument::NoSuchVerdict { named } => write!(
                out,
                "`says` is `{named}`. It is `confident` or `not_confident`"
            ),
            ReviewArgument::NoSuchChange { named } => {
                write!(out, "`change` is `{named}`. It is `removed` or `loosened`")
            }
            ReviewArgument::NoSuchBucket { named } => write!(
                out,
                "`bucket` is `{named}`. It is `needs_you`, `small_fix` or `for_context`"
            ),
        }
    }
}

fn refused(why: ReviewArgument) -> NotAnArgument {
    NotAnArgument::Reviewing(why)
}

/// The `review` field, or `None` where the call carries none.
pub(super) fn review(
    arguments: &Map<String, Value>,
) -> Result<Option<SubmittedReview>, NotAnArgument> {
    let Some(value) = arguments.get("review") else {
        return Ok(None);
    };
    let review = object(value, "review")?;
    closed(review, TOOL, REVIEW_FIELDS)?;
    let named = filled(review, "says")?;
    let says = Confidence::from_wire(named.trim()).ok_or_else(|| {
        refused(ReviewArgument::NoSuchVerdict {
            named: named.clone(),
        })
    })?;
    let mut areas = Vec::new();
    for entry in objects(review, "areas")? {
        closed(entry, TOOL, AREA_FIELDS)?;
        let files = list(entry, "files")?;
        let files: Vec<&str> = files.iter().map(String::as_str).collect();
        areas.push(Area::of(
            &filled(entry, "name")?,
            &text(entry, "what")?,
            &files,
        ));
    }
    let mut findings = Vec::new();
    for entry in objects(review, "findings")? {
        closed(entry, TOOL, FINDING_FIELDS)?;
        let named = text(entry, "bucket")?;
        let bucket = Bucket::from_wire(named.trim()).ok_or_else(|| {
            refused(ReviewArgument::NoSuchBucket {
                named: named.clone(),
            })
        })?;
        findings.push(Finding::of(
            bucket,
            &filled(entry, "finding")?,
            &text(entry, "why")?,
        ));
    }
    Ok(Some(SubmittedReview {
        says,
        reasons: list(review, "reasons")?,
        areas,
        tests: tests(review)?,
        findings,
    }))
}

fn tests(review: &Map<String, Value>) -> Result<TestsInChange, NotAnArgument> {
    let tests = object(
        review
            .get("tests")
            .ok_or(NotAnArgument::Missing { field: "tests" })?,
        "tests",
    )?;
    closed(tests, TOOL, TESTS_FIELDS)?;
    let mut proves = Vec::new();
    for entry in objects(tests, "proves")? {
        closed(entry, TOOL, PROVES_FIELDS)?;
        let count = entry
            .get("tests")
            .ok_or(NotAnArgument::Missing { field: "tests" })?
            .as_u64()
            .and_then(|count| u32::try_from(count).ok())
            .ok_or_else(|| refused(ReviewArgument::NotACount { field: "tests" }))?;
        proves.push(Proves::of(
            &text(entry, "area")?,
            &text(entry, "what")?,
            count,
        ));
    }
    let mut changed = Vec::new();
    for entry in objects(tests, "changed")? {
        closed(entry, TOOL, CHANGED_FIELDS)?;
        let named = text(entry, "change")?;
        let change = TestChange::from_wire(named.trim(), optional(entry, "replaced_by")?)
            .ok_or_else(|| {
                refused(ReviewArgument::NoSuchChange {
                    named: named.clone(),
                })
            })?;
        changed.push(ChangedTest {
            name: filled(entry, "name")?,
            change,
            why: optional(entry, "why")?,
        });
    }
    let mut untested = Vec::new();
    for entry in objects(tests, "untested")? {
        closed(entry, TOOL, UNTESTED_FIELDS)?;
        untested.push(Untested::of(&filled(entry, "code")?, &text(entry, "why")?));
    }
    Ok(TestsInChange {
        proves,
        changed,
        untested,
    })
}

fn object<'a>(
    value: &'a Value,
    field: &'static str,
) -> Result<&'a Map<String, Value>, NotAnArgument> {
    value
        .as_object()
        .ok_or_else(|| refused(ReviewArgument::NotAnObject { field }))
}

fn objects<'a>(
    within: &'a Map<String, Value>,
    field: &'static str,
) -> Result<Vec<&'a Map<String, Value>>, NotAnArgument> {
    let listed = within
        .get(field)
        .ok_or(NotAnArgument::Missing { field })?
        .as_array()
        .ok_or_else(|| refused(ReviewArgument::NotAListOfObjects { field }))?;
    listed
        .iter()
        .map(|entry| {
            entry
                .as_object()
                .ok_or_else(|| refused(ReviewArgument::NotAListOfObjects { field }))
        })
        .collect()
}

/// Text that may be left out. Absent, null and blank all read as nothing said.
fn optional(
    within: &Map<String, Value>,
    field: &'static str,
) -> Result<Option<String>, NotAnArgument> {
    match within.get(field) {
        None | Some(Value::Null) => Ok(None),
        Some(value) => {
            let said = value.as_str().ok_or(NotAnArgument::NotText { field })?;
            Ok(Some(said.to_string()).filter(|said| !said.trim().is_empty()))
        }
    }
}

/// The `review` property of `submit_evidence`'s input schema.
pub(super) fn review_property() -> Value {
    json!({
        "type": "object",
        "description":
            "Only on a step that asks for a review: whether you are confident in the \
             change and why, its areas, the tests in it, and what you found. Fleet checks \
             that every changed file is in an area and every test you name is in the diff.",
        "properties": {
            "says": { "type": "string", "enum": ["confident", "not_confident"] },
            "reasons": {
                "type": "array",
                "items": { "type": "string" },
                "description": "At most three short reasons for the verdict.",
            },
            "areas": {
                "type": "array",
                "description": "Every changed file belongs to an area.",
                "items": {
                    "type": "object",
                    "properties": {
                        "name": { "type": "string" },
                        "what": { "type": "string" },
                        "files": { "type": "array", "items": { "type": "string" } },
                    },
                    "required": ["name", "what", "files"],
                    "additionalProperties": false,
                },
            },
            "tests": {
                "type": "object",
                "properties": {
                    "proves": {
                        "type": "array",
                        "items": {
                            "type": "object",
                            "properties": {
                                "area": { "type": "string" },
                                "what": { "type": "string" },
                                "tests": { "type": "integer", "minimum": 0 },
                            },
                            "required": ["area", "what", "tests"],
                            "additionalProperties": false,
                        },
                    },
                    "changed": {
                        "type": "array",
                        "description":
                            "Tests the change removed or loosened. One given no why needs the person.",
                        "items": {
                            "type": "object",
                            "properties": {
                                "name": { "type": "string" },
                                "change": { "type": "string", "enum": ["removed", "loosened"] },
                                "replaced_by": { "type": "string" },
                                "why": { "type": "string" },
                            },
                            "required": ["name", "change"],
                            "additionalProperties": false,
                        },
                    },
                    "untested": {
                        "type": "array",
                        "items": {
                            "type": "object",
                            "properties": {
                                "code": { "type": "string" },
                                "why": { "type": "string" },
                            },
                            "required": ["code", "why"],
                            "additionalProperties": false,
                        },
                    },
                },
                "required": ["proves", "changed", "untested"],
                "additionalProperties": false,
            },
            "findings": {
                "type": "array",
                "items": {
                    "type": "object",
                    "properties": {
                        "bucket": { "type": "string", "enum": ["needs_you", "small_fix", "for_context"] },
                        "finding": { "type": "string" },
                        "why": { "type": "string" },
                    },
                    "required": ["bucket", "finding", "why"],
                    "additionalProperties": false,
                },
            },
        },
        "required": ["says", "reasons", "areas", "tests", "findings"],
        "additionalProperties": false,
    })
}
