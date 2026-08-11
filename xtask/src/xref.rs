//! Resolve every `§N.N` cross-reference against the heading index.
//!
//! This corpus is six documents that cite each other constantly, and its most
//! repeated defect is a citation that survives the thing it cites moving. One
//! audit found eight dangling references at once, including three that pointed
//! at sections which had been split into a different file entirely.
//!
//! Attribution is deliberately generous, because both orders occur here —
//! "`PLAN.md` §4.3" and "§2.7 of `ARCHITECTURE.md`" — and a first cut that
//! only looked backwards reported a dozen correct citations as dangling. A
//! reference resolves if **any** candidate has the section: the document named
//! just after it, the one named just before, or the file it sits in. That
//! still catches the real defect, which is a `§` that exists nowhere — the
//! shape of every dangling reference this corpus has actually had.

use crate::docs::{Doc, Finding};
use regex::Regex;
use std::collections::HashMap;

/// How far back a document name may sit and still claim a following `§`.
///
/// Beyond this, a name earlier in a long sentence is unrelated prose rather
/// than an attribution. Tuned against the corpus: raising it produces false
/// attributions, lowering it produces false "unresolved".
const ATTRIBUTION_WINDOW: usize = 40;

pub fn check(docs: &[Doc]) -> Vec<Finding> {
    let heading = Regex::new(r"^#{2,6}\s+(?:§\s*)?([0-9]+(?:\.[0-9]+)*)[.:]?\s+").unwrap();
    let section = Regex::new(r"§\s*([0-9]+(?:\.[0-9]+)*)").unwrap();
    let docname =
        Regex::new(r"(PLAN\.md|ARCHITECTURE\.md|PHASES\.md|traps\.md|AGENTS\.md|README\.md)")
            .unwrap();

    // Heading index per document.
    let mut index: HashMap<&str, Vec<String>> = HashMap::new();
    for doc in docs {
        let secs = doc
            .lines()
            .filter_map(|l| heading.captures(l).map(|c| c[1].to_string()))
            .collect();
        index.insert(doc.name, secs);
    }

    let mut findings = Vec::new();
    for doc in docs {
        for (n, line) in doc.lines().enumerate() {
            for m in section.captures_iter(line) {
                let whole = m.get(0).unwrap();
                let sec = &m[1];

                // Skip the headings themselves — "## §4.3 Foo" defines, not cites.
                if heading.is_match(line) {
                    continue;
                }

                let mut candidates: Vec<&str> = vec![doc.name];
                if let Some(dm) = docname
                    .find_iter(&line[..whole.start()])
                    .last()
                    .filter(|dm| whole.start() - dm.end() <= ATTRIBUTION_WINDOW)
                {
                    candidates.push(dm.as_str());
                }
                if let Some(dm) = docname
                    .find(&line[whole.end()..])
                    .filter(|dm| dm.start() <= ATTRIBUTION_WINDOW)
                {
                    candidates.push(dm.as_str());
                }

                let known = candidates
                    .iter()
                    .any(|t| index.get(t).is_some_and(|v| v.iter().any(|s| s == sec)));

                if !known {
                    findings.push(Finding {
                        file: doc.rel.to_string(),
                        line: n + 1,
                        message: format!(
                            "§{sec} resolves in none of [{}] — {}",
                            candidates.join(", "),
                            snippet(line)
                        ),
                    });
                }
            }
        }
    }
    findings
}

fn snippet(line: &str) -> String {
    let t = line.trim();
    if t.chars().count() <= 110 {
        return t.to_string();
    }
    t.chars().take(107).collect::<String>() + "..."
}
