//! One step of a workflow: what each key means, and where two of them disagree.
//!
//! **Absent is not a default.** Every optional key on [`Step`] says on its own
//! reader what leaving it out means. `settings.toml` names three attempts per
//! step as the Kit-level default, and there is no Kit-level anything for this
//! parser to read it from — inventing three here would put a threshold in the
//! one place nobody looking at a workflow would find it. An unknown model name
//! is refused here against the roster the caller resolved rather than carried
//! to the spawn — [`crate::Roster`] for why.
//!
//! **Two refusals need every key at once**, which is what keeps them here
//! rather than on the reader of either key: a gate that names a tier declaring
//! nothing, and a criterion put to the Judge about a product the step does not
//! produce.

use std::collections::BTreeMap;

use core_model::{
    AdvanceGate, EvidenceScope, EvidenceType, GateVerdict, JudgeCheck, ModelName, StepId,
};
use serde_yaml_ng::Value;

use super::mechanical::{self, MechanicalCheck};
use super::Structure;
use crate::error::{Fault, Refusal};
use crate::judge;
use crate::loops::{self, Looping};
use crate::roster::{self, Roster};
use crate::scope;
use crate::yaml::{self, Table};

/// The keys M1 reads inside a step.
const STEP_KEYS: &[&str] = &[
    "id",
    "label",
    "evidence",
    "mechanical_checks",
    "judge_checks",
    "advance_gate",
    "delivers",
    "evidence_scope",
    "declare_plan_at",
    "retry_limit",
    "model",
    "may_dispatch_jobs",
    "verdict_routing",
    "iteration_cap",
    "quiet_after_seconds",
    "poke_limit",
    "follows_plan",
    "records_plan",
];

/// **The schema's whole set, spelled out rather than sketched.** This held
/// `manifest_rule:<key>` while the form was deferred, which was the only honest
/// thing to print about a value no key was carried for. Now that both keys the
/// registry names are carried, a refusal at this key can say what may be
/// written — and `manifest_rule:nonsense` is refused as the typo it is rather
/// than as a feature that has not landed.
///
/// **The carried set has converged on it**, so [`Fault::NotYetCarried`] is
/// unreachable at this key and the second list is gone — `structure` kept its
/// pair because [`yaml::word`] takes both as arguments, and this reader takes
/// neither.
const GATE_LEGAL: &[&str] = &[
    "auto",
    "auto_if_judge_passes",
    "human_always",
    "manifest_rule:auto_merge",
    "manifest_rule:review_gate",
];

/// **Every value is a claim a Drone hands in and the gate measures**, which is
/// what `shown` never was: it meant *run the repository's harness*, and an
/// instruction in a list of claims is why one key could not say both. `captured`
/// says it now, beside this rather than among it — `#777`.
const EVIDENCE_CARRIED: &[(&str, EvidenceType)] = &[
    ("diff", EvidenceType::Diff),
    ("failing_test", EvidenceType::FailingTest),
    ("facts_note", EvidenceType::FactsNote),
    ("test_suite_run", EvidenceType::TestSuiteRun),
    ("bundle", EvidenceType::Bundle),
    ("document", EvidenceType::Document),
    ("plan", EvidenceType::Plan),
    ("review", EvidenceType::Review),
];
const EVIDENCE_LEGAL: &[&str] = &[
    "diff",
    "failing_test",
    "facts_note",
    "test_suite_run",
    "bundle",
    "document",
    "plan",
    "review",
];

/// The whole of the `evidence` block, and of the `submitted` object inside it.
const EVIDENCE_KEYS: &[&str] = &["submitted", "captured"];
const SUBMITTED_KEYS: &[&str] = &["type"];

/// One step of a workflow.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Step {
    id: StepId,
    label: String,
    evidence_type: Option<EvidenceType>,
    captured: bool,
    mechanical_checks: Vec<MechanicalCheck>,
    judge_checks: Vec<JudgeCheck>,
    advance_gate: AdvanceGate,
    delivers: bool,
    evidence_scope: Option<EvidenceScope>,
    retry_limit: u32,
    model: Option<ModelName>,
    may_dispatch_jobs: bool,
    verdict_routing: BTreeMap<GateVerdict, StepId>,
    iteration_cap: Option<u32>,
    quiet_after_seconds: Option<u32>,
    poke_limit: Option<u32>,
    follows_plan: bool,
    records_plan: bool,
}

