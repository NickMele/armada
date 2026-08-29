//! The glob dialect, stated as tests.
//!
//! **`packages/**` and `packages/*` differing silently is the failure this
//! whole feature exists to prevent**, so the two are asserted against the same
//! paths side by side rather than each being tested on its own.

use alloc::string::{String, ToString};
use alloc::vec::Vec;

use crate::job::covers::{BadPattern, Covers, PathPattern};

fn pattern(written: &str) -> PathPattern {
    PathPattern::parse(written).expect("a pattern this dialect reads")
}

fn covering(patterns: &[&str]) -> Covers {
    Covers::of(patterns.iter().map(|p| pattern(p)).collect()).expect("a non-empty list")
}

fn paths(paths: &[&str]) -> Vec<String> {
    paths.iter().map(|p| p.to_string()).collect()
}

#[test]
fn one_star_stops_at_a_separator_and_two_do_not() {
    let one = pattern("packages/*");
    let two = pattern("packages/**");
    assert!(one.matches("packages/tokens.css"));
    assert!(two.matches("packages/tokens.css"));
    // The whole point of the pair: only one of them reaches a nested file.
    assert!(!one.matches("packages/components/src/Badge.tsx"));
    assert!(two.matches("packages/components/src/Badge.tsx"));
}

#[test]
fn two_stars_match_no_segments_at_all() {
    assert!(pattern("packages/**").matches("packages"));
    assert!(pattern("**/Cargo.toml").matches("Cargo.toml"));
    assert!(pattern("**/Cargo.toml").matches("crates/store/Cargo.toml"));
}

#[test]
fn a_pattern_matches_a_whole_path_and_never_a_prefix() {
    let covers = pattern("packages");
    assert!(covers.matches("packages"));
    // No prefix is inferred. `packages` covering everything under it would be
    // a second, unwritten dialect sitting inside this one.
    assert!(!covers.matches("packages/tokens.css"));
}

#[test]
fn a_star_inside_a_segment_matches_part_of_a_name() {
    assert!(pattern("crates/*/src/**").matches("crates/store/src/read.rs"));
    assert!(pattern("**/*.rs").matches("crates/store/src/read.rs"));
    assert!(!pattern("**/*.rs").matches("crates/store/Cargo.toml"));
    assert!(pattern("apps/desktop/**/*.test.ts").matches("apps/desktop/src/a/b.test.ts"));
}

#[test]
fn a_dialect_this_one_is_not_is_refused_rather_than_read_literally() {
    // Each of these means something in a dialect a person might be thinking
    // of. Read as literal text every one of them is a pattern that matches
    // nothing and says nothing about why — which is the Check that stops
    // running and nobody notices.
    for written in ["src/?.rs", "src/[abc].rs", "src/{a,b}.rs", "!packages/**"] {
        assert!(
            matches!(
                PathPattern::parse(written),
                Err(BadPattern::Unsupported { .. })
            ),
            "{written} should be refused"
        );
    }
}

#[test]
fn a_pattern_that_is_not_shaped_like_one_is_refused_by_name() {
    assert_eq!(PathPattern::parse(""), Err(BadPattern::Empty));
    assert_eq!(PathPattern::parse("   "), Err(BadPattern::Empty));
    assert_eq!(
        PathPattern::parse("/packages/**"),
        Err(BadPattern::Absolute)
    );
    assert_eq!(
        PathPattern::parse("packages/"),
        Err(BadPattern::TrailingSlash)
    );
    assert_eq!(
        PathPattern::parse("packages//src"),
        Err(BadPattern::EmptySegment)
    );
    assert!(matches!(
        PathPattern::parse("packages/**x"),
        Err(BadPattern::StarStarNotAlone { .. })
    ));
    assert!(matches!(
        PathPattern::parse("packages/***"),
        Err(BadPattern::TooManyStars { .. })
    ));
}

#[test]
fn an_empty_list_is_not_a_cover() {
    // `when: []` and no `when` at all would otherwise be one value with
    // opposite meanings — never, and always.
    assert_eq!(Covers::of(Vec::new()), None);
}

#[test]
fn a_deleted_file_is_a_change_to_the_directory_it_was_in() {
    // The kind of change is not a parameter anywhere in this module. What
    // `changed_files` reports for a deletion is the old path, so a file removed
    // from `packages/` arrives as a path under `packages/` and covers the
    // Check that watches it.
    let covers = covering(&["packages/**"]);
    assert!(covers.matches_any(&paths(&["packages/components/src/Gone.tsx"])));
}

#[test]
fn both_sides_of_a_rename_cover_on_their_own() {
    // The git adapter runs no rename detection, so a rename arrives as two
    // paths — the old one deleted and the new one added. Either alone is
    // enough, which is what makes a rename *out of* a covered directory still
    // run the Check that covered it.
    let covers = covering(&["packages/**"]);
    assert!(covers.matches_any(&paths(&["packages/old.ts", "apps/desktop/new.ts"])));
    assert!(covers.matches_any(&paths(&["crates/store/old.rs", "packages/new.ts"])));
    assert!(!covers.matches_any(&paths(&["crates/store/old.rs", "crates/store/new.rs"])));
}

#[test]
fn any_pattern_covering_any_path_is_enough() {
    let covers = covering(&["packages/**", "apps/desktop/**"]);
    assert!(covers.matches_any(&paths(&[
        "crates/fleet/src/gate.rs",
        "apps/desktop/src/a.ts"
    ])));
    assert!(!covers.matches_any(&paths(&["crates/fleet/src/gate.rs"])));
    assert!(!covers.matches_any(&[]));
}

#[test]
fn the_written_form_is_what_a_message_quotes() {
    assert_eq!(
        covering(&["packages/**", "apps/desktop/**"]).written(),
        "packages/**, apps/desktop/**"
    );
    // Trimmed at parse, so a padded entry in a YAML list quotes back clean.
    assert_eq!(pattern("  packages/**  ").as_str(), "packages/**");
}
