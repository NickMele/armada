//! charkit's own `char.yml`, held to the same contract every other repo's is —
//! and held to the merge gate it is eventually going to replace.
//!
//! **Dogfooding is staged** (`ARCHITECTURE.md` §2.6). Through phase 6 the gate
//! runs the raw tools and charkit's config is a *test subject*; once phase 6
//! lands, `char check` becomes the gate. The reason for the interim is that a
//! gate which is also the thing under construction fails every PR when it
//! breaks, including the PR that fixes it.
//!
//! **What is deliberately not here yet, stated rather than stubbed.**
//! `ARCHITECTURE.md` §2.6 asks for a test that runs `char check --json` and
//! asserts it reaches the same verdict as the raw tools. That test cannot be
//! written honestly before the engine runs — an `#[ignore]`d body or one
//! asserting on a `bad_invocation` would be a green test that structurally
//! cannot fail, which is the failure mode this corpus records three times over.
//! It lands with the verb, in the phase-3 PR that makes `char check` execute.
//!
//! What *is* available now is the half that does not need the verb, and it is
//! not filler: it is the drift the phase-6 flip actually depends on. If this
//! config and `.github/workflows/gate.yml` disagree about what "lint" means,
//! then the day `char check` becomes the gate the gate silently changes.

mod support;

use armada_core::config::{parse, resolve, Defaults, ResolvedCheck, Scope};
use std::collections::{BTreeMap, BTreeSet};
use std::path::{Path, PathBuf};
use support::compile_schema;

/// The repository root, from this crate's manifest directory.
fn repo_root() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR")).join("../..")
}

fn read(path: &Path) -> String {
    std::fs::read_to_string(path).unwrap_or_else(|e| panic!("{}: {e}", path.display()))
}

fn own_config_text() -> String {
    read(&repo_root().join("char.yml"))
}

/// charkit's own checks, resolved, keyed by their derived id.
fn own_checks() -> BTreeMap<String, ResolvedCheck> {
    let text = own_config_text();
    let config = parse(&text, "char.yml").unwrap_or_else(|e| panic!("charkit's char.yml: {e}"));
    let resolved = resolve(config, &Defaults::built_in(), "char.yml")
        .unwrap_or_else(|e| panic!("charkit's char.yml: {e}"));
    resolved
        .components
        .values()
        .flat_map(|component| component.checks.values())
        .map(|check| (check.id.clone(), check.clone()))
        .collect()
}

#[test]
fn charkits_own_config_parses_resolves_and_validates_against_the_schema() {
    let text = own_config_text();

    // Parse and resolve: the path phase 2 already takes for any repo's config.
    let config = parse(&text, "char.yml").unwrap_or_else(|e| panic!("charkit's char.yml: {e}"));
    resolve(config, &Defaults::built_in(), "char.yml")
        .unwrap_or_else(|e| panic!("charkit's char.yml: {e}"));

    // And the authoritative artifact, which is the schema rather than the
    // structs (PLAN.md §4.1.1 decision 2). Both, because either one alone
    // accepts documents the other rejects — that is the whole reason the
    // fixture suite runs them together.
    let (schemas, index) = compile_schema();
    let instance: serde_json::Value =
        serde_yaml_ng::from_str(&text).expect("charkit's char.yml reads as a JSON value");
    if let Err(e) = schemas.validate(&instance, index) {
        panic!("charkit's char.yml does not validate:\n{e:#}");
    }
}

/// Check ids are **derived** as `<component>:<check>` and never written by
/// hand (PLAN.md §4.1). Asserting the exact set is what makes a rename of
/// either half visible — and this is also the list `char check <selector>` has
/// to be able to reach, so it is the closest thing to a selector contract that
/// exists before the verb does.
#[test]
fn every_check_id_resolves_and_the_set_is_the_one_the_gate_covers() {
    let ids: Vec<String> = own_checks().keys().cloned().collect();
    assert_eq!(
        ids,
        vec![
            "charkit:boundaries",
            "charkit:docs",
            "charkit:fmt",
            "charkit:lint",
            "charkit:test",
        ],
        "charkit's own check ids changed"
    );
}

