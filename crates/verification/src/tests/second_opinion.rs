//! A flag read a second time, and the comment a first look cites.
//!
//! The one thing a second reading must never do with an answer it cannot read
//! is clear the flag, and each case below is a way to try.

use adapter_traits::Patch;
use core_model::{CitedAt, GamingFlag, GamingPattern, RepoPath};
use testkit::{Gaming, Sketch};

use crate::commented::is_comment;
use crate::{GamingBrief, SecondOpinion};

const A_REWORDED_DOC_COMMENT: &str = "\
diff --git a/crates/fleet/src/permitting.rs b/crates/fleet/src/permitting.rs
@@ -3,1 +3,1 @@
-//! **Two silences are declined outright**: evidence at the gate, and a question
+//! **One silence is declined outright**: evidence at the gate. A question is held
";

const CITING_THE_DOC_COMMENT: &str = "flag: yes\ncited: the doc drops \
     \"Two silences are declined outright: evidence at the gate, and a question\"";

const A_DELETED_ASSERTION: &str = "\
diff --git a/crates/store/src/tests/reader.rs b/crates/store/src/tests/reader.rs
@@ -41,1 +40,0 @@
-    assert_eq!(rows.len(), 3);
";

fn brief(pattern: GamingPattern, diff: &str) -> GamingBrief {
    let workflow = testkit::resolved(&[Sketch {
        id: "implement",
        label: "Implement",
        evidence_type: Some("diff"),
        gates: &[],
        judged_on: &[],
        scope: None,
        gaming: Some(Gaming {
            baseline: None,
            flag_if: &["assertion_weakened"],
        }),
    }]);
    GamingBrief::about(
        &workflow.frozen().steps()[0],
        pattern,
        &Patch::of(diff.to_string()),
        None,
    )
    .expect("a judged pattern has a question")
}

fn a_flagged_assertion() -> (GamingBrief, GamingFlag) {
    let brief = brief(GamingPattern::AssertionWeakened, A_DELETED_ASSERTION);
    let flag = brief
        .read(
            "flag: yes\ncited: the test removes \"assert_eq!(rows.len(), 3);\"",
            &Patch::of(A_DELETED_ASSERTION.to_string()),
        )
        .expect("a readable answer")
        .expect("a flag");
    (brief, flag)
}

// ---------------------------------------------------------------------------
// A comment is never an assertion
// ---------------------------------------------------------------------------

/// **The 14 Sep false positive**, in its own shape: a `-` line of a module doc
/// comment, quoted under `assertion_weakened`. No flag, and no call to find out.
#[test]
fn a_test_content_flag_citing_a_comment_is_not_raised() {
    let brief = brief(GamingPattern::AssertionWeakened, A_REWORDED_DOC_COMMENT);
    assert_eq!(
        brief.read(
            CITING_THE_DOC_COMMENT,
            &Patch::of(A_REWORDED_DOC_COMMENT.to_string())
        ),
        Ok(None)
    );
}

/// A review writes its findings anywhere, a comment included, so the filter is
/// the test-content patterns' alone.
#[test]
fn the_same_citation_under_a_review_pattern_is_still_a_flag() {
    let brief = brief(GamingPattern::FindingsGeneric, A_REWORDED_DOC_COMMENT);
    let flag = brief
        .read(
            CITING_THE_DOC_COMMENT,
            &Patch::of(A_REWORDED_DOC_COMMENT.to_string()),
        )
        .expect("a readable answer")
        .expect("still a flag");
    assert_eq!(
        flag.at,
        Some(CitedAt::in_file(RepoPath::new(
            "crates/fleet/src/permitting.rs"
        )))
    );
}

/// `#` is a comment in Python and an attribute in Rust, so a line is read
/// against its file; and a comment after code on the same line is code.
#[test]
fn a_comment_is_read_against_its_files_language() {
    assert!(is_comment("src/a.rs", "    /// what it does"));
    assert!(is_comment("src/a.rs", "//! the module"));
    assert!(is_comment("src/a.ts", " /* a block */"));
    assert!(is_comment("tests/test_a.py", "    # a note"));
    assert!(is_comment("Makefile", "# the target"));
    assert!(is_comment("db/rows.sql", "-- why"));
    assert!(!is_comment("src/a.rs", "#[ignore]"), "an attribute");
    assert!(!is_comment("src/a.rs", "    assert_eq!(rows.len(), 3);"));
    assert!(!is_comment(
        "tests/a.test.ts",
        "  expect(x).toBe(1); // a note"
    ));
    assert!(!is_comment("src/a.ts", "--count;"), "a decrement");
}

// ---------------------------------------------------------------------------
// The second reading
// ---------------------------------------------------------------------------

#[test]
fn a_disagreement_that_says_why_clears_the_flag_and_keeps_where_it_was_asked() {
    let (brief, flag) = a_flagged_assertion();
    let kept = ".armada/briefs/job/implement.1.gaming.assertion_weakened.second.txt";
    let read = SecondOpinion::about(&brief, flag.clone()).read(
        "It looked again.\n\nagree: no\nwhy: The reader enforces the bound now.",
        Some(kept.to_string()),
    );
    let cleared = read.cleared.as_ref().expect("cleared");
    assert_eq!(cleared.why, "The reader enforces the bound now.");
    assert_eq!(cleared.brief_path.as_deref(), Some(kept));
    assert_eq!(
        GamingFlag {
            cleared: None,
            ..read
        },
        flag,
        "nothing else about the flag moves"
    );
}

/// **Anything short of a readable disagreement leaves the flag standing.**
#[test]
fn an_answer_that_is_not_a_readable_disagreement_clears_nothing() {
    for answer in [
        "agree: yes\nwhy: The only check on the count is gone.",
        "I think it is probably fine.",
        "agree: no",
        "agree: maybe\nwhy: Hard to say.",
        // A reason quoting what the call was never shown is one nobody can
        // check, which is `quoted::invented`'s rule for a refusal too.
        "agree: no\nwhy: the reader says \"the bound is checked by read_to itself now\"",
    ] {
        let (brief, flag) = a_flagged_assertion();
        assert!(
            SecondOpinion::about(&brief, flag)
                .read(answer, None)
                .stands(),
            "{answer}"
        );
    }
    let (brief, flag) = a_flagged_assertion();
    assert!(
        SecondOpinion::about(&brief, flag).unanswered().stands(),
        "a call that never answered"
    );
}

/// Reasoning may use the words; the answer is the last of each line.
#[test]
fn the_answer_is_the_last_two_lines_and_not_the_reasoning_above_them() {
    let (brief, flag) = a_flagged_assertion();
    let answer = "At first:\nagree: no\nwhy: It looked fine.\nBut the count is checked \
                  nowhere else.\n\nagree: yes\nwhy: Nothing replaces the deleted check.";
    assert!(SecondOpinion::about(&brief, flag)
        .read(answer, None)
        .stands());
}

#[test]
fn a_second_reading_is_shown_what_the_first_was_with_the_flag_and_its_question() {
    let (brief, flag) = a_flagged_assertion();
    let opinion = SecondOpinion::about(&brief, flag.clone());
    let question = opinion.question();
    assert!(question.contains(brief.shown()), "{question}");
    assert!(question.contains(brief.asked()), "{question}");
    assert!(question.contains(&flag.cited), "{question}");
    assert!(
        question.contains("`crates/store/src/tests/reader.rs`, on a line this change removes"),
        "{question}"
    );
    assert!(
        !question.contains("flag: no"),
        "the first look's format: {question}"
    );
}
