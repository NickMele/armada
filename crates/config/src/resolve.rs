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

use core_model::{
    EvidenceScope, EvidenceType, FrozenWorkflow, RepoPath, ResolvedCheck, ResolvedStep, StepId,
    WorkflowId,
};

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
        // **Between the two, and this is the one place both files are in
        // hand.** `evidence_type: visual` parses against no Manifest, and an
        // `evidence:` section is declared with no workflow in sight; only here
        // can a step asking to be shown be held to a repository that can show
        // it. Refusing at dispatch is the whole argument — the alternative is a
        // Job that reaches the step with a worktree cut and a Drone spawned and
        // then captures nothing, which reads as a broken harness rather than a
        // file that never had one.
        if manifest.harness().is_none() {
            let shows = shown_by_a_harness(def);
            if !shows.is_empty() {
                return Err(ResolveError::ShowsWithNoHarness {
                    workflow: def.path().to_path_buf(),
                    manifest: manifest.path().to_path_buf(),
                    steps: shows,
                });
            }
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

/// Every step whose evidence is what it looks like, in the order the workflow
/// declares them.
///
/// **All of them and not the first**, which is [`UnknownCheck`]'s rule: a
/// workflow with three such steps is one edit to the Manifest, and a refusal
/// naming one would make it three attempts to find that out.
fn shown_by_a_harness(def: &WorkflowDef) -> Vec<StepId> {
    def.steps()
        .iter()
        .filter(|step| step.evidence_type() == Some(EvidenceType::Visual))
        .map(|step| step.id().clone())
        .collect()
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
            // **`checks_as_written` and never `check_names`.** The names this
            // replaced were read top to bottom out of the workflow file, which
            // was the only place a repository could sequence its gate —
            // alphabetical would reorder it silently. That accessor holds the
            // argument and the measured seconds behind it.
            //
            // **An empty registry expands to nothing and is not refused.**
            // `docs/concepts/manifest.md` sanctions a Manifest declaring no
            // Checks — nearest-ancestor ownership makes an ungated workspace a
            // state rather than a mistake — so refusing here would make that
            // documented state unusable with any shipped workflow. And *run
            // what this repository declares* reads literally: a repository
            // declaring nothing runs nothing.
            //
            // What was wrong with it was the silence, not the expansion, so
            // the declaration is frozen onto the step and the record says both
            // halves. `ResolvedStep::gates_on_every_check` carries it.
            MechanicalCheck::EveryManifestCheck => {
                for name in manifest.checks_as_written() {
                    let declared = manifest
                        .check(name)
                        .expect("`checks_as_written` holds the keys of `checks`");
                    checks.push(lifted(name.clone(), declared, declared.expect_exit_code()));
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
        step.evidence_scope().map(|scope| fenced(scope, manifest)),
        step.retry_limit(),
        step.model().cloned(),
    )
    .dispatching(step.may_dispatch_jobs())
    // Its own builder for `dispatching`'s reason, and read straight off the
    // step: the file was required to say, and `config` already refused a
    // second step saying yes, so there is nothing left to decide here.
    .delivering(step.delivers())
    // **What the expansion above cannot leave behind.** One entry per declared
    // Check is what runs; that the step asked for all of them is a separate
    // fact, and on a repository declaring none it is the only one left.
    .gating_on_every_check(
        step.mechanical_checks()
            .iter()
            .any(|check| matches!(check, MechanicalCheck::EveryManifestCheck)),
    )
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

/// What a step's work stays out of where nothing above it said.
///
/// **Generated output, and that is the whole of the class.** These decide
/// nothing about what is checked or judged, no repository authors work into
/// them, and `docs/contracts/configuration.md` already reduced the shipped
/// workflows' lists to exactly these two once `.env` moved to the tier nothing
/// lifts. What has changed is where the two words are written: seven workflow
/// files each carrying them is seven places to correct a guess, and this is
/// one.
///
/// **It is a guess about ecosystems and it is deliberately short.** `target`
/// is Cargo's and `node_modules` is npm's, so a repository that is neither
/// inherits a fence that names nothing it has — which costs nothing, because
/// a fence only bites a path a Drone declares. What a longer list would cost
/// is the opposite and is not symmetric: a wrong entry refuses a Drone the
/// file that holds the fix, which is the failure `#417` was filed about. So
/// the list stays at the two entries that were already shipped, and a
/// repository that wants others writes them.
///
/// **Matched by [`core_model::under`], anchored at the repository root**, like
/// every other entry in this list — so `target` fences `target/debug` and not
/// `crates/x/target`. That is the existing reading of a workflow's own entries
/// and this tier does not get a second one.
const WHERE_NO_REPOSITORY_AUTHORS: &[&str] = &["node_modules", "target"];

/// One step's `exclude_paths`, resolved across the three tiers that state it.
///
/// # The order, and this is the only place it is written
///
/// The step's own list, then the repository's `drone.exclude_paths`, then
/// [`WHERE_NO_REPOSITORY_AUTHORS`]. `fleet::Liveness::at` is the worked example
/// this follows; the difference is that patience is two numbers each falling
/// back on its own, and this is one list falling back whole.
///
/// **A tier that states one states all of it.** The dialect has no negation —
/// `docs/contracts/configuration.md` refuses a leading `!` by name — so under a
/// union a repository could add a fence and never drop one, and the compiled-in
/// guess would be the one thing nobody could correct — the objection this key
/// answers.
///
/// **An empty list is unreachable, so absence has one reading.**
/// [`crate::yaml::list`] refuses the empty list wherever it could be written,
/// which is why this compares on `is_empty` and needs no `Option`.
///
/// **Frozen, and the Job's record says what it was.** This runs once, where a
/// workflow is resolved at daemon start, and every Job freezes the answer — so
/// a Job asked later why a path was refused answers from its own row rather
/// than from a file since edited. A save that moves the key is reported as
/// needing a restart, by [`crate::live`]. A step with no `evidence_scope` at
/// all is untouched: nothing fences a step that asks for no declaration.
fn fenced(scope: &EvidenceScope, manifest: &Manifest) -> EvidenceScope {
    if !scope.exclude_paths().is_empty() {
        return scope.clone();
    }
    let inherited: Vec<RepoPath> = match manifest.exclude_paths() {
        [] => WHERE_NO_REPOSITORY_AUTHORS
            .iter()
            .map(|path| RepoPath::new(*path))
            .collect(),
        stated => stated.to_vec(),
    };
    EvidenceScope::declared(
        scope.context_source(),
        inherited,
        scope.reference_docs().to_vec(),
        scope.scope_diff_check(),
        scope.declare_plan_at(),
    )
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
