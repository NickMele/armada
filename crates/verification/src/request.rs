//! What was asked for, in the requester's own words.
//!
//! # This is the outermost yardstick, and it is not the Drone's
//!
//! Constitutional rule 2 says the Judge is blind to the Drone's account **and**
//! that it receives the original task text. Only the first half was built: a
//! [`Brief`](crate::Brief) carried the step, the criterion, the work product,
//! the earlier steps' evidence and the Checks, and nothing at all about what
//! the Job was for. So a criterion could ask whether a document was internally
//! coherent and could not ask whether it answered the request — which is why
//! `feature/scope` and `bug/plan_matches_request` were dropped when the
//! workflows were restored, and why a beautifully-written scope note for the
//! wrong task advanced.
//!
//! # The source is a type, for [`Written`](crate::Written)'s reason
//!
//! [`Request::of`] takes a [`Job`] and nothing else. There is no constructor
//! from three strings and none from a [`Submission`](crate::Submission), so
//! there is no route by which a Drone's own words could arrive labelled as the
//! thing its work is measured against. Every field it reads is frozen at Job
//! creation — a Drone cannot propose a Job, and nothing in `fleet` appends to
//! `facts` after `drafted` builds it.
//!
//! # The whole Job's bar, never one step's
//!
//! `acceptance_criteria[]` is the Job's, and no criterion of a step names which
//! one of them it tests. `crates/config/src/judge.rs` refuses `source_ref` for
//! exactly that reason — the join from a Judge criterion into
//! `job.acceptance_criteria[]` **is not built**, and nothing here builds it.
//! What that costs is honest and stated in the text: the Judge is told these
//! are the conditions the *Job* must satisfy, so it cannot read one of them as
//! this step's own bar and refuse a first step for not having finished the
//! work.

use core_model::Job;

/// The request a Job answers: its title, its facts, and what its requester
/// said "done" means.
///
/// **Borrowed and [`Copy`]**, the shape [`Reference`](crate::Reference) already
/// has — a brief holds what it was given for as long as the call takes and owns
/// none of it.
///
/// There is no empty one and no `Option` of one. A [`Title`](core_model::Title)
/// cannot be blank, so every Job has a request, and a `Brief` that could be
/// assembled without one is the defect this type exists to close.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Request<'a> {
    title: &'a str,
    facts: &'a str,
    criteria: &'a [core_model::AcceptanceCriterion],
}

impl<'a> Request<'a> {
    /// The request, read off the Job that carries it.
    ///
    /// **The only constructor.** See this module's note on why it takes a
    /// `Job` rather than the three fields.
    pub fn of(job: &'a Job) -> Request<'a> {
        Request {
            title: job.title().as_str(),
            facts: job.facts().as_str(),
            criteria: job.acceptance_criteria(),
        }
    }

    /// The one line a person would recognise the Job by.
    pub fn title(&self) -> &'a str {
        self.title
    }

    /// The context the requester wrote. Legitimately empty.
    pub fn facts(&self) -> &'a str {
        self.facts
    }

    /// What the requester said "done" means, for the whole Job.
    pub fn criteria(&self) -> &'a [core_model::AcceptanceCriterion] {
        self.criteria
    }

    /// The request, laid out for one call.
    ///
    /// **Labelled twice over**: as the requester's words rather than anyone
    /// else's, and as the Job's bar rather than this step's. The second label
    /// is what stops the missing `source_ref` join becoming a refusal — a Judge
    /// shown a Job's acceptance criteria beside a first step's scope note would
    /// otherwise read them as conditions that step failed to meet.
    pub(crate) fn told(&self) -> String {
        let mut told = String::from(
            "What was asked for, in the requester's own words. This is the \
             request the whole Job answers, and it is the standard the work \
             below is measured against rather than something under judgment:\n\n",
        );
        told.push_str(&format!("  the request: {}\n", self.title));
        if !self.facts.trim().is_empty() {
            told.push_str(&format!("  what they said about it: {}\n", self.facts));
        }
        if !self.criteria.is_empty() {
            told.push_str(
                "  the whole Job is done when, which is not this step's bar on \
                 its own:\n",
            );
            for criterion in self.criteria {
                told.push_str(&format!("    - {}\n", criterion.text));
            }
        }
        told.push('\n');
        told
    }
}
