//! What the contract makes unrepresentable.
//!
//! The fixtures prove the schema can express six repo shapes. This file proves
//! the other half: that it *cannot* express the things PLAN.md says are
//! bad_config. A schema that accepts everything passes the fixture suite
//! perfectly.
//!
//! Each case names the rule and where it comes from. The first is the one that
//! matters most — `shell: true` beside `${files}` is arbitrary code execution
//! on every machine that runs `char check` on a branch anyone can push
//! (`docs/traps.md`), and PLAN.md §4.1 says it must be unrepresentable rather
//! than warned about.

use charkit_core::config::{parse, resolve, Defaults, SCHEMA};

fn rejected_by_schema(doc: &str) -> bool {
    let mut schemas = boon::Schemas::new();
    let mut compiler = boon::Compiler::new();
    let value: serde_json::Value = serde_json::from_str(SCHEMA).expect("schema is JSON");
    compiler.add_resource("char.schema.json", value).unwrap();
    let index = compiler.compile("char.schema.json", &mut schemas).unwrap();

    let Ok(instance) = serde_yaml_ng::from_str::<serde_json::Value>(doc) else {
        // Not even YAML-into-JSON: rejected earlier than the schema, which is
        // still rejected.
        return true;
    };
    schemas.validate(&instance, index).is_err()
}

fn rejected_by_the_core(doc: &str) -> bool {
    match parse(doc, "char.yml") {
        Err(_) => true,
        Ok(config) => resolve(config, &Defaults::built_in(), "char.yml").is_err(),
    }
}

