//! The one cross-file validation: a workflow's steps against a Manifest's
//! Checks.
//!
//! # Refused before dispatch, not at the step
//!
//! A step naming a Check the Manifest does not declare is a typo in a file, and
//! the cost of finding it late is a worktree checked out, a Drone spawned, two
//! steps of real work done and a Job that then stops on a name. So the check
//! runs once, here, before anything is dispatched.
//!
//! # The type is the enforcement, not a call somebody remembers to make
//!
//! [`ResolvedWorkflow`] has no constructor but [`ResolvedWorkflow::resolve`],
//! and its fields are private. There is no way to build one from a
//! [`WorkflowDef`] without the Manifest, and nothing downstream should accept a
//! bare `WorkflowDef` — the whole value of this type is that holding one is
//! proof the names resolved.
//!
//! **A resolved step carries the command, not the name.** [`ResolvedCheck`]
//! holds the `run` string lifted out of the Manifest, so the lookup that could
//! have missed happens exactly once and nothing at step time performs one at
//! all. That is the difference between a check that rejects a bad state and a
//! type in which the bad state cannot be written down.

use std::path::PathBuf;

use core_model::{StepId, WorkflowId};

use crate::error::{ResolveError, UnknownCheck};
use crate::manifest::Manifest;
use crate::workflow::{AdvanceGate, EvidenceType, MechanicalCheck, Step, WorkflowDef};

/// A deterministic assertion with everything it needs already in hand.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ResolvedCheck {
    /// The named Check, and the command it resolved to. `name` is kept beside
    /// `run` because evidence and escalation payloads cite the Check by name,
    /// and a bare command line in a message tells nobody which gate failed.
    ManifestCheck {
        name: String,
        run: String,
        expect_exit_code: i64,
    },
    DiffNonempty,
}

/// A step whose Checks have all resolved.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ResolvedStep {
    id: StepId,
    label: String,
    evidence_type: Option<EvidenceType>,
    checks: Vec<ResolvedCheck>,
    advance_gate: AdvanceGate,
}

impl ResolvedStep {
    pub fn id(&self) -> &StepId {
        &self.id
    }

    pub fn label(&self) -> &str {
        &self.label
    }

    pub fn evidence_type(&self) -> Option<EvidenceType> {
        self.evidence_type
    }

    /// All entries must pass. Empty on the common case of an ungated step.
    pub fn checks(&self) -> &[ResolvedCheck] {
        &self.checks
    }

    pub fn advance_gate(&self) -> AdvanceGate {
        self.advance_gate
    }
}

/// A workflow that can be dispatched against a specific Manifest.
///
/// Holding one means every Check its steps name was declared by that Manifest
/// at the moment this was built.
#[derive(Debug, Clone)]
pub struct ResolvedWorkflow {
    workflow: PathBuf,
    manifest: PathBuf,
    id: WorkflowId,
    name: String,
    version: u32,
    steps: Vec<ResolvedStep>,
}

impl ResolvedWorkflow {
    /// Check every named Check against the Manifest, and lift its command in.
    ///
    /// **Every unresolved name, not the first.** A workflow with three bad
    /// names is one edit, and a parser that stopped at the first would make it
    /// three.
    pub fn resolve(
        def: &WorkflowDef,
        manifest: &Manifest,
    ) -> Result<ResolvedWorkflow, ResolveError> {
        let mut steps = Vec::with_capacity(def.steps().len());
        let mut unknown = Vec::new();

        for step in def.steps() {
            steps.push(resolve_step(step, manifest, &mut unknown));
        }

        if !unknown.is_empty() {
            return Err(ResolveError::ChecksNotDeclared {
                workflow: def.path().to_path_buf(),
                manifest: manifest.path().to_path_buf(),
                unknown,
            });
        }

        Ok(ResolvedWorkflow {
            workflow: def.path().to_path_buf(),
            manifest: manifest.path().to_path_buf(),
            id: def.id().clone(),
            name: def.name().to_string(),
            version: def.version(),
            steps,
        })
    }

    /// The definition this came from.
    pub fn workflow_path(&self) -> &PathBuf {
        &self.workflow
    }

    /// The `armada.yml` its Checks resolved against.
    pub fn manifest_path(&self) -> &PathBuf {
        &self.manifest
    }

    /// The definition's own id — what a proposal's `workflow_id` must name.
    /// See [`crate::WorkflowDef::id`] for why the key exists.
    pub fn id(&self) -> &WorkflowId {
        &self.id
    }

    pub fn name(&self) -> &str {
        &self.name
    }

    pub fn version(&self) -> u32 {
        self.version
    }

    /// The steps, in order.
    pub fn steps(&self) -> &[ResolvedStep] {
        &self.steps
    }
}

/// A step's checks, resolved. A name that misses is recorded and the step is
/// still built, so one pass reports every miss in the workflow — the step
/// itself is discarded with the rest when `unknown` turns out non-empty.
fn resolve_step(step: &Step, manifest: &Manifest, unknown: &mut Vec<UnknownCheck>) -> ResolvedStep {
    let mut checks = Vec::with_capacity(step.mechanical_checks().len());
    for check in step.mechanical_checks() {
        match check {
            MechanicalCheck::DiffNonempty => checks.push(ResolvedCheck::DiffNonempty),
            MechanicalCheck::ManifestCheck {
                check,
                expect_exit_code,
            } => match manifest.check(check) {
                Some(declared) => checks.push(ResolvedCheck::ManifestCheck {
                    name: check.clone(),
                    run: declared.run().to_string(),
                    expect_exit_code: *expect_exit_code,
                }),
                None => unknown.push(UnknownCheck {
                    step: step.id().clone(),
                    check: check.clone(),
                    is_a_command: manifest.command(check).is_some(),
                    declared: manifest.check_names(),
                }),
            },
        }
    }
    ResolvedStep {
        id: step.id().clone(),
        label: step.label().to_string(),
        evidence_type: step.evidence_type(),
        checks,
        advance_gate: step.advance_gate(),
    }
}
