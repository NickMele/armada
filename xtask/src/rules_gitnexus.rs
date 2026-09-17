//! No GitNexus skill tells an agent to run a bare `analyze`.
//!
//! A bare `gitnexus analyze` appends a block to `AGENTS.md` that takes
//! `CLAUDE.md` past its fifty-line refusal, and overwrites the committed skills
//! with whatever version ran. `pnpm gitnexus:index` passes the flags that stop
//! both, and #1251 pointed every skill at it.
//!
//! **An upgrade undoes that silently.** Refreshing the skills from upstream puts
//! the bare command back in every stale-index line, and the next agent that
//! follows one fails the gate on a file it never meant to touch. #1253.
//!
//! **Only code is read** — inline spans and fenced lines — because that is what
//! an agent copies. A span counts as an `analyze` invocation when `gitnexus` or
//! `run.cjs` comes before the word, and it is let through when it carries a flag
//! that skips the injection.

use std::fs;
use std::path::Path;

use crate::Report;

const SKILLS: &str = ".claude/skills";
const PREFIX: &str = "gitnexus-";
const FIX: &str = "pnpm gitnexus:index";

/// Flags under which `analyze` leaves `AGENTS.md` alone. `--watch` refreshes
/// the graph only, by upstream's own account.
const SAFE: &[&str] = &["--skip-agents-md", "--index-only", "--watch"];

/// Rule: every `analyze` a GitNexus skill names skips the `AGENTS.md` block.
///
/// **No skills at all fails**, as every rule here does when its subject is
/// absent. Removing GitNexus removes this rule in the same commit.
pub fn no_skill_runs_a_bare_analyze(root: &Path) -> Report {
    let mut report = Report::new("no GitNexus skill runs a bare analyze");
    let Ok(entries) = fs::read_dir(root.join(SKILLS)) else {
        report.fail(format!("{SKILLS}/ — where the GitNexus skills live"));
        return report;
    };
    let mut skills: Vec<String> = entries
        .filter_map(|e| e.ok())
        .filter_map(|e| e.file_name().into_string().ok())
        .filter(|name| name.starts_with(PREFIX))
        .collect();
    skills.sort();
    if skills.is_empty() {
        report.fail(format!(
            "{SKILLS}/{PREFIX}*/SKILL.md — the skills an agent reads before using the code graph"
        ));
    }
    for skill in skills {
        let rel = format!("{SKILLS}/{skill}/SKILL.md");
        match fs::read_to_string(root.join(&rel)) {
            Ok(text) => check(&rel, &text, &mut report),
            Err(_) => report.fail(format!("{rel} — unreadable")),
        }
    }
    report
}

/// Refuses every bare `analyze` in one skill's text.
fn check(rel: &str, text: &str, report: &mut Report) {
    let mut fenced = false;
    for (index, line) in text.lines().enumerate() {
        if line.trim_start().starts_with("```") {
            fenced = !fenced;
            continue;
        }
        let spans = if fenced {
            vec![line]
        } else {
            inline_spans(line)
        };
        for span in spans {
            if is_bare_analyze(span) {
                report.fail(format!(
                    "{rel}:{} — `{}` writes a block into AGENTS.md; name `{FIX}`",
                    index + 1,
                    span.trim()
                ));
            }
        }
    }
}

/// The text between each pair of backticks on a line.
fn inline_spans(line: &str) -> Vec<&str> {
    line.split('`').skip(1).step_by(2).collect()
}

fn is_bare_analyze(span: &str) -> bool {
    let words: Vec<&str> = span.split_whitespace().collect();
    let Some(at) = words.iter().position(|w| *w == "analyze") else {
        return false;
    };
    let invoked = words[..at]
        .iter()
        .any(|w| w.contains("gitnexus") || w.ends_with("run.cjs"));
    invoked && !words.iter().any(|w| SAFE.contains(w))
}

#[cfg(test)]
mod tests;
