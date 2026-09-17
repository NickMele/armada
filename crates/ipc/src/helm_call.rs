//! One call a Helm session made that the person's own settings do not cover,
//! held open while they answer it in the dock. `#1389`.
//!
//! **A Drone's [`CommandInFlight`](crate::CommandInFlight) one subject over,
//! and deliberately not that type.** Every field of that one is about a Job —
//! `step_id`, `AllowForJob`, a rule written into `armada.yml` on the Job's
//! branch — and a Helm call has no Job. The two sets of answers are different
//! sets and a shared enum would offer a person an answer Fleet cannot take.
//!
//! **What decides is the person's own configuration, not Armada's.** Fleet
//! refuses nothing here and allows nothing here; it carries the question to the
//! dock and carries the answer back inside the call. Spike 19 measured the
//! path, spike 15 measured it first for a Drone.

use serde::{Deserialize, Serialize};

use crate::ids::{Instant, ManifestId};

/// What the CLI sends the door when it puts a call to a person — the
/// `--permission-prompt-tool` contract, as spike 15 recorded it.
///
/// **Deserialized here because this is where the bytes are**: `ipc` is one of
/// the two crates that may read JSON, and the tool's arguments arrive as an
/// agent's own object rather than as one of Armada's DTOs.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct AskingToRun {
    /// The tool the session reached for, in the harness's own spelling.
    pub tool_name: String,
    /// Its whole input, as the session sent it. **Carried back untouched on an
    /// allow** — the CLI runs `updatedInput`, so anything Armada rewrote here
    /// would be Armada editing a person's command behind them.
    #[serde(default)]
    pub input: serde_json::Value,
    /// The harness's id for the call, where it sent one.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub tool_use_id: Option<String>,
}

impl AskingToRun {
    /// What the call was on, in the words a person would use — a command, a
    /// path, a pattern.
    ///
    /// **`crates/adapters/src/transcript.rs`'s rule, read off a `Value`
    /// instead.** The keys are the tools' own and the order is
    /// first-present-wins, so a tool nothing here names still shows its command
    /// or its path, and none of it is an arm per tool. `None` is a tool whose
    /// arguments this vocabulary has no name for — never an invented line.
    pub fn detail(&self) -> Option<&str> {
        const KEYS: &[&str] = &[
            "command",
            "file_path",
            "path",
            "pattern",
            "url",
            "query",
            "prompt",
            "description",
        ];
        KEYS.iter()
            .find_map(|key| self.input.get(key)?.as_str())
            .filter(|said| !said.trim().is_empty())
    }
}

/// What the door answers the CLI with. **The whole vocabulary is two words**,
/// and it is the harness's rather than Armada's.
#[derive(Clone, Debug, PartialEq, Serialize)]
#[serde(tag = "behavior", rename_all = "lowercase")]
pub enum RunOrNot {
    Allow {
        /// The input as it arrived. See [`AskingToRun::input`].
        #[serde(rename = "updatedInput")]
        updated_input: serde_json::Value,
    },
    Deny {
        /// What the session is shown in place of the call's result. It reaches
        /// the model, so it is written for one.
        message: String,
    },
}

/// What a person may answer about one Helm call.
///
/// **Three, where a Drone's command offers three others.** There is no
/// `AllowForJob` because there is no Job, and no `armada.yml` to write to:
/// Helm runs in the person's own checkout under the person's own settings, so
/// the only place a lasting rule belongs is a settings file of theirs.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum HelmCallAnswer {
    /// Run it, this once. Nothing is written anywhere, and the same call asks
    /// again next time.
    AllowOnce,
    /// Run it, and write [`HelmCallInFlight::rule`] into the repository's own
    /// `.claude/settings.local.json` so the CLI allows it without asking — in
    /// Helm and in the person's terminal alike.
    ///
    /// **The only thing in Armada that writes a person's settings**, and it
    /// does so on this answer and no other.
    AllowAndRemember,
    /// Do not run it. The session is told, and goes on without it.
    Refuse,
}

