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
    /// **Held as well as published, since a refusal is a standing condition, not an instant.**
    /// The file on disk and Fleet's running values go on disagreeing until corrected, so a
    /// client not connected when the read happened still needs to learn it — `get_manifest_reading`
    /// answers from the held copy; publishing alone reaches only whoever was watching that second.
    ///
    /// **Every reading is held, including a quiet one** — Fleet holds its last reading, not its
    /// last interesting one; whether it is worth showing is `ManifestReading::worth_saying`,
    /// asked by the surface that draws it rather than answered here by dropping the fact.
    ///
    /// `pub` rather than `pub(crate)`: the watch lives in the composition root, above this
    /// crate, because what it watches is a path the root resolved.
    pub fn reread(&self, reading: ipc::ManifestReading) {
        *self.reading.lock().unwrap_or_else(|held| held.into_inner()) = Some(reading.clone());
        self.events.publish(ipc::Event::ManifestReread(reading));
    }

    /// What the last re-read came to, or `None` because there has not been one.
    pub(crate) fn last_reading(&self) -> Option<ipc::ManifestReading> {
        self.reading
            .lock()
            .unwrap_or_else(|held| held.into_inner())
            .clone()
    }
}
