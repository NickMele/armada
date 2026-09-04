//! The scope-request tool. Its own module for [`ask`](mod@super::ask)'s
//! reason: a tool is a name, a field list, an argument type, a parser and a
//! schema, and all five of one tool live together.
//!
//! **Two scope tools, and they are not two spellings of one.**
//! [`SCOPE_TOOL`](super::tools::SCOPE_TOOL) says where *this part's* work will
//! be, costs nothing and is replaced by calling it again. This asks the *whole
//! task's* stated scope to grow, which is answered rather than taken — so a
//! Drone whose plan turned out wrong calls the other one.
//!
//! **The answer is in the reply.** What answers it is a Judge call with a
//! budget on it, so it is held open like
//! [`run_checks`](super::tools::CHECKS_TOOL) rather than receipted like
//! [`ask_question`](super::ask::ASK_TOOL), whose answer is a person's and has
//! no budget to wait against.

use serde_json::{json, Map, Value};

use super::tools::{closed, filled, list, NotAnArgument};

/// The scope-request tool's name, bare.
pub const WIDEN_TOOL: &str = "request_scope";

/// The two fields it takes. Public for `EVIDENCE_FIELDS`' reason: a second
/// reader needs the same spelling, and a rename has to break it rather than
/// silently empty it.
pub const WIDEN_FIELDS: &[&str] = &["paths", "reason"];

/// What a Drone asks the task's scope to become, and why.
///
/// **No field for a path to remove.** Handing scope back is a person's act and
/// costs nothing to nobody, so there is no argument here through which a Drone
/// could narrow the task it was given — which is the property, not a check.
///
/// **No field for an acceptance criterion.** A criterion added at a widening is
/// the person's; a Drone does not raise its own bar, and it has no field here
/// to raise it with.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct RequestScope {
    /// Repository-relative, and never empty: a request for no paths is a
    /// request for nothing, and it is refused by name rather than accepted as
    /// a call that did nothing.
    pub paths: Vec<String>,
    /// Why, in the Drone's own words. **Never blank** — it is the whole of what
    /// the decision is made on beyond the paths themselves, and it is read by
    /// the person a refusal reaches.
    pub reason: String,
}

/// The tool's arguments, with every rule the type states checked here and
/// refused by name.
pub(super) fn requested(arguments: &Map<String, Value>) -> Result<RequestScope, NotAnArgument> {
    closed(arguments, WIDEN_TOOL, WIDEN_FIELDS)?;
    let paths = list(arguments, "paths")?;
    if paths.is_empty() {
        return Err(NotAnArgument::AskedForNothing);
    }
    Ok(RequestScope {
        paths,
        reason: filled(arguments, "reason")?,
    })
}

/// The scope-request tool, as the client is shown it.
///
/// **The description says what it is not**, three times over: not the other
/// scope tool, not a way to get out of the part you were given, and not
/// something a person is waiting behind. A Drone that read this as the cheap
/// tool would spend a Judge call on every plan correction.
pub(super) fn widen_tool() -> Value {
    json!({
        "name": WIDEN_TOOL,
        "description":
            "Ask for a file the task does not say it writes. Use it when the \
             work genuinely needs a path outside the task's stated scope — not \
             to correct your own plan for this part, which is what \
             declare_scope is for and which costs nothing. The request is read \
             against the part you were given and answered here, so the call \
             takes a while and you carry on when it returns. If it is \
             granted, declare the paths with declare_scope before you write \
             there. You may ask once per part.",
        "inputSchema": {
            "type": "object",
            "properties": {
                "paths": {
                    "type": "array",
                    "items": { "type": "string" },
                    "description":
                        "Repository-relative paths — a file, or a directory and \
                         everything under it. Only what is outside the task's \
                         scope already; naming what is already in it asks for \
                         nothing.",
                },
                "reason": {
                    "type": "string",
                    "description":
                        "Why the part you were given needs these paths, in your \
                         own words. One or two sentences. If the request is \
                         turned down this is what a person reads beside it.",
                },
            },
            "required": ["paths", "reason"],
            "additionalProperties": false,
        },
    })
}
