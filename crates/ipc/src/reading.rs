//! What Fleet's last read of `armada.yml` came to.
//!
//! **Fleet-wide, and not a Job's field.** Every `core_model::Envelope` in this
//! workspace is written into one Job's transcript, and a Manifest reload
//! belongs to no Job — which is why `crates/armada/src/watching.rs` had nowhere
//! to put this and said so on the daemon's console instead. There is no Job id
//! here because there is no Job.
//!
//! # A standing reading, not a notification
//!
//! [`FleetCapacity`](crate::FleetCapacity) is the shape this follows, and for
//! its reason. A refusal is not an instant that passes: the file on disk and
//! the values Fleet is running with disagree, and they go on disagreeing until
//! somebody fixes the file. So Fleet *holds* its last reading and answers for
//! it — `get_manifest_reading` — and pushes `manifest.reread` when it changes.
//! A client that was not connected when the read happened still learns it, and
//! a client that was does not have to have been looking.
//!
//! # Every fault, because a refused file is edited blind
//!
//! `config::LoadError` finds every fault rather than the first, and all of them
//! cross. A person correcting a file from a message naming one fault saves,
//! waits, and meets the next one — which is the same silence one round longer.

use serde::{Deserialize, Serialize};

use crate::ids::Instant;

/// Fleet's last reading of the Manifest, and what it did about it.
///
/// **Refusal is the absent field, not a flag.** `refused` present is the read
/// that did not take; absent is the read that did. That is the same shape
/// [`JobJudging`](crate::JobJudging) and [`ProposalMoved`](crate::ProposalMoved)
/// use for a fact whose absence is the message, rather than a boolean beside a
/// reason that is meaningless when it is false.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct ManifestReading {
    /// The file, as Fleet resolved it. A message that does not name the file is
    /// one somebody has to guess at, and a Fleet may hold more than one repo.
    pub path: String,
    /// When Fleet read it — **not when the file was saved**. The two differ by
    /// up to the settle window, and the filesystem is where the save's instant
    /// lives.
    pub at: Instant,
    /// The live keys this read moved, already in force. **Empty is the ordinary
    /// answer**: most saves edit something Fleet does not read live, and a save
    /// that changed nothing at all is a real reading rather than a missing one.
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub moved: Vec<ManifestMoved>,
    /// Sections that changed and were **not** adopted, spelled as `armada.yml`
    /// spells them — `checks`, `commands`, `setup`, `id`, `version`, `base`.
    ///
    /// Left as strings like every other closed set on this seam: a client
    /// renders the word rather than matching on it, so a section added later
    /// draws as itself instead of refusing the message around it.
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub at_restart: Vec<String>,
    /// Why the read did not take, or **absent because it did**.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub refused: Option<ManifestRefused>,
}

impl ManifestReading {
    /// Whether this reading is worth putting in front of somebody.
    ///
    /// **A save that moved nothing is not news.** Editing a comment, or an
    /// editor writing the same bytes back, is a reading like any other and
    /// drawing it would train a person to dismiss the surface that also carries
    /// the refusal. `config::Adopted::is_quiet` is the same judgement one crate
    /// down; this is it on the wire, so a client does not restate it.
    pub fn worth_saying(&self) -> bool {
        self.refused.is_some() || !self.moved.is_empty() || !self.at_restart.is_empty()
    }
}

/// One live key that changed, carrying both ends so a message can say what it
/// was rather than that something was.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct ManifestMoved {
    /// The key's path in `armada.yml` — `drone.poke_limit` — which is what a
    /// person would search the file for.
    pub key: String,
    /// **Absent is a real value, not a missing one**: the key was not in the
    /// file, and the repository was deferring to what Fleet runs with. A client
    /// spells that rather than leaving a blank, which would read as a number
    /// that failed to load.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub before: Option<u32>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub after: Option<u32>,
}

/// Why a read was refused, and what is running instead.
///
/// **The previous values stay in force**, which is the fact a person needs
/// second and the reason this is not a fault. One mistyped number is not
/// grounds for stopping every Job, so Fleet carries on with the last good
/// configuration — and something has to say so, or the file and the behaviour
/// disagree with nobody told.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct ManifestRefused {
    /// The whole refusal in one sentence, as `config::LoadError` renders it.
    ///
    /// **This is the only place a line number can appear.** A file that is not
    /// YAML at all has no keys to attribute a fault to, and the parser's own
    /// error is what carries the line and column — so it crosses as prose
    /// rather than being dropped for not fitting [`ManifestFault`].
    pub summary: String,
    /// What was wrong, key by key. **Empty where the document never became a
    /// document** — unreadable, or not YAML — in which case `summary` is the
    /// whole answer.
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub faults: Vec<ManifestFault>,
}

/// One key `armada.yml` was refused for.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct ManifestFault {
    /// The dotted path inside the document — `checks.build.run`,
    /// `steps[1].mechanical_checks[0].type`. Indices are the array position, so
    /// it points at a line rather than at a name somebody would have to count
    /// to find.
    pub key: String,
    /// What is wrong with it, in the words `config` used to refuse it.
    pub fault: String,
}
