//! Classification — **one cheap model call, and the confidence is shown**
//! (PLAN.md §14.2).
//!
//! Task text in, a workflow name out, with an explicit override and the
//! confidence surfaced so a guess is visible as a guess. It lives in Fleet
//! because it is needed the moment a Job can be spawned — long before Helm
//! exists — and putting it in the orchestrator would make every other caller
//! worse for no gain (`ARCHITECTURE.md` §1.9's corollary: ask which is the
//! lowest module that could own a thing, and put it there).
//!
//! **Haiku 4.5, decided rather than tuned** (PHASES.md §8.5). It runs on every
//! spawn, so its cost is the one that compounds, and picking one of four labels
//! with a confidence is exactly its shape. Keyword rules are free and wrong
//! often enough that the override becomes the normal path, which would make the
//! feature worse than not having it.

use super::workflow::STARTERS;
use crate::error::{ArmadaError, ErrClass};
use serde::Serialize;

/// The model, pinned (PHASES.md §8.5).
pub const MODEL: &str = "claude-haiku-4-5-20251001";

/// Below this, a spawn is confirmed rather than taken (PLAN.md §15.4).
///
/// **A threshold rather than a judgement**, because the failure it prevents is
/// specific: a misclassification wastes a whole budget, and the two cases where
/// an unconfirmed spawn is expensive — low confidence, and a workflow that ends
/// at you anyway — are both checkable.
pub const CONFIDENT: f64 = 0.75;

/// Where a Job's workflow came from.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
#[serde(rename_all = "lowercase")]
pub enum Source {
    /// A person named it with `--workflow`.
    Override,
    /// The model picked it.
    Model,
}

/// What classification decided.
#[derive(Debug, Clone, PartialEq, Serialize)]
pub struct Classification {
    /// The workflow name.
    pub workflow: String,
    /// How sure, or `None` for an override — **a certainty nobody measured must
    /// not be reported as 1.0**, which is the difference between "you said so"
    /// and "the model was certain".
    #[serde(skip_serializing_if = "Option::is_none")]
    pub confidence: Option<f64>,
    /// Who decided.
    pub source: Source,
}

impl Classification {
    /// A workflow a person named.
    pub fn overridden(workflow: &str) -> Classification {
        Classification {
            workflow: workflow.to_string(),
            confidence: None,
            source: Source::Override,
        }
    }

    /// Whether this is confident enough to spawn without asking.
    ///
    /// An override always is: you already answered the question.
    pub fn is_confident(&self) -> bool {
        self.confidence.is_none_or(|c| c >= CONFIDENT)
    }
}

/// The prompt, which is the whole of the model's instruction.
///
/// **Short, and it names each label once.** A prompt that described the
/// workflows at length would drift from `templates/guild/workflows/` the first
/// time one was edited; the labels are the contract and the descriptions are the
/// guild's. One clause each is what makes the four distinguishable, and nothing
/// beyond that earns its tokens on a call that runs on every spawn.
pub fn prompt(task: &str) -> String {
    format!(
        "Classify this software task as exactly one of:\n\
         design = deciding an approach, no code.\n\
         plan = writing down how, before building.\n\
         feature = building something new.\n\
         bug = something is broken and must be reproduced first.\n\n\
         Answer with one line of JSON and nothing else: \
         {{\"workflow\": \"<one of the four>\", \"confidence\": <0.0-1.0>}}\n\n\
         Task: {task}"
    )
}

/// The argv for the classifying call.
///
/// `--output-format json` rather than `stream-json`: there is one answer and no
/// stream worth watching, and the ceiling this call runs under is that it is one
/// turn of the cheapest model.
///
/// **A classifier is given nothing to work with, deliberately.** It picks one of
/// four labels from one line of text; it has no use for the caller's MCP
/// servers or their skills, and handing an unattended model a toolbelt it does
/// not need is capability granted for no reason. `--strict-mcp-config` with no
/// `--mcp-config` loads no MCP servers at all, and `--disable-slash-commands`
/// loads no skills.
///
/// **Not loading them is also most of what this call spends its time on** — a
/// measured 20.6s for a one-line task, on the path of every spawn. That is the
/// expected effect rather than a measured one: verifying it needs a real call,
/// and no test in this repository may make one.
pub fn argv(task: &str) -> Vec<String> {
    vec![
        super::drone::CLAUDE.to_string(),
        "--model".to_string(),
        MODEL.to_string(),
        "--print".to_string(),
        "--output-format".to_string(),
        "json".to_string(),
        // Least privilege, and it happens to be the cheap path too.
        "--strict-mcp-config".to_string(),
        "--disable-slash-commands".to_string(),
        prompt(task),
    ]
}

