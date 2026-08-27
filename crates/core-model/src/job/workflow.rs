//! The workflow a Job froze at creation: its steps, and what each declares.
//!
//! # Why the declaration lives on the record and not in a file
//!
//! A step that declared two Checks when a person approved the Job still
//! declares two when it runs. Reading `.armada/workflows/` at dispatch made the
//! approval provisional — an edit to the file moved the gate under an approved
//! Job and nothing detected it.
//!
//! It is the same discipline `acceptance_criteria` already has: frozen at
//! creation, and an editor of the file changes the next Job rather than this
//! one.
//!
//! # These types are here rather than in `config`
//!
//! `config` owns *resolving* a definition against a Manifest, and holding a
//! `config::ResolvedWorkflow` is still proof that happened. What that
//! resolution produces is a field of the Job, so it is spelled here, where the
//! record is — `config` re-exports it, and `store` reads one back off a row.

use alloc::string::String;
use alloc::vec::Vec;

use crate::job::ids::{StepId, WorkflowId};

/// What a step produces as its work product.
///
/// `review_findings` is deliberately absent: the registry records it as not
/// among the legal values, and until that is decided it is refused by name
/// where a definition is parsed.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum EvidenceType {
    Diff,
    FailingTest,
    FactsNote,
    TestSuiteRun,
    Bundle,
    Document,
}

/// What it takes to advance past a step. **One variant, of four.**
///
/// The schema's other three each need a Judge or a Manifest-level policy, and
/// neither exists yet. An enum rather than a string so widening it is a compile
/// error at every `match`.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum AdvanceGate {
    Auto,
}

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
    /// The step produced a non-empty diff.
    DiffNonempty,
}

/// The schema's `type` value for a named Check. **Spelled once**, here, so the
/// parser, the wire and a recorded result cannot disagree about what a check is
/// called.
pub const MANIFEST_CHECK: &str = "manifest_check";
/// The schema's `type` value for the built-in diff assertion.
pub const DIFF_NONEMPTY: &str = "diff_nonempty";

impl ResolvedCheck {
    /// The WorkflowDef schema's `type` value for this check.
    pub fn kind(&self) -> &'static str {
        match self {
            ResolvedCheck::ManifestCheck { .. } => MANIFEST_CHECK,
            ResolvedCheck::DiffNonempty => DIFF_NONEMPTY,
        }
    }

    /// The Manifest Check's name. **`None` on a built-in**, which names none —
    /// and that is why it is an `Option` rather than the kind repeated.
    pub fn name(&self) -> Option<&str> {
        match self {
            ResolvedCheck::ManifestCheck { name, .. } => Some(name),
            ResolvedCheck::DiffNonempty => None,
        }
    }

    /// What the check is written down as: its name, or its kind where it has
    /// no name. The one label a recorded result and a served declaration both
    /// use, so a person reading the two sees the same word.
    pub fn label(&self) -> &str {
        self.name().unwrap_or_else(|| self.kind())
    }

    /// The exit code the step expects. `None` where there is no command.
    pub fn expects(&self) -> Option<i64> {
        match self {
            ResolvedCheck::ManifestCheck {
                expect_exit_code, ..
            } => Some(*expect_exit_code),
            ResolvedCheck::DiffNonempty => None,
        }
    }
}

/// A step whose Checks have all resolved, as the Job froze it.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ResolvedStep {
    id: StepId,
    label: String,
    evidence_type: Option<EvidenceType>,
    checks: Vec<ResolvedCheck>,
    advance_gate: AdvanceGate,
}

impl ResolvedStep {
    /// Build one from parts already resolved.
    ///
    /// Two callers only: `config`, having checked every name against a
    /// Manifest, and `store`, reading back what a Job froze. It is public
    /// because the second of those exists — a frozen workflow that could not
    /// come off a row could not be frozen at all.
    pub fn frozen(
        id: StepId,
        label: String,
        evidence_type: Option<EvidenceType>,
        checks: Vec<ResolvedCheck>,
        advance_gate: AdvanceGate,
    ) -> ResolvedStep {
        ResolvedStep {
            id,
            label,
            evidence_type,
            checks,
            advance_gate,
        }
    }

    pub fn id(&self) -> &StepId {
        &self.id
    }

    /// Display only. **Nothing routes on it.**
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

/// The WorkflowDef a Job follows, as it stood when the Job was created.
///
/// **Fleet reads this and never the file.** The file it came from is not
/// carried: a path recorded on a record outlives the file at it, and what the
/// Job needs is the declaration rather than where it was typed.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct FrozenWorkflow {
    id: WorkflowId,
    name: String,
    version: u32,
    steps: Vec<ResolvedStep>,
}

impl FrozenWorkflow {
    /// See [`ResolvedStep::frozen`] for why this is public and who calls it.
    pub fn frozen(
        id: WorkflowId,
        name: String,
        version: u32,
        steps: Vec<ResolvedStep>,
    ) -> FrozenWorkflow {
        FrozenWorkflow {
            id,
            name,
            version,
            steps,
        }
    }

    /// The definition's own id — what a proposal's `workflow_id` must name, and
    /// what `jobs.workflow_id` holds.
    pub fn id(&self) -> &WorkflowId {
        &self.id
    }

    pub fn name(&self) -> &str {
        &self.name
    }

    pub fn version(&self) -> u32 {
        self.version
    }

    /// The steps, in order. **Order is the semantics** — there is no `order`
    /// field, because an array already has one.
    pub fn steps(&self) -> &[ResolvedStep] {
        &self.steps
    }

    /// One step by id, or `None` where this workflow does not declare it.
    pub fn step(&self, id: &StepId) -> Option<&ResolvedStep> {
        self.steps.iter().find(|step| step.id() == id)
    }

    /// The step that follows one, or `None` at the last.
    pub fn after(&self, id: &StepId) -> Option<&ResolvedStep> {
        let at = self.steps.iter().position(|step| step.id() == id)?;
        self.steps.get(at + 1)
    }
}

impl EvidenceType {
    /// Every variant, in the order the registry lists them.
    pub const ALL: &'static [EvidenceType] = &[
        EvidenceType::Diff,
        EvidenceType::FailingTest,
        EvidenceType::FactsNote,
        EvidenceType::TestSuiteRun,
        EvidenceType::Bundle,
        EvidenceType::Document,
    ];

    /// The wire value, which is also the WorkflowDef schema's spelling.
    pub fn as_wire(&self) -> &'static str {
        match self {
            EvidenceType::Diff => "diff",
            EvidenceType::FailingTest => "failing_test",
            EvidenceType::FactsNote => "facts_note",
            EvidenceType::TestSuiteRun => "test_suite_run",
            EvidenceType::Bundle => "bundle",
            EvidenceType::Document => "document",
        }
    }

    /// Read a stored value back. `None` where it is not one of the six.
    pub fn from_wire(value: &str) -> Option<EvidenceType> {
        EvidenceType::ALL
            .iter()
            .copied()
            .find(|kind| kind.as_wire() == value)
    }
}

impl AdvanceGate {
    pub fn as_wire(&self) -> &'static str {
        match self {
            AdvanceGate::Auto => "auto",
        }
    }

    pub fn from_wire(value: &str) -> Option<AdvanceGate> {
        match value {
            "auto" => Some(AdvanceGate::Auto),
            _ => None,
        }
    }
}
