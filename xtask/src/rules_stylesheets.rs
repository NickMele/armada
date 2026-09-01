//! The sheet the app loads carries every component's stylesheet.
//!
//! `PhaseStrip.css` was written and never appended to `index.css`. Nothing
//! loaded it, `.armada-phases` fell to the browser's `display: block`, and the
//! strip drew as a vertical stack. Storybook built, the gallery built, the
//! typecheck, the suite, `verify-docs` and `verify-tokens` were all green — it
//! was found by launching the app and reading a computed style. `RunTree.css`,
//! `StepStory.css` and `ActivityLog.css` were in the same state.
//!
//! **Nothing else sees it.** A story imports its own component's stylesheet, so
//! the component is styled in Storybook and bare in the app; the gallery reads
//! the tree rather than this file, so it is styled there too. `index.css` is
//! the only place the app's sheet is assembled, and a component missing from it
//! renders with its class names on the elements and no rules behind them, which
//! reads as a component drawn wrong rather than one never registered.
//!
//! **Both directions**, as the actions contract is held both ways: an import
//! naming a file that is not there is the other half of a rename, and reporting
//! only the missing half sends the reader to append a line rather than to fix
//! the one already written. Vite refuses a dangling import when it builds, so
//! that half is the earlier signal rather than the only one.

use std::fs;
use std::path::Path;

use crate::{files_with_ext, Report};

/// The component library. Every `.css` under it is a component's stylesheet.
const ROOT: &str = "packages/components/src";

/// The one file that assembles them.
const SHEET: &str = "packages/components/src/index.css";

/// Where a component's directory layout and this registration are written down.
const SKILL: &str = ".claude/skills/armada-components/SKILL.md";

/// One `@import` in the sheet: the line it is on, and the path it names,
/// resolved against the sheet's own directory.
struct Import {
    line: usize,
    path: String,
}

/// Every stylesheet under the library is imported by `index.css`, and every
/// import resolves to a file.
///
/// No list of trees and no assumption about depth — the subject is every `.css`
/// the walk finds, so a stylesheet in a tree nobody has invented yet is covered
/// on the day it lands. That is the failure this is guarding: `gallery/build.mjs`
/// carries a hand-written list of the three trees it inlines, and a tree left
/// off it renders unstyled with nothing said.
pub fn every_stylesheet_reaches_the_sheet_the_app_loads(root: &Path) -> Report {
    let mut report = Report::new("every component stylesheet is imported by index.css");

    let src = root.join(ROOT);
    if !src.is_dir() {
        report.fail(format!("{ROOT} — the component library the app draws from"));
        return report;
    }
    let Ok(text) = fs::read_to_string(root.join(SHEET)) else {
        report.fail(format!(
            "{SHEET} — the one file that assembles the app's stylesheet. \
             Without it every component under {ROOT}/ renders unstyled. {SKILL}"
        ));
        return report;
    };

    let imports = read_imports(&text, &mut report);
    let append_at = text.lines().count() + 1;

    for import in &imports {
        if !src.join(&import.path).is_file() {
            report.fail(format!(
                "{SHEET}:{} — `{}` is imported here and is not a file under {ROOT}/. \
                 A rename leaves both halves: this line, and the stylesheet under its \
                 new name that nothing imports",
                import.line, import.path
            ));
        }
        if let Some(first) = imports.iter().find(|other| other.path == import.path) {
            if first.line < import.line {
                report.fail(format!(
                    "{SHEET}:{} — `{}` is already imported at line {}. This list is \
                     append-only, so two appends of one line is what a merge produces",
                    import.line, import.path, first.line
                ));
            }
        }
    }

    for path in files_with_ext(root, &src, &["css"]) {
        let Some(rel) = path.strip_prefix(&format!("{ROOT}/")) else {
            continue;
        };
        if path == SHEET || imports.iter().any(|i| i.path == rel) {
            continue;
        }
        report.fail(format!(
            "{path} — a stylesheet {SHEET} does not import, so nothing loads it. \
             Append `@import \"./{rel}\";` at line {append_at}. Until then the \
             component draws with its class names and no rules behind them, which \
             reads as drawn wrong rather than never registered. {SKILL}"
        ));
    }

    report
}

/// Every relative `@import` in the sheet, with the line it sits on.
///
/// Comments are stripped first, newline for newline. A commented-out import is
/// the one way this file can look like it registers a stylesheet and not do it,
/// and it is exactly what somebody reaches for while bisecting a style.
///
/// An import that is not relative — a package specifier, a URL — names no file
/// in this tree and is not this rule's subject. An `@import` the parser cannot
/// read a path out of is, because the gate cannot tell what it registers.
fn read_imports(text: &str, report: &mut Report) -> Vec<Import> {
    let mut found = Vec::new();
    for (n, line) in strip_comments(text).lines().enumerate() {
        let Some(rest) = line.trim_start().strip_prefix("@import") else {
            continue;
        };
        let Some(spec) = quoted(rest) else {
            report.fail(format!(
                "{SHEET}:{} — an `@import` with no quoted path. The gate reads this \
                 file to know what the app loads, and cannot read this line",
                n + 1
            ));
            continue;
        };
        if !spec.starts_with("./") && !spec.starts_with("../") {
            continue;
        }
        found.push(Import {
            line: n + 1,
            path: normalize(&spec),
        });
    }
    found
}

/// `text` with every `/* … */` blanked and every line kept, so a line number
/// still means what it meant before.
fn strip_comments(text: &str) -> String {
    let mut out = String::with_capacity(text.len());
    let mut rest = text;
    while let Some(open) = rest.find("/*") {
        out.push_str(&rest[..open]);
        let after = &rest[open + 2..];
        let close = after.find("*/");
        let (comment, tail) = match close {
            Some(at) => (&after[..at], &after[at + 2..]),
            None => (after, ""),
        };
        for _ in comment.matches('\n') {
            out.push('\n');
        }
        rest = tail;
    }
    out.push_str(rest);
    out
}

/// The first quoted string in `s`, single or double quoted. `url(` and the
/// whitespace around it fall out of this rather than being parsed, because the
/// quotes are the only part that carries the path.
fn quoted(s: &str) -> Option<String> {
    let at = s.find(['"', '\''])?;
    let quote = s.as_bytes()[at] as char;
    let rest = &s[at + 1..];
    let (inner, _) = rest.split_once(quote)?;
    Some(inner.to_string())
}

/// A path relative to the sheet, reduced to a path relative to the library:
/// `./compositions/X/X.css` is `compositions/X/X.css`. Lexical, because the
/// segments are checked in as text and a symlink under the library would be a
/// separate defect.
///
/// A path that climbs out of the library keeps its `../` and so matches no
/// stylesheet the walk found, which is what the report then says.
fn normalize(spec: &str) -> String {
    let mut parts: Vec<&str> = Vec::new();
    for part in spec.split('/') {
        match part {
            "." | "" => {}
            ".." if matches!(parts.last(), Some(&last) if last != "..") => {
                parts.pop();
            }
            other => parts.push(other),
        }
    }
    parts.join("/")
}

#[cfg(test)]
mod tests;
