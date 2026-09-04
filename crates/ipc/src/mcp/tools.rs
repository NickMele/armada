//! The tools a Drone is given, their arguments and their schemas.
//!
//! Split from the transport beside it because they are two things: [`mod@super`]
//! reads JSON-RPC and decides what is answered, and this decides what a tool
//! takes. **A tool with nothing to share lives beside this file** —
//! [`dispatch`](mod@super::dispatch) and [`ask`](mod@super::ask) both do — so
//! what stays is [`closed`], [`text`] and [`list`], and a sixth tool is a new
//! file plus about six lines here.
//!
//! # No tool takes a job id or a step id
//!
//! Fleet knows both, and a value a Drone supplies is a value a Drone chose. A
//! call carrying one is refused **by name** — a field nothing reads is a promise
//! the call makes and the system does not keep.
//!
//! [`CHECKS_TOOL`] takes nothing at all, not even a Check name: the step's
//! Checks were frozen at approval, so a name here could only be a Drone
//! choosing which bar it is measured against.
//!
//! # Why declaring is a different call from submitting
//!
//! A scope is declared **before** the work and evidence after it, so one call
//! cannot carry both. Keeping them apart is what leaves `submit_evidence` at the
//! three prose fields the Agent Copy Contract names: a path list is a claim
//! Fleet checks against the worktree rather than reading.

use serde_json::{json, Map, Value};

use super::ask::{ASK_FIELDS, ASK_TOOL, FEWEST_OPTIONS, MOST_OPTIONS};
use super::dispatch::{DISPATCH_FIELDS, DISPATCH_TOOL};
use super::widening::{WIDEN_FIELDS, WIDEN_TOOL};

/// The Evidence tool's own name, bare. The client joins it to the server name
/// to make the allowlist entry.
pub const TOOL: &str = "submit_evidence";

/// The scope-declaration tool's name, bare.
pub const SCOPE_TOOL: &str = "declare_scope";

/// The dry-run tool's name, bare. **A question, not a submission** — it moves
/// no step, and `fleet::dry_run` says why the answer it returns can never be
/// one.
pub const CHECKS_TOOL: &str = "run_checks";

/// The three prose fields, and a refusal for anything else.
///
/// **Public because a second reader needs the same spelling.** The transcript
/// decoder names these keys to put a call's argument on a row, and it was
/// blind to all three for as long as it carried its own copy of the list — a
/// rename here has to break that reader rather than silently empty it.
pub const EVIDENCE_FIELDS: &[&str] = &["claimed", "shown_by", "not_claimed"];
/// The one field the scope tool takes. Public for [`EVIDENCE_FIELDS`]' reason.
pub const SCOPE_FIELDS: &[&str] = &["context_paths"];
/// The Checks tool takes none. See this module's comment.
///
/// Public for the same reason and carrying the opposite fact: a `run_checks`
/// row with an empty detail is **accurate**, because there was no argument.
pub const CHECKS_FIELDS: &[&str] = &[];
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
///
/// # One enum for every tool, and it is why this file is over 500 lines
///
/// A tool answers refusals through [`Incoming::NotASubmission`], which carries
/// one type, so a tool with a rule no other tool has adds a variant here even
/// when everything else about it lives next door. `ask_question` has four —
/// a list that is not one, too few or too many answers, two answers a person
/// could not tell apart, and a field that says nothing — because a question is
/// the only call with a *shape* to get wrong rather than a field to omit.
///
/// **Splitting them would mean a second refusal type on the transport**, and
/// then the arm that reads a tool call would have to decide which one it is
/// holding before it could say anything at all. The file is long because the
/// refusals are one closed set; that is the trade, and it is the same one
/// `event.rs` makes about its kinds.
///
/// [`Incoming::NotASubmission`]: super::Incoming::NotASubmission
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum NotAnArgument {
    /// A tool this server does not serve.
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
    /// A field whose value should be a list of strings and is not.
    NotAList {
        field: &'static str,
    },
    /// A field the tool does not take. Named rather than dropped.
    NotAField {
        named: String,
        tool: &'static str,
        takes: &'static [&'static str],
    },
    /// A list of answer objects that is not one, or one entry that is not.
    NotOptions {
        field: &'static str,
    },
    /// Too few or too many answers offered. **Refused rather than trimmed**: a
    /// question quietly cut to four is one a person answers without knowing what
    /// was left out.
    WrongCount {
        offered: usize,
    },
    /// Two answers a person could not tell apart. An answer names its label, so
    /// two the same is an answer that means either.
    SameTwice {
        label: String,
    },
    /// A field that is present, is text, and says nothing.
    Blank {
        field: &'static str,
    },
    /// A request for more scope that named no paths. **Refused rather than
    /// taken as a call that did nothing**: the call costs a Judge call and one
    /// is allowed per part, so a request for nothing would spend the only ask
    /// the Drone had.
    AskedForNothing,
}

