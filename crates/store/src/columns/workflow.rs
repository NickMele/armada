//! The frozen workflow, as one column, and the only nested document in this
//! dialect.
//!
//! **Its own file, and it is not a split by line count.** Everything in
//! [`super`] is a flat array or a scalar pair; this is a whole `WorkflowDef` —
//! steps, their Checks, their Judge criteria and the scope each declares —
//! written and read as one object. A table would be four more for a value that
//! is written once at creation, read whole, and never queried a piece at a
//! time.
//!
//! **Both directions still sit together**, which is the rule [`super`] states:
//! each `write_*` is immediately above its `read_*`, so a field added to one
//! and not the other is visible without leaving the screen. What moved is the
//! pair, never one half of it.
//!
//! The reading primitives are [`super`]'s, because every dialect in this crate
//! reports a malformed shape the same way.

use std::collections::BTreeMap;

use core_model::{
    AdvanceGate, ContextSource, Covers, CriterionId, DeclarePlanAt, EvidenceRef, EvidenceScope,
    EvidenceType, FrozenWorkflow, GamingCheck, GamingPattern, GateVerdict, JudgeCheck,
    JudgeCriterion, ModelName, Narrowing, OnRefusal, PathPattern, Prerequisite, RepoPath,
    ResolvedCheck, ResolvedStep, StepId, Ulid, WorkflowId, ARTIFACT_EXISTS, DIFF_NONEMPTY,
    MANIFEST_CHECK,
};
use serde_json::{json, Map, Value};

