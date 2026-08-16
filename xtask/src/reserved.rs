//! The reserved designs' numbering contract: one id per design, the filename
//! and the frontmatter agreeing on it, a row in the index, and every link that
//! cites a design resolving to a file.
//!
//! **This exists because the same defect landed three times.** `docs/reserved/`
//! gains a file whenever somebody uses Armada and finds a gap, so two branches
//! written the same afternoon both reach for the next free number. Git does not
//! notice — two files with different names are not a conflict — and neither
//! does any test, because the ids are documentation. All three collisions were
//! found by a human reading a table.
//!
//! The third was the one that argued for a gate. Its design was renumbered
//! twice on the way in, `020` to `021` to `022`; the filename and the index row
//! moved and **the frontmatter followed neither**, leaving the file claiming an
//! id that another design already held. The same renumber left a link in
//! `docs/commands/manifest/prune.md` pointing at a path that no longer existed.
//! One rename, three inconsistent records, and a clean `doclint`.
//!
//! None of the six rules is clever. Each is a thing that has already gone
//! wrong, phrased so that the branch which does it again fails at the gate
//! rather than at the merge.

use crate::docs::{Doc, Finding, RESERVED_DIR};
use regex::Regex;
use std::collections::BTreeMap;
use std::path::Path;

/// The index itself, which is a row per design rather than a design.
const INDEX: &str = "README.md";

pub fn check(docs: &[Doc], root: &Path) -> Result<Vec<Finding>, String> {
    let mut findings = corpus(docs);
    findings.extend(dead_links(root)?);
    findings.sort_by(|a, b| (&a.file, a.line).cmp(&(&b.file, b.line)));
    Ok(findings)
}

/// The five rules that need only the documents themselves.
///
/// Split from `check` so the tests exercise the shipped logic rather than a
/// paraphrase of it — `docs.rs` already learned that two implementations of one
/// rule make a green lint meaningless, and a test copy is the second one.
fn corpus(docs: &[Doc]) -> Vec<Finding> {
    let prefix = format!("{RESERVED_DIR}/");
    let named = Regex::new(r"^([0-9]{3})-[a-z0-9-]+\.md$").unwrap();

    let mut findings = Vec::new();
    // Keyed by id so the report reads in numeric order — a gate whose output
    // moves between runs is one nobody trusts enough to diff.
    let mut claims: BTreeMap<String, Vec<String>> = BTreeMap::new();
    let mut index: Option<&Doc> = None;
    let mut designs: Vec<&Doc> = Vec::new();

    for doc in docs {
        let Some(base) = doc.rel.strip_prefix(&prefix) else {
            continue;
        };
        if base == INDEX {
            index = Some(doc);
            continue;
        }

        // Rule 1 — the filename carries the id. A file that does not state one
        // cannot be checked against anything, and cannot be cited by the shape
        // every other document uses.
        let Some(caps) = named.captures(base) else {
            findings.push(Finding {
                file: doc.rel.clone(),
                line: 1,
                message: format!(
                    "`{base}` is not `NNN-kebab-title.md` — every reserved design is cited by its number"
                ),
            });
            continue;
        };
        let from_name = caps[1].to_string();
        designs.push(doc);

        // Rule 2 — the frontmatter must agree with the filename. This is the
        // half a renumber forgets, because renaming the file and editing the
        // index row both feel like having finished.
        match frontmatter_id(&doc.text) {
            Some((from_front, _)) if from_front == from_name => {}
            Some((from_front, line)) => findings.push(Finding {
                file: doc.rel.clone(),
                line,
                message: format!(
                    "frontmatter says `id: {from_front}` but the filename says `{from_name}` — a renumber moved one and not the other"
                ),
            }),
            None => findings.push(Finding {
                file: doc.rel.clone(),
                line: 1,
                message: "no `id:` in the frontmatter — nothing to check the filename against"
                    .to_string(),
            }),
        }

        // Rule 3 — a numbered H1 must carry the right number. The same
        // renumber that stranded the frontmatter left `# 021 — …` at the top
        // of the file now named `022`, which is the copy a reader sees first.
        //
        // **Conditional on purpose.** Four designs deliberately have an
        // unnumbered heading, and requiring the prefix would be a style rule
        // this check has no standing to invent. Agreement is the claim; the
        // convention is not.
        if let Some((from_head, line)) = heading_id(&doc.text) {
            if from_head != from_name {
                findings.push(Finding {
                    file: doc.rel.clone(),
                    line,
                    message: format!(
                        "the heading reads `# {from_head} — …` but the filename says `{from_name}` — the number a reader sees first"
                    ),
                });
            }
        }

        claims.entry(from_name).or_default().push(doc.rel.clone());
    }

    // Rule 4 — two designs may not claim one number. The collision this check
    // exists for.
    for (id, holders) in &claims {
        if holders.len() > 1 {
            findings.push(Finding {
                file: holders[0].clone(),
                line: 1,
                message: format!(
                    "id `{id}` is claimed by {} files ({}) — numbers are never reused or reassigned",
                    holders.len(),
                    holders.join(", ")
                ),
            });
        }
    }

    // Rule 5 — a design absent from the index is one nothing will ever surface,
    // which is the drift `docs.rs` already worried about for the corpus itself.
    if let Some(idx) = index {
        for doc in &designs {
            let base = doc.rel.strip_prefix(&prefix).unwrap_or(&doc.rel);
            if !idx.text.contains(base) {
                findings.push(Finding {
                    file: doc.rel.clone(),
                    line: 1,
                    message: format!("no row in `{RESERVED_DIR}/{INDEX}` links to `{base}`"),
                });
            }
        }
    }

    findings
}

