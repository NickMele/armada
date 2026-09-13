//! Reading a folder a person adds into a repository Fleet serves, and watching
//! every served `armada.yml` — the composition root's half of
//! `fleet::repositories::Locating`.
//!
//! **`located` has no side effects but the log**, because Fleet may still
//! refuse what it read; the watch, the records directory and the agent door wait for
//! `serving`, which Fleet calls once it has taken the repository.

use std::collections::BTreeMap;
use std::path::{Path, PathBuf};
use std::sync::{Arc, Mutex, OnceLock, PoisonError, Weak};

use adapter_traits::Vcs;
use adapters::{CreateWorktreeError, GitVcs, HeadlessAgent};
use config::{Reloads, Roster};
use fleet::repositories::{Located, Locating, NotLocated, SetUp};
use fleet::{Clock, Fleet, SystemClock};

use crate::setup::{Setup, SetupRefused};
use crate::watching::{self, Watching};

type Served = Fleet<HeadlessAgent, GitVcs, GitVcs>;

/// The reader Fleet is fitted with.
pub struct Locator {
    /// Fleet's data directory, where each repository's records go.
    machine: PathBuf,
    kit: PathBuf,
    roster: Roster,
    /// A Manifest's reload handle, read and not yet served.
    pending: Mutex<BTreeMap<String, Reloads>>,
    /// Held for as long as Fleet runs: dropping one stops its watch.
    watches: Mutex<Vec<Watching>>,
    /// Weak, because Fleet holds this.
    fleet: OnceLock<Weak<Served>>,
}

impl Locator {
    pub fn at(machine: &Path, kit: PathBuf, roster: Roster) -> Locator {
        Locator {
            machine: machine.to_path_buf(),
            kit,
            roster,
            pending: Mutex::new(BTreeMap::new()),
            watches: Mutex::new(Vec::new()),
            fleet: OnceLock::new(),
        }
    }

    /// The Fleet a re-read is handed to. Before any watch starts.
    pub fn bind(&self, fleet: &Arc<Served>) {
        let _ = self.fleet.set(Arc::downgrade(fleet));
    }

    /// Watch one served `armada.yml`, saying each re-read at the console and to
    /// Fleet. A watch that will not start is said, and Fleet serves anyway.
    pub fn watch(&self, reloads: Reloads) {
        let file = reloads.path().to_path_buf();
        let root = file
            .parent()
            .and_then(|dir| dir.canonicalize().ok())
            .map(|dir| dir.to_string_lossy().to_string())
            .unwrap_or_default();
        let fleet = self.fleet.get().cloned();
        let clock = SystemClock::new();
        let said = file.clone();
        match watching::watch(reloads, move |read| {
            if let Some(fleet) = fleet.as_ref().and_then(Weak::upgrade) {
                fleet.reread(&root, watching::reading(&read, &said, clock.now()));
            }
            watching::say(read, &said)
        }) {
            Ok(watching) => {
                println!("watching {} for edits", file.display());
                self.watches
                    .lock()
                    .unwrap_or_else(PoisonError::into_inner)
                    .push(watching);
            }
            Err(why) => eprintln!(
                "{} will not be watched, so an edit to it needs a restart: {why}",
                file.display()
            ),
        }
    }
}

impl Locating for Locator {
    fn located(&self, folder: &Path) -> Result<Located, NotLocated> {
        let not_one = |why: String| {
            // The person's sentence names no library; its codes stay here.
            eprintln!("{} was not read as a repository: {why}", folder.display());
            NotLocated::NotARepository {
                folder: folder.display().to_string(),
                why,
            }
        };
        let root = folder
            .canonicalize()
            .map_err(|why| not_one(why.to_string()))?;
        let root_text = root.to_string_lossy().to_string();
        // Opening the base is the cheapest git read, and it opens only a
        // repository's own root: a subfolder of one is refused here too.
        if let Err(why @ CreateWorktreeError::RepoUnreadable { .. }) =
            GitVcs::new().base_commit(&root_text, None)
        {
            return Err(not_one(why.to_string()));
        }
        let set_up = match Setup::at(&root, &self.kit, &self.roster) {
            Ok(setup) => {
                let left_out = setup
                    .left_out()
                    .iter()
                    .map(fleet::left_out_workflow)
                    .collect();
                let (manifest, workflows, reloads) = setup.into_parts();
                self.pending
                    .lock()
                    .unwrap_or_else(PoisonError::into_inner)
                    .insert(root_text.clone(), reloads);
                Some(SetUp::of(manifest, workflows).leaving_out(left_out))
            }
            Err(SetupRefused::NoManifest { .. }) => None,
            Err(why) => {
                return Err(NotLocated::Refused {
                    root: root_text,
                    why: why.to_string(),
                })
            }
        };
        let records_root = fleet::records::root(&self.machine, &root_text);
        Ok(Located {
            root: root_text,
            records_root: records_root.to_string_lossy().to_string(),
            set_up,
        })
    }

    fn serving(&self, root: &str) {
        let records_root = fleet::records::root(&self.machine, root);
        if let Err(why) = std::fs::create_dir_all(&records_root) {
            eprintln!("{} could not be made: {why}", records_root.display());
        }
        let _ = fleet::records::migrating::migrate(root, &records_root);
        if let Err(why) = crate::mcp::publish(Path::new(root)) {
            eprintln!("an agent standing in {root} will not find Armada: {why}");
        }
        let reloads = self
            .pending
            .lock()
            .unwrap_or_else(PoisonError::into_inner)
            .remove(root);
        if let Some(reloads) = reloads {
            self.watch(reloads);
        }
    }
}
