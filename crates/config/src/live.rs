//! The Manifest keys a person may change under a running Fleet, and the one
//! handle that may change them.
//!
//! **Not the whole file, and not even the whole of one section.**
//! `crates/config/settings.toml` files six of `armada.yml`'s keys as
//! `lifetime = "Live"`: four under `drone:` — `quiet_after_seconds`,
//! `poke_limit`, `cost_cap_micros_per_job` and `turn_cap_per_job` — and the two
//! top-level policies, `auto_merge` and `review_gate`. The Checks and Commands
//! registries and `drone.exclude_paths` are *Frozen for the Job*; the last sits
//! in the same `drone:` block as four live ones, which is why [`Frozen`] names a
//! key there and a section everywhere else. What decides is what was resolved against a
//! value at boot: every [`ResolvedWorkflow`] took the Checks this file declared
//! there, so swapping the whole Manifest would falsify a `Setup` silently.
//!
//! So the live keys sit behind a cell every clone of one Manifest shares, and
//! the rest is what it was when the daemon read it. **Four of the six are read
//! at a question rather than at a boot** — `auto_merge` and `review_gate`
//! whenever a gate or a sweep asks, both caps at every admission of a Job over
//! one of them — so somebody stopping a merge, or raising a ceiling under the
//! Job it is refusing, is owed an answer sooner than a restart.
//!
//! **[`Reloads`] is the only thing that can write, and the live keys are all it
//! can write.** Fleet is never handed one, so the crate holding the Manifest
//! cannot move it: the method is not on anything Fleet has.
//! [`ResolvedWorkflow`]: crate::ResolvedWorkflow

use std::path::{Path, PathBuf};
use std::sync::{Arc, RwLock};

use core_model::{AutoMerge, ReviewGate};

use crate::error::LoadError;
use crate::manifest::Manifest;

/// What `drone:` says that a save may move, together because one re-read
/// adopts all of it or none of it.
///
/// **Named for the section and not for patience**, which is what it was called
/// while it held two numbers about a quiet Drone. A ceiling is not patience,
/// and a type whose name covers half its fields is one nobody can add to.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub(crate) struct Dials {
    pub(crate) quiet_after_seconds: Option<u32>,
    pub(crate) poke_limit: Option<u32>,
    /// Millionths of a dollar one Job of this repository may spend. **A `u32`
    /// where the Job's own column is a `u64`**: `u32::MAX` micros is $4,294.96
    /// and a per-Job ceiling above that is not a ceiling, so the narrower type
    /// loses nothing. It was also what kept a move reportable without widening
    /// [`Moved`]; that half of the argument is spent, since `Moved` now carries
    /// the value as text for the two policies' sake, and the ceiling is the
    /// whole reason now.
    pub(crate) cost_cap_micros: Option<u32>,
    /// How many turns one Job of this repository may take. **A `u32` for the
    /// field above's reason and with more room to spare**: a per-Job ceiling of
    /// four billion turns is not a ceiling.
    pub(crate) turn_cap: Option<u32>,
}

/// Every live key's value at once: `drone:`'s [`Dials`] and the two top-level
/// policies.
///
/// **One struct rather than two cells**, because a re-read moves all of it or
/// none — [`Reloads::reread`] parses the whole file and adopts what it finds,
/// so a second lock would be a second instant for one save.
///
/// **Two types rather than six fields**, because the policies are not `drone:`
/// keys and [`Dials`] is named for that section. A flat struct would put
/// `auto_merge` beside `poke_limit` as though the file did, and the file does
/// not.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub(crate) struct InForce {
    pub(crate) dials: Dials,
    /// **Defaulted rather than optional**, unlike the pair above it. An absent
    /// `quiet_after_seconds` defers to the tier Fleet is running with and there
    /// is a tier to defer to; `auto_merge` and `review_gate` are `Manifest
    /// only` in the settings registry, so an absent key has nothing above it
    /// and means the policy's own default. `None` here would be a third
    /// reading of a two-reading value.
    pub(crate) auto_merge: AutoMerge,
    pub(crate) review_gate: ReviewGate,
}

