//! The tool roster and the `--allowedTools` list hold the same set.
//!
//! `crates/ipc/src/mcp/` is the authority on which tools exist, and
//! `tools::named` is the closed set a call is answered against.
//! `crates/adapters/src/harness.rs` builds the argument list a Drone is
//! launched with, by hand, as constants. **Nothing joined the two**, and on 30
//! Aug 2026 `ask_question` shipped in the roster and was absent from the
//! allowlist. No test, no gate and no type saw it.
//!
//! **Both directions, and they are different faults.** A tool in the roster and
//! not the allowlist is a Drone denied what Fleet advertises — silently,
//! because `--permission-mode dontAsk` refuses without prompting. A name in the
//! allowlist and not the roster is argv granting a tool that does not exist.
//!
//! **`dispatch_job` is granted, not given, and that is not a mismatch.** The
//! question here is whether a name can *ever* be rendered, so [`rendered_from`]
//! reads the whole of `allowlist`'s body and not which branch a name is in.
//!
//! The two spellings are joined in [`prefixed`] and nowhere else here; the
//! server half is read from the source rather than written down, in [`server`].
//!
//! **No `syn` and no `toml`**, for the reason [`crate::rules_enums`] has
//! neither: the gate keeps no dependencies. This reads constant declarations
//! and two function bodies as text.

use std::collections::{BTreeMap, BTreeSet};
use std::fs;
use std::path::Path;

use crate::Report;

/// Every file a tool's name may be declared in. A tool is a name, a field
/// list, an argument type, a parser and a schema, and `ask.rs` and
/// `dispatch.rs` each hold all five of one tool — so the constants are spread
/// across the directory rather than gathered in `tools.rs`.
const ROSTER_DIR: &str = "crates/ipc/src/mcp";

/// The closed set of tool names, and the file it is written in.
const ROSTER: &str = "crates/ipc/src/mcp/tools.rs";

/// The argument list a Drone is launched with.
const ALLOWLIST: &str = "crates/adapters/src/harness.rs";

/// The two files that each declare the name Armada's MCP server is registered
/// under. Neither reads the other, so this rule compares them: the join below
/// is only correct while they agree.
const SERVER_IN_IPC: (&str, &str) = ("crates/ipc/src/mcp/mod.rs", "SERVER");
const SERVER_IN_ADAPTERS: (&str, &str) = ("crates/adapters/src/mcp.rs", "EVIDENCE_SERVER");

/// How a client spells a tool served by a named MCP server. **The one place
/// this rule joins the two halves.** `harness.rs` is the one place the product
/// joins them, which is what makes the constants there comparable at all.
fn prefixed(server: &str, tool: &str) -> String {
    format!("mcp__{server}__{tool}")
}

/// What an allowlist entry looks like before the tool name. Used to tell a
/// mis-servered constant from one that is not a tool name at all.
fn prefix(server: &str) -> String {
    prefixed(server, "")
}

/// Every tool the roster serves is in the allowlist, and every tool the
/// allowlist names is in the roster.
pub fn the_roster_and_the_allowlist_hold_the_same_set(root: &Path) -> Report {
    let mut report = Report::new("the tool roster and the allowlist hold the same set");

    let Some(server) = server(root, &mut report) else {
        return report;
    };
    let Some(roster) = roster(root, &mut report) else {
        return report;
    };
    let Some(rendered) = rendered(root, &server, &mut report) else {
        return report;
    };
    compare(&roster, &rendered, &server, &mut report);
    report
}

/// Both directions, and each finding names the file to change.
fn compare(
    roster: &BTreeSet<String>,
    rendered: &BTreeSet<String>,
    server: &str,
    report: &mut Report,
) {
    for tool in roster {
        if !rendered.contains(tool) {
            report.fail(format!(
                "{ALLOWLIST} — `{tool}` is a tool {ROSTER} serves and `allowlist` renders no \
                 `{}` for. A Drone is offered it and refused it without being told, which \
                 reads as a Drone that went quiet",
                prefixed(server, tool)
            ));
        }
    }
    for tool in rendered {
        if !roster.contains(tool) {
            report.fail(format!(
                "{ALLOWLIST} — `allowlist` renders `{}`, and `named` in {ROSTER} answers no \
                 tool by that name. The argument list grants something that does not exist",
                prefixed(server, tool)
            ));
        }
    }
}

