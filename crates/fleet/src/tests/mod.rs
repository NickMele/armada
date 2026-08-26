//! What this crate proves about itself so far.
//!
//! Three subjects, and they are one subject: a Fleet that outlives the app is
//! only useful if something can find it, and a runtime file is only useful if
//! its reader can tell a live Fleet from a number that used to be one.
//!
//! `process` proves the identity primitive, `runtime` proves the file built on
//! it, `detach` proves a Drone is not in Fleet's process group. Nothing here
//! reads a clock, and nothing here is skipped on a platform: `ps` is spelled
//! the same way everywhere Armada runs.

mod detach;
mod process;
mod runtime;
mod tmp;
