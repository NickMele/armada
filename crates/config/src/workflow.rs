//! A WorkflowDef, in the slice M1 reads.
//!
//! **A field nothing reads is a promise the file makes and the system does not
//! keep**, so this parser refuses one rather than ignoring it. `verdict_routing`,
//! `iteration_cap`, `hard_prerequisite`, `default_gate_policy`, `on_fail` and
//! `on_gaming_flag` are refused because there is no loop; `evidence_scope` and
//! `declare_plan_at` are read, and [`crate::scope`] holds the two keys inside
//! that block that are not.
//!
//! **Three closed sets, each narrowed.** [`Structure`], `AdvanceGate` and
//! [`MechanicalCheck`] carry fewer variants than the schema has, and each is an
//! enum rather than a `String` so that widening one is a compile error at every
//! `match` reading it. A `String` would widen silently. The two a Job freezes —
//! `AdvanceGate` and [`EvidenceType`] — are `core-model`'s, because the record
//! carries them.
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

use std::collections::BTreeMap;
use std::path::{Path, PathBuf};

use core_model::{
    AdvanceGate, EvidenceScope, EvidenceType, JudgeCheck, ModelName, StepId, Ulid, WorkflowId,
    ARTIFACT_EXISTS, DIFF_NONEMPTY, MANIFEST_CHECK,
};
use serde_yaml_ng::Value;

use crate::error::{BadTarget, Fault, LoadError, Refusal};
use crate::judge;
use crate::roster::{self, Roster};
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
    "retry_limit",
    "model",
    "may_dispatch_jobs",
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

/// A deterministic assertion. **Three types, of five sanctioned.**
///
/// `test_run` and `pr_merged` are the other two, and each needs machinery M1
/// has not built — a per-step test invocation distinct from a named Check, and
/// a merged pull request.
///
/// **`artifact_exists` is carried now, and it carries a path rather than a
/// name.** The schema's samples name a registry entry — `root_cause_note` —
/// and there is no artifact registry to resolve one against. What there is, on
/// every step, is a worktree Fleet reads for itself, so the target is the
/// worktree-relative path of the file the step writes. That is also what makes
/// a step's product something the next step can open rather than a sentence the
/// last Drone typed about it.
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
    /// The step wrote the file named by `target`.
    ArtifactExists { target: String },
}

const CHECK_TYPE_LEGAL: &[&str] = &[
    MANIFEST_CHECK,
    DIFF_NONEMPTY,
    ARTIFACT_EXISTS,
    "test_run",
    "pr_merged",
];
const CHECK_TYPE_M1: &[&str] = &[MANIFEST_CHECK, DIFF_NONEMPTY, ARTIFACT_EXISTS];
const MANIFEST_CHECK_KEYS: &[&str] = &["type", "check", "expect_exit_code"];
const DIFF_NONEMPTY_KEYS: &[&str] = &["type"];
const ARTIFACT_EXISTS_KEYS: &[&str] = &["type", "target"];

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
    ///
    /// `roster` is what this machine can run a Drone as. See
    /// [`crate::Roster`] for why the list is a parameter rather than something
    /// this crate knows.
    pub fn load(path: &Path, roster: &Roster) -> Result<WorkflowDef, LoadError> {
        let text = std::fs::read_to_string(path).map_err(|cause| LoadError::Unreadable {
            path: path.to_path_buf(),
            cause,
        })?;
        WorkflowDef::parse(path, &text, roster)
    }

    /// Validate a definition already in hand. See [`crate::Manifest::parse`].
    pub fn parse(path: &Path, text: &str, roster: &Roster) -> Result<WorkflowDef, LoadError> {
        let root: Value = serde_yaml_ng::from_str(text).map_err(|cause| LoadError::NotYaml {
            path: path.to_path_buf(),
            cause,
        })?;
        let mut out = Vec::new();
        let parsed = read(path, &root, roster, &mut out);
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

fn read(path: &Path, root: &Value, roster: &Roster, out: &mut Vec<Refusal>) -> Option<WorkflowDef> {
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
                .filter_map(|(n, (at, item))| Some((n, step(at, item, structure, roster, out)?)))
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
    roster: &Roster,
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
            // **The second `artifact_exists` is refused, not dropped.** A step
            // has one deliverable — Fleet reads it into the Judge's brief as
            // the document the step produced — so two would be a choice made by
            // whichever the reader happened to reach first.
            let mut delivers: Option<String> = None;
            let mut checks = Vec::with_capacity(items.len());
            for (at, item) in items.iter() {
                let Some(check) = mechanical_check(at, item, out) else {
                    continue;
                };
                if let MechanicalCheck::ArtifactExists { target } = &check {
                    match &delivers {
                        None => delivers = Some(target.clone()),
                        Some(first) => {
                            out.push(Refusal::new(
                                format!("{at}.target"),
                                Fault::TwoDeliverables {
                                    first: first.clone(),
                                },
                            ));
                            continue;
                        }
                    }
                }
                checks.push(check);
            }
            checks
        })
        .unwrap_or_default();
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

/// A `target` that names exactly one file inside the worktree, or a refusal
/// saying which of the four ways it does not.
///
/// **Every one of these is a step no Drone could ever pass**, and each was
/// cheaper to catch here than at the gate. v1 measured the glob case on a real
/// Job: the `design` workflow named `docs/design/*.md`, the gate probed it as a
/// literal path, and the step retried until the Job hit its token ceiling with
/// the file it asked for sitting on disk. A pattern is refused rather than
/// matched because Fleet has to hand the next step's Drone a path, and
/// "whichever file matched" is not one.
///
/// The path is not resolved against a worktree here. This crate reads files and
/// never a Job's, and whether the Drone wrote it is the gate's question — what
/// is answered here is whether the string could name it at all.
fn artifact_target(at: &str, target: String, out: &mut Vec<Refusal>) -> Option<String> {
    let why = if target.contains('*') || target.contains('?') {
        Some(BadTarget::Globbed)
    } else if target.starts_with('/') {
        Some(BadTarget::Absolute)
    } else if target.ends_with('/') {
        Some(BadTarget::ADirectory)
    } else if target.split('/').any(|segment| segment == "..") {
        Some(BadTarget::Escapes)
    } else {
        None
    };
    match why {
        None => Some(target),
        Some(why) => {
            out.push(Refusal::new(
                at,
                Fault::NotAnArtifactPath { value: target, why },
            ));
            None
        }
    }
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
        ARTIFACT_EXISTS => {
            let target_key = table.at("target");
            let target = table
                .required("target", out)
                .and_then(|value| yaml::text(&target_key, value, out))
                .and_then(|target| artifact_target(&target_key, target, out));
            table.close(ARTIFACT_EXISTS_KEYS, out);
            Some(MechanicalCheck::ArtifactExists { target: target? })
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