impl Step {
    /// The stable key. Carried by Evidence, by events and by `current_step`;
    /// per-step counters are keyed to it, which is why renaming one is a
    /// different act from relabelling one.
    pub fn id(&self) -> &StepId {
        &self.id
    }

    /// Display only. **Nothing routes on it**, which is the whole reason it is
    /// a second field rather than a prettier `id`.
    pub fn label(&self) -> &str {
        &self.label
    }

    pub fn evidence_type(&self) -> Option<EvidenceType> {
        self.evidence_type
    }

    /// Whether Fleet runs the repository's `evidence:` harness for this step.
    /// **False where the block says nothing**, which is every step that asks to
    /// be read rather than looked at.
    pub fn captured(&self) -> bool {
        self.captured
    }

    /// All entries must pass. **Routinely empty** — a gateless step is the
    /// common case rather than an edge one, so absence is an empty list rather
    /// than a refusal.
    ///
    /// `implement` is why the field is an array: it needs both that the build
    /// succeeded and that the diff is non-empty, because a build passes cleanly
    /// on an empty diff and a Drone that did nothing would otherwise advance on
    /// the build alone.
    pub fn mechanical_checks(&self) -> &[MechanicalCheck] {
        &self.mechanical_checks
    }

    /// What the step asks the Judge. **Routinely empty**, which is what makes
    /// the semantic tier cold by default.
    pub fn judge_checks(&self) -> &[JudgeCheck] {
        &self.judge_checks
    }

    /// How many times a failed mechanical gate hands this step back to its
    /// Drone. **Zero where the file declares none**, which is what every step
    /// meant before the key was read.
    ///
    /// The ledger it is spent against is `store::attempt`, which counts a
    /// step's runs off its own log; `fleet::gate` is what asks.
    pub fn retry_limit(&self) -> u32 {
        self.retry_limit
    }

    /// **`auto` does not mean unverified.** It means the mechanical tier is the
    /// whole gate. `human_always` means the tiers still run and a person
    /// decides; the gate reader says why a `manifest_rule:` value is carried
    /// unresolved, and the step reader why the agreement rule reaches neither
    /// it nor a human gate.
    pub fn advance_gate(&self) -> AdvanceGate {
        self.advance_gate
    }

    /// Whether entering this step sends the work out — committed, pushed, and
    /// opened for review. **Required on every step, so there is no absent
    /// reading here**, and true on at most one step of a workflow.
    ///
    /// The step that sends the work out is the step that then holds while a
    /// person reads what went out, which is why this is a key on a step rather
    /// than on the definition.
    pub fn delivers(&self) -> bool {
        self.delivers
    }

    /// What the step's evidence is scoped to. **`None` on a step that declared
    /// no block**, which is every step written before one existed.
    pub fn evidence_scope(&self) -> Option<&EvidenceScope> {
        self.evidence_scope.as_ref()
    }

    /// What this step asks its Drone to be spawned as. **`None` on most
    /// steps**, and none means the Job's — the fallback is spelled on the
    /// record, at `Job::model_at`, and never re-derived here.
    pub fn model(&self) -> Option<&ModelName> {
        self.model.as_ref()
    }

    /// Whether a Drone on this step is given the tool that creates Jobs.
    /// **False on every step that leaves the key out**, which is every step of
    /// every workflow that creates none.
    pub fn may_dispatch_jobs(&self) -> bool {
        self.may_dispatch_jobs
    }

    /// Where this step goes on a verdict that neither advances nor ends.
    /// **Empty on every step of a linear workflow**, and empty is what makes a
    /// step's only exit forward. This is the edge that declares the loop —
    /// `structure` only labels it, which is why the two are checked against
    /// each other.
    pub fn verdict_routing(&self) -> &BTreeMap<GateVerdict, StepId> {
        &self.verdict_routing
    }

