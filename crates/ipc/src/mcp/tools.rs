//! The four tools a Drone is given, their arguments and their schemas.
//!
//! Split from the transport beside it because they are two things: [`mod@super`]
//! reads JSON-RPC and decides what is answered, and this decides what a tool
//! takes. A fifth tool touches only this file.
//!
//! # No tool takes a job id or a step id
//!
//! Fleet knows both, and a value a Drone supplies is a value a Drone chose. A
//! call carrying one is refused **by name** rather than ignored — a field
//! nothing reads is a promise the call makes and the system does not keep.
//!
//! [`CHECKS_TOOL`] takes nothing at all, not even a Check name: which Checks
//! gate the step was frozen when the Job was approved, so a name here could
//! only agree or disagree, and the disagreeing case is a Drone choosing which
//! bar it is measured against. Its field list is empty, which makes every
//! argument a named refusal.
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

/// The dry-run tool's name, bare. **A question, not a submission** — it moves
/// no step, and `fleet::dry_run` says why the answer it returns can never be
/// one.
pub const CHECKS_TOOL: &str = "run_checks";

/// The asking tool's name, bare.
///
/// **The one tool whose answer comes from a person.** Every other tool here is
/// answered by Fleet out of what it already knows; this one is answered when
/// somebody picks an option, which may be hours later and arrives as an
/// injected turn rather than as this call's return value.
pub const ASK_TOOL: &str = "ask_question";

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
/// The two fields the asking tool takes. Public for [`EVIDENCE_FIELDS`]' reason.
pub const ASK_FIELDS: &[&str] = &["question", "options"];

/// How few answers a question may offer.
///
/// **A question with one answer is not a question**, it is a notification, and
/// a Drone that wanted to tell a person something has the Job's own log for it.
/// Refused here rather than accepted and drawn as a single button, because a
/// surface with one control is a surface that reads as a confirmation.
pub const FEWEST_OPTIONS: usize = 2;

/// How many answers a question may offer.
///
/// Four, matching the shape this workspace's own structured-question tool uses.
/// The bound is not arbitrary: the whole value of asking rather than escalating
/// is that a person answers in one glance, and a list long enough to scroll is
/// a list somebody reads badly at 11pm. A Drone with more than four candidate
/// splits has not finished thinking.
pub const MOST_OPTIONS: usize = 4;

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

/// What a Drone asks a person, and what it will take as an answer.
///
/// **Structured, and there is no field for a free-text reply.** The owner asked
/// for questions with structured answers, and this type is that requirement
/// spelled: a Drone offers between [`FEWEST_OPTIONS`] and [`MOST_OPTIONS`]
/// answers, a person picks one, and nothing types prose. A person who needs to
/// say something the options do not cover uses `redirect_drone`, which already
/// exists and is the one route a person's words reach a Drone by.
///
/// **It is not a conversation.** `docs/scope.md` records that orchestrator
/// agents with sub agents was abandoned because having a conversation was not
/// the tool that was wanted, and warns that a design reaching for "ask the
/// agent" is reaching for that attempt again. The distinction is not whether a
/// person is involved — it is whether a conversation is the medium. One question
/// is outstanding per Job, it is asked once and answered once, and there is no
/// id here joining it to an earlier one.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct AskQuestion {
    /// What the Drone wants to know. **Never blank** — a question with nothing
    /// in it is a Drone asking a person to guess what it is stuck on.
    pub question: String,
    /// The answers it will accept. Between [`FEWEST_OPTIONS`] and
    /// [`MOST_OPTIONS`], each with a distinct non-blank label and a non-blank
    /// consequence.
    pub options: Vec<AskedOption>,
}

/// One answer the Drone said it would accept.
///
/// **Both fields are required and neither may be blank.** A label with no
/// consequence is a button whose effect a person has to guess, and guessing is
/// the failure this whole tool exists to remove.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct AskedOption {
    pub label: String,
    pub consequence: String,
}

/// Why a tool call did not read as a call of the tool it named.
///
/// **None of these is a gate failure.** Nothing was verified and the step has
/// neither advanced nor failed; the call was malformed and what the Drone is
/// told is to fix it and call again.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum NotAnArgument {
    /// A tool that is none of the three.
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
    /// A field whose value should be a list of objects and is not — or one of
    /// whose entries is not.
    NotOptions {
        field: &'static str,
    },
    /// Too few or too many answers offered. **Refused rather than trimmed**: a
    /// question silently reduced to four options is a question a person answers
    /// without knowing what was left out.
    WrongCount {
        offered: usize,
    },
    /// Two answers a person could not tell apart. An answer names its label, so
    /// two identical labels are an answer that means either.
    SameTwice {
        label: String,
    },
    /// A field that is present, is text, and says nothing.
    Blank {
        field: &'static str,
    },
}

