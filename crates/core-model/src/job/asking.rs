//! How a Job meets a Judge's refusal: ask a person, or stop the step.
//!
//! **Modelled on [`crate::WhenBlocked`], the same shape for the same reason.**
//! A person already knows that setting from the Job header, and
//! `docs/concepts/judge.md`'s asking design calls for a Job-level override that
//! reaches "both ways" — forcing every criterion to ask even one authored
//! `refuse`, or forcing every criterion to refuse even one that defaults to
//! asking — which is exactly the three-way shape `WhenBlocked` already uses.

use crate::job::judge::OnRefusal;

/// How this Job meets a Judge criterion that refuses.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub enum WhenRefused {
    /// Each criterion decides for itself, off its own [`OnRefusal`]. The
    /// default, and what every Job written before this setting existed reads
    /// as.
    #[default]
    PerCriterion,
    /// Every criterion asks, including one authored `refuse`.
    AlwaysAsk,
    /// Every criterion stops the step, including one that defaults to asking.
    AlwaysRefuse,
}

impl WhenRefused {
    pub const ALL: &'static [WhenRefused] = &[
        WhenRefused::PerCriterion,
        WhenRefused::AlwaysAsk,
        WhenRefused::AlwaysRefuse,
    ];

    pub fn as_wire(&self) -> &'static str {
        match self {
            WhenRefused::PerCriterion => "per_criterion",
            WhenRefused::AlwaysAsk => "always_ask",
            WhenRefused::AlwaysRefuse => "always_refuse",
        }
    }

    /// `None` where the text is neither spelling: a row written by something
    /// that did not share this enum.
    pub fn from_wire(value: &str) -> Option<WhenRefused> {
        WhenRefused::ALL
            .iter()
            .copied()
            .find(|w| w.as_wire() == value)
    }

    /// What this criterion actually does on a refusal, once the Job's setting
    /// and the criterion's own declaration are folded together.
    ///
    /// **The Job's setting overrides both ways**, per this module's own
    /// comment — the one exception is `declared_plan_drift`, which this
    /// function cannot see: `crate::WhenRefused` folds a *declared* criterion's
    /// [`OnRefusal`], and Fleet's own drift look is not one, so the absolute
    /// rule that it never refuses lives at the call site that builds its
    /// criterion, not here.
    pub fn resolve(&self, declared: OnRefusal) -> OnRefusal {
        match self {
            WhenRefused::PerCriterion => declared,
            WhenRefused::AlwaysAsk => OnRefusal::Ask,
            WhenRefused::AlwaysRefuse => OnRefusal::Refuse,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn per_criterion_defers_to_what_was_declared() {
        assert_eq!(
            WhenRefused::PerCriterion.resolve(OnRefusal::Ask),
            OnRefusal::Ask
        );
        assert_eq!(
            WhenRefused::PerCriterion.resolve(OnRefusal::Refuse),
            OnRefusal::Refuse
        );
    }

    #[test]
    fn always_ask_overrides_a_criterion_authored_refuse() {
        assert_eq!(
            WhenRefused::AlwaysAsk.resolve(OnRefusal::Refuse),
            OnRefusal::Ask
        );
    }

    #[test]
    fn always_refuse_overrides_a_criterion_that_defaults_to_asking() {
        assert_eq!(
            WhenRefused::AlwaysRefuse.resolve(OnRefusal::Ask),
            OnRefusal::Refuse
        );
    }

    #[test]
    fn the_wire_spelling_round_trips() {
        for setting in WhenRefused::ALL {
            assert_eq!(WhenRefused::from_wire(setting.as_wire()), Some(*setting));
        }
    }
}
