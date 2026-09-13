//! Scan against a real directory. Whether a file is there is the subject, so
//! the tree is on disk — `tests::drifting`'s reason.

use std::collections::BTreeMap;
use std::path::Path;

use ipc::{EvidenceStrength, RepositoryScan, ScannedWorkspace};

use super::{compose, scan, Checkout};
use crate::tests::tmp::TempDir;

/// A checkout holding these files, parents made.
fn checkout(files: &[(&str, &str)]) -> TempDir {
    let dir = TempDir::new();
    for (path, text) in files {
        let at = dir.path().join(path);
        std::fs::create_dir_all(at.parent().expect("a parent")).expect("a directory");
        std::fs::write(at, text).expect("a file");
    }
    dir
}

fn scanned(dir: &TempDir) -> RepositoryScan {
    scan("/repo", &Checkout::at(dir.path()))
}

fn workspace<'a>(scan: &'a RepositoryScan, dir: &str) -> &'a ScannedWorkspace {
    scan.workspaces
        .iter()
        .find(|one| one.dir == dir)
        .unwrap_or_else(|| panic!("{dir} is a workspace: {scan:#?}"))
}

fn dirs(scan: &RepositoryScan) -> Vec<&str> {
    scan.workspaces.iter().map(|one| one.dir.as_str()).collect()
}

/// Every file under `root` with its bytes, to compare before and after.
fn contents(root: &Path) -> BTreeMap<String, Vec<u8>> {
    let mut found = BTreeMap::new();
    let mut pending = vec![root.to_path_buf()];
    while let Some(dir) = pending.pop() {
        for entry in std::fs::read_dir(dir).expect("lists").flatten() {
            let path = entry.path();
            match path.is_dir() {
                true => pending.push(path),
                false => {
                    let key = path
                        .strip_prefix(root)
                        .expect("under")
                        .display()
                        .to_string();
                    found.insert(key, std::fs::read(&path).expect("reads"));
                }
            }
        }
    }
    found
}

