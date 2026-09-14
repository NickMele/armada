//! A worktree's build directories and the seed they start from. #1064.
//!
//! **Beside the run sheets that carry them, not inside `rehearsal`**: a seed is
//! something Fleet did when it cut a worktree, not a run a person started.

use serde::{Deserialize, Serialize};

/// What a Job's worktree was seeded with, or why it started cold.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(tag = "state", rename_all = "snake_case")]
pub enum WorktreeSeeding {
    /// Cloned from the base checkout's warm seed at `commit`.
    Seeded { commit: String, paths: Vec<String> },
    /// Nothing was cloned, and why, in a sentence.
    Cold { why: String },
    /// Nothing was written down: a worktree cut before seeding existed.
    Unrecorded,
}

/// `setup.seed` as the Manifest declares it, and where its warm-up stands.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct DeclaredSeed {
    pub paths: Vec<String>,
    /// The Commands `setup.seed.warm` names, in order.
    pub warmed_by: Vec<String>,
    pub warmth: SeedWarmth,
}

/// Whether the seed at the current base commit may be cloned.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(tag = "state", rename_all = "snake_case")]
pub enum SeedWarmth {
    Warm {
        commit: String,
    },
    /// A warm-up is running. A Job cut now starts cold.
    Warming {
        commit: String,
    },
    /// Not warm, and why, in a sentence.
    Cold {
        why: String,
    },
}
