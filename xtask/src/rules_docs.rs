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

    for (path, line, found) in docs::near_miss_headings(root) {
        report.fail(format!(
            "{path}:{line} heads its questions `{found}` — the walk reads `## Open questions` \
             exactly, so these are invisible and the check above passes while they sit there"
        ));
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
            // A named document resolves under docs/ or at the repo root —
            // the index legitimately points at ARCHITECTURE.md and README.md,
            // and refusing that would push the index into naming them
            // vaguely rather than by their filename.
            if token.ends_with(".md")
                && token != "INDEX.md"
                && !docs.join(token).is_file()
                && !root.join(token).is_file()
            {
                report.fail(format!(
                    "docs/INDEX.md names {token}, which is neither under docs/ nor at the root"
                ));
            }
        }
    }
    report
}

/// Rule sixteen: nothing in the repository links to the design workspace.
///
/// The workspace is one person's, and this repository is public. A link into it
/// publishes an address to something nobody outside can open — so it leaks and
/// is useless at the same time, which is how seventy-nine issues ended up
/// carrying dead links into a private account before anyone looked.
///
/// It is a grep, like the vendor-literal ban, and for the same reason: the
/// alternative is remembering. The needle is assembled rather than written so
/// this rule does not fail on itself.
///
/// **Red is the expected state while the contracts are still moving.** Each
/// line it names is a document that has not been carried across yet, and the
/// list shortens as they land.
pub fn nothing_links_to_the_design_workspace(root: &Path) -> Report {
    let mut report = Report::new("nothing links to the design workspace");
    let needle = concat!("notion", ".");
    const EXTS: &[&str] = &["md", "rs", "toml", "ts", "tsx", "js", "json", "py", "sh", "css", "yaml", "yml"];

    let mut looked = Vec::new();
    for dir in ["docs", "crates", "apps", "packages", "xtask", ".claude"] {
        looked.extend(files_with_ext(root, &root.join(dir), EXTS));
    }
    looked.push("CLAUDE.md".to_string());

    for path in looked {
        let Ok(text) = fs::read_to_string(root.join(&path)) else { continue };
        for (n, line) in text.lines().enumerate() {
            let at = line.find(needle);
            let Some(at) = at else { continue };
            // The word on its own is the tool being named, which is allowed.
            // What is banned is an address.
            let tail = &line[at + needle.len()..];
            if tail.starts_with("so/") || tail.starts_with("com/") {
                report.fail(format!(
                    "{path}:{} links to the design workspace — say what it is, not where",
                    n + 1
                ));
            }
        }
    }
    report
}

/// Rule eighteen: every repository path a document names still exists.
///
/// A pointer is the whole value of a document that refuses to restate what it
/// points at. When the file moves, the pointer does not follow it, and the
/// reader is left holding a path that resolves to nothing — which reads
/// exactly like a path that resolves to something until they go and look.
///
/// `docs/CLAUDE.md` reserves the bare path for v1 — `git show v1-final:<path>`
/// — so a path is dead only when it is in neither tree. The v1 tree is read
/// from the tag rather than inferred from a naming convention, because v1 and
/// v2 share top-level directories and a first-segment heuristic reads
/// `crates/core/src/fleet/drone.rs` as a typo when it is a citation.
///
/// Without the tag the rule cannot tell the two apart, and warns rather than
/// failing: a shallow checkout is not a defect in the prose.
///
/// This does not catch a pointer that names a directory where it meant a file.
/// `crates/core-model/` is a real directory, and eleven pointers once said it
/// where they meant one of seven files inside it.
pub fn every_path_a_document_names_exists(root: &Path) -> Report {
    let mut report = Report::new("every repository path a document names exists");

    const ROOTS: [&str; 6] = ["docs/", "crates/", "apps/", "packages/", "xtask/", ".claude/"];

    let v1 = std::process::Command::new("git")
        .args(["ls-tree", "-r", "--name-only", "v1-final"])
        .current_dir(root)
        .output();
    let v1: std::collections::BTreeSet<String> = match &v1 {
        Ok(out) if out.status.success() => String::from_utf8_lossy(&out.stdout)
            .lines()
            .map(str::to_string)
            .collect(),
        _ => {
            report.warn("the v1-final tag is not in this checkout — v1 citations are not checked");
            Default::default()
        }
    };

    let mut paths = files_with_ext(root, &root.join("docs"), &["md"]);
    for name in ["README.md", "ARCHITECTURE.md", "AGENTS.md", "CONTRIBUTING.md", "SECURITY.md"] {
        if root.join(name).is_file() {
            paths.insert(name.to_string());
        }
    }

    for rel in paths {
        // v1's own paths are quoted verbatim there, and are meant to be dead.
        if rel.starts_with("docs/v1-learnings/") {
            continue;
        }
        let Ok(text) = fs::read_to_string(root.join(&rel)) else {
            continue;
        };
        for (n, line) in text.lines().enumerate() {
            for span in line.split('`').skip(1).step_by(2) {
                let named = span.trim_end_matches(['.', ',', ':', ';']);
                if !ROOTS.iter().any(|r| named.starts_with(r)) {
                    continue;
                }
                // A glob or a placeholder names a shape, not a file.
                if named.contains(['*', '<', '{', ' ']) {
                    continue;
                }
                if root.join(named).exists() {
                    continue;
                }
                // A v1 citation resolves at the tag, not in the working tree.
                if v1.contains(named) || v1.iter().any(|p| p.starts_with(&format!("{named}/"))) {
                    continue;
                }
                report.fail(format!(
                    "{rel}:{} names {named}, which is in neither this tree nor v1-final",
                    n + 1
                ));
            }
        }
    }
    report
}
