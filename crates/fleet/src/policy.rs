//! The two Manifest policies, resolved for one Job, and what each of them
//! decides.
//!
//! # Where a `manifest_rule:` gate is answered
//!
//! `config` carries `manifest_rule:<key>` onto the frozen step without reading
//! it, `store` round-trips it unresolved, and this is the resolution: the
//! repository's word, folded across the Job's gating Manifests, met with what
//! the step itself declared. `crate::gate` reads [`Policies::review_gate`] at
//! the advance gate and `crate::under_review` reads
//! [`Policies::auto_merge`] on the sweep over an open pull request.
//!
//! # Resolved at the question and never onto the record
//!
//! Both settings are `Live`. A [`Policies`] is built where it is used and
//! dropped there — nothing holds one across a step, and nothing writes one
//! down. A Job whose repository changed its mind between two gates is answered
//! twice, differently, and that is the point of the lifetime.
//!
//! # One Manifest today, and the seam is named rather than assumed
//!
//! A Fleet holds one `armada.yml`. A Convoy is gated by several and
//! `docs/concepts/convoy.md` settles the fold — most-restrictive-wins, because
//! there is one pull request — so the fold is called here with the one Manifest
//! there is rather than skipped. [`Policies::gating`] is the single place a
//! second Manifest arrives.

use adapter_traits::WhatTheForgeRan;
use core_model::{AutoMerge, ResolvedStep, ReviewGate};

/// What the repository has said about one Job, both policies at once.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Policies {
    auto_merge: AutoMerge,
    review_gate: ReviewGate,
}

impl Policies {
    /// Fold every gating Manifest's word into one answer each.
    ///
    /// **Most-restrictive-wins for both**, which `core_model` owns and this
    /// only calls: `never` beats `tests-pass` beats `always`, `human_always`
    /// beats `auto_if_judge_passes`.
    ///
    /// It takes the pairs rather than the Manifests so that `fleet` does not
    /// grow an opinion about how a Job's gate list is assembled — that is
    /// `Job::gate_manifests` and, for a Convoy, a lookup this crate does not
    /// have yet.
    pub fn gating(said: impl IntoIterator<Item = (AutoMerge, ReviewGate)>) -> Policies {
        let (merges, gates): (Vec<AutoMerge>, Vec<ReviewGate>) = said.into_iter().unzip();
        Policies {
            auto_merge: AutoMerge::across(merges),
            review_gate: ReviewGate::across(gates),
        }
    }

    /// A repository that has said nothing, which resolves to the cautious
    /// value of each. **`drone::Drone::unstated`'s name and its meaning**: it
    /// is exactly `gating([])`, spelled so a caller with no Manifest in hand
    /// reads as one rather than as an empty iterator.
    pub fn unstated() -> Policies {
        Policies::gating([])
    }

    pub fn auto_merge(&self) -> AutoMerge {
        self.auto_merge
    }

    pub fn review_gate(&self) -> ReviewGate {
        self.review_gate
    }

    /// Whether a machine may take the work off a `manifest_rule:auto_merge`
    /// gate, given what the forge's own checks came to.
    ///
    /// | Resolved | Forge | Answer |
    /// |---|---|---|
    /// | `never` | anything | no — a person presses, `crate::merging` |
    /// | `tests-pass` | [`AllPassed`](WhatTheForgeRan::AllPassed) | yes |
    /// | `tests-pass` | anything else | no, and ask again next sweep |
    /// | `always` | anything, **including unreadable** | yes |
    ///
    /// **`always` does not read the forge at all**, and that is the value
    /// meaning what it says. A repository that wanted the checks consulted has
    /// `tests-pass` to say so with; making `always` wait on a reading would
    /// leave the two values one word apart in the file and identical in
    /// behaviour on every repository whose forge runs nothing.
    ///
    /// **`NothingRan` is not a pass under `tests-pass`**, which
    /// `adapter_traits` decided where the variant is declared: a repository
    /// with no automation has proved nothing, and merging on the strength of an
    /// empty list is the failure that reading is worded to prevent.
    ///
    /// **Who is asking is not read here.** The step's own gate is the
    /// caller's first question — `crate::merging` — because this type knows a
    /// policy and not a workflow.
    pub fn a_machine_may_merge(&self, forge: &WhatTheForgeRan) -> bool {
        match self.auto_merge {
            AutoMerge::Never => false,
            AutoMerge::TestsPass => matches!(forge, WhatTheForgeRan::AllPassed { .. }),
            AutoMerge::Always => true,
        }
    }