/// One call a Helm session is waiting on a person to answer, right now.
///
/// **It is not a status.** No Job moves, nothing is queued, and the wait ends
/// without either — the same argument
/// [`CommandInFlight`](crate::CommandInFlight) makes for a Drone. What is
/// waiting is one process, inside one tool call, for the length of one reply.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct HelmCallInFlight {
    /// Fleet's own id for this ask. **What an answer names**, so an answer
    /// from a window left open across one is a refusal rather than a
    /// coincidence.
    pub call: String,
    /// The repository whose Helm session asked. A person sees several.
    pub manifest_id: ManifestId,
    /// When Fleet was asked, by Fleet's clock.
    pub asked_at: Instant,
    /// The tool reached for, in the harness's own spelling.
    pub tool: String,
    /// The command, or the argument, bounded to one line. **Empty is a tool
    /// whose argument this vocabulary has no name for**, never an invented one.
    pub detail: String,
    /// Whether [`detail`](HelmCallInFlight::detail) is less than what was sent.
    pub truncated: bool,
    /// How many characters the argument had before anything was cut.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub length: Option<usize>,
    /// **The rule that would have to allow it**, in the CLI's own settings
    /// spelling — `Bash(gh issue list:*)`, `Edit`. What
    /// [`HelmCallAnswer::AllowAndRemember`] writes, and what a person reads
    /// before choosing it.
    pub rule: String,
    /// What a person may answer, in the order a surface draws them. An answer
    /// that is not here is refused.
    pub offers: Vec<HelmCallAnswer>,
    /// How long Fleet will hold the call open, in seconds, from `asked_at`. A
    /// surface says it rather than deriving it: the bound is Fleet's.
    pub holding_for_seconds: u64,
}

/// A person's answer to one Helm call. The request half of `answer_helm_call`.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct AnswerHelmCall {
    /// [`HelmCallInFlight::call`]. An id naming nothing waiting is a 409.
    pub call: String,
    /// One of that call's offers. Anything else is a 409.
    pub answer: HelmCallAnswer,
    /// Why, in the person's own words, carried to the session inside the
    /// refusal. **Only a refusal reads it** — `AnswerCommand::note`'s rule and
    /// its reason: the reason is in mind at the moment of refusing.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub note: Option<String>,
}

/// `helm.asking_to_run`: a call is waiting on a person. **Absent from
/// [`HelmCallsWaiting`] is the answer having landed**, which is how
/// `job.command_waiting` says the same thing.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct HelmAskingToRun {
    pub waiting: HelmCallInFlight,
}

/// `helm.call_answered`: what became of one ask, published whoever ended it —
/// a person, or the bound running out.
///
/// **Helm's act as its own event type**, the rule `helm.changed_checkout` and
/// `studio.helm_acted` already follow. A person who finds a command was run
/// reads this to see that they allowed it.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct HelmCallAnswered {
    pub call: String,
    pub manifest_id: ManifestId,
    /// The tool and the one-line detail again, so the record reads on its own.
    pub tool: String,
    pub detail: String,
    pub rule: String,
    pub settled: HelmCallSettled,
    pub at: Instant,
}

/// How one ask ended.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum HelmCallSettled {
    /// A person allowed it, this once.
    AllowedOnce,
    /// A person allowed it and Fleet wrote the rule into their settings.
    AllowedAndRemembered,
    /// A person allowed it and the rule could not be written. **The call still
    /// ran**: the answer was theirs and a settings file that would not open is
    /// not a reason to refuse them.
    AllowedButNotRemembered,
    /// A person refused it.
    Refused,
    /// Nobody answered inside the bound, so Fleet answered for them with a
    /// refusal. **Never an allow** — silence is not consent.
    Unanswered,
    /// The session went before the answer did. Nothing was run.
    SessionGone,
}

/// What `list_helm_calls` answers: every ask waiting on this person right now,
/// across every repository Fleet serves.
///
/// **Read on a resync and never polled.** The events above carry each ask as it
/// arrives; this is what a Bridge that started mid-wait reads once.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct HelmCallsWaiting {
    pub waiting: Vec<HelmCallInFlight>,
}
