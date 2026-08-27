//! The two tools a Drone is given, their arguments and their schemas.
//!
//! Split from the transport beside it because they are two things: [`mod@super`]
//! reads JSON-RPC and decides what is answered, and this decides what a tool
//! takes. A third tool touches only this file.
//!
//! # Two tools, and neither takes a job id or a step id
//!
//! Fleet knows both, and a value a Drone supplies is a value a Drone chose. A
//! call carrying one is refused **by name** rather than ignored — a field
//! nothing reads is a promise the call makes and the system does not keep.
//!
//! # Why declaring is a different call from submitting
//!
//! A scope is declared **before** the work and evidence is submitted after it,
//! so one call cannot carry both. Keeping them apart is also what leaves
//! `submit_evidence` at the three prose fields the Agent Copy Contract names:
//! a path list is not prose about the work, it is a claim about where the work
//! will be, and Fleet checks it against the worktree rather than reading it.

use serde_json::{json, Map, Value};

/// The Evidence tool's own name, bare. The client joins it to the server name
/// to make the allowlist entry.
pub const TOOL: &str = "submit_evidence";

/// The scope-declaration tool's name, bare.
pub const SCOPE_TOOL: &str = "declare_scope";

/// The three prose fields, and a refusal for anything else.
const EVIDENCE_FIELDS: &[&str] = &["claimed", "shown_by", "not_claimed"];
/// The one field the scope tool takes.
const SCOPE_FIELDS: &[&str] = &["context_paths"];

/// What a Drone hands over. **The Agent Copy Contract's Work submission
/// fields, spelled as the Drone is asked for them**, and nothing else.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct SubmitEvidence {
    pub claimed: String,
    pub shown_by: String,
    /// Required, and legitimately empty — which is why it is not an `Option`
    /// here either. A Drone that left nothing behind has answered; a Drone
    /// that omitted the field has not, and is refused by name.
    pub not_claimed: String,
}

/// Where a Drone says its work for this step will be.
///
/// **A claim, not evidence.** Nothing here gates until Fleet has compared it
/// with what the worktree actually holds, which is the whole point of the call
/// existing.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct DeclareScope {
    /// Repository-relative. Legitimately empty: a step that will change nothing
    /// has declared that, and it is a different answer from not calling at all.
    pub context_paths: Vec<String>,
}

/// Why a tool call did not read as a call of the tool it named.
///
/// **None of these is a gate failure.** Nothing was verified and the step has
/// neither advanced nor failed; the call was malformed and what the Drone is
/// told is to fix it and call again.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum NotAnArgument {
    /// A tool that is neither of the two.
    NoSuchTool {
        named: String,
    },
    /// `arguments` was absent, or was not an object.
    NoArguments {
        tool: &'static str,
        takes: &'static [&'static str],
    },
    Missing {
        field: &'static str,
    },
    NotText {
        field: &'static str,
    },
    /// A field whose value should be a list of paths and is not.
    NotAList {
        field: &'static str,
    },
    /// A field the tool does not take. Named rather than dropped.
    NotAField {
        named: String,
        tool: &'static str,
        takes: &'static [&'static str],
    },
}

impl core::fmt::Display for NotAnArgument {
    fn fmt(&self, out: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        match self {
            NotAnArgument::NoSuchTool { named } => write!(
                out,
                "there is no tool called `{named}`. The tools are `{TOOL}` and `{SCOPE_TOOL}`"
            ),
            NotAnArgument::NoArguments { tool, takes } => write!(
                out,
                "the call carried no arguments. `{tool}` takes {}",
                Listed(takes)
            ),
            NotAnArgument::Missing { field } => write!(
                out,
                "`{field}` is missing. It is required and may be empty only \
                 where it is not_claimed — submit again with it"
            ),
            NotAnArgument::NotText { field } => {
                write!(out, "`{field}` is not text. Submit again with a string")
            }
            NotAnArgument::NotAList { field } => write!(
                out,
                "`{field}` is not a list of repository-relative paths. Call \
                 again with one, using [] if this part changes nothing"
            ),
            // The two a Drone is likeliest to invent get the reason, because
            // "no such field" reads as an oversight it should work around.
            NotAnArgument::NotAField { named, .. } if named == "job_id" || named == "step_id" => {
                write!(
                    out,
                    "`{named}` is not a field of this tool. Fleet knows which Job \
                     and which step you are on and binds your call to them; \
                     remove it and call again"
                )
            }
            // `note` was a field until every step was given the same three, so
            // a Drone carrying an older habit is told where the content goes
            // rather than only that the field is gone.
            NotAnArgument::NotAField { named, .. } if named == "note" => write!(
                out,
                "`note` is not a field of this tool. Put the finding in the file \
                 or artifact you name in `shown_by`, and what it shows in \
                 `claimed` — then submit again"
            ),
            NotAnArgument::NotAField { named, tool, takes } => write!(
                out,
                "`{named}` is not a field of `{tool}`. It takes {} — remove it \
                 and call again",
                Listed(takes)
            ),
        }
    }
}

impl std::error::Error for NotAnArgument {}

