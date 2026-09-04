//! `armada.yml`: what M1 reads, and everything that is not one of them.
//!
//! The header and a test name counted the keys, and the count was already
//! wrong before `#414` added `drone:` — it said seven where the module said
//! eight and listed nine. Counting again would only move the number, so the
//! counts are gone and the list lives in `crate::manifest`.

use crate::error::Fault;
use crate::manifest::Manifest;
use crate::tests::{fault_at, named, refusals, refused as refused_at};

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
setup:
  requires:
    - fmt
"#;

fn parse(text: &str) -> Result<Manifest, crate::LoadError> {
    Manifest::parse(&named("armada.yml"), text)
}

#[test]
fn every_key_m1_reads_parses_and_nothing_else_is_needed() {
    let manifest = parse(WHOLE).expect("every key M1 reads");
    assert_eq!(manifest.version(), 1);
    assert_eq!(manifest.id().as_str(), "armada");
    assert_eq!(manifest.base(), Some("main"));
    assert_eq!(manifest.check_names(), ["build", "test"]);
    assert_eq!(manifest.command_names(), ["fmt", "reset"]);
    assert_eq!(
        manifest.check("test").map(super::super::Check::run),
        Some("cargo nextest run --workspace")
    );
    assert_eq!(manifest.prepared_by().len(), 1);
}

#[test]
fn destructive_is_read_when_present_and_false_when_absent() {
    let manifest = parse(WHOLE).expect("every key M1 reads");
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
    assert_eq!(
        *known,
        ["version", "id", "base", "checks", "commands", "setup", "drone"]
    );
}

