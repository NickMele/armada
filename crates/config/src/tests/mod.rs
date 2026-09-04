//! What the parser accepts and what it refuses.
//!
//! Split by subject rather than by kind, because a refusal and the acceptance
//! it is the mirror of belong beside each other: the interesting question about
//! `mechanical_checks` is that two of four steps in the worked example have
//! none, and that reads as a pair with the one that does.
//!
//! [`samples`] is the odd one out and is the point of the milestone step: it
//! loads the seven WorkflowDefs checked into this repository and asserts what
//! this parser does with each. A validator that cannot reject the samples in
//! its own repository is not a validator.

mod loops;
mod manifest;
mod model;
mod prerequisites;
mod samples;
mod scope;
mod workflow;

use std::path::{Path, PathBuf};

use crate::error::{Fault, LoadError, Refusal};
use crate::roster::Roster;

/// The models a fixture may name.
///
/// **Invented names, and that is the point.** The legal set is whatever the
/// caller resolved rather than anything this crate knows — see
/// [`crate::Roster`] — so a fixture roster spelled in a vendor's aliases would
/// read as though the parser had a list of its own. Whether the *shipped*
/// definitions name models this machine offers is `tests/shipped.rs`, against
/// the adapter's real roster.
pub(crate) fn roster() -> Roster {
    Roster::of(["the-deciding-model", "the-reporting-model"])
}

/// A path for a document that is never written. Every refusal names the file it
/// came from, and the tests exercise the parser rather than the filesystem.
pub(crate) fn named(name: &str) -> PathBuf {
    Path::new(name).to_path_buf()
}

/// The refusals a load produced, or a panic naming what loaded instead.
pub(crate) fn refusals<T: std::fmt::Debug>(result: Result<T, LoadError>) -> Vec<Refusal> {
    match result {
        Ok(loaded) => panic!("expected a refusal and got {loaded:?}"),
        Err(LoadError::Refused { refusals, .. }) => refusals,
        Err(other) => panic!("expected a refusal and got {other}"),
    }
}

/// Whether exactly one refusal names this key, and what its fault was.
pub(crate) fn fault_at<'a>(refusals: &'a [Refusal], key: &str) -> &'a Fault {
    let mut found = refusals.iter().filter(|r| r.key == key);
    let first = found
        .next()
        .unwrap_or_else(|| panic!("nothing refused `{key}`; refusals were {refusals:?}"));
    assert!(
        found.next().is_none(),
        "`{key}` was refused more than once: {refusals:?}"
    );
    &first.fault
}

/// Whether any refusal names this key.
pub(crate) fn refused(refusals: &[Refusal], key: &str) -> bool {
    refusals.iter().any(|r| r.key == key)
}
