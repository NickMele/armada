//! Every sentence a Helm reply reads as when it fails, written once here.
//!
//! **The whole sentence is Fleet's and no surface frames it.** The thread and
//! the record each prepended their own lead-in, so the reply-budget timeout
//! read `No reply came. no reply came within 900 seconds, so the session was
//! ended`. `docs/contracts/design-system.md` P6 — Fleet's own strings are
//! identical every time — makes a per-surface lead-in a second producer of one
//! sentence, and the prose rule *no sentence that survives deletion without
//! loss* is what deletes the lead-in rather than the tail.
//!
//! So each function returns a statement that stands alone: P5, event first and
//! cause after. **Lower case**, because the record draws these in its
//! `at · who · what` column; a proper noun keeps its capital.

use std::time::Duration;

/// **Says the session was ended**, or a person is left asking whether
/// something is still running.
pub(crate) fn nothing_came_back(budget: Duration) -> String {
    format!(
        "no reply came within {} seconds, so the session was ended",
        budget.as_secs()
    )
}

/// The process ran and exited without ever naming a session.
///
/// **One sentence however little it left behind.** Either half is absent on
/// some machines, and the template this replaces rendered that as `the session
/// never started (): `.
pub(crate) fn never_started(exited: &str, complaint: &str) -> String {
    match (exited.trim(), complaint.trim()) {
        ("", "") => "the session never started, and left nothing behind saying why".to_string(),
        ("", said) => format!("the session never started: {said}"),
        (status, "") => {
            format!("the session never started, and left nothing behind saying why ({status})")
        }
        (status, said) => format!("the session never started: {said} ({status})"),
    }
}

/// **One frame over six causes** — the model, the door's file, the
/// environment, the resume, the argv. Each is an adapter's sentence about a
/// value: `no model was named`, alone in a thread, says nothing about Helm.
pub(crate) fn no_session_started(why: impl std::fmt::Display) -> String {
    format!("the session could not be started: {why}")
}

/// The door could not be published, so the session would have come up unable
/// to reach Fleet.
pub(crate) fn door_unconfigured(why: impl std::fmt::Display) -> String {
    format!("Helm's door would not be configured: {why}")
}

/// **The program is named**, because the fix is usually the `PATH` or the
/// override that chose it.
pub(crate) fn would_not_start(program: &str, why: impl std::fmt::Display) -> String {
    format!("{program} would not start: {why}")
}

/// The process started and its pipes were gone before Fleet took them.
pub(crate) fn lost_before_held() -> String {
    "the session was gone before Fleet could hold on to it".to_string()
}

/// The turn would not encode, so nothing was sent.
pub(crate) fn message_not_encoded(why: impl std::fmt::Display) -> String {
    format!("the message could not be encoded for the session: {why}")
}

/// The turn would not write to the session's input.
pub(crate) fn message_not_written(why: impl std::fmt::Display) -> String {
    format!("the message could not be written to the session: {why}")
}

/// The stored session id would not read before the message was carried.
pub(crate) fn session_would_not_read(why: impl std::fmt::Display) -> String {
    format!("the conversation's stored session would not read: {why}")
}

/// A session the agent no longer holds would not drop from the store.
pub(crate) fn lost_session_not_forgotten(why: impl std::fmt::Display) -> String {
    format!("the lost session would not be forgotten: {why}")
}

/// **This one says Helm answered**, which is why no surface may put `no reply
/// came` in front of it: that read as both at once.
pub(crate) fn session_not_kept(why: impl std::fmt::Display) -> String {
    format!("Helm answered, and its session would not be kept for next time: {why}")
}

/// The resume was refused and the lost-session retry did not apply.
pub(crate) fn no_session_to_resume() -> String {
    "the agent had no session to resume, so nothing was carried".to_string()
}

/// **An error rather than a thread message**, under
/// `fleet.helm_session_unreadable`. A sentence for the same reason: `message`
/// is what renders when the code's own copy is missing.
pub(crate) fn stored_session_unreachable(why: impl std::fmt::Display) -> String {
    format!("the conversation's stored session could not be reached: {why}")
}
