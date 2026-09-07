//! A WorkflowDef, in the slice M1 reads.
//!
//! **A field nothing reads is a promise the file makes and the system does not
//! keep**, so this parser refuses one rather than ignoring it.
//! `hard_prerequisite`, `default_gate_policy`, `on_fail` and `on_gaming_flag`
//! are refused; `evidence_scope` and `declare_plan_at` are read, and
//! [`crate::scope`] holds the two keys inside that block that are not.
//!
//! **Three scopes, and each knows less than the one above it.** What is here
//! needs every step at once — the top-level keys, that no two steps share an
//! id, and that a routing edge names a step declared earlier. [`step`] needs
//! one step's keys at once, which is what its two disagreements are asked of.
//! [`mechanical`] needs one list entry and never sees a step. So the file rules
//! cannot be written a layer down, and neither layer down can reach up.
//!
//! **Three closed sets, each narrowed.** [`Structure`], `AdvanceGate` and
//! [`MechanicalCheck`] carry fewer variants than the schema has, and each is an
//! enum rather than a `String` so that widening one is a compile error at every
//! `match` reading it. A `String` would widen silently. The two a Job freezes —
//! `AdvanceGate` and `EvidenceType` — are `core-model`'s, because the record
//! carries them.

mod mechanical;
mod step;

pub use mechanical::MechanicalCheck;
pub use step::Step;

use std::collections::BTreeMap;
use std::path::{Path, PathBuf};

use core_model::{Ulid, WorkflowId};
use serde_yaml_ng::Value;

use crate::error::{BadReturn, Fault, LoadError, Refusal};
use crate::roster::Roster;
use crate::yaml::{self, Table};

/// The keys M1 reads at the top level of a WorkflowDef.
const TOP_LEVEL: &[&str] = &["version", "workflow_id", "name", "structure", "steps"];

/// How the steps are wired. **Both of the schema's two values.**
///
/// `loop` means a step returns to an earlier one by `verdict_routing` until it
/// converges or spends its `iteration_cap`. What that return needs is a
/// verdict, and a verdict now has two places to come from: `human_always` is a
/// carried gate that `fleet::gate` holds a step at, and a Judge panel runs from
/// `judge_checks`. The sentence that stood here until #263 said neither
/// existed, and it had outlived both.
///
/// **What is still missing is underneath the parser rather than in it.** The
/// step machine has no edge from `advanced` back to `running`, so the return
/// itself is a move `core-model` cannot express; `iteration_count` is a
/// `job_steps` column the schema records as deliberately absent, and it is not
/// `retry_count`, because a plan on its fourth honest draft is not a gate
/// failure; and `EscalationTrigger::LoopCap` exists with nothing raising it. So
/// a `loop` definition loads here and nothing yet runs it, which is why no
/// definition under `.armada/workflows/` declares one.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Structure {
    Linear,
    Loop,
}

const STRUCTURE_CARRIED: &[(&str, Structure)] =
    &[("linear", Structure::Linear), ("loop", Structure::Loop)];
/// The schema's whole set, and now also the carried set — so [`Fault::OutsideM1`]
/// is unreachable at this key and the third argument to [`yaml::word`] is the
/// same list as the second. Kept as an argument rather than collapsed, because
/// the two lists mean different things everywhere else and only the caller
/// knows when they have converged.
const STRUCTURE_LEGAL: &[&str] = &["linear", "loop"];

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
            STRUCTURE_LEGAL,
            out,
        )
    });

    let items = top
        .required("steps", out)
        .and_then(|value| yaml::list("steps", value, out))
        .unwrap_or_default();
    // Paired with the file position each step came from, because a step that
    // failed to parse is dropped and the duplicate-id report below has to name
    // the line the author wrote rather than the index in a shortened list.
    let placed: Vec<(usize, Step)> = items
        .iter()
        .enumerate()
        .filter_map(|(n, (at, item))| Some((n, step::read(at, item, structure, roster, out)?)))
        .collect();
    top.close(TOP_LEVEL, out);

    // **The other half of the rule `loops` holds the linear half of.** The
    // structure field is redundant with `verdict_routing` by construction and
    // that redundancy is the whole value of the field: without this, `loop` is
    // a label a file can wear while running as a straight line, and what
    // surfaces is a Job that advances off the end of a workflow its author
    // believed would come back.
    //
    // Reported at `structure` rather than at a step, because the absence is the
    // file's and there is no offending step to name. Asked of what the file
    // wrote rather than of what parsed — `yaml::any_holds` for why.
    if structure == Some(Structure::Loop) && !yaml::any_holds(&items, "verdict_routing") {
        out.push(Refusal::new(
            "structure",
            Fault::ContradictsStructure { structure: "loop" },
        ));
    }

    // Duplicate step ids, reported on the second occurrence and naming the
    // first. Every per-step counter in the system is keyed by this value, so
    // two steps sharing one id would share a retry budget and a verdict.
    let mut first_seen: BTreeMap<&str, usize> = BTreeMap::new();
    for (n, step) in &placed {
        match first_seen.get(step.id().as_str()) {
            Some(first_at) => out.push(Refusal::new(
                format!("steps[{n}].id"),
                Fault::DuplicateStepId {
                    first_at: *first_at,
                },
            )),
            None => {
                first_seen.insert(step.id().as_str(), *n);
            }
        }
    }
    // **A routing edge has to name a step, and an earlier one.** Every other
    // name in this crate is resolved where it is written rather than at the
    // gate, for the reason `artifact_exists` gives: a step no Drone could pass
    // costs a worktree, a Drone and a retry budget to discover. A routing
    // target is the same name one layer up, and the layer that would otherwise
    // find it is a Job standing at a human gate with nowhere to go.
    //
    // The order is checked here rather than deferred to the step machine
    // because there is nothing there to defer to: the edge a return takes is
    // `advanced -> running`, and a step the Job has not reached has advanced
    // nothing. A step routing at itself is refused for the opposite reason —
    // that move exists and is spelled `retry_limit`, and the two counters are
    // two counters so a Drone that failed four times and a plan asked for a
    // fourth draft do not read alike.
    //
    // Read off the file for `yaml::any_holds`'s reason: a target step dropped
    // for its own unrelated fault is still a step the author wrote. The
    // position is the index in the document, which is what `steps[n]` in the
    // refusal already names.
    let declared = yaml::placed_values(&items, "id");
    for (n, step) in &placed {
        for (verdict, target) in step.verdict_routing() {
            let at = format!("steps[{n}].verdict_routing.{}", verdict.as_wire());
            let value = target.as_str().to_string();
            let found = declared
                .iter()
                .find(|(_, id)| *id == target.as_str())
                .map(|(at, _)| *at);
            let fault = match found {
                None => Fault::RoutesToNoSuchStep {
                    value,
                    declared: declared.iter().map(|(_, id)| (*id).to_string()).collect(),
                },
                Some(target_at) if target_at == *n => Fault::NotAReturn {
                    value,
                    why: BadReturn::Itself,
                },
                Some(target_at) if target_at > *n => Fault::NotAReturn {
                    value,
                    why: BadReturn::Ahead,
                },
                Some(_) => continue,
            };
            out.push(Refusal::new(at, fault));
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
