//! The columns the field registry types `array<...>`, and the one place in this
//! workspace that parses untyped JSON.
//!
//! # Why JSON at all
//!
//! `job-fields.toml` gives `acceptance_criteria[]`, `dependencies` and
//! `scope_revisions[]` `storage = "Column"` — one column each on the `jobs`
//! row, not a table. Only two things can put an array in a column: a serde
//! derive on the `core-model` type, or a shape written out by hand here.
//! **The derive would put `serde` under `core-model`, which is a dependency
//! added to every crate in the workspace**, so it is written out by hand.
//!
//! # Both directions in one file, deliberately
//!
//! An encoder and a decoder that live apart drift, and the drift is silent
//! until a Job written by one build fails to load in the next. Each shape below
//! is a `write_*` immediately above its `read_*`, so a field added to one and
//! not the other is visible without leaving the screen.
//!
//! # No `#[serde(default)]` anywhere, because there is nothing to default
//!
//! A field that is absent is a malformed column and says so. Filling it in
//! would turn a Job written by a version that did not have the field into a Job
//! quietly missing it, which is the failure the version number exists for.

use core_model::{
    AcceptanceCriterion, Actor, CriteriaOwed, CriterionId, CriterionSource, DependencyDirection,
    DependencyEdge, EscalationTrigger, JobId, PilotReason, RepoPath, ScopeRevision,
    ScopeRevisionOutcome, StepId, Timestamp, TransitionReason, Ulid,
};
use serde_json::{json, Map, Value};

/// What a decode says when a shape is not what was written.
type Malformed = String;

// ------------------------------------------------------- reading primitives

fn object(value: &Value) -> Result<&Map<String, Value>, Malformed> {
    value
        .as_object()
        .ok_or_else(|| format!("expected an object, found {}", kind(value)))
}

fn array(value: &Value) -> Result<&Vec<Value>, Malformed> {
    value
        .as_array()
        .ok_or_else(|| format!("expected an array, found {}", kind(value)))
}

fn field<'v>(map: &'v Map<String, Value>, name: &str) -> Result<&'v Value, Malformed> {
    map.get(name).ok_or_else(|| format!("`{name}` is absent"))
}

fn text(map: &Map<String, Value>, name: &str) -> Result<String, Malformed> {
    field(map, name)?
        .as_str()
        .map(str::to_string)
        .ok_or_else(|| format!("`{name}` is not a string"))
}

fn flag(map: &Map<String, Value>, name: &str) -> Result<bool, Malformed> {
    field(map, name)?
        .as_bool()
        .ok_or_else(|| format!("`{name}` is not a boolean"))
}

fn texts(map: &Map<String, Value>, name: &str) -> Result<Vec<String>, Malformed> {
    array(field(map, name)?)?
        .iter()
        .map(|entry| {
            entry
                .as_str()
                .map(str::to_string)
                .ok_or_else(|| format!("`{name}` holds a non-string entry"))
        })
        .collect()
}

fn kind(value: &Value) -> &'static str {
    match value {
        Value::Null => "null",
        Value::Bool(_) => "a boolean",
        Value::Number(_) => "a number",
        Value::String(_) => "a string",
        Value::Array(_) => "an array",
        Value::Object(_) => "an object",
    }
}

fn parse(text: &str) -> Result<Value, Malformed> {
    serde_json::from_str(text).map_err(|e| e.to_string())
}

// --------------------------------------------------- acceptance_criteria[]

pub fn write_acceptance_criteria(criteria: &[AcceptanceCriterion]) -> String {
    let entries: Vec<Value> = criteria
        .iter()
        .map(|criterion| {
            json!({
                "criterion_id": criterion.criterion_id.as_str(),
                "text": criterion.text,
                "source": criterion.source.as_wire(),
            })
        })
        .collect();
    Value::Array(entries).to_string()
}

pub fn read_acceptance_criteria(stored: &str) -> Result<Vec<AcceptanceCriterion>, Malformed> {
    array(&parse(stored)?)?
        .iter()
        .map(|entry| {
            let entry = object(entry)?;
            let source = text(entry, "source")?;
            Ok(AcceptanceCriterion {
                criterion_id: CriterionId::new(text(entry, "criterion_id")?),
                text: text(entry, "text")?,
                source: CriterionSource::from_wire(&source)
                    .ok_or_else(|| format!("`source` holds `{source}`"))?,
            })
        })
        .collect()
}

// ------------------------------------------------------------- dependencies

pub fn write_dependencies(edges: &[DependencyEdge]) -> String {
    let entries: Vec<Value> = edges
        .iter()
        .map(|edge| {
            json!({
                "direction": edge.direction.as_wire(),
                "peer": edge.peer.as_str(),
            })
        })
        .collect();
    Value::Array(entries).to_string()
}

pub fn read_dependencies(stored: &str) -> Result<Vec<DependencyEdge>, Malformed> {
    array(&parse(stored)?)?
        .iter()
        .map(|entry| {
            let entry = object(entry)?;
            let direction = text(entry, "direction")?;
            Ok(DependencyEdge {
                direction: DependencyDirection::from_wire(&direction)
                    .ok_or_else(|| format!("`direction` holds `{direction}`"))?,
                peer: JobId::carried(Ulid::carried(text(entry, "peer")?)),
            })
        })
        .collect()
}

