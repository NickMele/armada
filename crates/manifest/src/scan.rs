//! Finding and reading the files [`armada_core::scan`] parses.
//!
//! The split is §1.2's: opening a file is I/O and lives here, turning its bytes
//! into evidence is a pure function and lives in the core. **The policy is the
//! core's too** — which names are worth reading, how deep to go, and which
//! directories to refuse — so this module chooses nothing; it walks.
//!
//! # Two ways to find the files, and the first one is git
//!
//! **Git already knows exactly what a repository ignores.** A `.gitignore` has
//! negations, precedence, per-directory files and a global excludes file, and a
//! hand-rolled subset of that is a scanner that calls a build output a package
//! on one machine and not on another. So when there is a checkout to ask,
//! `git ls-files --cached --others --exclude-standard` is the listing, and the
//! answer is the repository's own.
//!
//! **The walk is the fallback, not the plan.** A directory that is not a git
//! checkout still deserves a scan — `config scan` is the verb that runs before
//! anything has been set up — so [`SKIP_DIRS`] answers the same question by
//! hand. It is applied on the git path too, for the repository that commits its
//! `vendor/` tree: git would say that is not ignored, and the evidence would
//! still be somebody else's.

use armada_core::ctx::Run;
use armada_core::scan::{SourceFile, CANDIDATES, MAX_DEPTH, SKIP_DIRS, WORKFLOW_DIR};
use std::path::Path;

use crate::git;

/// Read every candidate file under `root`, bounded by depth and by what the
/// repository ignores.
///
/// A file that cannot be read is skipped rather than reported: `scan` exits 0
/// whenever the directory is readable, and one unreadable `Makefile` is not a
/// reason to withhold the other thirteen pieces of evidence.
pub fn read(run: &impl Run, root: &Path) -> Vec<SourceFile> {
    let mut paths = match git::visible_files(run, root) {
        Ok(listed) => listed.into_iter().filter(|p| wanted(p)).collect(),
        Err(_) => walk(root),
    };
    // Sorted here rather than trusted from git or from a directory listing:
    // neither has an order a golden snapshot can hold still.
    paths.sort();
    paths.dedup();

    paths
        .into_iter()
        .filter_map(|relative| {
            let text = std::fs::read_to_string(root.join(&relative)).ok()?;
            Some(SourceFile::new(relative, text))
        })
        .collect()
}

/// Whether a workspace-relative path is one a scan reads.
///
/// Three bounds, and every one of them is the core's constant rather than this
/// module's opinion: the name is interesting, nothing on the way to it is a
/// directory a scan refuses, and it is not deeper than [`MAX_DEPTH`].
fn wanted(relative: &str) -> bool {
    let segments: Vec<&str> = relative.split('/').collect();
    let Some((name, parents)) = segments.split_last() else {
        return false;
    };
    if parents.iter().any(|dir| SKIP_DIRS.contains(dir)) {
        return false;
    }
    // A workflow is named for its job rather than from a fixed list, so it is
    // matched by the directory it lives in — which is the whole reason that
    // directory is evidence.
    if relative.starts_with(&format!("{WORKFLOW_DIR}/")) {
        return parents.len() == 2 && (name.ends_with(".yml") || name.ends_with(".yaml"));
    }
    parents.len() <= MAX_DEPTH && CANDIDATES.contains(name)
}

