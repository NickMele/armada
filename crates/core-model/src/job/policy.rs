//! The two Manifest policies a `manifest_rule:<key>` gate reads, and how
//! several gating Manifests come to one answer.
//!
//! # Values, not gates
//!
//! [`AutoMerge`] and [`ReviewGate`] are what an `armada.yml` writes. They are
//! not [`AdvanceGate`](crate::AdvanceGate) variants and never become one: a
//! workflow step declares `manifest_rule:auto_merge` and the repository
//! declares `always`, and the two together are what `fleet::gate` rules on.
//! Folding a resolved policy back into a gate word would put the repository's
//! answer on the frozen step, which is exactly what
//! [`AdvanceGate::ManifestRuleReviewGate`](crate::AdvanceGate::ManifestRuleReviewGate)
//! says must not happen — the settings rows call both policies `Live`, so the
//! answer is only true for as long as nobody saves the file.
//!
//! # Most-restrictive-wins, and why it is here
//!
//! `docs/concepts/convoy.md` settles the resolution and gives the reason:
//! **there is one pull request**, so the most cautious gating Manifest holds.
//! `never` beats `tests-pass` beats `always`; `human_always` beats
//! `auto_if_judge_passes`. It is spelled here rather than in `config` because a
//! Job's gate list is a Job's field — `Job::gate_manifests` — and `config`
//! knows about one file at a time.
//!
//! **Nothing to resolve is the strictest answer, not an error.** Both folds
//! start at the default and only ever get stricter, so a Job gated by no
//! Manifest at all comes out `never` and `human_always` — which is the same
//! answer a repository that says nothing gets, and the only safe one to reach
//! by accident.
//!
//! # No registry rows, and that is the registry's own decision
//!
//! `enum-verbs.toml` carries a row for each `manifest_rule:` gate and both say
//! the same thing in as many words: the verb *"names the policy rather than
//! what it resolved to … naming the answer here would state it in the one place
//! that cannot know it."* Nothing renders an `AutoMerge` or a `ReviewGate`, so
//! neither set is a vocabulary the surface speaks and neither is in
//! `xtask::rules_enums`' list. `crates/config/settings.toml` is where the
//! values are written down, one row each.

/// Whether a machine may land this Job's work, as one `armada.yml` says it.
///
/// **The default is [`Never`](AutoMerge::Never) and it is not an absence.** A
/// repository that says nothing is a repository where no machine decides that
/// work lands, which is the answer `crates/fleet/src/merging.rs` was written
/// under: a person presses, and Fleet performs the merge so the Checks over the
/// merged tree run.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum AutoMerge {
    /// No machine decides. The step holds and a person answers it.
    #[default]
    Never,
    /// A machine may decide, once **the forge's** checks have all passed.
    ///
    /// **Not Armada's Checks.** `adapter_traits::WhatTheForgeRan` is the
    /// reading this names — the repository's own automation, run against the
    /// branch under review — and a Check declared in `armada.yml` has already
    /// run at the gate this policy is read at. Totalling the two would say a
    /// gate had held that never ran.
    TestsPass,
    /// A machine may decide, whatever ran.
    Always,
}

impl AutoMerge {
    /// Every value, for a refusal that names what the file could have said.
    pub const ALL: &'static [AutoMerge] =
        &[AutoMerge::Never, AutoMerge::TestsPass, AutoMerge::Always];

    /// Exactly as `armada.yml` writes it. **Hyphenated in the middle value**,
    /// which is `docs/concepts/manifest.md`'s spelling and the one the settings
    /// row carries; the underscored gate words next to it are a different
    /// vocabulary and are not made to match.
    pub fn as_written(&self) -> &'static str {
        match self {
            AutoMerge::Never => "never",
            AutoMerge::TestsPass => "tests-pass",
            AutoMerge::Always => "always",
        }
    }

    /// Read one back. `None` is a word the policy has no value for, which the
    /// caller refuses rather than defaults — a mistyped `auto_merge: nevr`
    /// silently meaning `never` would be right by luck here and wrong the day
    /// somebody mistypes `always`.
    pub fn from_written(value: &str) -> Option<AutoMerge> {
        AutoMerge::ALL
            .iter()
            .copied()
            .find(|policy| policy.as_written() == value)
    }

    /// How cautious this value is, lowest first. **Written out rather than
    /// derived from declaration order**, so reordering the variants cannot
    /// quietly reverse the resolution `convoy.md` fixed.
    fn caution(policy: &AutoMerge) -> u8 {
        let policy = *policy;
        match policy {
            AutoMerge::Never => 0,
            AutoMerge::TestsPass => 1,
            AutoMerge::Always => 2,
        }
    }

    /// The answer across every gating Manifest: **the most cautious one wins.**
    ///
    /// **Empty is [`Never`](AutoMerge::Never), and it is not the fold's own
    /// identity.** Taking a minimum over nothing is the *loosest* value, and
    /// writing it that way returned `always` for a Job gated by no Manifest —
    /// caught by the test below, which is why the case is answered by
    /// `unwrap_or_default` rather than by a starting value. Fail-closed is a
    /// decision here, not an accident of the algebra.
    pub fn across(policies: impl IntoIterator<Item = AutoMerge>) -> AutoMerge {
        policies
            .into_iter()
            .min_by_key(AutoMerge::caution)
            .unwrap_or_default()
    }
}

