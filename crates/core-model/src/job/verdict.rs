//! The verdict a human gate can answer with that neither advances the Job nor
//! ends it.
//!
//! **It lives here rather than in `config` because a Job keeps it.**
//! `verdict_routing` is frozen onto [`ResolvedStep`](crate::ResolvedStep) at
//! creation and read back off the `jobs.workflow` column, so the type has to be
//! reachable from the record and from `store` — and `config` depends on this
//! crate rather than the other way round. `config` re-exports it, so every
//! caller that already said `config::GateVerdict` still means this type.

use core::fmt;

/// A gate verdict that neither advances the Job nor ends it.
///
/// **One value, of the human gate's three.** `approve` advances and `reject`
/// ends the Job, so neither has anywhere to be routed to — which is why this is
/// an enum rather than the open string map the schema's JSON looks like. A
/// second non-terminal verdict widens this and every `match` on it at once.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub enum GateVerdict {
    RequestChanges,
}

impl GateVerdict {
    /// The word the file writes, and the key the frozen JSON is written under.
    pub fn as_wire(self) -> &'static str {
        match self {
            GateVerdict::RequestChanges => "request_changes",
        }
    }

    /// The verdict a stored key names, or `None` where this build has no such
    /// verdict.
    ///
    /// The mirror of [`as_wire`](GateVerdict::as_wire), for the same reason
    /// every other wire enum here has one: a frozen workflow comes back off a
    /// row, and a key written by a newer Armada is a refusal rather than a
    /// route quietly dropped.
    pub fn from_wire(word: &str) -> Option<GateVerdict> {
        match word {
            "request_changes" => Some(GateVerdict::RequestChanges),
            _ => None,
        }
    }
}

impl fmt::Display for GateVerdict {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(self.as_wire())
    }
}