/// The `id:` line of a document's frontmatter, with the line it sits on.
///
/// Deliberately not a YAML parse: the frontmatter here is five scalar keys, and
/// the one thing this needs — the line number, so a finding is clickable — is
/// what a parsed document throws away.
fn frontmatter_id(text: &str) -> Option<(String, usize)> {
    let mut lines = text.lines().enumerate();
    if !matches!(lines.next(), Some((_, "---"))) {
        return None;
    }
    for (n, line) in lines {
        if line.trim() == "---" {
            return None;
        }
        if let Some(rest) = line.strip_prefix("id:") {
            return Some((rest.trim().to_string(), n + 1));
        }
    }
    None
}

/// The number a numbered H1 claims, with its line. `None` when the heading
/// carries no number, which is legal.
fn heading_id(text: &str) -> Option<(String, usize)> {
    let numbered = Regex::new(r"^#\s+([0-9]{3})\s+—").unwrap();
    text.lines()
        .enumerate()
        .find(|(_, l)| l.starts_with("# "))
        .and_then(|(n, l)| numbered.captures(l).map(|c| (c[1].to_string(), n + 1)))
}

/// Rule 6 — every link to a reserved design, across every tracked markdown
/// file, that names a path which does not exist.
///
/// **Tracked files rather than the lint corpus, and that is the point.** The
/// dead link that prompted this rule lived in `docs/commands/manifest/prune.md`
/// — a per-verb reference page, not one of the six documents `docs::FIXED`
/// lists. A check scoped to the corpus would have reported clean on it.
///
/// Only links whose basename looks like a reserved design are examined. The
/// broader claim — that every markdown link in the repository resolves — is a
/// different and much larger check, and asserting it here would bury the
/// numbering defect this is for.
fn dead_links(root: &Path) -> Result<Vec<Finding>, String> {
    let link = Regex::new(r"\]\(([^)\s]+\.md)(?:#[^)]*)?\)").unwrap();
    let design = Regex::new(r"^[0-9]{3}-[a-z0-9-]+\.md$").unwrap();

    let mut findings = Vec::new();
    for rel in crate::privacy::tracked(root)? {
        if !rel.ends_with(".md") {
            continue;
        }
        // A tracked path that will not read is not this check's business —
        // `privacy` reports on the working tree's readability already.
        let Ok(text) = std::fs::read_to_string(root.join(&rel)) else {
            continue;
        };
        for (n, line) in text.lines().enumerate() {
            for m in link.captures_iter(line) {
                let target = &m[1];
                let base = target.rsplit('/').next().unwrap_or(target);
                if !design.is_match(base) {
                    continue;
                }
                match resolve(&rel, target) {
                    Some(path) if root.join(&path).is_file() => {}
                    Some(path) => findings.push(Finding {
                        file: rel.clone(),
                        line: n + 1,
                        message: format!("`{target}` resolves to `{path}`, which does not exist"),
                    }),
                    None => findings.push(Finding {
                        file: rel.clone(),
                        line: n + 1,
                        message: format!("`{target}` climbs past the repository root"),
                    }),
                }
            }
        }
    }
    Ok(findings)
}

