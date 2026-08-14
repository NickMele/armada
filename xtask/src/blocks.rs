//! Parse every fenced code block with a real parser.
//!
//! The point is not that the examples look right. It is that **Armada's own YAML
//! parser accepts them** — a doc example that `serde_yaml_ng` rejects is a
//! defect no amount of reading catches, and one that a third-party opinion
//! would answer differently is the wrong question.
//!
//! Phase 1 chose that parser (`PLAN.md` §4.1.1, decision 5) and it is the one
//! this check already used. The version is pinned once, in the workspace's
//! `[workspace.dependencies]`, and `crates/core` takes it from there — which is
//! what keeps "the lint parses examples with the parser that reads `armada.yml`"
//! true, rather than a comment asking two manifests to agree.
//!
//! Known limitation, stated rather than hidden: this checks YAML 1.2 only. The
//! Python predecessor also ran a 1.1 parser, which is how it caught
//! `setup: [bundle install, rails db:create]` — legal 1.2, a flow-context
//! colon error under 1.1. Rust has no maintained 1.1 parser. The trade is
//! accepted because the fixtures are validated by the parser that runs.

use crate::docs::{blocks, Doc, Finding};
use serde::Deserialize as _;
use std::io::Write;
use std::process::{Command, Stdio};

pub fn check(docs: &[Doc]) -> Vec<Finding> {
    let mut findings = Vec::new();

    for doc in docs {
        for b in blocks(&doc.text) {
            let err = match b.lang.as_str() {
                "yaml" | "yml" => check_yaml(&b.body),
                "json" => check_json(&b.body),
                "sh" | "bash" | "shell" => check_shell(&b.body),
                _ => None,
            };
            if let Some(message) = err {
                findings.push(Finding {
                    file: doc.rel.to_string(),
                    line: b.start,
                    message: format!("[{}] {message}", b.lang),
                });
            }
        }
    }
    findings
}

fn check_yaml(body: &str) -> Option<String> {
    // Multi-document is legal and the corpus uses it, so deserialize the
    // stream rather than a single value.
    for doc in serde_yaml_ng::Deserializer::from_str(body) {
        if let Err(e) = serde_yaml_ng::Value::deserialize(doc) {
            return Some(one_line(&e.to_string()));
        }
    }
    None
}

fn check_json(body: &str) -> Option<String> {
    // A block holding several objects back to back, or one fragment, is
    // deliberate authoring in this corpus — §3.1 shows alternative payloads in
    // a single fence. Accept a stream; reject only what is unparseable.
    let mut stream = serde_json::Deserializer::from_str(body).into_iter::<serde_json::Value>();
    for item in stream.by_ref() {
        if let Err(e) = item {
            return Some(one_line(&e.to_string()));
        }
    }
    None
}

fn check_shell(body: &str) -> Option<String> {
    // `bash -n` parses without executing. Placeholder syntax like `<name>` is
    // a redirect from a file that does not exist, which parses fine — so this
    // catches real syntax errors and not illustration.
    let mut child = Command::new("bash")
        .arg("-n")
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .spawn()
        .ok()?;
    child.stdin.as_mut()?.write_all(body.as_bytes()).ok()?;
    let out = child.wait_with_output().ok()?;
    if out.status.success() {
        return None;
    }
    Some(one_line(&String::from_utf8_lossy(&out.stderr)))
}

fn one_line(s: &str) -> String {
    let joined = s.trim().replace('\n', " | ");
    if joined.chars().count() <= 220 {
        return joined;
    }
    joined.chars().take(217).collect::<String>() + "..."
}
