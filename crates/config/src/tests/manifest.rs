//! `armada.yml`: the six keys, and everything that is not one of them.

use crate::error::Fault;
use crate::manifest::Manifest;
use crate::tests::{fault_at, named, refusals};

/// The whole of what M1 reads, in one file.
const WHOLE: &str = r#"
version: 1
id: armada
base: main
checks:
  build:
    run: cargo build --workspace
  test:
    run: cargo nextest run --workspace
commands:
  fmt:
    run: cargo fmt --all
  reset:
    run: rm -rf .armada/store.db
    destructive: true
"#;

fn parse(text: &str) -> Result<Manifest, crate::LoadError> {
    Manifest::parse(&named("armada.yml"), text)
}

#[test]
fn the_six_keys_parse_and_nothing_else_is_needed() {
    let manifest = parse(WHOLE).expect("the six keys");
    assert_eq!(manifest.version(), 1);
    assert_eq!(manifest.id().as_str(), "armada");
    assert_eq!(manifest.base(), Some("main"));
    assert_eq!(manifest.check_names(), ["build", "test"]);
    assert_eq!(manifest.command_names(), ["fmt", "reset"]);
    assert_eq!(
        manifest.check("test").map(super::super::Check::run),
        Some("cargo nextest run --workspace")
    );
}

#[test]
fn destructive_is_read_when_present_and_false_when_absent() {
    let manifest = parse(WHOLE).expect("the six keys");
    assert!(manifest.command("reset").expect("reset").is_destructive());
    assert!(!manifest.command("fmt").expect("fmt").is_destructive());
}

#[test]
fn a_repository_that_names_no_base_says_nothing_rather_than_guessing() {
    // Absent is not `main`. Inference happens where a repository can be read,
    // and a default written here would be a guess nothing could tell from an
    // answer.
    let manifest = parse("version: 1\nid: tooling\n").expect("no base at all");
    assert_eq!(manifest.base(), None);
}

#[test]
fn a_base_that_is_not_text_is_refused() {
    let refused = refusals(parse("version: 1\nid: armada\nbase: 7\n"));
    assert_eq!(
        fault_at(&refused, "base"),
        &Fault::WrongType {
            wanted: "text",
            found: "a number",
        }
    );
}

#[test]
fn a_workspace_that_gates_nothing_is_legal() {
    // Nearest-ancestor ownership means a workspace declaring no Checks is
    // ungated rather than malformed. Requiring `checks` would make the empty
    // case unwritable.
    let manifest = parse("version: 1\nid: tooling\n").expect("no registries at all");
    assert!(manifest.check_names().is_empty());
    assert!(manifest.command_names().is_empty());
}

#[test]
fn a_section_m1_does_not_read_hard_fails_and_names_what_it_does_read() {
    // Every deferred section of the concept page arrives this way. Refusing it
    // is what keeps the section additive later rather than a migration.
    let refused = refusals(parse(
        "version: 1\nid: armada\nbudget:\n  cap: 40\nports:\n  web:\n    container: 3000\n",
    ));
    assert!(matches!(
        fault_at(&refused, "budget"),
        Fault::Unknown { .. }
    ));
    let Fault::Unknown { known } = fault_at(&refused, "ports") else {
        panic!("ports should be an unknown key");
    };
    assert_eq!(*known, ["version", "id", "base", "checks", "commands"]);
}

#[test]
fn an_unknown_key_inside_a_check_hard_fails_too() {
    // `requires` and `timeout` are both on the concept page and neither is one
    // of the five. The refusal names the exact key, not the Check.
    let refused = refusals(parse(
        "version: 1\nid: armada\nchecks:\n  test:\n    run: cargo test\n    timeout: 600\n    requires: [migrate]\n",
    ));
    assert!(matches!(
        fault_at(&refused, "checks.test.timeout"),
        Fault::Unknown { .. }
    ));
    assert!(matches!(
        fault_at(&refused, "checks.test.requires"),
        Fault::Unknown { .. }
    ));
}

