//! One entry added to a document this crate did not write.
//!
//! What is asserted is mostly what survives: a merge that loses a key loses
//! somebody's configuration, and it does so in a file they will not read again
//! until the thing it configures stops working.

use serde::Serialize;

use crate::merging::{merged_into, NotMerged};

#[derive(Serialize)]
struct Entry {
    command: &'static str,
}

fn merged(existing: Option<&str>) -> Result<String, NotMerged> {
    merged_into(
        existing.map(str::as_bytes),
        "mcpServers",
        "armada-fleet",
        &Entry { command: "armada" },
    )
}

#[test]
fn no_file_yet_is_the_same_answer_as_an_empty_one() {
    let fresh = merged(None).expect("a document");
    let empty = merged(Some("")).expect("a document");
    let braces = merged(Some("{}")).expect("a document");

    assert_eq!(fresh, empty);
    assert_eq!(fresh, braces);
    assert!(fresh.contains("armada-fleet"), "{fresh}");
    assert!(fresh.ends_with('\n'), "a file ends with a newline");
}

#[test]
fn every_other_key_comes_back_out() {
    let written = merged(Some(
        r#"{"mcpServers":{"theirs":{"command":"x"}},"$schema":"somewhere","inputs":[]}"#,
    ))
    .expect("a document");

    for kept in ["theirs", "$schema", "somewhere", "inputs"] {
        assert!(written.contains(kept), "{kept} survived: {written}");
    }
    assert!(written.contains("armada-fleet"), "{written}");
}

/// A second publish replaces the one entry rather than appending a second.
#[test]
fn the_entry_is_replaced_and_not_doubled() {
    let first = merged(Some(r#"{"mcpServers":{"armada-fleet":{"command":"old"}}}"#))
        .expect("a document");

    assert!(!first.contains("old"), "{first}");
    assert_eq!(first.matches("armada-fleet").count(), 1, "{first}");
}

/// **Refused rather than replaced.** A root that is not an object, or a
/// `mcpServers` that is not one, was written by something with a different idea
/// of the file.
#[test]
fn a_document_that_is_not_an_object_is_refused() {
    assert!(matches!(
        merged(Some("[1, 2, 3]")),
        Err(NotMerged::NotAnObject { .. })
    ));
    assert!(matches!(
        merged(Some(r#"{"mcpServers":"none"}"#)),
        Err(NotMerged::NotAnObject { .. })
    ));
    assert!(matches!(
        merged(Some("{ not json")),
        Err(NotMerged::Unreadable(_))
    ));
}
