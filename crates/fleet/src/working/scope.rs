//! What the step said it would touch, and what the worktree actually holds.
//!
//! **The slot's half of `crate::scope` and `crate::footprint`.** A declaration
//! replaces rather than widens the one before it, the drift is kept in the
//! order first seen so the live check says a thing once, and the baseline the
//! step entered with is written at dispatch and never again.
//!
//! **Nothing here reads a disk.** Every value below arrives from a caller that
//! already looked, which is what keeps a worktree read on the two crate
//! modules that decide to make one rather than on any reader of the slot.

use adapter_traits::Footprint;
use core_model::{DeclaredPaths, RepoPath};

use crate::footprint::Publishing;
use crate::working::Working;

impl Working {
    /// Record what the Drone declared for the step it is on. **Replaces**, so a
    /// plan that turned out wrong is corrected by declaring again rather than
    /// by widening one that already exists.
    pub(crate) fn declares(&mut self, paths: DeclaredPaths) {
        self.declared = Some(paths);
        self.drifted.clear();
    }

    pub(crate) fn declared(&self) -> Option<&DeclaredPaths> {
        self.declared.as_ref()
    }

    /// Record what the worktree held as this step began.
    pub(crate) fn entering_with(&mut self, footprint: Footprint) {
        self.entered_with = Some(footprint);
    }

    /// The baseline this step's work is measured against.
    pub(crate) fn entered_with(&self) -> Option<&Footprint> {
        self.entered_with.as_ref()
    }

    /// What has been seen outside the plan so far. The third tripwire, and the
    /// observation the mid-step look is given.
    pub(crate) fn off_plan(&self) -> &[RepoPath] {
        &self.drifted
    }

    /// Add paths seen outside the plan, and answer with the ones that are new.
    /// Empty on every turn after the first that saw them, which is what keeps
    /// the live check from saying the same thing every tick.
    pub(crate) fn drifting(&mut self, seen: Vec<RepoPath>) -> Vec<RepoPath> {
        let fresh: Vec<RepoPath> = seen
            .into_iter()
            .filter(|path| !self.drifted.contains(path))
            .collect();
        self.drifted.extend(fresh.iter().cloned());
        fresh
    }

    /// When the worktree was last read for the live file list. **Mutable, and
    /// there is no read-only view of it**: every question anybody asks it is
    /// asked in the course of deciding whether to read again, which is a
    /// decision that records itself.
    pub(crate) fn publishing(&mut self) -> &mut Publishing {
        &mut self.publishing
    }
}
