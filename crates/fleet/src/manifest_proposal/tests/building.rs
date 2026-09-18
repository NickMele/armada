//! What a first proposal holds, by the convention each line applied.

use ipc::{ProposedPort, Provenance};

use super::{answered, checkout, drafts};

fn convention(file: &str, key: Option<&str>) -> Provenance {
    Provenance::Convention {
        file: file.to_string(),
        key: key.map(str::to_string),
    }
}

/// `cargo test` is convention citing `Cargo.toml`; an alias is a Command citing its config.
#[test]
fn a_cargo_package_is_tested_by_convention_and_its_aliases_are_commands() {
    let dir = checkout(&[
        (
            "Cargo.toml",
            "[package]\nname = \"tool\"\nversion = \"0.1.0\"\n",
        ),
        ("Cargo.lock", "version = 3\n"),
        (
            ".cargo/config.toml",
            "[alias]\nxtask = \"run -p xtask --\"\n",
        ),
    ]);
    let root = answered(&dir, ".");
    assert_eq!(root.checks.len(), 1);
    assert_eq!(root.checks[0].run, "cargo test");
    assert_eq!(root.checks[0].provenance, convention("Cargo.toml", None));
    assert_eq!(root.commands.len(), 1);
    assert_eq!(root.commands[0].run, "cargo xtask");
    assert_eq!(
        root.commands[0].provenance,
        convention(".cargo/config.toml", Some("alias.xtask"))
    );
    assert!(root.setup.is_none());
}

/// A tool section proposes its Check through the lockfile's runner, and the lockfile setup.
#[test]
fn a_python_project_runs_its_tools_through_its_lockfile() {
    let dir = checkout(&[
        (
            "pyproject.toml",
            "[project]\nname = \"svc\"\n[tool.pytest.ini_options]\n[tool.ruff]\n",
        ),
        ("uv.lock", "version = 1\n"),
    ]);
    let root = answered(&dir, ".");
    let checks: Vec<(&str, &str)> = root
        .checks
        .iter()
        .map(|one| (one.name.as_str(), one.run.as_str()))
        .collect();
    assert_eq!(
        checks,
        [("test", "uv run pytest"), ("lint", "uv run ruff check .")]
    );
    assert_eq!(
        root.checks[0].provenance,
        convention("pyproject.toml", Some("tool.pytest"))
    );
    assert_eq!(root.commands[0].run, "uv sync --frozen");
    assert_eq!(root.commands[0].provenance, convention("uv.lock", None));
    assert_eq!(root.setup.as_ref().expect("setup").requires, ["install"]);
}