/// A field list in prose, so no message ends in a dangling "takes ".
struct Listed<'a>(&'a [&'a str]);

impl core::fmt::Display for Listed<'_> {
    fn fmt(&self, out: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        for (n, field) in self.0.iter().enumerate() {
            match n {
                0 => {}
                n if n + 1 == self.0.len() => out.write_str(" and ")?,
                _ => out.write_str(", ")?,
            }
            write!(out, "{field}")?;
        }
        Ok(())
    }
}

pub(crate) fn submission(arguments: &Map<String, Value>) -> Result<SubmitEvidence, NotAnArgument> {
    closed(arguments, TOOL, EVIDENCE_FIELDS)?;
    Ok(SubmitEvidence {
        claimed: text(arguments, "claimed")?,
        shown_by: text(arguments, "shown_by")?,
        not_claimed: text(arguments, "not_claimed")?,
    })
}

pub(crate) fn declaration(arguments: &Map<String, Value>) -> Result<DeclareScope, NotAnArgument> {
    closed(arguments, SCOPE_TOOL, SCOPE_FIELDS)?;
    let field = "context_paths";
    let listed = arguments
        .get(field)
        .ok_or(NotAnArgument::Missing { field })?
        .as_array()
        .ok_or(NotAnArgument::NotAList { field })?;
    let mut context_paths = Vec::with_capacity(listed.len());
    for path in listed {
        context_paths.push(
            path.as_str()
                .ok_or(NotAnArgument::NotAList { field })?
                .into(),
        );
    }
    Ok(DeclareScope { context_paths })
}

/// Every argument is a field the tool takes.
fn closed(
    arguments: &Map<String, Value>,
    tool: &'static str,
    takes: &'static [&'static str],
) -> Result<(), NotAnArgument> {
    for named in arguments.keys() {
        if !takes.contains(&named.as_str()) {
            return Err(NotAnArgument::NotAField {
                named: named.clone(),
                tool,
                takes,
            });
        }
    }
    Ok(())
}

/// Which tool a call named, or the refusal.
pub(crate) fn named(name: &str) -> Result<&'static str, NotAnArgument> {
    match name {
        TOOL => Ok(TOOL),
        SCOPE_TOOL => Ok(SCOPE_TOOL),
        other => Err(NotAnArgument::NoSuchTool {
            named: other.to_string(),
        }),
    }
}

/// What a call with no `arguments` object is refused with, for either tool.
pub(crate) fn argumentless(tool: &'static str) -> NotAnArgument {
    match tool {
        SCOPE_TOOL => NotAnArgument::NoArguments {
            tool: SCOPE_TOOL,
            takes: SCOPE_FIELDS,
        },
        _ => NotAnArgument::NoArguments {
            tool: TOOL,
            takes: EVIDENCE_FIELDS,
        },
    }
}

fn text(arguments: &Map<String, Value>, field: &'static str) -> Result<String, NotAnArgument> {
    let value = arguments
        .get(field)
        .ok_or(NotAnArgument::Missing { field })?;
    Ok(value
        .as_str()
        .ok_or(NotAnArgument::NotText { field })?
        .to_string())
}

/// Both tools, as the client is shown them.
///
/// The Evidence description is the wording spike 6 measured — the `silent` arm
/// proved a description alone does not make a Drone call the tool, which is why
/// the obligation is in the baseline prompt; what the description still has to
/// do is say what the call is for and that the receipt is not a verdict.
///
/// **`additionalProperties` is false and the server checks it anyway.** The
/// schema is advice a client may enforce; [`closed`] is what makes a forged
/// field a named refusal rather than a silently accepted one.
pub(crate) fn listed() -> Vec<Value> {
    vec![evidence_tool(), scope_tool()]
}

fn evidence_tool() -> Value {
    json!({
        "name": TOOL,
        "description":
            "Report the outcome of the step you were given. This is the only way \
             to report: the result is not read from anything you write in prose. \
             Returns a receipt, not a verdict — the receipt does not mean the \
             step passed.",
        "inputSchema": {
            "type": "object",
            "properties": {
                "claimed": {
                    "type": "string",
                    "description":
                        "What the work now does, as an observable. Behaviour, not \
                         a description of the change you made.",
                },
                "shown_by": {
                    "type": "string",
                    "description":
                        "The artifact that demonstrates it — a named test, a \
                         command and its exit code, a rendered string, or a \
                         file you wrote. Every step names one here, including a \
                         step that changed nothing in the repository.",
                },
                "not_claimed": {
                    "type": "string",
                    "description":
                        "Everything the claim does not assert: the gap you left \
                         and the side effect you caused. Empty is a legal answer; \
                         omitting it is not.",
                },
            },
            "required": ["claimed", "shown_by", "not_claimed"],
            "additionalProperties": false,
        },
    })
}

fn scope_tool() -> Value {
    json!({
        "name": SCOPE_TOOL,
        "description":
            "Say which paths this part of the task will be in, before you start \
             work. Files you change outside them are checked against this, so a \
             plan that turns out wrong is worth updating by calling again — but \
             work that belongs to a later part does not become this part's by \
             being declared.",
        "inputSchema": {
            "type": "object",
            "properties": {
                "context_paths": {
                    "type": "array",
                    "items": { "type": "string" },
                    "description":
                        "Repository-relative paths — a file, or a directory and \
                         everything under it. Include what you will change and \
                         what has to be read to judge the change. Use [] if this \
                         part changes nothing.",
                },
            },
            "required": ["context_paths"],
            "additionalProperties": false,
        },
    })
}
