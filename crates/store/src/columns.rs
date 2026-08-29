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
    AcceptanceCriterion, Actor, AdvanceGate, ContextSource, Covers, CriteriaOwed, CriterionId,
    CriterionSource, DeclarePlanAt, DependencyDirection, DependencyEdge, EscalationTrigger,
    EvidenceRef, EvidenceScope, EvidenceType, FrozenWorkflow, GamingCheck, GamingPattern, JobId,
    JudgeCheck, JudgeCriterion, ModelName, PathPattern, PilotReason, RepoPath, ResolvedCheck,
    ResolvedStep, ScopeRevision, ScopeRevisionOutcome, StepId, Timestamp, TransitionReason, Ulid,
    WorkflowId, DIFF_NONEMPTY, MANIFEST_CHECK,
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

// -------------------------------------------------------------- jobs.workflow
//
// The whole WorkflowDef a Job froze, in one column. A table would be four more
// — steps, checks, and their order — for a value that is written once at
// creation, read whole, and never queried a piece at a time.

/// The frozen workflow, as one JSON object.
pub fn write_workflow(workflow: &FrozenWorkflow) -> String {
    json!({
        "workflow_id": workflow.id().as_str(),
        "name": workflow.name(),
        "version": workflow.version(),
        "steps": workflow.steps().iter().map(|step| json!({
            "id": step.id().as_str(),
            "label": step.label(),
            "evidence_type": step.evidence_type().map(|kind| kind.as_wire()),
            "advance_gate": step.advance_gate().as_wire(),
            "retry_limit": step.retry_limit(),
            "evidence_scope": step.evidence_scope().map(|scope| json!({
                "context_source": scope.context_source().as_wire(),
                "exclude_paths": scope.exclude_paths().iter()
                    .map(|path| path.as_str()).collect::<Vec<&str>>(),
                "reference_docs": scope.reference_docs().iter()
                    .map(EvidenceRef::as_wire).collect::<Vec<String>>(),
                "scope_diff_check": scope.scope_diff_check(),
                "declare_plan_at": scope.declare_plan_at().map(|at| at.as_wire()),
            })),
            "judge_checks": step.judge_checks().iter().map(|judge| json!({
                "model": judge.model().map(|model| model.as_str()),
                "panel_size": judge.panel_size(),
                "criteria": judge.criteria().iter().map(|criterion| json!({
                    "criterion_id": criterion.criterion_id.as_str(),
                    "question": criterion.question,
                })).collect::<Vec<Value>>(),
                "gaming_check": judge.gaming().map(|gaming| json!({
                    "baseline_ref": gaming.baseline().map(EvidenceRef::as_wire),
                    "flag_if": gaming.flag_if().iter()
                        .map(|pattern| pattern.as_wire()).collect::<Vec<&str>>(),
                })),
            })).collect::<Vec<Value>>(),
            "checks": step.checks().iter().map(|check| match check {
                ResolvedCheck::ManifestCheck { name, run, expect_exit_code, when } => json!({
                    "type": MANIFEST_CHECK,
                    "check": name,
                    "run": run,
                    "expect_exit_code": expect_exit_code,
                    // Absent rather than an empty list where the Check declares
                    // no `when`. The two mean opposite things — always, and
                    // never — and a row written before `when` existed carries
                    // no key at all, which reads back as the first.
                    "when": when.as_ref().map(|covers| covers.patterns().iter()
                        .map(|pattern| pattern.as_str()).collect::<Vec<&str>>()),
                }),
                ResolvedCheck::DiffNonempty => json!({ "type": DIFF_NONEMPTY }),
            }).collect::<Vec<Value>>(),
        })).collect::<Vec<Value>>(),
    })
    .to_string()
}

pub fn read_workflow(stored: &str) -> Result<FrozenWorkflow, Malformed> {
    let value = parse(stored)?;
    let root = object(&value)?;
    let mut steps = Vec::new();
    for entry in array(field(root, "steps")?)? {
        steps.push(read_step(object(entry)?)?);
    }
    Ok(FrozenWorkflow::frozen(
        WorkflowId::carried(Ulid::carried(text(root, "workflow_id")?)),
        text(root, "name")?,
        version(root)?,
        steps,
    ))
}