use super::{array, field, kind, object, parse, text, texts, Malformed};

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
            // The loop, written as the pair it is read as. Absent rather than
            // an empty object where the step closes no loop, which is every
            // step of every linear workflow and every row frozen before the
            // keys existed — and `iteration_cap` rides with it, because a cap
            // beside no routing is the half-statement `config` refuses in the
            // file and `ResolvedStep::looping` refuses in the value.
            "verdict_routing": step.closes_a_loop().then(|| {
                step.verdict_routing().iter()
                    .map(|(verdict, target)| {
                        (verdict.as_wire().to_string(), Value::from(target.as_str()))
                    })
                    .collect::<Map<String, Value>>()
            }),
            "iteration_cap": step.closes_a_loop().then(|| step.iteration_cap()),
            // Null where the step named none, which is the ordinary shape and
            // the shape of every row written before a step could name one. It
            // is not backfilled with the Job's: absent is the step deferring,
            // and a row that recorded the deferral as a decision would keep
            // answering with a model the workflow never asked for.
            "model": step.model().map(|model| model.as_str()),
            // Written on every step, because a boolean has no absent value
            // meaning anything other than `false`. A row frozen before the key
            // existed carries none, which reads back as `false`.
            "may_dispatch_jobs": step.may_dispatch_jobs(),
            // Written on every step, for `may_dispatch_jobs`' reason: a boolean
            // has no absent value meaning anything other than `false`, and here
            // `false` on every step would be a workflow that delivers nothing.
            // A row frozen before the key existed carries it on no step at all,
            // and `read_workflow` puts that reading back rather than letting it
            // fall to `false` — see there.
            "delivers": step.delivers(),
            // Written on every step, for `may_dispatch_jobs`' reason — and it
            // needs no `read_workflow`-style correction the way `delivers`
            // does: a row frozen before the key existed comes from a workflow
            // that could not have said it, so `false` is the fact rather than a
            // gap. It is written beside `checks` and not folded into it,
            // because on a repository declaring no Checks the expansion is
            // empty and this is the only thing left that says the step asked.
            "gates_on_every_check": step.gates_on_every_check(),
            // Null where the step named none, which is almost every step and
            // every row written before `#60`. It is not backfilled with what
            // Fleet was running at the time: absent is the step deferring to
            // whatever Fleet is running *now*, and a row that recorded the
            // deferral as a number would freeze a value the workflow never
            // asked for — which is the one thing `lifetime = "Live"` on
            // `drone-silence-threshold-quiet-after` forbids.
            //
            // Two keys and not one object, for the reason there are two
            // fields: a step overriding either half must not restate the
            // other, and a row holding `{quiet_after, pokes}` would make the
            // absent half unspellable.
            "quiet_after_seconds": step.quiet_after_seconds(),
            "poke_limit": step.poke_limit(),
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
                    "on_refusal": criterion.on_refusal.as_wire(),
                })).collect::<Vec<Value>>(),
                "gaming_check": judge.gaming().map(|gaming| json!({
                    "baseline_ref": gaming.baseline().map(EvidenceRef::as_wire),
                    "flag_if": gaming.flag_if().iter()
                        .map(|pattern| pattern.as_wire()).collect::<Vec<&str>>(),
                })),
            })).collect::<Vec<Value>>(),
            "checks": step.checks().iter().map(|check| match check {
                ResolvedCheck::ManifestCheck { name, run, expect_exit_code, when, requires, narrow } => json!({
                    "type": MANIFEST_CHECK,
                    "check": name,
                    "run": run,
                    "expect_exit_code": expect_exit_code,
                    // Absent where the Check requires nothing, which is every
                    // row written before the key existed and reads back the
                    // same way. Name and command line both, because a failure
                    // has to say what a person edits and what they re-run.
                    "requires": match requires.is_empty() {
                        true => None,
                        false => Some(requires.iter().map(|needed| json!({
                            "command": needed.name(),
                            "run": needed.run(),
                        })).collect::<Vec<Value>>()),
                    },
                    // Absent rather than an empty list where the Check declares
                    // no `when`. The two mean opposite things — always, and
                    // never — and a row written before `when` existed carries
                    // no key at all, which reads back as the first.
                    "when": when.as_ref().map(|covers| covers.patterns().iter()
                        .map(|pattern| pattern.as_str()).collect::<Vec<&str>>()),
                    // Absent where the Check declares no `narrow`, which is a
                    // Check that runs whole — the reading a row written before
                    // the key existed gets for free.
                    "narrow": narrow.as_ref().map(|narrowing| json!({
                        "run": narrowing.run(),
                        "each": narrowing.each(),
                        "from": narrowing.from().map(|covers| covers.patterns().iter()
                            .map(|pattern| pattern.as_str()).collect::<Vec<&str>>()),
                        "under": narrowing.under(),
                        // Absent rather than an empty list where the Check
                        // excludes nothing, for `requires`' reason: a row
                        // written before the key existed carries no key at
                        // all, and that reads back as the same sentence.
                        "except": match narrowing.except().is_empty() {
                            true => None,
                            false => Some(narrowing.except()),
                        },
                    })),
                }),
                ResolvedCheck::DiffNonempty => json!({ "type": DIFF_NONEMPTY }),
                ResolvedCheck::ArtifactExists { target } => json!({
                    "type": ARTIFACT_EXISTS,
                    "target": target,
                }),
            }).collect::<Vec<Value>>(),
        })).collect::<Vec<Value>>(),
    })
    .to_string()
}

pub fn read_workflow(stored: &str) -> Result<FrozenWorkflow, Malformed> {
    let value = parse(stored)?;
    let root = object(&value)?;
    let entries = array(field(root, "steps")?)?;
    let mut steps = Vec::new();
    for entry in entries {
        steps.push(read_step(object(entry)?)?);
    }
    // **A row that names no delivering step at all is one frozen before the
    // key existed, and its last step is what delivered.** Read over the whole
    // step list because that is the scope of the question: `delivers` is
    // written on every step from here on, so a row where *no* step carries it
    // is not a workflow that declared it delivers nothing — it is a Job created
    // when nothing could declare either, and what it was created under is that
    // the last step's advance landed the work.
    //
    // The alternative readings are both a Job broken in flight: `false`
    // everywhere stops an approved Job from ever pushing its branch, and
    // refusing the row stops it from loading at all. A record says what it
    // meant when it was written, which is the same rule `when`, `requires` and
    // `iteration_cap` are each read by one field up.
    if !entries.iter().any(declares_delivery) {
        if let Some(last) = steps.pop() {
            steps.push(last.delivering(true));
        }
    }
    Ok(FrozenWorkflow::frozen(
        WorkflowId::carried(Ulid::carried(text(root, "workflow_id")?)),
        text(root, "name")?,
        version(root)?,
        steps,
    ))
}

