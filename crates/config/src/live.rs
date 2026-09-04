//! The Manifest keys a person may change under a running Fleet, and the one
//! handle that may change them.
//!
//! **Not the whole file.** `crates/config/settings.toml` files two of
//! `armada.yml`'s keys as `lifetime = "Live"` — `drone.quiet_after_seconds` and
//! `drone.poke_limit` — and files `manifest-checks-init-commands-named-checks`
//! and `manifest-commands-registry` as *Frozen for the Job*. Swapping the whole
//! Manifest would move the second pair too, and every [`ResolvedWorkflow`] was
//! checked against the Checks the file declared at boot: holding a
//! `Setup` is supposed to be proof the two files agree, and a Manifest replaced
//! underneath it would falsify that without anything failing.
//!
//! So the live keys sit behind a cell every clone of one Manifest shares, and
//! the rest of the Manifest is what it was when the daemon read it.
//!
//! **[`Reloads`] is the only thing that can write, and the live keys are all it
//! can write.** Fleet is never handed one, so the crate that holds the Manifest
//! cannot move it — not by convention, but because the method is not on
//! anything Fleet has.
//!
//! [`ResolvedWorkflow`]: crate::ResolvedWorkflow

use std::path::{Path, PathBuf};
use std::sync::{Arc, RwLock};

use crate::error::LoadError;
use crate::manifest::Manifest;

/// The repository's patience with a quiet Drone: the two live keys, together
/// because they are read together and never separately.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub(crate) struct Patience {
    pub(crate) quiet_after_seconds: Option<u32>,
    pub(crate) poke_limit: Option<u32>,
}

/// Where the live keys are kept, shared by every clone of one Manifest.
///
/// **Clone shares rather than copies**, which is the whole point: `Setup` reads
/// the file, hands the Manifest to Fleet by value, and a reload has to reach the
/// one Fleet is holding rather than a copy nobody consults.
#[derive(Debug, Clone)]
pub(crate) struct Cell(Arc<RwLock<Patience>>);

impl Cell {
    pub(crate) fn holding(patience: Patience) -> Cell {
        Cell(Arc::new(RwLock::new(patience)))
    }

    /// **A poisoned lock is read through rather than unwrapped.** What it
    /// guards is two `Option<u32>`, so a panic elsewhere cannot have left it
    /// half-written — and a Fleet that goes down because a lock was poisoned by
    /// an unrelated panic is exactly the failure this whole module refuses.
    pub(crate) fn read(&self) -> Patience {
        *self
            .0
            .read()
            .unwrap_or_else(std::sync::PoisonError::into_inner)
    }

    /// Write, and hand back what was there. For [`Cell::read`]'s reason a
    /// poisoned lock is written through.
    fn replace(&self, patience: Patience) -> Patience {
        let mut held = self
            .0
            .write()
            .unwrap_or_else(std::sync::PoisonError::into_inner);
        std::mem::replace(&mut held, patience)
    }
}

/// Which live key moved. Named rather than spelled at each call site, so a
/// message about a reload and the file it came from use one vocabulary.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum LiveKey {
    QuietAfterSeconds,
    PokeLimit,
}

impl LiveKey {
    /// The key's path in `armada.yml`, which is what a person would search for.
    pub fn as_str(&self) -> &'static str {
        match self {
            LiveKey::QuietAfterSeconds => "drone.quiet_after_seconds",
            LiveKey::PokeLimit => "drone.poke_limit",
        }
    }
}

/// A part of the file that changed and will not take effect until a restart.
///
/// **Reported rather than adopted.** A person who edits `checks:` under a
/// running Fleet is owed the same sentence `#430` is about: the file and the
/// behaviour disagree, and something has to say so.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Frozen {
    Id,
    Version,
    Base,
    Checks,
    Commands,
    Setup,
}

impl Frozen {
    pub fn as_str(&self) -> &'static str {
        match self {
            Frozen::Id => "id",
            Frozen::Version => "version",
            Frozen::Base => "base",
            Frozen::Checks => "checks",
            Frozen::Commands => "commands",
            Frozen::Setup => "setup",
        }
    }
}

/// One live key's move, carrying both ends so a message can say what it was.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Moved {
    pub key: LiveKey,
    pub before: Option<u32>,
    pub after: Option<u32>,
}

impl std::fmt::Display for Moved {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(
            f,
            "{} {} -> {}",
            self.key.as_str(),
            said(self.before),
            said(self.after)
        )
    }
}

/// An absent key is a repository deferring to what Fleet is running with, and
/// that reads differently from a number — so it is spelled rather than blank.
fn said(value: Option<u32>) -> String {
    match value {
        Some(number) => number.to_string(),
        None => String::from("unset"),
    }
}

/// What one re-read of `armada.yml` came to.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Adopted {
    moved: Vec<Moved>,
    at_restart: Vec<Frozen>,
}

impl Adopted {
    /// The live keys that changed, in hand already applied. Empty where the
    /// file was saved without either of them moving, which is most saves.
    pub fn moved(&self) -> &[Moved] {
        &self.moved
    }

