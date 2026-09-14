//! Holding what a re-read of `armada.yml` came to, and saying so.
//!
//! **A child module of `daemon`, so the field stays private.** What is here is
//! two methods on [`Fleet`] and no state of its own; it is a file rather than
//! twenty more lines in `daemon.rs` because that file is at the 900 the gate
//! refuses, and this is the one concern in it that is about the Manifest rather
//! than about a Job.

use super::Fleet;

impl<H, V, W> Fleet<H, V, W> {
    /// Take what a re-read of `armada.yml` came to: hold it, and say so.
    ///
    /// **Held as well as published, because a refusal is a standing condition
    /// rather than an instant.** The file on disk and the values Fleet is
    /// running with go on disagreeing until somebody corrects the file, so a
    /// client that was not connected when the read happened still has to be
    /// able to learn it — which is what `get_manifest_reading` answers from.
    /// Publishing alone would make a refusal visible only to whoever happened
    /// to be looking at the window in the second it fired.
    ///
    /// **Every reading is held, including a quiet one.** What Fleet holds is
    /// its last reading, not its last interesting reading; whether it is worth
    /// putting in front of somebody is `ManifestReading::worth_saying`, asked
    /// by the surface that would draw it rather than answered here by dropping
    /// the fact.
    ///
    /// `pub` rather than `pub(crate)`: the watch lives in the composition root,
    /// above this crate, because what it watches is a path the root resolved.
    ///
    /// `root` is the repository whose `armada.yml` was read; a root no longer
    /// served is still published and held nowhere.
    pub fn reread(&self, root: &str, reading: ipc::ManifestReading) {
        if let Some(repository) = self.repositories.at(root) {
            repository.read(reading.clone());
        }
        self.rehearsals.workspace_dirs().forget(Some(root));
        self.events.publish(ipc::Event::ManifestReread(reading));
    }
}