/// Cases the **schema** must reject. Some are also caught by the structs; the
/// schema is the one that must, because it is the artifact an agent authors
/// against and the one `config verify` runs.
#[test]
fn the_schema_rejects_what_the_contract_forbids() {
    let cases: &[(&str, &str)] = &[
        (
            "shell: true beside ${files} — arbitrary code execution (PLAN.md §4.1)",
            "version: 1\ncomponents:\n  api:\n    checks:\n      lint:\n        cmd: eslint ${files}\n        shell: true\n",
        ),
        (
            "the same, via fix: rather than cmd:",
            "version: 1\ncomponents:\n  api:\n    checks:\n      lint:\n        cmd: eslint .\n        fix: eslint --fix ${files}\n        shell: true\n",
        ),
        (
            "${files} pasted inside another token — it expands to n arguments (PLAN.md §4.1)",
            "version: 1\ncomponents:\n  api:\n    checks:\n      lint:\n        cmd: ruff check --stdin-filename=${files}\n",
        ),
        (
            "scope: component with ${files} — the two say opposite things (PLAN.md §4.1)",
            "version: 1\ncomponents:\n  api:\n    checks:\n      e2e:\n        cmd: pytest ${files}\n        scope: component\n",
        ),
        (
            "a check with in: and a secrets grant — exec puts the value in argv (PLAN.md §4.1)",
            "version: 1\ncomponents:\n  api:\n    checks:\n      test:\n        cmd: pytest\n        in: api\n        secrets: [TOKEN]\n",
        ),
        (
            "a commands: entry shadowing a built-in verb (PLAN.md §4.5)",
            "version: 1\ncommands:\n  check:\n    cmd: ./scripts/check.sh\n",
        ),
        (
            "the same for a verb PLAN.md §4.5's own list omitted",
            "version: 1\ncommands:\n  explain:\n    cmd: ./scripts/explain.sh\n",
        ),
        (
            "${files} in a commands: entry — there is no scope to compute (PLAN.md §4.5)",
            "version: 1\ncommands:\n  fmt:\n    cmd: ./scripts/fmt.sh ${files}\n",
        ),
        (
            "${env.NAME} outside an env: block (PLAN.md §4.4)",
            "version: 1\ncomponents:\n  api:\n    checks:\n      lint:\n        cmd: ruff check --config ${env.RUFF_CONFIG}\n",
        ),
        (
            "a default operator — the stopping point is the error (PLAN.md §4.4)",
            "version: 1\ncommands:\n  build:\n    cmd: ./build.sh\n    env:\n      CI: ${env.CI ?? \"0\"}\n",
        ),
        (
            "${ref} outside a provider cmd (PLAN.md §4.4)",
            "version: 1\ncomponents:\n  api:\n    run:\n      driver: command\n      cmd: serve --token ${ref}\n",
        ),
        (
            "a provider cmd with no ${ref} — it could never resolve anything (PLAN.md §4.4)",
            "version: 1\nsecret_providers:\n  op: { cmd: \"op read\" }\n",
        ),
        (
            "a provider with shell: true — rule 1 exists to keep ${ref} out of a shell (PLAN.md §4.7)",
            "version: 1\nsecret_providers:\n  op: { cmd: \"op read ${ref}\", shell: true }\n",
        ),
        (
            "two ready kinds in one mapping (PLAN.md §6.0)",
            "version: 1\ncomponents:\n  api:\n    run:\n      driver: command\n      cmd: serve\n      ready: { tcp: api, none: true }\n",
        ),
        (
            "a ready mapping with a timeout and no kind (PLAN.md §6.0)",
            "version: 1\ncomponents:\n  api:\n    run:\n      driver: command\n      cmd: serve\n      ready: { timeout: 30 }\n",
        ),
        (
            "ready.tcp as a number — a number is the pre-claim port (PLAN.md §6.0)",
            "version: 1\ncomponents:\n  api:\n    run:\n      driver: command\n      cmd: serve\n      ready: { tcp: 5432 }\n",
        ),
        (
            "driver: compose with a cmd: beside it (PLAN.md §6)",
            "version: 1\ncomponents:\n  api:\n    run:\n      driver: compose\n      file: [docker-compose.yml]\n      cmd: serve\n",
        ),
        (
            "driver: command with a file: beside it (PLAN.md §6)",
            "version: 1\ncomponents:\n  api:\n    run:\n      driver: command\n      cmd: serve\n      file: [docker-compose.yml]\n",
        ),
        (
            "a vendor-named driver (PLAN.md §6)",
            "version: 1\ncomponents:\n  api:\n    run:\n      driver: procfile\n      cmd: serve\n",
        ),
        (
            "a component root escaping the workspace (PLAN.md §5)",
            "version: 1\ncomponents:\n  api:\n    root: ../other-repo/api\n",
        ),
        (
            "an absolute component root (PLAN.md §5)",
            "version: 1\ncomponents:\n  api:\n    root: /srv/api\n",
        ),
        (
            "`.` as a spelling of the workspace root — one idea, one spelling",
            "version: 1\ncomponents:\n  api:\n    root: .\n",
        ),
        (
            "an owns.files path escaping the workspace (PLAN.md §5)",
            "version: 1\ncomponents:\n  api:\n    owns:\n      files: [\"../elsewhere/cache\"]\n",
        ),
        (
            "a component name containing the reserved colon (PLAN.md §4.1)",
            "version: 1\ncomponents:\n  api:lint:\n    checks:\n      lint: { cmd: ruff check }\n",
        ),
        (
            "an uppercase component name — the grammar is lowercase (PLAN.md §4.1)",
            "version: 1\ncomponents:\n  API:\n    checks:\n      lint: { cmd: ruff check }\n",
        ),
        (
            "a provider name that is legal as a component name and illegal as a URI scheme (PLAN.md §4.7)",
            "version: 1\nsecret_providers:\n  aws_sm: { cmd: \"aws get ${ref}\" }\n",
        ),
        (
            "a secret reference with no scheme (PLAN.md §4.7)",
            "version: 1\nsecrets:\n  TOKEN: Engineering/github/token\n",
        ),
        (
            "ports: under a commands: entry — the block is already claimed (PLAN.md §4.5)",
            "version: 1\ncommands:\n  wt:\n    cmd: ./wt.sh\n    owns:\n      ports: [api]\n",
        ),
        (
            "containers: under a component-level owns: — setup has no runtime handles (PLAN.md §6.0)",
            "version: 1\ncomponents:\n  api:\n    owns:\n      containers: \"label=x=1\"\n",
        ),
        (
            "no version at all (PLAN.md §4.1)",
            "components:\n  api:\n    checks:\n      lint: { cmd: ruff check }\n",
        ),
        (
            "a version this char does not understand",
            "version: 2\ncomponents: {}\n",
        ),
        (
            "an unknown top-level key",
            "version: 1\nservices:\n  api: {}\n",
        ),
        (
            "an unknown key inside a check",
            "version: 1\ncomponents:\n  api:\n    checks:\n      lint: { cmd: ruff check, retries: 3 }\n",
        ),
        (
            "an unknown key inside a setup step object",
            "version: 1\ncomponents:\n  api:\n    setup:\n      - { cmd: bundle install, once: true }\n",
        ),
        (
            "a check with no cmd:",
            "version: 1\ncomponents:\n  api:\n    checks:\n      lint: { timeout: 60 }\n",
        ),
        (
            "cost: 0 — a check costs at least one slot",
            "version: 1\ncomponents:\n  api:\n    checks:\n      lint: { cmd: ruff check, cost: 0 }\n",
        ),
        (
            "a port outside the port range",
            "version: 1\ncomponents:\n  api:\n    run:\n      driver: command\n      cmd: serve\n      ports: { api: 70000 }\n",
        ),
        (
            "an env value that is not a string — YAML would make it a number",
            "version: 1\ncommands:\n  build:\n    cmd: ./build.sh\n    env:\n      PORT: 3000\n",
        ),
    ];

    let mut accepted = Vec::new();
    for (why, doc) in cases {
        if !rejected_by_schema(doc) {
            accepted.push(*why);
        }
    }
    assert!(
        accepted.is_empty(),
        "the schema accepted documents it must reject:\n  {}",
        accepted.join("\n  ")
    );
}

