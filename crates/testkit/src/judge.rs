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

use adapter_traits::{Ask, JudgeCall, ModelClient};

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
}
