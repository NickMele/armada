//! The icon half of the gate: one rule, reading the registry and every glyph
//! `apps/` imports.
//!
//! It sits beside `rules_design` rather than inside it because the subject is
//! different. That rule reads a file for a value that should have been a
//! token; this one reads a file against a registry, and has to parse the
//! registry first.
//!
//! **No `toml` crate**, and the gate keeps no dependencies — so this is a line
//! parser for the one shape `packages/icons/icons.toml` has: table headers,
//! `key = value`, `#` comments, no multi-line strings. That is not general
//! TOML and does not try to be. A line it cannot read is reported as a line it
//! cannot read, which is the honest failure for a hand-authored file: a
//! registry a scan silently skips is worse than no registry.

use std::collections::BTreeMap;
use std::fs;
use std::path::Path;

use crate::{files_with_ext, Report};

const REGISTRY: &str = "packages/icons/icons.toml";

/// The statuses the registry's own header declares. Checked against that list
/// rather than against the values in the file today, because `Retired` is a
/// declared status with no rows yet, and a rule that only knows what it has
/// already seen rejects the first correct use of it.
const STATUSES: &[&str] = &["Specified", "Proposed", "Retired", "Banned"];

/// The keys an entry may carry. An unrecognised key is a typo — the value it
/// holds is read by nobody, which is the silent half of the failure.
const KEYS: &[&str] = &[
    "means",
    "group",
    "size",
    "status",
    "reserved",
    "notes",
    "paired_with",
    "glyphs",
];

/// `lucide-react` exports that are not glyphs. Importing one of these is
/// ordinary and must not be read as an unregistered icon.
const NOT_A_GLYPH: &[&str] = &[
    "Icon",
    "LucideIcon",
    "LucideProps",
    "IconNode",
    "IconNodeChild",
    "createLucideIcon",
    "icons",
    "dynamicIconImports",
];

/// One `[icons.<name>]` table, reduced to what the rule asks of it.
struct Entry {
    line: usize,
    /// Whether `means` is set anywhere — on the table, or on a `usage`
    /// sub-table. `file-cog` carries no top-level meaning because it has two,
    /// one per context, and demanding one on the table would force a summary
    /// that is not true of either.
    means: bool,
    status: Option<String>,
}

/// One glyph named by an import, with where it was named.
struct Use {
    glyph: String,
    /// How the source spelled it — the component name, so the failure names
    /// the token the author will search for rather than only its kebab form.
    written: String,
    path: String,
    line: usize,
}

/// Rule seventeen: every glyph `apps/` imports is in the icon registry, and a
/// banned glyph is imported nowhere.
///
/// The registry decides what a silhouette is permitted to mean: `circle-check`
/// is a Judge verdict and never a completed step; `hourglass` reads *be
/// patient* for the state that most needs to read as *wrong*, so it is banned.
/// None of that survives an engineer reaching into `lucide-react` for the glyph
/// that looks right, and the result is not a broken build — it is a registry
/// that quietly stops describing the UI while still cited as the authority on
/// it. So the scan runs from what `apps/` imports back to the file.
///
/// **The reverse direction is deliberately not checked.** An entry with no use
/// is the expected state — the registry was authored ahead of the surfaces, and
/// most of Bridge does not exist yet. Failing on it would make the file's
/// purpose, which is to decide before building, into a violation.
///
/// **A wholesale `import * as Icons from "lucide-react"` fails on sight**, not
/// because the style is wrong but because the gate cannot see through it: every
/// glyph reached that way is invisible here. Named imports stay checkable.
///
/// The well-formedness half is not redundant with a generator, because there is
/// no generator — this file is authored by hand out of a retired Notion
/// database, and a missing `means` or an undeclared status is a row somebody
/// half-carried across.
pub fn every_glyph_in_use_is_registered(root: &Path) -> Report {
    let mut report = Report::new("every glyph a surface uses is in the icon registry");

    let Ok(text) = fs::read_to_string(root.join(REGISTRY)) else {
        report.fail(format!(
            "{REGISTRY} — the icon registry, which this rule checks against"
        ));
        return report;
    };
    let entries = read_registry(&text, &mut report);

    for u in glyphs_in_use(root, &mut report) {
        match entries.get(&u.glyph) {
            None => report.fail(format!(
                "{}:{} — `{}` (imported as `{}`) has no entry in {REGISTRY}. \
                 A glyph is decided in the registry before it is used",
                u.path, u.line, u.glyph, u.written
            )),
            Some(entry) if entry.status.as_deref() == Some("Banned") => report.fail(format!(
                "{}:{} — `{}` is `status = \"Banned\"` in {REGISTRY}:{}. \
                 Nothing in Armada may use it",
                u.path, u.line, u.glyph, entry.line
            )),
            Some(_) => {}
        }
    }

    // The string-keyed form: `icons["hourglass"]`, or a glyph name in a data
    // table. Only banned names are looked for, because that is a closed set of
    // one — searching for any registry name in any quoted string would flag the
    // registry's own prose the moment a component quoted it.
    let banned: Vec<&String> = entries
        .iter()
        .filter(|(_, e)| e.status.as_deref() == Some("Banned"))
        .map(|(name, _)| name)
        .collect();
    for path in app_sources(root) {
        let Ok(text) = fs::read_to_string(root.join(&path)) else {
            continue;
        };
        for name in &banned {
            for (n, line) in text.lines().enumerate() {
                if line.contains(&format!("\"{name}\"")) || line.contains(&format!("'{name}'")) {
                    report.fail(format!(
                        "{path}:{} — `{name}` is banned in {REGISTRY} and this names it",
                        n + 1
                    ));
                }
            }
        }
    }

    report
}