/// The subset the **core** must also reject, because a phase-2 verb loads a
/// config through `parse`/`resolve` and no schema runs at that point. Anything
/// here that only the schema caught would be a document char happily acts on.
#[test]
fn the_core_rejects_what_it_cannot_turn_into_a_typed_value() {
    let cases: &[(&str, &str)] = &[
        (
            "no version",
            "components: {}\n",
        ),
        (
            "a version this char does not understand",
            "version: 2\n",
        ),
        (
            "an unknown top-level key",
            "version: 1\nservices: {}\n",
        ),
        (
            "an unknown key inside a check",
            "version: 1\ncomponents:\n  api:\n    checks:\n      lint: { cmd: ruff check, retries: 3 }\n",
        ),
        (
            "a check with no cmd:",
            "version: 1\ncomponents:\n  api:\n    checks:\n      lint: { timeout: 60 }\n",
        ),
        (
            "a vendor-named driver",
            "version: 1\ncomponents:\n  api:\n    run:\n      driver: procfile\n      cmd: serve\n",
        ),
        (
            "driver: compose with no file:",
            "version: 1\ncomponents:\n  api:\n    run:\n      driver: compose\n",
        ),
        (
            "driver: command with no cmd:",
            "version: 1\ncomponents:\n  api:\n    run:\n      driver: command\n",
        ),
        (
            "two ready kinds",
            "version: 1\ncomponents:\n  api:\n    run:\n      driver: command\n      cmd: serve\n      ready: { tcp: api, none: true }\n",
        ),
        (
            "an unknown scope",
            "version: 1\ncomponents:\n  api:\n    checks:\n      lint: { cmd: ruff check, scope: repo }\n",
        ),
        (
            "an unknown stdio",
            "version: 1\ncommands:\n  wt: { cmd: ./wt.sh, stdio: tty }\n",
        ),
        (
            "an env value that is not a string",
            "version: 1\ncommands:\n  build:\n    cmd: ./build.sh\n    env:\n      PORT: 3000\n",
        ),
    ];

    let mut accepted = Vec::new();
    for (why, doc) in cases {
        if !rejected_by_the_core(doc) {
            accepted.push(*why);
        }
    }
    assert!(
        accepted.is_empty(),
        "the core accepted documents it must reject:\n  {}",
        accepted.join("\n  ")
    );
}

/// Every `bad_config` says where, and says what to do about it.
///
/// `next_action` is required for this class and for no other, because it is the
/// one class where char genuinely knows the fix (`ARCHITECTURE.md` §1.7). A
/// class whose remediation field is usually empty teaches agents to ignore it.
#[test]
fn every_config_error_carries_a_where_and_a_next_action() {
    let docs = [
        "version: 2\n",
        "version: 1\ncomponents:\n  api:\n    run:\n      driver: procfile\n      cmd: serve\n",
        "version: 1\ncomponents:\n  api:\n    checks:\n      lint: { cmd: ruff, scope: repo }\n",
        "version: 1\ncomponents: {\n",
    ];
    for doc in docs {
        let err = match parse(doc, "char.yml") {
            Err(e) => e,
            Ok(config) => {
                resolve(config, &Defaults::built_in(), "char.yml").expect_err("should not resolve")
            }
        };
        assert_eq!(err.class, charkit_core::error::ErrClass::BadConfig);
        assert!(
            err.r#where.starts_with("char.yml"),
            "where was {}",
            err.r#where
        );
        assert!(err.next_action.is_some(), "no next_action for {doc:?}");
    }
}
