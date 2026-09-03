//! What a request's own link resolves to, as a value — no process, the same
//! as `judge`'s cases beside these.

use adapter_traits::LinkLookup;

use crate::IssueLookup;

#[test]
fn a_bare_issue_link_resolves_to_a_gh_call() {
    let call = IssueLookup
        .resolve("https://github.com/NickMele/armada/issues/90")
        .expect("a call");
    assert_eq!(call.program(), "gh");
    assert_eq!(
        call.args(),
        [
            "issue",
            "view",
            "90",
            "--repo",
            "NickMele/armada",
            "--json",
            "title,body",
            "--jq",
            r#".title + "\n\n" + .body"#,
        ]
    );
}

#[test]
fn a_link_with_no_scheme_still_resolves() {
    let call = IssueLookup
        .resolve("please look at github.com/NickMele/armada/issues/90 today")
        .expect("a call");
    assert_eq!(call.args()[2], "90");
}

#[test]
fn a_trailing_query_or_fragment_does_not_join_the_number() {
    let call = IssueLookup
        .resolve("https://github.com/NickMele/armada/issues/90?tab=comments")
        .expect("a call");
    assert_eq!(call.args()[2], "90");
}

#[test]
fn a_pull_request_link_is_not_an_issue_link() {
    assert!(IssueLookup
        .resolve("https://github.com/NickMele/armada/pull/90")
        .is_none());
}

#[test]
fn plain_prose_resolves_to_nothing() {
    assert!(IssueLookup
        .resolve("fix the flaky test in the checkout step")
        .is_none());
}

#[test]
fn a_non_numeric_issue_segment_resolves_to_nothing() {
    assert!(IssueLookup
        .resolve("https://github.com/NickMele/armada/issues/latest")
        .is_none());
}
