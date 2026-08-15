//! The Drone: **the argv Fleet builds, and the ledger it reads back**.
//!
//! A Job's conversation is an ordinary Claude Code session (PHASES.md §9.1 F1),
//! so there is no session mechanism here — only a uuid the caller assigns before
//! anything starts, and a subprocess.
//!
//! ```text
//! claude --session-id <uuid> --print --output-format stream-json <prompt>
//! claude --resume     <uuid> --print --output-format stream-json <answer>
//! claude --resume     <uuid>                                     # boarding
//! ```
//!
//! **This module is where the bugs are, which is why it is pure.** A missing
//! `--session-id` mints a session Fleet cannot find again; `--resume` where
//! `--session-id` was meant starts a Job's second turn as its first. Both are
//! argv bugs, and a test that faked a higher layer would catch neither
//! (`ARCHITECTURE.md` §1.1). **No test in this repository spawns a real session
//! or spends a token** (PHASES.md §8.5) — the argv is asserted here and recorded
//! `stream-json` is fed back as the response.
//!
//! **Budgets need no accounting layer** (PHASES.md §9.1 F2). Every turn ends
//! with a `result` event carrying `total_cost_usd`, `usage`, `num_turns` and
//! `duration_api_ms`; [`ledger`] reads them and nothing here estimates anything.

use super::job::Spend;
use crate::error::{ArmadaError, ErrClass};
use serde::Serialize;

/// The program every Drone is.
pub const CLAUDE: &str = "claude";

/// The argv for a Job's **first** turn.
///
/// `--session-id` is what makes the caller the one who assigns identity, which
/// is the whole of PHASES.md §9.1 F1: the uuid exists before the process, so the
/// transcript's location is known before there is a transcript.
pub fn spawn_argv(uuid: &str, prompt: &str) -> Vec<String> {
    let mut argv = vec![
        CLAUDE.to_string(),
        "--session-id".to_string(),
        uuid.to_string(),
    ];
    argv.extend(headless());
    argv.push(prompt.to_string());
    argv
}

/// The argv for **continuing** a Job — `armada fleet answer`.
///
/// **An answer is a continuation, not a new run**, so the budget is not reset
/// and the session is resumed rather than minted. Resetting the ceiling here
/// would make budgets unenforceable for any Job that asks a question.
pub fn resume_argv(uuid: &str, prompt: &str) -> Vec<String> {
    let mut argv = vec![CLAUDE.to_string(), "--resume".to_string(), uuid.to_string()];
    argv.extend(headless());
    argv.push(prompt.to_string());
    argv
}

/// The argv `armada fleet board` prints, and execs under `--exec`.
///
/// **Interactive, and deliberately so.** Boarding hands you the conversation to
/// drive yourself; it does not stream a running Drone's output at you, which is
/// the pty work withdrawn in PHASES.md §9.1 F1.
pub fn board_argv(uuid: &str) -> Vec<String> {
    vec![CLAUDE.to_string(), "--resume".to_string(), uuid.to_string()]
}

/// A bounded headless turn with a live event stream.
fn headless() -> [String; 3] {
    [
        "--print".to_string(),
        "--output-format".to_string(),
        "stream-json".to_string(),
    ]
}

/// What one turn reported.
#[derive(Debug, Clone, PartialEq, Serialize)]
pub struct Turn {
    /// The ledger, summed from the `result` event.
    pub spend: Spend,
    /// Why the turn ended, as Claude Code spelled it.
    pub stop_reason: Option<String>,
    /// Whether the turn itself failed.
    pub is_error: bool,
    /// The turn's own text, when it produced one.
    pub result: Option<String>,
    /// The rate-limit window, when one was reported.
    ///
    /// **Strictly better than a fixed concurrency cap**, which was only ever a
    /// proxy for the same thing: the orchestrator can decline to spawn when a
    /// window reset is close (PHASES.md §9.1 F2).
    pub rate_limit: Option<RateLimit>,
}

/// The rate-limit window a turn passed through.
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct RateLimit {
    /// `allowed`, `allowed_warning`, `rejected` — whatever the event said.
    pub status: String,
    /// Which window: `five_hour`, and whatever else arrives.
    pub kind: String,
    /// When it resets, as seconds since the epoch.
    pub resets_at: Option<u64>,
}