/// Read the model's answer.
///
/// **Two layers, because Claude Code wraps the model's text.** `--output-format
/// json` answers with an envelope whose `result` is what the model said, so the
/// JSON object being looked for is inside a string inside a document. A parser
/// that only handled one layer would work in a unit test written against the
/// inner form and fail against every real call.
pub fn parse(stdout: &str) -> Result<Classification, ArmadaError> {
    let inner = match serde_json::from_str::<serde_json::Value>(stdout.trim()) {
        Ok(envelope) => envelope
            .get("result")
            .and_then(serde_json::Value::as_str)
            .map(str::to_string)
            .unwrap_or_else(|| stdout.to_string()),
        Err(_) => stdout.to_string(),
    };

    let object = first_object(&inner).ok_or_else(|| refuse("the answer carried no JSON object"))?;
    let value: serde_json::Value = serde_json::from_str(&object)
        .map_err(|e| refuse(&format!("the answer did not parse: {e}")))?;

    let workflow = value
        .get("workflow")
        .and_then(serde_json::Value::as_str)
        .ok_or_else(|| refuse("the answer named no workflow"))?
        .trim()
        .to_ascii_lowercase();

    // **A label outside the four is refused rather than corrected.** Guessing
    // which of them a hallucinated word meant is how a Job runs the wrong
    // workflow with a confidence attached to a decision nobody made.
    if !STARTERS.contains(&workflow.as_str()) {
        return Err(refuse(&format!("`{workflow}` is not one of the four")));
    }

    Ok(Classification {
        workflow,
        // A missing confidence is reported as zero rather than as certainty:
        // this whole module exists so that a guess is visible as one.
        confidence: Some(
            value
                .get("confidence")
                .and_then(serde_json::Value::as_f64)
                .unwrap_or(0.0)
                .clamp(0.0, 1.0),
        ),
        source: Source::Model,
    })
}

/// The first balanced `{…}` in a string, so a model that added a sentence either
/// side of its JSON is still read rather than refused.
fn first_object(text: &str) -> Option<String> {
    let start = text.find('{')?;
    let mut depth = 0usize;
    for (offset, c) in text[start..].char_indices() {
        match c {
            '{' => depth += 1,
            '}' => {
                depth -= 1;
                if depth == 0 {
                    return Some(text[start..start + offset + c.len_utf8()].to_string());
                }
            }
            _ => {}
        }
    }
    None
}