/// Whether this step's row carries a delivery declaration at all. **Null is
/// absent**, which is how every other optional key on the row is read.
fn declares_delivery(entry: &Value) -> bool {
    entry
        .as_object()
        .and_then(|step| step.get("delivers"))
        .is_some_and(|found| !found.is_null())
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
        read_step_model(entry)?,
    )
    .dispatching(read_may_dispatch_jobs(entry)?)
    // **False where the key is absent, and `read_workflow` is what corrects a
    // row where every step is.** A step alone cannot tell "this step does not
    // deliver" from "nothing on this record could say", and the two need
    // opposite answers.
    .delivering(read_delivers(entry)?)
    .gating_on_every_check(read_gates_on_every_check(entry)?)
    .looping(read_verdict_routing(entry)?, read_iteration_cap(entry)?)
    .quiet_after(read_patience(entry, "quiet_after_seconds")?)
    .poking(read_patience(entry, "poke_limit")?))
}

/// Where the step's gate verdicts route. **Absent and null both read as no
/// loop**, which is every step of every linear workflow and every row frozen
/// before the keys existed.
///
/// A verdict this build does not have is a refusal rather than a route
/// dropped: a Job frozen with a loop that quietly came back linear would
/// advance past a gate the workflow said routes backwards, and nothing would
/// say so. A target is not checked against the step list here — `config`
/// refused a route naming no step, naming itself or pointing ahead where the
/// definition was parsed, and re-deciding it against a frozen record would be
/// a second reading of a rule that already ran.
fn read_verdict_routing(
    entry: &Map<String, Value>,
) -> Result<BTreeMap<GateVerdict, StepId>, Malformed> {
    let routing = match entry.get("verdict_routing") {
        None | Some(Value::Null) => return Ok(BTreeMap::new()),
        Some(Value::Object(routing)) => routing,
        Some(other) => return Err(format!("`verdict_routing` is {}", kind(other))),
    };
    let mut read = BTreeMap::new();
    for (word, target) in routing {
        let verdict = GateVerdict::from_wire(word)
            .ok_or_else(|| format!("`verdict_routing` holds the verdict `{word}`"))?;
        let Value::String(target) = target else {
            return Err(format!("`verdict_routing.{word}` is {}", kind(target)));
        };
        read.insert(verdict, StepId::new(target));
    }
    Ok(read)
}

/// How many passes the step's loop declared. **Absent and null both read as
/// zero**, which is the fail-closed default `ResolvedStep::looping` states: a
/// loop that was wired and not capped stops on its first return and says so.
///
/// **Not [`read_patience`], though the middle of the match is identical.** That
/// one answers `None` where the key is absent and this one answers `0`, and the
/// two mean opposite things: a step with no `quiet_after_seconds` is deferring
/// to Fleet's standing value, and a step with no `iteration_cap` is a step no
/// loop may go round. One reader would have to pick a default and would be
/// wrong for one of them.
///
/// A value that is there and is not a count is a refusal rather than a zero,
/// for [`read_retry_limit`]'s reason — a Job frozen with a cap must not lose it
/// quietly, because losing it is a Job that stops on a draft it was entitled to
/// redo.
fn read_iteration_cap(entry: &Map<String, Value>) -> Result<u32, Malformed> {
    match entry.get("iteration_cap") {
        None | Some(Value::Null) => Ok(0),
        Some(Value::Number(found)) => found
            .as_u64()
            .filter(|n| *n <= u64::from(u32::MAX))
            .and_then(|n| u32::try_from(n).ok())
            .ok_or_else(|| format!("`iteration_cap` holds `{found}`")),
        Some(other) => Err(format!("`iteration_cap` is {}", kind(other))),
    }
}

/// One half of the step's patience. **Absent and null both read as none**,
/// which is the backfill every row frozen before `#60` needs and is also what
/// a step that leaves the key out means: Fleet's standing value applies.
///
/// A value that is there and is not a count is a refusal rather than a none.
/// A step that asked for twenty minutes of patience and silently got two would
/// escalate as `stalled` in the middle of the work it declared itself slow for
/// — and the only trace would be a Job that stopped for no reason anyone could
/// find, which is the shape `#60` was opened about.
///
/// **One reader for both keys**, because the two are the same kind of number
/// read the same way. What is *not* shared is what they mean: they resolve
/// independently, and `fleet::Liveness::at` is where that happens.
fn read_patience(entry: &Map<String, Value>, key: &str) -> Result<Option<u32>, Malformed> {
    match entry.get(key) {
        None | Some(Value::Null) => Ok(None),
        Some(Value::Number(found)) => found
            .as_u64()
            .filter(|n| *n <= u64::from(u32::MAX))
            .and_then(|n| u32::try_from(n).ok())
            .map(Some)
            .ok_or_else(|| format!("`{key}` holds `{found}`")),
        Some(other) => Err(format!("`{key}` is {}", kind(other))),
    }
}

