//! `after_merge.checks`: which of a repository's Checks are worth running
//! against the tree a merge left behind.
//!
//! Its own file rather than more of `manifest`, for [`crate::tests::prerequisites`]'s
//! reason: the subject is a cross-registry resolution, and `manifest.rs` is
//! already the length where a reader stops finding things.
//!
//! **The section is opt-in and every one of these tests turns on that.** What it
//! costs is a full build and test run on the machine somebody is working on, for
//! an event nobody is waiting for — `docs/practices/rust.md` section 8 names a
//! hook that rebuilt on merge as the cause of v1's real build cost. So absent
//! means nothing runs, and there is no value of this key that means *all of
//! them*. `#474`.

use crate::error::Fault;
use crate::manifest::Manifest;
use crate::tests::{fault_at, named, refusals};

fn parse(text: &str) -> Result<Manifest, crate::LoadError> {
    Manifest::parse(&named("armada.yml"), text)
}

const PROVES_TWO_OF_THREE: &str = r#"
version: 1
id: armada
checks:
  build:
    run: cargo build --workspace
  test:
    run: cargo nextest run --workspace
  storybook:
    run: pnpm build-storybook
    when: ["packages/**"]
after_merge:
  checks: [build, test]
"#;

#[test]
fn the_list_names_what_runs_and_nothing_else_does() {
    // The whole shape of the opt-in: three Checks declared, two named, and the
    // third is not run after a merge because nobody asked for it to be.
    let manifest = parse(PROVES_TWO_OF_THREE).expect("two named Checks");
    let named: Vec<&str> = manifest
        .proved_after_a_merge()
        .iter()
        .map(|check| check.label())
        .collect();
    assert_eq!(named, vec!["build", "test"]);
}

#[test]
fn a_manifest_that_says_nothing_proves_nothing() {
    // **Absent is off, and off is the default.** A repository that never wrote
    // the section never spends a person's machine on a merge.
    let manifest = parse("version: 1\nid: a\nchecks:\n  build:\n    run: cargo build\n")
        .expect("no after_merge at all");
    assert!(manifest.proved_after_a_merge().is_empty());
}

#[test]
fn the_command_line_comes_across_resolved() {
    // Resolved at load, like `setup.requires`, so nothing at the moment of a
    // merge performs a lookup that could miss.
    let manifest = parse(PROVES_TWO_OF_THREE).expect("two named Checks");
    let core_model::ResolvedCheck::ManifestCheck { run, .. } = &manifest.proved_after_a_merge()[0]
    else {
        panic!("a Manifest Check");
    };
    assert_eq!(run, "cargo build --workspace");
}

#[test]
fn a_checks_own_when_is_dropped() {
    // **`when` answers *did this step touch anything I cover*, and after a
    // merge there is no step to ask it of.** The list is the filter, so a Check
    // named here runs whatever the merge changed — the alternative is a Check
    // that silently never runs because nothing computed a diff to compare it
    // against.
    let manifest = parse(
        "version: 1\nid: a\nchecks:\n  storybook:\n    run: pnpm build-storybook\n    \
         when: [\"packages/**\"]\nafter_merge:\n  checks: [storybook]\n",
    )
    .expect("a covered Check, named anyway");
    let core_model::ResolvedCheck::ManifestCheck { when, .. } = &manifest.proved_after_a_merge()[0]
    else {
        panic!("a Manifest Check");
    };
    assert!(when.is_none());
}

#[test]
fn a_name_no_check_declares_is_refused_at_load() {
    // The same question `requires` asks of `commands`, asked of the other
    // registry — and asked here rather than an hour later, when somebody merges
    // and a run finds nothing to do.
    let refused = refusals(parse(
        "version: 1\nid: a\nchecks:\n  build:\n    run: cargo build\n\
         after_merge:\n  checks: [tset]\n",
    ));
    let Fault::NotADeclaredCheck {
        value, declared, ..
    } = fault_at(&refused, "after_merge.checks[0]")
    else {
        panic!("{refused:?}");
    };
    assert_eq!(value, "tset");
    assert_eq!(declared, &vec![String::from("build")]);
}

#[test]
fn a_command_named_where_a_check_belongs_is_told_which_registry_it_is_in() {
    // A real name in the wrong registry is a different mistake with a different
    // fix, and the message says so rather than listing what is declared.
    let refused = refusals(parse(
        "version: 1\nid: a\nchecks:\n  build:\n    run: cargo build\n\
         commands:\n  fmt:\n    run: cargo fmt --all\nafter_merge:\n  checks: [fmt]\n",
    ));
    assert!(matches!(
        fault_at(&refused, "after_merge.checks[0]"),
        Fault::NotADeclaredCheck {
            is_a_command: true,
            ..
        }
    ));
}

#[test]
fn a_check_that_prepares_the_worktree_is_refused() {
    // **The refusal `#474` is really about.** A prerequisite writes in the tree
    // it runs in — `fmt` rewrites files — and the tree here is the repository a
    // person is standing in. Fleet fast-forwards that repository only over a
    // clean checkout and refuses everything else; a run that then reformatted it
    // would put back exactly what those refusals exist to keep out.
    let refused = refusals(parse(
        "version: 1\nid: a\nchecks:\n  format:\n    run: cargo fmt --all --check\n    \
         requires: [fmt]\ncommands:\n  fmt:\n    run: cargo fmt --all\n\
         after_merge:\n  checks: [format]\n",
    ));
    let Fault::PreparesTheRepository { value, requires } =
        fault_at(&refused, "after_merge.checks[0]")
    else {
        panic!("{refused:?}");
    };
    assert_eq!(value, "format");
    assert_eq!(requires, "fmt");
}

#[test]
fn one_name_written_twice_is_refused() {
    // Running a suite twice against one commit costs the second run and changes
    // nothing, which is the same sentence `requires` already carries.
    let refused = refusals(parse(
        "version: 1\nid: a\nchecks:\n  build:\n    run: cargo build\n\
         after_merge:\n  checks: [build, build]\n",
    ));
    assert!(matches!(
        fault_at(&refused, "after_merge.checks[1]"),
        Fault::RequiredTwice { first_at: 0 }
    ));
}

#[test]
fn a_section_with_nothing_under_it_is_refused() {
    // `after_merge:` with no `checks` says nothing, exactly as `setup:` with no
    // `requires` does — and `Table::close` reports no fault for an empty table,
    // so the key is required rather than left to it.
    let refused = refusals(parse(
        "version: 1\nid: a\nchecks:\n  build:\n    run: cargo build\nafter_merge: {}\n",
    ));
    assert!(matches!(
        fault_at(&refused, "after_merge.checks"),
        Fault::Missing
    ));
}

#[test]
fn a_key_the_section_does_not_read_is_refused() {
    // A key nothing reads is a promise the system does not keep — the rule the
    // whole parser is built on, and the reason every deferred section stays
    // additive.
    let refused = refusals(parse(
        "version: 1\nid: a\nchecks:\n  build:\n    run: cargo build\n\
         after_merge:\n  checks: [build]\n  every: true\n",
    ));
    assert!(matches!(
        fault_at(&refused, "after_merge.every"),
        Fault::Unknown { .. }
    ));
}
