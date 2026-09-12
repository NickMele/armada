//! Whether the repository still has what `armada.yml` names.
//!
//! **Not [`adrift`](mod@crate::adrift)**, which is this crate's refusal enum
//! and shares only a root with the word. This is the read behind
//! `get_manifest_drift`, and [`configured`](mod@crate::configured) is its
//! neighbour: that answers what the file declares, this answers whether the
//! repository still has it.
//!
//! # Why a read of the repository exists beside a read of the file
//!
//! Fleet already re-reads `armada.yml` when it changes and reports what it
//! could not adopt — `crate::daemon::rereading`. That is a read of the file,
//! and a `run` line naming a script somebody deleted parses perfectly, is
//! adopted without complaint, and says nothing. The first thing that notices is
//! a Job failing on it, which is the silence this module ends.
//!
//! # It runs nothing
//!
//! No process is started here and none may be. Drift runs on opening a surface,
//! beside a dry-run that runs a real test suite behind its own button, and the
//! only reason the two can sit together is that this half is a handful of
//! `stat` calls. A `which`-style probe would make the free read cost a spawn
//! per declaration and would answer a different question anyway — whether the
//! *machine* has a tool, not whether the *repository* has a file.
//!
//! # What a word has to look like before it is looked for
//!
//! [`checks_runner::split`] gives the words the runner would actually execute —
//! the same splitter, so drift and the runner cannot come to disagree about
//! what a line invokes. Of those words, one names a repository path when it
//! holds a `/` and survives [`named_path`]'s exclusions. Everything else is a
//! tool on `PATH`, a flag, or a word this read has no way to resolve, and each
//! of those is left alone rather than guessed at: the verdict `gone` means *the
//! repository no longer has this*, which can only be said having looked and not
//! found. Absence of anything to look for is not evidence of absence, so it
//! reads `current` with `checked` at zero — and the wire carries that number so
//! a surface can say which rows its clean list is actually about.
//!
//! **A `/` in the word, and the whole word, is the rule.** `bash scripts/ci.sh`
//! is judged on `scripts/ci.sh` and not on `bash`, which is the shape the issue
//! was filed about: the program is an interpreter that is always installed and
//! the thing that went missing is its argument.
//!
//! # What is deliberately not resolved
//!
//! A `pnpm test` naming a `package.json` script, and a `cargo xtask` naming a
//! workspace member, each name something real that this read does not follow.
//! Following them means teaching this module one build tool's vocabulary at a
//! time, and a rule that knows `pnpm` and not `bun` reports a clean row for the
//! repository it does not know. Both come back `current` with `checked` at
//! zero, which is the true answer to what was asked rather than a false one to
//! what was not.

use std::path::Path;

use config::Manifest;
use ipc::{Declaration, Drift, ManifestDrift};

/// Read one Manifest against the checkout it was loaded from.
///
/// **A free function over a `&Manifest` and a path, not a method on `Fleet`.**
/// Nothing here needs a store, a roster or a clock, and the seam is what lets
/// the cases below drive it with a parsed file and a temporary directory —
/// `checks_runner`'s own argument for being a crate, one scale down.
pub(crate) fn drift(manifest: &Manifest, checkout: &Path) -> ManifestDrift {
    let mut declarations = Vec::new();

    // Checks first and in declaration order, because that is the order the
    // file writes them and the order they run in. `check_names` is the same
    // set sorted, which is for a message rather than for a read like this.
    for name in manifest.checks_as_written() {
        let Some(check) = manifest.check(name) else {
            continue;
        };
        declarations.push(declared("checks", name, "run", check.run(), checkout));
    }

    // Commands and servers are both written under `commands:` and the file
    // spells them one way, so the section here is what a person would search
    // for rather than which registry `config` sorted them into.
    for name in manifest.command_names() {
        let Some(command) = manifest.command(&name) else {
            continue;
        };
        declarations.push(declared("commands", &name, "run", command.run(), checkout));
    }
    for name in manifest.server_names() {
        let Some(server) = manifest.server(&name) else {
            continue;
        };
        // Its own row each, and in the order the file reads: what builds it,
        // what starts it, what says it is ready. Each can go missing on its
        // own, and one row for the three would carry a verdict about none.
        if let Some(run) = server.run() {
            declarations.push(declared("commands", &name, "run", run, checkout));
        }
        declarations.push(declared(
            "commands",
            &name,
            "serve",
            server.serve(),
            checkout,
        ));
        if let Some(ready) = server.ready() {
            declarations.push(declared("commands", &name, "ready", ready, checkout));
        }
    }

    ManifestDrift {
        path: manifest.path().display().to_string(),
        checkout: checkout.display().to_string(),
        declarations,
    }
}

/// One line, judged.
fn declared(section: &str, name: &str, key: &str, run: &str, checkout: &Path) -> Declaration {
    Declaration {
        section: section.to_string(),
        name: name.to_string(),
        key: key.to_string(),
        run: run.to_string(),
        drift: judged(run, checkout),
    }
}

/// The verdict for one `run` line.
///
/// **`None` from the splitter is a line with no program in it**, which the
/// parser does not admit — an empty `run` is refused at load. It is answered
/// here as nothing checked rather than as a fault, because a read that could
/// refuse would be a second opinion about a file `config` already accepted.
pub(crate) fn judged(run: &str, checkout: &Path) -> Drift {
    let Some((program, arguments)) = checks_runner::split(run) else {
        return Drift::Current { checked: 0 };
    };

    let mut checked = 0u32;
    let mut missing = Vec::new();
    for word in std::iter::once(&program).chain(arguments.iter()) {
        let Some(path) = named_path(word) else {
            continue;
        };
        checked += 1;
        if !checkout.join(path).exists() {
            missing.push(path.to_string());
        }
    }

    match missing.is_empty() {
        true => Drift::Current { checked },
        false => Drift::Gone { missing },
    }
}

/// The path one word names inside the repository, or [`None`] where it names
/// none.
///
/// Every exclusion below is a word that holds a `/` and is still not a path in
/// this checkout. Each one is a false `gone` avoided, and a false `gone` is
/// worse than a missed one here: the missed one is the state this whole module
/// is replacing, and the false one teaches a person that the amber means
/// nothing.
fn named_path(word: &str) -> Option<&str> {
    // A flag. `--manifest-path=a/b` carries a path and is not one; the value
    // spelled as its own word — `-C packages/web` — is, and arrives here as
    // that word.
    if word.starts_with('-') {
        return None;
    }
    // An address. Nothing on this machine is at `https://…/x`.
    if word.contains("://") {
        return None;
    }
    // A pattern, not a path. There is no shell, so nothing expands these and
    // the literal string is what would be passed — but `src/**/*.rs` is a set
    // a tool matches for itself, and looking for a file of that name finds
    // nothing however healthy the repository is.
    if word.contains(['*', '?', '[', '{', '$']) {
        return None;
    }
    // Outside the repository. An absolute path is the machine's and a `..`
    // climbs out of the checkout, and drift answers about the repository —
    // `#719`, and the journey's own wording.
    let path = Path::new(word);
    if path.is_absolute() || path.components().any(|c| c.as_os_str() == "..") {
        return None;
    }
    // The one thing that makes a word a path at all. A bare `cargo` is looked
    // up on `PATH` by the operating system and is a tool, not a file here.
    match word.contains('/') {
        true => Some(word),
        false => None,
    }
}
