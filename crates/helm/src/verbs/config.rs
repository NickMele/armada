//! `armada manifest config` — layers 1 and 3 of the bootstrap sandwich
//! (PLAN.md §5).
//!
//! ```text
//! 1. scan     Armada    an evidence report, never a config
//! 2. author   an agent  the armada.yml, from evidence + schema + an example
//! 3. verify   Armada    pass/fail, with the key path and the fix
//! ```
//!
//! Layer 2 is not here, and its absence is the design. Armada reports facts and
//! validates a written file; deciding which of four test scripts is canonical
//! is judgement, and a scanner that supplies judgement produces a file nobody
//! can trust and everybody has to re-read.
//!
//! **`scan` is the one verb that runs with no `armada.yml` at all** (PLAN.md
//! §2.1), which is why it takes a directory rather than an [`App`]: there is no
//! workspace to resolve, no lease to hold and nothing to reap.
//!
//! [`App`]: crate::app::App

use armada_core::envelope::{Envelope, ScanData};
use armada_core::error::{ArmadaError, Status};
use armada_core::scan;
use std::path::Path;

use crate::verbs::Output;

/// `armada manifest config scan`, over one directory.
///
/// **Exit 0 whenever the directory is readable**
/// (`docs/commands/manifest/config.md`): it reports rather than judges, so
/// there is no outcome it could fail on. A file that does not parse contributes
/// nothing and the other twelve pieces of evidence still print.
pub fn scan(root: &Path) -> Result<Output, ArmadaError> {
    let files = armada_manifest::scan::read(root);
    let evidence = scan::scan(&files);
    Ok(Output::Scan(Box::new(Envelope::ok(
        "config scan",
        // **No workspace id, deliberately.** A repository being scanned has not
        // been claimed and has no identity yet; inventing one here would be the
        // first thing `scan` decided.
        None,
        Status::Ok,
        ScanData {
            results: scan::findings(&evidence),
            evidence,
        },
    ))))
}
