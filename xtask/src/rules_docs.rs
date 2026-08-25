//! The documentation half of the gate.
//!
//! `docs/` is exempt from every other rule here — `SOURCE_ROOTS` is crates,
//! apps, packages and xtask, on the reasoning that prose is not source. That
//! held while `docs/` carried spike records. It stops holding the moment a
//! contract lands there, because a contract is read by whoever writes the code
//! that obeys it, and nothing was checking the most-read documents in the repo.

use std::fs;
use std::path::Path;

use crate::{docs, files_with_ext, Report};

/// Rule thirteen: every open question is collected, and every citation of one
/// resolves.
///
/// Two failures, and they are different problems wearing the same shape. A
/// stale `OPEN.md` means somebody wrote a question and the project-wide view no
/// longer shows it. A citation with no question means somebody answered one and
/// left an opt-out standing on an argument that no longer exists — the quieter
/// of the two, and the reason the slug mechanism is worth having at all.
pub fn every_open_question_is_collected(root: &Path) -> Report {
    let mut report = Report::new("open questions are collected, and citations resolve");

    match docs::outputs(root) {
        Err(why) => report.fail(why),
        Ok(outputs) => {
            for (rel, want) in outputs {
                match fs::read_to_string(root.join(&rel)) {
                    Ok(have) if have == want => {}
                    Ok(_) => report.fail(format!(
                        "{rel} — stale or hand-edited. \
                         Run `cargo xtask verify-docs --write` and commit what it emits"
                    )),
                    Err(_) => report.fail(format!("{rel} — not generated yet")),
                }
            }
        }
    }

    let questions = docs::read(root);
    for (slug, file, line) in docs::citations(root) {
        if !questions.iter().any(|q| q.slug.as_deref() == Some(slug.as_str())) {
            report.fail(format!(
                "{file}:{line} cites [{slug}], which no document asks. \
                 Either the question was answered and this opt-out outlived it, \
                 or the slug is a typo"
            ));
        }
    }

    report
}

/// Rule fifteen: every document is in an index, and every index entry is a
/// document.
///
/// The same both-ways shape rule eight uses for the v1 harvest, widened to all
/// of `docs/`. It is not about tidiness: a document nobody indexed is a
/// document nobody is expected to read, and the failure it produces is
/// somebody rewriting from scratch what was already written down.
///
/// A directory carrying its own `INDEX.md` is listed as a directory, and its
/// contents are that index's problem. Nesting the rule rather than flattening
/// it is what keeps the top-level index short enough that people read it.
pub fn every_document_is_indexed(root: &Path) -> Report {
    let mut report = Report::new("every document is in an index, and every entry is a document");
    let docs = root.join("docs");
    let index_path = docs.join("INDEX.md");

    let Ok(index) = fs::read_to_string(&index_path) else {
        report.fail("docs/INDEX.md — the index of everything written down");
        return report;
    };

    for path in files_with_ext(root, &docs, &["md"]) {
        let rel = path.strip_prefix("docs/").unwrap_or(&path);
        if rel == "INDEX.md" {
            continue;
        }
        // A subdirectory with its own index is named as a directory instead.
        if let Some((dir, _)) = rel.rsplit_once('/') {
            if docs.join(dir).join("INDEX.md").is_file() {
                if !index.contains(&format!("{dir}/")) {
                    report.fail(format!(
                        "docs/{dir}/ has its own index and docs/INDEX.md does not name it"
                    ));
                }
                continue;
            }
        }
        if !index.contains(rel) {
            report.fail(format!("{path} is a document docs/INDEX.md does not mention"));
        }
    }

    for line in index.lines() {
        for token in line.split(|c: char| c.is_whitespace() || "()[]<>,;\"'`".contains(c)) {
            let token = token.trim_start_matches("./");
            if token.ends_with(".md") && token != "INDEX.md" && !docs.join(token).is_file() {
                report.fail(format!("docs/INDEX.md names {token}, which is not under docs/"));
            }
        }
    }
    report
}
