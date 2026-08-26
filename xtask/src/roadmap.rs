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
//! | `verify-roadmap` | CI and on demand | the issue named by a file, and what a capability's steps add up to |
//!
//! Splitting it is the honest shape rather than a compromise. The offline half
//! catches the mistake somebody makes while writing — a file with no issue
//! number, a slug that disagrees with its filename — and catches it before the
//! commit. The online half catches the mistake nobody makes on purpose: an
//! issue deleted, relabelled, or moved to another milestone under a file that
//! still claims the old one.
//!
//! It also does the arithmetic a checkbox would only display: a capability
//! references its steps by number, and a capability closed while its steps are
//! open is a failure rather than a rendering.

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
    /// Every `#N` the body references. On a capability, these are its steps.
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
            r#".[] | [(.number|tostring), .state, (.milestone.title // ""), .title, ((.body // "") | gsub("[\r\n]"; " "))] | @tsv"#,
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
            refs: issue_refs(f.next().unwrap_or_default()),
        });
    }
    Ok(out)
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
    // pick one and demote four. The capability issue references its steps by
    // number instead, and the arithmetic lives here where it can fail rather
    // than in a checkbox that only renders.
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
