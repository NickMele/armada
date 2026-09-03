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

use adapter_traits::{Ask, Heard, JudgeCall, ModelClient};

use crate::harness::HeadlessAgent;

/// The model a request is read by when nothing names one.
///
/// **A constant, where the Judge's is derived.** They are separate dials for
/// the reason `ARMADA_JUDGE_MODEL` is not `ARMADA_MODEL`: a machine that raised
/// one by raising the other would pay the raise on every dispatch as well as on
/// every criterion. `crates/config/settings.toml`'s `job-proposer-model` row
/// still reads `undecided`, so this one is a stand-in stating what the binary
/// takes — the arrangement [`HeadlessAgent::judge_model`] was in until that
/// row was decided.
const PROPOSER_MODEL: &str = "haiku";

impl HeadlessAgent {
    /// The model a step that names none is judged by: **the cheapest alias on
    /// the roster**, which is what `crates/config/settings.toml`'s
    /// `judge-model` row decided on 28 Aug 2026.
    ///
    /// **Read off the roster rather than written down again.** A constant here
    /// would be a second statement of a value the settings row now owns, and
    /// two statements is how the second one goes stale. The row's peer polarity
    /// is *member of the Kit set*, and an entry of [`HeadlessAgent::models`] is
    /// a member by construction rather than by a check.
    ///
    /// **The last entry, because the roster runs strongest first** — the order
    /// a picker offers, so its end is its cheap end. That is an assumption
    /// about a list this file does not own, which is why a test pins what comes
    /// back to the settings row's value: a reordered roster breaks the build
    /// rather than quietly redeciding a setting.
    pub fn judge_model() -> &'static str {
        match HeadlessAgent::models().last() {
            Some(cheapest) => cheapest,
            // The roster is a non-empty constant, so nothing reaches here. The
            // arm exists so that deriving a value cannot panic at a gate, and
            // it falls back to the model a Job itself would get rather than to
            // a fourth spelling written for the fallback's sake.
            None => HeadlessAgent::default_model(),
        }
    }

    /// The model a dispatch request is read by when nothing names one.
    pub fn proposer_model() -> &'static str {
        PROPOSER_MODEL
    }
}

impl ModelClient for HeadlessAgent {
    fn render(&self, ask: &Ask) -> JudgeCall {
        JudgeCall::rendered(ask, self.program(), asking(ask, Watched::No))
    }

    /// The same call, printing what it is doing while it does it.
    ///
    /// **Every confinement argument is the same one**, which is the property
    /// worth stating: the two renders differ in the output format and in
    /// nothing else, so a watched call is not a call with a longer leash. The
    /// shared builder below is what makes that true by construction rather than
    /// by two lists agreeing.
    fn render_watched(&self, ask: &Ask) -> JudgeCall {
        JudgeCall::rendered(ask, self.program(), asking(ask, Watched::Yes))
    }

    /// Read one line of what [`render_watched`](ModelClient::render_watched)
    /// prints. **The other half of that render**, and the reason both are on
    /// one trait: the flag that chooses the format and the reader that
    /// understands it are one decision, and a client that changed the format
    /// without the reader would go quiet rather than fail.
    fn heard(&self, line: &str) -> Heard {
        crate::watching::heard(line)
    }
}

/// Whether the call reports on itself. The one axis the two renders differ on.
#[derive(Clone, Copy, PartialEq, Eq)]
enum Watched {
    No,
    Yes,
}

/// The argument list both renders use.
///
/// **One builder, because the confinement is the argument list.** A Judge's
/// bounds are `--max-turns 1`, an empty `--allowedTools` and a strict MCP
/// config with no file — and this module's own note says the flag is a floor
/// rather than a fence, so what actually holds is the shape of the argv. Two
/// lists that happened to agree the day they were written would stop agreeing
/// the first time one of them was edited, and the one that drifts is whichever
/// is read less.
fn asking(ask: &Ask, watched: Watched) -> Vec<String> {
    let mut args: Vec<String> = vec!["-p".into(), "--output-format".into()];
    match watched {
        // The Drone's format, on a call that is not a Drone. It carries the
        // harness's own progress lines, which is the whole reason to pay for
        // it: `--include-partial-messages` is what makes the model's answer
        // arrive as it is written rather than at the end. `crate::watching`
        // reads them, and `--verbose` is what the format requires to emit them.
        Watched::Yes => {
            args.push("stream-json".into());
            args.push("--verbose".into());
            args.push("--include-partial-messages".into());
        }
        // Plain text: the answer on stdout and nothing else. Still the default
        // for the Judge, whose caller has nobody waiting on it — a gate runs
        // without a person watching, and paying for a stream nothing reads
        // would be paying for the surface rather than the verdict.
        Watched::No => args.push("text".into()),
    }
    args.extend([
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
    ]);
    args
}
