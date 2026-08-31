//! What this crate refuses, by name.
//!
//! Typed leaf enums with structured fields, per the Error Contract: the key
//! that was wrong and the value that was in it, never a sentence assembled at
//! the raise site. The `Display` impls exist so a refusal reads, and the fields
//! exist so a caller can match on it without reading prose.
//!
//! # Every refusal names three things
//!
//! The file, the key and what was wrong. "Refuse loudly" is the requirement,
//! and a message that names two of the three sends somebody back to the file to
//! guess which line it meant. [`Refusal`] carries the key and the fault;
//! [`LoadError::Refused`] carries the file and holds them.
//!
//! # A refusal set, not the first refusal
//!
//! Parsing collects every fault in the document and returns them together. A
//! parser that stops at the first one makes fixing an `armada.yml` a sequence
//! of round trips, and — the reason that actually decides it — it would hide
//! the finding this milestone step exists to surface: four checked-in workflow
//! samples declare `structure: "linear"` and carry `verdict_routing`, and every
//! one of them also carries keys M1 does not read. Under a bail-on-first parser
//! the contradiction is never reached.
//!
//! This is the same shape as `store`'s [`LoadAllError::SomeJobsUnreadable`],
//! for the same reason: the caller decides what a partial answer is worth, and
//! the function does not decide on its behalf.
//!
//! [`LoadAllError::SomeJobsUnreadable`]: store::LoadAllError

use std::error::Error;
use std::fmt;
use std::path::PathBuf;

use core_model::{BadPattern, StepId};

/// One key, and one thing wrong with it.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Refusal {
    /// The dotted path to the key inside its document —
    /// `steps[1].mechanical_checks[0].type`, `checks.build.run`. Indices are
    /// the array position, so the message points at a line rather than at a
    /// name the author would have to count to find.
    pub key: String,
    pub fault: Fault,
}

impl Refusal {
    pub(crate) fn new(key: impl Into<String>, fault: Fault) -> Self {
        Refusal {
            key: key.into(),
            fault,
        }
    }
}

/// What was wrong with one key.
///
/// The two "not a legal value" cases are deliberately separate variants.
/// [`Fault::OutsideM1`] is a value the schema sanctions and this milestone does
/// not implement — the fix is to wait or to rewrite the workflow.
/// [`Fault::NotInTheSchema`] is a value no version of Armada has ever had — the
/// fix is a typo correction. Collapsing them would make every refusal read like
/// a missing feature.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Fault {
    /// A required key is absent.
    Missing,
    /// A key nothing at M1 reads. **This is the hard-fail that makes every
    /// deferred section additive later rather than a migration**: a file that
    /// loads today cannot be carrying a key a later milestone will give a
    /// different meaning to.
    Unknown {
        /// What M1 does read at this position, so the message can name it.
        known: &'static [&'static str],
    },
    /// The key is there and holds the wrong shape.
    WrongType {
        wanted: &'static str,
        found: &'static str,
    },
    /// A string with nothing in it, or a `steps` array with no steps. Distinct
    /// from [`Fault::Missing`]: the author wrote the key and left it blank,
    /// which is a different mistake from never having written it.
    Empty,
    /// A value the schema sanctions that M1 does not carry.
    OutsideM1 {
        value: String,
        /// What M1 carries at this key.
        carried: &'static [&'static str],
    },
    /// A value outside the schema's own set.
    NotInTheSchema {
        value: String,
        legal: &'static [&'static str],
    },
    /// **A name declared under both `checks` and `commands`.** They are sibling
    /// maps sharing no keys, so the same name in both leaves nothing able to
    /// say which registry a reference meant.
    DeclaredInBothRegistries,
    /// Two steps in one workflow carry one `id`. Reported on the second, and
    /// names where the first was, because the fix is to look at both.
    DuplicateStepId { first_at: usize },
    /// **`verdict_routing` on a `linear` workflow.** The `structure` field is
    /// redundant with `verdict_routing` by construction and that redundancy is
    /// its whole value: declared intent, checked against what was wired.
    /// Without this refusal a routing edge added to a workflow the author
    /// believes is linear is legal config that surfaces as a Job which never
    /// terminates.
    ContradictsStructure { structure: &'static str },
    /// **`context_paths` in a definition.** The schema puts it on the resolved
    /// object: the Drone supplies the paths at declaration time and Fleet
    /// validates them, so at definition time there is nothing to author.
    BelongsToTheResolvedObject,
    /// **`declare_plan_at` with no `evidence_scope`.** The key says when the
    /// plan is declared and the block says what it is measured against; one
    /// without the other declares a plan nothing reads.
    PlanWithoutAScope,
    /// **A step's `advance_gate` and its `judge_checks` say different things.**
    /// The two are one statement made twice, and a file where they disagree has
    /// no reading that is not a guess about which the author meant.
    GateAndJudgeDisagree { gate: &'static str },
    /// **A step asks the Judge and declares nothing for it to look at.** A step
    /// with no `evidence_type` produces no work product, so every criterion on
    /// it is a call made against nothing and a refusal that could not have gone
    /// otherwise. Four such checks shipped in one commit and nobody noticed
    /// until a Job escalated at its first step — see #153.
    JudgedWithNothingToShow,
    /// **A `checks.<name>.when` entry the glob dialect cannot read.** Refused
    /// at load rather than matched literally, because a pattern written in some
    /// other dialect matches nothing and a Check that silently never runs again
    /// is the failure `when` exists to prevent.
    NotAPathPattern { value: String, why: BadPattern },
    /// **An `artifact_exists` target that cannot name one file.** Refused where
    /// the definition is parsed rather than discovered at the gate, because
    /// every one of these fails at the gate whatever the Drone wrote: v1
    /// shipped a `design` workflow whose target was `docs/design/*.md`, probed
    /// it as a literal path, and made the step unpassable for every Job — the
    /// commit that fixed it dates the Job it was measured against.
    ///
    /// The path is also what Fleet has to be able to hand the next step's
    /// Drone, so "whichever file matched" is not an answer this can carry.
    NotAnArtifactPath { value: String, why: BadTarget },
}

/// Why a step's `artifact_exists` target cannot name the one file the next step
/// will be pointed at.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum BadTarget {
    /// It holds `*` or `?`. A pattern matches none, one or many, and none of
    /// those three is a path a brief can quote.
    Globbed,
    /// It starts at the filesystem root, so it names something outside the
    /// worktree — where it would be the same file for every Job on the machine.
    Absolute,
    /// It climbs out of the worktree with `..`.
    Escapes,
    /// It ends in `/`, so it names a directory and a directory is not the
    /// deliverable.
    ADirectory,
}

