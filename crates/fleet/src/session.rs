//! Speaking to a Drone that is already running, and ending one.
//!
//! # Why this is a trait with no implementation in this crate
//!
//! **The mechanism is measured and the session is not built.** Spike 4
//! established that Fleet can inject a turn into a live session — the harness
//! reads one JSON object per line on stdin with the stream held open, and
//! re-emits each message when it is consumed, which is what made the latency
//! measurable rather than inferred. Three runs, delivered in 1.59s mid-task and
//! 2.85s idle.
//!
//! What that mechanism needs is a live session, and a live session needs a
//! spawned Drone — which is a different milestone step, and it is not done.
//! `DroneSpawnConfig` and `DroneHandle` are still uninhabited placeholders in
//! `adapter-traits`, so **no implementation of this trait can exist yet, and
//! neither can a fake of one that holds a handle.** The trait is stated here so
//! the gate is complete up to the seam and the missing half is one named thing
//! rather than a gap in the middle of a function.
//!
//! # What the spike also established, and what it costs this trait
//!
//! **Delivery waits for the current turn to end.** A message injected while the
//! Drone is inside a tool call is consumed when that call returns — measured at
//! 33.14s against a 40-second command, of which none was latency. For the gate
//! that cost is zero: a Drone that has just submitted evidence is between turns
//! by definition, which is exactly the moment the gate speaks.
//!
//! # Two methods, and neither can start anything
//!
//! [`tell`](LiveSession::tell) and [`terminate`](LiveSession::terminate). There
//! is no spawn, no respawn and no restart, because the gate must not be able to
//! produce a Drone — and no way to remove a worktree, because nothing in this
//! workspace can.

use verification::OutcomeTurn;

/// A Drone's live session, from the gate's side.
pub trait LiveSession {
    /// Errors this implementation can raise. Named by the implementation, so a
    /// caller can tell a session that has already ended from one that would not
    /// accept the write.
    type Error;

    /// Inject a turn. The Drone reads it at the next turn boundary.
    ///
    /// Called only where the step advanced. A failed step ends the Job, and the
    /// Drone is terminated rather than told — see `verification::OutcomeTurn`.
    fn tell(&self, turn: &OutcomeTurn) -> Result<(), Self::Error>;

    /// End the Drone.
    ///
    /// **The worktree is untouched.** Removal is driven by Job retention and
    /// never by a process ending, and there is no method in this workspace that
    /// could remove one anyway. The branch is left exactly as the Drone left
    /// it, which is what "a person reads the branch" depends on.
    fn terminate(&self) -> Result<(), Self::Error>;
}
