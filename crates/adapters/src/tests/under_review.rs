//! Reading the forge's answer about a pull request nobody has merged yet.
//!
//! **No process starts here.** What `gh pr view` would answer is a fake's to
//! say, everywhere above this crate; what is under test in this file is the
//! half that runs on this machine — undoing jq's escaping so a comment that was
//! a paragraph is a paragraph again, and totalling a rollup of checks written in
//! two different shapes into the one answer a caller acts on.

use adapter_traits::{ReviewVerdict, WhatPeopleSaid, WhatTheForgeRan};

use crate::under_review::{as_written, folded, what_people_said, what_review_verdict, Checks};

/// The whole reason this file undoes anything: a comment is a paragraph and
/// the wire it arrives on is one line.
#[test]
fn a_remark_that_spanned_four_lines_comes_back_spanning_four_lines() {
    assert_eq!(
        as_written("please\\n\\nfix the\\ttest\\nthanks"),
        "please\n\nfix the\ttest\nthanks"
    );
}

#[test]
fn a_backslash_somebody_typed_survives_and_an_unknown_escape_is_left_alone() {
    assert_eq!(as_written("C:\\\\Users"), "C:\\Users");
    assert_eq!(as_written("\\q"), "\\q");
    assert_eq!(
        as_written("ends with a backslash \\"),
        "ends with a backslash \\"
    );
}

#[test]
fn a_word_this_build_has_no_name_for_is_silence_and_not_an_answer() {
    assert_eq!(what_people_said(""), WhatPeopleSaid::NobodyHasLooked);
    assert_eq!(what_people_said("APPROVED"), WhatPeopleSaid::Approved);
    assert_eq!(what_people_said("MUTINY"), WhatPeopleSaid::Unreadable);
}

#[test]
fn a_rollup_with_nothing_in_it_is_not_a_pass() {
    assert_eq!(Checks::default().totalled(), WhatTheForgeRan::NothingRan);
}

#[test]
fn one_failure_beside_six_pending_is_a_failure() {
    let mut checks = Checks::default();
    checks.saw("build", "COMPLETED", "FAILURE");
    for _ in 0..6 {
        checks.saw("test", "IN_PROGRESS", "");
    }
    let WhatTheForgeRan::SomeFailed { failed, checks } = checks.totalled() else {
        panic!("a check that failed does not become a pass when the rest finish");
    };
    assert_eq!(failed.len(), 1);
    assert_eq!(failed[0].as_written(), "build");
    assert_eq!(checks, 7);
}

#[test]
fn a_status_context_is_read_off_its_state_and_a_check_run_off_its_conclusion() {
    let mut checks = Checks::default();
    // A status context: no `status`, and `state` folded into the same field.
    checks.saw("ci/legacy", "", "PENDING");
    checks.saw("build", "COMPLETED", "SUCCESS");
    assert_eq!(
        checks.totalled(),
        WhatTheForgeRan::StillWaiting {
            finished: 1,
            checks: 2
        }
    );
}

#[test]
fn a_conclusion_this_build_has_no_word_for_is_not_reported_as_a_pass() {
    let mut checks = Checks::default();
    checks.saw("build", "COMPLETED", "SOMETHING_NEW");
    assert!(matches!(
        checks.totalled(),
        WhatTheForgeRan::SomeFailed { .. }
    ));
}

// --------------------------------------- the records the reduction prints

/// Exactly what `jq` printed for a pull request with a change requested, a
/// green check, one still running, a failing status context, a comment
/// spanning lines, a review with a body and a comment the forge named nothing
/// — **copied from a run of the reduction against a forge payload**, tabs and
/// escapes and all, rather than invented here.
fn what_the_reduction_printed() -> Vec<String> {
    [
        "said\tCHANGES_REQUESTED",
        "check\tbuild\tCOMPLETED\tSUCCESS",
        "check\tbridge_test\tIN_PROGRESS\t",
        "check\tci/legacy\t\tFAILURE",
        "remark\tIC_kwDOfirst\tsomeone\t2026-09-08T10:00:00Z\tfirst line\\nsecond\\tline",
        "remark\tPRR_kwDOsecond\treviewer\t2026-09-08T11:00:00Z\tplease fix",
        "remark\t\tnobody\t2026-09-08T12:00:00Z\ta comment the forge named nothing",
    ]
    .iter()
    .map(|line| line.to_string())
    .collect()
}