/// What the step asked to be run as. **Absent and null both read as none**,
/// which is the backfill every row frozen before a step could name a model
/// needs, and is also what a step that names none means today.
///
/// A value that is there and blank is a refusal rather than a none: `""` in
/// this column is a workflow that meant to say something, and a Drone spawned
/// on the Job's model instead would be the dial silently not applying.
fn read_step_model(entry: &Map<String, Value>) -> Result<Option<ModelName>, Malformed> {
    match entry.get("model") {
        None | Some(Value::Null) => Ok(None),
        Some(Value::String(named)) => Ok(Some(
            ModelName::new(named).map_err(|blank| format!("`model` {blank}"))?,
        )),
        Some(other) => Err(format!("`model` is {}", kind(other))),
    }
}

/// Whether a Drone on this step was given the tool that creates Jobs.
/// **Absent and null both read as `false`**, which is the backfill every row
/// frozen before the key existed needs and is also what a step that leaves the
/// key out means.
///
/// A value that is there and is not a boolean is a refusal rather than a
/// `false`: this key is what puts the dispatch tool in a Drone's hands, and a
/// step that quietly lost it is a Job that runs and can do nothing.
fn read_may_dispatch_jobs(entry: &Map<String, Value>) -> Result<bool, Malformed> {
    match entry.get("may_dispatch_jobs") {
        None | Some(Value::Null) => Ok(false),
        Some(Value::Bool(found)) => Ok(*found),
        Some(other) => Err(format!("`may_dispatch_jobs` is {}", kind(other))),
    }
}

/// Whether entering this step sends the work out. **Absent reads as `false`,
/// and that is only half the answer** — the other half is in
/// [`read_workflow`], which is the only scope that can tell a step declaring it
/// does not deliver from a row where nothing could declare anything.
///
/// A value that is there and is not a boolean is a refusal rather than a
/// `false`, for [`read_retry_limit`]'s reason: a Job frozen with a delivering
/// step must not lose it quietly, because losing it is a Job whose branch never
/// goes out and nothing saying why.
fn read_delivers(entry: &Map<String, Value>) -> Result<bool, Malformed> {
    match entry.get("delivers") {
        None | Some(Value::Null) => Ok(false),
        Some(Value::Bool(found)) => Ok(*found),
        Some(other) => Err(format!("`delivers` is {}", kind(other))),
    }
}

/// Whether the definition asked for every declared Check rather than naming
/// them. **Absent reads as `false` and needs no second half**, unlike
/// [`read_delivers`]: no workflow frozen before the key existed could say it,
/// so `false` is what those steps meant.
///
/// A value that is there and is not a boolean is a refusal rather than a
/// `false`, for [`read_may_dispatch_jobs`]'s reason: this is the only thing on
/// the record that tells a step gated on an empty Checks registry from a step
/// that declared no gate, and losing it quietly is losing the distinction.
fn read_gates_on_every_check(entry: &Map<String, Value>) -> Result<bool, Malformed> {
    match entry.get("gates_on_every_check") {
        None | Some(Value::Null) => Ok(false),
        Some(Value::Bool(found)) => Ok(*found),
        Some(other) => Err(format!("`gates_on_every_check` is {}", kind(other))),
    }
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
        // Absent reads as `Ask`, exactly as `config::judge` reads an absent
        // `on_refusal`: a row written before this field existed is a
        // criterion that asked by default in every version that could have
        // written it.
        let on_refusal = match criterion.get("on_refusal") {
            Some(Value::String(named)) => OnRefusal::from_wire(named).unwrap_or_default(),
            _ => OnRefusal::default(),
        };
        criteria.push(JudgeCriterion {
            criterion_id: CriterionId::new(text(criterion, "criterion_id")?),
            question: text(criterion, "question")?,
            on_refusal,
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
            requires: read_requires(entry)?,
            narrow: read_narrow(entry)?,
        }),
        DIFF_NONEMPTY => Ok(ResolvedCheck::DiffNonempty),
        ARTIFACT_EXISTS => Ok(ResolvedCheck::ArtifactExists {
            target: text(entry, "target")?,
        }),
        other => Err(format!("`type` holds `{other}`")),
    }
}

