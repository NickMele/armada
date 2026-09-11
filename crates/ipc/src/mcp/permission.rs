//! The permission tool: what the harness asks before it runs a call the
//! Drone's allowlist does not cover, and what it takes back. Its own module for
//! [`ask`](mod@super::ask)'s reason: a tool is a name, a field list, an
//! argument type, a parser and a schema, and all five of one tool live
//! together.
//!
//! # The one tool the model never calls
//!
//! A Drone is spawned with this tool named as the one that answers a
//! permission prompt, so a call outside the allowlist arrives here rather than
//! being refused where nobody sees it. **It is in `tools/list` and it is not on
//! the allowlist, and both are deliberate.** The harness will not use a prompt
//! tool it cannot find in the list; the model is not shown it and has nothing
//! to call it for. `xtask/src/rules_toolbelt.rs` holds that pair.
//!
//! # A field it does not read is read past, not refused
//!
//! Every other tool here refuses a field it does not take, because the model
//! writes those arguments and a dropped field is a Drone believing something it
//! sent was read. Here the harness writes them, and it adds fields between
//! releases — refusing one would turn a harness upgrade into every command a
//! Drone asks about being refused. So the three fields read are required and
//! anything beside them is ignored.

use serde_json::{json, Map, Value};

use super::tools::{filled, NotAnArgument};

/// The permission tool's name, bare.
pub const PERMISSION_TOOL: &str = "permission";

/// The three fields it reads. Public for `EVIDENCE_FIELDS`' reason. **Not a
/// closed set** — see the module.
pub const PERMISSION_FIELDS: &[&str] = &["tool_name", "input", "tool_use_id"];

/// The harness's name for its shell tool: the one tool whose input this module
/// reads a field out of, because a command is what a person allows.
const SHELL: &str = "Bash";

/// One call the harness is asking about: which tool, with what, under which id.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct PermissionAsked {
    /// The tool the Drone reached for, in the harness's own spelling.
    pub tool: String,
    /// The call's arguments, exactly as the harness sent them.
    ///
    /// **Kept whole and handed back unchanged on an allow.** The answer names
    /// the input the call runs with, and an input Fleet rebuilt would be Fleet
    /// choosing what runs.
    pub input: Value,
    /// The harness's id for the call — the one the transcript rows carry, which
    /// is what joins this question to the row a person reads.
    pub call: String,
}

impl PermissionAsked {
    /// The command, where the call is a shell command and names one.
    ///
    /// `None` for every other tool, and for a shell call whose input carries no
    /// command text: there is nothing there a person could read and allow.
    pub fn command(&self) -> Option<&str> {
        if self.tool != SHELL {
            return None;
        }
        self.input.get("command").and_then(Value::as_str)
    }
}

/// The tool's arguments. A blank tool name or call id is refused by name —
/// neither could be joined to anything a person is shown.
pub(super) fn asked(arguments: &Map<String, Value>) -> Result<PermissionAsked, NotAnArgument> {
    let tool = filled(arguments, "tool_name")?;
    let field = "input";
    let input = arguments
        .get(field)
        .ok_or(NotAnArgument::Missing { field })?
        .clone();
    Ok(PermissionAsked {
        tool,
        input,
        call: filled(arguments, "tool_use_id")?,
    })
}

/// The permission tool, as the client is shown it.
///
/// **`additionalProperties` is true**, the one schema here where it is: the
/// module says why the harness's extra fields are read past.
pub(super) fn permission_tool() -> Value {
    json!({
        "name": PERMISSION_TOOL,
        "description":
            "Answers whether a call outside your allowed tools may run. The \
             harness asks this for you when you reach for one; do not call it \
             yourself.",
        "inputSchema": {
            "type": "object",
            "properties": {
                "tool_name": {
                    "type": "string",
                    "description": "The tool the call is for.",
                },
                "input": {
                    "type": "object",
                    "description": "The call's arguments, as they would run.",
                },
                "tool_use_id": {
                    "type": "string",
                    "description": "The call's id.",
                },
            },
            "required": ["tool_name", "input", "tool_use_id"],
            "additionalProperties": true,
        },
    })
}
