//! The session each Helm conversation resumes. `#939`.
//!
//! **Kept here so a Fleet restart or a reboot resumes it** rather than starting
//! over: no process idles between messages, so this row is the whole of what
//! carries a conversation across a gap.
//!
//! **Keyed by conversation, and the key is Fleet's to spell.** It is a
//! repository's Manifest id today; a session per topic would write a different
//! key into the same table.

use core_model::Timestamp;
use rusqlite::OptionalExtension;

use crate::error::{fault, DatabaseFault, WriteError};
use crate::open::Store;

/// Version 64 — one resumable session per Helm conversation.
///
/// **No foreign key**, `manifest_allowed_commands`' reason: a conversation
/// outlives every Job, so `forget_job` has nothing to delete here.
pub(crate) const V64: &str = r#"
CREATE TABLE helm_sessions (
    conversation TEXT PRIMARY KEY,
    session_id   TEXT NOT NULL,
    kept_at      TEXT NOT NULL
) STRICT;
"#;

impl Store {
    /// The session this conversation resumes, or `None` where the next message
    /// starts one.
    pub fn helm_session(&self, conversation: &str) -> Result<Option<String>, DatabaseFault> {
        self.conn
            .query_row(
                "SELECT session_id FROM helm_sessions WHERE conversation = ?1",
                [conversation],
                |row| row.get(0),
            )
            .optional()
            .map_err(fault("reading a Helm conversation's session"))
    }

    /// Keep the session a reply ran in. **A resumed session reports the id it
    /// was given**, spike 016, so this rewrites the same value on every reply
    /// and moves only `kept_at`.
    pub fn keep_helm_session(
        &mut self,
        conversation: &str,
        session_id: &str,
        at: &Timestamp,
    ) -> Result<(), WriteError> {
        self.conn
            .execute(
                "INSERT INTO helm_sessions (conversation, session_id, kept_at)
                 VALUES (?1, ?2, ?3)
                 ON CONFLICT (conversation)
                 DO UPDATE SET session_id = excluded.session_id, kept_at = excluded.kept_at",
                (conversation, session_id, at.as_str()),
            )
            .map(|_| ())
            .map_err(fault("keeping a Helm conversation's session"))
            .map_err(WriteError::Database)
    }

    /// Forget the session, so the next message starts a new one. `true` where
    /// one was held.
    pub fn forget_helm_session(&mut self, conversation: &str) -> Result<bool, WriteError> {
        self.conn
            .execute(
                "DELETE FROM helm_sessions WHERE conversation = ?1",
                [conversation],
            )
            .map(|deleted| deleted > 0)
            .map_err(fault("forgetting a Helm conversation's session"))
            .map_err(WriteError::Database)
    }
}
