//! Every generated vocabulary is read by a surface, or declared as not yet
//! drawn.
//!
//! Four times in one day a word on screen turned out to be one somebody typed
//! into a component while the registry that owns it sat generated and unread —
//! `SILENCE` (#346), `CHANGE_KIND` (#465), the `step_state` glyph (#347), then
//! five at once (#468). Each was found by an agent that happened to need the
//! answer, because nothing else could: the generated file compiles, the rule
//! beside this one proves it matches its registries, and both stay green while
//! nothing imports a line of it.
//!
//! **A map nobody reads is not a half-finished feature, it is a false claim.**
//! `enum-verbs.toml` is cited across the repository as the authority on how a
//! variant reads. Where nothing carries a vocabulary into a surface, editing a
//! row changes no pixel, and the registry is authoritative over nothing while
//! still being read as authoritative.

// Why the reverse direction is checked here and refused in `rules_icons`.
//
// `every_glyph_in_use_is_registered` deliberately does not fail on a registry
// entry with no use, because `packages/icons/icons.toml` is authored ahead of
// the surfaces on purpose — deciding before building is the file's whole job.
//
// A generated module is the opposite case. Nothing decides to emit a vocabulary
// except a name in the generator's own wanted list, and that list is a claim
// that a surface renders it — `apps/desktop/codegen/vocabulary.mjs` says the
// five it added last are "drawn on four surfaces nobody had a word for". An
// emitted map with no reader is that claim failing, from either side: Bridge
// ignoring a registry, or the generator emitting what nobody wanted.

use std::collections::BTreeSet;
use std::fs;
use std::path::Path;

use crate::{files_with_ext, Report};

/// The generated modules whose exports must reach a surface.
///
/// The same three files the `REQUIRED` list beside this one compares against
/// their registries, and deliberately spelled again rather than shared: that
/// list exists so an output that stops being *emitted* is caught, and this one
/// so an output that stops being *read* is. A file could legitimately leave one
/// list and stay on the other, and a single constant would hide the day that
/// happened.
const GENERATED: &[&str] = &[
    "packages/components/src/generated/vocabulary.ts",
    "packages/components/src/generated/actions.ts",
    "packages/protocol/src/generated/protocol-version.ts",
];

/// The word for a directory whose files are output rather than surface. A
/// generated module naming another generated module is one emitter's own
/// business and never a reader.
const GENERATED_DIR: &str = "/generated/";

/// A vocabulary that is emitted and legitimately has no reader yet, with the
/// reason it is ahead of its surface.
///
/// **Per vocabulary, never a switch.** A flag on the rule would take the whole
/// file out of the gate on the day one row got ahead, and the four defects this
/// rule exists to catch would land under it. Naming one symbol leaves every
/// other one checked.
///
/// **The reason is a field and not a comment**, because the gate prints it. A
/// person reading `FAIL` on the next vocabulary should see, in the same output,
/// which ones are deliberately unread and why — otherwise the list becomes the
/// place exemptions go to stop being read.
///
/// An entry here is refused once its symbol gains a reader, so the list shrinks
/// on its own as the surfaces land rather than outliving them.
const AHEAD_OF_ITS_SURFACE: &[(&str, &str)] = &[(
    "GAPS",
    "A report rather than a vocabulary: every variant the registry has no \
     sanctioned verb, glyph or hue for. `reading.ts` and `badge.ts` each derive \
     one variant's gaps at the point they render it, which is what a surface \
     needs; nothing yet draws the whole list, and the surface that would is \
     Doctor's, which is not built",
)];

/// One exported binding of a generated module.
struct Emitted {
    name: String,
    /// Where it is declared, so a failure names the line to read.
    file: &'static str,
    line: usize,
}

/// Rule: every generated vocabulary is read by a surface.
pub fn every_generated_vocabulary_has_a_reader(root: &Path) -> Report {
    let mut report = Report::new("every generated vocabulary has a reader");

    let mut emitted: Vec<Emitted> = Vec::new();
    for file in GENERATED {
        match fs::read_to_string(root.join(file)) {
            Ok(text) => emitted.extend(exports(&text, file)),
            // Not reported as a failure: the rule beside this one already fails
            // on a required output that is not checked in, and two lines about
            // one missing file reads as two problems.
            Err(_) => continue,
        }
    }

    if emitted.is_empty() {
        report.fail(
            "no `export const` in any generated module. A rule that finds nothing to \
             check passes on every repository, including a broken one",
        );
        return report;
    }

    let read_somewhere = imported(root, &mut report);
    for one in &emitted {
        let read = read_somewhere.contains(&one.name);
        let declared = AHEAD_OF_ITS_SURFACE
            .iter()
            .find(|(name, _)| *name == one.name);

        match (read, declared) {
            (false, None) => report.fail(format!(
                "{}:{} — `{}` is generated and nothing outside {GENERATED_DIR} reads it. \
                 The words it carries are the registry's and no surface is asking for them, \
                 so editing a row changes nothing on screen. Read it where the surface draws \
                 the value, or add it to AHEAD_OF_ITS_SURFACE in \
                 xtask/src/rules_vocabulary/readers.rs with the reason it is ahead",
                one.file, one.line, one.name
            )),
            (true, Some((_, why))) => report.fail(format!(
                "{}:{} — `{}` is declared ahead of its surface in \
                 xtask/src/rules_vocabulary/readers.rs and something reads it now. \
                 Delete the entry, whose reason was: {why}",
                one.file, one.line, one.name
            )),
            _ => {}
        }
    }

    for (name, why) in AHEAD_OF_ITS_SURFACE {
        if !emitted.iter().any(|one| &one.name == name) {
            report.fail(format!(
                "xtask/src/rules_vocabulary/readers.rs — `{name}` is declared ahead of its \
                 surface and no generated module exports it. A declaration outliving the \
                 thing it excuses is how the list stops being read. Its reason was: {why}"
            ));
        }
        if why.trim().is_empty() {
            report.fail(format!(
                "xtask/src/rules_vocabulary/readers.rs — `{name}` is declared with no reason. \
                 The gate prints the reason, and an empty one declares nothing"
            ));
        }
    }

    report
}

