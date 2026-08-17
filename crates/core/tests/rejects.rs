//! What the contract makes unrepresentable.
//!
//! The fixtures prove the schema can express six repo shapes. This file proves
//! the other half: that it *cannot* express the things PLAN.md says are
//! bad_config. A schema that accepts everything passes the fixture suite
//! perfectly.
//!
//! Each case names the rule and where it comes from. The first is the one that
//! matters most — `shell: true` beside `${files}` is arbitrary code execution
//! on every machine that runs `armada manifest check` on a branch anyone can push
//! (`docs/traps.md`), and PLAN.md §4.1 says it must be unrepresentable rather
//! than warned about.

use armada_core::config::{parse, resolve, Defaults, SCHEMA};

fn rejected_by_schema(doc: &str) -> bool {
    let mut schemas = boon::Schemas::new();
    let mut compiler = boon::Compiler::new();
    let value: serde_json::Value = serde_json::from_str(SCHEMA).expect("schema is JSON");
    compiler.add_resource("armada.schema.json", value).unwrap();
    let index = compiler
        .compile("armada.schema.json", &mut schemas)
        .unwrap();

    let Ok(instance) = serde_yaml_ng::from_str::<serde_json::Value>(doc) else {
        // Not even YAML-into-JSON: rejected earlier than the schema, which is
        // still rejected.
        return true;
    };
    schemas.validate(&instance, index).is_err()
}

