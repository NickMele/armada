//! What a Judge call is started as, in a crate that cannot start one.
//!
//! # The same split as the Drone, for a different reason
//!
//! [`ModelClient`] renders an [`Ask`] into a [`JudgeCall`] and Fleet runs it.
//! For a Drone the split makes confinement assertable without spawning; here it
//! also keeps `adapters` free of a process, so the one thing that starts
//! anything stays `fleet`'s.
//!
//! # What a [`JudgeCall`] cannot say
//!
//! There is no worktree, no directory, no MCP configuration path and no
//! toolbelt on this type. A Judge reads the evidence it was handed and cannot
//! reach the repository, because there is no field in which to name one — not
//! because a check rejects the attempt.
//!
//! The question goes in on stdin for [`crate::harness`]'s reason: argv is
//! world-readable through `ps`, and a criterion quotes the work.

use alloc::string::String;
use alloc::vec::Vec;

use crate::harness::{Environment, Model, SpawnConfigRefused};

/// One narrow question, put to one model.
///
/// **One criterion per ask.** A call carrying several questions gets one answer
/// covering all of them, which is the open-ended prompt the narrow-question
/// rule exists to refuse.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Ask {
    model: Model,
    question: String,
    environment: Environment,
}

impl Ask {
    /// The only way to make one. An empty question is refused where it is
    /// written rather than answered agreeably by a model.
    ///
    /// The environment is the caller's, not the adapter's: a Judge call is
    /// Fleet's own and authenticates as Fleet, which is the one way it differs
    /// from a Drone's confinement rather than copies it.
    pub fn put(
        model: Model,
        question: &str,
        environment: Environment,
    ) -> Result<Ask, SpawnConfigRefused> {
        if question.trim().is_empty() {
            return Err(SpawnConfigRefused::PromptEmpty);
        }
        Ok(Ask {
            model,
            question: String::from(question),
            environment,
        })
    }

    pub fn model(&self) -> &Model {
        &self.model
    }

    pub fn environment(&self) -> &Environment {
        &self.environment
    }

    /// The assembled question. **Not an argument** — see this module's comment.
    pub fn question(&self) -> &str {
        &self.question
    }
}

/// A one-shot model call: a process, not yet started.
///
/// **No directory and no stdio.** Where it runs is chosen by whoever runs it,
/// and a field here would be a way for an adapter to point a Judge at a
/// worktree.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct JudgeCall {
    program: String,
    args: Vec<String>,
    environment: Environment,
    question: String,
}

impl JudgeCall {
    /// Render an ask into a startable process.
    ///
    /// **The question and the environment come from the ask**, not from the
    /// caller, so an implementation cannot put a different question on stdin
    /// or hand the call something it was not given.
    pub fn rendered(ask: &Ask, program: &str, args: Vec<String>) -> JudgeCall {
        JudgeCall {
            program: String::from(program),
            args,
            environment: ask.environment().clone(),
            question: String::from(ask.question()),
        }
    }

    pub fn program(&self) -> &str {
        &self.program
    }

    pub fn args(&self) -> &[String] {
        &self.args
    }

    pub fn environment(&self) -> &Environment {
        &self.environment
    }

    /// What goes in on stdin, and then the pipe closes. A Judge takes one turn
    /// because there is nothing left to write to it.
    pub fn question(&self) -> &str {
        &self.question
    }
}

/// A model call that carries no toolset — the Judge, the Job-shape classifier,
/// and generated copy. Distinct from [`crate::AgentHarness`], which carries one.
///
/// # Rendering cannot fail
///
/// There is no associated error type and no `Result`, which is what makes this
/// object-safe: Fleet holds one behind a pointer rather than being generic over
/// it, the way it already holds a clock. Everything an implementation could
/// refuse — a blank model, an empty question — is refused by [`Ask`] before one
/// exists.
pub trait ModelClient {
    /// How this client would ask one question.
    fn render(&self, ask: &Ask) -> JudgeCall;
}