/// A relative link resolved against the directory holding the file it was
/// written in, as a repo-relative path.
///
/// Resolving properly rather than comparing basenames is what catches a `../`
/// at the wrong depth — the shape a link acquires when a citation is copied
/// between two documents at different depths, and five different path shapes
/// for these designs are already in use.
fn resolve(from: &str, target: &str) -> Option<String> {
    let mut parts: Vec<&str> = from.split('/').collect();
    parts.pop()?;
    for seg in target.split('/') {
        match seg {
            "" | "." => {}
            ".." => {
                parts.pop()?;
            }
            other => parts.push(other),
        }
    }
    Some(parts.join("/"))
}

#[cfg(test)]
mod tests {
    use super::*;

    fn doc(rel: &str, text: &str) -> Doc {
        Doc {
            name: rel.rsplit('/').next().unwrap().to_string(),
            rel: rel.to_string(),
            text: text.to_string(),
        }
    }

    /// A design with an unnumbered heading — legal, and what four of the real
    /// ones look like.
    fn design(id: &str) -> String {
        format!("---\nid: {id}\ntitle: A thing\nstatus: RESERVED\n---\n\n# A thing\n")
    }

    /// A design whose heading carries a number, which is the majority shape.
    fn numbered(id: &str, heading: &str) -> String {
        format!("---\nid: {id}\ntitle: A thing\nstatus: RESERVED\n---\n\n# {heading} — A thing\n")
    }

    /// The index, linking exactly the basenames given.
    fn index(bases: &[&str]) -> Doc {
        let rows: String = bases
            .iter()
            .map(|b| format!("| [x]({b}) | a design |\n"))
            .collect();
        doc("docs/reserved/README.md", &format!("# Reserved\n\n{rows}"))
    }

    fn messages(docs: &[Doc]) -> Vec<String> {
        corpus(docs).into_iter().map(|f| f.message).collect()
    }

    #[test]
    fn a_consistent_set_is_clean() {
        let docs = vec![
            index(&["001-a.md", "002-b.md"]),
            doc("docs/reserved/001-a.md", &design("001")),
            doc("docs/reserved/002-b.md", &design("002")),
        ];
        assert!(corpus(&docs).is_empty());
    }

    /// The exact defect found in the merge: the filename was renumbered, the
    /// frontmatter was left behind, and the stale value names a real design.
    #[test]
    fn a_renumber_that_forgot_the_frontmatter_is_caught() {
        let docs = vec![
            index(&["020-tui.md", "022-docker.md"]),
            doc("docs/reserved/020-tui.md", &design("020")),
            doc("docs/reserved/022-docker.md", &design("020")),
        ];
        let found = corpus(&docs);
        assert_eq!(found.len(), 1, "{:?}", messages(&docs));
        assert_eq!(found[0].file, "docs/reserved/022-docker.md");
        // Line 2 — the `id:` line, so the finding lands on the thing to edit.
        assert_eq!(found[0].line, 2);
        assert!(
            found[0].message.contains("`id: 020`") && found[0].message.contains("`022`"),
            "{}",
            found[0].message
        );
    }

    /// Two branches each taking the next free number — the collision itself.
    #[test]
    fn two_designs_claiming_one_number_are_caught() {
        let docs = vec![
            index(&["021-one.md", "021-two.md"]),
            doc("docs/reserved/021-one.md", &design("021")),
            doc("docs/reserved/021-two.md", &design("021")),
        ];
        let m = messages(&docs);
        assert_eq!(m.len(), 1, "{m:?}");
        assert!(m[0].contains("id `021` is claimed by 2 files"), "{m:?}");
    }

    #[test]
    fn a_design_missing_from_the_index_is_caught() {
        let docs = vec![
            index(&["001-a.md"]),
            doc("docs/reserved/001-a.md", &design("001")),
            doc("docs/reserved/002-b.md", &design("002")),
        ];
        let m = messages(&docs);
        assert_eq!(m.len(), 1, "{m:?}");
        assert!(m[0].contains("no row in"), "{m:?}");
        assert!(m[0].contains("002-b.md"), "{m:?}");
    }

