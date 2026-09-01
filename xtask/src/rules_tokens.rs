//! The token half of the gate: what `verify-tokens` generates, and the one
//! thing Tailwind compiles that a custom property cannot survive.
//!
//! Split from `rules.rs` because the subject is the token pipeline rather than
//! Rust source — and because the second rule here answers a defect the first
//! could never see. `the_tokens_generate_what_is_checked_in` proves the
//! checked-in files are what the CSS says; it has no opinion about whether what
//! the CSS says works in a browser. It did not, for eight months: both
//! breakpoints were emitted under `@theme inline`, so a responsive variant
//! compiled to `@media (width >= var(--layout-breakpoint-narrow))` and every
//! browser dropped the rule. The gate was green throughout.

use std::fs;
use std::path::Path;

use crate::{files_with_ext, Report};

/// Rule eleven: the checked-in token outputs say what the CSS says.
///
/// `verify-tokens` is the task that regenerates them, and this rule is what
/// makes the gate notice. A step that ships work no rule watches has quietly
/// narrowed the gate's coverage — the gate only knows what somebody wrote a
/// rule for, and going green means every subject a rule names has landed.
pub fn the_tokens_generate_what_is_checked_in(root: &Path) -> Report {
    let mut report = Report::new("the design tokens generate what is checked in");
    match crate::tokens::outputs(root) {
        Err(why) => report.fail(why),
        Ok(outputs) => {
            for (rel, want) in outputs {
                match fs::read_to_string(root.join(&rel)) {
                    Ok(have) if have == want => {}
                    Ok(_) => report.fail(format!(
                        "{rel} — stale or hand-edited. \
                         Run `cargo xtask verify-tokens --write` and commit what it emits"
                    )),
                    Err(_) => report.fail(format!("{rel} — not generated yet")),
                }
            }
        }
    }
    report
}

/// The generated theme. The only file that may declare a `--breakpoint-*`.
const THEME: &str = "packages/tokens/tokens.theme.css";

/// Rule twenty-eight: no media query resolves through a custom property.
///
/// **`var()` is not legal in a media feature value.** A browser does not
/// substitute and then evaluate — it fails to parse the condition and drops the
/// whole rule, silently, in the direction that looks like success: the class
/// lands in the markup, the stylesheet compiles, and nothing ever matches.
///
/// Two shapes, because the same defect arrives by two routes:
///
/// - **A hand-written `@media` reading a token.** The obvious one.
/// - **A `--breakpoint-*` whose value is a `var()`.** Not obvious at all, and
///   the one that shipped. Tailwind writes that value straight into the media
///   feature, so the declaration *is* the media query — which is why this rule
///   reads the emitted theme rather than only the stylesheets around it.
///
/// A breakpoint therefore carries its literal, in a plain `@theme` rather than
/// the `inline` block the rest of the theme lives in. `xtask/src/tokens.rs`
/// spells that as `Slot::Breakpoint`, and the emitter refuses an aliased value
/// before it can reach the file this rule reads.
pub fn no_media_query_resolves_through_a_custom_property(root: &Path) -> Report {
    let mut report = Report::new("no media query resolves through a custom property");
    const EXT: [&str; 1] = ["css"];

    let mut files = files_with_ext(root, &root.join("apps"), &EXT);
    files.extend(files_with_ext(root, &root.join("packages"), &EXT));
    if !root.join(THEME).is_file() {
        report.fail(format!(
            "{THEME} — the emitted theme, where a breakpoint lands"
        ));
    }

    for path in files {
        let Ok(text) = fs::read_to_string(root.join(&path)) else {
            continue;
        };
        // Comments first. Several of these files argue in prose about exactly
        // this defect, and a rule that reads `@media (width >= …) var()` out of
        // the sentence explaining it reports the explanation as the fault.
        let code = strip_comments(&text);

        for (line, prelude) in preludes(&code) {
            if prelude.contains("var(") {
                report.fail(format!(
                    "{path}:{line} — `@media{prelude}` reads a custom property. \
                     A media feature value cannot be a var(): the browser drops the \
                     whole rule rather than resolving it. Write the literal"
                ));
            }
        }

        for (line, name, value) in breakpoints(&code) {
            if value.contains("var(") {
                report.fail(format!(
                    "{path}:{line} — `{name}: {value}` aliases a token, and Tailwind \
                     writes that value into `@media (width >= …)` where var() is not \
                     legal. Emit the literal — see Slot::Breakpoint in xtask/src/tokens.rs"
                ));
            }
        }
    }
    report
}

/// Every `@media` prelude in the file: the line it opens on, and the text
/// between `@media` and the `{` that follows it.
fn preludes(code: &str) -> Vec<(usize, String)> {
    let mut out = Vec::new();
    for (at, _) in code.match_indices("@media") {
        let line = code[..at].lines().count().max(1);
        let rest = &code[at + "@media".len()..];
        let end = rest.find('{').unwrap_or(rest.len());
        out.push((line, rest[..end].trim_end().to_string()));
    }
    out
}

/// Every `--breakpoint-…` declaration: its line, its name and its value. The
/// `--breakpoint-*: initial` namespace reset is a declaration of nothing and is
/// not one of these.
fn breakpoints(code: &str) -> Vec<(usize, String, String)> {
    let mut out = Vec::new();
    for (n, raw) in code.lines().enumerate() {
        for decl in raw.split(';') {
            let decl = decl.trim();
            let Some(rest) = decl.strip_prefix("--breakpoint-") else {
                continue;
            };
            let Some((key, value)) = rest.split_once(':') else {
                continue;
            };
            if key == "*" {
                continue;
            }
            out.push((
                n + 1,
                format!("--breakpoint-{key}"),
                value.trim().to_string(),
            ));
        }
    }
    out
}

/// The file with every `/* … */` blanked out, newlines kept so a line number
/// still means what it says.
fn strip_comments(text: &str) -> String {
    let mut out = String::with_capacity(text.len());
    let mut rest = text;
    while let Some(start) = rest.find("/*") {
        out.push_str(&rest[..start]);
        let body = &rest[start + 2..];
        let end = body.find("*/");
        let (skipped, after) = match end {
            Some(at) => (&body[..at], &body[at + 2..]),
            None => (body, ""),
        };
        for _ in skipped.matches('\n') {
            out.push('\n');
        }
        rest = after;
        if end.is_none() {
            break;
        }
    }
    out.push_str(rest);
    out
}

#[cfg(test)]
mod tests;
