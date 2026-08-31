//! How many Drones Fleet may run, how many it is running, and what is holding
//! the next one back.
//!
//! **Fleet-wide, and not a Job's field.** `JobSummary::queued_reason` says why
//! *this row* has not started and folds four different situations into
//! `waiting_on_resources`, which is the one label `job-statuses.toml` grants a
//! `queued` Job. This says which of the four it is, once, for the whole Fleet —
//! so a Board row keeps the label it had and a status bar can say what nobody
//! could ask before.

use serde::de::Deserializer;
use serde::{Deserialize, Serialize, Serializer};

/// What Fleet's admission would answer right now.
///
/// # `occupied` is the roster's count and never a count of `running` rows
///
/// `Slots::count` is what admission itself measures the bound against. A count
/// derived from statuses disagrees with it: a Job that escalated on a refusal
/// keeps its Drone alive and idle, so a redirect costs no respawn, and it keeps
/// its place in the roster while its status is `escalated`. A status-derived
/// count would read "1 of 2" while Fleet admits nothing — wrong in exactly the
/// moment somebody is asking why nothing started.
///
/// # `held_by` is one answer, not a set
///
/// `fleet::admitting::Room` asks the bound before it reads the machine, so a
/// Fleet at its cap never pays three processes for a reading it would not have
/// acted on. That ordering is deliberate and is not relaxed for this payload:
/// "the cap is spent *and* the disk is full" cannot be carried. What crosses is
/// what is stopping admission now, and the next thing crosses once that clears.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct FleetCapacity {
    /// How many Jobs may be worked at once. `settings.concurrency-cap`.
    pub bound: u32,
    /// How many are being worked, from the roster admission counts against.
    pub occupied: u32,
    /// What stops the next Drone starting. **Absent means nothing does** — not
    /// "unknown", and not "Fleet did not look". An unreadable machine admits
    /// rather than refuses, so it is absent here too.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub held_by: Option<AdmissionHold>,
}

impl FleetCapacity {
    /// The one sanctioned way to build one, so `held_by` is spelled by the
    /// registry rather than by a caller.
    pub fn of(
        bound: usize,
        occupied: usize,
        held_by: Option<core_model::AdmissionHold>,
    ) -> FleetCapacity {
        FleetCapacity {
            // Saturating rather than `as`, which would wrap a bound nobody can
            // configure into a small number that looks plausible.
            bound: u32::try_from(bound).unwrap_or(u32::MAX),
            occupied: u32::try_from(occupied).unwrap_or(u32::MAX),
            held_by: held_by.map(AdmissionHold::from),
        }
    }
}

/// Which one thing is holding the next Drone back, as the registry spells it.
///
/// # The one open set on this seam, and why it is the exception
///
/// Every other closed set here is a [`wire_enum!`](crate::enums) over a
/// `core-model` enum, whose deserializer **refuses** a spelling the domain does
/// not have. That is right for `JobStatus`: Bridge matches on it to choose a
/// screen, so a variant it never heard of is a Job it would draw wrong.
///
/// This one is only ever *written* by Fleet and *read as opaque* by Bridge,
/// looked up in the generated vocabulary — already
/// `Record<string, Rendering | undefined>`, and already answering for a key it
/// does not hold. `docs/practices/protocol.md` gives that its own row: a
/// variant added to a set the other side reads as opaque is **minor**.
///
/// **That is the whole point of carrying a spelling.** A fifth reason a Job
/// sits at `queued` — `#51`'s budget cap next — is a variant in `core-model`, a
/// row in `enum-verbs.toml` and a codegen run, and moves neither number in
/// `protocol-version.toml`.
///
/// **Nothing mints a spelling.** Both ways in go through `core-model`, and
/// [`AdmissionHold::from_wire`] answers `None` for a word the registry does not
/// carry. The tolerance is on the reading side alone.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct AdmissionHold(String);

impl AdmissionHold {
    /// The wire value, which is also the `enum-verbs.toml` key.
    pub fn as_wire(&self) -> &str {
        &self.0
    }

    /// Build one from a spelling, **checked against the registry**. `None`
    /// where it is not one this build has — the same refusal every other set
    /// here makes on the way in, and deliberately not the one made on the way
    /// back off the wire.
    ///
    /// It exists so a client holding a string rather than a domain value can
    /// still build a message, which is what `api`'s fake daemon does and what
    /// keeps "a fake built out of `ipc` alone" true.
    pub fn from_wire(spelling: &str) -> Option<AdmissionHold> {
        core_model::AdmissionHold::from_wire(spelling).map(AdmissionHold::from)
    }

    /// The domain value, where this Fleet's `core-model` has one. `None` is a
    /// spelling from a newer peer, and is a real answer rather than a fault.
    pub fn domain(&self) -> Option<core_model::AdmissionHold> {
        core_model::AdmissionHold::from_wire(&self.0)
    }
}

impl From<core_model::AdmissionHold> for AdmissionHold {
    fn from(hold: core_model::AdmissionHold) -> AdmissionHold {
        AdmissionHold(hold.as_wire().to_owned())
    }
}

impl Serialize for AdmissionHold {
    fn serialize<S: Serializer>(&self, out: S) -> Result<S::Ok, S::Error> {
        out.serialize_str(&self.0)
    }
}

impl<'de> Deserialize<'de> for AdmissionHold {
    /// **Any spelling is read back**, which is the difference from every other
    /// set on this seam and is argued on the type above. A reader that cannot
    /// render it says so from the registry's own gap list; a reader that
    /// refused the message would lose the two numbers beside it as well.
    fn deserialize<D: Deserializer<'de>>(input: D) -> Result<Self, D::Error> {
        String::deserialize(input).map(AdmissionHold)
    }
}