impl fmt::Display for BadTarget {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            BadTarget::Globbed => write!(f, "it is a pattern, and a step's artifact is one file"),
            BadTarget::Absolute => write!(f, "it is absolute, and the step works in a worktree"),
            BadTarget::Escapes => write!(f, "it climbs out of the worktree"),
            BadTarget::ADirectory => write!(f, "it names a directory"),
        }
    }
}

impl fmt::Display for Fault {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Fault::Missing => write!(f, "is required and is not there"),
            Fault::Unknown { known } => write!(
                f,
                "is not a key M1 reads. M1 reads {}",
                Listed(known, "nothing here")
            ),
            Fault::WrongType { wanted, found } => write!(f, "wanted {wanted} and holds {found}"),
            Fault::Empty => write!(f, "is empty"),
            Fault::OutsideM1 { value, carried } => write!(
                f,
                "is `{value}`, which M1 does not carry. M1 carries {}",
                Listed(carried, "nothing yet")
            ),
            Fault::NotInTheSchema { value, legal } => write!(
                f,
                "is `{value}`, which is not one the schema has. The schema has {}",
                Listed(legal, "no value")
            ),
            Fault::DeclaredInBothRegistries => write!(
                f,
                "is declared under both `checks` and `commands`, which share no names"
            ),
            Fault::DuplicateStepId { first_at } => {
                write!(f, "repeats the id already used by steps[{first_at}]")
            }
            Fault::ContradictsStructure { structure } => write!(
                f,
                "declares a routing edge, and the workflow declares `structure: {structure}`"
            ),
            Fault::BelongsToTheResolvedObject => write!(
                f,
                "is a field of the resolved evidence scope and not of the \
                 definition. The Drone declares the paths and Fleet validates \
                 them, so nothing can be authored here"
            ),
            Fault::PlanWithoutAScope => write!(
                f,
                "says when the plan is declared and the step declares no \
                 `evidence_scope` for it to be measured against"
            ),
            Fault::JudgedWithNothingToShow => write!(
                f,
                "asks the Judge a question and declares no `evidence_type`, so \
                 the step produces nothing the Judge could be shown"
            ),
            Fault::NotAPathPattern { value, why } => {
                write!(f, "is `{value}`, which is not a path pattern: {why}")
            }
            Fault::NotAnArtifactPath { value, why } => write!(
                f,
                "is `{value}`, which cannot name the file this step writes: {why}"
            ),
            Fault::GateAndJudgeDisagree { gate: "auto" } => write!(
                f,
                "is `auto`, which is the mechanical tier alone, and the step \
                 declares `judge_checks` criteria nothing would read"
            ),
            Fault::GateAndJudgeDisagree { gate } => write!(
                f,
                "is `{gate}`, and the step declares no `judge_checks` criterion \
                 for a Judge to read"
            ),
        }
    }
}

/// A comma-separated value list with a stated fallback for the empty case, so
/// no message ever ends in a dangling "M1 reads ".
struct Listed<'a>(&'a [&'a str], &'a str);

