//! Which runner a Check is driven by, frozen onto the workflow with it.
//!
//! **A name and the package it runs in, never the commands.** The commands
//! live in the runner's own description, which is data Armada ships, learns or
//! reads from the repository — `docs/concepts/runner-adapter.md`. A Check
//! naming one is saying *this is vitest, in this package*; every way of running
//! less than all of it follows from that and is written once per runner rather
//! than once per Check.
//!
//! **Frozen for `Narrowing`'s reason.** A Job carries the runner its workflow
//! was approved with, so a Manifest edited while it runs cannot change what its
//! Checks do halfway through.

use alloc::string::String;

/// The runner one Check is driven by.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Runner {
    name: String,
    pkg: Option<String>,
}

impl Runner {
    /// Build one from keys already read off a Manifest, or off a row written
    /// from one. **`config` and `store` are the only callers**, which is
    /// [`Narrowing::declared`](super::Narrowing)'s shape and its reason.
    pub fn declared(name: String, pkg: Option<String>) -> Runner {
        Runner { name, pkg }
    }

    /// Which description answers for this Check.
    pub fn name(&self) -> &str {
        &self.name
    }

    /// What `{pkg}` resolves to in that description's templates. **`None`
    /// where the Check declares none**, and then a template naming `{pkg}` has
    /// nothing to put there and the Check runs whole instead — a runner whose
    /// commands are rooted at the repository needs no package and says so by
    /// omitting it.
    pub fn pkg(&self) -> Option<&str> {
        self.pkg.as_deref()
    }
}