const MONOREPO: &[(&str, &str)] = &[
    ("package.json", r#"{"scripts":{"lint":"pnpm -r lint"}}"#),
    ("pnpm-lock.yaml", ""),
    (
        "pnpm-workspace.yaml",
        "packages:\n  - 'apps/*'\n  - \"libs/**\"\n",
    ),
    (
        "apps/web/package.json",
        r#"{"scripts":{"test":"vitest run","lint":"eslint .","dev":"next dev --port 3000"}}"#,
    ),
    (
        "apps/admin/package.json",
        r#"{"scripts":{"test":"vitest run","lint":"eslint ."}}"#,
    ),
    ("apps/api/package.json", r#"{"scripts":{"test":"vitest"}}"#),
    ("apps/empty/package.json", r#"{"name":"empty"}"#),
];

#[test]
fn every_workspace_is_found_in_one_pass_with_the_file_each_finding_came_from() {
    let dir = checkout(MONOREPO);
    let scan = scanned(&dir);
    assert_eq!(
        dirs(&scan),
        [".", "apps/admin", "apps/api", "apps/empty", "apps/web"]
    );

    let web = workspace(&scan, "apps/web");
    assert_eq!(web.declared_by[0].file, "pnpm-workspace.yaml");
    assert_eq!(web.declared_by[0].entry, "apps/*");
    assert_eq!(web.evidence, EvidenceStrength::Strong);
    let test = web
        .runnables
        .iter()
        .find(|r| r.name == "test")
        .expect("test");
    assert_eq!(
        (test.file.as_str(), test.key.as_str(), test.run.as_str()),
        ("apps/web/package.json", "scripts.test", "vitest run")
    );
    assert_eq!(web.ports.len(), 1, "the one port a file declares");
    assert_eq!(
        (web.ports[0].key.as_str(), web.ports[0].container),
        ("scripts.dev", 3000)
    );

    let root = workspace(&scan, ".");
    assert_eq!(
        root.lockfiles[0].file, "pnpm-lock.yaml",
        "cited where it sits"
    );
    assert!(web.lockfiles.is_empty(), "and not copied into a member");
    assert!(
        scan.not_read
            .iter()
            .any(|one| one.file == "pnpm-workspace.yaml: libs/**"),
        "a pattern it does not expand is said: {:?}",
        scan.not_read
    );
}

#[test]
fn strength_is_what_the_files_name_and_nothing_else() {
    let dir = checkout(&[
        ("apps/empty/package.json", "{}"),
        ("apps/go/go.mod", "module x"),
        ("apps/web/package.json", r#"{"scripts":{"test":"vitest"}}"#),
    ]);
    let scan = scanned(&dir);
    assert_eq!(
        workspace(&scan, "apps/web").evidence,
        EvidenceStrength::Strong
    );
    assert_eq!(
        workspace(&scan, "apps/empty").evidence,
        EvidenceStrength::Thin
    );
    let go = workspace(&scan, "apps/go");
    assert_eq!(go.evidence, EvidenceStrength::NotFollowed, "never clean");
    assert_eq!(go.not_read[0].file, "apps/go/go.mod");
    assert_eq!(
        workspace(&scan, ".").evidence,
        EvidenceStrength::NotFollowed
    );
}

#[test]
fn a_name_every_sibling_declares_is_marked_where_it_is_missing_and_only_there() {
    let dir = checkout(MONOREPO);
    let scan = scanned(&dir);
    let api = workspace(&scan, "apps/api");
    assert_eq!(api.missing.len(), 1, "{:?}", api.missing);
    assert_eq!(api.missing[0].name, "lint");
    assert_eq!(api.missing[0].declared_in, ["apps/admin", "apps/web"]);
    assert!(
        workspace(&scan, "apps/web").missing.is_empty(),
        "`dev` is web's own"
    );
    assert!(
        workspace(&scan, ".").missing.is_empty(),
        "the root is not a sibling"
    );
    assert!(
        workspace(&scan, "apps/empty").missing.is_empty(),
        "a thin workspace is outside the batch, neither marked nor erasing a mark"
    );
}

#[test]
fn a_workspace_whose_scripts_would_not_read_is_neither_marked_nor_a_sibling() {
    let dir = checkout(&[
        (
            "apps/a/package.json",
            r#"{"scripts":{"test":"x","lint":"y"}}"#,
        ),
        ("apps/b/package.json", r#"{"scripts":{"test":"x"}}"#),
        ("apps/c/package.json", r#"{"scripts":{"test": 1}}"#),
    ]);
    let scan = scanned(&dir);
    let unread = workspace(&scan, "apps/c");
    assert!(unread.missing.is_empty());
    assert_eq!(unread.not_read[0].file, "apps/c/package.json");
    assert_eq!(workspace(&scan, "apps/b").missing[0].name, "lint");
}

#[test]
fn scan_writes_nothing() {
    let dir = checkout(MONOREPO);
    let before = contents(dir.path());
    scanned(&dir);
    assert_eq!(contents(dir.path()), before);
}

#[test]
fn a_checkout_that_will_not_list_says_why_rather_than_looking_empty() {
    let scan = scan("/nowhere", &Checkout::at("/nowhere/at/all"));
    assert_eq!(dirs(&scan), ["."]);
    assert_eq!(scan.not_read[0].file, ".");
    assert!(scan.not_read[0].why.contains("would not list"));
}

#[test]
fn yaml_under_a_hidden_directory_is_reported_not_read_and_not_skipped() {
    let dir = checkout(&[(".ci/pipeline.yml", "jobs: {}"), (".ci/x/deep.yaml", "")]);
    let scan = scanned(&dir);
    let files: Vec<&str> = scan.not_read.iter().map(|one| one.file.as_str()).collect();
    assert_eq!(files, [".ci/pipeline.yml", ".ci/x/deep.yaml"]);
}

#[test]
fn cargo_members_aliases_and_pyproject_tools_are_read_through_their_files() {
    let dir = checkout(&[
        ("Cargo.toml", "[workspace]\nmembers = [\"crates/*\"]\n"),
        (
            ".cargo/config.toml",
            "[alias]\nxtask = \"run -p xtask --\"\n",
        ),
        ("crates/core/Cargo.toml", "[package]\nname = \"core\"\n"),
        (
            "tools/py/pyproject.toml",
            "[tool.ruff]\n[tool.pytest.ini_options]\n[tool.ruff.lint]\n",
        ),
    ]);
    let scan = scanned(&dir);
    let root = workspace(&scan, ".");
    assert_eq!(root.runnables[0].key, "alias.xtask");
    assert_eq!(root.runnables[0].file, ".cargo/config.toml");
    let core = workspace(&scan, "crates/core");
    assert_eq!(core.declared_by[0].file, "Cargo.toml");
    assert_eq!(
        core.evidence,
        EvidenceStrength::Thin,
        "no alias, no runnable"
    );
    let py = workspace(&scan, "tools/py");
    let tools: Vec<&str> = py.tools.iter().map(|t| t.tool.as_str()).collect();
    assert_eq!(tools, ["ruff", "pytest"], "each section once");
}

#[test]
fn a_port_is_evidence_only_where_a_file_declares_one() {
    let dir = checkout(&[
        ("package.json", r#"{"scripts":{"dev":"vite"}}"#),
        (
            "compose.yaml",
            "services:\n  web:\n    ports:\n      - \"8080:3000\"\n  db:\n    image: pg\n",
        ),
        (
            ".env.example",
            "PORT=4000\nDATABASE_URL=x\nDB_PORT=\"5432\"\n",
        ),
    ]);
    let scan = scanned(&dir);
    let root = workspace(&scan, ".");
    let ports: Vec<(&str, &str, u16)> = root
        .ports
        .iter()
        .map(|p| (p.file.as_str(), p.key.as_str(), p.container))
        .collect();
    assert_eq!(
        ports,
        [
            (".env.example", "PORT", 4000),
            (".env.example", "DB_PORT", 5432),
            ("compose.yaml", "services.web.ports[0]", 3000)
        ]
    );
    let services: Vec<&str> = root.services.iter().map(|s| s.name.as_str()).collect();
    assert_eq!(services, ["web", "db"]);

    let bare = checkout(&[("package.json", r#"{"scripts":{"dev":"vite"}}"#)]);
    assert!(
        workspace(&scanned(&bare), ".").ports.is_empty(),
        "none is common"
    );
}

#[test]
fn a_compose_file_reads_every_port_form_or_says_which_it_could_not() {
    let text = "services:\n  web: &web\n    ports: [\"3000\", \"127.0.0.1:80:8080/tcp\"]\n  \
                api:\n    build: .\n    ports:\n      - target: 9000\n        published: 1\n      \
                - 5000-5010:5000-5010\n      - \"${API_PORT}\"\n";
    let services = compose::services(text).expect("a shape it reads");
    let ports: Vec<Result<u16, bool>> = services
        .iter()
        .flat_map(|s| {
            s.ports
                .iter()
                .map(|p| p.container.clone().map_err(|_| false))
        })
        .collect();
    assert_eq!(
        ports,
        [Ok(3000), Ok(8080), Ok(9000), Err(false), Err(false)]
    );
    assert_eq!(services[1].ports[1].key, "services.api.ports[1]");

    assert!(compose::services("services: {}\n")
        .expect("empty")
        .is_empty());
    assert!(compose::services("volumes:\n  data:\n")
        .expect("none")
        .is_empty());
    assert!(
        compose::services("services:\n  web: nginx\n").is_none(),
        "not a shape it reads"
    );
}