impl fmt::Display for Listed<'_> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        if self.0.is_empty() {
            return write!(f, "{}", self.1);
        }
        for (n, item) in self.0.iter().enumerate() {
            if n > 0 {
                write!(f, ", ")?;
            }
            write!(f, "`{item}`")?;
        }
        Ok(())
    }
}

/// Why a configuration file did not load.
///
/// One type for `armada.yml` and for a WorkflowDef, because `path` already says
/// which file it was and two enums with identical variants would be the second
/// vocabulary the practice doc forbids.
#[derive(Debug)]
pub enum LoadError {
    /// The file could not be read. The `io::Error` is carried rather than
    /// formatted, so the chain stays traversable up to the wire.
    Unreadable {
        path: PathBuf,
        cause: std::io::Error,
    },
    /// The bytes are not YAML at all. Same treatment: the parser's own error is
    /// the cause, and it carries the line and column.
    ///
    /// **A key written twice lands here**, because the parser refuses a
    /// duplicate mapping key rather than letting the last one win. That is the
    /// loud answer to a file that says two things about one Check, and it
    /// arrives before this crate sees a document at all.
    NotYaml {
        path: PathBuf,
        cause: serde_yaml_ng::Error,
    },
    /// The document is YAML and Armada will not have it. **Every fault found,
    /// not the first.**
    Refused {
        path: PathBuf,
        refusals: Vec<Refusal>,
    },
}

impl LoadError {
    /// The file the refusal is about. Present on every variant, because a
    /// message that does not name the file is one somebody has to guess at.
    pub fn path(&self) -> &PathBuf {
        match self {
            LoadError::Unreadable { path, .. }
            | LoadError::NotYaml { path, .. }
            | LoadError::Refused { path, .. } => path,
        }
    }

    /// What was wrong, key by key. Empty for the two variants where the
    /// document never became a document.
    pub fn refusals(&self) -> &[Refusal] {
        match self {
            LoadError::Refused { refusals, .. } => refusals,
            _ => &[],
        }
    }
}

impl fmt::Display for LoadError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            LoadError::Unreadable { path, cause } => {
                write!(f, "{} could not be read: {cause}", path.display())
            }
            LoadError::NotYaml { path, cause } => {
                write!(f, "{} is not YAML: {cause}", path.display())
            }
            LoadError::Refused { path, refusals } => {
                write!(f, "{} was refused", path.display())?;
                for refusal in refusals {
                    write!(f, "; `{}` {}", refusal.key, refusal.fault)?;
                }
                Ok(())
            }
        }
    }
}

impl Error for LoadError {
    fn source(&self) -> Option<&(dyn Error + 'static)> {
        match self {
            LoadError::Unreadable { cause, .. } => Some(cause),
            LoadError::NotYaml { cause, .. } => Some(cause),
            LoadError::Refused { .. } => None,
        }
    }
}

/// A step that names a Check, and the Manifest that does not declare it.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct UnknownCheck {
    /// The step whose `mechanical_checks` named it.
    pub step: StepId,
    /// The name it asked for.
    pub check: String,
    /// **The name is declared, under `commands`.** A different mistake with a
    /// different fix, so it is a field rather than a second error type: the
    /// author wrote a real name in the wrong registry.
    pub is_a_command: bool,
    /// What the Manifest does declare under `checks`, in order.
    pub declared: Vec<String>,
}

/// Why a WorkflowDef could not be resolved against a Manifest.
///
/// **The one cross-file validation, and the point of this milestone step.** A
/// step naming a Check absent from the Manifest is refused here — before
/// dispatch — rather than when the Job arrives at that step with a worktree
/// already checked out and a Drone already spawned.
#[derive(Debug)]
pub enum ResolveError {
    /// One or more steps name Checks the Manifest does not declare. **All of
    /// them**, so a workflow with three bad names is three lines of output and
    /// one edit rather than three dispatches.
    ChecksNotDeclared {
        workflow: PathBuf,
        manifest: PathBuf,
        unknown: Vec<UnknownCheck>,
    },
}

impl fmt::Display for ResolveError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let ResolveError::ChecksNotDeclared {
            workflow,
            manifest,
            unknown,
        } = self;
        write!(
            f,
            "{} names {} Check(s) {} does not declare",
            workflow.display(),
            unknown.len(),
            manifest.display()
        )?;
        for miss in unknown {
            write!(f, "; step `{}` needs `{}`", miss.step.as_str(), miss.check)?;
            if miss.is_a_command {
                write!(f, ", which is declared as a Command, not a Check")?;
            } else {
                let names: Vec<&str> = miss.declared.iter().map(String::as_str).collect();
                write!(f, ". Declared Checks are {}", Listed(&names, "none"))?;
            }
        }
        Ok(())
    }
}

impl Error for ResolveError {}