/// Every `[icons.<name>]` table, reporting anything malformed as it goes.
fn read_registry(text: &str, report: &mut Report) -> BTreeMap<String, Entry> {
    let mut entries: BTreeMap<String, Entry> = BTreeMap::new();
    // The glyph the current section belongs to. A `usage` sub-table contributes
    // to the table above it; a `[conventions.*]` table is not a glyph at all.
    let mut current: Option<String> = None;
    // `paired_with` values and `conventions.*.glyphs` members, checked once the
    // whole map is built — a forward reference is legal, a dangling one is not.
    let mut references: Vec<(usize, String, String)> = Vec::new();

    for (n, raw) in text.lines().enumerate() {
        let line = raw.trim();
        let ln = n + 1;
        if line.is_empty() || line.starts_with('#') {
            continue;
        }

        if line.starts_with('[') {
            current = None;
            match section(line) {
                Some(Section::Icon(name)) => {
                    check_glyph_name(&name, ln, report);
                    if let Some(first) = entries.get(&name) {
                        report.fail(format!(
                            "{REGISTRY}:{ln} — `{name}` already has a table at line {}. \
                             A second one overwrites the first; a glyph used twice \
                             carries the second context as a `usage` sub-table",
                            first.line
                        ));
                    } else {
                        entries.insert(
                            name.clone(),
                            Entry {
                                line: ln,
                                means: false,
                                status: None,
                            },
                        );
                    }
                    current = Some(name);
                }
                Some(Section::Usage(name)) => {
                    if !entries.contains_key(&name) {
                        report.fail(format!(
                            "{REGISTRY}:{ln} — a `usage` sub-table for `{name}`, \
                             which has no `[icons.{name}]` table above it"
                        ));
                    }
                    current = Some(name);
                }
                Some(Section::Conventions) => {}
                None => report.fail(format!(
                    "{REGISTRY}:{ln} — `{line}` is not a table this registry defines. \
                     It has `[icons.<glyph>]`, `[[icons.<glyph>.usage]]` and `[conventions.<name>]`"
                )),
            }
            continue;
        }

        let Some((key, value)) = line.split_once(" = ") else {
            report.fail(format!(
                "{REGISTRY}:{ln} — `{line}` is neither a table header nor `key = value`"
            ));
            continue;
        };
        let key = key.trim();
        if !KEYS.contains(&key) {
            report.fail(format!(
                "{REGISTRY}:{ln} — `{key}` is not a key this registry defines"
            ));
            continue;
        }
        let value = unquote(value.trim());

        match key {
            "means" => {
                if value.is_empty() {
                    report.fail(format!("{REGISTRY}:{ln} — `means` is empty"));
                } else if let Some(entry) = current.as_ref().and_then(|c| entries.get_mut(c)) {
                    entry.means = true;
                }
            }
            "status" => {
                if !STATUSES.contains(&value.as_str()) {
                    report.fail(format!(
                        "{REGISTRY}:{ln} — `status = \"{value}\"` is outside the declared set: {}",
                        STATUSES.join(", ")
                    ));
                } else if let Some(entry) = current.as_ref().and_then(|c| entries.get_mut(c)) {
                    // A `usage` sub-table may restate the status; the table's
                    // own line is the one that decides, so it is not overwritten.
                    entry.status.get_or_insert(value.clone());
                }
            }
            "paired_with" => references.push((ln, "paired_with".into(), value)),
            "glyphs" => {
                for name in array(&value) {
                    references.push((ln, "conventions.glyphs".into(), name));
                }
            }
            _ => {}
        }
    }

    for (name, entry) in &entries {
        if !entry.means {
            report.fail(format!(
                "{REGISTRY}:{} — `{name}` has no `means`, on the table or on a `usage` sub-table. \
                 A glyph with no recorded meaning is not a registry entry",
                entry.line
            ));
        }
        if entry.status.is_none() {
            report.fail(format!(
                "{REGISTRY}:{} — `{name}` has no `status`",
                entry.line
            ));
        }
    }
    for (ln, what, name) in references {
        if !entries.contains_key(&name) {
            report.fail(format!(
                "{REGISTRY}:{ln} — `{what}` names `{name}`, which has no entry in this file"
            ));
        }
    }

    entries
}