/// Whether a person signs off on a workflow's review step, as one `armada.yml`
/// says it.
///
/// **The default is [`HumanAlways`](ReviewGate::HumanAlways)**, which
/// `crates/config/settings.toml` states outright and `docs/concepts/manifest.md`
/// repeats.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum ReviewGate {
    /// A person answers, exactly as
    /// [`AdvanceGate::HumanAlways`](crate::AdvanceGate::HumanAlways) does.
    #[default]
    HumanAlways,
    /// The step advances where the mechanical tier held and the Judge did not
    /// refuse — [`AdvanceGate::AutoIfJudgePasses`](crate::AdvanceGate::AutoIfJudgePasses)'s
    /// meaning, reached through a policy instead of through the file.
    ///
    /// **It says nothing about a step that asks the Judge nothing.** A
    /// judgeless step resolving to this would advance on the mechanical tier
    /// alone while reading as judged, which is the disagreement
    /// `config::workflow::step` refuses when the gate is a plain word and
    /// cannot refuse when it is a policy. `fleet::gate` is where that meets a
    /// step's declaration.
    AutoIfJudgePasses,
}

impl ReviewGate {
    /// Every value, for [`AutoMerge::ALL`]'s reason.
    pub const ALL: &'static [ReviewGate] =
        &[ReviewGate::HumanAlways, ReviewGate::AutoIfJudgePasses];

    /// Exactly as `armada.yml` writes it. **The same two words the gate uses**,
    /// and deliberately so: a repository setting `auto_if_judge_passes` is
    /// asking for the gate that word already names, so one spelling covers
    /// both.
    pub fn as_written(&self) -> &'static str {
        match self {
            ReviewGate::HumanAlways => "human_always",
            ReviewGate::AutoIfJudgePasses => "auto_if_judge_passes",
        }
    }

    /// Read one back, for [`AutoMerge::from_written`]'s reason.
    pub fn from_written(value: &str) -> Option<ReviewGate> {
        ReviewGate::ALL
            .iter()
            .copied()
            .find(|policy| policy.as_written() == value)
    }

    /// How cautious this value is, for [`AutoMerge::caution`]'s reason.
    fn caution(policy: &ReviewGate) -> u8 {
        match policy {
            ReviewGate::HumanAlways => 0,
            ReviewGate::AutoIfJudgePasses => 1,
        }
    }

    /// The answer across every gating Manifest: **the most cautious one wins.**
    ///
    /// Empty is [`HumanAlways`](ReviewGate::HumanAlways), for
    /// [`AutoMerge::across`]'s reason and by its means.
    pub fn across(policies: impl IntoIterator<Item = ReviewGate>) -> ReviewGate {
        policies
            .into_iter()
            .min_by_key(ReviewGate::caution)
            .unwrap_or_default()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use alloc::vec;

    /// **A Job gated by nothing is a Job nothing merges.** The fold starts at
    /// the loosest value, so this is the one case where reading the code the
    /// wrong way round produces `always` — which is why it is the first test.
    #[test]
    fn no_gating_manifest_is_the_strictest_answer_and_not_the_loosest() {
        assert_eq!(AutoMerge::across(vec![]), AutoMerge::Never);
        assert_eq!(ReviewGate::across(vec![]), ReviewGate::HumanAlways);
    }

    /// One Manifest resolves to itself, whatever it says. Without this the fold
    /// above could be a constant and both other tests would still pass.
    #[test]
    fn one_gating_manifest_is_its_own_answer() {
        for policy in AutoMerge::ALL {
            assert_eq!(AutoMerge::across(vec![*policy]), *policy);
        }
        for policy in ReviewGate::ALL {
            assert_eq!(ReviewGate::across(vec![*policy]), *policy);
        }
    }

    /// `never` beats `tests-pass` beats `always`, in either order the
    /// Manifests are listed in — `convoy.md`'s rule, and the order-independence
    /// is what makes it a resolution rather than a precedence.
    #[test]
    fn the_most_cautious_gating_manifest_holds() {
        assert_eq!(
            AutoMerge::across(vec![AutoMerge::Always, AutoMerge::TestsPass]),
            AutoMerge::TestsPass
        );
        assert_eq!(
            AutoMerge::across(vec![AutoMerge::TestsPass, AutoMerge::Always]),
            AutoMerge::TestsPass
        );
        assert_eq!(
            AutoMerge::across(vec![
                AutoMerge::Always,
                AutoMerge::Never,
                AutoMerge::TestsPass
            ]),
            AutoMerge::Never
        );
        assert_eq!(
            ReviewGate::across(vec![ReviewGate::AutoIfJudgePasses, ReviewGate::HumanAlways]),
            ReviewGate::HumanAlways
        );
        assert_eq!(
            ReviewGate::across(vec![ReviewGate::HumanAlways, ReviewGate::AutoIfJudgePasses]),
            ReviewGate::HumanAlways
        );
    }

    /// The spellings round-trip, and a word neither policy has is `None`
    /// rather than the default — the whole reason `from_written` answers with
    /// an `Option`.
    #[test]
    fn a_word_the_policy_has_no_value_for_is_refused_rather_than_defaulted() {
        for policy in AutoMerge::ALL {
            assert_eq!(AutoMerge::from_written(policy.as_written()), Some(*policy));
        }
        for policy in ReviewGate::ALL {
            assert_eq!(ReviewGate::from_written(policy.as_written()), Some(*policy));
        }
        assert_eq!(AutoMerge::from_written("tests_pass"), None);
        assert_eq!(AutoMerge::from_written("human_always"), None);
        assert_eq!(ReviewGate::from_written("auto"), None);
        assert_eq!(ReviewGate::from_written("never"), None);
    }
}
