//! What the Job proposer is asked, and how its answer reads. **Pure** —
//! `proposal` makes the call, the way `judging` runs what
//! `verification::Brief` assembles.
//!
//! # Nothing here assigns a workflow by default
//!
//! [`Proposal`] has no arm meaning "the usual one". A call that chooses none,
//! and one that names a workflow this Fleet does not hold, both answer
//! [`Unresolved`] — the request is refused and comes back unchanged. A nearest
//! fit would not be a guess a person could correct later: the definition is
//! frozen into the Job at creation and becomes the yardstick it is judged
//! against.
//!
//! # A call that could not be made is not that refusal
//!
//! `Err` and `Ok(Unresolved)` are different answers and reach a person as
//! different statuses. The network, the quota and the timeout say nothing about
//! the request.
//!
//! # It is not asked which files the work touches
//!
//! That is the first step's, answered by a Drone that has read the code and
//! declared its scope through the scope tool. A proposal guessing paths would
//! be a second source for it, settled worse and earlier.
//! `docs/concepts/job-proposer.md`.

use std::collections::BTreeMap;
use std::fmt;

use config::ResolvedWorkflow;
use core_model::{Ulid, WorkflowId};
use verification::field;

use crate::judging::CallFailed;

/// The block an answer owes per Job, and the one word that declines.
///
/// The last two paragraphs are load-bearing and neither is decoration. The
/// first stops a list being answered from; the second stops one Job becoming
/// three, which is the failure a proposer that *can* split work invents.
const ANSWER_FORMAT: &str = "\
Answer with nothing but the block below, once for each Job the work needs.

    job: <its number, counting from 1>
    workflow: <the id, spelled exactly as it appears above>
    title: <what to call this Job, in the words the request used>
    because: <why that workflow, in one line>
    after: <the job numbers that must finish first, comma separated. Leave \
the line out where none do>

If no workflow above fits the request:

    workflow: none
    because: <what the request asks for that no workflow above covers>

Answer `none` rather than the nearest fit. The workflow named here is frozen \
into the Job and becomes the standard its work is held to, so a near miss is \
not something anybody can correct afterwards.

Write one Job unless the work genuinely cannot land as one change. A split \
that could have landed together is three reviews where one was needed.

Do not work out which files are involved. That is the first step's, and it is \
answered there by reading the code rather than guessed at here.";

/// One Job a reading proposed, before Fleet has minted anything.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct ProposedJob {
    pub workflow_id: WorkflowId,
    pub title: String,
    /// Why this workflow. **Entry zero's rationale**, and the only durable
    /// trace the call ever ran — nothing else on the record says a proposal
    /// happened.
    pub because: Option<String>,
    /// The one-based positions of the Jobs that must finish first. **Always
    /// earlier than this one**, which is what makes a plan creatable in order.
    pub after: Vec<usize>,
}

/// What one call proposed. **No arm of this means "the usual one".**
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum Proposal {
    /// The Jobs the request became, in dependency order. Never empty.
    Resolved(Vec<ProposedJob>),
    /// No workflow resolved, and the request is refused at dispatch.
    Unresolved(Unresolved),
}

/// Why no workflow resolved. **Both arms end the same way for the person** —
/// the request comes back — and they are two values because the sentence a
/// person reads is different.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum Unresolved {
    /// It read the request and none of the workflows covers it.
    NoneFits { because: Option<String> },
    /// It named a workflow this repository does not hold. Refused rather than
    /// nearest-matched: a name nothing holds is not evidence about which one
    /// was meant.
    NotHeld { named: String },
}

impl fmt::Display for Unresolved {
    fn fmt(&self, out: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Unresolved::NoneFits { because: Some(why) } => write!(
                out,
                "no workflow this repository holds fits the request: {why}"
            ),
            Unresolved::NoneFits { because: None } => {
                out.write_str("no workflow this repository holds fits the request")
            }
            Unresolved::NotHeld { named } => write!(
                out,
                "the proposal named `{named}`, which this repository does not hold"
            ),
        }
    }
}

/// Why no proposal was made at all. **Never an assignment**, and never the
/// refusal above: a request nothing could be read about is not a request that
/// was read and declined.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum NotProposed {
    /// The call itself — the program, the network, the quota, the budget.
    Call(CallFailed),
    /// It answered, and the answer has no `workflow:` line in it.
    NamesNoWorkflow,
    /// A Job in the plan was given no name. Fleet writes none of its own: a
    /// title is the requester's words, and inventing one invents them.
    NamesNoTitle { at: usize },
    /// A Job waits on one that is not before it, or on one that is not in the
    /// plan at all. **Neither is creatable**: an edge points at a minted id, so
    /// a plan is created in order or not at all.
    OutOfOrder { at: usize, after: usize },
}

impl fmt::Display for NotProposed {
    fn fmt(&self, out: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            NotProposed::Call(why) => write!(out, "{why}"),
            NotProposed::NamesNoWorkflow => out.write_str(
                "the answer names no workflow at all, so nothing was proposed either way",
            ),
            NotProposed::NamesNoTitle { at } => write!(
                out,
                "job {at} was given no name, and Fleet does not write one of its own"
            ),
            NotProposed::OutOfOrder { at, after } => write!(
                out,
                "job {at} waits on job {after}, which is not before it in the plan"
            ),
        }
    }
}