/// A port is `read`: a variable keeps its name, compose needs none, a flag gets one derived.
#[test]
fn ports_keep_a_files_variable_and_a_compose_service_needs_none() {
    let dir = checkout(&[
        ("package.json", r#"{"scripts":{"dev":"vite --port 5173"}}"#),
        (".env.example", "WEB_PORT=8080\n"),
        (
            "compose.yaml",
            "services:\n  db:\n    image: postgres\n    ports:\n      - \"5432:5432\"\n",
        ),
    ]);
    let root = answered(&dir, ".");
    let mut ports = root.ports.clone();
    ports.sort_by(|a, b| a.name.cmp(&b.name));
    let read = |file: &str, key: &str| Provenance::Read {
        file: file.to_string(),
        key: key.to_string(),
    };
    let port = |name: &str, container, env: Option<&str>, provenance| ProposedPort {
        name: name.to_string(),
        container: Some(container),
        env: env.map(str::to_string),
        provenance,
    };
    assert_eq!(
        ports,
        [
            port(
                "db",
                5432,
                None,
                read("compose.yaml", "services.db.ports[0]")
            ),
            port(
                "dev",
                5173,
                Some("DEV_PORT"),
                read("package.json", "scripts.dev")
            ),
            port(
                "web",
                8080,
                Some("WEB_PORT"),
                read(".env.example", "WEB_PORT")
            ),
        ]
    );
}

/// Only a member the pattern names shares the root's lockfile, so a Rust service gets no `pnpm`.
#[test]
fn only_a_member_the_pattern_names_is_installed_by_the_roots_lockfile() {
    let dir = checkout(&[
        ("package.json", r#"{"name":"mono","private":true}"#),
        ("pnpm-lock.yaml", "lockfileVersion: '9.0'\n"),
        ("pnpm-workspace.yaml", "packages:\n  - 'apps/*'\n"),
        (
            "apps/web/package.json",
            r#"{"scripts":{"test":"vitest run","build":"vite build"}}"#,
        ),
        (
            "services/api/Cargo.toml",
            "[package]\nname = \"api\"\nversion = \"0.1.0\"\n",
        ),
    ]);
    let web = answered(&dir, "apps/web");
    assert_eq!(web.checks[0].run, "pnpm run test");
    assert_eq!(
        web.commands
            .iter()
            .map(|one| one.name.as_str())
            .collect::<Vec<_>>(),
        ["build", "install"]
    );
    assert_eq!(
        web.commands[1].provenance,
        convention("pnpm-lock.yaml", None)
    );

    let api = answered(&dir, "services/api");
    assert!(api.commands.is_empty(), "{:?}", api.commands);
    assert!(api.setup.is_none());
}

/// Two workspaces with one directory name would key two Manifests to one id.
#[test]
fn two_workspaces_sharing_a_name_are_told_apart_by_path() {
    let dir = checkout(&[
        (
            "pnpm-workspace.yaml",
            "packages:\n  - 'apps/*'\n  - 'packages/*'\n",
        ),
        ("apps/ui/package.json", r#"{"scripts":{"test":"vitest"}}"#),
        (
            "packages/ui/package.json",
            r#"{"scripts":{"test":"vitest"}}"#,
        ),
        ("apps/shop/package.json", r#"{"scripts":{"test":"vitest"}}"#),
    ]);
    let ids: Vec<(String, String)> = drafts(&dir)
        .iter()
        .map(|one| (one.dir().to_string(), one.answer().id.value))
        .filter(|(dir, _)| dir != ".")
        .collect();
    let spelled: Vec<(&str, &str)> = ids
        .iter()
        .map(|(dir, id)| (dir.as_str(), id.as_str()))
        .collect();
    assert_eq!(
        spelled,
        [
            ("apps/shop", "shop"),
            ("apps/ui", "apps-ui"),
            ("packages/ui", "packages-ui")
        ]
    );
}

/// **Detected from what the script runs, not from what is installed.** A
/// `package.json` naming `vitest` in its `test` script gives the runner away,
/// and that is the one signal a Scan already carries — it records no
/// dependency list, and a package holding vitest while running something else
/// would make one say the opposite. #1456.
#[test]
fn a_test_script_naming_a_known_runner_proposes_it() {
    let dir = checkout(&[
        (
            "package.json",
            "{\"name\":\"screens\",\"scripts\":{\"test\":\"vitest run\"}}",
        ),
        ("pnpm-lock.yaml", "lockfileVersion: '9.0'\n"),
    ]);
    let root = answered(&dir, ".");
    let test = root
        .checks
        .iter()
        .find(|check| check.name == "test")
        .expect("the test script is a check");
    let runner = test.runner.as_ref().expect("vitest is detected");
    assert_eq!(runner.name, "vitest");
    assert_eq!(
        runner.dir, None,
        "the root workspace is the repository, and there is no package below it to name"
    );
}

/// A script naming nothing Armada ships a description of proposes no runner,
/// and the Check runs whole — which is what every Check did before any of this.
#[test]
fn a_test_script_naming_no_known_runner_proposes_none() {
    let dir = checkout(&[
        (
            "package.json",
            "{\"name\":\"thing\",\"scripts\":{\"test\":\"node --test\"}}",
        ),
        ("pnpm-lock.yaml", "lockfileVersion: '9.0'\n"),
    ]);
    let root = answered(&dir, ".");
    let test = root
        .checks
        .iter()
        .find(|check| check.name == "test")
        .expect("the test script is a check");
    assert_eq!(test.runner, None);
}