    /// How many times this step may be returned to before the Job escalates as
    /// `loop_cap`. **`None` where the step declares none**, which is not
    /// unbounded and not a number invented here: the schema defaults it from
    /// `default_gate_policy.iteration_cap`, that block is still refused as
    /// deferred, and inventing a ceiling at this layer would put the bound on a
    /// Job in the one place nobody reading the workflow would look for it.
    ///
    /// **Not [`Step::retry_limit`], and never spent against it.** A retry is a
    /// step that failed going again; an iteration is a step that was asked for
    /// another draft.
    pub fn iteration_cap(&self) -> Option<u32> {
        self.iteration_cap
    }

    /// How long this step's Drone may say nothing before Fleet pokes it, in
    /// seconds. **`None` where the file declares none**, which is Fleet's
    /// standing value — the fallback is spelled at `fleet::Liveness::at` and
    /// never re-derived here, for [`model`](Step::model)'s reason.
    pub fn quiet_after_seconds(&self) -> Option<u32> {
        self.quiet_after_seconds
    }

    /// How many nudges this step's quiet Drone gets. **`None` where the file
    /// declares none**, and read independently of
    /// [`quiet_after_seconds`](Step::quiet_after_seconds) because the two fall
    /// back independently: a step that wants longer between pokes does not
    /// thereby want more pokes.
    pub fn poke_limit(&self) -> Option<u32> {
        self.poke_limit
    }

    /// Whether this step works the Job's plan. **False where the file leaves
    /// the key out**, which is every step of every workflow written before it
    /// existed.
    pub fn follows_plan(&self) -> bool {
        self.follows_plan
    }

    /// Whether this step records the Job's plan **beside its own product**,
    /// declared with `records_plan: true` rather than inferred from
    /// `evidence.submitted.type`. **False where the file leaves the key
    /// out** — which includes every step whose product is `plan` outright:
    /// that case is carried on [`Step::evidence_type`] and read together with
    /// this one wherever "does this step record the plan" is the question,
    /// never through this field alone. `#1006`.
    pub fn records_plan(&self) -> bool {
        self.records_plan
    }
}

