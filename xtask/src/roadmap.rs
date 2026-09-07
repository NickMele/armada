//! The half of the capability check that needs a network, kept out of the gate.
//!
//! A capability is a GitHub issue, and `docs/capabilities/<slug>.md` when it has
//! prose worth keeping. A file may not name an issue that is not there; an issue
//! with no file is normal, and the comment on that decision below says why.
//! **The gate cannot check the first of those**:
//! `xtask` has no dependencies and must run on a checkout with nothing built —
//! a gate that fails on a plane is a gate people learn to ignore. So the
//! binding is checked from both ends:
//!
//! | Check | Runs | Sees |
//! |---|---|---|
//! | `verify-foundations` | always, offline | the file: frontmatter, slug, index |
//! | `verify-roadmap` | CI and on demand | the issue named by a file, and what a capability's steps add up to |
//!
//! The offline half catches the mistake made while writing — no issue number, a
//! slug disagreeing with its filename — before the commit. The online half
//! catches the one nobody makes on purpose: an issue deleted, relabelled or
//! moved to another milestone under a file still claiming the old one.
//!
//! It also does the arithmetic a checkbox would only display: a capability
//! closed while its steps are open is a failure rather than a rendering.

use std::collections::BTreeMap;
use std::process::Command;

use crate::capabilities::{self, Capability};
use crate::repo_root;

/// What `gh` says about one issue.
struct Tracked {
    number: u64,
    title: String,
    milestone: Option<String>,
    open: bool,
    /// On a capability, the steps it declares under `**Steps**`. Empty on a step.
    refs: Vec<u64>,
}

/// Ask `gh` for one label's issues as tab-separated lines.
///
/// `--jq ... @tsv` rather than parsing JSON here: an issue body is arbitrary
/// text written by whoever filed it, and a hand-rolled scanner over that is a
/// bug waiting for the first body containing a brace. `@tsv` escapes the
/// separator, so the split is safe no matter what the prose does.
fn issues(label: &str) -> Result<Vec<Tracked>, String> {
    let run = Command::new("gh")
        .args([
            "issue", "list",
            "--label", label,
            "--state", "all",
            "--limit", "500",
            "--json", "number,title,state,milestone,body",
            "--jq",
            r#".[] | [(.number|tostring), .state, (.milestone.title // ""), .title, ((.body // "") | gsub("[\r\n]"; "\u0001"))] | @tsv"#,
        ])
        .current_dir(repo_root())
        .output()
        .map_err(|_| "`gh` is not on PATH — verify-roadmap needs it".to_string())?;

    if !run.status.success() {
        return Err(format!(
            "`gh issue list --label {label}` failed: {}",
            String::from_utf8_lossy(&run.stderr).trim()
        ));
    }

    let text = String::from_utf8_lossy(&run.stdout).to_string();
    let mut out = Vec::new();
    for line in text.lines() {
        let mut f = line.split('\t');
        let (Some(number), Some(state), Some(milestone), Some(title)) =
            (f.next(), f.next(), f.next(), f.next())
        else {
            continue;
        };
        let Ok(number) = number.parse::<u64>() else {
            continue;
        };
        out.push(Tracked {
            number,
            title: title.to_string(),
            milestone: Some(milestone.to_string()).filter(|m| !m.is_empty()),
            open: state.eq_ignore_ascii_case("OPEN"),
            refs: step_refs(f.next().unwrap_or_default()),
        });
    }
    Ok(out)
}

/// A body's newlines, carried through `@tsv` as a character it cannot contain.
///
/// `@tsv` escapes a real newline, which would make the body unreadable as one
/// field. A control character survives the round trip and lets the section
/// structure below be read rather than guessed at from a flattened line.
const BODY_NEWLINE: char = '\u{1}';

/// The steps a capability declares, in order.
///
/// **Not every `#N` in a body is a step.** A capability cites other issues for
/// reasons that are not ownership: #384 names a Pilot step to say which worktree
/// it must never reclaim, #215 quotes *"#140 depends on #137"* as an example of
/// prose. Counting those made two closed Recovery capabilities read as owing
/// work Recovery does not own, and no rule about milestones separates the two —
/// #100 is a Trust capability and three of its four steps are Board's.
///
/// So ownership is declared rather than inferred: the references under a
/// `**Steps**` heading are the steps, and a capability with no such heading
/// declares none and reports no arithmetic. That is the honest answer — a count
/// assembled from whatever the prose happened to link was never a claim anybody
/// made.
fn step_refs(body: &str) -> Vec<u64> {
    let mut out: Vec<u64> = Vec::new();
    let mut inside = false;
    for line in body.split(BODY_NEWLINE) {
        let line = line.trim();
        if line == "**Steps**" || line == "## Steps" {
            inside = true;
            continue;
        }
        if !inside {
            continue;
        }
        // The section runs to the next heading, or to the end of the body.
        if line.starts_with("## ") {
            break;
        }
        for n in issue_refs(line) {
            if !out.contains(&n) {
                out.push(n);
            }
        }
    }
    out
}

