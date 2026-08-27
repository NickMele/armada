//! A WorkflowDef, in the slice M1 reads.
//!
//! # Six fields, of nine
//!
//! `version`, `name`, `structure`, `steps[]`, and within a step `id`, `label`,
//! `evidence_type`, `mechanical_checks[]`, `judge_checks[]` and `advance_gate`.
//! The rest of the schema — `evidence_scope`, `retry_limit`, `verdict_routing`,
//! `iteration_cap`, `hard_prerequisite`, `default_gate_policy`, `on_fail`,
//! `on_gaming_flag` — is refused, because there is no retry ledger and no loop.
//! A field nothing reads is a promise the file makes and the system does not
//! keep. `evidence_scope` and `declare_plan_at` are read; see [`crate::scope`]
//! for the two keys inside the block that are not.
//!
//! # Three closed sets, each narrowed
//!
//! [`Structure`], `AdvanceGate` and [`MechanicalCheck`] each carry fewer
//! variants than the schema has, and each is an enum rather than a string
//! so that widening one is a compile error at every `match` that reads it. A
//! `String` here would widen silently. The two a Job freezes —
//! [`AdvanceGate`] and [`EvidenceType`] — are `core-model`'s, because the
//! record carries them.
//!
//! # `mechanical_checks` is absent more often than it is present
//!
//! Two of the four steps in this milestone step's worked example carry none,
//! and the concept page says a gateless step is the common case rather than an
//! edge one. So the field is optional and its absence is an empty list, not a
//! refusal — and [`Step::mechanical_checks`] returns a slice that is routinely
//! empty.
//!
//! # `advance_gate: auto` does not mean unverified
//!
//! It means the mechanical tier is the whole gate. Where a step also declares
//! `mechanical_checks`, **all entries must pass** — the array exists because
//! `implement` needs both that the build succeeded and that the diff is
//! non-empty, since a build passes cleanly on an empty diff and a Drone that
//! did nothing would otherwise advance on the build alone.

use std::collections::BTreeMap;
use std::path::{Path, PathBuf};

use core_model::{
    AdvanceGate, EvidenceScope, EvidenceType, JudgeCheck, StepId, Ulid, WorkflowId, DIFF_NONEMPTY,
    MANIFEST_CHECK,
};
use serde_yaml_ng::Value;

use crate::error::{Fault, LoadError, Refusal};
use crate::judge;
use crate::scope;
use crate::yaml::{self, Table};

/// The keys M1 reads at the top level of a WorkflowDef.
const TOP_LEVEL: &[&str] = &["version", "workflow_id", "name", "structure", "steps"];
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
];

/// How the steps are wired. **One variant, of two.**
///
/// `loop` is the schema's other value and M1 does not carry it: a loop returns
/// to an earlier step by `verdict_routing`, which needs a verdict, which needs
/// a Judge or a human gate. Neither exists yet, so a `loop` definition would
/// load and then have no way to close its own loop.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Structure {
    Linear,
}

const STRUCTURE_CARRIED: &[(&str, Structure)] = &[("linear", Structure::Linear)];
const STRUCTURE_LEGAL: &[&str] = &["linear", "loop"];
const STRUCTURE_M1: &[&str] = &["linear"];

const GATE_LEGAL: &[&str] = &[
    "auto",
    "auto_if_judge_passes",
    "human_always",
    "manifest_rule:<key>",
];
const GATE_CARRIED: &[&str] = &["auto", "auto_if_judge_passes"];

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

/// A deterministic assertion. **Two types, of five sanctioned.**
///
/// `artifact_exists`, `test_run` and `pr_merged` are the other three, and each
/// needs machinery M1 has not built — an artifact registry, a per-step test
/// invocation distinct from a named Check, and a merged pull request.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum MechanicalCheck {
    /// A named Check in the repo's `armada.yml` ran and exited as expected.
    ///
    /// `check` is a name, never a command string, so what actually runs is
    /// written in exactly one place. Whether that name exists is not this
    /// type's question — see [`crate::ResolvedWorkflow`].
    ManifestCheck {
        check: String,
        expect_exit_code: i64,
    },
    /// The step produced a non-empty diff.
    DiffNonempty,
}

const CHECK_TYPE_LEGAL: &[&str] = &[
    MANIFEST_CHECK,
    DIFF_NONEMPTY,
    "artifact_exists",
    "test_run",
    "pr_merged",
];
const CHECK_TYPE_M1: &[&str] = &[MANIFEST_CHECK, DIFF_NONEMPTY];
const MANIFEST_CHECK_KEYS: &[&str] = &["type", "check", "expect_exit_code"];
const DIFF_NONEMPTY_KEYS: &[&str] = &["type"];

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

    /// All entries must pass. Routinely empty.
    pub fn mechanical_checks(&self) -> &[MechanicalCheck] {
        &self.mechanical_checks
    }

    /// What the step asks the Judge. **Routinely empty**, which is what makes
    /// the semantic tier cold by default.
    pub fn judge_checks(&self) -> &[JudgeCheck] {
        &self.judge_checks
    }

    pub fn advance_gate(&self) -> AdvanceGate {
        self.advance_gate
    }

    /// What the step's evidence is scoped to. **`None` on a step that declared
    /// no block**, which is every step written before one existed.
    pub fn evidence_scope(&self) -> Option<&EvidenceScope> {
        self.evidence_scope.as_ref()
    }
}