#[test]
fn a_name_in_both_registries_is_refused_at_load() {
    // The two registries mean different things — one gates advancement, one
    // does not — so a reference to `build` would resolve to two commands with
    // two meanings and nothing in the file says which.
    let refused = refusals(parse(
        "version: 1\nid: armada\nchecks:\n  build:\n    run: cargo build\ncommands:\n  build:\n    run: make\n",
    ));
    assert_eq!(
        fault_at(&refused, "commands.build"),
        &Fault::DeclaredInBothRegistries
    );
}

#[test]
fn a_check_with_no_command_is_refused() {
    let refused = refusals(parse(
        "version: 1\nid: armada\nchecks:\n  test:\n    expect_exit_code: 0\n",
    ));
    assert_eq!(fault_at(&refused, "checks.test.run"), &Fault::Missing);
}

#[test]
fn an_empty_command_is_a_different_fault_from_a_missing_one() {
    let refused = refusals(parse(
        "version: 1\nid: armada\ncommands:\n  fmt:\n    run: \"\"\n",
    ));
    assert_eq!(fault_at(&refused, "commands.fmt.run"), &Fault::Empty);
}

#[test]
fn destructive_that_is_not_a_flag_is_refused() {
    let refused = refusals(parse(
        "version: 1\nid: armada\ncommands:\n  reset:\n    run: rm -rf db\n    destructive: yes please\n",
    ));
    assert_eq!(
        fault_at(&refused, "commands.reset.destructive"),
        &Fault::WrongType {
            wanted: "true or false",
            found: "text",
        }
    );
}

#[test]
fn every_fault_in_the_file_is_reported_not_only_the_first() {
    // The reason this matters is the workflow samples: under a bail-on-first
    // parser the contradiction those files carry is never reached.
    let refused = refusals(parse("checks:\n  test:\n    walk: cargo test\n"));
    assert_eq!(fault_at(&refused, "version"), &Fault::Missing);
    assert_eq!(fault_at(&refused, "id"), &Fault::Missing);
    assert_eq!(fault_at(&refused, "checks.test.run"), &Fault::Missing);
    assert!(matches!(
        fault_at(&refused, "checks.test.walk"),
        Fault::Unknown { .. }
    ));
}

#[test]
fn a_refusal_names_the_file_the_key_and_what_was_wrong() {
    // "Refuse loudly" is the requirement, and a message naming two of the three
    // sends somebody back to the file to guess which line it meant.
    let error = parse("version: 1\nid: armada\nchecks:\n  test: cargo test\n")
        .expect_err("a check is a map, not a string");
    let message = error.to_string();
    assert!(message.contains("armada.yml"), "{message}");
    assert!(message.contains("checks.test"), "{message}");
    assert!(message.contains("wanted a map and holds text"), "{message}");
}

#[test]
fn a_version_that_is_not_a_positive_whole_number_is_refused() {
    let refused = refusals(parse("version: \"1\"\nid: armada\n"));
    assert_eq!(
        fault_at(&refused, "version"),
        &Fault::WrongType {
            wanted: "a positive whole number",
            found: "text",
        }
    );
}

#[test]
fn bytes_that_are_not_yaml_carry_the_parser_error_as_a_cause() {
    // The Error Contract wants a real cause chain rather than a formatted
    // string, so the line and column the parser found stay traversable.
    use std::error::Error;
    let error = parse("version: 1\n  id: armada\n").expect_err("not YAML");
    assert!(error.source().is_some(), "{error}");
    assert!(error.to_string().contains("armada.yml"), "{error}");
}

#[test]
fn one_key_written_twice_is_refused_rather_than_won_by_the_last() {
    // The parser refuses a duplicate mapping key outright, so this arrives as
    // `NotYaml` rather than as a keyed refusal. That is the loud answer and it
    // is checked here because the quiet one — last write wins — would let an
    // `armada.yml` say two things about one Check and act on one of them.
    let error = parse("version: 1\nid: armada\nid: other\n").expect_err("one id, twice");
    assert!(matches!(error, crate::LoadError::NotYaml { .. }), "{error}");
    assert!(error.to_string().contains("duplicate"), "{error}");
}