impl core::fmt::Display for NotAnArgument {
    fn fmt(&self, out: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        match self {
            NotAnArgument::NoSuchTool { named } => write!(
                out,
                "there is no tool called `{named}`. The tools are `{TOOL}`, \
                 `{SCOPE_TOOL}`, `{CHECKS_TOOL}` and `{ASK_TOOL}`"
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
                 with `label`, which is what the person picks, and \
                 `consequence`, which is what you will do if they pick it"
            ),
            NotAnArgument::WrongCount { offered } => write!(
                out,
                "you offered {offered} answers. Offer between {FEWEST_OPTIONS} \
                 and {MOST_OPTIONS}: one answer is not a question, and a list \
                 longer than {MOST_OPTIONS} is one a person reads badly"
            ),
            NotAnArgument::SameTwice { label } => write!(
                out,
                "two answers are both labelled `{label}`. An answer names its \
                 label, so two the same is an answer that means either — give \
                 each one a label of its own and call again"
            ),
            NotAnArgument::Blank { field } => write!(
                out,
                "`{field}` is empty. It has to say something — call again with \
                 it filled in"
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

/// The asking tool's arguments.
///
/// **Every rule the type states is checked here and refused by name.** A Drone
/// whose question would draw badly is told what to fix and calls again; nothing
/// is trimmed, defaulted or accepted-and-repaired, because the repair would be
/// Fleet deciding what a person is asked.
pub(crate) fn question(arguments: &Map<String, Value>) -> Result<AskQuestion, NotAnArgument> {
    closed(arguments, ASK_TOOL, ASK_FIELDS)?;
    let question = filled(arguments, "question")?;
    let field = "options";
    let listed = arguments
        .get(field)
        .ok_or(NotAnArgument::Missing { field })?
        .as_array()
        .ok_or(NotAnArgument::NotOptions { field })?;
    if listed.len() < FEWEST_OPTIONS || listed.len() > MOST_OPTIONS {
        return Err(NotAnArgument::WrongCount {
            offered: listed.len(),
        });
    }
    let mut options: Vec<AskedOption> = Vec::with_capacity(listed.len());
    for offered in listed {
        let offered = offered
            .as_object()
            .ok_or(NotAnArgument::NotOptions { field })?;
        closed(offered, ASK_TOOL, OPTION_FIELDS)?;
        let option = AskedOption {
            label: filled(offered, "label")?,
            consequence: filled(offered, "consequence")?,
        };
        if options.iter().any(|held| held.label == option.label) {
            return Err(NotAnArgument::SameTwice {
                label: option.label,
            });
        }
        options.push(option);
    }
    Ok(AskQuestion { question, options })
}

/// What one answer's object may carry. **Not public**: it is refused through
/// [`NotAnArgument::NotAField`] naming the tool, so no second reader spells it.
const OPTION_FIELDS: &[&str] = &["label", "consequence"];

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
        CHECKS_TOOL => Ok(CHECKS_TOOL),
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
        ASK_TOOL => NotAnArgument::NoArguments {
            tool: ASK_TOOL,
            takes: ASK_FIELDS,
        },
        CHECKS_TOOL => NotAnArgument::NoArguments {
            tool: CHECKS_TOOL,
            takes: CHECKS_FIELDS,
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
fn filled(arguments: &Map<String, Value>, field: &'static str) -> Result<String, NotAnArgument> {
    let said = text(arguments, field)?;
    if said.trim().is_empty() {
        return Err(NotAnArgument::Blank { field });
    }
    Ok(said)
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
    vec![evidence_tool(), scope_tool(), checks_tool(), ask_tool()]
}

/// The asking tool.
///
/// **The description says what happens after the call returns**, because that
/// is the part no other tool here has: the receipt is not the answer, the answer
/// arrives as a later turn, and a Drone that treats the receipt as a refusal
/// will guess — which is the exact failure this tool was built to remove.
///
/// It also says what asking is *not* for. Spike 6 measured that a description
/// alone does not make a Drone call a tool, and the same finding cuts the other
/// way: a tool described only by what it does gets called for everything. The
/// bar is stated here and again in the briefing.
fn ask_tool() -> Value {
    json!({
        "name": ASK_TOOL,
        "description":
            "Ask the person who approved this task a question you cannot answer \
             from the repository, and offer them the answers you would accept. \
             Use it when guessing would change what gets built, not when you \
             merely have a preference — you were given the task because the \
             ordinary decisions are yours. You get back a receipt, not an \
             answer: the answer arrives later as a turn in this session, and it \
             may be a while. Stop and wait for it. One question at a time, and \
             asking again before this one is answered is refused.",
        "inputSchema": {
            "type": "object",
            "properties": {
                "question": {
                    "type": "string",
                    "description":
                        "What you need to know, in one sentence, with the facts \
                         that make it decidable. Not what you would like an \
                         opinion on.",
                },
                "options": {
                    "type": "array",
                    "minItems": FEWEST_OPTIONS,
                    "maxItems": MOST_OPTIONS,
                    "items": {
                        "type": "object",
                        "properties": {
                            "label": {
                                "type": "string",
                                "description":
                                    "What the person picks, short enough to read \
                                     at a glance. Distinct from every other label.",
                            },
                            "consequence": {
                                "type": "string",
                                "description":
                                    "What you will do if they pick it. This is \
                                     what they are actually deciding, so say the \
                                     effect and not the reasoning.",
                            },
                        },
                        "required": ["label", "consequence"],
                        "additionalProperties": false,
                    },
                    "description":
                        "The answers you will accept. Two to four. There is no \
                         free-text reply — everything you want to be told has to \
                         be one of these.",
                },
            },
            "required": ["question", "options"],
            "additionalProperties": false,
        },
    })
}

/// The dry-run tool.
///
/// **The description says what it is not**, twice: not a verdict, and not a
/// substitute for submitting. Spike 6 measured that a description alone does
/// not make a Drone call a tool, which is why the offer is also in the
/// briefing — but what a description still has to do is stop a Drone reading a
/// green run as a finished step.
///
/// It names no Check and offers no way to. `docs/concepts/drone.md` used to
/// keep the Checks from a Drone entirely; what replaced that is the Judge and
/// the gaming patterns, not a parameter through which a Drone picks its own
/// bar.
fn checks_tool() -> Value {
    json!({
        "name": CHECKS_TOOL,
        "description":
            "Run the checks that gate the part you are on, in your worktree, and \
             get back what each one did and where its output was written. Call \
             it when you want to know whether the work holds up, before you \
             submit. It is not a verdict and it advances nothing — the checks \
             are run again when you submit, and only that run decides anything. \
             There is a limit on how many times one part may ask, and a second \
             call while one is still running is refused.",
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