/// One step, or [`None`] where something on it was refused.
///
/// `structure` is the workflow's, and is a parameter for [`loops::looping`]'s
/// reason: what the routing map means is a fact about the file rather than
/// about the step.
pub(super) fn read(
    at: &str,
    value: &Value,
    structure: Option<Structure>,
    roster: &Roster,
    out: &mut Vec<Refusal>,
) -> Option<Step> {
    let mut table = Table::open(at, value, out)?;

    // Read first, because the routing map's refusal depends on the structure
    // the file declared and every other key on the step does not.
    let looping = loops::looping(&mut table, structure == Some(Structure::Linear), out);

    let id = table
        .required("id", out)
        .and_then(|value| yaml::text(&table.at("id"), value, out));
    let label = table
        .required("label", out)
        .and_then(|value| yaml::text(&table.at("label"), value, out));
    let (evidence_type, captured) = evidence(&mut table, out);
    // **Absent is false, and anything that is not a boolean is a refusal** —
    // `may_dispatch_jobs`'s rule, for its reason: a value read as absent
    // would be a step written to record the plan beside its own product that
    // silently does not. A step whose *whole* product is the plan does not
    // need this key at all — `evidence.submitted.type: "plan"` already says
    // so, unchanged, and `is_plan_step` below reads both together so the two
    // spellings converge on one answer wherever the question is asked. `#1006`.
    let records_plan_key = table.at("records_plan");
    let records_plan = match table.optional("records_plan") {
        None => Some(false),
        Some(value) => yaml::flag(&records_plan_key, value, out),
    };
    // **Whether this step records the plan at all**, folding the declared
    // field and the inferred case into the one answer everything below asks
    // for. A step cannot both leave `records_plan` unset and have it read as
    // true, so a refused `records_plan` reads as `false` here exactly as
    // `model` and `follows_plan` already do — the refusal is already in
    // `out`, and a definition carrying one does not load at all.
    let is_plan_step = records_plan.unwrap_or(false) || evidence_type == Some(EvidenceType::Plan);
    let mechanical_checks = mechanical::checks(&mut table, is_plan_step, out);
    let judge_checks = judge::checks(&mut table, roster, out);
    let evidence_scope = scope::evidence_scope(&mut table, out);
    // **Absent is none, and a malformed one is a refusal rather than none.**
    // A file that writes `retry_limit: "three"` meant to buy retries, and
    // silently giving it zero would be the parser deciding the budget.
    let retry_key = table.at("retry_limit");
    let retry_limit = match table.optional("retry_limit") {
        None => Some(0),
        Some(value) => yaml::counted(&retry_key, value, out),
    };
    // **Absent is the Job's, and a name the machine does not have is a
    // refusal.** Absent has to stay absent all the way to the record — see
    // `ResolvedStep::model` — and the refusal has to be here rather than at
    // spawn, which is what `crate::roster` exists to say.
    let model_key = table.at("model");
    let model = table
        .optional("model")
        .and_then(|value| yaml::text(&model_key, value, out))
        .and_then(|named| roster::offered(&model_key, named, roster, out));
    // **Absent is false, and anything that is not a boolean is a refusal.**
    // This key is what puts the dispatch tool in a Drone's hands, so a value
    // read as absent would be a step written to create Jobs that silently
    // cannot — and a Job that goes quiet is an argument-list fault, which is
    // the hardest kind to see.
    let grant_key = table.at("may_dispatch_jobs");
    let may_dispatch_jobs = match table.optional("may_dispatch_jobs") {
        None => Some(false),
        Some(value) => yaml::flag(&grant_key, value, out),
    };
    // **Absent is Fleet's, and absent has to stay absent all the way to the
    // record** — `ResolvedStep::quiet_after_seconds` says why. A step written
    // with the number Fleet happens to ship would be a second place that
    // number lives, and it would freeze a value marked live.
    //
    // **`positive`, so zero is refused rather than carried.** A
    // `quiet_after_seconds: 0` is a step whose Drone is quiet the instant it is
    // spawned, which pokes it on the first turn and escalates it on the third
    // — a sentence nobody means, and the two keys disagree about zero for
    // exactly the reason `counted` and `positive` were split.
    //
    // A refused value reads as absent from here, which is `model`'s
    // arrangement and is safe for `model`'s reason: the refusal is already in
    // `out`, and a definition with any refusal in it does not load at all.
    let quiet_key = table.at("quiet_after_seconds");
    let quiet_after_seconds = table
        .optional("quiet_after_seconds")
        .and_then(|value| yaml::positive(&quiet_key, value, out));
    // **`counted`, because zero is a sentence here.** A step with
    // `poke_limit: 0` says its Drone gets no nudge at all and the first
    // silence past the threshold escalates, which is a legitimate thing to ask
    // for on a step where a poke costs a model run and buys nothing.
    let poke_key = table.at("poke_limit");
    let poke_limit = table
        .optional("poke_limit")
        .and_then(|value| yaml::counted(&poke_key, value, out));
    // **Absent is false, and anything that is not a boolean is a refusal** —
    // `may_dispatch_jobs`'s rule, for its reason: a value read as absent would
    // be a step written to work the plan that silently is not given the
    // tools. Whether this step may say so at all — a plan strictly earlier —
    // is a fact about the whole workflow and is refused in `super::workflow`.
    let follows_key = table.at("follows_plan");
    let follows_plan = match table.optional("follows_plan") {
        None => Some(false),
        Some(value) => yaml::flag(&follows_key, value, out),
    };
    // **Required, and there is no default to fall back to.** A file that does
    // not say whether a step sends the work out has two readings and neither is
    // safe: taken as delivering, a workflow that produces a document opens a
    // pull request for it; taken as not, every workflow already written stops
    // pushing a branch and nothing says so. The registry row carries the trade.
    let delivers_key = table.at("delivers");
    let delivers = table
        .required("delivers", out)
        .and_then(|value| yaml::flag(&delivers_key, value, out));
    let gate_key = table.at("advance_gate");
    let advance_gate = table
        .required("advance_gate", out)
        .and_then(|value| gate(&gate_key, value, out));

    // The gate and the checks are two statements of one thing, so a file that
    // makes them disagree is refused rather than resolved. A step gated on a
    // Judge that asks nothing would advance on the mechanical tier alone while
    // reading as verified; a step that asks and is gated `auto` spends money on
    // an answer nothing reads.
    //
    // **`human_always` is in neither disagreement, in either direction**, and
    // that is not an exemption carved for it — it is the rule's own premise not
    // applying. The rule compares a gate that names a *tier* against that
    // tier's declaration. `human_always` names an actor: a person answers, and
    // no tier decides. So neither failure it detects can arise. Nothing advances
    // on the mechanical tier alone while reading as verified, because nothing
    // advances on the machine's say-so at all. And a Judge here is not spent on
    // an answer nothing reads — a refusal stops the step before the work reaches
    // a person, and a criterion it did not refuse is written down beside the
    // evidence they open. The designed samples declare a human gate both ways —
    // Feature's and Revert's `review` carry a Judge, Design Plan's `present` and
    // Prototype's `build` carry none — and both are legitimate, which is what
    // separates this from `auto` with a Judge, where the answer has nowhere to
    // go at all.
    let judged = judge_checks.iter().any(JudgeCheck::fires);

    // **A blind judge check is refused where it is written, not discovered on a
    // Job.** The Judge is shown what the step produced, and what a step produces
    // is `evidence_type`. A step declaring none produces nothing, so a criterion
    // on it is a call made against an empty page — which is not a refusal the
    // Drone can retry against and not a pass either.
    //
    // This is the narrowest form of the rule that is true at parse time. Whether
    // a `diff` step will actually have a diff is a runtime fact, answered by
    // `verification::Product::of` and carried as a call that could not be made.
    // Whether a step has anything at all is knowable from the file, and four
    // checks that did not landed in one commit.
    if judged && evidence_type.is_none() {
        out.push(Refusal::new(
            format!("{at}.judge_checks"),
            Fault::JudgedWithNothingToShow,
        ));
    }

    // **A step that records the plan is not done until Fleet's own record
    // says so.** `plan_recorded` is the only check that reads that record, so
    // a step recording the plan — whether its whole product is `plan` or it
    // declared `records_plan: true` beside another product — and declaring
    // none of these has nothing gating the one thing its Judge and every
    // following step will trust. The reverse — `plan_recorded` on a step that
    // does not record the plan — is refused where the check itself is read,
    // in `super::mechanical`.
    let has_plan_recorded = mechanical_checks
        .iter()
        .any(|check| matches!(check, MechanicalCheck::PlanRecorded { .. }));
    if is_plan_step && !has_plan_recorded {
        out.push(Refusal::new(
            format!("{at}.mechanical_checks"),
            Fault::PlanStepWithoutPlanRecorded,
        ));
    }

    // **A `manifest_rule:` gate is outside the rule too, and not for
    // `human_always`'s reason.** That one names an actor, so neither failure
    // can arise; this one names a policy whose value is not in this file, so
    // neither failure can be *established* from it. The designed Code Review
    // declares `manifest_rule:review_gate` on a step carrying no Judge, which
    // is legal and which a repository setting `auto_if_judge_passes` turns
    // into the first failure. Refusing it here would refuse a shape the design
    // sanctions; `#525` is where a resolved policy meets a step's declaration.
    let disagrees = matches!(
        (advance_gate, judged),
        (Some(AdvanceGate::AutoIfJudgePasses), false) | (Some(AdvanceGate::Auto), true)
    );
    if disagrees {
        out.push(Refusal::new(
            &gate_key,
            Fault::GateAndJudgeDisagree {
                gate: advance_gate.map(|gate| gate.as_wire()).unwrap_or("auto"),
            },
        ));
    }
    table.close(STEP_KEYS, out);
    if disagrees || (judged && evidence_type.is_none()) || (is_plan_step && !has_plan_recorded) {
        return None;
    }
    let Looping {
        routing,
        iteration_cap,
    } = looping?;

    Some(Step {
        id: StepId::new(id?),
        label: label?,
        evidence_type,
        captured: captured?,
        mechanical_checks,
        judge_checks,
        advance_gate: advance_gate?,
        delivers: delivers?,
        evidence_scope,
        retry_limit: retry_limit?,
        model,
        may_dispatch_jobs: may_dispatch_jobs?,
        verdict_routing: routing,
        iteration_cap,
        quiet_after_seconds,
        poke_limit,
        follows_plan: follows_plan?,
        records_plan: records_plan?,
    })
}