/// The name the server is registered under, from the adapter that writes the
/// configuration document a client reads.
///
/// `ipc`'s own copy is compared against it rather than ignored: the two are
/// equal today and nothing makes them stay that way, and a disagreement makes
/// every allowlist entry wrong while both files still read correctly on their
/// own. `None` when either is unreadable — the join has no value, so there is
/// nothing to compare and saying so once beats reporting every tool twice.
fn server(root: &Path, report: &mut Report) -> Option<String> {
    let (adapters_path, adapters_name) = SERVER_IN_ADAPTERS;
    let (ipc_path, ipc_name) = SERVER_IN_IPC;
    let adapters = declared(root, adapters_path, adapters_name, report)?;
    let ipc = declared(root, ipc_path, ipc_name, report)?;
    if adapters != ipc {
        report.fail(format!(
            "{adapters_path} registers the server as `{adapters}` and {ipc_path} calls it \
             `{ipc}`. Every allowlist entry carries the first and every tool is served under \
             the second, so one of the two names nothing"
        ));
        return None;
    }
    Some(adapters)
}

/// One `&str` constant, by name, from one file.
fn declared(root: &Path, path: &str, name: &str, report: &mut Report) -> Option<String> {
    let Ok(text) = fs::read_to_string(root.join(path)) else {
        report.fail(format!("{path} — the file `{name}` is declared in"));
        return None;
    };
    match str_consts(&text).remove(name) {
        Some(value) => Some(value),
        None => {
            report.fail(format!(
                "{path} — `{name}` is not declared there as a `&str`. It is half of every \
                 tool name a Drone is allowed to call"
            ));
            None
        }
    }
}

/// Every tool name a call may name, bare.
///
/// Read from `named`'s arms rather than from the constants, because a constant
/// is a spelling and `named` is the set: a name `named` does not answer is not
/// callable, and requiring the allowlist to carry it would be wrong. The
/// constants are read too, to resolve each arm's identifier to its literal.
fn roster(root: &Path, report: &mut Report) -> Option<BTreeSet<String>> {
    let Ok(tools) = fs::read_to_string(root.join(ROSTER)) else {
        report.fail(format!("{ROSTER} — the file `named` is written in"));
        return None;
    };
    let mut spellings = BTreeMap::new();
    for path in crate::files_with_ext(root, &root.join(ROSTER_DIR), &["rs"]) {
        if let Ok(text) = fs::read_to_string(root.join(&path)) {
            spellings.extend(str_consts(&text));
        }
    }
    roster_from(&tools, &spellings, report)
}

/// The set half of [`roster`], over text rather than files, so each way it can
/// fail is provable against a source built for it.
fn roster_from(
    tools: &str,
    spellings: &BTreeMap<String, String>,
    report: &mut Report,
) -> Option<BTreeSet<String>> {
    let Some(body) = body(tools, "fn named(") else {
        report.fail(format!(
            "{ROSTER} — `fn named(` is not there. It is the closed set of tool names this \
             rule compares the allowlist against"
        ));
        return None;
    };

    let mut found = BTreeSet::new();
    for line in body.lines() {
        let Some((left, _)) = line.split_once(" => Ok(") else {
            continue;
        };
        let name = left.trim();
        if !is_constant(name) {
            continue;
        }
        match spellings.get(name) {
            Some(tool) => {
                found.insert(tool.clone());
            }
            None => report.fail(format!(
                "{ROSTER} — `named` answers `{name}`, which is no `&str` constant under \
                 {ROSTER_DIR}/. Its spelling cannot be compared with anything"
            )),
        }
    }

    // Not a warning. A comparison with nothing on one side reports ok and
    // proves nothing, which is the failure this whole rule exists to end.
    if found.is_empty() {
        report.fail(format!(
            "{ROSTER} — `named` resolved to no tool at all, so nothing was compared against \
             {ALLOWLIST}"
        ));
        return None;
    }
    Some(found)
}

/// Every tool name `allowlist` can put in argv, bare.
fn rendered(root: &Path, server: &str, report: &mut Report) -> Option<BTreeSet<String>> {
    let Ok(text) = fs::read_to_string(root.join(ALLOWLIST)) else {
        report.fail(format!(
            "{ALLOWLIST} — the file the argument list is built in"
        ));
        return None;
    };
    rendered_from(&text, server, report)
}

