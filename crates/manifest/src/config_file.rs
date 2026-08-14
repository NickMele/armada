//! Reading `armada.yml` off disk.
//!
//! The read lives here because the core does no I/O — it takes the text and
//! gives back a typed value. That split is what lets every config test run
//! against a string with no tmpdir, and it is why `armada-core` has no file
//! API to accidentally reach for.
//!
//! Finding *which* `armada.yml` — the walk up to the git root, the two-or-more
//! rule, nested workspaces (PLAN.md §2.1) — is workspace resolution, and
//! arrives with the rest of the ownership layer in phase 2. This module reads a
//! path it is given.

use armada_core::error::{ArmadaError, ConfigWhere, ErrClass};
use std::path::Path;

/// Read a config file, mapping an I/O failure into char's own vocabulary.
///
/// A missing `armada.yml` is `bad_config` — the workspace is not set up — while
/// anything else about the filesystem is `environment`: the repo is fine and
/// the machine is not, which is the distinction that decides whether an agent
/// edits a file or a human fixes a disk (`ARCHITECTURE.md` §1.7).
pub fn read(path: &Path) -> Result<String, ArmadaError> {
    std::fs::read_to_string(path).map_err(|e| {
        let display = path.display().to_string();
        match e.kind() {
            std::io::ErrorKind::NotFound => ArmadaError::bad_config(
                ConfigWhere::File { file: display },
                "no armada.yml here",
                "run `char config scan` to gather evidence, then author a armada.yml",
            ),
            _ => ArmadaError {
                class: ErrClass::Environment,
                r#where: display,
                message: format!("could not read armada.yml: {e}"),
                next_action: None,
            },
        }
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_missing_config_is_bad_config_and_says_what_to_do() {
        let err = read(Path::new("/nonexistent/armada.yml")).unwrap_err();
        assert_eq!(err.class, ErrClass::BadConfig);
        assert!(err.next_action.is_some());
    }
}