#[test]
fn an_unknown_key_inside_a_check_hard_fails_too() {
    // `timeout` is on the concept page and is not one of the three this parser
    // reads. The refusal names the exact key, not the Check.
    let refused = refusals(parse(
        "version: 1\nid: armada\nchecks:\n  test:\n    run: cargo test\n    timeout: 600\n",
    ));
    assert!(matches!(
        fault_at(&refused, "checks.test.timeout"),
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

// ------------------------------------------------------- setup.requires

#[test]
fn a_required_command_arrives_resolved_to_the_line_that_runs() {
    // The whole reason it is resolved at load: a caller holds the name **and**
    // the command line, so a failure can say which entry of the file it was
    // and what was actually executed.
    let manifest = parse(WHOLE).expect("every key M1 reads");
    let prepared = manifest.prepared_by();
    assert_eq!(prepared.len(), 1);
    assert_eq!(prepared[0].name(), "fmt");
    assert_eq!(prepared[0].run(), "cargo fmt --all");
}

#[test]
fn a_repository_that_requires_nothing_says_so_with_an_empty_list_of_its_own() {
    let manifest = parse("version: 1\nid: a\n").expect("no setup at all");
    assert!(manifest.prepared_by().is_empty());
}

#[test]
fn order_is_the_files_and_is_kept() {
    // `[install, generate]` is a sequence somebody wrote. Sorting it — which
    // every other registry in this file is — would run the second before what
    // it depends on.
    let manifest = parse(
        "version: 1\nid: a\ncommands:\n  zeta:\n    run: b\n  alpha:\n    run: a\n\
         setup:\n  requires: [zeta, alpha]\n",
    )
    .expect("two commands in the author's order");
    let named: Vec<&str> = manifest.prepared_by().iter().map(|p| p.name()).collect();
    assert_eq!(named, ["zeta", "alpha"]);
}

#[test]
fn a_name_no_command_declares_is_refused_at_load() {
    // The guard `tests/shipped.rs` already puts on a step naming an undeclared
    // Check, one file earlier. Without it the worktree is never prepared and
    // what a person sees is whichever Check needed what was not installed.
    let refused = refusals(parse(
        "version: 1\nid: a\ncommands:\n  fmt:\n    run: x\nsetup:\n  requires: [bootstrap]\n",
    ));
    let Fault::NotADeclaredCommand {
        value,
        is_a_check,
        declared,
    } = fault_at(&refused, "setup.requires[0]")
    else {
        panic!("expected an undeclared command, got {refused:?}");
    };
    assert_eq!(value, "bootstrap");
    assert!(!is_a_check);
    assert_eq!(declared, &["fmt".to_string()]);
}

#[test]
fn a_name_declared_as_a_check_is_a_different_refusal_from_a_name_declared_nowhere() {
    // The mirror of `UnknownCheck::is_a_command`: a real name in the wrong
    // registry has a different fix from a typo, and one message covering both
    // sends the author looking in the wrong place.
    let refused = refusals(parse(
        "version: 1\nid: a\nchecks:\n  build:\n    run: x\nsetup:\n  requires: [build]\n",
    ));
    assert!(matches!(
        fault_at(&refused, "setup.requires[0]"),
        Fault::NotADeclaredCommand {
            is_a_check: true,
            ..
        }
    ));
}

#[test]
fn one_name_required_twice_is_refused_rather_than_run_twice() {
    let refused = refusals(parse(
        "version: 1\nid: a\ncommands:\n  fmt:\n    run: x\nsetup:\n  requires: [fmt, fmt]\n",
    ));
    assert!(matches!(
        fault_at(&refused, "setup.requires[1]"),
        Fault::RequiredTwice { first_at: 0 }
    ));
}

#[test]
fn a_destructive_command_cannot_be_required() {
    // `fleet::spawning` withholds a destructive Command from a Drone for the
    // reason this refuses one here: the flag asks for an approval, and there is
    // nobody to ask before the first Drone exists.
    let refused = refusals(parse(
        "version: 1\nid: a\ncommands:\n  reset:\n    run: x\n    destructive: true\n\
         setup:\n  requires: [reset]\n",
    ));
    assert!(matches!(
        fault_at(&refused, "setup.requires[0]"),
        Fault::RequiresSomethingDestructive { value } if value == "reset"
    ));
}

#[test]
fn an_empty_requires_is_refused_rather_than_read_as_none() {
    // `when`'s answer, for `when`'s reason: a list with nothing in it is a key
    // to delete, and reading it as "requires nothing" would make two different
    // files mean one thing.
    let refused = refusals(parse(
        "version: 1\nid: a\ncommands:\n  fmt:\n    run: x\nsetup:\n  requires: []\n",
    ));
    assert!(matches!(fault_at(&refused, "setup.requires"), Fault::Empty));
}

#[test]
fn a_setup_with_no_requires_is_refused() {
    let refused = refusals(parse("version: 1\nid: a\nsetup: {}\n"));
    assert!(matches!(
        fault_at(&refused, "setup.requires"),
        Fault::Missing
    ));
}

#[test]
fn requires_is_the_only_key_setup_has() {
    let refused = refusals(parse(
        "version: 1\nid: a\ncommands:\n  fmt:\n    run: x\n\
         setup:\n  requires: [fmt]\n  timeout: 60\n",
    ));
    assert!(matches!(
        fault_at(&refused, "setup.timeout"),
        Fault::Unknown { known } if *known == ["requires"]
    ));
}

#[test]
fn every_bad_requirement_in_one_file_is_reported_in_one_pass() {
    let refused = refusals(parse(
        "version: 1\nid: a\ncommands:\n  fmt:\n    run: x\n\
         setup:\n  requires: [nope, also_nope]\n",
    ));
    assert!(refused_at(&refused, "setup.requires[0]"));
    assert!(refused_at(&refused, "setup.requires[1]"));
}

// `drone:` — the repository's own patience with a quiet Drone. `#414`.

#[test]
fn a_repository_says_how_long_its_drones_may_be_quiet_and_how_often_they_are_asked() {
    let manifest =
        parse("version: 1\nid: a\ndrone:\n  quiet_after_seconds: 300\n  poke_limit: 3\n")
            .expect("both halves");
    assert_eq!(manifest.quiet_after_seconds(), Some(300));
    assert_eq!(manifest.poke_limit(), Some(3));
}

#[test]
fn a_repository_that_says_nothing_defers_rather_than_defaulting() {
    // Absent is not 120 and not 2. The chain is Fleet's constant, this section
    // and the step, and only `fleet::Liveness::at` knows the order — a number
    // invented here would be a second place it lives.
    let manifest = parse(WHOLE).expect("a file with no `drone` section at all");
    assert_eq!(manifest.quiet_after_seconds(), None);
    assert_eq!(manifest.poke_limit(), None);
}

#[test]
fn each_half_of_the_repository_value_is_written_on_its_own() {
    // Two rows in `settings.toml` and not one pair, which is what buys this: a
    // repository wanting more patience does not thereby want more pokes, and
    // saying so does not mean restating a number it has no opinion about.
    let patient =
        parse("version: 1\nid: a\ndrone:\n  quiet_after_seconds: 300\n").expect("one half");
    assert_eq!(patient.quiet_after_seconds(), Some(300));
    assert_eq!(patient.poke_limit(), None);

    let unasked = parse("version: 1\nid: a\ndrone:\n  poke_limit: 0\n").expect("the other half");
    assert_eq!(unasked.quiet_after_seconds(), None);
    assert_eq!(unasked.poke_limit(), Some(0));
}

#[test]
fn zero_pokes_is_carried_and_zero_patience_is_refused() {
    // The one place the two keys disagree, and each is right about its own. A
    // `poke_limit: 0` says the first silence past the threshold escalates with
    // no nudge, which somebody may mean; a `quiet_after_seconds: 0` says every
    // Drone here is quiet the instant it is spawned, which nobody does. `#60`
    // settled that at the step and the reading is unchanged by the value being
    // written a tier up — one number read two ways is the drift this whole
    // chain exists to avoid.
    assert_eq!(
        parse("version: 1\nid: a\ndrone:\n  poke_limit: 0\n")
            .expect("no nudges at all")
            .poke_limit(),
        Some(0)
    );
    let refused = refusals(parse(
        "version: 1\nid: a\ndrone:\n  quiet_after_seconds: 0\n",
    ));
    assert_eq!(
        fault_at(&refused, "drone.quiet_after_seconds"),
        &Fault::WrongType {
            wanted: "a positive whole number",
            found: "a number outside that range",
        }
    );
}

#[test]
fn a_drone_section_that_declares_neither_key_is_refused() {
    // `setup: {}`'s rule, for a section where both keys are optional: the
    // author wrote the section and left it blank, which is a key to delete
    // rather than a repository deferring. `Table::close` reports nothing for an
    // empty table, so this is asked here or not at all.
    let refused = refusals(parse("version: 1\nid: a\ndrone: {}\n"));
    assert_eq!(fault_at(&refused, "drone"), &Fault::Empty);
}

#[test]
fn a_key_the_drone_section_does_not_read_hard_fails_and_names_the_two_it_does() {
    // `heartbeat_interval_minutes` is a real `settings.toml` row with a
    // Manifest tier and nothing reading it, so it is exactly the key this
    // refusal keeps out of a file until a reader exists.
    let refused = refusals(parse(
        "version: 1\nid: a\ndrone:\n  quiet_after_seconds: 300\n  heartbeat_interval_minutes: 5\n",
    ));
    assert!(matches!(
        fault_at(&refused, "drone.heartbeat_interval_minutes"),
        Fault::Unknown { known } if *known == ["quiet_after_seconds", "poke_limit"]
    ));
}

#[test]
fn a_patience_that_is_not_a_number_is_refused_by_name() {
    let refused = refusals(parse(
        "version: 1\nid: a\ndrone:\n  quiet_after_seconds: five minutes\n  poke_limit: lots\n",
    ));
    assert_eq!(
        fault_at(&refused, "drone.quiet_after_seconds"),
        &Fault::WrongType {
            wanted: "a positive whole number",
            found: "text",
        }
    );
    assert_eq!(
        fault_at(&refused, "drone.poke_limit"),
        &Fault::WrongType {
            wanted: "a whole number of zero or more",
            found: "text",
        }
    );
}