/// Read a turn out of a `stream-json` transcript.
///
/// **One JSON document per line, and only two of them matter.** Everything
/// between the `system` init and the final `result` is the conversation, which
/// is the Drone's business and not Fleet's — PLAN.md §15.2's rule that the
/// orchestrator reads summaries rather than raw transcripts starts here, with
/// Fleet declining to parse them either.
///
/// A stream with no `result` event is a turn that did not finish: the process
/// was killed, the deadline elapsed, or `claude` died. That is an ordinary
/// outcome for a Drone and is reported as `None` rather than as a failure.
pub fn ledger(stream: &str) -> Option<Turn> {
    let mut turn: Option<Turn> = None;
    let mut rate_limit: Option<RateLimit> = None;

    for line in stream.lines() {
        let line = line.trim();
        if line.is_empty() {
            continue;
        }
        let Ok(event) = serde_json::from_str::<serde_json::Value>(line) else {
            // A partial last line is what a killed process leaves behind. The
            // turns before it are still real, so the line is skipped rather
            // than failing the read.
            continue;
        };
        match event.get("type").and_then(|t| t.as_str()) {
            Some("result") => turn = Some(read_result(&event)),
            Some("rate_limit_event") => rate_limit = read_rate_limit(&event),
            _ => {}
        }
    }

    turn.map(|mut turn| {
        // The window is reported alongside the turn even though it arrives on
        // its own event, because the caller's question is "may I spawn another
        // one", and that is one question rather than two.
        turn.rate_limit = rate_limit;
        turn
    })
}

fn read_result(event: &serde_json::Value) -> Turn {
    let usage = event.get("usage");
    let count = |key: &str| {
        usage
            .and_then(|u| u.get(key))
            .and_then(serde_json::Value::as_u64)
            .unwrap_or(0)
    };
    Turn {
        spend: Spend {
            cost_usd: event
                .get("total_cost_usd")
                .and_then(serde_json::Value::as_f64)
                .unwrap_or(0.0),
            // **Every kind of token, because every kind is billed.** Counting
            // input and output alone understates a cached turn by an order of
            // magnitude — the spike's own numbers were 4 input against 44357
            // cache reads — and a ceiling computed from the smaller number is a
            // ceiling that never stops anything.
            tokens: count("input_tokens")
                + count("output_tokens")
                + count("cache_creation_input_tokens")
                + count("cache_read_input_tokens"),
            turns: event
                .get("num_turns")
                .and_then(serde_json::Value::as_u64)
                .unwrap_or(0) as u32,
            api_ms: event
                .get("duration_api_ms")
                .and_then(serde_json::Value::as_u64)
                .unwrap_or(0),
        },
        stop_reason: event
            .get("stop_reason")
            .and_then(serde_json::Value::as_str)
            .map(str::to_string),
        is_error: event
            .get("is_error")
            .and_then(serde_json::Value::as_bool)
            .unwrap_or(false),
        result: event
            .get("result")
            .and_then(serde_json::Value::as_str)
            .map(str::to_string),
        rate_limit: None,
    }
}

fn read_rate_limit(event: &serde_json::Value) -> Option<RateLimit> {
    let info = event.get("rate_limit_info").or(Some(event))?;
    Some(RateLimit {
        status: info.get("status")?.as_str()?.to_string(),
        kind: info
            .get("rateLimitType")
            .or_else(|| info.get("rate_limit_type"))
            .and_then(serde_json::Value::as_str)
            .unwrap_or("unknown")
            .to_string(),
        resets_at: info
            .get("resetsAt")
            .or_else(|| info.get("resets_at"))
            .and_then(serde_json::Value::as_u64),
    })
}

