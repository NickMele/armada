//! charkit's imperative shell.
//!
//! Everything that touches the outside world lives here: subprocesses, the
//! filesystem, docker, git, the process module. The core proposes, this crate
//! attempts, and failures return to the core as data
//! (`ARCHITECTURE.md` §1.2).
//!
//! Phase 1 has no runtime, so this crate holds exactly two things: reading a
//! config file off disk, which the core is not allowed to do, and the SIGPIPE
//! restoration `main` needs before it writes a single byte.

#![deny(unsafe_code)]
#![warn(missing_docs)]

pub mod config_file;
pub mod posix;
