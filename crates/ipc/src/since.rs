//! What happened on the stream since a cursor, counted rather than carried.
//!
//! **An agent's substitute for the socket.** It cannot be interrupted mid-turn,
//! so it asks at the start of each one; every event returned whole would stay
//! in its session for the rest of it, so this returns how many and of what.

use serde::{Deserialize, Serialize};

use crate::event::Cursor;

/// How many events of one kind crossed.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct EventTally {
    /// The kind as `/events` publishes it — `job.state_changed`. **A string and
    /// not a closed set**: a reader that refused an unknown kind would lose the
    /// counts beside it, and nothing here matches on the value.
    pub kind: String,
    pub count: u64,
}

/// The answer to one poll.
///
/// **The window is bounded and says when it lost something**, which is the
/// promise `Missed` makes on the socket: nobody may silently believe they hold
/// the whole history.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct EventsSince {
    /// The cursor that was asked from.
    pub from: Cursor,
    /// The position the next published event will take. **Ask from this next
    /// time** — a caller adding up counts to derive it would drift the first
    /// time the window dropped anything.
    pub upto: Cursor,
    /// One row per kind that crossed, ordered by kind. Empty means nothing
    /// happened, which is a real and common answer.
    pub kinds: Vec<EventTally>,
    /// How many events after `from` were dropped before they could be counted.
    /// **Absent means none were.**
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub missed: Option<u64>,
}
