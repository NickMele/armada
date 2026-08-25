//! The half of the capability check that needs a network, kept out of the gate.
//!
//! A capability is two things bound together: a GitHub issue that tracks it,
//! and `docs/capabilities/<slug>.md` that holds the prose an issue body buries
//! the moment it closes. Neither half is allowed to exist alone.
//!
//! **The gate cannot check that.** `xtask` has no dependencies and must run on
//! a checkout with nothing built — a gate that fails on a plane is a gate people
//! learn to ignore. So the binding is checked from both ends by two different
//! things:
//!
//! | Check | Runs | Sees |
//! |---|---|---|
//! | `verify-foundations` | always, offline | the file: frontmatter, slug, index |
//! | `verify-roadmap` | CI and on demand | the issue: that it exists, and that every capability issue has a file |
//!
//! Splitting it is the honest shape rather than a compromise. The offline half
//! catches the mistake somebody makes while writing — a file with no issue
//! number, a slug that disagrees with its filename — and catches it before the
//! commit. The online half catches the mistake nobody makes on purpose: an
//! issue deleted, or a capability tracked in GitHub that never got its prose.

use std::collections::BTreeMap;
use std::process::Command;

use crate::capabilities::{self, Capability};
use crate::repo_root;

/// What `gh` says about one issue.
struct Tracked {
    number: u64,
    title: String,
    milestone: Option<String>,
    is_capability: bool,
}

/// Read every issue labelled `capability` from GitHub.
fn tracked() -> Result<Vec<Tracked>, String> {
    let run = Command::new("gh")
        .args([
            "issue", "list",
            "--label", "capability",
            "--state", "all",
            "--limit", "500",
            "--json", "number,title,labels,milestone",
        ])
        .current_dir(repo_root())
        .output()
        .map_err(|_| "`gh` is not on PATH — verify-roadmap needs it".to_string())?;

    if !run.status.success() {
        return Err(format!(
            "`gh issue list` failed: {}",
            String::from_utf8_lossy(&run.stderr).trim()
        ));
    }

    // No serde here for the same reason there is no serde anywhere in xtask.
    // The shape is three known keys from one trusted command, so the scan is
    // small enough to read; anything more structured belongs behind the `store`
    // and `ipc` boundary where untyped JSON is actually permitted.
    let text = String::from_utf8_lossy(&run.stdout).to_string();
    let mut out = Vec::new();
    for chunk in text.split("{\"labels\"").skip(1) {
        let number = field_number(chunk, "\"number\":");
        let title = field_string(chunk, "\"title\":\"");
        if let Some(number) = number {
            out.push(Tracked {
                number,
                title: title.unwrap_or_default(),
                milestone: field_string(chunk, "\"milestone\":{\"description\":")
                    .and(field_string(chunk, "\"title\":\"")),
                is_capability: chunk.contains("\"name\":\"capability\""),
            });
        }
    }
    Ok(out)
}

fn field_number(chunk: &str, key: &str) -> Option<u64> {
    let at = chunk.find(key)? + key.len();
    let digits: String = chunk[at..].chars().take_while(|c| c.is_ascii_digit()).collect();
    digits.parse().ok()
}

fn field_string(chunk: &str, key: &str) -> Option<String> {
    let at = chunk.find(key)? + key.len();
    let end = chunk[at..].find('"')?;
    Some(chunk[at..at + end].to_string())
}

/// Compare the two halves and print what disagrees.
pub fn verify() -> Result<Vec<String>, String> {
    let root = repo_root();
    let files: Vec<Capability> = capabilities::read(&root)?;
    let issues = tracked()?;

    let by_number: BTreeMap<u64, &Tracked> = issues.iter().map(|t| (t.number, t)).collect();
    let mut problems = Vec::new();

    for cap in &files {
        match by_number.get(&cap.issue) {
            None => problems.push(format!(
                "docs/capabilities/{}.md names issue #{}, which is not a capability issue \
                 in this repo — it was deleted, relabelled, or the number is wrong",
                cap.slug, cap.issue
            )),
            Some(t) if !t.is_capability => problems.push(format!(
                "docs/capabilities/{}.md names issue #{} (\"{}\"), which has lost its \
                 `capability` label",
                cap.slug, cap.issue, t.title
            )),
            Some(t) => {
                // The file says which milestone claims the capability; the issue
                // says the same thing to GitHub. Two places, so they can differ
                // — which is the whole reason this is checked rather than
                // trusted.
                if let (Some(want), Some(have)) = (&cap.milestone, &t.milestone) {
                    if want != have {
                        problems.push(format!(
                            "docs/capabilities/{}.md says milestone \"{want}\" but issue #{} \
                             is on \"{have}\"",
                            cap.slug, cap.issue
                        ));
                    }
                }
            }
        }
    }

    for t in &issues {
        if !files.iter().any(|c| c.issue == t.number) {
            problems.push(format!(
                "issue #{} (\"{}\") is a capability with no docs/capabilities file — \
                 the prose an issue body buries when it closes has nowhere to live",
                t.number, t.title
            ));
        }
    }

    Ok(problems)
}
