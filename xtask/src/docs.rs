//! The open-questions walk, and the file it writes.
//!
//! An open question lives in the document it blocks, under `## Open questions`
//! — the convention `rust.md`, `bridge.md` and `protocol.md` already follow.
//! That keeps the question next to the thing that gives it meaning, and next
//! to the person who can answer it.
//!
//! What a per-document section cannot do is answer *what is open across the
//! whole project*, which is the one thing a database view was good at. So the
//! completeness is restored the way this repo restores completeness everywhere
//! else: by a walk, not by somebody remembering to update a list. `OPEN.md` is
//! generated, `verify-docs` fails when it is stale, and the gate carries that.
//!
//! # Slugs
//!
//! A question may open with a `[slug]`, which makes it citable:
//!
//! ```text
//! - **[protocol-codegen]** What generates the TypeScript from `ipc`.
//! ```
//!
//! The design-lint escape hatch cites one. So **answering a question breaks
//! every citation of it**, on purpose: delete the bullet, and the gate names
//! the code that was waiting on the answer. Today an answered question leaves
//! its opt-outs behind and nothing notices.

use std::collections::BTreeMap;
use std::fmt::Write as _;
use std::fs;
use std::path::Path;

use crate::{files_with_ext, walk};

/// The heading a question has to sit under to be collected.
const HEADING: &str = "## Open questions";

/// The generated index. Named in `SKIP` so the walk never reads itself.
const OUTPUT: &str = "docs/OPEN.md";

/// Generated or archival files the walk does not read. `v1-learnings` is a
/// record of what v1 taught, and its open questions are v1's, not v2's.
const SKIP: &[&str] = &["docs/OPEN.md", "docs/v1-learnings/"];

/// One open question, and where it is written.
pub struct Question {
    pub slug: Option<String>,
    pub text: String,
    pub file: String,
}

/// Headings that mean "open questions" and are not the one heading the walk
/// reads. A document using one of these has questions nobody can see.
///
/// This is not pedantry about a title. Four contracts arrived on the first day
/// of the migration under `## Open` and `# Open`, and `verify-docs` stayed
/// green while five questions sat uncollected — which is exactly the silent
/// failure the walk exists to prevent, produced by the walk itself.
pub fn near_miss_headings(root: &Path) -> Vec<(String, usize, String)> {
    let mut out = Vec::new();
    let mut paths = files_with_ext(root, &root.join("docs"), &["md"]);
    paths.extend(
        files_with_ext(root, &root.join("crates"), &["md"])
            .into_iter()
            .filter(|p| p.ends_with("/README.md")),
    );
    for path in paths {
        if SKIP.iter().any(|s| path.starts_with(s)) {
            continue;
        }
        let Ok(text) = fs::read_to_string(root.join(&path)) else {
            continue;
        };
        for (n, line) in text.lines().enumerate() {
            let t = line.trim_end();
            if !t.starts_with('#') {
                continue;
            }
            let title = t.trim_start_matches('#').trim();
            let looks_like = title.eq_ignore_ascii_case("open")
                || title.eq_ignore_ascii_case("open items")
                || title.eq_ignore_ascii_case("open question")
                || title.eq_ignore_ascii_case("questions")
                || title.eq_ignore_ascii_case("open questions");
            if looks_like && t != HEADING {
                out.push((path.clone(), n + 1, t.to_string()));
            }
        }
    }
    out
}

/// Every open question, in file order.
///
/// `docs/` plus any `README.md` under `crates/`. A question about the domain
/// data blocks the crate that will read it, not a document about it, and the
/// rule everywhere else here is that a question lives with the thing it blocks.
pub fn read(root: &Path) -> Vec<Question> {
    let mut out = Vec::new();
    let mut paths = files_with_ext(root, &root.join("docs"), &["md"]);
    paths.extend(
        files_with_ext(root, &root.join("crates"), &["md"])
            .into_iter()
            .filter(|p| p.ends_with("/README.md")),
    );
    for path in paths {
        if SKIP.iter().any(|s| path.starts_with(s)) {
            continue;
        }
        let Ok(text) = fs::read_to_string(root.join(&path)) else {
            continue;
        };
        out.extend(in_one_file(&text, &path));
    }
    out
}

/// The bullets under this file's `## Open questions`, if it has one. A bullet
/// runs until the next bullet or the next heading, so a question may explain
/// itself over several lines without being cut in half.
fn in_one_file(text: &str, path: &str) -> Vec<Question> {
    let mut out: Vec<Question> = Vec::new();
    let mut inside = false;
    for line in text.lines() {
        if line.trim_end() == HEADING {
            inside = true;
            continue;
        }
        if inside && line.starts_with("## ") {
            break;
        }
        if !inside {
            continue;
        }
        let trimmed = line.trim_start();
        if let Some(rest) = trimmed.strip_prefix("- ") {
            let (slug, text) = split_slug(rest);
            out.push(Question {
                slug,
                text,
                file: path.to_string(),
            });
        } else if !trimmed.is_empty() {
            if let Some(last) = out.last_mut() {
                last.text.push(' ');
                last.text.push_str(trimmed);
            }
        }
    }
    out
}

