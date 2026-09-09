//! Giving one Job more money, and who is giving it.
//!
//! **A Job over its cost cap waits at `queued` reading `over_budget` until
//! somebody raises the cap** — `docs/concepts/machine.md`, Budget, says so and
//! nothing on this seam carried the act. So the label was a dead end: the
//! remedy was a number, the number was a machine-wide setting, and raising it
//! for one Job meant raising it for every Job or restarting Fleet.
//!
//! **Two numbers cross and neither is a verdict**, which is
//! [`JobSpend`](crate::JobSpend)'s rule one route over: the new ceiling, and
//! which surface a person acted through. Whether the Job is then inside its
//! budget is the pair being compared, and the answer comes back on the summary
//! as `queued_reason`.

use serde::{Deserialize, Serialize};

/// A new cost ceiling for one Job.
///
/// **It raises and never lowers.** A value at or under the cap in force is
/// refused at the Fleet boundary rather than here, for
/// [`Overruled`](crate::Overruled)'s reason: a decoded request is well-formed,
/// and a raise that raises nothing is a value that cannot work. Lowering a
/// running Job's ceiling is a different act nobody has asked for, and it would
/// strand work mid-flight rather than let it finish.
///
/// **The turn cap is not here.** `budget-turn-cap-per-job` catches what a wide
/// dollar ceiling misses, and a Job that turns and turns was usually not
/// askable as written — the remedy there is the brief, not a bigger number. One
/// body carrying either would be one route meaning whichever the caller had in
/// mind, and this one is named for the number it moves.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct CapRaise {
    /// What this Job may cost from now on, in millionths of a dollar — the unit
    /// [`JobSpend`](crate::JobSpend) reads in, so the figure a person typed and
    /// the figure they were shown are the same integer.
    pub cost_cap_micros: u64,
    /// Which surface the raise came through. See [`RaisedBy`].
    pub raised_by: RaisedBy,
}

/// Which surface a raise came through, and therefore what it is allowed to ask
/// for.
///
/// **A statement of provenance, not a credential.** Nothing on this seam
/// authenticates anybody — Fleet and Bridge share a machine, and
/// `docs/practices/protocol.md` is explicit that the lifeboat and the protocol
/// alike carry no auth concept. So this field says which surface composed the
/// request, and it is filled in by that surface rather than by whoever is using
/// it: Bridge sends [`Person`](RaisedBy::Person) because a person pressed a
/// control, and the Helm tool adapter will send [`Helm`](RaisedBy::Helm)
/// because it is Fleet's own code wrapping a model's request. **The model has
/// no field for this**, exactly as a Drone has no argument that widens its own
/// allowlist.
///
/// The one vocabulary in this crate besides [`Claim`](crate::Claim) that spells
/// no domain value, and for that type's reason: nothing in `core-model` decides
/// it, because it is not a state anything is in.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum RaisedBy {
    /// Somebody pressed a control. **Unbounded** — the budget is theirs, and a
    /// ceiling on what its owner may set is a setting arguing with the person
    /// who set it.
    Person,
    /// Helm asked on somebody's behalf. **Bounded**, and
    /// `crates/fleet/src/raising.rs` carries the bound and the argument for its
    /// shape: an agent that can lift its own budget has no budget.
    Helm,
}

impl RaisedBy {
    /// Every variant, in the order a picker would offer them.
    pub const ALL: &'static [RaisedBy] = &[RaisedBy::Person, RaisedBy::Helm];

    /// The wire value.
    pub fn as_wire(&self) -> &'static str {
        match self {
            RaisedBy::Person => "person",
            RaisedBy::Helm => "helm",
        }
    }

    /// Read a spelling back. `None` where nothing spells it.
    pub fn from_wire(value: &str) -> Option<RaisedBy> {
        RaisedBy::ALL
            .iter()
            .copied()
            .find(|raised| raised.as_wire() == value)
    }
}
