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
//! and its fields are private. Holding one is proof the names resolved, which
//! is why nothing downstream accepts a bare [`WorkflowDef`]. A resolved step
//! carries the Manifest's command rather than its name, so the lookup that
//! could have missed happens once.
//!
//! # What it produces is `core-model`'s, because a Job keeps it
//!
//! [`ResolvedWorkflow::frozen`] hands back the [`FrozenWorkflow`] a Job is
//! created with. Resolution happens here once; from then on Fleet reads the
//! Job's copy and never this file again.

use std::path::PathBuf;

use core_model::{FrozenWorkflow, ResolvedCheck, ResolvedStep, WorkflowId};

use crate::error::{ResolveError, UnknownCheck};
use crate::manifest::Manifest;
use crate::workflow::{MechanicalCheck, Step, WorkflowDef};

/// A workflow that can be dispatched against a specific Manifest.
///
/// Holding one means every Check its steps name was declared by that Manifest
/// at the moment this was built.
#[derive(Debug, Clone)]
pub struct ResolvedWorkflow {
    workflow: PathBuf,
    manifest: PathBuf,
    frozen: FrozenWorkflow,
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
            frozen: FrozenWorkflow::frozen(
                def.id().clone(),
                def.name().to_string(),
                def.version(),
                steps,
            ),
        })
    }

    /// What a Job freezes. **The paths do not travel with it** — a path on a
    /// record outlives the file at it, and what a Job needs is the declaration.
    pub fn frozen(&self) -> &FrozenWorkflow {
        &self.frozen
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
        self.frozen.id()
    }

    pub fn name(&self) -> &str {
        self.frozen.name()
    }

    pub fn version(&self) -> u32 {
        self.frozen.version()
    }

    /// The steps, in order.
    pub fn steps(&self) -> &[ResolvedStep] {
        self.frozen.steps()
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
    ResolvedStep::frozen(
        step.id().clone(),
        step.label().to_string(),
        step.evidence_type(),
        checks,
        step.advance_gate(),
        step.judge_checks().to_vec(),
        step.evidence_scope().cloned(),
        step.retry_limit(),
    )
}
