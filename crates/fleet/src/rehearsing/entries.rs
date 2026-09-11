//! What the run sheet lists, from what the Job froze, and the one entry a run
//! names. **A name resolves here or nowhere**, so nothing a caller sends
//! reaches a process as argv.

use checks_runner::Narrowed;
use config::Manifest;
use core_model::{Job, Narrowing, Prerequisite, ResolvedCheck};

use super::Unrehearsable;

/// One Check or Command, resolved: everything a run of it needs.
#[derive(Clone, Debug, PartialEq, Eq)]
pub(crate) struct Entry {
    pub(crate) name: String,
    pub(crate) run: String,
    pub(crate) expect_exit_code: i64,
    pub(crate) requires: Vec<Prerequisite>,
    pub(crate) narrow: Option<Narrowing>,
    pub(crate) destructive: bool,
    pub(crate) frozen: bool,
}

/// The sheet's three groups.
pub(crate) struct Listed {
    setup: Vec<Entry>,
    checks: Vec<Entry>,
    commands: Vec<Entry>,
}

/// What the Job froze, and what Fleet's Manifest holds for everything it did
/// not.
///
/// **`has_snapshot` is the whole of `#650`'s change to this function.** Where
/// a Job carries a Manifest snapshot, `manifest` **is** that snapshot,
/// already parsed, and every entry it declares is the Job's own — so the
/// sheet is `declared(manifest)` with nothing left to merge and nothing left
/// unfrozen. Where it does not — a Job created before the migration —
/// `manifest` is Fleet's live one, the way every Job used to be read, and
/// what follows is that reading, unchanged: **a Check the workflow names is
/// the Job's own** even there, because the gate judges against that, so the
/// sheet lists it rather than the file.
pub(crate) fn frozen(job: &Job, manifest: &Manifest, has_snapshot: bool) -> Listed {
    if has_snapshot {
        let mut listed = declared(manifest);
        for entry in listed
            .setup
            .iter_mut()
            .chain(listed.checks.iter_mut())
            .chain(listed.commands.iter_mut())
        {
            entry.frozen = true;
        }
        return listed;
    }
    let held = manifest;
    let mut froze: Vec<Entry> = Vec::new();
    for step in job.workflow().steps() {
        for check in step.checks() {
            let ResolvedCheck::ManifestCheck {
                name,
                run,
                expect_exit_code,
                requires,
                narrow,
                ..
            } = check
            else {
                continue;
            };
            if froze.iter().any(|entry| &entry.name == name) {
                continue;
            }
            froze.push(Entry {
                name: name.clone(),
                run: run.clone(),
                expect_exit_code: *expect_exit_code,
                requires: requires.clone(),
                narrow: narrow.clone(),
                destructive: false,
                frozen: true,
            });
        }
    }
    // A Command a frozen Check requires runs as the Job froze it.
    let prerequisites: Vec<Prerequisite> = froze
        .iter()
        .flat_map(|entry| entry.requires.iter().cloned())
        .collect();
    let mut listed = declared(held);
    for entry in listed.checks.iter_mut() {
        if let Some(at) = froze.iter().position(|kept| kept.name == entry.name) {
            *entry = froze.remove(at);
        }
    }
    // Frozen Checks the Manifest no longer declares are still the Job's.
    listed.checks.extend(froze);
    for entry in listed.setup.iter_mut().chain(listed.commands.iter_mut()) {
        if let Some(needed) = prerequisites.iter().find(|p| p.name() == entry.name) {
            entry.run = needed.run().to_string();
            entry.frozen = true;
        }
    }
    for needed in &prerequisites {
        let listed_already = listed
            .setup
            .iter()
            .chain(listed.commands.iter())
            .any(|entry| entry.name == needed.name());
        if !listed_already {
            listed
                .commands
                .push(command(needed.name(), needed.run(), false, true));
        }
    }
    listed
}