/// The failure a Drone that could not be started reports.
///
/// **`environment`, not `tool_failed`.** `claude` missing from `PATH` is the
/// machine being incomplete rather than the repository being wrong, and the
/// correct response is the identical command after a person fixes something
/// Armada cannot (`ARCHITECTURE.md` §1.7).
pub fn not_on_path() -> ArmadaError {
    ArmadaError {
        class: ErrClass::Environment,
        r#where: CLAUDE.to_string(),
        message: "`claude` is not on PATH, so no Drone can be started".to_string(),
        next_action: Some("install Claude Code, then retry unchanged".to_string()),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    const UUID: &str = "15bfa340-33b1-4f81-bd7f-688f0f01dbb0";

    /// **The argv PHASES.md §8.5 names, exactly.** A test asserting on anything
    /// less specific than the whole vector would pass with `--session-id`
    /// missing, which is the bug that loses a Job's transcript.
    #[test]
    fn a_first_turn_assigns_the_session_id_before_anything_starts() {
        assert_eq!(
            spawn_argv(UUID, "reproduce the flake"),
            [
                "claude",
                "--session-id",
                UUID,
                "--print",
                "--output-format",
                "stream-json",
                "reproduce the flake",
            ]
        );
    }

    /// **`--resume`, never a second `--session-id`.** Minting where continuing
    /// was meant starts a Job's next turn as its first, with none of the context
    /// the answer was an answer to.
    #[test]
    fn continuing_a_job_resumes_the_session_rather_than_minting_one() {
        let argv = resume_argv(UUID, "yes, raise it to 90s");
        assert_eq!(
            argv,
            [
                "claude",
                "--resume",
                UUID,
                "--print",
                "--output-format",
                "stream-json",
                "yes, raise it to 90s",
            ]
        );
        assert!(!argv.iter().any(|a| a == "--session-id"));
    }

    /// Boarding is interactive: no `--print`, because the whole point is that
    /// you drive it.
    #[test]
    fn boarding_is_the_interactive_resume_and_carries_no_prompt() {
        assert_eq!(board_argv(UUID), ["claude", "--resume", UUID]);
    }

    /// The prompt is the last element and is never split. A task arrives as free
    /// text and a shell has already had its turn with it.
    #[test]
    fn the_prompt_is_one_argument_however_many_words_it_has() {
        let argv = spawn_argv(UUID, "add rate limiting to the API --json");
        assert_eq!(argv.last().unwrap(), "add rate limiting to the API --json");
        assert_eq!(argv.len(), 7);
    }

    /// **The measured values from the spike** (PHASES.md §9.1 F2), read back off
    /// a recorded stream. Nothing here is estimated and no test spends a token.
    const RECORDED: &str = r#"
{"type":"system","subtype":"init","session_id":"15bfa340-33b1-4f81-bd7f-688f0f01dbb0"}
{"type":"assistant","message":{"content":[{"type":"text","text":"working"}]}}
{"type":"rate_limit_event","rate_limit_info":{"status":"allowed","rateLimitType":"five_hour","resetsAt":1754748131}}
{"type":"result","subtype":"success","is_error":false,"num_turns":2,"duration_api_ms":2956,"total_cost_usd":0.1724735,"stop_reason":"end_turn","result":"done","usage":{"input_tokens":4,"output_tokens":85,"cache_creation_input_tokens":14815,"cache_read_input_tokens":44357}}
"#;

    #[test]
    fn the_turns_ledger_is_read_straight_off_the_result_event() {
        let turn = ledger(RECORDED).expect("a finished turn has a result event");
        assert_eq!(turn.spend.turns, 2);
        assert_eq!(turn.spend.api_ms, 2_956);
        assert!((turn.spend.cost_usd - 0.1724735).abs() < 1e-9);
        assert_eq!(turn.stop_reason.as_deref(), Some("end_turn"));
        assert!(!turn.is_error);
        assert_eq!(turn.result.as_deref(), Some("done"));
    }

    /// **Every kind of token, because every kind is billed.** Input plus output
    /// alone is 89 against a real 59,261 — a ceiling computed from the smaller
    /// number never stops anything.
    #[test]
    fn a_cached_turn_counts_its_cache_tokens_and_not_only_its_input() {
        let turn = ledger(RECORDED).unwrap();
        assert_eq!(turn.spend.tokens, 4 + 85 + 14_815 + 44_357);
        assert_ne!(turn.spend.tokens, 4 + 85, "the cache was not counted");
    }

    /// The window travels with the turn, because "may I spawn another one" is
    /// one question rather than two.
    #[test]
    fn the_rate_limit_window_is_reported_alongside_the_turn() {
        let limit = ledger(RECORDED).unwrap().rate_limit.expect("a window");
        assert_eq!(limit.status, "allowed");
        assert_eq!(limit.kind, "five_hour");
        assert_eq!(limit.resets_at, Some(1_754_748_131));
    }

    /// **A killed Drone is an ordinary outcome, not a parse failure.** The Job
    /// survives, the record still says what it spent up to then, and `ls`
    /// reports it — which is the whole distinction between a Job and a Drone.
    #[test]
    fn a_stream_that_never_finished_reports_no_turn_rather_than_failing() {
        assert_eq!(ledger(""), None);
        assert_eq!(
            ledger("{\"type\":\"system\",\"subtype\":\"init\"}\n{\"type\":\"assis"),
            None
        );
    }

    /// A turn that ended in an error still carries its ledger: the spend
    /// happened whether or not the work did.
    #[test]
    fn a_failed_turn_still_reports_what_it_cost() {
        let turn = ledger(
            r#"{"type":"result","is_error":true,"num_turns":1,"total_cost_usd":0.02,"usage":{"input_tokens":10,"output_tokens":2}}"#,
        )
        .unwrap();
        assert!(turn.is_error);
        assert_eq!(turn.spend.tokens, 12);
        assert!((turn.spend.cost_usd - 0.02).abs() < 1e-9);
    }

    /// The last `result` wins, so a stream carrying two turns reports the one
    /// that finished last.
    #[test]
    fn the_last_result_event_is_the_one_reported() {
        let turn = ledger(
            "{\"type\":\"result\",\"num_turns\":1}\n{\"type\":\"result\",\"num_turns\":9}\n",
        )
        .unwrap();
        assert_eq!(turn.spend.turns, 9);
    }

    /// A missing `claude` is the machine's problem and not the repository's, so
    /// the class is the one whose correct response is *fix the machine, then
    /// retry unchanged*.
    #[test]
    fn a_missing_claude_is_an_environment_failure() {
        let error = not_on_path();
        assert_eq!(error.class, ErrClass::Environment);
        assert_eq!(error.class.exit_code(), 6);
    }
}
