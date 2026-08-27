//! The closed sets the wire carries, each over the domain enum that owns it.
//!
//! # Why these wrap a domain value instead of restating one
//!
//! `core-model` carries an `as_wire`/`from_wire` pair beside every enum, and
//! **the wire value is also the registry key**. A second copy of that mapping
//! here is the defect that was removed from `store`, so each type below holds
//! the domain value and spells it through the pair — there is no `match` in
//! this file mapping a variant to a name, and there cannot be one.
//!
//! What crosses the boundary is still the DTO. The domain enums carry no serde
//! derive and cannot be serialised at all; every impl below is written here,
//! and a variant added upstream is a major bump either way — the wire vocabulary
//! is the registry's, and only the registry may widen it.
//!
//! An arriving spelling the domain does not have is **refused, not defaulted**.
//! A peer that sent it does not share this vocabulary, and guessing which
//! variant it meant is how a Job renders as something it is not.

use core_model::Origin as DomainOrigin;
use serde::de::{Deserializer, Error as _};
use serde::{Deserialize, Serialize, Serializer};

/// A closed set, over the `core-model` enum that owns its spellings.
macro_rules! wire_enum {
    ($(#[$meta:meta])* $name:ident, $domain:ty, $what:literal) => {
        $(#[$meta])*
        #[derive(Clone, Copy, Debug, PartialEq, Eq)]
        pub struct $name($domain);

        impl $name {
            /// The wire value, which is also the registry key. Named as the
            /// domain pair names it, because it is the domain pair.
            pub fn as_wire(&self) -> &'static str {
                self.0.as_wire()
            }

            /// Read a spelling back. `None` where it is not one the registry
            /// has — the same refusal the deserializer makes, for a caller
            /// holding a string rather than a message.
            pub fn from_wire(spelling: &str) -> Option<$name> {
                <$domain>::from_wire(spelling).map($name)
            }

            /// The domain value. Total, because nothing builds one of these
            /// from a spelling the domain does not have.
            pub fn domain(&self) -> $domain {
                self.0
            }
        }

        impl From<$domain> for $name {
            fn from(value: $domain) -> Self {
                $name(value)
            }
        }

        impl Serialize for $name {
            fn serialize<S: Serializer>(&self, out: S) -> Result<S::Ok, S::Error> {
                out.serialize_str(self.0.as_wire())
            }
        }

        impl<'de> Deserialize<'de> for $name {
            fn deserialize<D: Deserializer<'de>>(input: D) -> Result<Self, D::Error> {
                let spelling = String::deserialize(input)?;
                <$domain>::from_wire(&spelling)
                    .map($name)
                    .ok_or_else(|| D::Error::custom(format!("`{spelling}` is not {}", $what)))
            }
        }
    };
}

wire_enum! {
    /// Where a Job is. Twelve, from `domain/job-statuses.toml`.
    JobStatus, core_model::JobStatus, "a Job status"
}
wire_enum! {
    /// What happens if the Job waits, not how it feels.
    Urgency, core_model::Urgency, "an urgency"
}
wire_enum! {
    /// Where a Job came from.
    Origin, DomainOrigin, "an origin"
}
wire_enum! {
    /// What one declared mechanical Check did. Five, from
    /// `domain/check-outcomes.toml`, and only `passed` advances a step.
    CheckOutcome, core_model::CheckOutcome, "a check outcome"
}
wire_enum! {
    /// What one Judge call answered about one criterion. **Neither word is an
    /// approval**: `met` reads "no objection", and there is no third variant.
    JudgeVerdict, core_model::JudgeVerdict, "a judge verdict"
}
wire_enum! {
    /// Who caused a transition. Three ways, and a message that did not record
    /// which never will.
    Actor, core_model::Actor, "an actor"
}
wire_enum! {
    /// Which verification source answers a criterion.
    CriterionSource, core_model::CriterionSource, "a criterion source"
}
wire_enum! {
    /// Where one step of the frozen WorkflowDef got to. The inner machine's
    /// states, which are rows rather than a field on the Job.
    StepState, core_model::StepState, "a step state"
}
wire_enum! {
    /// Which way a dependency edge points.
    DependencyDirection, core_model::DependencyDirection, "a dependency direction"
}

/// The four origins a Job proposed over the wire may claim.
///
/// **`sub_dispatched` is not among them, and that is the point.** A Job spawned
/// by a step of another Job is created inside Fleet and never proposed by a
/// peer, so the refusal is at the boundary rather than in a check behind it:
/// `Origin::top_level` is the narrowing `core-model` already carries, and this
/// deserialises through it.
///
/// It has no `as_wire` of its own upstream either — the spelling comes from the
/// `Origin` it converts into, which is why widening one cannot leave the other
/// behind.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct TopLevelOrigin(core_model::TopLevelOrigin);

impl TopLevelOrigin {
    pub fn as_wire(&self) -> &'static str {
        DomainOrigin::from(self.0).as_wire()
    }

    /// Read a spelling back. `None` for `sub_dispatched`, and for anything the
    /// registry does not have.
    pub fn from_wire(spelling: &str) -> Option<TopLevelOrigin> {
        DomainOrigin::from_wire(spelling)
            .and_then(|origin| origin.top_level())
            .map(TopLevelOrigin)
    }

    pub fn domain(&self) -> core_model::TopLevelOrigin {
        self.0
    }
}

impl From<core_model::TopLevelOrigin> for TopLevelOrigin {
    fn from(origin: core_model::TopLevelOrigin) -> Self {
        TopLevelOrigin(origin)
    }
}

impl Serialize for TopLevelOrigin {
    fn serialize<S: Serializer>(&self, out: S) -> Result<S::Ok, S::Error> {
        out.serialize_str(self.as_wire())
    }
}

impl<'de> Deserialize<'de> for TopLevelOrigin {
    fn deserialize<D: Deserializer<'de>>(input: D) -> Result<Self, D::Error> {
        let spelling = String::deserialize(input)?;
        DomainOrigin::from_wire(&spelling)
            .and_then(|origin| origin.top_level())
            .map(TopLevelOrigin)
            .ok_or_else(|| {
                D::Error::custom(format!(
                    "`{spelling}` is not an origin a proposed Job may claim"
                ))
            })
    }
}
