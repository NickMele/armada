//! The ids and instants the wire spells as strings.
//!
//! Every one is a newtype rather than a bare `String`, for the reason
//! `core-model`'s own ids are: a `JobId` cannot be handed where a `ManifestId`
//! is wanted, and that costs one wrapper and removes a whole class of call.
//!
//! Each carries the conversion **both** ways at the Fleet boundary — `From<&…>`
//! for what leaves, `to_domain` for what arrives. Neither direction reads or
//! writes a spelling of its own: a wire id is the domain id's `as_str`, and
//! nothing here maps a variant to a name.

use core_model::Ulid;
use serde::{Deserialize, Serialize};

/// An id the wire spells as a string, over a `core-model` id newtype.
///
/// `#[serde(transparent)]`, so the wrapper is invisible on the wire and the
/// generated TypeScript is a string rather than an object with one field.
macro_rules! wire_id {
    ($(#[$meta:meta])* $name:ident, $domain:ty) => {
        $(#[$meta])*
        #[derive(Clone, Debug, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize)]
        #[serde(transparent)]
        pub struct $name(String);

        impl $name {
            /// Carry an id Fleet minted. Nothing here mints one — **Fleet is
            /// the sole authority for the ids that name records**, and an id
            /// invented by a peer joins to nothing.
            pub fn carried(value: impl Into<String>) -> Self {
                $name(value.into())
            }

            pub fn as_str(&self) -> &str {
                &self.0
            }

            /// The domain id an arriving one names. The inbound half of the
            /// boundary conversion, and the only place it happens.
            pub fn to_domain(&self) -> $domain {
                <$domain>::carried(Ulid::carried(self.0.clone()))
            }
        }

        impl From<&$domain> for $name {
            fn from(id: &$domain) -> Self {
                $name(id.as_str().to_string())
            }
        }
    };
}

wire_id! {
    /// The Job a message is about.
    JobId, core_model::JobId
}
wire_id! {
    /// The Drone working a Job. **Presence, not state** — a Job with none is a
    /// Job no process is on, which also suspends the liveness clock.
    DroneId, core_model::DroneId
}
wire_id! {
    /// The project a Job belongs to, and the Job Board's scoping key.
    ManifestId, core_model::ManifestId
}
wire_id! {
    /// The WorkflowDef a Job follows.
    WorkflowId, core_model::WorkflowId
}

/// A step's identifier, from the WorkflowDef and never generated.
///
/// Not a [`wire_id`]: the domain type is a string the WorkflowDef author wrote,
/// not a minted id, so there is no `Ulid` underneath to carry.
#[derive(Clone, Debug, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize)]
#[serde(transparent)]
pub struct StepId(String);

impl StepId {
    pub fn carried(value: impl Into<String>) -> Self {
        StepId(value.into())
    }
    pub fn as_str(&self) -> &str {
        &self.0
    }
    pub fn to_domain(&self) -> core_model::StepId {
        core_model::StepId::new(self.0.clone())
    }
}

impl From<&core_model::StepId> for StepId {
    fn from(id: &core_model::StepId) -> Self {
        StepId(id.as_str().to_string())
    }
}

/// One question a Drone asked, by the id Fleet minted for it.
///
/// Not a [`wire_id`]: **a question is not a record**. It lives on the working
/// slot for as long as it is unanswered and is gone the instant it is answered,
/// exactly as [`JudgeInFlight`](crate::JudgeInFlight) does, so there is no
/// `core-model` id underneath for this to carry. Fleet still mints it, and for
/// the reason every other id is minted there: an answer naming an id a peer
/// invented joins to nothing.
///
/// **It is what makes a stale answer a refusal rather than a wrong Job.** A
/// window left open on a question that has since been answered and replaced
/// would otherwise send a label that matches the *new* question's options by
/// coincidence, and the cost of that here is a dispatched Job that runs and
/// spends.
#[derive(Clone, Debug, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize)]
#[serde(transparent)]
pub struct QuestionId(String);

impl QuestionId {
    pub fn carried(value: impl Into<String>) -> Self {
        QuestionId(value.into())
    }
    pub fn as_str(&self) -> &str {
        &self.0
    }
}

impl From<&core_model::Ulid> for QuestionId {
    fn from(id: &core_model::Ulid) -> Self {
        QuestionId(id.as_str().to_string())
    }
}

/// One proposal the Job proposer is reading, by the id Fleet minted for it.
///
/// [`QuestionId`]'s terms exactly, and for the same reason: **a proposal in
/// flight is not a record.** It exists between the request arriving and the
/// Jobs being minted, and nothing stores one — a proposal that produced Jobs is
/// those Jobs, and one that did not is a refusal. So there is no `core-model`
/// id underneath for this to carry.
///
/// **It is what a stop names.** Nothing else on the wire identifies a call that
/// has not finished: a proposal has no Job to hang off yet, which is exactly
/// what separates it from [`JudgeInFlight`](crate::JudgeInFlight), and a stop
/// naming a request's text would stop the wrong one the moment two people
/// dispatch the same words.
#[derive(Clone, Debug, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize)]
#[serde(transparent)]
pub struct ProposalId(String);

impl ProposalId {
    pub fn carried(value: impl Into<String>) -> Self {
        ProposalId(value.into())
    }
    pub fn as_str(&self) -> &str {
        &self.0
    }
}

impl From<&core_model::Ulid> for ProposalId {
    fn from(id: &core_model::Ulid) -> Self {
        ProposalId(id.as_str().to_string())
    }
}

/// An acceptance criterion's frozen identifier.
#[derive(Clone, Debug, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize)]
#[serde(transparent)]
pub struct CriterionId(String);

impl CriterionId {
    pub fn carried(value: impl Into<String>) -> Self {
        CriterionId(value.into())
    }
    pub fn as_str(&self) -> &str {
        &self.0
    }
    pub fn to_domain(&self) -> core_model::CriterionId {
        core_model::CriterionId::new(self.0.clone())
    }
}

impl From<&core_model::CriterionId> for CriterionId {
    fn from(id: &core_model::CriterionId) -> Self {
        CriterionId(id.as_str().to_string())
    }
}

/// RFC3339, UTC, millisecond precision — the log envelope's `Timestamp`.
///
/// **Time is injected, never read.** Nothing in this crate produces one; every
/// instant on the wire came off a record that was handed it.
#[derive(Clone, Debug, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize)]
#[serde(transparent)]
pub struct Instant(String);

impl Instant {
    pub fn carried(value: impl Into<String>) -> Self {
        Instant(value.into())
    }
    pub fn as_str(&self) -> &str {
        &self.0
    }
    pub fn to_domain(&self) -> core_model::Timestamp {
        core_model::Timestamp::from_rfc3339(self.0.clone())
    }
}

impl From<&core_model::Timestamp> for Instant {
    fn from(at: &core_model::Timestamp) -> Self {
        Instant(at.as_str().to_string())
    }
}
