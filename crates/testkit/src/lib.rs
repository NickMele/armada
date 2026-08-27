//! The fake harness, and the fixtures that drive it.
//!
//! `testkit` exists because v1 shipped 586 commits, 2,181 passing tests and a
//! working screen, and the Jobs it completed did not do what they claimed. A
//! fake that only emits happy-path output reproduces exactly that: a fast green
//! suite proving nothing about the part that actually failed. So the fixtures
//! are weighted towards misbehaviour, and each one is a specification of what a
//! misbehaving Drone's output stream really looks like, written before the
//! detector that catches it.
//!
//! Fixtures live in `fixtures/ndjson/`, one file per failure mode, beside the
//! pinned real capture the format-drift contract test compares against.
//!
//! # Why the fixtures are not here yet
//!
//! The five fixtures are the gate's subjects and are not written by M0 step 7 —
//! the gate names them missing, which is the gate working. This crate is the
//! place they will land.
//!
//! # The fakes
//!
//! [`FakeHarness`] renders a Drone that is a harmless program rather than an
//! agent, and records what it was asked to render — so a test can start one end
//! to end and then read back the launch. It does not decode a transcript:
//! whether a line of a vendor's stream becomes the right event is asserted in
//! `adapters`, against the real decoder and the captured spike streams.
//!
//! [`FakeVcs`] creates a Job's worktree without a repository under it, and
//! [`FakeWorkProduct`] reads a diff out of one that does not exist. A suite
//! that needs a real checkout per case is one people stop running, so the cases
//! that genuinely need version control's own opinion stay in `adapters` and
//! everything else fakes the seam.
//!
//! # The fixtures that are not fakes
//!
//! [`resolved`] builds a `ResolvedWorkflow` by running two small documents
//! through the same parsers Fleet uses. It is not a fake of anything: the
//! parsing is real, and it is here because a workflow is what every gate test
//! needs and no test can construct one.

mod harness;
mod vcs;
mod work_product;
mod workflow;

pub use harness::{FakeHarness, FakeHarnessRefused};
pub use vcs::{Delivered, Delivering, FakeCommit, FakeVcs, FakeVcsError};
pub use work_product::{FakeDiffRefused, FakeWorkProduct};
pub use workflow::{frozen, resolved, Gate, Sketch};
