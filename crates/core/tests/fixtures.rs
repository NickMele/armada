//! The six fixture configs, and what they are for.
//!
//! `PHASES.md` §8.1: isolation cannot stop an abstraction being shaped around
//! the one repo anyone has seen. Six configs written before any code can,
//! provided each of them can fail the schema in a way no other one can. This
//! file is where that claim is enforced rather than asserted:
//!
//! | Test | What it would catch |
//! |---|---|
//! | parses | a key the structs know and the schema does not |
//! | validates | a key the schema knows and the structs do not |
//! | golden | a rename, a default silently changing, a derivation drifting |
//! | every property used | a key invented on the spot, that no repo shape asked for |
//!
//! **There is no snapshot update flag, deliberately** (`ARCHITECTURE.md`
//! §1.6). On a mismatch the failure prints the resolved config in full; paste
//! it into the `.json` beside the fixture *after reading the diff*. A flag
//! turns this suite into "regenerate, commit, don't read", which is exactly
//! the thing golden tests exist to prevent.

mod support;

use charkit_core::config::{parse, resolve, Defaults, SCHEMA};
use std::collections::BTreeSet;
use std::path::{Path, PathBuf};
use support::compile_schema;

/// The six from `PHASES.md` §8.1. Named rather than discovered, so deleting
/// one is a failing test rather than a quietly smaller suite.
const FIXTURES: &[&str] = &[
    "go-service",
    "multi-lang",
    "pnpm-monorepo",
    "polyglot-web",
    "python-ml",
    "rails-monolith",
];

/// The nested workspace `pnpm-monorepo` declares. It is an ordinary workspace
/// with its own complete config, which is the point of it.
const NESTED: &str = "pnpm-monorepo/apps/site";

fn fixtures_dir() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR")).join("../../tests/fixtures")
}

/// Every config in the fixture tree: the six, plus any nested workspace.
fn all_configs() -> Vec<(String, PathBuf)> {
    let dir = fixtures_dir();
    let mut out: Vec<(String, PathBuf)> = FIXTURES
        .iter()
        .map(|name| ((*name).to_string(), dir.join(name).join("char.yml")))
        .collect();
    out.push((NESTED.to_string(), dir.join(NESTED).join("char.yml")));
    out
}

fn read(path: &Path) -> String {
    std::fs::read_to_string(path).unwrap_or_else(|e| panic!("{}: {e}", path.display()))
}

#[test]
fn every_fixture_is_present() {
    for (name, path) in all_configs() {
        assert!(path.is_file(), "{name}: no char.yml at {}", path.display());
    }
}

#[test]
fn every_fixture_parses_and_resolves() {
    for (name, path) in all_configs() {
        let text = read(&path);
        let config = parse(&text, "char.yml").unwrap_or_else(|e| panic!("{name}: {e}"));
        resolve(config, &Defaults::built_in(), "char.yml")
            .unwrap_or_else(|e| panic!("{name}: {e}"));
    }
}

#[test]
fn every_fixture_validates_against_the_schema() {
    let (schemas, index) = compile_schema();
    for (name, path) in all_configs() {
        let text = read(&path);
        let instance: serde_json::Value = serde_yaml_ng::from_str(&text)
            .unwrap_or_else(|e| panic!("{name}: could not read as a JSON value: {e}"));
        if let Err(e) = schemas.validate(&instance, index) {
            panic!("{name} does not validate:\n{e:#}");
        }
    }
}

#[test]
fn every_fixture_matches_its_golden_resolved_config() {
    let mut stale = Vec::new();
    for (name, path) in all_configs() {
        let text = read(&path);
        let config = parse(&text, "char.yml").unwrap_or_else(|e| panic!("{name}: {e}"));
        let resolved = resolve(config, &Defaults::built_in(), "char.yml")
            .unwrap_or_else(|e| panic!("{name}: {e}"));

        let actual =
            serde_json::to_string_pretty(&resolved).expect("resolved config serializes") + "\n";
        let golden_path = path.with_file_name("resolved.json");
        let expected = read(&golden_path);

        if actual != expected {
            stale.push(format!(
                "--- {name} — {} is stale.\n\
                 --- Read this diff before pasting it in; there is no update flag.\n\
                 {}\n\
                 --- the resolved config in full:\n{actual}",
                golden_path.display(),
                first_difference(&expected, &actual),
            ));
        }
    }
    assert!(stale.is_empty(), "{}", stale.join("\n\n"));
}

