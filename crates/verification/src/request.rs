//! What was asked for, in the requester's own words.
//!
//! # The outermost yardstick, and it is not the Drone's
//!
//! Rule 2 says the Judge is blind to the Drone's account **and** that it
//! receives the original task text. Only the first half was built, so a
//! criterion could ask whether a document held together and could not ask
//! whether it answered anything — and a scope note for the wrong task passed.
//!
//! # The source is a type, for [`Written`](crate::Written)'s reason
//!
//! [`Request::of`] takes a [`Job`] and nothing else: no constructor from three
//! strings, none from a [`Submission`](crate::Submission). So there is no route
//! by which the Drone's own words arrive labelled as the standard. Every field
//! it reads is frozen at creation — a Drone cannot propose a Job, and nothing
//! appends to `facts` after `fleet::drafting` builds it.
//!
//! # The whole Job's bar, never one step's
//!
//! No criterion of a step names which acceptance criterion it tests;
//! `config/src/judge.rs` refuses `source_ref` because that join **is not
//! built**, and nothing here builds it. So the text says the criteria are the
//! *Job's*, which is what stops a Judge reading them as this step's bar and
//! refusing a first step for not having finished the work.

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
