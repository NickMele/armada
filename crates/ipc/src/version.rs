//! What each side speaks, and what a mismatch between them means.
//!
//! `protocol-version.toml` at the repo root is the source of truth for both
//! numbers and `build.rs` embeds them here. `docs/practices/protocol.md` holds
//! the change-by-change table that decides which of the two moves.

use std::fmt;

use serde::{Deserialize, Deserializer, Serialize};

// The two numbers, from `protocol-version.toml`. Private: nothing outside this
// module should be able to hold half a version.
include!(concat!(env!("OUT_DIR"), "/protocol_version.rs"));

/// What this build speaks.
pub const PROTOCOL_VERSION: ProtocolVersion = ProtocolVersion::new(PROTOCOL_MAJOR, PROTOCOL_MINOR);

/// One side's protocol version.
///
/// **One wire field carrying both numbers, not two fields.** Two would let
/// either side compare the majors and forget the minors, which is the bug this
/// type replaces — [`ProtocolVersion::reading`] is the only comparison, and
/// there is no bare number left to spell `==` against.
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Serialize)]
pub struct ProtocolVersion {
    /// Moves when a message an older peer already parses stops parsing the same
    /// way. A mismatch is refused in either direction.
    pub major: u32,
    /// Moves when the change is additive only. A mismatch is survivable in one
    /// direction — see [`Skew`].
    pub minor: u32,
}

impl ProtocolVersion {
    pub const fn new(major: u32, minor: u32) -> Self {
        Self { major, minor }
    }

    /// What a Bridge speaking `self` should do about a Fleet speaking `fleet`.
    ///
    /// The receiver is Bridge's own version because Bridge is the side that
    /// decides: it reads Fleet's from the runtime file before opening a socket.
    pub fn reading(self, fleet: Self) -> Skew {
        if self.major != fleet.major {
            Skew::Incompatible
        } else if fleet.minor == self.minor {
            Skew::Same
        } else if fleet.minor > self.minor {
            Skew::FleetAhead
        } else {
            Skew::FleetBehind
        }
    }
}

impl fmt::Display for ProtocolVersion {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}.{}", self.major, self.minor)
    }
}

/// What a version read off a peer means for the connection.
///
/// **Only [`Skew::Same`] and [`Skew::FleetAhead`] may connect**, and the
/// asymmetry is the whole point. A minor bump is additive-only, so the newer
/// side's additions are things the older side never asks for and never reads —
/// safe when Fleet is the newer one, because Bridge ignores what it does not
/// recognise. Reversed, it is not: a newer Bridge may require a field an older
/// Fleet was built before sending, and additive-only promises nothing about
/// what a *newer reader* needs.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Skew {
    /// Both sides speak the same protocol. Connect, and say nothing.
    Same,
    /// Fleet has additions Bridge does not know about. Connect, and carry a
    /// banner: what Bridge draws is complete, and there is more it cannot see.
    FleetAhead,
    /// Bridge has additions Fleet was built before. Refuse — the missing field
    /// would arrive as a hole partway through a Job rather than at startup.
    FleetBehind,
    /// Different protocols. The lifeboat, not a banner.
    Incompatible,
}

impl Skew {
    /// Whether the full protocol may be spoken at all.
    pub fn connects(self) -> bool {
        matches!(self, Skew::Same | Skew::FleetAhead)
    }
}

/// Read as `{"major":4,"minor":0}`, and also as a bare `4`.
///
/// The bare form is what version 4 shipped as, and it names a major at minor
/// zero — so a Fleet from before this pair existed still reads rather than
/// coming back as a runtime file nothing wrote.
impl<'de> Deserialize<'de> for ProtocolVersion {
    fn deserialize<D: Deserializer<'de>>(deserializer: D) -> Result<Self, D::Error> {
        #[derive(Deserialize)]
        #[serde(untagged)]
        enum Written {
            Pair { major: u32, minor: u32 },
            Bare(u32),
        }

        Ok(match Written::deserialize(deserializer)? {
            Written::Pair { major, minor } => Self { major, minor },
            Written::Bare(major) => Self { major, minor: 0 },
        })
    }
}