/// The `evidence` block: what the Drone hands in, and whether Fleet captures.
///
/// **Two keys because they are two questions**, and holding them in one value
/// is exactly what stopped a step handing in a patch *and* being captured. The
/// submitted half is a claim the gate measures; `captured` is an instruction to
/// Fleet that gates nothing. `#777`.
///
/// **Absent is neither**, which is every step that produces nothing a Judge
/// reads. A block present but empty is the same answer said out loud.
fn evidence(table: &mut Table<'_>, out: &mut Vec<Refusal>) -> (Option<EvidenceType>, Option<bool>) {
    let at = table.at("evidence");
    let Some(value) = table.optional("evidence") else {
        return (None, Some(false));
    };
    let Some(mut block) = Table::open(&at, value, out) else {
        return (None, None);
    };
    let submitted_at = block.at("submitted");
    let submitted = block.optional("submitted").and_then(|value| {
        let mut inner = Table::open(&submitted_at, value, out)?;
        let type_at = inner.at("type");
        // **Required inside the block, because writing `submitted:` is asking
        // for something.** A step handing nothing in leaves the key out.
        let kind = inner.required("type", out).and_then(|value| {
            yaml::word(
                &type_at,
                value,
                EVIDENCE_CARRIED,
                EVIDENCE_LEGAL,
                EVIDENCE_LEGAL,
                out,
            )
        });
        inner.close(SUBMITTED_KEYS, out);
        kind
    });
    // **Absent is false, and anything that is not a boolean is a refusal** —
    // `may_dispatch_jobs`'s rule, for its reason: a value read as absent would
    // be a step written to be captured that silently is not.
    let captured_at = block.at("captured");
    let captured = match block.optional("captured") {
        None => Some(false),
        Some(value) => yaml::flag(&captured_at, value, out),
    };
    block.close(EVIDENCE_KEYS, out);
    (submitted, captured)
}