/// `**[slug]** the question` becomes `(Some("slug"), "the question")`. A
/// bullet with no slug is still a question — it just cannot be cited.
fn split_slug(rest: &str) -> (Option<String>, String) {
    let body = rest.trim();
    let stripped = body.strip_prefix("**").unwrap_or(body);
    let Some(inner) = stripped.strip_prefix('[') else {
        return (None, body.to_string());
    };
    let Some(end) = inner.find(']') else {
        return (None, body.to_string());
    };
    let slug = &inner[..end];
    if slug.is_empty()
        || !slug
            .chars()
            .all(|c| c.is_ascii_lowercase() || c.is_ascii_digit() || c == '-')
    {
        return (None, body.to_string());
    }
    let after = inner[end + 1..].trim_start_matches("**").trim();
    (Some(slug.to_string()), after.to_string())
}

/// `docs/OPEN.md`, grouped by the document each question blocks.
pub fn open_md(questions: &[Question]) -> String {
    let mut by_file: BTreeMap<&str, Vec<&Question>> = BTreeMap::new();
    for q in questions {
        by_file.entry(&q.file).or_default().push(q);
    }

    let mut out = String::from(
        "<!-- GENERATED by `cargo xtask verify-docs --write`. Do not hand-edit. -->\n\n\
         # Open questions\n\n\
         Every question is written in the document it blocks, under that file's\n\
         `## Open questions`. This file is the walk over all of them, so the\n\
         project-wide view survives without a second place to keep it current.\n\n\
         A question in **[brackets]** carries a slug and can be cited — from a\n\
         design-lint opt-out, or from anything else that is waiting on it.\n\
         Answering one means deleting its bullet, which breaks those citations\n\
         on purpose and makes the gate name what was waiting.\n",
    );

    if questions.is_empty() {
        out.push_str("\nNothing is open.\n");
        return out;
    }

    for (file, qs) in &by_file {
        let _ = write!(out, "\n## {file}\n\n");
        for q in qs {
            match &q.slug {
                Some(slug) => {
                    let _ = writeln!(out, "- **[{slug}]** {}", q.text);
                }
                None => {
                    let _ = writeln!(out, "- {}", q.text);
                }
            }
        }
    }
    out
}

/// The one output, as (path, contents).
pub fn outputs(root: &Path) -> Result<Vec<(String, String)>, String> {
    let questions = read(root);

    let mut seen: BTreeMap<&str, &str> = BTreeMap::new();
    for q in &questions {
        let Some(slug) = q.slug.as_deref() else {
            continue;
        };
        if let Some(other) = seen.insert(slug, &q.file) {
            return Err(format!(
                "[{slug}] is claimed by both {other} and {} — a citation would be \
                 ambiguous, so one of them needs a different slug",
                q.file
            ));
        }
    }

    Ok(vec![(OUTPUT.to_string(), open_md(&questions))])
}

/// Every `[slug]` cited from source, with the file and line that cites it.
///
/// Read separately from the questions themselves so the check can say which
/// side is wrong: a citation with no question is a stale opt-out, and that is
/// a different failure from a question nobody cites.
pub fn citations(root: &Path) -> Vec<(String, String, usize)> {
    const MARKER: &str = "armada-allow-off-contract:";
    let mut out = Vec::new();
    for dir in ["apps", "crates", "packages", "xtask"] {
        walk(&root.join(dir), &mut |path| {
            let Ok(rel) = path.strip_prefix(root) else {
                return;
            };
            let rel = rel.to_string_lossy().replace('\\', "/");
            let Ok(text) = fs::read_to_string(path) else {
                return;
            };
            if !text.contains(MARKER) {
                return;
            }
            let lines: Vec<&str> = text.lines().collect();
            for (i, line) in lines.iter().enumerate() {
                if !line.contains(MARKER) {
                    continue;
                }
                // A doc comment showing the syntax is an illustration, not an
                // opt-out. The rule that defines the marker has to be able to
                // write it down without the gate reading it as a real citation.
                let start = line.trim_start();
                if start.starts_with("///") || start.starts_with("//!") {
                    continue;
                }
                let Some(next) = lines.get(i + 1) else {
                    continue;
                };
                let Some(open) = next.find('[') else { continue };
                let Some(close) = next[open..].find(']') else {
                    continue;
                };
                let slug = &next[open + 1..open + close];
                if !slug.is_empty()
                    && slug
                        .chars()
                        .all(|c| c.is_ascii_lowercase() || c.is_ascii_digit() || c == '-')
                {
                    out.push((slug.to_string(), rel.clone(), i + 2));
                }
            }
        });
    }
    out
}