fn read_step(entry: &Map<String, Value>) -> Result<ResolvedStep, Malformed> {
    let gate = text(entry, "advance_gate")?;
    let evidence_type = match field(entry, "evidence_type")? {
        Value::Null => None,
        Value::String(named) => Some(
            EvidenceType::from_wire(named)
                .ok_or_else(|| format!("`evidence_type` holds `{named}`"))?,
        ),
        other => return Err(format!("`evidence_type` is {}", kind(other))),
    };
    let mut checks = Vec::new();
    for check in array(field(entry, "checks")?)? {
        checks.push(read_check(object(check)?)?);
    }
    Ok(ResolvedStep::frozen(
        StepId::new(text(entry, "id")?),
        text(entry, "label")?,
        evidence_type,
        checks,
        AdvanceGate::from_wire(&gate).ok_or_else(|| format!("`advance_gate` holds `{gate}`"))?,
        read_judge_checks(entry)?,
        read_evidence_scope(entry)?,
        read_retry_limit(entry)?,
    ))
}

/// How many hand-backs the step declared. **Absent reads as none**, the same
/// backfill `read_evidence_scope` and `read_judge_checks` get: every workflow
/// frozen before a retry budget existed had none, and none is what those steps
/// meant. A value that is there and is not a count is a refusal rather than a
/// zero — a Job frozen with a budget must not lose it quietly.
fn read_retry_limit(entry: &Map<String, Value>) -> Result<u32, Malformed> {
    match entry.get("retry_limit") {
        None | Some(Value::Null) => Ok(0),
        Some(Value::Number(found)) => found
            .as_u64()
            .filter(|n| *n <= u64::from(u32::MAX))
            .and_then(|n| u32::try_from(n).ok())
            .ok_or_else(|| format!("`retry_limit` holds `{found}`")),
        Some(other) => Err(format!("`retry_limit` is {}", kind(other))),
    }
}

/// What the step's evidence is scoped to.
///
/// **Absent reads as none**, for the reason [`read_judge_checks`] gives: every
/// workflow written before a scope existed declared none, and none is what
/// those steps meant.
fn read_evidence_scope(entry: &Map<String, Value>) -> Result<Option<EvidenceScope>, Malformed> {
    let Some(Value::Object(scope)) = entry.get("evidence_scope") else {
        return Ok(None);
    };
    let source = text(scope, "context_source")?;
    let declare_plan_at = match scope.get("declare_plan_at") {
        Some(Value::String(at)) => Some(
            DeclarePlanAt::from_wire(at)
                .ok_or_else(|| format!("`declare_plan_at` holds `{at}`"))?,
        ),
        _ => None,
    };
    let mut exclude_paths = Vec::new();
    if let Some(Value::Array(excluded)) = scope.get("exclude_paths") {
        for path in excluded {
            exclude_paths
                .push(RepoPath::new(path.as_str().ok_or_else(|| {
                    "an excluded path is not a string".to_string()
                })?));
        }
    }
    // **Absent reads as none**, the same backfill every other key on this
    // object gets: a workflow frozen before a yardstick existed named none.
    let mut reference_docs = Vec::new();
    if let Some(Value::Array(referenced)) = scope.get("reference_docs") {
        for named in referenced {
            let named = named
                .as_str()
                .ok_or_else(|| "a reference doc is not a string".to_string())?;
            reference_docs.push(
                EvidenceRef::parse(named)
                    .ok_or_else(|| format!("`reference_docs` holds `{named}`"))?,
            );
        }
    }
    Ok(Some(EvidenceScope::declared(
        ContextSource::from_wire(&source)
            .ok_or_else(|| format!("`context_source` holds `{source}`"))?,
        exclude_paths,
        reference_docs,
        scope
            .get("scope_diff_check")
            .and_then(Value::as_bool)
            .unwrap_or(false),
        declare_plan_at,
    )))
}

/// What the step asks the Judge.
///
/// **Absent reads as none**, which is the one backfill this column needs: every
/// workflow written before the semantic tier existed declared no criterion, and
/// no criterion is exactly what those steps meant.
fn read_judge_checks(entry: &Map<String, Value>) -> Result<Vec<JudgeCheck>, Malformed> {
    let Some(Value::Array(declared)) = entry.get("judge_checks") else {
        return Ok(Vec::new());
    };
    let mut checks = Vec::new();
    for judge in declared {
        checks.push(read_judge(object(judge)?)?);
    }
    Ok(checks)
}