/// The `#N` references in a body, deduplicated and in order.
fn issue_refs(body: &str) -> Vec<u64> {
    let mut out: Vec<u64> = Vec::new();
    for chunk in body.split('#').skip(1) {
        let digits: String = chunk.chars().take_while(|c| c.is_ascii_digit()).collect();
        if digits.is_empty() {
            continue;
        }
        // `#161C23` is a colour, not an issue. A reference ends at the number.
        if chunk[digits.len()..].starts_with(|c: char| c.is_ascii_alphanumeric()) {
            continue;
        }
        if let Ok(n) = digits.parse() {
            if !out.contains(&n) {
                out.push(n);
            }
        }
    }
    out
}

/// Compare the two halves and print what disagrees.
/// What a run found: the problems, and the progress it computed on the way.
pub struct Outcome {
    pub problems: Vec<String>,
    pub progress: Vec<(u64, String, usize, usize)>,
}

pub fn verify() -> Result<Outcome, String> {
    let root = repo_root();
    let files: Vec<Capability> = capabilities::read(&root)?;
    let caps = issues("capability")?;
    let steps = issues("step")?;

    let by_number: BTreeMap<u64, &Tracked> = caps.iter().map(|t| (t.number, t)).collect();
    let mut problems = Vec::new();

    for cap in &files {
        match by_number.get(&cap.issue) {
            None => problems.push(format!(
                "docs/capabilities/{}.md names issue #{}, which is not a capability issue \
                 in this repo — it was deleted, relabelled, or the number is wrong",
                cap.slug, cap.issue
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

    // Capability -> steps, computed rather than displayed.
    //
    // A step satisfies as many capabilities as it genuinely serves — M1 step 9
    // serves five — so a sub-issue link, which has one parent, would have to
    // pick one and demote four. The capability issue lists its steps under
    // `**Steps**` instead (see `step_refs`), and the arithmetic lives here where
    // it can fail rather than in a checkbox that only renders.
    let step_open: BTreeMap<u64, bool> = steps.iter().map(|s| (s.number, s.open)).collect();
    for cap in &caps {
        let mine: Vec<u64> = cap
            .refs
            .iter()
            .copied()
            .filter(|n| step_open.contains_key(n))
            .collect();
        if mine.is_empty() {
            continue;
        }
        let left = mine.iter().filter(|n| step_open[n]).count();
        if !cap.open && left > 0 {
            problems.push(format!(
                "issue #{} (\"{}\") is closed with {left} of its {} steps still open — \
                 a capability is not real until the work that makes it real has landed",
                cap.number,
                cap.title,
                mine.len()
            ));
        }
    }

    // Deliberately NOT the other direction. A capability issue with no
    // docs/capabilities file is fine and is the normal case: the Notion pages
    // these came from are properties only, with no body at all, so a file today
    // would be frontmatter and one sentence. Files arrive when a capability
    // acquires prose worth keeping, and the check above binds them from that
    // moment. Requiring one up front would have produced fifty-one pieces of
    // scaffolding that read as done.

    let mut progress: Vec<(u64, String, usize, usize)> = caps
        .iter()
        .filter_map(|cap| {
            let mine: Vec<u64> = cap
                .refs
                .iter()
                .copied()
                .filter(|n| step_open.contains_key(n))
                .collect();
            if mine.is_empty() {
                return None;
            }
            let done = mine.iter().filter(|n| !step_open[n]).count();
            Some((cap.number, cap.title.clone(), done, mine.len()))
        })
        .collect();
    progress.sort_by_key(|(n, _, _, _)| *n);

    Ok(Outcome { problems, progress })
}

#[cfg(test)]
mod tests {
    use super::*;

    /// The body as `issues()` hands it over: newlines carried as `BODY_NEWLINE`.
    fn body(lines: &[&str]) -> String {
        lines.join("\u{1}")
    }

    #[test]
    fn a_capability_declares_its_steps_under_the_steps_heading() {
        let b = body(&[
            "Pilot is the only way to see inside a running Drone today.",
            "",
            "**Steps**",
            "",
            "Ordered. The tee and the log come first.",
            "",
            "- #101",
            "- #102",
        ]);
        assert_eq!(step_refs(&b), vec![101, 102]);
    }

    /// The bug that made two closed Recovery capabilities owe Pilot's work.
    #[test]
    fn a_cross_reference_outside_the_section_is_not_a_step() {
        let b = body(&[
            "| The Job is not piloted | A person is at an unrestricted toolset — #367 |",
            "",
            "`reclaim.rs` does the work and #297 gave it a route.",
        ]);
        assert!(step_refs(&b).is_empty());
    }

    #[test]
    fn the_section_ends_at_the_next_heading() {
        let b = body(&["**Steps**", "- #101", "", "## Watch for", "- #999"]);
        assert_eq!(step_refs(&b), vec![101]);
    }
}