/// Where the live keys are kept, shared by every clone of one Manifest.
///
/// **Clone shares rather than copies**, which is the whole point: `Setup` reads
/// the file, hands the Manifest to Fleet by value, and a reload has to reach the
/// one Fleet is holding rather than a copy nobody consults.
#[derive(Debug, Clone)]
pub(crate) struct Cell(Arc<RwLock<InForce>>);

impl Cell {
    pub(crate) fn holding(in_force: InForce) -> Cell {
        Cell(Arc::new(RwLock::new(in_force)))
    }

    /// **A poisoned lock is read through rather than unwrapped.** What it
    /// guards is four `Option<u32>` and two `Copy` enums, so a panic elsewhere
    /// cannot have left it half-written — and a Fleet that goes down because a
    /// lock was poisoned by an unrelated panic is exactly the failure this
    /// whole module refuses.
    pub(crate) fn read(&self) -> InForce {
        *self
            .0
            .read()
            .unwrap_or_else(std::sync::PoisonError::into_inner)
    }

    /// Write, and hand back what was there. For [`Cell::read`]'s reason a
    /// poisoned lock is written through.
    fn replace(&self, in_force: InForce) -> InForce {
        let mut held = self
            .0
            .write()
            .unwrap_or_else(std::sync::PoisonError::into_inner);
        std::mem::replace(&mut held, in_force)
    }
}

/// Which live key moved. Named rather than spelled at each call site, so a
/// message about a reload and the file it came from use one vocabulary.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum LiveKey {
    QuietAfterSeconds,
    PokeLimit,
    /// **Live, and that is the whole reason it exists.** A Job over its cap is
    /// refused at every admission until the number moves, so a cap needing a
    /// restart would be a lever nobody can pull while the thing it is about is
    /// happening. One Job was refused its last step at $5.28 against a $5
    /// compile-time constant, and there was no reachable number anywhere.
    CostCapMicrosPerJob,
    /// **The other ceiling, and live for the same reason.** It shipped as a
    /// compile-time constant with no tier under it at all, and Job
    /// `01M22TYSAE0023MADDP5ZQEYGW` stranded at 393 turns against 300 with its
    /// cheap final step never run.
    TurnCapPerJob,
    /// The two below are the only live keys that are not `drone:`'s, and the
    /// only two whose value is a word rather than a number.
    AutoMerge,
    ReviewGate,
}

impl LiveKey {
    /// The key's path in `armada.yml`, which is what a person would search for.
    pub fn as_str(&self) -> &'static str {
        match self {
            LiveKey::QuietAfterSeconds => "drone.quiet_after_seconds",
            LiveKey::PokeLimit => "drone.poke_limit",
            LiveKey::CostCapMicrosPerJob => "drone.cost_cap_micros_per_job",
            LiveKey::TurnCapPerJob => "drone.turn_cap_per_job",
            // Top-level and undotted, because that is where `armada.yml` writes
            // them and this string is what somebody searches the file for. It
            // is also the key a `manifest_rule:<key>` gate names, and one
            // spelling is what keeps the gate and the file findable from each
            // other.
            LiveKey::AutoMerge => "auto_merge",
            LiveKey::ReviewGate => "review_gate",
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
    /// **A key, where every other variant is a section.** Four of `drone:`'s
    /// five keys are live and this one is not — every `ResolvedWorkflow` took
    /// its copy of this list at boot — so naming the section would tell a
    /// person that a `quiet_after_seconds` they just changed needs a restart,
    /// which is the opposite of true.
    DroneExcludePaths,
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
            Frozen::DroneExcludePaths => "drone.exclude_paths",
        }
    }
}