    #[test]
    fn a_file_that_is_not_numbered_is_caught() {
        let docs = vec![index(&[]), doc("docs/reserved/notes.md", "# Notes\n")];
        let m = messages(&docs);
        assert_eq!(m.len(), 1, "{m:?}");
        assert!(m[0].contains("is not `NNN-kebab-title.md`"), "{m:?}");
    }

    /// A design with no frontmatter at all — the id exists in one place only,
    /// so there is nothing to disagree with and the check must say so rather
    /// than pass.
    #[test]
    fn a_design_with_no_frontmatter_is_caught() {
        let docs = vec![
            index(&["003-c.md"]),
            doc("docs/reserved/003-c.md", "# A thing\n"),
        ];
        let m = messages(&docs);
        assert_eq!(m.len(), 1, "{m:?}");
        assert!(m[0].contains("no `id:` in the frontmatter"), "{m:?}");
    }

    /// The sixth stale record the same renumber left: the number a reader sees
    /// before anything else.
    #[test]
    fn a_heading_carrying_the_old_number_is_caught() {
        let docs = vec![
            index(&["022-docker.md"]),
            doc("docs/reserved/022-docker.md", &numbered("022", "021")),
        ];
        let found = corpus(&docs);
        assert_eq!(found.len(), 1, "{:?}", messages(&docs));
        assert_eq!(found[0].line, 7, "the heading's own line");
        assert!(
            found[0].message.contains("`# 021 — …`"),
            "{}",
            found[0].message
        );
    }

    #[test]
    fn a_heading_carrying_the_right_number_is_clean() {
        let docs = vec![
            index(&["022-docker.md"]),
            doc("docs/reserved/022-docker.md", &numbered("022", "022")),
        ];
        assert!(corpus(&docs).is_empty());
    }

    /// Four designs deliberately have no number in the heading. Requiring one
    /// would be a style rule this check has no standing to invent.
    #[test]
    fn an_unnumbered_heading_is_legal() {
        let docs = vec![
            index(&["020-tui.md"]),
            doc("docs/reserved/020-tui.md", &design("020")),
        ];
        assert!(corpus(&docs).is_empty());
    }

    #[test]
    fn the_index_itself_is_never_a_design() {
        assert!(corpus(&[index(&[])]).is_empty());
    }

    /// Documents outside `docs/reserved/` are not designs and are not judged by
    /// these rules — the corpus holds `PLAN.md` and friends too.
    #[test]
    fn the_rest_of_the_corpus_is_ignored() {
        let docs = vec![
            doc("docs/PLAN.md", "# Plan\n"),
            doc("AGENTS.md", "# Agents\n"),
        ];
        assert!(corpus(&docs).is_empty());
    }

    #[test]
    fn frontmatter_id_is_read_only_from_the_block() {
        assert_eq!(
            frontmatter_id("---\nid: 007\n---\n"),
            // Line 2 — line 1 is the opening `---`.
            Some(("007".to_string(), 2))
        );
        // A body mentioning `id:` after the block has closed is prose.
        assert_eq!(frontmatter_id("---\ntitle: x\n---\n\nid: 009\n"), None);
        assert_eq!(frontmatter_id("# Just a heading\n"), None);
    }

    #[test]
    fn links_resolve_from_the_file_that_wrote_them() {
        // The five shapes actually in use in this repository.
        for (from, target) in [
            ("docs/reserved/README.md", "001-a.md"),
            ("docs/commands/manifest/prune.md", "../../reserved/001-a.md"),
            ("docs/PLAN.md", "reserved/001-a.md"),
            ("README.md", "docs/reserved/001-a.md"),
            ("docs/commands/reference.md", "../reserved/001-a.md"),
        ] {
            assert_eq!(
                resolve(from, target).as_deref(),
                Some("docs/reserved/001-a.md"),
                "{from} -> {target}"
            );
        }
    }

    /// A `../` at the wrong depth is the defect proper resolution exists to
    /// catch, and it must not silently land somewhere plausible.
    #[test]
    fn a_link_at_the_wrong_depth_does_not_resolve_to_the_right_place() {
        assert_eq!(
            resolve("docs/commands/manifest/prune.md", "../reserved/001-a.md").as_deref(),
            Some("docs/commands/reserved/001-a.md")
        );
    }

    #[test]
    fn a_link_climbing_past_the_root_is_not_resolved() {
        assert_eq!(resolve("README.md", "../../elsewhere/001-a.md"), None);
    }
}
