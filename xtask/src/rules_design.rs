//! The design half of the gate: the one rule that reads `apps/` for a value
//! that should have been a token.
//!
//! Split from `rules.rs` at the 500-line line. It sits alone because its
//! subject is different from every other rule there — those read Rust, this
//! reads what a renderer ships.

use std::fs;
use std::path::Path;

use crate::{files_with_ext, Report};

/// Rule twelve: no off-contract design value under `apps/`.
///
/// Two things Tailwind cannot refuse for us. Replacing its scales stops
/// `bg-slate-800` and `h-16` resolving — measured, not assumed — but an
/// arbitrary value like `bg-[#161C23]` compiles clean under any config, and a
/// hex or a `px` literal in a style object never reaches Tailwind at all.
///
/// It lives here rather than in a JS linter because a second enforcement
/// system is a second definition of one rule, and two definitions drift. This
/// is the same grep shape as the vendor-literal ban.
///
/// **The escape hatch is one line in the file**, naming a reason and linking
/// the Armada Question that justifies it:
///
/// ```text
/// // armada-allow-off-contract: the diff viewer needs pixel parity with git
/// // https://app.notion.com/p/<the question row>
/// ```
///
/// An unexplained opt-out is not one: the marker must carry a reason and the
/// link must be on the next line, or the file is still checked.
pub fn no_off_contract_design_value(root: &Path) -> Report {
    let mut report = Report::new("no off-contract design value under apps/");
    const MARKER: &str = "armada-allow-off-contract:";

    let files = files_with_ext(root, &root.join("apps"), &["ts", "tsx", "js", "jsx", "css", "html"]);
    for path in files {
        // The renderer's own stylesheet is where the token files are imported.
        if path.ends_with("styles/index.css") {
            continue;
        }
        let Ok(text) = fs::read_to_string(root.join(&path)) else { continue };
        if opted_out(&text, MARKER) {
            continue;
        }
        for (n, line) in text.lines().enumerate() {
            let code = line.trim_start();
            if code.starts_with("//") || code.starts_with('*') || code.starts_with("/*") {
                continue;
            }
            for what in off_contract(code) {
                report.fail(format!("{path}:{} — {what}", n + 1));
            }
        }
    }
    report
}

/// An opt-out counts only when the marker carries a reason and the line under
/// it carries the link. Either half alone is an unexplained opt-out.
fn opted_out(text: &str, marker: &str) -> bool {
    let lines: Vec<&str> = text.lines().collect();
    lines.iter().enumerate().any(|(i, line)| {
        let Some((_, reason)) = line.split_once(marker) else { return false };
        if reason.trim().len() < 12 {
            return false;
        }
        lines.get(i + 1).is_some_and(|next| next.contains("notion.com/"))
    })
}

/// Everything off-contract on one line. All of them, not the first: a line
/// carrying three violations is three things to fix, and reporting one at a
/// time turns a single edit into three gate runs.
fn off_contract(code: &str) -> Vec<String> {
    let mut found = Vec::new();
    // A Tailwind arbitrary value: `bg-[#161C23]`, `h-[84px]`, `text-[13px]`.
    // A `[` after a utility-shaped token is the whole signature.
    let bytes = code.as_bytes();
    for (i, _) in code.match_indices('[') {
        let before = &code[..i];
        let tail: String = before
            .chars()
            .rev()
            .take_while(|c| c.is_ascii_alphanumeric() || *c == '-')
            .collect();
        if tail.len() < 2 || !tail.contains('-') {
            continue;
        }
        // `foo[0]` is an index, not a utility; a utility's bracket opens a value.
        let inside: String = code[i + 1..].chars().take_while(|c| *c != ']').collect();
        if inside.is_empty() || inside.chars().all(|c| c.is_ascii_digit()) {
            continue;
        }
        let utility: String = tail.chars().rev().collect();
        found.push(format!(
            "`{utility}[{inside}]` is an arbitrary value — spell it with a token"
        ));
    }
    let _ = bytes;

    // What an arbitrary value contained is already reported as that arbitrary
    // value; reporting its hex again turns one violation into two lines.
    let mut rest_of_line = String::new();
    let mut depth = 0usize;
    for c in code.chars() {
        match c {
            '[' => depth += 1,
            ']' => depth = depth.saturating_sub(1),
            _ if depth == 0 => rest_of_line.push(c),
            _ => {}
        }
    }
    let code = rest_of_line.as_str();

    // A hex colour or a px literal in a style object.
    for rest in code.split('#').skip(1) {
        let hex: String = rest.chars().take_while(|c| c.is_ascii_hexdigit()).collect();
        if (hex.len() == 3 || hex.len() == 6 || hex.len() == 8) && !code.contains("://") {
            found.push(format!("`#{hex}` is a colour literal — spell it with a token"));
        }
    }
    for (i, _) in code.match_indices("px") {
        let before: String = code[..i].chars().rev().take_while(|c| c.is_ascii_digit()).collect();
        if !before.is_empty() && !code[i + 2..].starts_with(|c: char| c.is_ascii_alphanumeric()) {
            let n: String = before.chars().rev().collect();
            found.push(format!("`{n}px` is a length literal — spell it with a token"));
        }
    }
    found
}
