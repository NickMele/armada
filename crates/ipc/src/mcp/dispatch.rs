//! The dispatch tool: what a Drone says when it asks for another Job to exist.
//!
//! # It takes no parent id, and refuses one by name
//!
//! The rule the other tools hold to, on the call where breaking it would cost
//! the most: a parent id supplied by a Drone is a parent id a Drone chose, and
//! this is the tool whose effect is *other Jobs existing*. One Drone minting
//! children under another Drone's Job is the shape that refusal exists to make
//! unspeakable. Fleet knows the parent — it is the Job whose Drone holds the
//! connection the call arrived on.
//!
//! # `after` names ids and not positions
//!
//! Each call answers with the id Fleet minted, so a Drone dispatching five Jobs
//! has four ids in hand by the time it dispatches the fifth. Naming them is
//! what makes an edge point at a record rather than at a position in a plan
//! nothing has stored. **Which ids are namable is Fleet's answer, not this
//! module's**: a sibling of this call's own parent, and nothing else.
//!
//! # There is no `model`, no `urgency` and no `write_targets`
//!
//! A Drone choosing what its children are spawned as is a Drone choosing what
//! they cost, and none of the three is a decision decomposition makes.

use serde_json::{json, Map, Value};

use super::tools::{closed, list, text, NotAnArgument};

/// The dispatch tool's own name, bare.
pub const DISPATCH_TOOL: &str = "dispatch_job";

/// The five fields the dispatch tool takes. Public for [`EVIDENCE_FIELDS`]'
/// reason: the transcript decoder names these keys to put a call's argument on
/// a row.
///
/// [`EVIDENCE_FIELDS`]: super::EVIDENCE_FIELDS
pub const DISPATCH_FIELDS: &[&str] =
    &["title", "workflow", "brief", "acceptance_criteria", "after"];

/// One Job a Drone is asking Fleet to create.
///
/// **None of these five is optional and two of them are legitimately empty**,
/// which is `SubmitEvidence::not_claimed`'s rule: a Drone that decided this
/// child needs no criteria has answered, and a Drone that omitted the field has
/// not.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct DispatchJob {
    /// What a person reads in a Board row. Refused blank by `core_model::Title`
    /// where the Job is drafted, not here — this module reads arguments and
    /// decides no meanings.
    pub title: String,
    /// The workflow id, spelled as this repository's own definitions spell it.
    /// A name nothing holds is Fleet's refusal, and it names what is held.
    pub workflow: String,
    /// What the child's Drone is told about the work — the parent's own
    /// reading of it, which is the whole product of the decomposition and the
    /// thing a person read at the gate.
    pub brief: String,
    /// What the child's work is held to. Frozen onto the child at creation, so
    /// a criterion written here cannot be lowered afterwards.
    pub acceptance_criteria: Vec<String>,
    /// The ids of the siblings this child must wait for. Empty is the ordinary
    /// case — a child that waits for nothing.
    pub after: Vec<String>,
}

/// Read one call's arguments, or say which field is wrong.
pub(super) fn dispatch(arguments: &Map<String, Value>) -> Result<DispatchJob, NotAnArgument> {
    closed(arguments, DISPATCH_TOOL, DISPATCH_FIELDS)?;
    Ok(DispatchJob {
        title: text(arguments, "title")?,
        workflow: text(arguments, "workflow")?,
        brief: text(arguments, "brief")?,
        acceptance_criteria: list(arguments, "acceptance_criteria")?,
        after: list(arguments, "after")?,
    })
}

/// The tool, as the client is shown it.
///
/// **The description says what the call does to the world**, because this is
/// the only tool whose effect outlives the step that made it. A Drone reading
/// "returns the id" and nothing else would have no way to know that the thing
/// it just made will spawn a process and spend.
pub(super) fn dispatch_tool() -> Value {
    json!({
        "name": DISPATCH_TOOL,
        "description":
            "Create one Job for a piece of the work you were given, and get \
             back the id it was created under. The Job is real: it is queued \
             immediately, it gets its own worktree and its own agent, and it \
             spends. Call it once per piece, in an order where anything a \
             piece waits for was created before it. It is available only on \
             the part of your task that dispatches, and only after a person \
             has read the plan you wrote.",
        "inputSchema": {
            "type": "object",
            "properties": {
                "title": {
                    "type": "string",
                    "description":
                        "What to call this Job, in a line somebody scanning a \
                         list can tell apart from its siblings.",
                },
                "workflow": {
                    "type": "string",
                    "description":
                        "The id of the workflow this Job runs under, spelled \
                         exactly as this repository spells it. It is frozen \
                         onto the Job and becomes the standard its work is \
                         held to, so answer with the one that fits rather \
                         than the nearest.",
                },
                "brief": {
                    "type": "string",
                    "description":
                        "What this Job's agent is told: the piece of the work, \
                         and what you learned that it needs. It has not read \
                         what you read.",
                },
                "acceptance_criteria": {
                    "type": "array",
                    "items": { "type": "string" },
                    "description":
                        "What this Job's work has to be true for it to be \
                         accepted, one per entry. Use [] where the workflow's \
                         own checks are the whole bar.",
                },
                "after": {
                    "type": "array",
                    "items": { "type": "string" },
                    "description":
                        "The ids this call gave you for the Jobs that must \
                         finish first. Only Jobs you created for this same \
                         task can be named. Use [] where nothing has to come \
                         first.",
                },
            },
            "required": [
                "title", "workflow", "brief", "acceptance_criteria", "after"
            ],
            "additionalProperties": false,
        },
    })
}