/// A Check that says which paths it covers, beside one that says nothing.
const SCOPED: &str = r#"
version: 1
id: armada
checks:
  build:
    run: cargo build --workspace
  storybook:
    run: pnpm -C packages/components build-storybook
    when: ["packages/**", "apps/desktop/**"]
"#;

#[test]
fn a_check_may_say_which_paths_it_covers() {
    let manifest = parse(SCOPED).expect("a `when` this dialect reads");
    let storybook = manifest.check("storybook").expect("storybook");
    let covers = storybook.when().expect("two patterns");
    assert_eq!(covers.written(), "packages/**, apps/desktop/**");
    assert!(covers.matches_any(&["packages/components/src/Badge.tsx".to_string()]));
    assert!(!covers.matches_any(&["crates/fleet/src/gate.rs".to_string()]));
}

#[test]
fn a_check_with_no_when_says_none_and_never_covers_nothing() {
    // **Absent means always**, and it is `None` here rather than an empty list
    // so the two cannot be confused downstream. Every `armada.yml` written
    // before `when` existed lands on this branch.
    let manifest = parse(SCOPED).expect("a `when` this dialect reads");
    assert_eq!(manifest.check("build").expect("build").when(), None);
}

#[test]
fn a_when_that_is_not_a_list_is_refused() {
    let refused = refusals(parse(
        "version: 1\nid: a\nchecks:\n  build:\n    run: x\n    when: packages/**\n",
    ));
    assert!(matches!(
        fault_at(&refused, "checks.build.when"),
        Fault::WrongType {
            wanted: "a list",
            ..
        }
    ));
}

#[test]
fn an_empty_when_is_refused_rather_than_read_as_always() {
    // `when: []` is a Check that can never run. Reading it as "always" would
    // be the parser deciding the author meant the opposite of what they wrote;
    // reading it as "never" would be a Check that silently stops running.
    let refused = refusals(parse(
        "version: 1\nid: a\nchecks:\n  build:\n    run: x\n    when: []\n",
    ));
    assert!(matches!(
        fault_at(&refused, "checks.build.when"),
        Fault::Empty
    ));
}

#[test]
fn a_pattern_from_another_dialect_is_refused_and_names_itself() {
    let refused = refusals(parse(
        "version: 1\nid: a\nchecks:\n  build:\n    run: x\n    when: [\"src/[ab].rs\"]\n",
    ));
    let fault = fault_at(&refused, "checks.build.when[0]");
    assert!(matches!(fault, Fault::NotAPathPattern { value, .. } if value == "src/[ab].rs"));
    // The message says which character, so the author can see which dialect
    // they were writing in rather than being told the pattern is wrong.
    assert!(fault.to_string().contains('['), "{fault}");
}

#[test]
fn every_bad_pattern_in_one_file_is_reported_in_one_pass() {
    let refused = refusals(parse(
        "version: 1\nid: a\nchecks:\n  build:\n    run: x\n    when: [\"/abs\", \"trailing/\"]\n",
    ));
    assert!(super::refused(&refused, "checks.build.when[0]"));
    assert!(super::refused(&refused, "checks.build.when[1]"));
}

#[test]
fn when_is_the_only_key_a_check_gained() {
    // The header's rule is unchanged: every key but `run` and `when` is refused
    // by name, so a deferred section still arrives with the code that honours
    // it.
    let refused = refusals(parse(
        "version: 1\nid: a\nchecks:\n  build:\n    run: x\n    timeout: 60\n",
    ));
    assert!(matches!(
        fault_at(&refused, "checks.build.timeout"),
        Fault::Unknown { known } if known.contains(&"when")
    ));
}
