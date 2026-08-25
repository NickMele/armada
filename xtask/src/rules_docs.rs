//! The documentation half of the gate.
//!
//! `docs/` is exempt from every other rule here — `SOURCE_ROOTS` is crates,
//! apps, packages and xtask, on the reasoning that prose is not source. That
//! held while `docs/` carried spike records. It stops holding the moment a
//! contract lands there, because a contract is read by whoever writes the code
//! that obeys it, and nothing was checking the most-read documents in the repo.

use std::fs;
use std::path::Path;

use crate::{docs, Report};

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
