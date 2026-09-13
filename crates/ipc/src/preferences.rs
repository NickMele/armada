//! A person's Bridge preferences, kept the way [`crate::limits`] are kept —
//! Fleet-wide, surviving a relaunch, and read back whole after every save.
//!
//! **`value` is a plain `bool` because the one preference this build has is
//! one.** A second preference of a different shape is a decision for whoever
//! adds it, not a generality bought here on spec.

use serde::{Deserialize, Serialize};

/// Every preference Fleet knows, and what is in force for each.
///
/// **Flat, one field per known preference** — `store::Preferences`' shape,
/// carried across the wire rather than restated as a map: a name outside this
/// struct cannot be read, which is the closed set on this side of the seam.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct Preferences {
    pub where_things_are_open: bool,
}

/// A save — `POST /preferences/save`. **One preference, not the set.**
/// `SaveLimits` sends every field it has an opinion on in one request because
/// the three limits are always read and saved together; a preference is read
/// as a whole struct but saved one at a time; leaving the rest is stated
/// rather than achieved by omission.
///
/// **`name` is a plain string, not a closed wire type.** The set it must
/// belong to is `store`'s own `CHECK`, and a name outside it is refused by
/// name — `fleet.unknown_preference` — rather than failing to decode, which is
/// what a closed wire type here would do instead and would say nothing about
/// which name was sent.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct SavePreference {
    pub name: String,
    pub value: bool,
}