/// Every `export const NAME` in a generated module, with its line.
///
/// **Consts only, and not the types beside them.** `Rendering`, `Lifecycle`,
/// `Gap` and the four `Action*` aliases are the shapes the consts are made of;
/// a shape used only inside the file it is declared in is documentation of the
/// map that carries it, not a vocabulary a surface failed to draw. Checking
/// them would put five entries in `AHEAD_OF_ITS_SURFACE` that say the same
/// thing, which is how an exemption list stops being read.
fn exports(text: &str, file: &'static str) -> Vec<Emitted> {
    text.lines()
        .enumerate()
        .filter_map(|(n, line)| {
            let rest = line.strip_prefix("export const ")?;
            let name: String = rest
                .chars()
                .take_while(|c| c.is_ascii_alphanumeric() || *c == '_')
                .collect();
            (!name.is_empty()).then(|| Emitted {
                name,
                file,
                line: n + 1,
            })
        })
        .collect()
}

/// Every generated symbol some surface imports by name.
///
/// **A reading is an import, never a mention.** Every instance of this defect
/// came with a comment naming the registry — a sentence saying `enum-verbs.toml`
/// carries no rows for the value being typed in by hand — so a scan that
/// counted words would pass on the exact file it exists to fail. It would also
/// count `const ORIGIN = "manual"` in `Composer.tsx`, which is an unrelated
/// local that happens to share a name with a vocabulary; that one was found by
/// proving the rule fires, and it is why the scan is not a word search.
///
/// **Only `@armada/` and `generated/` specifiers**, which is every route a
/// generated symbol travels: a package outside `packages/components` reaches
/// them through `@armada/components` or `@armada/protocol`, and a file inside
/// one reaches its own `src/generated/`. A local import of a same-named symbol
/// from somewhere else is not a reading of this one.
///
/// **`export … from` does not count and `import` does.** A barrel forwarding a
/// name is not a surface drawing it, and `packages/components/src/index.ts`
/// re-exports the vocabulary with a `*` — which names no symbol and so counts
/// as nothing, which is right.
fn imported(root: &Path, report: &mut Report) -> BTreeSet<String> {
    let mut found = BTreeSet::new();

    for path in sources(root) {
        let Ok(text) = fs::read_to_string(root.join(&path)) else {
            continue;
        };
        for (line, stmt) in statements(&text) {
            let Some(spec) = specifier(&stmt) else {
                continue;
            };
            if !(spec.starts_with("@armada/") || spec.contains("generated")) {
                continue;
            }
            if stmt.contains("* as ") {
                report.fail(format!(
                    "{path}:{line} — this imports `{spec}` wholesale. The gate cannot tell \
                     which vocabularies that reaches, so it cannot tell a read one from an \
                     unread one. Import them by name"
                ));
                continue;
            }
            // A default or side-effect import reaches no named binding, so it
            // is neither a reading nor something the gate is blind to.
            let Some((open, close)) = stmt.find('{').zip(stmt.rfind('}')).filter(|(o, c)| o < c)
            else {
                continue;
            };
            for named in stmt[open + 1..close].split(',') {
                let Some(word) = named.split_whitespace().next() else {
                    continue;
                };
                if word != "type" {
                    found.insert(word.to_string());
                }
            }
        }
    }

    found
}

/// Every `import` statement in a source, as `(first line, one line of text)`.
///
/// **A statement, not a line.** An import long enough to wrap is how a file
/// reaching for four vocabularies at once is written, and a line scan would see
/// four names it could not attribute and a specifier with no names.
///
/// A statement starts at a line whose first word is `import` and runs to the
/// line carrying its specifier. `export … from` is not collected: forwarding a
/// name is not drawing it.
fn statements(text: &str) -> Vec<(usize, String)> {
    let mut out = Vec::new();
    let mut open: Option<(usize, String)> = None;
    for (n, raw) in text.lines().enumerate() {
        let line = raw.trim();
        if open.is_none() {
            if !(line == "import" || line.starts_with("import ") || line.starts_with("import{")) {
                continue;
            }
            open = Some((n + 1, String::new()));
        }
        let Some((_, body)) = open.as_mut() else {
            continue;
        };
        body.push(' ');
        body.push_str(line);
        if line.contains(" from ") || line.ends_with(';') {
            let (at, body) = open.take().expect("open, checked above");
            out.push((at, body));
        }
    }
    out
}

/// The quoted module a statement imports from, or nothing where it names none.
fn specifier(stmt: &str) -> Option<String> {
    let at = stmt.find(" from \"").or_else(|| stmt.find(" from '"))?;
    Some(
        stmt[at + " from \"".len()..]
            .chars()
            .take_while(|c| *c != '"' && *c != '\'')
            .collect(),
    )
}

/// Every TypeScript source that could be a reader: `apps/` and `packages/`,
/// minus anything under a `generated/` directory.
fn sources(root: &Path) -> BTreeSet<String> {
    let mut out = BTreeSet::new();
    for dir in ["apps", "packages"] {
        for path in files_with_ext(root, &root.join(dir), &["ts", "tsx"]) {
            if !path.contains(GENERATED_DIR) {
                out.insert(path);
            }
        }
    }
    out
}

#[cfg(test)]
mod tests;
