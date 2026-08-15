//! Probing — **a summary of a transcript, and never a word to the Drone**
//! (PLAN.md §15.2).
//!
//! The orchestrator has to know how a Job is going, and there are two ways to
//! find out. One is to message the Drone and ask; the other is to read what it
//! has already written. **Asking costs you the thing you were measuring** — the
//! Drone stops working to answer, its context grows by the exchange, and the
//! budget pays for a turn that produced nothing. So probe never resumes,
//! interrupts, or writes to a Job: it reads the transcript off disk and hands it
//! to a second, cheap model.
//!
//! **The same model classification uses, for the same reason.** It runs whenever
//! anyone asks how the fleet is doing, so its cost is the one that compounds,
//! and summarising a few hundred lines into a paragraph is exactly its shape
//! (PHASES.md §8.5). One pinned model for both keeps "what does the cheap path
//! cost" a single answer.
//!
//! **The other half of §15.2 is why this exists at all.** If Helm read raw
//! transcripts it would fill its window in three days of work and start
//! forgetting the fleet — the one thing nothing else can do for it. A summary is
//! not a convenience here; it is the design constraint.

use crate::error::{ArmadaError, ErrClass};

/// How much transcript is worth summarising.
///
/// **The tail, not the head.** A Job's early turns are the plan and its recent
/// ones are the state; "how is it going" is a question about the second. Capped
/// rather than unbounded because the summarising call is meant to be cheap, and
/// a Job that has run for an hour has a transcript nobody wants to pay to read
/// twice.
pub const TAIL_EVENTS: usize = 40;

/// The prompt, which is the whole of the model's instruction.
///
/// **It is told what it is reading and what to leave out.** The failure mode of
/// an unconstrained summariser is a retelling — every tool call in order, at the
/// length of the thing it was meant to compress — which costs the orchestrator
/// exactly what reading the transcript would have.
pub fn prompt(task: &str, transcript: &str) -> String {
    format!(
        "You are reading a coding agent's transcript to answer one question for its \
         orchestrator: how is this going?\n\n\
         Answer in at most four sentences of plain prose. Say what it is working on now, \
         what it has finished, and anything that looks stuck or wrong. Do not list the \
         tool calls, do not quote code, and do not offer advice.\n\n\
         The task it was given: {task}\n\n\
         The transcript:\n{transcript}"
    )
}

/// The argv for the summarising call.
///
/// **Identical in shape to [`classify::argv`](super::classify::argv), and that
/// is deliberate**: one cheap, unattended, tool-less turn. A summariser is given
/// nothing to work with — `--strict-mcp-config` with no `--mcp-config` loads no
/// MCP servers at all and `--disable-slash-commands` loads no skills — because
/// it reads one string and writes one paragraph, and handing an unattended model
/// a toolbelt it does not need is capability granted for no reason.
///
/// **A probe that could call tools could resume the Job it is describing**,
/// which is the one thing this whole module exists to prevent. The least
/// privilege here is not incidental.
pub fn argv(task: &str, transcript: &str) -> Vec<String> {
    vec![
        super::drone::CLAUDE.to_string(),
        "--model".to_string(),
        super::classify::MODEL.to_string(),
        "--print".to_string(),
        "--output-format".to_string(),
        "json".to_string(),
        "--strict-mcp-config".to_string(),
        "--disable-slash-commands".to_string(),
        prompt(task, transcript),
    ]
}

/// The last [`TAIL_EVENTS`] lines of a transcript, as the model is given them.
///
/// **Lines rather than parsed events.** `stream-json` is one JSON document per
/// line and the summariser reads prose out of them perfectly well, while a
/// parser here would have to agree with [`super::drone::read`] about every event
/// shape Claude Code emits — a second reader of the same format, drifting.
pub fn tail(stream: &str) -> (String, usize) {
    let lines: Vec<&str> = stream
        .lines()
        .map(str::trim)
        .filter(|line| !line.is_empty())
        .collect();
    let events = lines.len();
    let from = events.saturating_sub(TAIL_EVENTS);
    (lines[from..].join("\n"), events)
}

