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
//! # Against the main checkout, and not a Job's worktree
//!
//! The surface this answers is the project's Manifest, read before anything
//! has been dispatched. A Job's own run sheet rehearses the Manifest that Job
//! froze, in that Job's tree, and is a different question with a different
//! answer — `docs/journeys/run-and-edit-a-manifest.md`, under *Running one
//! inside a Job*.
//!
//! # It runs nothing
//!
//! No process is started here and none may be. Drift runs on opening a surface,
//! beside a dry-run that runs a real test suite behind its own button, and the
//! only reason the two can sit together is that this half is `stat` calls and
//! file reads. A `which`-style probe would cost a spawn per declaration and
//! answer a different question anyway — whether the *machine* has a tool, not
//! whether the *repository* has what the tool would look for.
//!
//! # Paths, and then the tool
//!
//! [`checks_runner::split`] gives the words the runner would actually execute —
//! the same splitter, so drift and the runner cannot come to disagree about
//! what a line invokes. Two readings are then taken of them.
//!
//! **Paths.** A word holding a `/` that survives [`named_path`]'s exclusions is
//! looked for in the checkout. `bash scripts/ci.sh` is judged on `scripts/ci.sh`
//! and not on `bash`, which is the shape `#719` was filed about.
//!
//! **The tool's own files.** `pnpm typecheck` names a script in a
//! `package.json`, and `cargo xtask` an alias in `.cargo/config.toml` — a
//! quarter of this repository's own `run` lines, which read as clean with
//! nothing checked until these were followed. [`following`] holds which tools
//! are known and [`declared`] reads their files.
//!
//! # What was not followed is said on the row
//!
//! A word naming something this read did not resolve — an unknown tool, a
//! package manager's own subcommand, a file that would not scan — is recorded
//! with its reason, **whatever the verdict**, and never counted as missing.
//! `gone` means the repository no longer has this, which can only be said
//! having looked and not found. The owner's decision on the question the first
//! version of this module left open: follow the tools it can, and say plainly
//! on every row what it did not.

use std::path::Path;

use config::Manifest;
use ipc::{Declaration, Drift, ManifestDrift, Unfollowed};

pub(crate) use declared::Repository;

pub(crate) mod declared;
mod following;

/// Read one Manifest against the checkout it was loaded from.
///
/// **A free function over a `&Manifest` and a path, not a method on `Fleet`.**
/// Nothing here needs a store, a roster or a clock, and the seam is what lets
/// the cases below drive it with a parsed file and a temporary directory —
/// `checks_runner`'s own argument for being a crate, one scale down.
pub(crate) fn drift(manifest: &Manifest, checkout: &Path) -> ManifestDrift {
    let mut declarations = Vec::new();
    let repo = &mut Repository::at(checkout);

    // Checks first and in declaration order, because that is the order the
    // file writes them and the order they run in. `check_names` is the same
    // set sorted, which is for a message rather than for a read like this.
    for name in manifest.checks_as_written() {
        let Some(check) = manifest.check(name) else {
            continue;
        };
        declarations.push(declared("checks", name, "run", check.run(), repo));
    }

    // Commands and servers are both written under `commands:` and the file
    // spells them one way, so the section here is what a person would search
    // for rather than which registry `config` sorted them into.
    for name in manifest.command_names() {
        let Some(command) = manifest.command(&name) else {
            continue;
        };
        declarations.push(declared("commands", &name, "run", command.run(), repo));
    }
    for name in manifest.server_names() {
        let Some(server) = manifest.server(&name) else {
            continue;
        };
        // Its own row each, and in the order the file reads: what builds it,
        // what starts it, what says it is ready. Each can go missing on its
        // own, and one row for the three would carry a verdict about none.
        if let Some(run) = server.run() {
            declarations.push(declared("commands", &name, "run", run, repo));
        }
        declarations.push(declared("commands", &name, "serve", server.serve(), repo));
        if let Some(ready) = server.ready() {
            declarations.push(declared("commands", &name, "ready", ready, repo));
        }
    }

    ManifestDrift {
        path: manifest.path().display().to_string(),
        checkout: checkout.display().to_string(),
        declarations,
    }
}

/// One line, judged.
fn declared(section: &str, name: &str, key: &str, run: &str, repo: &mut Repository) -> Declaration {
    let (drift, unfollowed) = judged(run, repo);
    Declaration {
        section: section.to_string(),
        name: name.to_string(),
        key: key.to_string(),
        run: run.to_string(),
        drift,
        unfollowed,
    }
}

/// What one line's words came to, as they are read.
///
/// **Three lists, and the verdict is made from two of them at the end.** A
/// word is found, missing, or not followed, and only the first two bear on
/// `gone` — which is what keeps a tool this read does not know out of the
/// verdict.
#[derive(Default)]
pub(crate) struct Found {
    checked: u32,
    missing: Vec<String>,
    unfollowed: Vec<Unfollowed>,
}

impl Found {
    fn found(&mut self) {
        self.checked += 1;
    }

    fn missing(&mut self, what: String) {
        if !self.missing.contains(&what) {
            self.missing.push(what);
        }
    }

    fn unfollowed(&mut self, word: &str, why: &str) {
        self.unfollowed.push(Unfollowed {
            word: word.to_string(),
            why: why.to_string(),
        });
    }
}

/// The verdict for one `run` line, and what it did not follow.
///
/// **`None` from the splitter is a line with no program in it**, which the
/// parser does not admit — an empty `run` is refused at load. It is answered
/// here as nothing checked rather than as a fault, because a read that could
/// refuse would be a second opinion about a file `config` already accepted.
pub(crate) fn judged(run: &str, repo: &mut Repository) -> (Drift, Vec<Unfollowed>) {
    let Some((program, arguments)) = checks_runner::split(run) else {
        return (Drift::Current { checked: 0 }, Vec::new());
    };

    let mut found = Found::default();
    for word in std::iter::once(&program).chain(arguments.iter()) {
        let Some(path) = named_path(word) else {
            continue;
        };
        match repo.checkout().join(path).exists() {
            true => found.found(),
            false => found.missing(path.to_string()),
        }
    }
    following::follow(&program, &arguments, repo, &mut found);

    let drift = match found.missing.is_empty() {
        true => Drift::Current {
            checked: found.checked,
        },
        false => Drift::Gone {
            missing: found.missing,
        },
    };
    (drift, found.unfollowed)
}

/// The path one word names inside the repository, or [`None`] where it names
/// none.
///
/// Every exclusion below is a word that holds a `/` and is still not a path in
/// this checkout. Each one is a false `gone` avoided, and a false `gone` is
/// worse than a missed one here: the missed one is the state this whole module
/// is replacing, and the false one teaches a person that the amber means
/// nothing.
pub(crate) fn named_path(word: &str) -> Option<&str> {
    // A flag. `--manifest-path=a/b` carries a path and is not one; the value
    // spelled as its own word — `-C packages/web` — is, and arrives here as
    // that word.
    if word.starts_with('-') {
        return None;
    }
    // A scoped package name. `@armada/desktop` holds a `/` and names a package
    // by its `package.json` name, not a directory — and the first version of
    // this rule called it missing on every `--filter` line.
    if word.starts_with('@') {
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