    /// The sections that changed and were not adopted. See [`Frozen`].
    pub fn at_restart(&self) -> &[Frozen] {
        &self.at_restart
    }

    /// Nothing this Fleet reads changed. **The ordinary answer** — a save that
    /// edits a comment, or an editor writing the same bytes back.
    pub fn is_quiet(&self) -> bool {
        self.moved.is_empty() && self.at_restart.is_empty()
    }
}

/// What the file said, at daemon start, about everything that is not live.
///
/// A shape rather than the Manifest itself: what is wanted is whether a section
/// changed, and holding the boot Manifest to answer that would be a second
/// Manifest for something to read by mistake.
#[derive(Debug, Clone, PartialEq, Eq)]
struct AtStart {
    id: String,
    version: u32,
    base: Option<String>,
    checks: Vec<String>,
    commands: Vec<String>,
    setup: Vec<String>,
}

impl AtStart {
    fn of(manifest: &Manifest) -> AtStart {
        AtStart {
            id: manifest.id().as_str().to_string(),
            version: manifest.version(),
            base: manifest.base().map(str::to_string),
            checks: manifest.check_names(),
            commands: manifest.command_names(),
            setup: manifest
                .prepared_by()
                .iter()
                .map(|step| step.name().to_string())
                .collect(),
        }
    }

    /// **Names, not bodies, for `checks` and `commands`.** A Check whose `run:`
    /// changed is as frozen as one that was added, and this comparison would
    /// miss it — recorded rather than hidden, because carrying every body here
    /// would be carrying the Manifest under another name. See the report on
    /// `#430`.
    fn against(&self, other: &AtStart) -> Vec<Frozen> {
        let mut changed = Vec::new();
        if self.id != other.id {
            changed.push(Frozen::Id);
        }
        if self.version != other.version {
            changed.push(Frozen::Version);
        }
        if self.base != other.base {
            changed.push(Frozen::Base);
        }
        if self.checks != other.checks {
            changed.push(Frozen::Checks);
        }
        if self.commands != other.commands {
            changed.push(Frozen::Commands);
        }
        if self.setup != other.setup {
            changed.push(Frozen::Setup);
        }
        changed
    }
}

/// The one handle that may move a Manifest's live keys under a running Fleet.
///
/// **It is produced by [`Manifest::reloadable`] and by nothing else**, so a
/// Manifest that arrived any other way — a fixture, a parse in a test, the one
/// Fleet holds — has no writer anywhere. That is the capability: the wrong call
/// is not available rather than checked for.
///
/// **It cannot change anything but the live keys.** There is no method that
/// takes a Check, a Command or a base branch, so the frozen half of the file
/// cannot move by this road even by mistake.
#[derive(Debug)]
pub struct Reloads {
    path: PathBuf,
    live: Cell,
    at_start: AtStart,
}

impl Reloads {
    pub(crate) fn of(path: PathBuf, live: Cell, manifest: &Manifest) -> Reloads {
        Reloads {
            path,
            live,
            at_start: AtStart::of(manifest),
        }
    }

    /// The file this re-reads. The path is held rather than passed in, so
    /// nothing can point a reload at a second file.
    pub fn path(&self) -> &Path {
        &self.path
    }

    /// Read the file again and adopt whatever its live keys now say.
    ///
    /// **The whole file is parsed and every refusal it carries is returned**,
    /// rather than the two keys being picked out of the YAML: a Manifest that
    /// no longer parses is a file somebody is mid-edit in, and adopting two
    /// numbers out of it would be reading a document Armada has refused.
    ///
    /// **On any refusal nothing moves.** The last good configuration stays in
    /// force and the caller is handed what was wrong with the file — a fleet
    /// that stopped because somebody mistyped a number would be worse than one
    /// that ignored the edit.
    pub fn reread(&self) -> Result<Adopted, LoadError> {
        let fresh = Manifest::load(&self.path)?;
        let after = fresh.patience();
        let before = self.live.replace(after);
        Ok(Adopted {
            moved: moved(before, after),
            at_restart: self.at_start.against(&AtStart::of(&fresh)),
        })
    }
}

/// Which of the two halves actually changed. **Each on its own**, for the
/// reason `crates/config/settings.toml` holds two rows rather than one pair:
/// a repository that changed its poke budget did not thereby change its
/// patience, and a message saying both moved would be wrong.
fn moved(before: Patience, after: Patience) -> Vec<Moved> {
    let mut changed = Vec::new();
    if before.quiet_after_seconds != after.quiet_after_seconds {
        changed.push(Moved {
            key: LiveKey::QuietAfterSeconds,
            before: before.quiet_after_seconds,
            after: after.quiet_after_seconds,
        });
    }
    if before.poke_limit != after.poke_limit {
        changed.push(Moved {
            key: LiveKey::PokeLimit,
            before: before.poke_limit,
            after: after.poke_limit,
        });
    }
    changed
}
