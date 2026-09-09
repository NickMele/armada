//! The one cross-file validation: a workflow's steps against a Manifest's
//! Checks.
//!
//! # Refused before dispatch, not at the step
//!
//! A step naming a Check the Manifest does not declare is a typo in a file, and
//! the cost of finding it late is a worktree checked out, a Drone spawned, two
//! steps of real work done and a Job that then stops on a name. So the check
//! runs once, here, before anything is dispatched — along with the two ways
//! files that agree on a name still disagree, which is [`Disagreement`].
//!
//! # This is also where a step stops enumerating
//!
//! `every_manifest_check` has no answer until a Manifest is in hand, and gets
//! one here. Expanded, it means *run what this repository declares* and still
//! freezes onto the Job as the list of Checks the Job gated on — so nothing
//! downstream of this file knows the spelling exists.
//!
//! # The type is the enforcement, not a call somebody remembers to make
//!
//! [`ResolvedWorkflow`] has no constructor but [`ResolvedWorkflow::resolve`]
//! and its fields are private, so holding one is proof the names resolved — and
//! a resolved step carries the Manifest's command rather than its name.
//! [`ResolvedWorkflow::frozen`] hands back the [`FrozenWorkflow`] a Job is
//! created with; from then on Fleet reads the Job's copy and never this file.

use std::path::PathBuf;

use core_model::{FrozenWorkflow, ResolvedCheck, ResolvedStep, WorkflowId};

use crate::error::{Disagreement, ResolveError, UnknownCheck};
use crate::manifest::{Check, Manifest};
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
        let mut disagreements = Vec::new();

        for step in def.steps() {
            steps.push(resolve_step(
                step,
                manifest,
                &mut unknown,
                &mut disagreements,
            ));
        }

        // **A missing name first, and the rest only once every name is
        // there.** A step's copy of an exit code is compared against a Check
        // the Manifest declares, so a name that resolved to nothing has
        // nothing to disagree with — reporting both would name one edit twice
        // and put the derived complaint above the one that caused it.
        if !unknown.is_empty() {
            return Err(ResolveError::ChecksNotDeclared {
                workflow: def.path().to_path_buf(),
                manifest: manifest.path().to_path_buf(),
                unknown,
            });
        }
        if !disagreements.is_empty() {
            return Err(ResolveError::StepsDisagreeWithTheManifest {
                workflow: def.path().to_path_buf(),
                manifest: manifest.path().to_path_buf(),
                disagreements,
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
fn resolve_step(
    step: &Step,
    manifest: &Manifest,
    unknown: &mut Vec<UnknownCheck>,
    disagreements: &mut Vec<Disagreement>,
) -> ResolvedStep {
    let mut checks = Vec::with_capacity(step.mechanical_checks().len());
    for check in step.mechanical_checks() {
        match check {
            MechanicalCheck::DiffNonempty => checks.push(ResolvedCheck::DiffNonempty),
            // Nothing to resolve: the target is a path in the Job's own
            // worktree, and no worktree exists at the moment this runs. What
            // the parser already established is that the string could name a
            // file at all, which is the half a Manifest could never answer.
            MechanicalCheck::ArtifactExists { target } => {
                checks.push(ResolvedCheck::ArtifactExists {
                    target: target.clone(),
                })
            }
            // **The set is read once, here, and the Job freezes what it
            // found.** A step that says *every Check* is answered against the
            // Manifest in hand and becomes an ordinary list of resolved
            // Checks — so nothing downstream learns a fourth kind, `store`
            // writes the same rows it always did, and the record still says
            // which Checks the Job actually gated on rather than a promise
            // that would re-read `armada.yml` mid-Job.
            //
            // **Sorted, because a registry has no order to keep.** `checks:`
            // is a map, so unlike `setup.requires` there is no sequence
            // somebody wrote; `check_names` hands back the `BTreeMap`'s order
            // and one machine reads it the same as the next.
            MechanicalCheck::EveryManifestCheck => {
                if manifest.check_names().is_empty() {
                    disagreements.push(Disagreement::NoChecksDeclared {
                        step: step.id().clone(),
                    });
                }
                for name in manifest.check_names() {
                    let declared = manifest
                        .check(&name)
                        .expect("a name `check_names` handed back is declared");
                    checks.push(lifted(name, declared, declared.expect_exit_code()));
                }
            }
            MechanicalCheck::ManifestCheck {
                check,
                expect_exit_code,
            } => match manifest.check(check) {
                // `when`, `requires` and `narrow` are lifted here beside `run`,
                // and for the same reason: all four are the Manifest's and all
                // four are frozen onto the Job, so an edit to `armada.yml`
                // changes the next Job rather than this one. A step cannot
                // narrow or widen any of them — the owner's decision is that
                // the repository declares once what a Check covers and every
                // workflow inherits it, so there is no step-level key to read
                // here and none to add.
                //
                // **`expect_exit_code` joined them, and this is where the old
                // key is held to it.** A step that writes nothing takes the
                // Manifest's; a step that writes the same number is a workflow
                // not yet edited and is carried unchanged; a step that writes a
                // different one is refused, because two files disagreeing about
                // what a passing run looks like has no reading that is not a
                // guess.
                //
                // `requires` is already resolved against the Commands registry
                // by `Manifest::parse`, so nothing here can miss: a name that
                // did not resolve refused the file before a workflow was
                // looked at.
                Some(declared) => {
                    let expects = declared.expect_exit_code();
                    if let Some(restated) = expect_exit_code {
                        if *restated != expects {
                            disagreements.push(Disagreement::ExitCode {
                                step: step.id().clone(),
                                check: check.clone(),
                                step_expects: *restated,
                                manifest_expects: expects,
                            });
                        }
                    }
                    checks.push(lifted(check.clone(), declared, expects));
                }
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
        step.model().cloned(),
    )
    .dispatching(step.may_dispatch_jobs())
    // Its own builder for `dispatching`'s reason, and read straight off the
    // step: the file was required to say, and `config` already refused a
    // second step saying yes, so there is nothing left to decide here.
    .delivering(step.delivers())
    // **Both keys through one builder**, which is the shape `dispatching` set
    // and the reason `frozen`'s ten positional arguments did not become
    // twelve. The cap is a count and never an `Option` on the record: absent
    // and "no loop here" are the same sentence, and `looping` states why zero
    // is the fail-closed answer.
    .looping(
        step.verdict_routing().clone(),
        step.iteration_cap().unwrap_or(0),
    )
    .quiet_after(step.quiet_after_seconds())
    .poking(step.poke_limit())
}

/// One declared Check, copied onto the record.
///
/// **One function, because both spellings produce the same thing.** A step that
/// names a Check and a step that gates on every one of them must freeze
/// identical rows, and two copies of five field assignments is how the second
/// one loses `narrow` the next time a key is added.
fn lifted(name: String, declared: &Check, expect_exit_code: i64) -> ResolvedCheck {
    ResolvedCheck::ManifestCheck {
        name,
        run: declared.run().to_string(),
        expect_exit_code,
        when: declared.when().cloned(),
        requires: declared.requires().to_vec(),
        narrow: declared.narrow().cloned(),
    }
}
