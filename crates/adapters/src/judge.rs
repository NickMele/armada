//! The same CLI as a Drone, invoked as a call rather than as a session.
//!
//! # It is not a Drone, and the argument list is where that is true
//!
//! No `--input-format stream-json`, so stdin is one prompt and then EOF rather
//! than a session. `--max-turns 1`, so the model answers once. An empty
//! `--allowedTools`, so nothing is permitted. `--strict-mcp-config` with no
//! `--mcp-config`, so no server at all is reachable — not even the Evidence
//! tool a Drone always holds.
//!
//! # A Judge cannot reach the repository because nothing points it at one
//!
//! `JudgeCall` carries no directory, so there is no worktree on the argument
//! list and none to inherit: Fleet runs it somewhere with no repository under
//! it. The patch is text inside the question.
//!
//! # `--allowedTools ""` is a floor, not a fence
//!
//! Measured in this crate's harness spike: the flag is a permission allowlist
//! and removed none of the built-in tools. What actually bounds a Judge is
//! having a single turn and a working directory with nothing in it.

use adapter_traits::{Ask, JudgeCall, ModelClient};

use crate::harness::HeadlessAgent;

/// The model a Judge call gets when nothing names one: the cheapest alias this
/// CLI takes.
///
/// **`crates/config/settings.toml` names none** — the `judge-model` row reads
/// `undecided` — so the value comes from here for the reason the Drone's
/// default does, and is a stand-in until that row carries one.
const JUDGE_MODEL: &str = "haiku";

impl HeadlessAgent {
    /// The model a step that names none is judged by.
    pub fn judge_model() -> &'static str {
        JUDGE_MODEL
    }
}

impl ModelClient for HeadlessAgent {
    fn render(&self, ask: &Ask) -> JudgeCall {
        let args: Vec<String> = vec![
            "-p".into(),
            "--output-format".into(),
            "text".into(),
            "--model".into(),
            ask.model().as_str().into(),
            // One turn. A verifier that can take a second turn is a verifier
            // that can go looking, which is the property this call is bought
            // for.
            "--max-turns".into(),
            "1".into(),
            "--permission-mode".into(),
            "dontAsk".into(),
            // No `--mcp-config` beside it, which is what makes the set empty.
            "--strict-mcp-config".into(),
            "--allowedTools".into(),
            String::new(),
        ];
        JudgeCall::rendered(ask, self.program(), args)
    }
}
