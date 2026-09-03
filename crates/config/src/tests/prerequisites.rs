//! `checks.<name>.requires`: the Commands a Check needs run before it.
//!
//! Its own file rather than more of `manifest`, because the subject is a
//! cross-registry resolution rather than a key's shape — and because
//! `manifest.rs` is already the length where a reader stops finding things.
//!
//! **The failure these exist against is a real one.** A Job spent all three
//! `implement` attempts failing `format` — `cargo fmt --all --check` — on a
//! different unformatted line each time, while the Manifest beside it already
//! declared `commands.fmt: cargo fmt --all`. Nothing could make the fix run,
//! because the key that says so was designed and never built (`#387`).

use crate::error::Fault;
use crate::manifest::Manifest;
use crate::tests::{fault_at, named, refusals, refused as refused_at};

fn parse(text: &str) -> Result<Manifest, crate::LoadError> {
    Manifest::parse(&named("armada.yml"), text)
}

/// The Manifest the incident wanted: a Check whose fix is declared beside it.
const FIXED: &str = r#"
version: 1
id: armada
checks:
  format:
    run: cargo fmt --all --check
    requires: [fmt]
commands:
  fmt:
    run: cargo fmt --all
"#;

#[test]
fn a_check_carries_the_command_line_of_what_runs_before_it() {
    // Resolved at load, so nothing downstream performs a lookup that could
    // miss. Both halves: the name is what a person edits, the line is what
    // they re-run by hand.
    let manifest = parse(FIXED).expect("a declared prerequisite");
    let requires = manifest.check("format").expect("format").requires();
    assert_eq!(requires.len(), 1);
    assert_eq!(requires[0].name(), "fmt");
    assert_eq!(requires[0].run(), "cargo fmt --all");
}

#[test]
fn a_check_that_declares_none_requires_nothing() {
    // Absent is empty, and empty is what every Manifest written before this key
    // existed means. There is no third answer for it to become.
    let manifest = parse("version: 1\nid: a\nchecks:\n  build:\n    run: cargo build\n")
        .expect("no requires at all");
    assert!(manifest
        .check("build")
        .expect("build")
        .requires()
        .is_empty());
}

#[test]
fn the_order_the_file_wrote_is_the_order_kept() {
    // `[migrate, seed]` is a sequence somebody wrote, and a set would run the
    // second against what the first had not done yet.
    let manifest = parse(
        "version: 1\nid: a\nchecks:\n  e2e:\n    run: pnpm e2e\n    requires: [migrate, seed]\n\
         commands:\n  migrate:\n    run: pnpm migrate\n  seed:\n    run: pnpm seed\n",
    )
    .expect("two prerequisites");
    let names: Vec<&str> = manifest
        .check("e2e")
        .expect("e2e")
        .requires()
        .iter()
        .map(core_model::Prerequisite::name)
        .collect();
    assert_eq!(names, ["migrate", "seed"]);
}

#[test]
fn a_command_declared_below_the_check_that_needs_it_still_resolves() {
    // The registries are read independently and joined afterwards, so nobody
    // writing an `armada.yml` has to think about which section comes first.
    let manifest = parse(FIXED).expect("commands declared after checks");
    assert_eq!(
        manifest.check("format").expect("format").requires()[0].run(),
        "cargo fmt --all"
    );
}

#[test]
fn a_prerequisite_naming_no_declared_command_fails_the_manifest() {
    // **The Manifest, not the Job.** A name that resolves to nothing would
    // otherwise be found at a gate, by a Drone, with a worktree checked out and
    // a retry budget already being spent — which is the shape of the failure
    // this whole key exists to end.
    let refused = refusals(parse(
        "version: 1\nid: a\nchecks:\n  format:\n    run: x\n    requires: [fmt]\n",
    ));
    assert!(matches!(
        fault_at(&refused, "checks.format.requires[0]"),
        Fault::NotADeclaredCommand { value, is_a_check, .. } if value == "fmt" && !is_a_check
    ));
}

#[test]
fn a_prerequisite_naming_a_check_says_so_rather_than_saying_nothing_declares_it() {
    // A real name in the wrong registry is a different mistake with a different
    // fix, and the message says which.
    let refused = refusals(parse(
        "version: 1\nid: a\nchecks:\n  build:\n    run: x\n  test:\n    run: y\n    requires: [build]\n",
    ));
    assert!(matches!(
        fault_at(&refused, "checks.test.requires[0]"),
        Fault::NotADeclaredCommand {
            is_a_check: true,
            ..
        }
    ));
}

#[test]
fn one_name_written_twice_in_a_checks_requires_is_refused() {
    // Running it twice costs the second run and changes nothing, and silently
    // de-duplicating would decide on the author's behalf what they meant.
    let refused = refusals(parse(
        "version: 1\nid: a\nchecks:\n  format:\n    run: x\n    requires: [fmt, fmt]\n\
         commands:\n  fmt:\n    run: y\n",
    ));
    assert!(matches!(
        fault_at(&refused, "checks.format.requires[1]"),
        Fault::RequiredTwice { first_at: 0 }
    ));
}

#[test]
fn a_destructive_command_cannot_be_a_checks_prerequisite() {
    // The flag means a person approves before it runs. A prerequisite runs
    // inside Fleet's own gate, with the Drone already waiting on the answer and
    // nobody to ask.
    let refused = refusals(parse(
        "version: 1\nid: a\nchecks:\n  test:\n    run: x\n    requires: [reset]\n\
         commands:\n  reset:\n    run: y\n    destructive: true\n",
    ));
    assert!(matches!(
        fault_at(&refused, "checks.test.requires[0]"),
        Fault::RequiresSomethingDestructive { value } if value == "reset"
    ));
}

#[test]
fn an_empty_requires_on_a_check_is_refused_rather_than_read_as_none() {
    // `when`'s answer, for `when`'s reason: a list with nothing in it is a key
    // to delete, and reading it as "requires nothing" would make two different
    // files mean one thing.
    let refused = refusals(parse(
        "version: 1\nid: a\nchecks:\n  test:\n    run: x\n    requires: []\n",
    ));
    assert!(matches!(
        fault_at(&refused, "checks.test.requires"),
        Fault::Empty
    ));
}

#[test]
fn every_bad_prerequisite_in_one_file_is_reported_in_one_pass() {
    // Two bad names is one edit, on both Checks and across them.
    let refused = refusals(parse(
        "version: 1\nid: a\nchecks:\n  one:\n    run: x\n    requires: [nope]\n\
         \x20 two:\n    run: y\n    requires: [also_nope]\n",
    ));
    assert!(refused_at(&refused, "checks.one.requires[0]"));
    assert!(refused_at(&refused, "checks.two.requires[0]"));
}
