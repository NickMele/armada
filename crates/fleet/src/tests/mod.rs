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

mod detach;
mod gate;
mod process;
mod runtime;
mod tmp;