    /// What a step gated `manifest_rule:review_gate` comes to, against the step
    /// itself.
    ///
    /// # The judgeless step, which is legal and is the hazard
    ///
    /// `crates/config/src/workflow/step.rs` refuses a step gated
    /// `auto_if_judge_passes` that asks the Judge nothing — the gate names a
    /// tier and the tier is not declared, so the step would advance on the
    /// mechanical tier alone while reading as judged. **That rule cannot run at
    /// parse time when the gate is a policy**, because the file does not say
    /// what the policy is, and the designed Code Review declares exactly this
    /// shape: `"judge_checks": []` on a `manifest_rule:review_gate` step.
    /// Refusing it there would refuse a shape the design sanctions.
    ///
    /// So it is refused here, where both halves are in hand — and **refused
    /// means held for a person**, not failed.
    ///
    /// | Answer | Why not |
    /// |---|---|
    /// | Advance anyway | The failure the parse-time rule exists to prevent, reached by another road |
    /// | Fail the step | The step passed every tier it declared. Nothing is wrong with the work, and a Drone would be handed back a configuration mismatch it cannot fix from a worktree |
    /// | **Hold for a person** | The most cautious of the policy's own two values, reached by the same most-restrictive-wins principle the fold above runs on. The Job lands where every other unanswerable gate lands, on a surface that already draws it |
    ///
    /// **And it is said out loud.** A repository that asked for automation and
    /// got a hold is owed the reason, which is why this answers a
    /// [`HeldBecause`] rather than a `bool`: `crate::gate` carries the word onto the
    /// ruling and `crate::dispatch` writes the line into the Job's own log.
    /// Downgrading in silence would be the same defect one layer up — a gate
    /// that reads as one thing and behaves as another.
    pub fn at_a_review_gate(&self, step: &ResolvedStep) -> HeldBecause {
        match self.review_gate {
            ReviewGate::HumanAlways => HeldBecause::ThePolicyAsksForAPerson,
            // **The step's own declaration, not what the Judge was actually
            // asked.** `crate::gate` also asks the Judge where a step drifted
            // off its declared plan, which is a fact about one run — so reading
            // that instead would make the same step advance on its own and hold
            // on its next attempt. The parse-time rule compares against the
            // declaration and this is that rule, so it compares against the
            // declaration too.
            ReviewGate::AutoIfJudgePasses if !step.asks_the_judge() => {
                HeldBecause::TheStepAsksNoJudge
            }
            ReviewGate::AutoIfJudgePasses => HeldBecause::Not,
        }
    }
}

/// Why a step gated on a policy is holding, or that it is not.
///
/// **A word rather than a flag**, because the two holds are different sentences
/// to a person: one is the repository's answer arriving, and the other is the
/// repository's answer being unreachable on this step.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum HeldBecause {
    /// It is not held. The policy resolved to advancing and the step can carry
    /// it.
    Not,
    /// `review_gate` is `human_always`, which is its default and its cautious
    /// value.
    ThePolicyAsksForAPerson,
    /// `review_gate` is `auto_if_judge_passes` and the step declares no Judge
    /// criterion, so there is no judgment for the policy to advance on. See
    /// [`Policies::at_a_review_gate`].
    TheStepAsksNoJudge,
}

impl HeldBecause {
    /// Whether this holds the step. Named so the call site reads as the
    /// question rather than as a comparison against a variant.
    pub fn holds(&self) -> bool {
        !matches!(self, HeldBecause::Not)
    }

    /// The sentence for the Job's own log, or [`None`] where nothing is worth
    /// saying.
    ///
    /// **`ThePolicyAsksForAPerson` says nothing**, and that is deliberate: it
    /// is the default answer on the commonest gate in the fleet, and a line per
    /// step would bury the one line here that means somebody has a file to fix.
    pub fn worth_saying(&self) -> Option<&'static str> {
        match self {
            HeldBecause::Not | HeldBecause::ThePolicyAsksForAPerson => None,
            HeldBecause::TheStepAsksNoJudge => Some(
                "this repository's review_gate is auto_if_judge_passes and this step asks the \
                 Judge nothing, so it is held for a person rather than advanced on the \
                 mechanical tier alone",
            ),
        }
    }
}