/// A workflow definition, parsed and validated against nothing but itself.
///
/// **It is not dispatchable.** Its steps name Checks and this type has no way
/// to know whether those names resolve; [`crate::ResolvedWorkflow`] is the type
/// that has been checked against a Manifest, and it is the only one that can be
/// built from this.
#[derive(Debug, Clone)]
pub struct WorkflowDef {
    path: PathBuf,
    id: WorkflowId,
    version: u32,
    name: String,
    structure: Structure,
    steps: Vec<Step>,
}

impl WorkflowDef {
    /// Read and validate a workflow definition.
    pub fn load(path: &Path) -> Result<WorkflowDef, LoadError> {
        let text = std::fs::read_to_string(path).map_err(|cause| LoadError::Unreadable {
            path: path.to_path_buf(),
            cause,
        })?;
        WorkflowDef::parse(path, &text)
    }

    /// Validate a definition already in hand. See [`crate::Manifest::parse`].
    pub fn parse(path: &Path, text: &str) -> Result<WorkflowDef, LoadError> {
        let root: Value = serde_yaml_ng::from_str(text).map_err(|cause| LoadError::NotYaml {
            path: path.to_path_buf(),
            cause,
        })?;
        let mut out = Vec::new();
        let parsed = read(path, &root, &mut out);
        match parsed {
            Some(def) if out.is_empty() => Ok(def),
            _ => Err(LoadError::Refused {
                path: path.to_path_buf(),
                refusals: out,
            }),
        }
    }

    pub fn path(&self) -> &Path {
        &self.path
    }

    /// The definition's own id — **`workflow_id`, the sixth key.**
    ///
    /// Added because a proposal names a `workflow_id` and until this key
    /// existed there was nothing to join that id to: a Job could be proposed
    /// against a workflow invented at the keyboard, stored, and shown on the
    /// board claiming a workflow Fleet had never heard of.
    ///
    /// **Not a new spelling.** `domain/workflow-samples/bug.json` — the
    /// designed definition, which is the authority on the schema — already
    /// carries `workflow_id` at the top level, valued with the slug `bug`. M1's
    /// reduced form simply did not read it. So this reads the key the schema
    /// has, with the value that schema gives it, rather than minting an id
    /// nothing else would agree with.
    pub fn id(&self) -> &WorkflowId {
        &self.id
    }

    pub fn version(&self) -> u32 {
        self.version
    }

    pub fn name(&self) -> &str {
        &self.name
    }

    pub fn structure(&self) -> Structure {
        self.structure
    }

    /// The steps, in file order. **Order is the semantics** — there is no
    /// `order` field, because an array already has one and two statements of
    /// the same thing can disagree.
    pub fn steps(&self) -> &[Step] {
        &self.steps
    }
}

fn read(path: &Path, root: &Value, out: &mut Vec<Refusal>) -> Option<WorkflowDef> {
    let mut top = Table::open("", root, out)?;

    let version = top
        .required("version", out)
        .and_then(|value| yaml::positive("version", value, out));
    let id = top
        .required("workflow_id", out)
        .and_then(|value| yaml::text("workflow_id", value, out));
    let name = top
        .required("name", out)
        .and_then(|value| yaml::text("name", value, out));
    let structure = top.required("structure", out).and_then(|value| {
        yaml::word(
            "structure",
            value,
            STRUCTURE_CARRIED,
            STRUCTURE_LEGAL,
            STRUCTURE_M1,
            out,
        )
    });

    // Paired with the file position each step came from, because a step that
    // failed to parse is dropped and the duplicate-id report below has to name
    // the line the author wrote rather than the index in a shortened list.
    let placed: Vec<(usize, Step)> = top
        .required("steps", out)
        .and_then(|value| yaml::list("steps", value, out))
        .map(|items| {
            items
                .iter()
                .enumerate()
                .filter_map(|(n, (at, item))| Some((n, step(at, item, structure, out)?)))
                .collect()
        })
        .unwrap_or_default();
    top.close(TOP_LEVEL, out);

    // Duplicate step ids, reported on the second occurrence and naming the
    // first. Every per-step counter in the system is keyed by this value, so
    // two steps sharing one id would share a retry budget and a verdict.
    let mut first_seen: BTreeMap<&str, usize> = BTreeMap::new();
    for (n, step) in &placed {
        match first_seen.get(step.id.as_str()) {
            Some(first_at) => out.push(Refusal::new(
                format!("steps[{n}].id"),
                Fault::DuplicateStepId {
                    first_at: *first_at,
                },
            )),
            None => {
                first_seen.insert(step.id.as_str(), *n);
            }
        }
    }
    let steps: Vec<Step> = placed.iter().map(|(_, step)| step.clone()).collect();

    Some(WorkflowDef {
        path: path.to_path_buf(),
        id: WorkflowId::carried(Ulid::carried(id?)),
        version: version?,
        name: name?,
        structure: structure?,
        steps,
    })
}

