//! How a Job meets a command its Drone was not granted, and the commands a
//! person allowed for it.
//!
//! **Settings on the Job, not moves of it.** Neither changes a status or a step,
//! so no event carries them and `store` keeps each as the authority for itself.
//! Both are read at the next permission question or spawn, so changing either
//! needs no respawn.

use alloc::string::String;

use crate::envelope::{Actor, Timestamp};

/// How a Job meets a command its Drone was not granted.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub enum WhenBlocked {
    /// Refuse the command and hold the Job for a person, asking nobody. What
    /// every Job written before this setting existed reads as, and this
    /// type's own default — read where the store cannot say, not where a Job
    /// simply chose nothing: a new Job is bound to `AskMe` at creation, so
    /// this is the safe answer to a read that failed, not the answer a new
    /// Job starts at.
    #[default]
    RefuseAndHold,
    /// Put the command to a person, who allows or rejects it. What a Job
    /// starts at, so nothing runs ungranted without a person also being
    /// asked.
    AskMe,
    /// Allow it without asking. **Not what `armada.yml` withholds**: a command
    /// declared destructive, and one the harness cannot grant, are refused
    /// under this as under the others, because no person's setting widens them.
    AllowAll,
}

impl WhenBlocked {
    pub const ALL: &'static [WhenBlocked] = &[
        WhenBlocked::RefuseAndHold,
        WhenBlocked::AskMe,
        WhenBlocked::AllowAll,
    ];

    pub fn as_wire(&self) -> &'static str {
        match self {
            WhenBlocked::RefuseAndHold => "refuse_and_hold",
            WhenBlocked::AskMe => "ask_me",
            WhenBlocked::AllowAll => "allow_all",
        }
    }

    /// `None` where the text is neither spelling: a row written by something
    /// that did not share this enum.
    pub fn from_wire(value: &str) -> Option<WhenBlocked> {
        WhenBlocked::ALL
            .iter()
            .copied()
            .find(|w| w.as_wire() == value)
    }
}

/// How far a person's allow reaches.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Reach {
    /// This Job only. Every later spawn of it carries the command.
    Job,
    /// This Job, and the person also made it permanent in the repository's
    /// `armada.yml`.
    Repository,
}

impl Reach {
    pub const ALL: &'static [Reach] = &[Reach::Job, Reach::Repository];

    pub fn as_wire(&self) -> &'static str {
        match self {
            Reach::Job => "job",
            Reach::Repository => "repository",
        }
    }

    pub fn from_wire(value: &str) -> Option<Reach> {
        Reach::ALL.iter().copied().find(|r| r.as_wire() == value)
    }
}

/// A command a person allowed for one Job.
///
/// `run` is the command text as the person allowed it. Nothing in this crate
/// interprets it.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct AllowedCommand {
    pub run: String,
    pub reach: Reach,
    pub allowed_at: Timestamp,
    pub by: Actor,
}