#[test]
fn the_records_fold_into_one_reading() {
    let read = folded(&what_the_reduction_printed());
    assert_eq!(read.people, WhatPeopleSaid::ChangesRequested);
    let WhatTheForgeRan::SomeFailed { failed, checks } = &read.checks else {
        panic!("the status context failed, whatever the two check runs did");
    };
    assert_eq!(failed[0].as_written(), "ci/legacy");
    assert_eq!(*checks, 3);
    assert_eq!(
        read.remarks.len(),
        2,
        "a comment and a review with a body, and never the one with no handle: \
         nothing could pick it, so a row for it would be one no act could reach"
    );
    assert_eq!(read.remarks[0].id.as_written(), "IC_kwDOfirst");
    assert_eq!(
        read.remarks[0].said.as_written(),
        "first line\nsecond\tline",
        "the paragraph it was written as"
    );
    assert_eq!(read.remarks[0].by.as_written(), "someone");
}

/// **A forge that answered nothing this understood said nothing.** An empty
/// rollup read out of records that never arrived would render a forge nobody
/// could reach as a repository with no automation.
#[test]
fn records_with_no_tag_this_knows_are_no_reading_at_all() {
    let notices = vec![String::from("Warning: something from the tool")];
    let read = folded(&notices);
    assert!(!read.was_answered());
    assert_eq!(read.checks, WhatTheForgeRan::Unreadable);
}

/// A pull request nobody has looked at, with nothing configured to run against
/// it — **an answer, and never the silence above it**.
#[test]
fn a_pull_request_nobody_has_touched_is_an_answer() {
    let read = folded(&[String::from("said\t")]);
    assert!(read.was_answered());
    assert_eq!(read.people, WhatPeopleSaid::NobodyHasLooked);
    assert_eq!(read.checks, WhatTheForgeRan::NothingRan);
    assert!(read.remarks.is_empty());
}

/// **A word this build has no name for is silence**, for [`what_people_said`]'s
/// own reason: `COMMENTED`, `DISMISSED` and `PENDING` are real forge states and
/// none of them is a verdict a person acts on, so the reduction's own `select`
/// keeps them out before this is ever asked.
#[test]
fn a_review_state_that_is_neither_verdict_is_none() {
    assert_eq!(
        what_review_verdict("APPROVED"),
        Some(ReviewVerdict::Approved)
    );
    assert_eq!(
        what_review_verdict("CHANGES_REQUESTED"),
        Some(ReviewVerdict::ChangesRequested)
    );
    assert_eq!(what_review_verdict("COMMENTED"), None);
    assert_eq!(what_review_verdict(""), None);
}

/// One review that both wrote a note and landed a verdict produces both rows —
/// a `remark` a person can pick off the pull request, and a `verdict` naming
/// who and which — and the two are independent readings of the one record.
#[test]
fn a_reviewed_and_a_verdict_are_read_off_the_same_review_independently() {
    let read = folded(&[
        String::from("said\tAPPROVED"),
        String::from("remark\tPRR_1\talice\t2026-09-08T10:00:00Z\tlooks good"),
        String::from("verdict\talice\tAPPROVED"),
    ]);
    assert_eq!(read.remarks.len(), 1);
    assert_eq!(read.remarks[0].by.as_written(), "alice");
    assert_eq!(read.verdicts.len(), 1);
    assert_eq!(read.verdicts[0].by.as_written(), "alice");
    assert_eq!(read.verdicts[0].verdict, ReviewVerdict::Approved);
}

/// A conversation comment's own address on the forge, carried where the forge
/// answers one; a comment the reduction printed no sixth field for carries
/// none.
#[test]
fn a_remarks_url_is_carried_where_the_forge_answers_one() {
    let read = folded(&[
        String::from("said\t"),
        String::from(
            "remark\tIC_1\talice\t2026-09-08T10:00:00Z\tlooks good\thttps://forge.invalid/pull/1#issuecomment-1",
        ),
        String::from("remark\tPRR_1\tbob\t2026-09-08T11:00:00Z\tone more thing"),
    ]);
    assert_eq!(read.remarks.len(), 2);
    assert_eq!(
        read.remarks[0].url.as_ref().map(|url| url.as_written()),
        Some("https://forge.invalid/pull/1#issuecomment-1")
    );
    assert!(
        read.remarks[1].url.is_none(),
        "no sixth field, so no address to carry"
    );
}
