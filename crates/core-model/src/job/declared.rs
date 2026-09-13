//! The two closed sets a workflow step declares: what its work product is, and
//! what it takes to advance past it.
//!
//! **Beside [`super::workflow`] rather than in it**, because `xtask::rules_enums`
//! holds each against a registry table by reading the file its variants are
//! spelled in — so the pair has a file of its own rather than sharing one with
//! the resolved types that carry them.

/// What a step produces as its work product.
///
/// **Nothing a Drone does turns on this.** `verification::Accepted::of` matches
/// a submission's type against the step's, and Fleet fills the submission's from
/// that same step, so the two cannot disagree. Whether it still earns its place
/// is open at `[evidence-mcp-submission-schema]` in `docs/OPEN.md`.
///
/// `review_findings` is deliberately absent: the registry records it as not among
/// the legal values, so it is refused by name where a definition is parsed.
///
/// **Every value is something a Drone hands in and the gate measures.** `shown`
/// meant *Fleet, run the repository's harness* — an instruction, not a claim —
/// which is why a step could not both hand in a patch and be captured.
/// `ResolvedStep::captured` says it now. `#777`.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum EvidenceType {
    Diff,
    FailingTest,
    FactsNote,
    TestSuiteRun,
    Bundle,
    Document,
    /// A plan Fleet holds: an approach and tasks, recorded through
    /// `record_plan` rather than written to a file. `crate::WorkPlan`.
    Plan,
}

/// What it takes to advance past a step. **Five variants, of the schema's four
/// forms** — the fourth form is `manifest_rule:<key>` and the registry names
/// two keys, so it reaches an enum as one variant each.
///
/// **A variant per key rather than one carrying the key**, which is what keeps
/// `ALL` a list of identifiers and `as_wire` a list of literals — the two
/// things `xtask::rules_enums` reads to hold this set against the registry. A
/// payload would be invisible to both. A third key is a third variant and a
/// compile error at every `match`, which is what the enum is for.
///
/// **Two name a tier, one names an actor, and two name a policy.** `auto` and
/// `auto_if_judge_passes` say which of Fleet's tiers is the whole gate,
/// [`HumanAlways`](AdvanceGate::HumanAlways) says the tiers do not decide at
/// all, and the two `manifest_rule` variants say the repository decides — read
/// where the gate is read and never resolved onto the record.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum AdvanceGate {
    /// The mechanical tier is the whole gate.
    Auto,
    /// The mechanical tier holds **and** the Judge did not refuse. Not a score
    /// above a bar: there is no such thing as a Judge pass, only a mechanical
    /// pass a Judge declined to refuse.
    AutoIfJudgePasses,
    /// A person answers. The tiers still run and still stop the step — what
    /// they establish is the material a person reads, not the verdict.
    ///
    /// **The step does not advance here.** It holds at `awaiting_review` for
    /// one of the three answers `fleet::reviewing` implements: approve,
    /// request changes, or reject. A `mechanical_checks[]` or a `judge_checks[]`
    /// on such a step is not spent on an answer nothing reads: a tier that
    /// stops the step keeps the work away from the person, and a tier that does
    /// not is written down beside the evidence they open.
    HumanAlways,
    /// The repository's `auto_merge` policy decides whether this step's work
    /// lands without a person. Its values are `never`, `checks-pass` and
    /// `always`, and they are not gate words — what resolves them into one is
    /// `fleet::gate`.
    ManifestRuleAutoMerge,
    /// The repository's `review_gate` policy decides whether a person signs
    /// off, between `human_always` and `auto_if_judge_passes`.
    ///
    /// **Frozen unresolved, unlike every other field of a step.**
    /// `crates/config/settings.toml` declares both policies `Live`, so an
    /// `armada.yml` saved mid-Job moves them; freezing the answer at Job
    /// creation would put a decision on the record that the repository had not
    /// made yet, and would read as though it had.
    ManifestRuleReviewGate,
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
        EvidenceType::Plan,
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
            EvidenceType::Plan => "plan",
        }
    }

    /// Read a stored value back. `None` where it is not one of the set.
    pub fn from_wire(value: &str) -> Option<EvidenceType> {
        EvidenceType::ALL
            .iter()
            .copied()
            .find(|kind| kind.as_wire() == value)
    }
}

impl AdvanceGate {
    /// Every variant, in the order the tiers run, with the two policy forms
    /// last because what they resolve to is not known here.
    pub const ALL: &'static [AdvanceGate] = &[
        AdvanceGate::Auto,
        AdvanceGate::AutoIfJudgePasses,
        AdvanceGate::HumanAlways,
        AdvanceGate::ManifestRuleAutoMerge,
        AdvanceGate::ManifestRuleReviewGate,
    ];

    pub fn as_wire(&self) -> &'static str {
        match self {
            AdvanceGate::Auto => "auto",
            AdvanceGate::AutoIfJudgePasses => "auto_if_judge_passes",
            AdvanceGate::HumanAlways => "human_always",
            AdvanceGate::ManifestRuleAutoMerge => "manifest_rule:auto_merge",
            AdvanceGate::ManifestRuleReviewGate => "manifest_rule:review_gate",
        }
    }

    pub fn from_wire(value: &str) -> Option<AdvanceGate> {
        match value {
            "auto" => Some(AdvanceGate::Auto),
            "auto_if_judge_passes" => Some(AdvanceGate::AutoIfJudgePasses),
            "human_always" => Some(AdvanceGate::HumanAlways),
            "manifest_rule:auto_merge" => Some(AdvanceGate::ManifestRuleAutoMerge),
            "manifest_rule:review_gate" => Some(AdvanceGate::ManifestRuleReviewGate),
            _ => None,
        }
    }
}
