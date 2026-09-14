//! The note tool: a short message from one working Drone to another Job's. #1000.
//!
//! **Taken, never answered.** The receipt says Fleet took the note; it reaches
//! the other Job's Drone with that Drone's next peer turn, marked as another
//! Drone's words. Who may be addressed is `fleet::peers`' answer, not this
//! module's.

use serde_json::{json, Map, Value};

use super::tools::{closed, filled, NotAnArgument};

/// The note tool's name, bare.
pub const NOTE_TOOL: &str = "leave_note";

/// The two fields the note tool takes. Public for `EVIDENCE_FIELDS`' reason.
pub const NOTE_FIELDS: &[&str] = &["to", "note"];

/// The longest note, in characters. A note is a sentence of warning, not a
/// design, and it lands in another Drone's session uninvited.
pub const MOST_NOTE_CHARS: usize = 600;

/// What a Drone leaves for another Job's Drone.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct LeaveNote {
    /// The other Job's handle, as a peer turn prints it: `12-fix-the-writer`.
    pub to: String,
    /// What that Drone needs to know. Never blank, never over [`MOST_NOTE_CHARS`].
    pub note: String,
}

/// The note tool's arguments. Too long is refused rather than cut, because a
/// cut note says something its writer did not.
pub(super) fn left(arguments: &Map<String, Value>) -> Result<LeaveNote, NotAnArgument> {
    closed(arguments, NOTE_TOOL, NOTE_FIELDS)?;
    let to = filled(arguments, "to")?.trim().to_string();
    let note = filled(arguments, "note")?;
    if note.chars().count() > MOST_NOTE_CHARS {
        return Err(NotAnArgument::TooLong {
            field: "note",
            most: MOST_NOTE_CHARS,
        });
    }
    Ok(LeaveNote { to, note })
}

/// The note tool, described by what it is for and what it is not.
pub(super) fn note_tool() -> Value {
    json!({
        "name": NOTE_TOOL,
        "description":
            "Leave a short note for the Drone working another Job in this \
             repository that changes files this Job changes — for example \
             \"I am adding migration V64, take V65\". Address it by the Job's \
             handle, exactly as an OTHER JOBS WRITING WHERE YOU ARE turn prints \
             it. The note reaches that Drone within a few minutes, marked as \
             your words. Nothing waits for a reply and none comes back through \
             this tool. It is not a way to ask a person anything: use \
             ask_question for that.",
        "inputSchema": {
            "type": "object",
            "properties": {
                "to": {
                    "type": "string",
                    "description": "The other Job's handle, such as 12-fix-the-writer.",
                },
                "note": {
                    "type": "string",
                    "maxLength": MOST_NOTE_CHARS,
                    "description": "What that Drone needs to know, in a sentence or two.",
                },
            },
            "required": NOTE_FIELDS,
        },
    })
}