impl std::error::Error for NotProposed {}

/// What one call is asked, assembled.
///
/// **Held rather than passed as a bare string**, for [`verification::Brief`]'s
/// reason: what the proposer is told is built in one place and asserted on
/// without a model.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Brief {
    question: String,
}

impl Brief {
    /// Assemble the question.
    ///
    /// **Two inputs and no third.** There is no parameter for the Manifest, the
    /// repository, the Board or the Jobs already running — every extra token is
    /// money on a call that fires on every dispatch, and a proposer that could
    /// read the repository is a Drone at many times the price.
    pub fn about(request: &str, workflows: &BTreeMap<WorkflowId, ResolvedWorkflow>) -> Brief {
        let mut question = String::new();
        question.push_str(
            "You are deciding what a piece of work is: which workflow it runs \
             under, what it will write, and whether it is one Job or several. \
             Answer only the question at the end.\n\n",
        );
        question.push_str("The request, as the person wrote it:\n\n");
        question.push_str(request);
        question.push_str("\n\nThe workflows this repository holds, and the steps each runs:\n\n");
        if workflows.is_empty() {
            question.push_str("  (this repository holds none)\n");
        }
        for workflow in workflows.values() {
            question.push_str(&format!(
                "  {} — {}\n",
                workflow.id().as_str(),
                workflow.name()
            ));
            // The steps are how a workflow is told apart from its five
            // neighbours. A name alone separates `bug` from `revert` and does
            // not separate `feature` from `refactor`.
            let steps: Vec<&str> = workflow.steps().iter().map(|step| step.label()).collect();
            question.push_str(&format!("    {}\n", steps.join(" -> ")));
        }
        question.push('\n');
        question.push_str(ANSWER_FORMAT);
        Brief { question }
    }

    /// The text that goes to the model, exactly as it goes.
    pub fn question(&self) -> &str {
        &self.question
    }

    /// Read one answer back, against the workflows this Fleet actually holds.
    ///
    /// **The catalogue is a parameter and not a convenience.** It is what makes
    /// a workflow nothing holds unrepresentable in the `Ok` half: there is no
    /// input to this that produces a `Resolved` naming an id the map does not
    /// have, none that produces one with a blank title, and none that produces
    /// a plan whose edges do not point backwards.
    pub fn read(
        &self,
        answer: &str,
        held: &BTreeMap<WorkflowId, ResolvedWorkflow>,
    ) -> Result<Proposal, NotProposed> {
        let blocks = blocks(answer);
        // Declining is one answer about the whole request rather than one Job's
        // line, so it is read off the first block and ends the reading.
        let Some(first) = blocks.first() else {
            return Err(NotProposed::NamesNoWorkflow);
        };
        if declines(first) {
            return Ok(Proposal::Unresolved(Unresolved::NoneFits {
                because: field(first, "because"),
            }));
        }
        let mut jobs = Vec::with_capacity(blocks.len());
        for (position, block) in blocks.iter().enumerate() {
            let at = position + 1;
            let Some(named) = field(block, "workflow") else {
                return Err(NotProposed::NamesNoWorkflow);
            };
            let workflow_id = WorkflowId::carried(Ulid::carried(named.clone()));
            if !held.contains_key(&workflow_id) {
                return Ok(Proposal::Unresolved(Unresolved::NotHeld { named }));
            }
            let title = field(block, "title").ok_or(NotProposed::NamesNoTitle { at })?;
            let after = after(block);
            if let Some(&ahead) = after.iter().find(|&&waits| waits >= at || waits == 0) {
                return Err(NotProposed::OutOfOrder { at, after: ahead });
            }
            jobs.push(ProposedJob {
                workflow_id,
                title,
                because: field(block, "because"),
                after,
            });
        }
        Ok(Proposal::Resolved(jobs))
    }
}

/// One block per `job:` line, or the whole answer where it names none.
///
/// The tolerance is one-directional on purpose: an answer for a single Job that
/// skipped the numbering is still one Job, while an answer for several that
/// skipped it is not expressible — there would be nothing for `after` to name.
fn blocks(answer: &str) -> Vec<String> {
    let mut found: Vec<String> = Vec::new();
    for line in answer.lines() {
        if line.trim().starts_with("job:") {
            found.push(String::new());
        }
        if let Some(current) = found.last_mut() {
            current.push_str(line);
            current.push('\n');
        }
    }
    match found.is_empty() {
        true => vec![answer.to_string()],
        false => found,
    }
}

/// Whether this block declines rather than choosing.
fn declines(block: &str) -> bool {
    field(block, "workflow").is_some_and(|named| named.eq_ignore_ascii_case("none"))
}

/// The job numbers this block waits on. A word that is not a number is dropped
/// rather than refused — `after: none` is a model saying no.
fn after(block: &str) -> Vec<usize> {
    field(block, "after")
        .map(|line| {
            line.split(',')
                .filter_map(|entry| entry.trim().parse::<usize>().ok())
                .collect()
        })
        .unwrap_or_default()
}
