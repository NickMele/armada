//! One step of a workflow: what each key means, and where two of them disagree.
//!
//! **Absent is not a default.** Every optional key on [`Step`] says on its own
//! reader what leaving it out means, and none of them invents a value here.
//! `settings.toml` names three attempts per step as the Kit-level default and
//! there is no Kit-level anything for this parser to read it from; inventing
//! three here would put a threshold in the one place nobody looking at a
//! workflow would find it. The workflows under `.armada/workflows/` declare
//! their own, in the file, where an author reading the step sees what it costs.
//! An unknown model name is refused here against the roster the caller resolved
//! rather than carried to the spawn — [`crate::Roster`] for why.
//!
//! **Two refusals need every key at once**, which is what keeps them here
//! rather than on the reader of either key: a gate that names a tier declaring
//! nothing, and a criterion put to the Judge about a product the step does not
//! produce. Each is a step no Drone could pass, and each is refused where it is
//! written.

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
    "evidence_type",
    "mechanical_checks",
    "judge_checks",
    "advance_gate",
    "evidence_scope",
    "declare_plan_at",
    "retry_limit",
    "model",
    "may_dispatch_jobs",
    "verdict_routing",
    "iteration_cap",
    "quiet_after_seconds",
    "poke_limit",
];

const GATE_LEGAL: &[&str] = &[
    "auto",
    "auto_if_judge_passes",
    "human_always",
    "manifest_rule:<key>",
];
const GATE_CARRIED: &[&str] = &["auto", "auto_if_judge_passes", "human_always"];

const EVIDENCE_CARRIED: &[(&str, EvidenceType)] = &[
    ("diff", EvidenceType::Diff),
    ("failing_test", EvidenceType::FailingTest),
    ("facts_note", EvidenceType::FactsNote),
    ("test_suite_run", EvidenceType::TestSuiteRun),
    ("bundle", EvidenceType::Bundle),
    ("document", EvidenceType::Document),
];
const EVIDENCE_LEGAL: &[&str] = &[
    "diff",
    "failing_test",
    "facts_note",
    "test_suite_run",
    "bundle",
    "document",
];

/// One step of a workflow.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Step {
    id: StepId,
    label: String,
    evidence_type: Option<EvidenceType>,
    mechanical_checks: Vec<MechanicalCheck>,
    judge_checks: Vec<JudgeCheck>,
    advance_gate: AdvanceGate,
    evidence_scope: Option<EvidenceScope>,
    retry_limit: u32,
    model: Option<ModelName>,
    may_dispatch_jobs: bool,
    verdict_routing: BTreeMap<GateVerdict, StepId>,
    iteration_cap: Option<u32>,
    quiet_after_seconds: Option<u32>,
    poke_limit: Option<u32>,
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
    /// decides; the gate reader says why the fourth value is still refused, and
    /// the step reader why the agreement rule does not reach a human gate.
    pub fn advance_gate(&self) -> AdvanceGate {
        self.advance_gate
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
    let evidence_type = table.optional("evidence_type").and_then(|value| {
        yaml::word(
            &table.at("evidence_type"),
            value,
            EVIDENCE_CARRIED,
            EVIDENCE_LEGAL,
            EVIDENCE_LEGAL,
            out,
        )
    });
    let mechanical_checks = mechanical::checks(&mut table, out);
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
    if disagrees || (judged && evidence_type.is_none()) {
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
        mechanical_checks,
        judge_checks,
        advance_gate: advance_gate?,
        evidence_scope,
        retry_limit: retry_limit?,
        model,
        may_dispatch_jobs: may_dispatch_jobs?,
        verdict_routing: routing,
        iteration_cap,
        quiet_after_seconds,
        poke_limit,
    })
}

/// `advance_gate` has its own reader because one of the schema's four values is
/// a prefix form — `manifest_rule:<key>` — and a closed word list cannot match
/// it. Getting this wrong would refuse `manifest_rule:auto_merge` as a typo
/// when it is a real value M1 has not reached.
///
/// **That prefix form is the only one still refused.** It names a key resolved
/// against a Manifest-level policy and then across a Convoy's gating Manifests,
/// and neither is built — so reading it as the value the settings row happens to
/// default it to would be this parser answering a repository's question for it.
/// `human_always` needs none of that machinery: it names a person, and the acts
/// a person takes at the gate are `fleet::reviewing`'s and exist.
fn gate(at: &str, value: &Value, out: &mut Vec<Refusal>) -> Option<AdvanceGate> {
    let found = yaml::text(at, value, out)?;
    if found == "auto" {
        return Some(AdvanceGate::Auto);
    }
    if found == "auto_if_judge_passes" {
        return Some(AdvanceGate::AutoIfJudgePasses);
    }
    if found == "human_always" {
        return Some(AdvanceGate::HumanAlways);
    }
    // The prefix form and nothing else. A `manifest_rule:<key>` names a policy
    // that resolves per repository and across a Convoy's gating Manifests, and
    // reading it as the value that policy would most often produce would be
    // this parser deciding a repository's question for it.
    let deferred = found.starts_with("manifest_rule:");
    let fault = if deferred {
        Fault::NotYetCarried {
            value: found,
            carried: GATE_CARRIED,
        }
    } else {
        Fault::NotInTheSchema {
            value: found,
            legal: GATE_LEGAL,
        }
    };
    out.push(Refusal::new(at, fault));
    None
}
