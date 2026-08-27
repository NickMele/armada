//! What this crate proves about itself so far.
//!
//! The OS lifecycle is three subjects that are one subject: a Fleet that
//! outlives the app is only useful if something can find it, and a runtime file
//! is only useful if its reader can tell a live Fleet from a number that used
//! to be one.
//!
//! `process` proves the identity primitive, `runtime` proves the file built on
//! it, `detach` proves a Drone is not in Fleet's process group. Nothing here
//! reads a clock, and nothing here is skipped on a platform: `ps` is spelled
//! the same way everywhere Armada runs.
//!
//! `gate` is the fourth and it is a different subject: what Fleet does when
//! Evidence lands. Most of its cases are cases where nothing advances, which is
//! the proportion the milestone is about.
//!
//! `drone` and `session` are the fifth: what a Drone is given, what it is told,
//! and what its death means for the Job. They start a real child — a shell that
//! prints something and reads a line — because the two questions they exist to
//! answer are questions about an operating system rather than about a value.
//! **Nothing here starts an agent.** What a Drone is confined to is a rendering
//! and is asserted in `adapters`, where no process is involved at all.
//!
//! `evidence` is the seventh: a tool call arriving as JSON-RPC over the router
//! that ships, reaching the inbox, and advancing the step it was for.
//!
//! `frozen` is the eighth: what a running Job keeps hold of. Its workflow does
//! not move under it, it knows which Drone is on it, and its Checks leave their
//! output on disk.
//!
//! `landing` is the ninth: a finished Job's work reaches its branch. Most of its
//! cases are ones where no commit is made, which is where the rule is.
//!
//! `daemon` and `serving` are the sixth, and they are the first that are about
//! the whole: a Job driven from created to completed against fakes, read back
//! out of a reopened store, and the same five operations answered over the
//! router that ships. `briefing` and `host` are the two seams that arrived with
//! them — what a Drone is told, and the one place a clock is read and an id is
//! invented.

mod briefing;
mod checks;
mod daemon;
mod detach;
mod detail;
mod drone;
mod evidence;
mod frozen;
mod gate;
mod host;
mod landing;
mod process;
mod redispatch;
mod runtime;
mod serving;
mod session;
mod tmp;
mod transcript;