/// The same set, found by hand, for a directory git cannot answer for.
fn walk(root: &Path) -> Vec<String> {
    let mut found = Vec::new();
    let mut pending = vec![(root.to_path_buf(), String::new())];

    while let Some((dir, prefix)) = pending.pop() {
        let Ok(entries) = std::fs::read_dir(&dir) else {
            continue;
        };
        for entry in entries.flatten() {
            let Some(name) = entry.file_name().to_str().map(str::to_string) else {
                continue;
            };
            let relative = if prefix.is_empty() {
                name.clone()
            } else {
                format!("{prefix}/{name}")
            };
            // `is_dir` follows symlinks, and a symlinked directory is how a
            // bounded walk becomes an unbounded one — a link to `..` is a
            // cycle, and one into `/` is the whole filesystem.
            let kind = entry.file_type();
            if kind.as_ref().is_ok_and(std::fs::FileType::is_symlink) {
                continue;
            }
            if kind.as_ref().is_ok_and(std::fs::FileType::is_dir) {
                // The depth bound is on the directory rather than on the file,
                // so a tree that is too deep is never descended into at all —
                // the difference between reading `node_modules` and refusing to
                // open it.
                let descend =
                    !SKIP_DIRS.contains(&name.as_str()) && relative.split('/').count() <= MAX_DEPTH;
                if descend {
                    pending.push((entry.path(), relative));
                }
                continue;
            }
            if wanted(&relative) {
                found.push(relative);
            }
        }
    }
    found
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::process::RealRun;

    fn write(root: &Path, relative: &str, text: &str) {
        let path = root.join(relative);
        std::fs::create_dir_all(path.parent().expect("a parent")).expect("mkdir");
        std::fs::write(path, text).expect("write");
    }

    fn git_init(root: &Path) {
        for argv in [
            &["init", "-q", "-b", "main"][..],
            &["add", "-A"][..],
            &[
                "-c",
                "user.email=a@b.test",
                "-c",
                "user.name=a",
                "commit",
                "-q",
                "-m",
                "x",
            ][..],
        ] {
            std::process::Command::new("git")
                .args(argv)
                .current_dir(root)
                .output()
                .expect("git runs");
        }
    }

    fn paths(files: &[SourceFile]) -> Vec<&str> {
        files.iter().map(|f| f.path.as_str()).collect()
    }

    /// **The bug, at the layer that caused it.** The evidence is one level
    /// down, and the root holds only the two things the first version found.
    #[test]
    fn a_monorepos_nested_manifests_are_found() {
        let dir = tempfile::tempdir().unwrap();
        write(dir.path(), "docker-compose.yml", "services: {}");
        write(dir.path(), ".github/workflows/ci.yml", "jobs: {}");
        write(dir.path(), "web/package.json", "{}");
        write(dir.path(), "web/pnpm-lock.yaml", "");
        write(dir.path(), "backend/pyproject.toml", "[project]");
        write(dir.path(), "backend/uv.lock", "");
        git_init(dir.path());

        assert_eq!(
            paths(&read(&RealRun, dir.path())),
            [
                ".github/workflows/ci.yml",
                "backend/pyproject.toml",
                "backend/uv.lock",
                "docker-compose.yml",
                "web/package.json",
                "web/pnpm-lock.yaml",
            ]
        );
    }

    /// **What the repository ignores, the scan ignores**, and git is what
    /// answers that — including the rules a hand-rolled parser gets wrong.
    #[test]
    fn a_gitignored_manifest_is_not_evidence() {
        let dir = tempfile::tempdir().unwrap();
        write(dir.path(), ".gitignore", "generated/\n");
        write(dir.path(), "package.json", "{}");
        write(dir.path(), "generated/package.json", "{}");
        git_init(dir.path());

        assert_eq!(paths(&read(&RealRun, dir.path())), ["package.json"]);
    }

    /// The skip list applies on the git path too, for the repository that
    /// commits its `vendor/` tree: git would say that is not ignored, and the
    /// evidence would still be somebody else's.
    #[test]
    fn a_committed_dependency_tree_is_still_refused() {
        let dir = tempfile::tempdir().unwrap();
        write(dir.path(), "package.json", "{}");
        write(dir.path(), "node_modules/left-pad/package.json", "{}");
        write(dir.path(), "vendor/thing/Gemfile", "");
        git_init(dir.path());

        assert_eq!(paths(&read(&RealRun, dir.path())), ["package.json"]);
    }

    /// **A lockfile eight levels down is a vendored copy, not a package.**
    #[test]
    fn nothing_deeper_than_the_bound_is_read() {
        let dir = tempfile::tempdir().unwrap();
        write(dir.path(), "a/b/c/package.json", "{}");
        write(dir.path(), "a/b/c/d/package.json", "{}");
        git_init(dir.path());

        assert_eq!(paths(&read(&RealRun, dir.path())), ["a/b/c/package.json"]);
    }

    /// **The fallback is a real scan, not an empty one.** `config scan` runs
    /// before anything is set up, and a directory nobody has run `git init` in
    /// is exactly that situation.
    #[test]
    fn a_directory_that_is_not_a_checkout_is_walked_instead() {
        let dir = tempfile::tempdir().unwrap();
        write(dir.path(), "package.json", "{}");
        write(dir.path(), "backend/pyproject.toml", "[project]");
        write(dir.path(), "node_modules/left-pad/package.json", "{}");
        write(dir.path(), ".github/workflows/ci.yml", "jobs: {}");
        write(dir.path(), "README.md", "# not evidence");

        assert_eq!(
            paths(&read(&RealRun, dir.path())),
            [
                ".github/workflows/ci.yml",
                "backend/pyproject.toml",
                "package.json",
            ]
        );
    }

    /// A symlinked directory is how a bounded walk becomes an unbounded one.
    #[cfg(unix)]
    #[test]
    fn the_walk_does_not_follow_a_directory_symlink() {
        let dir = tempfile::tempdir().unwrap();
        write(dir.path(), "real/package.json", "{}");
        std::os::unix::fs::symlink(dir.path().join("real"), dir.path().join("loop")).unwrap();

        assert_eq!(paths(&read(&RealRun, dir.path())), ["real/package.json"]);
    }

    #[test]
    fn an_empty_directory_reads_as_no_evidence_rather_than_as_a_failure() {
        let dir = tempfile::tempdir().unwrap();
        assert!(read(&RealRun, dir.path()).is_empty());
    }
}