impl core::fmt::Display for NotAnArgument {
    fn fmt(&self, out: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        match self {
            NotAnArgument::NoSuchTool { named } => write!(
                out,
                "there is no tool called `{named}`. The tools are `{TOOL}`, \
                 `{SCOPE_TOOL}`, `{WIDEN_TOOL}`, `{CHECKS_TOOL}`, \
                 `{DISPATCH_TOOL}` and `{ASK_TOOL}`"
            ),
            // The empty list is a real case now: `run_checks` takes nothing, so
            // a call of it with no arguments is a correct call and never
            // reaches here. A message reading "takes " with nothing after it
            // would be this refusal arriving at a tool it does not apply to.
            NotAnArgument::NoArguments { tool, takes: [] } => write!(
                out,
                "`{tool}` takes no arguments and none were expected. This \
                 refusal is a fault in Fleet rather than in the call"
            ),
            NotAnArgument::NoArguments { tool, takes } => write!(
                out,
                "the call carried no arguments. `{tool}` takes {}",
                Listed(takes)
            ),
            NotAnArgument::Missing { field } => write!(
                out,
                "`{field}` is missing. Every field this tool takes is required, \
                 and the ones that may be empty are empty rather than absent — \
                 call again with it"
            ),
            NotAnArgument::NotText { field } => {
                write!(out, "`{field}` is not text. Submit again with a string")
            }
            NotAnArgument::NotAList { field } => write!(
                out,
                "`{field}` is not a list of strings. Call again with one, \
                 using [] where the list is legitimately empty"
            ),
            // The two a Drone is likeliest to invent get the reason, because
            // "no such field" reads as an oversight it should work around.
            NotAnArgument::NotAField { named, .. }
                if named == "job_id" || named == "step_id" || named == "parent_id" =>
            {
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
            // A Drone that invented a Check name is told who decides that,
            // because "no such field" reads as an oversight to work around and
            // the answer here is that there is nothing to work around.
            NotAnArgument::NotAField {
                named,
                tool,
                takes: [],
            } => write!(
                out,
                "`{named}` is not a field of `{tool}`, which takes no arguments \
                 at all. Which checks gate the part you are on was settled when \
                 this task was approved; remove it and call again"
            ),
            NotAnArgument::NotAField { named, tool, takes } => write!(
                out,
                "`{named}` is not a field of `{tool}`. It takes {} — remove it \
                 and call again",
                Listed(takes)
            ),
            NotAnArgument::NotOptions { field } => write!(
                out,
                "`{field}` is not a list of answers. Each entry is an object \
                 with `label`, what the person picks, and `consequence`, what \
                 you will do if they pick it"
            ),
            NotAnArgument::WrongCount { offered } => {
                let (fewest, most) = (FEWEST_OPTIONS, MOST_OPTIONS);
                write!(
                    out,
                    "you offered {offered} answers. Offer between {fewest} and \
                     {most}: one is not a question, and a longer list is one a \
                     person reads badly"
                )
            }
            NotAnArgument::SameTwice { label } => write!(
                out,
                "two answers are both labelled `{label}`. An answer names its \
                 label, so two the same means either — give each its own and \
                 call again"
            ),
            NotAnArgument::Blank { field } => write!(
                out,
                "`{field}` is empty. It has to say something — call again with \
                 it filled in"
            ),
            NotAnArgument::AskedForNothing => out.write_str(
                "you asked for no paths. Name the ones the work needs that the \
                 task's scope does not already cover, or get on with the work \
                 inside the scope you have",
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

/// The Checks tool's arguments, which is to say: that there are none.
///
/// **A call carrying a field is refused by name** rather than having it
/// dropped, which is [`closed`]'s rule applied to an empty list.
pub(crate) fn nothing(arguments: &Map<String, Value>) -> Result<(), NotAnArgument> {
    closed(arguments, CHECKS_TOOL, CHECKS_FIELDS)
}

pub(crate) fn declaration(arguments: &Map<String, Value>) -> Result<DeclareScope, NotAnArgument> {
    closed(arguments, SCOPE_TOOL, SCOPE_FIELDS)?;
    Ok(DeclareScope {
        context_paths: list(arguments, "context_paths")?,
    })
}

/// One field that is a list of strings, entry by entry.
///
/// **Legitimately empty and never absent.** A list nothing is in is an answer;
/// a missing field is not, and the two are told apart here rather than by a
/// caller reading a length.
///
/// `pub(super)` for [`closed`]'s reason — the dispatch tool takes two of these.
pub(super) fn list(
    arguments: &Map<String, Value>,
    field: &'static str,
) -> Result<Vec<String>, NotAnArgument> {
    let listed = arguments
        .get(field)
        .ok_or(NotAnArgument::Missing { field })?
        .as_array()
        .ok_or(NotAnArgument::NotAList { field })?;
    let mut entries = Vec::with_capacity(listed.len());
    for entry in listed {
        entries.push(
            entry
                .as_str()
                .ok_or(NotAnArgument::NotAList { field })?
                .into(),
        );
    }
    Ok(entries)
}

/// Every argument is a field the tool takes.
///
/// `pub(super)` because [`dispatch`](mod@super::dispatch) and
/// [`ask`](mod@super::ask) read their own tools' arguments and must refuse an
/// unknown field identically. A second copy of this loop is a second place the
/// rule could stop being true.
pub(super) fn closed(
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
        WIDEN_TOOL => Ok(WIDEN_TOOL),
        CHECKS_TOOL => Ok(CHECKS_TOOL),
        DISPATCH_TOOL => Ok(DISPATCH_TOOL),
        ASK_TOOL => Ok(ASK_TOOL),
        other => Err(NotAnArgument::NoSuchTool {
            named: other.to_string(),
        }),
    }
}

/// What a call with no `arguments` object is refused with, for the two tools
/// that take one. `run_checks` never reaches this — the transport answers it
/// before the arguments are looked for.
pub(crate) fn argumentless(tool: &'static str) -> NotAnArgument {
    match tool {
        SCOPE_TOOL => NotAnArgument::NoArguments {
            tool: SCOPE_TOOL,
            takes: SCOPE_FIELDS,
        },
        WIDEN_TOOL => NotAnArgument::NoArguments {
            tool: WIDEN_TOOL,
            takes: WIDEN_FIELDS,
        },
        ASK_TOOL => NotAnArgument::NoArguments {
            tool: ASK_TOOL,
            takes: ASK_FIELDS,
        },
        CHECKS_TOOL => NotAnArgument::NoArguments {
            tool: CHECKS_TOOL,
            takes: CHECKS_FIELDS,
        },
        DISPATCH_TOOL => NotAnArgument::NoArguments {
            tool: DISPATCH_TOOL,
            takes: DISPATCH_FIELDS,
        },
        _ => NotAnArgument::NoArguments {
            tool: TOOL,
            takes: EVIDENCE_FIELDS,
        },
    }
}

/// Text that says something. **The blank check is here rather than at the Fleet
/// boundary**, unlike [`crate::Redirection`]'s: this is a tool call, and a
/// refusal a Drone reads is what it acts on. A 422 has nowhere to arrive.
pub(super) fn filled(
    arguments: &Map<String, Value>,
    field: &'static str,
) -> Result<String, NotAnArgument> {
    let said = text(arguments, field)?;
    if said.trim().is_empty() {
        return Err(NotAnArgument::Blank { field });
    }
    Ok(said)
}

/// One text field. `pub(super)` for [`closed`]'s reason.
pub(super) fn text(
    arguments: &Map<String, Value>,
    field: &'static str,
) -> Result<String, NotAnArgument> {
    let value = arguments
        .get(field)
        .ok_or(NotAnArgument::Missing { field })?;
    Ok(value
        .as_str()
        .ok_or(NotAnArgument::NotText { field })?
        .to_string())
}

/// Every tool, as the client is shown them.
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
    vec![
        evidence_tool(),
        scope_tool(),
        checks_tool(),
        super::widening::widen_tool(),
        super::dispatch::dispatch_tool(),
        super::ask::ask_tool(),
    ]
}

/// The dry-run tool.
///
/// **The description says what it is not**, twice: not a verdict, and not a
/// substitute for submitting. Spike 6 measured that a description alone does
/// not make a Drone call a tool, which is why the offer is also in the
/// briefing — but what a description still has to do is stop a Drone reading a
/// green run as a finished step.
///
/// It offers no way to pick a Check. `docs/concepts/drone.md` used to keep the
/// Checks from a Drone entirely; what replaced that is the Judge and the gaming
/// patterns, not a parameter through which a Drone picks its own bar.
///
/// **It points at the brief rather than naming the Checks itself.** Which
/// Checks gate a part is a fact about the part, and this description is
/// assembled once with no step in hand — `Answered::Tools` carries none, and
/// threading one in would make a static list into per-step state that a client
/// is free to cache past the boundary it was true at. `fleet::terms::Checking`
/// names them, per step, in the same session. Two places naming them would be
/// two places to disagree; this one says where the list is, which is the answer
/// a Drone deciding whether to spend a call actually needs.
///
/// **It says what a brief naming none means.** A part with no Checks is offered
/// no block at all, so a Drone sent looking for a list that is not there would
/// be left to work out whether it had missed something or the tool was broken.
fn checks_tool() -> Value {
    json!({
        "name": CHECKS_TOOL,
        "description":
            "Run the checks that gate the part you are on — the ones your brief \
             names under FINDING OUT WHERE YOU STAND — in your worktree, and get \
             back what each one did and where its output was written. It runs \
             all of them and takes no arguments; there is nothing to choose. \
             Call it when you want to know whether the work holds up, before you \
             submit. It is not a verdict and it advances nothing — the checks \
             are run again when you submit, and only that run decides anything. \
             There is a limit on how many times one part may ask, a second call \
             while one is still running is refused, and a part whose brief names \
             no checks has none to run.",
        "inputSchema": {
            "type": "object",
            "properties": {},
            "additionalProperties": false,
        },
    })
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