/// Read the model's answer.
///
/// **Two layers, because Claude Code wraps the model's text**, exactly as
/// classification's does: `--output-format json` answers with an envelope whose
/// `result` is what the model said.
pub fn parse(stdout: &str) -> Result<String, ArmadaError> {
    let summary = match serde_json::from_str::<serde_json::Value>(stdout.trim()) {
        Ok(envelope) => envelope
            .get("result")
            .and_then(serde_json::Value::as_str)
            .map(str::to_string)
            .unwrap_or_else(|| stdout.to_string()),
        Err(_) => stdout.to_string(),
    };
    let summary = summary.trim().to_string();
    if summary.is_empty() {
        return Err(ArmadaError {
            class: ErrClass::ToolFailed,
            r#where: "probe".to_string(),
            message: "the summarising call answered nothing".to_string(),
            next_action: Some(
                "`armada fleet board <job>` reads the transcript yourself".to_string(),
            ),
        });
    }
    Ok(summary)
}

/// What a Drone that has written nothing is reported as.
///
/// **A sentence, not an error.** An empty transcript is the ordinary state a
/// moment after `spawn` returns, and a probe that failed on it would make the
/// first thing anyone tries look broken.
pub const NOTHING_YET: &str = "This Job has not written anything yet.";

#[cfg(test)]
mod tests {
    use super::*;

    /// **The one property the whole module is for.** Asserted on the argv,
    /// because that is what `execve` receives: nothing here resumes a session,
    /// and nothing here can.
    #[test]
    fn the_summarising_call_cannot_reach_the_job_it_describes() {
        let argv = argv("fix the flake", "{\"type\":\"assistant\"}");
        assert!(
            !argv.iter().any(|a| a == "--resume" || a == "--session-id"),
            "a probe that can resume is not a probe: {argv:?}"
        );
        assert!(argv.contains(&"--strict-mcp-config".to_string()));
        assert!(argv.contains(&"--disable-slash-commands".to_string()));
        assert!(argv.contains(&"--print".to_string()));
    }

    /// One pinned cheap model for both cheap calls, so "what does this cost" has
    /// one answer.
    #[test]
    fn it_runs_on_the_same_model_classification_does() {
        let argv = argv("t", "s");
        let model = argv.iter().position(|a| a == "--model").expect("--model");
        assert_eq!(argv[model + 1], super::super::classify::MODEL);
    }

    #[test]
    fn the_tail_is_the_recent_end_and_says_how_much_there_was() {
        let stream: String = (0..TAIL_EVENTS + 10)
            .map(|n| format!("line {n}\n"))
            .collect();
        let (tail, events) = tail(&stream);
        assert_eq!(events, TAIL_EVENTS + 10);
        assert_eq!(tail.lines().count(), TAIL_EVENTS);
        assert!(tail.starts_with("line 10"));
        assert!(tail.ends_with(&format!("line {}", TAIL_EVENTS + 9)));
    }

    #[test]
    fn an_empty_transcript_reads_as_nothing_rather_than_as_a_failure() {
        let (tail, events) = tail("\n  \n");
        assert_eq!(events, 0);
        assert!(tail.is_empty());
    }

    /// The wrapper Claude Code puts around what the model said.
    #[test]
    fn the_answer_is_read_through_both_layers() {
        let wrapped = r#"{"type":"result","result":"It is running the tests."}"#;
        assert_eq!(parse(wrapped).unwrap(), "It is running the tests.");
        // A bare answer, for a model that was not wrapped.
        assert_eq!(parse("  It is running.  ").unwrap(), "It is running.");
    }

    #[test]
    fn an_empty_answer_is_refused_with_somewhere_to_go() {
        let error = parse(r#"{"type":"result","result":"   "}"#).unwrap_err();
        assert_eq!(error.class, ErrClass::ToolFailed);
        assert!(error.next_action.is_some());
    }
}
