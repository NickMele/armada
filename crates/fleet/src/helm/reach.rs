//! What Helm may do, as one predicate over the operations the agent door offers.
//!
//! **A denylist of acts, not an allowlist.** `#73` drew Helm's line as an
//! allowlist inside the `agent_access` column the door shares with every
//! agent on the machine; Nick's later ruling (`#1150`) reversed the default —
//! Helm may call any command it is offered, when a person asks it to in
//! conversation, except what is named in [`RESERVED`]. The column still keeps
//! Helm and a Drone apart: `agent_access = "No"` rows stay invisible to every
//! agent regardless of this list, for reasons that have nothing to do with
//! Helm's authority (a Drone's own exclusions, or a read too costly to hand
//! every session) — this predicate only ever sees what the door already
//! offered.

use ipc::door::Reachable;

/// How far this machine lets Helm go: the Machine setting *Helm action
/// authority*, resolved. `Fleet::helm_authority` (`crate::daemon::seams`) is
/// the one place the setting becomes one of these two; `#943`.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Authority {
    /// The setting's shipped value: every read, and every act not in
    /// [`RESERVED`].
    Acting,
    /// Every read, and no act.
    ReadOnly,
}

/// The one act that stays a person's, independent of what the machine's
/// authority setting allows. `undo_run` reverses a run's own files, and
/// nothing else here draws the line — `#1150` named it the sole exception to
/// "any command Helm is offered, on request."
pub(super) const RESERVED: &[&str] = &["undo_run"];

/// Whether Helm, under `authority`, may call one operation the door offers.
///
/// Every query is a read and every read is Helm's. A command is Helm's where
/// the machine lets Helm act at all and the operation is not [`RESERVED`].
pub fn may(authority: Authority, offered: &Reachable) -> bool {
    match offered.kind {
        "query" => true,
        "command" => authority == Authority::Acting && !RESERVED.contains(&offered.operation),
        _ => false,
    }
}
