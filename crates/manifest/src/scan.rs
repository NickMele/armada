//! Reading the files [`armada_core::scan`] parses.
//!
//! The split is §1.2's: opening a file is I/O and lives here, turning its bytes
//! into evidence is a pure function and lives in the core. This module knows
//! only *which* files are worth reading, and that list is the core's
//! [`CANDIDATES`] rather than a heuristic of its own.
//!
//! **Nothing walks the tree.** `armada manifest config scan` promises to depend
//! on nothing but a readable directory, and a recursive walk of a repository
//! nobody has configured yet is a promise about `node_modules` that nobody
//! made. The one directory that is listed is `.github/workflows/`, because that
//! directory is the evidence.

use armada_core::scan::{SourceFile, CANDIDATES, WORKFLOW_DIR};
use std::path::Path;

/// Read every candidate file that is present under `root`.
///
/// A file that cannot be read is skipped rather than reported: `scan` exits 0
/// whenever the directory is readable, and one unreadable `Makefile` is not a
/// reason to withhold the other thirteen pieces of evidence.
pub fn read(root: &Path) -> Vec<SourceFile> {
    let mut files = Vec::new();

    for name in CANDIDATES {
        if let Ok(text) = std::fs::read_to_string(root.join(name)) {
            files.push(SourceFile::new(*name, text));
        }
    }

    // The one listing, and it is bounded by a directory whose contents are
    // workflows by definition.
    let Ok(entries) = std::fs::read_dir(root.join(WORKFLOW_DIR)) else {
        return files;
    };
    let mut workflows: Vec<String> = entries
        .filter_map(|entry| {
            let path = entry.ok()?.path();
            let extension = path.extension()?.to_str()?;
            (extension == "yml" || extension == "yaml")
                .then(|| path.file_name()?.to_str().map(str::to_string))?
        })
        .collect();
    // Sorted here, because a directory listing has no order a golden snapshot
    // can hold still.
    workflows.sort();
    for name in workflows {
        let relative = format!("{WORKFLOW_DIR}/{name}");
        if let Ok(text) = std::fs::read_to_string(root.join(&relative)) {
            files.push(SourceFile::new(relative, text));
        }
    }

    files
}

#[cfg(test)]
mod tests {
    use super::*;

    fn write(root: &Path, relative: &str, text: &str) {
        let path = root.join(relative);
        std::fs::create_dir_all(path.parent().expect("a parent")).expect("mkdir");
        std::fs::write(path, text).expect("write");
    }

    #[test]
    fn only_the_candidates_and_the_workflows_are_read() {
        let dir = tempfile::tempdir().unwrap();
        write(dir.path(), "package.json", "{}");
        write(dir.path(), "README.md", "# not evidence");
        write(dir.path(), ".github/workflows/ci.yml", "jobs: {}");
        write(dir.path(), ".github/workflows/notes.md", "not a workflow");
        write(dir.path(), "node_modules/left-pad/package.json", "{}");

        let read = read(dir.path());
        let paths: Vec<&str> = read.iter().map(|f| f.path.as_str()).collect();
        assert_eq!(paths, ["package.json", ".github/workflows/ci.yml"]);
    }

    /// A directory listing has no order, and a golden snapshot needs one.
    #[test]
    fn workflows_come_back_in_a_stable_order() {
        let dir = tempfile::tempdir().unwrap();
        for name in ["release.yml", "ci.yml", "nightly.yaml"] {
            write(dir.path(), &format!(".github/workflows/{name}"), "jobs: {}");
        }
        let paths: Vec<String> = read(dir.path()).into_iter().map(|f| f.path).collect();
        assert_eq!(
            paths,
            [
                ".github/workflows/ci.yml",
                ".github/workflows/nightly.yaml",
                ".github/workflows/release.yml",
            ]
        );
    }

    #[test]
    fn an_empty_directory_reads_as_no_evidence_rather_than_as_a_failure() {
        let dir = tempfile::tempdir().unwrap();
        assert!(read(dir.path()).is_empty());
    }
}
