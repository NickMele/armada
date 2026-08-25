//! Reading `docs/capabilities/`, and the half of its check that works offline.
//!
//! One file per capability, paired to one GitHub issue. The issue tracks — its
//! steps are sub-issues, so progress is computed rather than reported. The file
//! holds the prose, the rationale, and the links to the concepts it depends on,
//! because an issue body buries all of that the moment the issue closes, and
//! because a file is reviewable in the same diff as the code that delivered it.
//!
//! The frontmatter is the binding:
//!
//! ```text
//! ---
//! capability: worktree-per-job
//! issue: 42
//! milestone: M1 — Dogfood
//! ---
//! ```
//!
//! What this module checks is everything visible without a network: that the
//! frontmatter is there and well formed, that the slug agrees with the
//! filename, that no two files claim one issue, and that the index knows every
//! file and every file is in the index. Whether issue 42 exists is
//! `verify-roadmap`'s question — see [`crate::roadmap`] for why that is a
//! separate command rather than a fourteenth rule.

use std::collections::BTreeMap;
use std::fs;
use std::path::Path;

use crate::{files_with_ext, Report};

/// One capability, as its file declares it.
pub struct Capability {
    pub slug: String,
    pub issue: u64,
    pub milestone: Option<String>,
    pub path: String,
}

/// Every capability file, or the first thing wrong with one.
pub fn read(root: &Path) -> Result<Vec<Capability>, String> {
    let dir = root.join("docs/capabilities");
    let mut out = Vec::new();
    for path in files_with_ext(root, &dir, &["md"]) {
        let name = path.rsplit('/').next().unwrap_or(&path);
        if name == "INDEX.md" {
            continue;
        }
        let text = fs::read_to_string(root.join(&path))
            .map_err(|_| format!("{path} could not be read"))?;
        out.push(parse(&text, &path)?);
    }
    Ok(out)
}

fn parse(text: &str, path: &str) -> Result<Capability, String> {
    let stem = path
        .rsplit('/')
        .next()
        .and_then(|n| n.strip_suffix(".md"))
        .unwrap_or_default()
        .to_string();

    let mut front = BTreeMap::new();
    let mut lines = text.lines();
    if lines.next().map(str::trim) != Some("---") {
        return Err(format!("{path} has no frontmatter — it needs `capability:` and `issue:`"));
    }
    for line in lines {
        if line.trim() == "---" {
            break;
        }
        if let Some((k, v)) = line.split_once(':') {
            front.insert(k.trim().to_string(), v.trim().to_string());
        }
    }

    let slug = front
        .get("capability")
        .ok_or_else(|| format!("{path} frontmatter has no `capability:` slug"))?;
    if slug != &stem {
        return Err(format!(
            "{path} declares `capability: {slug}` but is named {stem}.md — \
             the filename is the identity, so one of them is a typo"
        ));
    }
    let issue = front
        .get("issue")
        .ok_or_else(|| format!("{path} frontmatter has no `issue:` number"))?;
    let issue: u64 = issue
        .trim_start_matches('#')
        .parse()
        .map_err(|_| format!("{path} has `issue: {issue}`, which is not an issue number"))?;

    Ok(Capability {
        slug: slug.clone(),
        issue,
        milestone: front.get("milestone").filter(|m| !m.is_empty()).cloned(),
        path: path.to_string(),
    })
}

/// Rule fourteen: every capability file is well formed and indexed.
///
/// The half a gate can check with no network. It says nothing about whether the
/// issue exists — that is `cargo xtask verify-roadmap`, and it is deliberately
/// not here, because a rule that needs GitHub is a rule that fails on a train.
pub fn every_capability_is_bound_and_indexed(root: &Path) -> Report {
    let mut report = Report::new("every capability names an issue, and the index knows it");
    let dir = root.join("docs/capabilities");
    if !dir.is_dir() {
        return report; // Nothing has migrated yet. Rule fifteen is the one that would notice.
    }

    let caps = match read(root) {
        Ok(caps) => caps,
        Err(why) => {
            report.fail(why);
            return report;
        }
    };

    let mut by_issue: BTreeMap<u64, &str> = BTreeMap::new();
    for cap in &caps {
        if let Some(other) = by_issue.insert(cap.issue, &cap.path) {
            report.fail(format!(
                "{} and {other} both claim issue #{} — an issue tracks one capability",
                cap.path, cap.issue
            ));
        }
    }

    let index_path = dir.join("INDEX.md");
    let Ok(index) = fs::read_to_string(&index_path) else {
        if !caps.is_empty() {
            report.fail("docs/capabilities/INDEX.md — capabilities exist and nothing lists them");
        }
        return report;
    };

    for cap in &caps {
        let name = format!("{}.md", cap.slug);
        if !index.contains(&name) {
            report.fail(format!("{} is a capability the index does not mention", cap.path));
        }
    }
    for line in index.lines() {
        for token in line.split(|c: char| !(c.is_alphanumeric() || c == '-' || c == '_' || c == '.')) {
            if token.ends_with(".md") && token != "INDEX.md" && !dir.join(token).is_file() {
                report.fail(format!(
                    "the capability index names {token}, which is not in docs/capabilities/"
                ));
            }
        }
    }
    report
}