fn read_judge(judge: &Map<String, Value>) -> Result<JudgeCheck, Malformed> {
    let model = match judge.get("model") {
        Some(Value::String(named)) => {
            Some(ModelName::new(named).map_err(|blank| format!("`judge.model` {blank}"))?)
        }
        _ => None,
    };
    let panel_size = judge
        .get("panel_size")
        .and_then(Value::as_u64)
        .and_then(|size| u32::try_from(size).ok())
        .ok_or_else(|| "`judge.panel_size` is not a panel size".to_string())?;
    let mut criteria = Vec::new();
    for criterion in array(field(judge, "criteria")?)? {
        let criterion = object(criterion)?;
        criteria.push(JudgeCriterion {
            criterion_id: CriterionId::new(text(criterion, "criterion_id")?),
            question: text(criterion, "question")?,
        });
    }
    Ok(JudgeCheck::declared(
        model,
        panel_size,
        criteria,
        read_gaming_check(judge)?,
    ))
}

/// The second look, where the step declared one. **Absent reads as none**, for
/// the reason `judge_checks` itself does: every workflow frozen before this
/// existed declared no gaming check.
fn read_gaming_check(judge: &Map<String, Value>) -> Result<Option<GamingCheck>, Malformed> {
    let Some(Value::Object(gaming)) = judge.get("gaming_check") else {
        return Ok(None);
    };
    let baseline = match gaming.get("baseline_ref") {
        Some(Value::String(named)) => Some(
            EvidenceRef::parse(named)
                .ok_or_else(|| format!("`gaming_check.baseline_ref` holds `{named}`"))?,
        ),
        _ => None,
    };
    let mut flag_if = Vec::new();
    for pattern in array(field(gaming, "flag_if")?)? {
        let named = pattern
            .as_str()
            .ok_or_else(|| "`gaming_check.flag_if` holds something that is not a pattern")?;
        flag_if.push(
            GamingPattern::from_wire(named)
                .ok_or_else(|| format!("`gaming_check.flag_if` holds `{named}`"))?,
        );
    }
    Ok(Some(GamingCheck::declared(baseline, flag_if)))
}

fn read_check(entry: &Map<String, Value>) -> Result<ResolvedCheck, Malformed> {
    let named = text(entry, "type")?;
    match named.as_str() {
        MANIFEST_CHECK => Ok(ResolvedCheck::ManifestCheck {
            name: text(entry, "check")?,
            run: text(entry, "run")?,
            expect_exit_code: field(entry, "expect_exit_code")?
                .as_i64()
                .ok_or_else(|| "`expect_exit_code` is not an integer".to_string())?,
            when: read_when(entry)?,
        }),
        DIFF_NONEMPTY => Ok(ResolvedCheck::DiffNonempty),
        other => Err(format!("`type` holds `{other}`")),
    }
}

/// Which paths a stored Check covers.
///
/// **Absent and null both read as `None`, which means always.** A row written
/// before `when` existed carries no key, and that is the same sentence as a
/// Check that declares no `when` — the additive case this whole key is built to
/// keep working.
///
/// A pattern the dialect cannot read is malformed rather than dropped: a Check
/// whose scope was silently widened to everything, or narrowed to nothing, is
/// worse than a Job that refuses to load.
fn read_when(entry: &Map<String, Value>) -> Result<Option<Covers>, Malformed> {
    let Some(value) = entry.get("when") else {
        return Ok(None);
    };
    if value.is_null() {
        return Ok(None);
    }
    let mut patterns = Vec::new();
    for item in array(value)? {
        let written = item
            .as_str()
            .ok_or_else(|| "`when` holds something that is not a pattern".to_string())?;
        patterns.push(
            PathPattern::parse(written)
                .map_err(|why| format!("`when` holds `{written}`, which {why}"))?,
        );
    }
    // `Covers::of` answers `None` on an empty list, which would read back as
    // "always" — the opposite of what a stored empty list would have meant. It
    // cannot be written by `write_workflow`, and it is refused rather than
    // reinterpreted.
    Covers::of(patterns)
        .map(Some)
        .ok_or_else(|| "`when` is an empty list".to_string())
}

/// A definition's own version number. `u32` on the record, so a stored value
/// outside that range is malformed rather than clamped.
fn version(root: &Map<String, Value>) -> Result<u32, Malformed> {
    field(root, "version")?
        .as_u64()
        .and_then(|version| u32::try_from(version).ok())
        .ok_or_else(|| "`version` is not a version number".to_string())
}