/// The gate on inventing keys.
///
/// Phase 1's done-when is that all six fixtures are expressible **with no
/// fields invented on the spot**. Stated that way it is unfalsifiable; stated
/// as "every property this schema defines is used by at least one fixture" it
/// is a test. A key that no repo shape asked for fails here, which is the only
/// pressure against a schema growing keys nothing needs.
#[test]
fn every_schema_property_is_used_by_at_least_one_fixture() {
    let schema: serde_json::Value = serde_json::from_str(SCHEMA).expect("schema is JSON");
    let mut declared = BTreeSet::new();
    collect_properties(&schema, &mut declared);

    let mut used = BTreeSet::new();
    for (_, path) in all_configs() {
        let instance: serde_json::Value =
            serde_yaml_ng::from_str(&read(&path)).expect("fixture is a JSON value");
        collect_keys(&instance, &mut used);
    }

    let unused: Vec<&String> = declared.difference(&used).collect();
    assert!(
        unused.is_empty(),
        "the schema declares keys no fixture uses: {unused:?}\n\
         Either a fixture should exercise it, or the key does not yet exist."
    );
}

/// The schema and the structs must reject the same documents.
///
/// Neither generates the other (PLAN.md §4.1.1, decision 2), so this is what
/// stops them drifting: an unknown key is `deny_unknown_fields` on one side and
/// `additionalProperties: false` on the other, and a fixture that only one of
/// them accepts fails one of the two tests above. Here it is asserted directly
/// on a document neither should take.
#[test]
fn the_schema_and_the_structs_agree_about_an_unknown_key() {
    let doc = "version: 1\ncomponents:\n  api:\n    rooot: services/api\n";

    let parsed = parse(doc, "char.yml");
    assert!(parsed.is_err(), "the structs accepted an unknown key");

    let (schemas, index) = compile_schema();
    let instance: serde_json::Value = serde_yaml_ng::from_str(doc).unwrap();
    assert!(
        schemas.validate(&instance, index).is_err(),
        "the schema accepted an unknown key"
    );
}

/// Every key under any `properties` object — the schema's own vocabulary.
///
/// Plain recursion, because a `properties` map's *values* are themselves
/// schemas and have to be walked too. The only way this over-collects is a
/// config key literally named `properties`, which would be a key char accepts
/// and therefore ought to be exercised anyway.
fn collect_properties(node: &serde_json::Value, out: &mut BTreeSet<String>) {
    match node {
        serde_json::Value::Object(map) => {
            if let Some(serde_json::Value::Object(properties)) = map.get("properties") {
                out.extend(properties.keys().cloned());
            }
            for value in map.values() {
                collect_properties(value, out);
            }
        }
        serde_json::Value::Array(items) => {
            for item in items {
                collect_properties(item, out);
            }
        }
        _ => {}
    }
}

/// Every object key anywhere in a document.
fn collect_keys(node: &serde_json::Value, out: &mut BTreeSet<String>) {
    match node {
        serde_json::Value::Object(map) => {
            for (key, value) in map {
                out.insert(key.clone());
                collect_keys(value, out);
            }
        }
        serde_json::Value::Array(items) => {
            for item in items {
                collect_keys(item, out);
            }
        }
        _ => {}
    }
}

/// The first line that differs, with its neighbours — enough to see what
/// changed without printing two whole documents twice.
fn first_difference(expected: &str, actual: &str) -> String {
    let expected: Vec<&str> = expected.lines().collect();
    let actual: Vec<&str> = actual.lines().collect();
    for (n, (e, a)) in expected.iter().zip(actual.iter()).enumerate() {
        if e != a {
            return format!("    line {}:\n      golden: {e}\n      actual: {a}", n + 1);
        }
    }
    format!(
        "    identical for {} lines, then golden has {} and actual has {}",
        expected.len().min(actual.len()),
        expected.len(),
        actual.len()
    )
}