/// What a stored Check needs run before it, in the order the row wrote them.
///
/// **Absent and null both read as none**, which is a Check nothing runs first.
/// A row written before `requires` existed carries no key, and that is the same
/// sentence — the additive case this key is built to keep working.
///
/// **Order is the row's and is kept.** `[migrate, seed]` is a sequence, so the
/// list is read straight through rather than sorted or collected into a set.
///
/// An entry missing either half is malformed rather than repaired: a
/// prerequisite whose name was dropped fails a gate nobody could attribute, and
/// one whose command line was dropped runs nothing while reading as if it had.
fn read_requires(entry: &Map<String, Value>) -> Result<Vec<Prerequisite>, Malformed> {
    let Some(value) = entry.get("requires") else {
        return Ok(Vec::new());
    };
    if value.is_null() {
        return Ok(Vec::new());
    }
    let mut requires = Vec::new();
    for item in array(value)? {
        let needed = object(item)?;
        requires.push(Prerequisite::resolved(
            text(needed, "command")?,
            text(needed, "run")?,
        ));
    }
    Ok(requires)
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
    read_patterns(entry, "when")
}

/// A stored list of path patterns under one key. **One function for `when` and
/// for `narrow.from`**, which are the same list read the same way: absent means
/// always, and an empty list is refused rather than read as the opposite.
fn read_patterns(entry: &Map<String, Value>, key: &str) -> Result<Option<Covers>, Malformed> {
    let Some(value) = entry.get(key) else {
        return Ok(None);
    };
    if value.is_null() {
        return Ok(None);
    }
    let mut patterns = Vec::new();
    for item in array(value)? {
        let written = item
            .as_str()
            .ok_or_else(|| format!("`{key}` holds something that is not a pattern"))?;
        patterns.push(
            PathPattern::parse(written)
                .map_err(|why| format!("`{key}` holds `{written}`, which {why}"))?,
        );
    }
    // `Covers::of` answers `None` on an empty list, which would read back as
    // "always" — the opposite of what a stored empty list would have meant. It
    // cannot be written by `write_workflow`, and it is refused rather than
    // reinterpreted.
    Covers::of(patterns)
        .map(Some)
        .ok_or_else(|| format!("`{key}` is an empty list"))
}

/// What a stored Check narrows to, where it narrows at all.
///
/// **Absent and null both read as none**, which is a Check that runs whole —
/// the reading every row written before the key existed gets, and the same
/// additive shape `read_requires` has.
///
/// `run` and `each` missing is malformed rather than repaired: a narrowing with
/// no command to run would read as a Check that narrows and then runs nothing.
fn read_narrow(entry: &Map<String, Value>) -> Result<Option<Narrowing>, Malformed> {
    let Some(value) = entry.get("narrow") else {
        return Ok(None);
    };
    if value.is_null() {
        return Ok(None);
    }
    let narrowing = object(value)?;
    Ok(Some(Narrowing::declared(
        text(narrowing, "run")?,
        text(narrowing, "each")?,
        read_patterns(narrowing, "from")?,
        match narrowing.get("under") {
            None | Some(Value::Null) => None,
            Some(_) => Some(text(narrowing, "under")?),
        },
        match narrowing.get("except") {
            None | Some(Value::Null) => Vec::new(),
            Some(_) => texts(narrowing, "except")?,
        },
    )))
}

/// A definition's own version number. `u32` on the record, so a stored value
/// outside that range is malformed rather than clamped.
fn version(root: &Map<String, Value>) -> Result<u32, Malformed> {
    field(root, "version")?
        .as_u64()
        .and_then(|version| u32::try_from(version).ok())
        .ok_or_else(|| "`version` is not a version number".to_string())
}
