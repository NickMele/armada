//! What the rule matches, and — as much of the point — what it does not. The
//! disk-walking half is not tested directly; see `rules_privacy.rs`'s module
//! comment for why.

use super::*;

/// A read a `let` put the path in a few lines earlier, which is the shape that
/// failed three times.
#[test]
fn a_binding_built_from_transcript_of_and_read_later_is_named() {
    let text = "\
let path = transcript_of(&root, handle, drone);
fleet.turn().await;
let said = std::fs::read_to_string(&path).unwrap_or_default();
";
    assert_eq!(bare_transcript_reads(text, 0), vec![3]);
}

/// The build and the read on one line.
#[test]
fn a_read_of_the_call_itself_is_named() {
    let text = "let said = std::fs::read_to_string(transcript_of(&root, handle, drone));\n";
    assert_eq!(bare_transcript_reads(text, 0), vec![1]);
}

/// A qualified call, and a binding the formatter broke onto its own line.
#[test]
fn a_qualified_call_over_several_lines_is_followed() {
    let text = "\
let at =
    crate::transcript::transcript_of(&root, handle, drone);
let rows = std::fs::read_to_string(&at).expect(\"on disk\");
";
    assert_eq!(bare_transcript_reads(text, 0), vec![3]);
}

/// **The narrowness is the rule.** A fixture read is written before the test
/// starts and cannot race; there are ninety-nine of them across the test
/// modules and none is this rule's business.
#[test]
fn a_read_of_anything_else_is_not_named() {
    let text = "\
let fixture = at.path().join(\"ndjson/happy-path.ndjson\");
let rows = std::fs::read_to_string(&fixture).expect(\"the fixture\");
let log = std::fs::read_to_string(log_of(&root, handle)).expect(\"the log\");
";
    assert!(bare_transcript_reads(text, 0).is_empty());
}

/// A transcript path that is waited on rather than read is what the rule wants.
#[test]
fn a_path_built_and_never_read_is_not_named() {
    let text = "\
let at = transcript_of(&root, handle, drone);
assert!(at.is_file(), \"the migration moved it\");
";
    assert!(bare_transcript_reads(text, 0).is_empty());
}

/// **The false positive the first run over this repository found.** The next
/// function binds the same name to the Job's log, and reading that is not this
/// rule's business.
#[test]
fn a_name_bound_to_something_else_is_no_longer_a_transcript() {
    let text = "\
fn rows(at: &TempDir) -> String {
    let path = transcript_of(&root, handle, drone);
    std::fs::read_to_string(path).expect(\"the transcript\")
}

fn job_log(at: &TempDir) -> String {
    let path = log_of(&root, handle);
    std::fs::read_to_string(path).expect(\"the log\")
}
";
    assert_eq!(bare_transcript_reads(text, 0), vec![3]);
}

/// A binding whose name is a prefix of another word is not that word.
#[test]
fn a_longer_name_that_starts_with_the_binding_is_not_it() {
    let text = "\
let path = transcript_of(&root, handle, drone);
let rows = std::fs::read_to_string(&pathological).unwrap();
";
    assert!(bare_transcript_reads(text, 0).is_empty());
}

/// Production code reads this file for real, and the seam is the test's alone.
#[test]
fn a_file_with_no_test_module_has_nowhere_to_look() {
    let text = "let file = fs::File::open(transcript_of(&root, handle, drone)).await;\n";
    assert_eq!(
        where_the_tests_start("crates/fleet/src/backfill.rs", text),
        None
    );
}

#[test]
fn a_file_under_src_tests_is_a_test_module_throughout() {
    assert_eq!(
        where_the_tests_start("crates/fleet/src/tests/peers.rs", ""),
        Some(0)
    );
}

#[test]
fn an_inline_cfg_test_block_is_read_from_where_it_opens() {
    let text = "\
fn of(root: &str) -> PathBuf {
    transcripts_dir(root)
}

#[cfg(test)]
mod tests {
    use super::*;
}
";
    assert_eq!(
        where_the_tests_start("crates/fleet/src/transcript/mod.rs", text),
        Some(5)
    );
}

/// `#[cfg(test)] mod tests;` names a file the path half already covers, and a
/// declaration is not a block.
#[test]
fn a_declared_test_module_is_not_an_inline_one() {
    let text = "#[cfg(test)]\nmod tests;\n";
    assert_eq!(where_the_tests_start("xtask/src/rules_docs.rs", text), None);
}

/// The walking half, over the repository itself — the same shape
/// `rules_errors` uses to say the tree is clean today. It names the sites, so a
/// failure here is read rather than re-derived.
#[test]
fn the_repository_reads_every_transcript_through_the_one_reader() {
    let report = no_bare_transcript_read_in_a_test(&crate::repo_root());
    let named: Vec<&str> = report
        .findings
        .iter()
        .map(|finding| match finding {
            crate::Finding::Fail(what) | crate::Finding::Warn(what) => what.as_str(),
        })
        .collect();
    assert!(named.is_empty(), "{named:#?}");
}