/// One live key's move, carrying both ends so a message can say what it was.
///
/// **Both ends are the value as `armada.yml` writes it, and that is why they
/// are text.** They were `Option<u32>` while every live key held a number; a
/// policy's value is a word, and a second pair of fields for the words would be
/// four fields stating one fact — the vocabulary split this repository refuses.
/// Nothing on either side of the wire does arithmetic on these: they are
/// rendered, in one sentence, by [`Display`](std::fmt::Display) here and by
/// `ManifestNotice` in Bridge.
///
/// **[`None`] stays the absent key**, which is not the empty string and is
/// spelled by [`said`] rather than left blank.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Moved {
    pub key: LiveKey,
    pub before: Option<String>,
    pub after: Option<String>,
}

impl std::fmt::Display for Moved {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(
            f,
            "{} {} -> {}",
            self.key.as_str(),
            said(self.before.as_deref()),
            said(self.after.as_deref())
        )
    }
}

/// An absent key is a repository deferring to what Fleet is running with, and
/// that reads differently from a number — so it is spelled rather than blank.
fn said(value: Option<&str>) -> String {
    match value {
        Some(written) => written.to_string(),
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
    exclude_paths: Vec<String>,
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
            // Bodies, not names, because the whole value is the list: unlike
            // `checks`, there is nothing behind an entry here that a shorter
            // comparison could miss.
            exclude_paths: manifest
                .exclude_paths()
                .iter()
                .map(|path| path.as_str().to_string())
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
        if self.exclude_paths != other.exclude_paths {
            changed.push(Frozen::DroneExcludePaths);
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
        let after = fresh.in_force();
        let before = self.live.replace(after);
        Ok(Adopted {
            moved: moved(before, after),
            at_restart: self.at_start.against(&AtStart::of(&fresh)),
        })
    }
}

/// Which of the five live keys actually changed. **Each on its own**, for the
/// reason `crates/config/settings.toml` holds a row apiece: a repository that
/// changed its poke budget did not thereby change its patience or what it will
/// spend, and the two policies are as independent again — `auto_merge` is about
/// whether work lands and `review_gate` about whether somebody signs off, one
/// step apart. A message saying they all moved would be wrong about four.
fn moved(before: InForce, after: InForce) -> Vec<Moved> {
    let mut changed = Vec::new();
    // **Every number is rendered where it is read**, so `unset` has one
    // spelling for all five keys and the composition root copies `Moved`
    // field for field rather than converting three of them.
    let said = |value: Option<u32>| value.map(|number| number.to_string());
    if before.dials.quiet_after_seconds != after.dials.quiet_after_seconds {
        changed.push(Moved {
            key: LiveKey::QuietAfterSeconds,
            before: said(before.dials.quiet_after_seconds),
            after: said(after.dials.quiet_after_seconds),
        });
    }
    if before.dials.poke_limit != after.dials.poke_limit {
        changed.push(Moved {
            key: LiveKey::PokeLimit,
            before: said(before.dials.poke_limit),
            after: said(after.dials.poke_limit),
        });
    }
    if before.dials.cost_cap_micros != after.dials.cost_cap_micros {
        changed.push(Moved {
            key: LiveKey::CostCapMicrosPerJob,
            before: said(before.dials.cost_cap_micros),
            after: said(after.dials.cost_cap_micros),
        });
    }
    if before.dials.turn_cap != after.dials.turn_cap {
        changed.push(Moved {
            key: LiveKey::TurnCapPerJob,
            before: said(before.dials.turn_cap),
            after: said(after.dials.turn_cap),
        });
    }
    // **Always `Some` on both ends**, unlike the three above: an absent policy
    // key is the policy's default rather than a deferral, so the sentence a
    // person reads names the value that was in force rather than the word
    // `unset`. `InForce::auto_merge` carries why.
    if before.auto_merge != after.auto_merge {
        changed.push(Moved {
            key: LiveKey::AutoMerge,
            before: Some(before.auto_merge.as_written().to_string()),
            after: Some(after.auto_merge.as_written().to_string()),
        });
    }
    if before.review_gate != after.review_gate {
        changed.push(Moved {
            key: LiveKey::ReviewGate,
            before: Some(before.review_gate.as_written().to_string()),
            after: Some(after.review_gate.as_written().to_string()),
        });
    }
    changed
}