enum Section {
    Icon(String),
    Usage(String),
    /// Not a glyph — a rule that names several at once. Its own name is not
    /// checked, because nothing else in the repository resolves it.
    Conventions,
}

fn section(line: &str) -> Option<Section> {
    if let Some(rest) = line
        .strip_prefix("[[icons.")
        .and_then(|r| r.strip_suffix(".usage]]"))
    {
        return Some(Section::Usage(rest.to_string()));
    }
    if let Some(rest) = line
        .strip_prefix("[icons.")
        .and_then(|r| r.strip_suffix(']'))
    {
        return Some(Section::Icon(rest.to_string()));
    }
    if line.starts_with("[conventions.") && line.ends_with(']') {
        return Some(Section::Conventions);
    }
    None
}

/// Whether a name is shaped like a lucide glyph: lowercase kebab-case, digits
/// allowed inside. This cannot know whether lucide ships it — that needs the
/// package, and the gate runs on a checkout with nothing installed. It catches
/// the case the shape does catch: a PascalCase component name or a prose
/// fragment carried into the key by hand.
fn check_glyph_name(name: &str, line: usize, report: &mut Report) {
    let shaped = !name.is_empty()
        && name.starts_with(|c: char| c.is_ascii_lowercase())
        && name.ends_with(|c: char| c.is_ascii_alphanumeric())
        && !name.contains("--")
        && name
            .chars()
            .all(|c| c.is_ascii_lowercase() || c.is_ascii_digit() || c == '-');
    if !shaped {
        report.fail(format!(
            "{REGISTRY}:{line} — `{name}` is not shaped like a lucide glyph name, \
             which is lowercase kebab-case"
        ));
    }
}

fn unquote(value: &str) -> String {
    let value = value.trim();
    value
        .strip_prefix('"')
        .and_then(|v| v.strip_suffix('"'))
        .unwrap_or(value)
        .to_string()
}

/// The members of `["a", "b"]`. The registry's arrays hold glyph names, which
/// contain no commas, so splitting on one is enough.
fn array(value: &str) -> Vec<String> {
    value
        .trim_start_matches('[')
        .trim_end_matches(']')
        .split(',')
        .map(|part| unquote(part.trim()))
        .filter(|part| !part.is_empty())
        .collect()
}

