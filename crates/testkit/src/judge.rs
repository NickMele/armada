//! A [`ModelClient`] that renders a shell instead of a model.
//!
//! # What it is faithful about
//!
//! **The seam and the runner.** It takes a real `Ask` and answers with a real
//! `JudgeCall`, so a test drives Fleet's own process runner, Fleet's own stdin
//! write and `verification`'s own answer parser. Only the model is faked, which
//! is the one part a suite must never call: a live judgment costs money, needs
//! a network and needs a credential.
//!
//! The rendered program reads its stdin to the end before printing, the way the
//! real CLI does. A fake that exited without reading would make every call look
//! like a broken pipe.
//!
//! # Scripted by criterion, because unanimity is the thing under test
//!
//! A panel that all agrees proves nothing about a fold that takes any single
//! refusal. [`FakeJudge::answering`] scripts an answer per question, matched on
//! a fragment of the question's own text, and [`FakeJudge::saying`] gives every
//! criterion the same one.

use std::collections::BTreeMap;
use std::sync::Mutex;

use adapter_traits::{Ask, Heard, JudgeCall, ModelClient};

/// A judge that answers whatever the test wrote.
#[derive(Debug)]
pub struct FakeJudge {
    default: Option<String>,
    by_criterion: BTreeMap<String, String>,
    failing: Option<&'static str>,
    asked: Mutex<Vec<String>>,
}

impl FakeJudge {
    /// Every criterion gets this answer.
    pub fn saying(answer: &str) -> FakeJudge {
        FakeJudge {
            default: Some(answer.to_string()),
            by_criterion: BTreeMap::new(),
            failing: None,
            asked: Mutex::new(Vec::new()),
        }
    }

    /// No objection to anything. The answer a well-behaved change draws.
    pub fn with_no_objection() -> FakeJudge {
        FakeJudge::saying("verdict: met")
    }

    /// A refusal, cited the way the three named fields require.
    pub fn refusing(expected: &str, produced: &str, consequence: &str) -> FakeJudge {
        FakeJudge::saying(&format!(
            "verdict: not_met\nexpected: {expected}\nproduced: {produced}\n\
             consequence: {consequence}"
        ))
    }

    /// One answer per question, keyed by a fragment of the question's text. A
    /// question matching no fragment draws the default, and with no default
    /// draws nothing readable — which is itself a case worth scripting.
    pub fn answering(scripted: &[(&str, &str)]) -> FakeJudge {
        FakeJudge {
            default: None,
            by_criterion: scripted
                .iter()
                .map(|(fragment, answer)| ((*fragment).to_string(), (*answer).to_string()))
                .collect(),
            failing: None,
            asked: Mutex::new(Vec::new()),
        }
    }

    /// A call that cannot be made at all — the network, the quota, the timeout.
    /// **The case that must be neither a refusal nor a pass.**
    pub fn that_fails(standing_in_for: &'static str) -> FakeJudge {
        FakeJudge {
            default: None,
            by_criterion: BTreeMap::new(),
            failing: Some(standing_in_for),
            asked: Mutex::new(Vec::new()),
        }
    }

    /// Every question this judge was handed, in order. **What a test asserts
    /// the brief does not contain**, and what says the Judge never ran at all
    /// on a step that asks nothing.
    pub fn asked(&self) -> Vec<String> {
        self.asked.lock().expect("not poisoned").clone()
    }

    /// What this judge would answer a question. Matched on the question's text
    /// because a criterion id is not on the `Ask` — the model is never told
    /// one, and a citation names the criterion on Fleet's side.
    fn answer(&self, question: &str) -> Option<String> {
        self.by_criterion
            .iter()
            .find(|(fragment, _)| question.contains(fragment.as_str()))
            .map(|(_, answer)| answer.clone())
            .or_else(|| self.default.clone())
    }
}

impl ModelClient for FakeJudge {
    fn render(&self, ask: &Ask) -> JudgeCall {
        self.asked
            .lock()
            .expect("not poisoned")
            .push(ask.question().to_string());
        let script = match (self.failing, self.answer(ask.question())) {
            (Some(_), _) => String::from("cat >/dev/null; exit 3"),
            (None, Some(_)) => String::from("cat >/dev/null; printf %s \"$0\""),
            (None, None) => String::from("cat >/dev/null"),
        };
        let mut args = vec![String::from("-c"), script];
        if let Some(answer) = self.answer(ask.question()) {
            args.push(answer);
        }
        JudgeCall::rendered(ask, "/bin/sh", args)
    }

