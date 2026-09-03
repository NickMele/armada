//! The generated vocabulary says what `crates/core-model/domain/` says.
//!
//! The registries are gated and their generated TypeScript was not, so a
//! registry change nobody regenerated against left a file that compiles clean
//! and renders the words the Rust side has already moved off. Every other gate
//! stayed green, because none of them reads it.
//!
//! **The comparison is against the registries, never between the two emitted
//! copies.** `apps/desktop/.../vocabulary.ts` and
//! `packages/components/.../vocabulary.ts` are one generator writing one string
//! twice, so they cannot disagree and proving they agree proves nothing.
//!
//! **The rule refuses; it does not rewrite** — the argument
//! [`crate::rules_protocol::version`] makes for the same codegen. A gate that
//! silently runs the generator means nobody learns the step exists.

// Why this one runs the generator and rule eleven does not.
//
// `verify-tokens` regenerates in Rust because the emitter *is* Rust. This
// emitter is Node, and a second emitter in Rust is two implementations that
// have to agree byte for byte — the drift this rule exists to catch, moved
// into the gate itself. So the gate runs the one generator there is, with
// `--emit`, which writes nothing and prints what it would have written.
//
// That makes `node` a prerequisite of the gate, beside the `git` and `cargo`
// the rules around it already spawn. The generator imports nothing outside
// `node:`, so it needs no `node_modules` and the gate still runs on a checkout
// with nothing built.

use std::fs;
use std::path::Path;
use std::process::Command;

use crate::Report;

/// The generator, and the command a person runs to fix what this reports.
const CODEGEN: &str = "apps/desktop/codegen/vocabulary.mjs";
const WRITE: &str = "pnpm --filter @armada/desktop codegen";

/// The outputs that must be in a run's emission.
///
/// The rule compares everything `--emit` prints, so a third output is gated the
/// day it is added. This list is the other direction: an output that stops
/// being emitted stops being compared, silently, and the vocabulary is the copy
/// a story reads — losing it is exactly how a hand-typed verb got into a
/// fixture in the first place.
///
/// **Two files where there were three.** The vocabulary was emitted twice
/// because the dependency ran one way and a story could not reach the app's
/// copy; `@armada/protocol` is a package both depend on, so the duplicate
/// collapsed. The vocabulary itself stayed with the components: a glyph is a
/// React component, and the wire package imports nothing.
/// **`actions.ts` is here because it was a hand transcription first.** 592
/// lines of `actions.toml` retyped into `packages/components/src/`, the third
/// copy of one registry — it went the day the generator learned to emit it, and
/// a generated file nothing compares to its source is the same drift wearing a
/// banner that says it is not.
const REQUIRED: &[&str] = &[
    "packages/components/src/generated/vocabulary.ts",
    "packages/components/src/generated/actions.ts",
    "packages/protocol/src/generated/protocol-version.ts",
];

/// Rule: the generated vocabulary says what the registries say.
pub fn the_generated_vocabulary_says_what_the_registries_say(root: &Path) -> Report {
    let mut report = Report::new("the generated vocabulary says what the registries say");

    let emitted = match emit(root) {
        Ok(emitted) => emitted,
        Err(why) => {
            report.fail(why);
            return report;
        }
    };

    for want in REQUIRED {
        if !emitted.iter().any(|(rel, _)| rel == want) {
            report.fail(format!(
                "{CODEGEN} no longer emits {want}. An output that stops being written stops \
                 being compared, and the file left behind is stale from that moment on"
            ));
        }
    }

    for (rel, want) in &emitted {
        match fs::read_to_string(root.join(rel)) {
            Ok(have) if &have == want => {}
            Ok(_) => report.fail(format!(
                "STALE {rel} — it is not what `{CODEGEN}` emits from \
                 crates/core-model/domain/. Run `{WRITE}` and commit what it writes"
            )),
            Err(_) => report.fail(format!(
                "{rel} — `{CODEGEN}` emits it and it is not checked in. Run `{WRITE}`"
            )),
        }
    }

    report
}

/// Run the generator in the mode that writes nothing, and read back every file
/// it would have written as `(repo-relative path, text)`.
fn emit(root: &Path) -> Result<Vec<(String, String)>, String> {
    let run = Command::new("node")
        .arg(CODEGEN)
        .arg("--emit")
        .current_dir(root)
        .output()
        .map_err(|e| {
            format!("{CODEGEN} — cannot run `node`: {e}. The gate reads what the generator emits")
        })?;

    if !run.status.success() {
        // The generator stops rather than emitting a half-read registry, and
        // what it says is the finding — a status with no `terminal`, a verb
        // with no matching lifecycle. Repeating "codegen failed" would drop it.
        let why = String::from_utf8_lossy(&run.stderr);
        return Err(format!(
            "{CODEGEN} — the generator stopped: {}",
            why.trim().lines().next_back().unwrap_or("no reason given")
        ));
    }

    let stdout = String::from_utf8(run.stdout)
        .map_err(|_| format!("{CODEGEN} — emitted bytes that are not UTF-8"))?;
    parse(&stdout)
}

/// `path\0text\0` repeated. A trailing empty fragment after the last NUL is
/// the terminator, not a fourth field; anything else left over means the
/// generator's half of this format changed and the gate is reading pairs that
/// are not pairs.
fn parse(stdout: &str) -> Result<Vec<(String, String)>, String> {
    let mut fields: Vec<&str> = stdout.split('\0').collect();
    if fields.last() == Some(&"") {
        fields.pop();
    }
    if !fields.len().is_multiple_of(2) {
        return Err(format!(
            "{CODEGEN} --emit — {} NUL-separated fields, and they come in pairs of path and text",
            fields.len()
        ));
    }
    if fields.is_empty() {
        return Err(format!(
            "{CODEGEN} --emit — emitted nothing. A generator with no outputs gates no files"
        ));
    }
    Ok(fields
        .chunks(2)
        .map(|pair| (pair[0].to_string(), pair[1].to_string()))
        .collect())
}

#[cfg(test)]
mod tests;
