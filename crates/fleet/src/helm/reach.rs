//! What Helm may do, as one predicate over the operations the agent door offers.
//!
//! **An allowlist of acts rather than the `agent_access` column.** The column is
//! one set shared by every agent on the machine, and `#73` drew Helm's line
//! inside it: `undo_run` reads `Yes` and stays a person's. Reading the column
//! directly would also hand Helm any command marked `Yes` later, undecided.

use ipc::door::Reachable;

/// How far this machine lets Helm go: the Machine setting *Helm action
/// authority*, resolved. `Fleet::helm_authority` (`crate::daemon::seams`) is
/// the one place the setting becomes one of these two; `#943`.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Authority {
    /// The setting's shipped value: every read, and the acts in [`ACTS`].
    Acting,
    /// Every read, and no act.
    ReadOnly,
}

/// The acts `#73` decided are Helm's, keyed as the inventory keys them.
///
/// `propose_job` and `propose_from_request` read `Drafts only`, which the door
/// offers a Helm session and no other agent (`#941`).
pub(super) const ACTS: &[&str] = &[
    "examine_job",
    "propose_job",
    "propose_from_request",
    "raise_cost_cap",
    "raise_turn_cap",
    "redirect_drone",
    "show_again",
    "start_run",
    "stop_run",
    "start_server",
    "stop_server",
];

/// Whether Helm, under `authority`, may call one operation the door offers.
///
/// Every query is a read and every read is Helm's. A command is Helm's only
/// where it is one of [`ACTS`] and the machine lets Helm act.
pub fn may(authority: Authority, offered: &Reachable) -> bool {
    match offered.kind {
        "query" => true,
        "command" => authority == Authority::Acting && ACTS.contains(&offered.operation),
        _ => false,
    }
}
