//! What the boot read found, and what the reconciliation did about it.
//!
//! **Its own file, and not because it is big.** `daemon.rs` is at the 900 lines
//! the gate refuses, and of everything in it this is the one shape that is not
//! the daemon: it is what one call *answered*, held by nobody and written
//! nowhere. The cut is where the file was already going to be cut.

use core_model::JobId;

#[derive(Debug, Default)]
pub struct Reconciled {
    /// Jobs the store says were `running` and whose Drone is gone — **asked
    /// about and answered**, not assumed. Every one is now `escalated`, reason
    /// `interrupted`. A Job whose probe would not run is here too, and its log
    /// line says the process may still be there.
    pub interrupted: Vec<JobId>,
    /// Jobs whose Drone outlived this Fleet's predecessor and was taken back
    /// over. **Each is still where it was** — ordinarily `running` — with the
    /// process in a slot, its pid attributing its own calls again, and a row in
    /// its transcript saying what nothing observed. See `crate::readopting`.
    pub adopted: Vec<JobId>,
    /// Rows whose cached status disagreed with the log and were corrected.
    pub repaired: usize,
    /// Rows that would not rebuild at all. **Never dropped** — carried out so a
    /// caller cannot end up holding a short list with nothing saying so.
    pub unreadable: Vec<String>,
    /// The Jobs dispatched on the way out, where the bound had room and they
    /// were waiting. Empty on the ordinary boot.
    pub admitted: Vec<JobId>,
}