fn rejected_by_the_core(doc: &str) -> bool {
    match parse(doc, "armada.yml") {
        Err(_) => true,
        Ok(config) => resolve(config, &Defaults::built_in(), "armada.yml").is_err(),
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
            "manifest:\n  version: 1\n  components:\n    api:\n      checks:\n        lint:\n          cmd: eslint ${files}\n          shell: true\n",
        ),
        (
            "the same, via fix: rather than cmd:",
            "manifest:\n  version: 1\n  components:\n    api:\n      checks:\n        lint:\n          cmd: eslint .\n          fix: eslint --fix ${files}\n          shell: true\n",
        ),
        (
            "${files} pasted inside another token — it expands to n arguments (PLAN.md §4.1)",
            "manifest:\n  version: 1\n  components:\n    api:\n      checks:\n        lint:\n          cmd: ruff check --stdin-filename=${files}\n",
        ),
        (
            "scope: component with ${files} — the two say opposite things (PLAN.md §4.1)",
            "manifest:\n  version: 1\n  components:\n    api:\n      checks:\n        e2e:\n          cmd: pytest ${files}\n          scope: component\n",
        ),
        (
            "a check with in: and a secrets grant — exec puts the value in argv (PLAN.md §4.1)",
            "manifest:\n  version: 1\n  components:\n    api:\n      checks:\n        test:\n          cmd: pytest\n          in: api\n          secrets: [TOKEN]\n",
        ),
        (
            "a commands: entry shadowing a built-in verb (PLAN.md §4.5)",
            "manifest:\n  version: 1\n  commands:\n    check:\n      cmd: ./scripts/check.sh\n",
        ),
        (
            "the same for a verb PLAN.md §4.5's own list omitted",
            "manifest:\n  version: 1\n  commands:\n    explain:\n      cmd: ./scripts/explain.sh\n",
        ),
        // **Two names the schema was silently letting through.** They are
        // built-in verbs and were absent from the forbidden list, so a repo
        // could declare them and have Armada's verb shadow theirs — the exact
        // failure §4.5 exists to prevent, happening with no error at all.
        (
            "a commands: entry named `skills`, which the schema used to allow",
            "manifest:\n  version: 1\n  commands:\n    skills:\n      cmd: ./scripts/skills.sh\n",
        ),
        (
            "a commands: entry named `render`, the same omission",
            "manifest:\n  version: 1\n  commands:\n    render:\n      cmd: ./scripts/render.sh\n",
        ),
        // And the name this milestone took. Promoting a verb into Armada's
        // namespace takes it from every repository, which is the trade §4.5
        // states — so the schema has to close the moment the verb ships.
        (
            "a commands: entry named `components`, now that the verb exists",
            "manifest:\n  version: 1\n  commands:\n    components:\n      cmd: ./scripts/c.sh\n",
        ),
        (
            "a skill named `components`, which carries the same rule (§4.8)",
            "manifest:\n  version: 1\n  skills:\n    components:\n      summary: list them\n      doc: docs/c.md\n",
        ),
        // `commands` is the name `armada manifest commands` took, and it is the
        // most pointed case of the trade: the verb that lists a repository's
        // `commands:` block is the one name that block may no longer contain.
        (
            "a commands: entry named `commands`, now that the listing verb exists",
            "manifest:\n  version: 1\n  commands:\n    commands:\n      cmd: ./scripts/list.sh\n",
        ),
        (
            "a skill named `commands`, which carries the same rule (§4.8)",
            "manifest:\n  version: 1\n  skills:\n    commands:\n      summary: list them\n      doc: docs/c.md\n",
        ),
        (
            "${files} in a commands: entry — there is no scope to compute (PLAN.md §4.5)",
            "manifest:\n  version: 1\n  commands:\n    fmt:\n      cmd: ./scripts/fmt.sh ${files}\n",
        ),
        (
            "${env.NAME} outside an env: block (PLAN.md §4.4)",
            "manifest:\n  version: 1\n  components:\n    api:\n      checks:\n        lint:\n          cmd: ruff check --config ${env.RUFF_CONFIG}\n",
        ),
        (
            "a default operator — the stopping point is the error (PLAN.md §4.4)",
            "manifest:\n  version: 1\n  commands:\n    build:\n      cmd: ./build.sh\n      env:\n        CI: ${env.CI ?? \"0\"}\n",
        ),
        (
            "${ref} outside a provider cmd (PLAN.md §4.4)",
            "manifest:\n  version: 1\n  components:\n    api:\n      run:\n        driver: command\n        cmd: serve --token ${ref}\n",
        ),
        (
            "a provider cmd with no ${ref} — it could never resolve anything (PLAN.md §4.4)",
            "manifest:\n  version: 1\n  secret_providers:\n    op: { cmd: \"op read\" }\n",
        ),
        (
            "${env.NAME} in a provider cmd — argv-split, so nothing would expand it (PLAN.md §4.4)",
            "manifest:\n  version: 1\n  secret_providers:\n    op: { cmd: \"op read ${env.VAULT}/${ref}\" }\n",
        ),
        (
            "a provider with shell: true — rule 1 exists to keep ${ref} out of a shell (PLAN.md §4.7)",
            "manifest:\n  version: 1\n  secret_providers:\n    op: { cmd: \"op read ${ref}\", shell: true }\n",
        ),
        (
            "two ready kinds in one mapping (PLAN.md §6.0)",
            "manifest:\n  version: 1\n  components:\n    api:\n      run:\n        driver: command\n        cmd: serve\n        ready: { tcp: api, none: true }\n",
        ),
        (
            "a ready mapping with a timeout and no kind (PLAN.md §6.0)",
            "manifest:\n  version: 1\n  components:\n    api:\n      run:\n        driver: command\n        cmd: serve\n        ready: { timeout: 30 }\n",
        ),
        (
            "ready.tcp as a number — a number is the pre-claim port (PLAN.md §6.0)",
            "manifest:\n  version: 1\n  components:\n    api:\n      run:\n        driver: command\n        cmd: serve\n        ready: { tcp: 5432 }\n",
        ),
        (
            "driver: compose with a cmd: beside it (PLAN.md §6)",
            "manifest:\n  version: 1\n  components:\n    api:\n      run:\n        driver: compose\n        file: [docker-compose.yml]\n        cmd: serve\n",
        ),
        (
            "driver: command with a file: beside it (PLAN.md §6)",
            "manifest:\n  version: 1\n  components:\n    api:\n      run:\n        driver: command\n        cmd: serve\n        file: [docker-compose.yml]\n",
        ),
        (
            "a vendor-named driver (PLAN.md §6)",
            "manifest:\n  version: 1\n  components:\n    api:\n      run:\n        driver: procfile\n        cmd: serve\n",
        ),
        (
            "a component root escaping the workspace (PLAN.md §5)",
            "manifest:\n  version: 1\n  components:\n    api:\n      root: ../other-repo/api\n",
        ),
        (
            "an absolute component root (PLAN.md §5)",
            "manifest:\n  version: 1\n  components:\n    api:\n      root: /srv/api\n",
        ),
        (
            "`.` as a spelling of the workspace root — one idea, one spelling",
            "manifest:\n  version: 1\n  components:\n    api:\n      root: .\n",
        ),
        (
            "an owns.files path escaping the workspace (PLAN.md §5)",
            "manifest:\n  version: 1\n  components:\n    api:\n      owns:\n        files: [\"../elsewhere/cache\"]\n",
        ),
        (
            "a component name containing the reserved colon (PLAN.md §4.1)",
            "manifest:\n  version: 1\n  components:\n    api:lint:\n      checks:\n        lint: { cmd: ruff check }\n",
        ),
        (
            "an uppercase component name — the grammar is lowercase (PLAN.md §4.1)",
            "manifest:\n  version: 1\n  components:\n    API:\n      checks:\n        lint: { cmd: ruff check }\n",
        ),
        (
            "a provider name that is legal as a component name and illegal as a URI scheme (PLAN.md §4.7)",
            "manifest:\n  version: 1\n  secret_providers:\n    aws_sm: { cmd: \"aws get ${ref}\" }\n",
        ),
        (
            "a secret reference with no scheme (PLAN.md §4.7)",
            "manifest:\n  version: 1\n  secrets:\n    TOKEN: Engineering/github/token\n",
        ),
        (
            "ports: under a commands: entry — the block is already claimed (PLAN.md §4.5)",
            "manifest:\n  version: 1\n  commands:\n    wt:\n      cmd: ./wt.sh\n      owns:\n        ports: [api]\n",
        ),
        (
            "containers: under a component-level owns: — setup has no runtime handles (PLAN.md §6.0)",
            "manifest:\n  version: 1\n  components:\n    api:\n      owns:\n        containers: \"label=x=1\"\n",
        ),
        (
            "no manifest: section at all (PLAN.md §4.1)",
            "version: 1\ncomponents: {}\n",
        ),
        (
            "no version at all (PLAN.md §4.1)",
            "manifest:\n  components:\n    api:\n      checks:\n        lint: { cmd: ruff check }\n",
        ),
        (
            "a version this Armada does not understand",
            "manifest:\n  version: 2\n  components: {}\n",
        ),
        (
            "an unknown top-level key",
            "manifest:\n  version: 1\n  services:\n    api: {}\n",
        ),
        (
            "an unknown key inside a check",
            "manifest:\n  version: 1\n  components:\n    api:\n      checks:\n        lint: { cmd: ruff check, retries: 3 }\n",
        ),
        (
            "an unknown key inside a setup step object",
            "manifest:\n  version: 1\n  components:\n    api:\n      setup:\n        - { cmd: bundle install, once: true }\n",
        ),
        (
            "a check with no cmd:",
            "manifest:\n  version: 1\n  components:\n    api:\n      checks:\n        lint: { timeout: 60 }\n",
        ),
        (
            "cost: 0 — a check costs at least one slot",
            "manifest:\n  version: 1\n  components:\n    api:\n      checks:\n        lint: { cmd: ruff check, cost: 0 }\n",
        ),
        (
            "a port outside the port range",
            "manifest:\n  version: 1\n  components:\n    api:\n      run:\n        driver: command\n        cmd: serve\n        ports: { api: 70000 }\n",
        ),
        (
            "an env value that is not a string — YAML would make it a number",
            "manifest:\n  version: 1\n  commands:\n    build:\n      cmd: ./build.sh\n      env:\n        PORT: 3000\n",
        ),
        (
            "an unknown key under fleet: (034 §6.4) — additionalProperties: false must extend to the new section",
            "manifest:\n  version: 1\nfleet:\n  rebase: always\n",
        ),
        (
            "an unknown key under fleet.land: (034 §6.4)",
            "manifest:\n  version: 1\nfleet:\n  land:\n    strategy: squash\n",
        ),
        (
            "fleet.land.merge outside auto | never (034 §6.4)",
            "manifest:\n  version: 1\nfleet:\n  land:\n    merge: sometimes\n",
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
/// here that only the schema caught would be a document Armada happily acts on.
#[test]
fn the_core_rejects_what_it_cannot_turn_into_a_typed_value() {
    let cases: &[(&str, &str)] = &[
        (
            "no manifest: section",
            "version: 1\n",
        ),
        (
            "no version",
            "manifest:\n  components: {}\n",
        ),
        (
            "a version this Armada does not understand",
            "manifest:\n  version: 2\n",
        ),
        (
            "an unknown top-level key",
            "manifest:\n  version: 1\n  services: {}\n",
        ),
        (
            "an unknown key inside a check",
            "manifest:\n  version: 1\n  components:\n    api:\n      checks:\n        lint: { cmd: ruff check, retries: 3 }\n",
        ),
        (
            "a check with no cmd:",
            "manifest:\n  version: 1\n  components:\n    api:\n      checks:\n        lint: { timeout: 60 }\n",
        ),
        (
            "a vendor-named driver",
            "manifest:\n  version: 1\n  components:\n    api:\n      run:\n        driver: procfile\n        cmd: serve\n",
        ),
        (
            "driver: compose with no file:",
            "manifest:\n  version: 1\n  components:\n    api:\n      run:\n        driver: compose\n",
        ),
        (
            "driver: command with no cmd:",
            "manifest:\n  version: 1\n  components:\n    api:\n      run:\n        driver: command\n",
        ),
        (
            "two ready kinds",
            "manifest:\n  version: 1\n  components:\n    api:\n      run:\n        driver: command\n        cmd: serve\n        ready: { tcp: api, none: true }\n",
        ),
        (
            "an unknown scope",
            "manifest:\n  version: 1\n  components:\n    api:\n      checks:\n        lint: { cmd: ruff check, scope: repo }\n",
        ),
        (
            "an unknown stdio",
            "manifest:\n  version: 1\n  commands:\n    wt: { cmd: ./wt.sh, stdio: tty }\n",
        ),
        (
            "an env value that is not a string",
            "manifest:\n  version: 1\n  commands:\n    build:\n      cmd: ./build.sh\n      env:\n        PORT: 3000\n",
        ),
        (
            "an unknown key under fleet: (034 §6.4) — deny_unknown_fields must extend to the new section",
            "manifest:\n  version: 1\nfleet:\n  rebase: always\n",
        ),
        (
            "an unknown key under fleet.land: (034 §6.4)",
            "manifest:\n  version: 1\nfleet:\n  land:\n    strategy: squash\n",
        ),
        (
            "fleet.land.merge outside auto | never (034 §6.4)",
            "manifest:\n  version: 1\nfleet:\n  land:\n    merge: sometimes\n",
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
/// one class where Armada genuinely knows the fix (`ARCHITECTURE.md` §1.7). A
/// class whose remediation field is usually empty teaches agents to ignore it.
#[test]
fn every_config_error_carries_a_where_and_a_next_action() {
    let docs = [
        "version: 2\n",
        "manifest:\n  version: 1\n  components:\n    api:\n      run:\n        driver: procfile\n        cmd: serve\n",
        "manifest:\n  version: 1\n  components:\n    api:\n      checks:\n        lint: { cmd: ruff, scope: repo }\n",
        "manifest:\n  version: 1\n  components: {\n",
    ];
    for doc in docs {
        let err = match parse(doc, "armada.yml") {
            Err(e) => e,
            Ok(config) => resolve(config, &Defaults::built_in(), "armada.yml")
                .expect_err("should not resolve"),
        };
        assert_eq!(err.class, armada_core::error::ErrClass::BadConfig);
        assert!(
            err.r#where.starts_with("armada.yml"),
            "where was {}",
            err.r#where
        );
        assert!(err.next_action.is_some(), "no next_action for {doc:?}");
    }
}