/// Everywhere a glyph can be imported.
///
/// `packages/` is here as well as `apps/`, because a shared surface component
/// is exactly the kind of thing that lands there and imports an icon — and a
/// rule that watches only the app would let the registry go stale in the one
/// place two surfaces share.
fn app_sources(root: &Path) -> Vec<String> {
    let mut out: Vec<String> = Vec::new();
    for dir in ["apps", "packages"] {
        out.extend(files_with_ext(
            root,
            &root.join(dir),
            &["ts", "tsx", "js", "jsx"],
        ));
    }
    out
}

/// Every glyph named by a `lucide-react` import under `apps/`.
fn glyphs_in_use(root: &Path, report: &mut Report) -> Vec<Use> {
    const MODULE: &str = "lucide-react";
    let mut found = Vec::new();

    for path in app_sources(root) {
        let Ok(text) = fs::read_to_string(root.join(&path)) else {
            continue;
        };
        for (i, _) in text.match_indices(MODULE) {
            let before = &text[..i];
            // The module has to be quoted. `lucide-react-native` and the word
            // in a comment are both excluded by this.
            if !(before.ends_with('"') || before.ends_with('\'')) {
                continue;
            }
            let after = &text[i + MODULE.len()..];
            let line = 1 + before.matches('\n').count();

            // A deep import names its glyph in the path:
            // `lucide-react/dist/esm/icons/hourglass`.
            if let Some(sub) = after.strip_prefix('/') {
                let sub: String = sub
                    .chars()
                    .take_while(|c| *c != '"' && *c != '\'')
                    .collect();
                let leaf = sub.rsplit('/').next().unwrap_or_default();
                let leaf = leaf.split('.').next().unwrap_or_default();
                if !leaf.is_empty() {
                    found.push(Use {
                        glyph: leaf.to_string(),
                        written: leaf.to_string(),
                        path: path.clone(),
                        line,
                    });
                }
                continue;
            }
            if !(after.starts_with('"') || after.starts_with('\'')) {
                continue;
            }

            // Walk back to the statement that imports it. A `;` in between
            // means the nearest keyword belongs to an earlier statement.
            let start = [before.rfind("import"), before.rfind("export")]
                .into_iter()
                .flatten()
                .max();
            let Some(start) = start else { continue };
            let stmt = &before[start..];
            if stmt.contains(';') {
                continue;
            }
            if stmt
                .trim_start_matches(|c: char| c.is_alphabetic())
                .trim_start()
                .starts_with("type ")
            {
                continue;
            }

            let braces = stmt.find('{').zip(stmt.rfind('}'));
            let Some((open, close)) = braces.filter(|(o, c)| o < c) else {
                report.fail(format!(
                    "{path}:{line} — this imports `{MODULE}` wholesale. \
                     The gate cannot tell which glyphs that reaches, so it cannot check them \
                     against {REGISTRY}. Import the glyphs by name"
                ));
                continue;
            };
            for spec in stmt[open + 1..close].split(',') {
                let spec = spec.trim();
                let Some(written) = spec.split_whitespace().next() else {
                    continue;
                };
                // `type Foo`, `Activity as ActivityIcon` — the first word is
                // the export unless the first word is `type`.
                let written = if written == "type" { continue } else { written };
                if written.is_empty() || NOT_A_GLYPH.contains(&written) {
                    continue;
                }
                if !written.starts_with(|c: char| c.is_ascii_uppercase()) {
                    continue;
                }
                found.push(Use {
                    glyph: kebab(written),
                    written: written.to_string(),
                    path: path.clone(),
                    line,
                });
            }
        }
    }
    found
}

/// A lucide component name as its registry key: `ArrowUpToLine` becomes
/// `arrow-up-to-line`, `FolderGit2` becomes `folder-git-2`.
///
/// A handful of lucide names glue a digit to a letter — `Grid2X2` is
/// `grid-2x2`, not `grid-2-x-2`. Neither spelling is in the registry, so both
/// fail; the message names the one this produced, which is close enough to
/// search for.
fn kebab(component: &str) -> String {
    let mut out = String::new();
    let mut prev_digit = false;
    for (i, c) in component.chars().enumerate() {
        let boundary = i > 0 && (c.is_ascii_uppercase() || (c.is_ascii_digit() && !prev_digit));
        if boundary {
            out.push('-');
        }
        out.extend(c.to_lowercase());
        prev_digit = c.is_ascii_digit();
    }
    out
}
