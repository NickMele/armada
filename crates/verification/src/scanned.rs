//! What the patch alone says about gaming, for the three patterns it answers.
//!
//! **No model is called here.** A pattern the diff answers and one a Judge
//! answers are told apart by `GamingPattern::decided_by`, and this reads only
//! the first kind. Split off `gaming.rs`, which is the judged half.
//!
//! # `check_config_edited` is a name match, never a command expansion
//!
//! Armada refuses to model what `pnpm test` resolves to, so this does not try.
//! It answers the weaker, honest question — did the step edit a file that
//! configures how commands run — and lets a legitimate edit flag rather than
//! block. See `docs/concepts/workflow.md`, Evidence gaming.

use core_model::{CitedAt, GamingFlag, GamingPattern, RepoPath};

use adapter_traits::Patch;

use crate::located::{header_path, hunk_start};

/// Files that configure how a command runs.
///
/// **A name match, and deliberately not an expansion of the step's `run:`
/// string** — see this module's comment. The set is the ecosystems Armada is
/// pointed at rather than a claim to be exhaustive; a repository whose gate
/// resolves through something not named here is not covered, which is the
/// honest state of the open question this pattern was raised against.
const CHECK_CONFIG_FILES: &[&str] = &[
    "package.json",
    "pnpm-workspace.yaml",
    "turbo.json",
    "nx.json",
    "makefile",
    "gnumakefile",
    "justfile",
    "rakefile",
    "taskfile.yml",
    "cargo.toml",
    "pyproject.toml",
    "pytest.ini",
    "tox.ini",
    "setup.cfg",
    "go.mod",
    "pom.xml",
    "build.gradle",
    "phpunit.xml",
    ".rspec",
];

/// Filename fragments that make a configuration file, whatever it is called
/// around them — `jest.config.ts`, `vitest.config.mjs`, `.mocharc.json`.
const CHECK_CONFIG_FRAGMENTS: &[&str] = &[".config.", "mocharc", "karma.conf"];

/// What a skip marker looks like, across the ecosystems a Manifest points at.
const SKIP_MARKERS: &[&str] = &[
    ".skip(",
    ".todo(",
    "xit(",
    "xdescribe(",
    "#[ignore]",
    "@ignore",
    "@disabled",
    "pytest.mark.skip",
    "unittest.skip",
    "t.skip(",
    "t.skipnow(",
];

/// What makes a path a test file.
const TEST_MARKERS: &[&str] = &["test", "spec", "__tests__"];

/// Every flag the patch alone establishes, for the patterns the step declared.
///
/// **No model is called.** A pattern the diff answers and a pattern a Judge
/// answers are told apart by [`GamingPattern::decided_by`], so a call site
/// cannot spend money on one of these by accident.
///
/// **Every flag from here names a file**, because the scan found it by walking
/// one — and only the skip marker names a line, because only it is about a
/// line the change leaves behind. [`located`](mod@crate::located) is the same
/// question asked of a judged answer.
pub fn in_the_diff(patch: &Patch, patterns: &[GamingPattern]) -> Vec<GamingFlag> {
    let wanted = |pattern: GamingPattern| patterns.contains(&pattern);
    let mut flags = Vec::new();
    let mut path = String::new();
    let mut deleted = false;
    // Which post-image line the next added or context line is. `None` until a
    // hunk header sets it, so no line number is offered before one has.
    let mut post: Option<u32> = None;
    for line in patch.as_str().lines() {
        if let Some(named) = header_path(line) {
            path = named;
            deleted = false;
            post = None;
            continue;
        }
        if let Some(start) = hunk_start(line) {
            post = Some(start);
            continue;
        }
        if line.starts_with("deleted file mode") {
            deleted = true;
            // The header order is `diff --git`, then the mode line, so the path
            // is already known by the time this is read.
            if wanted(GamingPattern::TestDeleted) && is_test_path(&path) {
                flags.push(GamingFlag {
                    pattern: GamingPattern::TestDeleted,
                    cited: format!("`{path}` is removed whole"),
                    // The file and never a line: there is no post image of a
                    // file the change removes, so every line in it is gone.
                    at: Some(CitedAt::in_file(RepoPath::new(&path))),
                    // Nothing was asked and nothing was kept: the patch said
                    // it, and there is no call for a person to read back.
                    asked: None,
                    brief_path: None,
                    cleared: None,
                });
            }
            continue;
        }
        let Some(added) = line.strip_prefix('+') else {
            if line.starts_with(' ') {
                post = post.map(|n| n + 1);
            }
            continue;
        };
        if added.starts_with("++") {
            continue;
        }
        let at = post;
        post = post.map(|n| n + 1);
        if deleted {
            continue;
        }
        if wanted(GamingPattern::TestSkipped) {
            if let Some(marker) = skip_marker(added) {
                flags.push(GamingFlag {
                    pattern: GamingPattern::TestSkipped,
                    cited: format!("`{path}` gains `{marker}`: {}", added.trim()),
                    // The marker is on a line the change writes, so the change
                    // leaves it in the file and the number is navigable.
                    at: Some(match at {
                        Some(line) => CitedAt::at_line(RepoPath::new(&path), line),
                        None => CitedAt::in_file(RepoPath::new(&path)),
                    }),
                    asked: None,
                    brief_path: None,
                    cleared: None,
                });
            }
        }
    }
    if wanted(GamingPattern::CheckConfigEdited) {
        flags.extend(config_edits(patch));
    }
    flags.sort_by_key(|flag| flag.pattern);
    flags
}

/// Every configuration file the patch touches, once each.
fn config_edits(patch: &Patch) -> Vec<GamingFlag> {
    let mut seen: Vec<String> = Vec::new();
    for line in patch.as_str().lines() {
        let Some(path) = header_path(line) else {
            continue;
        };
        if is_check_config(&path) && !seen.contains(&path) {
            seen.push(path);
        }
    }
    seen.into_iter()
        .map(|path| GamingFlag {
            pattern: GamingPattern::CheckConfigEdited,
            cited: format!("`{path}` configures how a command runs, and this change edits it"),
            // The file and no line. The finding is that this file was edited
            // at all, so naming one of its lines would narrow a claim that is
            // about the whole of it.
            at: Some(CitedAt::in_file(RepoPath::new(&path))),
            asked: None,
            brief_path: None,
            cleared: None,
        })
        .collect()
}

fn is_test_path(path: &str) -> bool {
    let lower = path.to_ascii_lowercase();
    TEST_MARKERS.iter().any(|marker| lower.contains(marker))
}

fn is_check_config(path: &str) -> bool {
    let lower = path.to_ascii_lowercase();
    let name = lower.rsplit('/').next().unwrap_or(&lower);
    CHECK_CONFIG_FILES.contains(&name)
        || CHECK_CONFIG_FRAGMENTS
            .iter()
            .any(|fragment| name.contains(fragment))
}

fn skip_marker(line: &str) -> Option<&'static str> {
    let lower = line.to_ascii_lowercase();
    SKIP_MARKERS
        .iter()
        .copied()
        .find(|marker| lower.contains(marker))
}