/// The set half of [`rendered`], over text for [`roster_from`]'s reason.
///
/// The whole of `allowlist`'s body, both branches: a name rendered only under a
/// grant is still a name a Drone can be given, and telling the two apart is
/// `harness.rs`'s job rather than this rule's.
fn rendered_from(text: &str, server: &str, report: &mut Report) -> Option<BTreeSet<String>> {
    let Some(body) = body(text, "fn allowlist(") else {
        report.fail(format!(
            "{ALLOWLIST} — `fn allowlist(` is not there. It is what puts Armada's own tools \
             in argv"
        ));
        return None;
    };

    let prefix = prefix(server);
    let mut found = BTreeSet::new();
    let mut declared = 0usize;
    for (name, value) in str_consts(text) {
        let Some(tool) = tool_name(&value, &prefix, &name, report) else {
            continue;
        };
        declared += 1;
        if mentions(body, &name) {
            found.insert(tool);
        }
    }

    if declared == 0 {
        report.fail(format!(
            "{ALLOWLIST} — no constant there spells `{prefix}<tool>`, so nothing was compared \
             against {ROSTER}"
        ));
        return None;
    }
    Some(found)
}

/// The bare tool name a constant carries, or nothing if it carries none.
///
/// A value holding `mcp__` under a different server name is reported rather
/// than skipped: it is an entry the client will never match, and it reads
/// correctly on its own line.
fn tool_name(value: &str, prefix: &str, name: &str, report: &mut Report) -> Option<String> {
    if let Some(tool) = value.strip_prefix(prefix) {
        return Some(tool.to_string());
    }
    if value.starts_with("mcp__") {
        report.fail(format!(
            "{ALLOWLIST} — `{name}` is `{value}`, which is not under `{prefix}`. A client \
             matches the whole name, so this one is allowed for nobody"
        ));
    }
    None
}

/// Every `const NAME: &str = "value";` in a file, as `NAME -> value`.
///
/// Any visibility, because the roster's constants are `pub` and the harness's
/// are private. Within one file the compiler forbids a name twice; across the
/// roster's files the later file wins, which is a spelling nobody could have
/// imported unambiguously anyway.
fn str_consts(text: &str) -> BTreeMap<String, String> {
    let mut found = BTreeMap::new();
    for line in text.lines() {
        let Some((before, rest)) = line.split_once("const ") else {
            continue;
        };
        let before = before.trim();
        if !(before.is_empty() || before.starts_with("pub")) {
            continue;
        }
        let Some((name, value)) = rest.split_once(": &str = ") else {
            continue;
        };
        let Some(literal) = value.strip_prefix('"').and_then(|v| v.split('"').next()) else {
            continue;
        };
        if is_constant(name.trim()) {
            found.insert(name.trim().to_string(), literal.to_string());
        }
    }
    found
}

/// A screaming-snake-case identifier — how a constant is spelled, and what
/// tells one from a binding or a pattern on the same line.
fn is_constant(name: &str) -> bool {
    !name.is_empty()
        && name
            .chars()
            .all(|c| c.is_ascii_uppercase() || c.is_ascii_digit() || c == '_')
}

/// One function's body, from its signature to the closing brace in column one.
///
/// `rustfmt` puts every brace inside a function under at least one level of
/// indentation, so a `}` at the start of a line is the end of the function and
/// nothing else.
fn body<'a>(text: &'a str, signature: &str) -> Option<&'a str> {
    let start = text.find(signature)?;
    let rest = &text[start..];
    let end = rest.find("\n}")?;
    Some(&rest[..end])
}

/// Whether a body names an identifier, as a whole word.
///
/// Whole-word rather than substring, because a constant named for a tool is a
/// prefix of nothing today and would be silently absorbed by a longer one
/// tomorrow — which is the class of miss this rule was written for.
fn mentions(body: &str, name: &str) -> bool {
    body.match_indices(name).any(|(at, _)| {
        let before = body[..at].chars().next_back();
        let after = body[at + name.len()..].chars().next();
        !before.is_some_and(is_word) && !after.is_some_and(is_word)
    })
}

fn is_word(c: char) -> bool {
    c.is_alphanumeric() || c == '_'
}

#[cfg(test)]
mod tests;