fn step(
    at: &str,
    value: &Value,
    structure: Option<Structure>,
    out: &mut Vec<Refusal>,
) -> Option<Step> {
    let mut table = Table::open(at, value, out)?;

    // `verdict_routing` is the one deferred key with a refusal of its own. As
    // an unknown key it would read as "M1 does not do that yet", when on a
    // linear workflow it is wrong at every milestone: the declared structure
    // and the wiring disagree, and the file says so about itself.
    if structure == Some(Structure::Linear) && table.present("verdict_routing") {
        table.ignore("verdict_routing");
        out.push(Refusal::new(
            format!("{at}.verdict_routing"),
            Fault::ContradictsStructure {
                structure: "linear",
            },
        ));
    }

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
    let mechanical_checks = table
        .optional("mechanical_checks")
        .and_then(|value| yaml::list(&table.at("mechanical_checks"), value, out))
        .map(|items| {
            items
                .iter()
                .filter_map(|(at, item)| mechanical_check(at, item, out))
                .collect::<Vec<MechanicalCheck>>()
        })
        .unwrap_or_default();
    let judge_checks = judge::checks(&mut table, out);
    let evidence_scope = scope::evidence_scope(&mut table, out);
    let gate_key = table.at("advance_gate");
    let advance_gate = table
        .required("advance_gate", out)
        .and_then(|value| gate(&gate_key, value, out));

    // The gate and the checks are two statements of one thing, so a file that
    // makes them disagree is refused rather than resolved. A step gated on a
    // Judge that asks nothing would advance on the mechanical tier alone while
    // reading as verified; a step that asks and is gated `auto` spends money on
    // an answer nothing reads.
    let judged = judge_checks.iter().any(JudgeCheck::fires);
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
    if disagrees {
        return None;
    }

    Some(Step {
        id: StepId::new(id?),
        label: label?,
        evidence_type,
        mechanical_checks,
        judge_checks,
        advance_gate: advance_gate?,
        evidence_scope,
    })
}

/// `advance_gate` has its own reader because one of the schema's four values is
/// a prefix form — `manifest_rule:<key>` — and a closed word list cannot match
/// it. Getting this wrong would refuse `manifest_rule:auto_merge` as a typo
/// when it is a real value M1 has not reached.
fn gate(at: &str, value: &Value, out: &mut Vec<Refusal>) -> Option<AdvanceGate> {
    let found = yaml::text(at, value, out)?;
    if found == "auto" {
        return Some(AdvanceGate::Auto);
    }
    if found == "auto_if_judge_passes" {
        return Some(AdvanceGate::AutoIfJudgePasses);
    }
    let deferred = found.starts_with("manifest_rule:") || found == "human_always";
    let fault = if deferred {
        Fault::OutsideM1 {
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

fn mechanical_check(at: &str, value: &Value, out: &mut Vec<Refusal>) -> Option<MechanicalCheck> {
    let mut table = Table::open(at, value, out)?;
    let kind = table
        .required("type", out)
        .and_then(|value| yaml::text(&table.at("type"), value, out))?;

    match kind.as_str() {
        MANIFEST_CHECK => {
            let check = table
                .required("check", out)
                .and_then(|value| yaml::text(&table.at("check"), value, out));
            let expect_exit_code = table
                .required("expect_exit_code", out)
                .and_then(|value| yaml::integer(&table.at("expect_exit_code"), value, out));
            table.close(MANIFEST_CHECK_KEYS, out);
            Some(MechanicalCheck::ManifestCheck {
                check: check?,
                expect_exit_code: expect_exit_code?,
            })
        }
        DIFF_NONEMPTY => {
            table.close(DIFF_NONEMPTY_KEYS, out);
            Some(MechanicalCheck::DiffNonempty)
        }
        other => {
            let fault = if CHECK_TYPE_LEGAL.contains(&other) {
                Fault::OutsideM1 {
                    value: kind.clone(),
                    carried: CHECK_TYPE_M1,
                }
            } else {
                Fault::NotInTheSchema {
                    value: kind.clone(),
                    legal: CHECK_TYPE_LEGAL,
                }
            };
            out.push(Refusal::new(table.at("type"), fault));
            None
        }
    }
}