    /// The same answer, printed as a watched call prints one.
    ///
    /// **A real stream, slowly.** The script emits the three lines a caller
    /// reads progress off — the harness starting, the request reaching the
    /// vendor, the model thinking — then the answer, with a beat between each,
    /// so a test watching a call actually sees it move rather than seeing one
    /// batch arrive at the end. The beats are what make a stop reachable: a
    /// call that finished before anybody could ask it to stop proves nothing
    /// about stopping.
    ///
    /// **The lines are built here and printed there.** Assembling JSON inside a
    /// shell script means quoting an answer through two languages, and the
    /// first answer containing a quotation mark would produce a line the reader
    /// silently drops — a fake that goes quiet where the real one would not. So
    /// each line is a finished string by the time the shell sees it, and the
    /// shell only writes what it is handed.
    ///
    /// A failing judge still fails **after** the progress: a call that dies
    /// before it says anything is a different case, and
    /// [`FakeJudge::that_fails`]'s unwatched render is where that one lives.
    fn render_watched(&self, ask: &Ask) -> JudgeCall {
        self.asked
            .lock()
            .expect("not poisoned")
            .push(ask.question().to_string());
        let mut lines = vec![
            String::from(r#"{"type":"system","subtype":"init"}"#),
            String::from(r#"{"type":"system","subtype":"status","status":"requesting"}"#),
            String::from(r#"{"type":"system","subtype":"thinking_tokens","estimated_tokens":120}"#),
        ];
        if self.failing.is_none() {
            if let Some(answer) = self.answer(ask.question()) {
                lines.push(assistant(&answer));
            }
        }
        // One `printf` per line with a beat between, then the exit the fake is
        // scripted for. `$0`-style splicing is deliberately not used: every
        // line is already a literal by the time it reaches the shell.
        let mut script = String::from("cat >/dev/null; ");
        for line in &lines {
            script.push_str(&format!("printf '%s\n' {}; sleep 0.05; ", quoted(line)));
        }
        script.push_str(match self.failing {
            Some(_) => "exit 3",
            None => "true",
        });
        JudgeCall::rendered(ask, "/bin/sh", vec![String::from("-c"), script])
    }

    /// Read a line the way the real client does. **Delegated rather than
    /// re-implemented**: a fake with its own reader would let a suite pass
    /// against a format the shipped reader cannot parse, which is the one thing
    /// a fake at this seam must not do.
    fn heard(&self, line: &str) -> Heard {
        adapters::HeadlessAgent::at("/bin/false").heard(line)
    }
}

/// One `assistant` line carrying `answer`, as the real stream carries one.
///
/// **The answer is escaped rather than interpolated.** A refusal cites three
/// fields on three lines and any answer may hold a quotation mark, so a
/// `format!` that dropped the string in raw would produce a line that is not
/// JSON — and `heard` would return `Nothing` for it, which is a fake going
/// quiet exactly where the real client would not.
///
/// No JSON library: testkit depends on none, the shape is three nested objects
/// fixed at compile time, and the only variable in it is one string. What that
/// string needs is [`escaped`], which is below and is six lines.
fn assistant(answer: &str) -> String {
    format!(
        r#"{{"type":"assistant","message":{{"content":[{{"type":"text","text":"{}"}}]}}}}"#,
        escaped(answer)
    )
}

/// One JSON string body. The five escapes RFC 8259 requires of a bare string,
/// plus the `\u` form for the remaining control characters — a criterion's
/// answer is prose and a tab in it should not end the line.
fn escaped(value: &str) -> String {
    let mut out = String::with_capacity(value.len());
    for character in value.chars() {
        match character {
            '"' => out.push_str("\\\""),
            '\\' => out.push_str("\\\\"),
            '\n' => out.push_str("\\n"),
            '\r' => out.push_str("\\r"),
            '\t' => out.push_str("\\t"),
            other if (other as u32) < 0x20 => out.push_str(&format!("\\u{:04x}", other as u32)),
            other => out.push(other),
        }
    }
    out
}

/// A string the shell will hand back unchanged. Single quotes, with the one
/// character that ends them spliced.
fn quoted(value: &str) -> String {
    format!("'{}'", value.replace('\'', r"'\''"))
}

/// One refusal, as the record holds it.
///
/// **Written out once here rather than once per test file**, for
/// [`asked_for`](crate::asked_for)'s reason one level along: a `Judgment` is
/// seven fields with no `Default`, four of which are `Option`s that mean
/// different things, and a test that wants "the Judge refused c1" should not
/// have to say so in eight lines.
///
/// `brief_path` is `None`: where a brief was kept is a fact about a filesystem
/// and a fixture has none. The test that cares asserts on it directly.
pub fn refusal(
    criterion: &str,
    expected: &str,
    produced: &str,
    consequence: &str,
) -> core_model::Judgment {
    core_model::Judgment {
        criterion_id: core_model::CriterionId::new(criterion),
        verdict: core_model::JudgeVerdict::NotMet,
        expected: Some(expected.to_string()),
        produced: Some(produced.to_string()),
        consequence: Some(consequence.to_string()),
        brief_path: None,
    }
}
