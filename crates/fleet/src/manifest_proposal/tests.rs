//! Proposals against a real directory, for `scanning::tests`' reason.

mod amending;
mod building;
mod creating;
mod writing;

use ipc::ManifestProposal;

use super::{propose, Draft};
use crate::scanning::{scan, Checkout};
use crate::tests::tmp::TempDir;

/// A checkout holding these files, parents made.
fn checkout(files: &[(&str, &str)]) -> TempDir {
    let dir = TempDir::new();
    for (path, text) in files {
        let at = dir.path().join(path);
        std::fs::create_dir_all(at.parent().expect("a parent")).expect("a directory");
        std::fs::write(at, text).expect("a file");
    }
    dir
}

/// Every proposal for `dir`, as built from a Scan of it.
fn drafts(dir: &TempDir) -> Vec<Draft> {
    let root = dir.path().to_string_lossy().to_string();
    propose(&scan(&root, &Checkout::at(&root)))
}

fn draft(dir: &TempDir, workspace: &str) -> Draft {
    drafts(dir)
        .into_iter()
        .find(|one| one.dir() == workspace)
        .unwrap_or_else(|| panic!("{workspace} is proposed"))
}

fn answered(dir: &TempDir, workspace: &str) -> ManifestProposal {
    draft(dir, workspace).answer()
}