/// Why a classification could not be read.
///
/// **`tool_failed`: the model ran and answered something Fleet cannot use.**
/// That is a real result rather than Armada's fault, and the caller's correct
/// response is to name the workflow with `--workflow` — which is exactly what
/// the override is for.
fn refuse(message: &str) -> ArmadaError {
    ArmadaError {
        class: ErrClass::ToolFailed,
        r#where: "classify".to_string(),
        message: format!("classification failed: {message}"),
        next_action: Some(format!(
            "name it yourself: --workflow {}",
            STARTERS.join("|")
        )),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// The pinned model, in the argv, on every spawn. It is the cost that
    /// compounds, so the one that is asserted.
    #[test]
    fn the_classifying_call_names_haiku_and_asks_for_one_json_answer() {
        let argv = argv("add rate limiting to the API");
        assert_eq!(argv[0], "claude");
        assert_eq!(argv[1], "--model");
        assert_eq!(argv[2], "claude-haiku-4-5-20251001");
        assert_eq!(&argv[3..6], ["--print", "--output-format", "json"]);
        assert!(argv
            .last()
            .unwrap()
            .contains("add rate limiting to the API"));
    }

    /// **A classifier is handed no capability it does not need.** It picks one
    /// of four labels from one line of text; an unattended model given the
    /// caller's MCP servers and skills has been granted a toolbelt for no
    /// reason, and paid for loading it on every spawn.
    #[test]
    fn the_classifier_is_given_neither_mcp_servers_nor_skills() {
        let argv = argv("anything");
        for denied in ["--strict-mcp-config", "--disable-slash-commands"] {
            assert!(
                argv.iter().any(|word| word == denied),
                "{argv:?} does not carry {denied}"
            );
        }
        // And no `--mcp-config`, which is what makes `--strict-mcp-config` mean
        // *none* rather than *only these*.
        assert!(!argv.iter().any(|word| word == "--mcp-config"));
    }

    #[test]
    fn the_prompt_names_all_four_labels_exactly_once() {
        let prompt = prompt("anything");
        for label in STARTERS {
            let named = prompt.matches(label).count();
            assert_eq!(
                named, 1,
                "`{label}` appears {named} times; this prompt runs on every spawn"
            );
        }
    }

    /// **Two layers**: Claude Code's envelope, then the model's own line.
    #[test]
    fn an_answer_wrapped_in_claude_codes_envelope_is_read_through_it() {
        let classification = parse(
            r#"{"type":"result","subtype":"success","result":"{\"workflow\": \"feature\", \"confidence\": 0.91}"}"#,
        )
        .expect("the wrapped answer reads");
        assert_eq!(classification.workflow, "feature");
        assert_eq!(classification.confidence, Some(0.91));
        assert_eq!(classification.source, Source::Model);
    }

    /// A model that said something either side of its JSON is read rather than
    /// refused: the answer is there and refusing it would spend the call twice.
    #[test]
    fn an_answer_with_prose_around_it_is_still_read() {
        let classification =
            parse("Here you go:\n{\"workflow\":\"bug\",\"confidence\":0.8}\nHope that helps")
                .unwrap();
        assert_eq!(classification.workflow, "bug");
    }

    /// **A label outside the four is refused rather than corrected.** Guessing
    /// which one a hallucinated word meant runs the wrong workflow under a
    /// confidence nobody produced.
    #[test]
    fn a_workflow_that_is_not_one_of_the_four_is_refused() {
        let error = parse(r#"{"workflow":"refactor","confidence":0.99}"#).unwrap_err();
        assert_eq!(error.class, ErrClass::ToolFailed);
        assert!(error.message.contains("refactor"));
        assert!(error.next_action.unwrap().contains("--workflow"));
    }

    #[test]
    fn an_unreadable_answer_is_refused_and_names_the_override() {
        for bad in ["", "no idea", r#"{"confidence":0.9}"#] {
            let error = parse(bad).unwrap_err();
            assert!(
                error.next_action.unwrap().contains("--workflow"),
                "`{bad}` did not point at the override"
            );
        }
    }

    /// **A guess must be visible as a guess.** A model that answered without a
    /// confidence gets zero, not one — the field exists precisely so that
    /// "certain" and "did not say" are different rows on the screen.
    #[test]
    fn an_answer_with_no_confidence_is_not_treated_as_certain() {
        let classification = parse(r#"{"workflow":"design"}"#).unwrap();
        assert_eq!(classification.confidence, Some(0.0));
        assert!(!classification.is_confident());
    }

    #[test]
    fn a_confidence_outside_the_range_is_clamped_rather_than_believed() {
        assert_eq!(
            parse(r#"{"workflow":"plan","confidence":4.2}"#)
                .unwrap()
                .confidence,
            Some(1.0)
        );
        assert_eq!(
            parse(r#"{"workflow":"plan","confidence":-1}"#)
                .unwrap()
                .confidence,
            Some(0.0)
        );
    }

    /// **An override carries no confidence.** Reporting `1.0` for a workflow a
    /// person typed would make "you said so" indistinguishable from "the model
    /// was certain", and only one of those is evidence.
    #[test]
    fn an_override_reports_no_confidence_and_is_always_confident_enough() {
        let overridden = Classification::overridden("bug");
        assert_eq!(overridden.confidence, None);
        assert_eq!(overridden.source, Source::Override);
        assert!(overridden.is_confident());
        let json = serde_json::to_string(&overridden).unwrap();
        assert!(!json.contains("confidence"), "{json}");
    }

    #[test]
    fn the_threshold_is_the_line_between_spawning_and_confirming() {
        let at = |c: f64| Classification {
            workflow: "feature".to_string(),
            confidence: Some(c),
            source: Source::Model,
        };
        assert!(at(CONFIDENT).is_confident());
        assert!(!at(CONFIDENT - 0.01).is_confident());
    }
}
