//! The two ways a call under `land/` may end early: [`Refused`], before
//! anything joins the line, and [`Stopped`], which ends one branch's turn
//! with a stored outcome — `scripts/land`'s `Refused` and `Stop` exceptions,
//! as values propagated with `?` rather than raised and caught.

use super::git::GitFailed;
use super::outcome::{OutcomePatch, OutcomeState};

/// Said to the caller before anything joins the line — `preflight` and
/// `land` (joining the queue) are the only two callers that produce this.
/// Never written to an outcome file: there is no branch turn yet for it to
/// belong to.
#[derive(Debug)]
pub struct Refused(pub String);

impl std::fmt::Display for Refused {
    fn fmt(&self, out: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        out.write_str(&self.0)
    }
}

impl std::error::Error for Refused {}

impl From<GitFailed> for Refused {
    fn from(cause: GitFailed) -> Refused {
        Refused(cause.to_string())
    }
}

/// Ends one branch's turn, once it is in the runner. Caught exactly once, in
/// the runner loop, which is what turns this into a stored [`Outcome`](super::outcome::Outcome)
/// via [`merge_outcome`](super::outcome::merge_outcome).
#[derive(Debug)]
pub struct Stopped {
    pub state: OutcomeState,
    pub detail: String,
    pub patch: OutcomePatch,
}

impl Stopped {
    /// The ordinary case: something in the turn's own plumbing did not run,
    /// or the branch's Checks pass but the tool that would tell is missing.
    /// Nothing about the branch's own combination was learned either way.
    pub fn stopped(detail: impl Into<String>) -> Stopped {
        Stopped {
            state: OutcomeState::Stopped,
            detail: detail.into(),
            patch: OutcomePatch::default(),
        }
    }

    /// The gate or a Check found the combination broken.
    pub fn red(detail: impl Into<String>, patch: OutcomePatch) -> Stopped {
        Stopped {
            state: OutcomeState::Red,
            detail: detail.into(),
            patch,
        }
    }

    /// The base does not merge into the branch cleanly.
    pub fn conflict(detail: impl Into<String>, patch: OutcomePatch) -> Stopped {
        Stopped {
            state: OutcomeState::Conflict,
            detail: detail.into(),
            patch,
        }
    }

    /// A `Stopped` in a state neither [`stopped`](Stopped::stopped),
    /// [`red`](Stopped::red) nor [`conflict`](Stopped::conflict) covers —
    /// `prove`'s own `ungated` and `landed`, which are constructed nowhere
    /// else.
    pub fn of(state: OutcomeState, detail: impl Into<String>, patch: OutcomePatch) -> Stopped {
        Stopped {
            state,
            detail: detail.into(),
            patch,
        }
    }

    /// Attach a patch to a `Stopped` built without one — `stopped()` is
    /// usually called with just a detail, and a few call sites need to
    /// carry `failed`/`new_lines`/`logs` on a stopped turn too.
    pub fn with_patch(mut self, patch: OutcomePatch) -> Stopped {
        self.patch = patch;
        self
    }
}

impl From<GitFailed> for Stopped {
    fn from(cause: GitFailed) -> Stopped {
        Stopped::stopped(cause.to_string())
    }
}