// ---------------------------------------------------------- scope_revisions

pub fn write_scope_revisions(revisions: &[ScopeRevision]) -> String {
    let entries: Vec<Value> = revisions
        .iter()
        .map(|revision| {
            json!({
                "at_step": revision.at_step.as_ref().map(StepId::as_str),
                "paths_added": paths(&revision.paths_added),
                "paths_removed": paths(&revision.paths_removed),
                "atomic_before": revision.atomic_before,
                "atomic_after": revision.atomic_after,
                "rationale": revision.rationale,
                "outcome": revision.outcome.as_str(),
                "approved_by": revision.approved_by.as_wire(),
                "at": revision.at.as_str(),
            })
        })
        .collect();
    Value::Array(entries).to_string()
}

fn paths(paths: &[RepoPath]) -> Vec<&str> {
    paths.iter().map(RepoPath::as_str).collect()
}

pub fn read_scope_revisions(stored: &str) -> Result<Vec<ScopeRevision>, Malformed> {
    array(&parse(stored)?)?
        .iter()
        .map(|entry| {
            let entry = object(entry)?;
            let approved_by = text(entry, "approved_by")?;
            let at_step = match field(entry, "at_step")? {
                Value::Null => None,
                Value::String(id) => Some(StepId::new(id.clone())),
                other => return Err(format!("`at_step` is {}", kind(other))),
            };
            Ok(ScopeRevision {
                at_step,
                paths_added: repo_paths(texts(entry, "paths_added")?),
                paths_removed: repo_paths(texts(entry, "paths_removed")?),
                atomic_before: flag(entry, "atomic_before")?,
                atomic_after: flag(entry, "atomic_after")?,
                rationale: text(entry, "rationale")?,
                outcome: ScopeRevisionOutcome::recorded(text(entry, "outcome")?),
                approved_by: Actor::from_wire(&approved_by)
                    .ok_or_else(|| format!("`approved_by` holds `{approved_by}`"))?,
                at: Timestamp::from_rfc3339(text(entry, "at")?),
            })
        })
        .collect()
}

fn repo_paths(raw: Vec<String>) -> Vec<RepoPath> {
    raw.into_iter().map(RepoPath::new).collect()
}

// ------------------------------------------- job_events.reason_kind / _value
//
// Two columns rather than one blob. `reason_kind` makes the row self-describing
// — a person reading the table sees which of the five a reason is without
// knowing the rule that maps a status onto it — and a kind that does not fit
// the status it arrived at is then a detectable fault rather than an
// unrepresentable one.

/// The kind, and the value where the kind has one.
pub fn write_reason(reason: &TransitionReason) -> (&'static str, Option<String>) {
    match reason {
        TransitionReason::Unqualified => ("unqualified", None),
        TransitionReason::DerivedAtRead => ("derived_at_read", None),
        TransitionReason::Escalation(trigger) => {
            ("escalation", Some(trigger.as_wire().to_string()))
        }
        TransitionReason::Pilot(pilot) => ("pilot", Some(pilot.as_wire().to_string())),
        TransitionReason::Attestation(owed) => {
            let ids: Vec<&str> = owed.ids().map(CriterionId::as_str).collect();
            ("attestation", Some(Value::from(ids).to_string()))
        }
    }
}

pub fn read_reason(kind: &str, value: Option<&str>) -> Result<TransitionReason, Malformed> {
    match (kind, value) {
        ("unqualified", None) => Ok(TransitionReason::Unqualified),
        ("derived_at_read", None) => Ok(TransitionReason::DerivedAtRead),
        ("escalation", Some(trigger)) => EscalationTrigger::from_wire(trigger)
            .map(TransitionReason::Escalation)
            .ok_or_else(|| format!("`{trigger}` is not an escalation trigger")),
        ("pilot", Some(pilot)) => PilotReason::from_wire(pilot)
            .map(TransitionReason::Pilot)
            .ok_or_else(|| format!("`{pilot}` is not a pilot reason")),
        ("attestation", Some(owed)) => read_criteria_owed(owed),
        (kind, _) => Err(format!("`{kind}` is not a reason kind, or its value is")),
    }
}

/// **Never empty**, structurally: a Job cannot wait on an attestation it does
/// not owe, so an empty list is malformed rather than a Job owing nothing.
fn read_criteria_owed(stored: &str) -> Result<TransitionReason, Malformed> {
    let parsed = parse(stored)?;
    let mut ids = array(&parsed)?
        .iter()
        .map(|entry| {
            entry
                .as_str()
                .map(CriterionId::new)
                .ok_or_else(|| "an owed criterion is not a string".to_string())
        })
        .collect::<Result<Vec<CriterionId>, Malformed>>()?
        .into_iter();
    let first = ids
        .next()
        .ok_or_else(|| "an attestation owes no criteria".to_string())?;
    Ok(TransitionReason::Attestation(CriteriaOwed::owing(
        first,
        ids.collect(),
    )))
}