/// What a Manifest declares, none of it frozen. The worktree's own file, or
/// the base [`frozen`] lays the Job's entries over.
pub(crate) fn declared(theirs: &Manifest) -> Listed {
    let destructive = |name: &str| theirs.command(name).is_some_and(|c| c.is_destructive());
    let setup: Vec<Entry> = theirs
        .prepared_by()
        .iter()
        .map(|prep| command(prep.name(), prep.run(), destructive(prep.name()), false))
        .collect();
    let checks = theirs
        .checks_as_written()
        .iter()
        .filter_map(|name| {
            let check = theirs.check(name)?;
            Some(Entry {
                name: name.clone(),
                run: check.run().to_string(),
                expect_exit_code: check.expect_exit_code(),
                requires: check.requires().to_vec(),
                narrow: check.narrow().cloned(),
                destructive: false,
                frozen: false,
            })
        })
        .collect();
    let commands = theirs
        .command_names()
        .into_iter()
        .filter(|name| !setup.iter().any(|entry| &entry.name == name))
        .filter_map(|name| {
            let found = theirs.command(&name)?;
            Some(command(&name, found.run(), found.is_destructive(), false))
        })
        .collect();
    Listed {
        setup,
        checks,
        commands,
    }
}

fn command(name: &str, run: &str, destructive: bool, frozen: bool) -> Entry {
    Entry {
        name: name.to_string(),
        run: run.to_string(),
        expect_exit_code: 0,
        requires: Vec::new(),
        narrow: None,
        destructive,
        frozen,
    }
}

impl Listed {
    /// The entry a run names, or the refusal that lists what there is.
    pub(crate) fn named(&self, name: &str) -> Result<Entry, Unrehearsable> {
        self.every()
            .find(|entry| entry.name == name)
            .cloned()
            .ok_or_else(|| Unrehearsable::NotDeclared {
                name: name.to_string(),
                declared: self.every().map(|entry| entry.name.clone()).collect(),
            })
    }

    /// Whether two listings would run the same things, **whatever each says
    /// about where it came from**.
    pub(crate) fn same_as(&self, other: &Listed) -> bool {
        let plain = |entries: &[Entry]| -> Vec<Entry> {
            entries
                .iter()
                .cloned()
                .map(|mut entry| {
                    entry.frozen = false;
                    entry
                })
                .collect()
        };
        plain(&self.setup) == plain(&other.setup)
            && plain(&self.checks) == plain(&other.checks)
            && plain(&self.commands) == plain(&other.commands)
    }

    /// The three groups as the sheet draws them, narrowed against `changed`.
    #[allow(clippy::type_complexity)]
    pub(crate) fn sheet(
        &self,
        changed: &[String],
    ) -> (Vec<ipc::RunEntry>, Vec<ipc::RunEntry>, Vec<ipc::RunEntry>) {
        let drawn = |entries: &[Entry]| entries.iter().map(|e| wired(e, changed)).collect();
        (
            drawn(&self.setup),
            drawn(&self.checks),
            drawn(&self.commands),
        )
    }

    fn every(&self) -> impl Iterator<Item = &Entry> {
        self.setup
            .iter()
            .chain(self.checks.iter())
            .chain(self.commands.iter())
    }
}

fn wired(entry: &Entry, changed: &[String]) -> ipc::RunEntry {
    let narrow_run = match (&entry.narrow, changed.is_empty()) {
        (Some(narrow), false) => match checks_runner::narrowed(Some(narrow), changed) {
            Narrowed::To(command) => Some(command),
            Narrowed::Nothing | Narrowed::Whole => None,
        },
        _ => None,
    };
    ipc::RunEntry {
        name: entry.name.clone(),
        run: entry.run.clone(),
        narrows: entry.narrow.is_some(),
        narrow_run,
        requires: entry
            .requires
            .iter()
            .map(|needed| needed.name().to_string())
            .collect(),
        expect_exit_code: entry.expect_exit_code,
        destructive: entry.destructive,
        frozen: entry.frozen,
    }
}
