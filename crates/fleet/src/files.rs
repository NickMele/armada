//! Walking the checkout for `search_files` — `ipc::FilesFound`'s own notes
//! carry the rest of the reasoning.
//!
//! **Not `.gitignore`-aware, and that is a named gap rather than a decision.**
//! `crates/ipc/operations.toml` says the same: the bound and the ignore rules
//! both need a repository sized in the design workspace before either is
//! settled, and this excludes the handful of directories large enough to make
//! an uninformed walk useless in the meantime.

use std::path::{Path, PathBuf};

/// Directories never descended into. Each is either enormous, generated, or
/// both — walking it buys no result an `@` mention would ever want.
const EXCLUDED: &[&str] = &[".git", "node_modules", "target"];

/// How many paths `search` answers with at most. **A stop, not a truncation of
/// a sorted list** — the walk gives up as soon as it has this many rather than
/// reading the whole tree and cutting the tail, which is what keeps one
/// keystroke from costing an unbounded read on a repository nobody has sized.
const RESULT_CAP: usize = 50;

/// Every file under `root` whose path, relative to `root`, contains `query`
/// case-insensitively — `query` empty matches everything, up to the cap.
///
/// **Never fails.** A subdirectory this cannot read is skipped rather than
/// failing the search: `ipc::FilesFound`'s own notes carry the reasoning.
pub(crate) fn search(root: &Path, query: &str) -> Vec<String> {
    let needle = query.to_lowercase();
    let mut found = Vec::new();
    let mut pending = vec![PathBuf::new()];
    while let Some(relative) = pending.pop() {
        if found.len() >= RESULT_CAP {
            break;
        }
        let Ok(entries) = std::fs::read_dir(root.join(&relative)) else {
            continue;
        };
        for entry in entries.flatten() {
            if found.len() >= RESULT_CAP {
                break;
            }
            let name = entry.file_name();
            let Some(name) = name.to_str() else { continue };
            if EXCLUDED.contains(&name) {
                continue;
            }
            let child = relative.join(name);
            let Ok(file_type) = entry.file_type() else {
                continue;
            };
            if file_type.is_dir() {
                pending.push(child);
                continue;
            }
            if !file_type.is_file() {
                continue;
            }
            // Forward-slashed regardless of host, so the path a Windows Fleet
            // walks reads the same as the one every other platform in this
            // workspace already assumes — `docs/practices/protocol.md`'s
            // same-machine rule for a staged path draws the same line.
            let text = child.to_string_lossy().replace('\\', "/");
            if needle.is_empty() || text.to_lowercase().contains(needle.as_str()) {
                found.push(text);
            }
        }
    }
    found.sort();
    found
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::tests::tmp::TempDir;

    /// A checkout with a nested file, one of the three excluded directories,
    /// and a file that only matches by case.
    fn a_checkout() -> TempDir {
        let home = TempDir::new();
        std::fs::create_dir_all(home.path().join("src/log")).expect("a nested directory");
        std::fs::write(home.path().join("src/log/reader.rs"), b"").expect("a file");
        std::fs::write(home.path().join("README.md"), b"").expect("a file");
        std::fs::create_dir_all(home.path().join("node_modules/left-pad")).expect("excluded");
        std::fs::write(home.path().join("node_modules/left-pad/index.js"), b"")
            .expect("a file under an excluded directory");
        home
    }

    /// An empty query matches every file under the root, not none of them —
    /// this is what lets a popup opened on a bare `@` show something to pick
    /// from rather than an empty list.
    #[test]
    fn an_empty_query_matches_everything_excluded_directories_aside() {
        let home = a_checkout();
        let found = search(home.path(), "");
        assert_eq!(found, vec!["README.md", "src/log/reader.rs"]);
    }

    /// `.git`, `node_modules` and `target` are never descended into, whatever
    /// is typed — the walk never reads what is under them at all.
    #[test]
    fn the_three_excluded_directories_are_never_descended_into() {
        let home = a_checkout();
        assert!(search(home.path(), "left-pad").is_empty());
        assert!(search(home.path(), "index").is_empty());
    }

    /// The match is case-insensitive, against the path relative to the root —
    /// a person typing `@REA` still finds `README.md`.
    #[test]
    fn the_match_is_case_insensitive() {
        let home = a_checkout();
        assert_eq!(search(home.path(), "reAdMe"), vec!["README.md"]);
    }

    /// Narrowed to nothing is an empty list, not every path — the two must
    /// stay distinguishable, or a popup narrowed to nothing would draw as one
    /// nobody has typed into yet.
    #[test]
    fn a_query_matching_nothing_answers_empty() {
        let home = a_checkout();
        assert!(search(home.path(), "zzz-nothing-matches-this").is_empty());
    }

    /// A path is joined with `/` and never `\`, whatever the host — the same
    /// rule a staged path already follows on this seam.
    #[test]
    fn nested_paths_are_forward_slashed() {
        let home = a_checkout();
        let found = search(home.path(), "reader");
        assert_eq!(found, vec!["src/log/reader.rs"]);
        assert!(!found[0].contains('\\'));
    }

    /// **A stop, not a truncation of a sorted list.** More files exist than
    /// the cap, and what comes back is bounded by it rather than filtered
    /// down to it afterwards.
    #[test]
    fn the_result_is_capped() {
        let home = TempDir::new();
        for n in 0..RESULT_CAP + 10 {
            std::fs::write(home.path().join(format!("file-{n:03}.txt")), b"").expect("a file");
        }
        assert_eq!(search(home.path(), "").len(), RESULT_CAP);
    }

    /// A directory this cannot read at all — the root itself missing — is
    /// skipped rather than failing the whole search.
    #[test]
    fn a_root_that_cannot_be_read_answers_empty_rather_than_failing() {
        let missing = std::env::temp_dir().join("armada-files-search-missing-root");
        assert!(search(&missing, "").is_empty());
    }
}