/// `advance_gate` has its own reader because one of the schema's four forms is
/// a prefix form — `manifest_rule:<key>` — and the key is what the value means.
/// A closed word list over the whole spelling would still work, and this is not
/// one: the prefix is matched first so a key nothing defines names *itself* in
/// the refusal rather than reading as an unrecognised gate.
///
/// **Nothing here resolves the key**, and `AdvanceGate` carries it through to
/// the record for that reason. What a `manifest_rule:` gate settles on is the
/// repository's, read where the gate is read — resolving it at parse time would
/// be this file answering a repository's question, and answering it once for
/// every Job at daemon start, when the settings row calls the policy `Live`.
fn gate(at: &str, value: &Value, out: &mut Vec<Refusal>) -> Option<AdvanceGate> {
    let found = yaml::text(at, value, out)?;
    // **A key the registry does not name is refused by name.** `auto_merge` and
    // `review_gate` are the two `workflowdef-fields.toml` declares, and a third
    // is a row there before it is a variant — so `manifest_rule:nonsense` is a
    // typo with a spelling to correct, not a policy waiting to be built.
    if let Some(key) = found.strip_prefix("manifest_rule:") {
        return match key {
            "auto_merge" => Some(AdvanceGate::ManifestRuleAutoMerge),
            "review_gate" => Some(AdvanceGate::ManifestRuleReviewGate),
            _ => {
                out.push(Refusal::new(
                    at,
                    Fault::NotInTheSchema {
                        value: found,
                        legal: GATE_LEGAL,
                    },
                ));
                None
            }
        };
    }
    if found == "auto" {
        return Some(AdvanceGate::Auto);
    }
    if found == "auto_if_judge_passes" {
        return Some(AdvanceGate::AutoIfJudgePasses);
    }
    if found == "human_always" {
        return Some(AdvanceGate::HumanAlways);
    }
    out.push(Refusal::new(
        at,
        Fault::NotInTheSchema {
            value: found,
            legal: GATE_LEGAL,
        },
    ));
    None
}
