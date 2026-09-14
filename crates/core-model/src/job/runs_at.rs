//! Where a Check runs: in a Drone's own mid-step run, at a step's gate, or once
//! before the work is handed off. #849.
//!
//! **The repository declares it once and every workflow inherits it**, the rule
//! `when` and `narrow` already follow — so there is no step-level spelling and
//! nothing a Drone can choose.

/// Where a Manifest Check runs, as `checks.<name>.runs_at` declared it.
///
/// **`Everywhere` is the absent key**, which is what every Check did before this
/// existed: a Drone's run asks it and every gate naming it runs it.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum RunsAt {
    /// A Drone's own run and every gate. The default.
    #[default]
    Everywhere,
    /// Every gate, and never a Drone's own run.
    Gate,
    /// Only at the gate of the last step before handoff, and there only once
    /// every other Check on that gate has passed.
    Handoff,
}

impl RunsAt {
    /// Every value, in the order a reader meets them.
    pub const ALL: [RunsAt; 3] = [RunsAt::Everywhere, RunsAt::Gate, RunsAt::Handoff];

    /// The spelling `armada.yml`, a frozen row and the wire all use.
    pub fn as_wire(self) -> &'static str {
        match self {
            RunsAt::Everywhere => "everywhere",
            RunsAt::Gate => "gate",
            RunsAt::Handoff => "handoff",
        }
    }

    /// The value a spelling names, or `None` for one this build has no value for.
    pub fn from_wire(written: &str) -> Option<RunsAt> {
        RunsAt::ALL
            .into_iter()
            .find(|runs| runs.as_wire() == written)
    }

    /// Whether a Drone's own mid-step run asks this Check.
    pub fn mid_step(self) -> bool {
        self == RunsAt::Everywhere
    }
}
