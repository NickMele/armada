//! charkit's imperative shell.
//!
//! Everything that touches the outside world lives here: subprocesses, the
//! filesystem, docker, git, the process module. The core proposes, this crate
//! attempts, and failures return to the core as data
//! (`ARCHITECTURE.md` §1.2).
//!
//! Phase 2 fills it in: the three seams' real implementations
//! ([`process::RealRun`], [`clock::SystemClock`], [`net::RealFetch`]), the
//! machine-global store ([`db`]), the process-group wrapper, and the docker,
//! git and filesystem modules — which are **not** seams, because they are
//! subprocesses and files respectively, and faking either would hide the bug
//! class char exists to prevent.

#![deny(unsafe_code)]
#![warn(missing_docs)]

pub mod clock;
pub mod config_file;
pub mod db;
pub mod discovery;
pub mod docker;
pub mod fs;
pub mod git;
pub mod machine;
pub mod net;
pub mod posix;
pub mod process;
pub mod runs;
