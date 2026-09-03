//! The fake harness, and the fixtures that drive it.
//!
//! `testkit` exists because v1 shipped 586 commits, 2,181 passing tests and a
//! working screen, and the Jobs it completed did not do what they claimed. A
//! fake emitting only happy-path output reproduces exactly that: a fast green
//! suite proving nothing about the part that failed. So the fixtures are
//! weighted towards misbehaviour, and each is a specification of what a
//! misbehaving Drone's output stream really looks like, written before the
//! detector that catches it. They live in `crates/testkit/fixtures/ndjson/`,
//! one file per failure mode, beside the pinned real capture the format-drift
//! contract test compares against, and
//! `docs/contracts/testkit-fixtures.md` is the roster.
//!
//! **A fake stands in for a seam, never for the thing being asserted.**
//! [`FakeHarness`] renders a Drone that is a harmless program and records what
//! it was asked to render; whether a line of a vendor's stream becomes the
//! right event is `adapters`', against the real decoder. [`FakeJudge`] renders
//! a shell printing a scripted verdict, so a Judge call exercises Fleet's
//! runner and `verification`'s parser with no model, network or credential.
//! [`FakeVcs`] and [`FakeWorkProduct`] stand in for a repository, because a
//! suite needing a real checkout per case is one people stop running.
//!
//! [`resolved`], [`asking`] and [`asked_for`] fake nothing: the parsing is
//! real, and they are here because a workflow and a `Job` are what every gate
//! test needs and neither is a thing a test can write out in a line.

mod harness;
mod job;
mod judge;
mod link_lookup;
mod vcs;
mod work_product;
mod workflow;

pub use harness::{FakeHarness, FakeHarnessRefused};
pub use job::{asked_for, asking};
pub use judge::{refusal, FakeJudge};
pub use link_lookup::FakeLinkLookup;
pub use vcs::{Delivered, Delivering, FakeCommit, FakeVcs, FakeVcsError};
pub use work_product::{FakeDiffRefused, FakeWorkProduct, Holding, Written};
pub use workflow::{frozen, modelled, requiring, resolved, retried, Gaming, Gate, Scoped, Sketch};
