//! The design half of the gate: the one rule that reads what a renderer ships,
//! looking for a value that should have been a token.
//!
//! Split from `rules.rs` at the 500-line line. It sits alone because its
//! subject is different from every other rule there — those read Rust, this
//! reads what a renderer ships.

use std::fs;
use std::path::Path;

use crate::{files_with_ext, Report};

/// Rule twelve: no off-contract design value in anything a renderer ships.
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
/// The cost is accepted deliberately: a linter would say so in the editor,
/// where it is cheapest to fix, and the gate only says so at the gate. What
/// buys that back is that there is one place to read the rule and one place it
/// can be wrong. A hook once enforced a ceiling the gate did not, and the
/// result was a document compressed to satisfy a rule nobody could find.
///
/// **The escape hatch is two lines in the file** — a reason, then a citation
/// of the open question that justifies it:
///
/// ```text
/// // armada-allow-off-contract: the diff viewer needs pixel parity with git
/// // [diff-gutter-parity]
/// ```
///
/// An unexplained opt-out is not one: the marker must carry a reason and the
/// citation must be on the next line, or the file is still checked.
///
/// **Two citation forms are accepted.** A `[slug]` naming a question in some
/// document's `## Open questions` section is the one to write — it is the only
/// form the gate can follow, so a question that gets answered breaks its
/// citations on purpose. A `docs/` path is accepted for a question that has a
/// whole document to itself.
///
/// This first accepted a link into the design workspace and nothing else, which
/// would have failed silently the first time a contract moved into the repo: a
/// correct opt-out citing a repo path would read as unexplained, and the file
/// would go back to being checked with nobody told why. That form is gone now
/// for a second reason — rule sixteen.
pub fn no_off_contract_design_value(root: &Path) -> Report {
    let mut report = Report::new("no off-contract design value in what a renderer ships");
    const MARKER: &str = "armada-allow-off-contract:";
    const EXT: [&str; 6] = ["ts", "tsx", "js", "jsx", "css", "html"];

    // `apps/` was the whole surface while Bridge held every component. The
    // components moved to a package of their own, and a rule that reads only
    // `apps/` would have stopped watching the files it exists for — silently,
    // and while still reporting green.
    let mut files = files_with_ext(root, &root.join("apps"), &EXT);
    files.extend(files_with_ext(root, &root.join("packages").join("components"), &EXT));
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
/// it cites something. Either half alone is an unexplained opt-out.
fn opted_out(text: &str, marker: &str) -> bool {
    let lines: Vec<&str> = text.lines().collect();
    lines.iter().enumerate().any(|(i, line)| {
        let Some((_, reason)) = line.split_once(marker) else { return false };
        if reason.trim().len() < 12 {
            return false;
        }
        lines.get(i + 1).is_some_and(|next| cites_a_question(next))
    })
}

/// Whether a line names an open question: a `[slug]`, or a path under `docs/`.
/// A bare word is not a citation — it has to be findable.
fn cites_a_question(line: &str) -> bool {
    if line.contains("docs/") {
        return true;
    }
    let Some(open) = line.find('[') else { return false };
    let Some(close) = line[open..].find(']') else { return false };
    let slug = &line[open + 1..open + close];
    !slug.is_empty()
        && slug.len() <= 60
        && slug.chars().all(|c| c.is_ascii_lowercase() || c.is_ascii_digit() || c == '-')
}

/// Whether a bracket's contents test an attribute rather than name a value —
/// `data-variant="primary"`, `aria-expanded='true'`, `type=checkbox`. A
/// Tailwind arbitrary value carries a length, a colour, a `var()` or an
/// arbitrary property written with a colon, and none of those shapes is an
/// identifier followed by `=`.
fn is_attribute_test(inside: &str) -> bool {
    let Some((name, _)) = inside.split_once('=') else { return false };
    let name = name.trim_end_matches(['~', '|', '^', '$', '*']);
    !name.is_empty()
        && name.chars().next().is_some_and(|c| c.is_ascii_alphabetic())
        && name.chars().all(|c| c.is_ascii_alphanumeric() || c == '-' || c == '_' || c == ':')
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
        // A CSS attribute selector wears the same brackets. Two signals tell
        // them apart, and both are needed: `[data-variant="primary"]` tests an
        // attribute against a value, which no arbitrary value does; and a
        // selector's bracket hangs off a class, an id, another selector or a
        // nesting `&`. The preceding character alone is not enough — a real
        // `class="bg-[#161C23]"` sits behind a quote too, so quotes are not on
        // the list and that violation is still caught.
        if is_attribute_test(&inside) {
            continue;
        }
        let before_tail = before[..before.len() - tail.len()].chars().next_back();
        if matches!(before_tail, Some('.' | '#' | ']' | ')' | '&' | '*')) {
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