/// **The drift catcher, and the reason this file is worth having before the
/// verb exists.**
///
/// `ARCHITECTURE.md` §2.6 flips the gate to `char check` in phase 6. That flip
/// is only safe if the two agree *now*: a `lint` check spelled `cargo clippy`
/// with the `-D warnings` dropped would pass here, pass in a dogfood run, and
/// silently weaken the gate on the day it replaces it.
///
/// One direction only, deliberately. The gate legitimately runs things that are
/// not checks — the toolchain probe, `cargo build`, the MSRV job, the coverage
/// ratchet — so "every `run:` line is a check" is false and asserting it would
/// force this config to grow entries nothing wants.
///
/// **Whole commands, never a substring, and that is not fastidiousness.**
/// Written the obvious way — `gate_text.contains(cmd)` — this test passes when
/// `lint` is weakened from `cargo clippy --all-targets -- -D warnings` to
/// `cargo clippy --all-targets`, because the weaker string is a prefix of the
/// stronger one. Measured by inverting it: that mutation was the one drift the
/// substring form could not see, and it is precisely the drift that matters.
#[test]
fn every_check_command_is_one_the_merge_gate_actually_runs() {
    let commands = gate_commands();
    for (id, check) in own_checks() {
        assert!(
            commands.contains(&check.cmd),
            "{id}: `{}` is not a command .github/workflows/gate.yml runs — \
             the config and the gate have drifted, and phase 6 flips one into the other.\n\
             The gate runs: {commands:#?}",
            check.cmd
        );
    }
}

/// Every command any `run:` step in the gate executes, one per line, trimmed.
///
/// Read through the YAML parser rather than by grepping for `run:`, so a step
/// written as a block scalar contributes each of its lines and a `run` that is
/// a value rather than a key contributes nothing.
fn gate_commands() -> BTreeSet<String> {
    let text = read(&repo_root().join(".github/workflows/gate.yml"));
    let document: serde_yaml_ng::Value =
        serde_yaml_ng::from_str(&text).expect("the gate workflow is YAML");
    let mut out = BTreeSet::new();
    collect_run_steps(&document, &mut out);
    out
}

fn collect_run_steps(node: &serde_yaml_ng::Value, out: &mut BTreeSet<String>) {
    match node {
        serde_yaml_ng::Value::Mapping(map) => {
            for (key, value) in map {
                if key.as_str() == Some("run") {
                    if let Some(script) = value.as_str() {
                        out.extend(
                            script
                                .lines()
                                .map(str::trim)
                                .filter(|line| !line.is_empty())
                                .map(str::to_string),
                        );
                    }
                }
                collect_run_steps(value, out);
            }
        }
        serde_yaml_ng::Value::Sequence(items) => {
            for item in items {
                collect_run_steps(item, out);
            }
        }
        _ => {}
    }
}

/// Every check here is `scope: component`, and that is cargo's property rather
/// than a default being dodged.
///
/// `cargo clippy`, `cargo fmt --check` and `cargo test` take a workspace and
/// not a file list, so a `scope: file` spelling would promise a `${files}` none
/// of them could receive — and a file-scoped check with an empty set is
/// `SKIPPED` (PLAN.md §4.1), which on the default branch would report a green
/// run that checked nothing.
///
/// The consequence is stated so it is not mistaken for coverage: **charkit's
/// own config exercises no `${files}` path at all.** The fixtures do.
#[test]
fn charkits_checks_are_component_scoped_and_none_of_them_can_expand_files() {
    for (id, check) in own_checks() {
        assert_eq!(check.scope, Scope::Component, "{id}");
        assert!(
            !check.cmd.contains("${files}"),
            "{id}: `${{files}}` in a component-scoped check is bad_config"
        );
    }
}
