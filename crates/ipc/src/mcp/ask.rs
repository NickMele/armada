//! The asking tool: what a Drone may ask a person, and what it will take back.
//!
//! # Its own module, and the fifth tool costs about six lines
//!
//! [`tools`](mod@super::tools) holds the three tools a Drone has always had and
//! the helpers every tool shares — [`closed`](super::tools::closed),
//! [`text`](super::tools::text) and the field-list prose. The dispatch tool made
//! that split and asked the next one to keep it, so this is the ask half doing
//! that: a tool is a name, a field list, an argument type, a parser and a
//! schema, and all five of one tool live together.
//!
//! What stays next door is anything two tools share. Nothing here is shared.
//!
//! # The one tool whose answer is not in the reply
//!
//! Every other tool on this seam is answered from something Fleet already
//! holds. This one is answered when a person picks an option, which may be hours
//! later and arrives as a turn injected into the Drone's session rather than as
//! this call's return value. `crates/fleet/src/questioning.rs` is where that
//! happens and why it does not block.

use serde_json::{json, Map, Value};

use super::tools::{closed, filled, NotAnArgument};

/// The asking tool's name, bare.
///
/// **The one tool whose answer comes from a person.** Every other tool here is
/// answered by Fleet out of what it already knows; this one is answered when
/// somebody picks an option, which may be hours later and arrives as an
/// injected turn rather than as this call's return value.
pub const ASK_TOOL: &str = "ask_question";

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

/// The asking tool's arguments.
///
/// **Every rule the type states is checked here and refused by name.** A Drone
/// whose question would draw badly is told what to fix and calls again; nothing
/// is trimmed, defaulted or accepted-and-repaired, because the repair would be
/// Fleet deciding what a person is asked.
pub(super) fn question(arguments: &Map<String, Value>) -> Result<AskQuestion, NotAnArgument> {
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
pub(super) fn ask_tool() -> Value {
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
