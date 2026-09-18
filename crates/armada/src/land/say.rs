//! `merge_outcome`, with the clock reading it takes as an argument supplied
//! here — `scripts/land`'s own `say()`, which stamps every call with
//! `time.strftime` fresh. `fleet::clock::SystemClock` is this crate's own
//! doorway onto the machine's clock, matching every other caller of it.

use fleet::clock::{Clock, SystemClock};

use super::dir::StateDir;
use super::outcome::{merge_outcome, Outcome, OutcomePatch, OutcomeState};
use super::stop::Stopped;

pub fn say(
    state: &StateDir,
    branch: &str,
    status: OutcomeState,
    detail: impl Into<String>,
    patch: OutcomePatch,
) -> Result<Outcome, Stopped> {
    let updated = SystemClock::new().now();
    merge_outcome(state, branch, status, detail, updated.as_str(), patch)
        .map_err(|why| Stopped::stopped(why.to_string()))
}
