//! Verify of a workspace's own `armada.yml`, below the repository root: its
//! own setup and Checks, in its own directory, holding the checkout's one
//! Verify slot.

use std::path::Path;
use std::process::Command;
use std::sync::Arc;
use std::time::Duration;

use axum::http::StatusCode;
use testkit::{FakeHarness, FakeVcs, FakeWorkProduct};

use crate::daemon::Fleet;
use crate::gate::CheckBudget;
use crate::tests::daemon::fittings;
use crate::tests::http::call;
use crate::tests::tmp::TempDir;

pub(super) type Fixture = Fleet<FakeHarness, FakeVcs, FakeWorkProduct>;

/// The root's: `slow` outlives the assertions around it.
const ROOT: &str = r#"version: 1
id: 01FIXTUREMANIFEST
checks:
  slow:
    run: "/bin/sh -c 'echo going; sleep 30'"
"#;

/// `apps/web`'s own: setup leaves a file its second Check needs, and its first
/// Check writes down where it ran.
const WEB: &str = r#"version: 1
id: 01WEBMANIFEST
checks:
  where:
    run: "/bin/sh -c 'pwd -P > ran-in.txt'"
  prepared:
    run: "/bin/sh -c 'test -f prepared.txt'"
commands:
  prepare:
    run: "/bin/sh -c 'echo ready > prepared.txt'"
  say:
    run: "/bin/sh -c 'pwd -P; echo said > said.txt'"
setup:
  requires: [prepare]
"#;

/// YAML that Armada refuses: a Check with nothing to run.
const BROKEN: &str = "version: 1\nid: 01BROKENMANIFEST\nchecks:\n  lint:\n    run: \"\"\n";

/// A checkout with a root Manifest, `apps/web`'s, and `apps/broken`'s.
pub(super) fn a_checkout(home: &TempDir) -> Arc<Fixture> {
    let manifest = config::Manifest::parse(Path::new("armada.yml"), ROOT)
        .unwrap_or_else(|why| panic!("the root manifest did not parse: {why}"));
    let mut fittings = fittings(home, FakeWorkProduct::changed(&[]));
    fittings.starting().manifest = manifest;
    fittings.budget = CheckBudget::of(Duration::from_secs(120));
    let at = home.path();
    let git = |args: &[&str]| {
        let run = Command::new("git")
            .arg("-C")
            .arg(at)
            .args([
                "-c",
                "user.name=a person",
                "-c",
                "user.email=a@person.invalid",
            ])
            .args(args)
            .output()
            .expect("git on PATH");
        assert!(run.status.success(), "git {args:?} failed");
    };
    git(&["-c", "init.defaultBranch=main", "init", "--quiet"]);
    std::fs::write(at.join("armada.yml"), ROOT).expect("the root Manifest");
    for (dir, text) in [("apps/web", WEB), ("apps/broken", BROKEN)] {
        std::fs::create_dir_all(at.join(dir)).expect("a workspace");
        std::fs::write(at.join(dir).join("armada.yml"), text).expect("its Manifest");
    }
    git(&["add", "."]);
    git(&["commit", "--quiet", "-m", "the first commit"]);
    Arc::new(Fleet::assembled(fittings))
}

async fn ended(fleet: &Fixture) -> ipc::CheckoutVerify {
    tokio::time::timeout(Duration::from_secs(60), async {
        loop {
            let sheet = fleet
                .checkout_run_sheet(fleet.first())
                .await
                .expect("a sheet");
            if let Some(verify) = sheet.verify.filter(|one| one.ended_at.is_some()) {
                return verify;
            }
            tokio::time::sleep(Duration::from_millis(20)).await;
        }
    })
    .await
    .expect("the Verify ended")
}

fn code(refused: &Result<ipc::CheckoutVerify, api::Refusal>) -> String {
    match refused {
        Err(api::Refusal::Unacceptable(error)) => error.code.clone(),
        other => panic!("not a 422: {other:?}"),
    }
}

/// **A workspace Verify runs that file's setup, then its Checks in written
/// order, once each, in that workspace's directory** — and the Verify names
/// the workspace, on its answer and on the sheet.
#[tokio::test]
async fn a_workspace_verify_runs_its_own_checks_in_its_directory() {
    let home = TempDir::new();
    let fleet = a_checkout(&home);

    let begun = Arc::clone(&fleet)
        .begin_checkout_verify(fleet.first(), Some("./apps/web/"))
        .await
        .expect("underway");
    assert_eq!(begun.workspace.as_deref(), Some("apps/web"));
    let verify = ended(&fleet).await;
    assert_eq!(verify.id, begun.id);
    assert_eq!(
        verify.workspace.as_deref(),
        Some("apps/web"),
        "the sheet says which file ran"
    );

    let ran: Vec<(&str, Option<i32>)> = verify
        .steps
        .iter()
        .map(|step| match &step.state {
            ipc::VerifyStepState::Ran { record } => (step.name.as_str(), record.exit_code),
            other => panic!("`{}` did not run: {other:?}", step.name),
        })
        .collect();
    assert_eq!(
        ran,
        [
            ("prepare", Some(0)),
            ("where", Some(0)),
            ("prepared", Some(0))
        ],
        "its own setup, then its own Checks, and never the root's `slow`"
    );

    let web = home.path().join("apps/web");
    let said = std::fs::read_to_string(web.join("ran-in.txt")).expect("`where` wrote it");
    assert_eq!(
        Path::new(said.trim()),
        web.canonicalize().expect("the workspace")
    );
    assert!(!home.path().join("ran-in.txt").exists());
    assert!(!home.path().join("prepared.txt").exists());
}

/// **One Verify per checkout.** A root Verify underway refuses a workspace
/// Verify in the same tree, and the root's is the one the sheet names.
#[tokio::test]
async fn a_root_verify_underway_refuses_a_workspace_verify() {
    let home = TempDir::new();
    let fleet = a_checkout(&home);

    let root = Arc::clone(&fleet)
        .begin_checkout_verify(fleet.first(), None)
        .await
        .expect("the root's is underway");
    assert_eq!(root.workspace, None, "absent is the root's file");

    let refused = Arc::clone(&fleet)
        .begin_checkout_verify(fleet.first(), Some("apps/web"))
        .await;
    assert!(
        matches!(refused, Err(api::Refusal::IllegalMove(_))),
        "{refused:?}"
    );

    let ipc::VerifyStepState::Running { run_id } = &root.steps[0].state else {
        panic!("the root's first step is out: {:?}", root.steps[0]);
    };
    fleet
        .stop_checkout_rehearsal(run_id.clone(), fleet.first())
        .await
        .expect("the step stopped");
    assert_eq!(ended(&fleet).await.workspace, None);
}

/// **A workspace outside the root is refused, and so is a missing or
/// unloadable file** — before anything runs, and through the route a caller
/// sends it on.
#[tokio::test]
async fn a_workspace_outside_the_root_or_without_a_file_is_refused() {
    let home = TempDir::new();
    let fleet = a_checkout(&home);
    let elsewhere = TempDir::new();
    std::fs::write(elsewhere.path().join("armada.yml"), WEB).expect("a file outside");
    std::os::unix::fs::symlink(elsewhere.path(), home.path().join("apps/away")).expect("a link");

    for outside in ["../elsewhere", "apps/../../elsewhere", "/etc", "apps/away"] {
        let refused = Arc::clone(&fleet)
            .begin_checkout_verify(fleet.first(), Some(outside))
            .await;
        assert_eq!(
            code(&refused),
            "fleet.workspace_outside_repository",
            "{outside}"
        );
    }
    let missing = Arc::clone(&fleet)
        .begin_checkout_verify(fleet.first(), Some("apps/missing"))
        .await;
    assert_eq!(code(&missing), "fleet.workspace_manifest_unreadable");

    let broken = Arc::clone(&fleet)
        .begin_checkout_verify(fleet.first(), Some("apps/broken"))
        .await;
    assert_eq!(code(&broken), "fleet.workspace_manifest_unreadable");
    let Err(api::Refusal::Unacceptable(error)) = &broken else {
        unreachable!("read above");
    };
    let Some(ipc::WireValue::List(faults)) = error.fields.get("faults") else {
        panic!("the refusal carries its faults: {error:?}");
    };
    assert!(!faults.is_empty(), "{error:?}");

    let app = api::router(api::Served::sharing(
        Arc::clone(&fleet),
        ipc::RunId::carried("01RUN"),
        fleet.events(),
    ));
    let (status, body) = call(
        &app,
        "POST",
        "/manifest/start_verify",
        r#"{"workspace":"../elsewhere"}"#,
    )
    .await;
    assert_eq!(status, StatusCode::UNPROCESSABLE_ENTITY);
    assert!(String::from_utf8_lossy(&body).contains("fleet.workspace_outside_repository"));

    let sheet = fleet
        .checkout_run_sheet(fleet.first())
        .await
        .expect("a sheet");
    assert_eq!(sheet.verify, None, "nothing was begun");
}

/// A repository with `apps/web/armada.yml` and nothing at its root, served
/// beside the Fleet's own. Answers its root.
pub(super) fn only_a_workspace(home: &TempDir, fleet: &Fixture) -> String {
    let root = home.path().join("alone");
    std::fs::create_dir_all(root.join("apps/web")).expect("a workspace");
    std::fs::write(root.join("apps/web/armada.yml"), WEB).expect("its Manifest");
    // Scan finds a workspace by its package file; the sheet lists what Scan finds.
    std::fs::write(root.join("apps/web/package.json"), "{}").expect("its package");
    let git = |args: &[&str]| {
        let run = Command::new("git")
            .arg("-C")
            .arg(&root)
            .args([
                "-c",
                "user.name=a person",
                "-c",
                "user.email=a@person.invalid",
            ])
            .args(args)
            .output()
            .expect("git on PATH");
        assert!(run.status.success(), "git {args:?} failed");
    };
    git(&["-c", "init.defaultBranch=main", "init", "--quiet"]);
    git(&["add", "."]);
    git(&["commit", "--quiet", "-m", "the first commit"]);
    let located = crate::repositories::Located {
        root: root.to_string_lossy().to_string(),
        records_root: home
            .path()
            .join("alone-records")
            .to_string_lossy()
            .to_string(),
        set_up: None,
    };
    fleet
        .repositories()
        .add(located)
        .expect("served without a Manifest");
    root.to_string_lossy().to_string()
}

pub(super) fn routed(fleet: &Arc<Fixture>) -> axum::Router {
    api::router(api::Served::sharing(
        Arc::clone(fleet),
        ipc::RunId::carried("01RUN"),
        fleet.events(),
    ))
}

/// **A workspace verifies on its own, named by `?repository=`, before its
/// repository has a root Manifest** — and the run sheet read the same way
/// names the workspace it ran.
#[tokio::test]
async fn a_workspace_verifies_by_repository_where_the_root_has_no_manifest() {
    let home = TempDir::new();
    let fleet = a_checkout(&home);
    let root = only_a_workspace(&home, &fleet);
    let app = routed(&fleet);
    let scope = format!("?repository={root}");

    let (status, body) = call(
        &app,
        "POST",
        &format!("/manifest/start_verify{scope}"),
        r#"{"workspace":"apps/web"}"#,
    )
    .await;
    assert_eq!(
        status,
        StatusCode::ACCEPTED,
        "{}",
        String::from_utf8_lossy(&body)
    );

    let verify = tokio::time::timeout(Duration::from_secs(60), async {
        loop {
            let (status, body) =
                call(&app, "GET", &format!("/manifest/run_sheet{scope}"), "").await;
            assert_eq!(status, StatusCode::OK, "{}", String::from_utf8_lossy(&body));
            let sheet: ipc::CheckoutRunSheet = ipc::decode("a sheet", &body).expect("a sheet");
            assert!(sheet.checks.is_empty(), "no root Manifest lists nothing");
            if let Some(verify) = sheet.verify.filter(|one| one.ended_at.is_some()) {
                return verify;
            }
            tokio::time::sleep(Duration::from_millis(20)).await;
        }
    })
    .await
    .expect("the Verify ended");

    assert_eq!(verify.workspace.as_deref(), Some("apps/web"));
    let codes: Vec<(&str, Option<i32>)> = verify
        .steps
        .iter()
        .map(|step| match &step.state {
            ipc::VerifyStepState::Ran { record } => (step.name.as_str(), record.exit_code),
            other => panic!("`{}` did not run: {other:?}", step.name),
        })
        .collect();
    assert_eq!(
        codes,
        [
            ("prepare", Some(0)),
            ("where", Some(0)),
            ("prepared", Some(0))
        ]
    );
    assert!(Path::new(&root).join("apps/web/ran-in.txt").exists());
    let own = fleet
        .checkout_run_sheet(fleet.first())
        .await
        .expect("the Fleet's own sheet");
    assert_eq!(
        own.verify, None,
        "held by its own root, not the first repository's"
    );
}

/// **With no workspace, a repository with no root Manifest is refused plainly,
/// and so is a route naming both `manifest_id` and `repository`.**
#[tokio::test]
async fn no_workspace_or_both_names_is_refused_where_the_root_has_no_manifest() {
    let home = TempDir::new();
    let fleet = a_checkout(&home);
    let root = only_a_workspace(&home, &fleet);
    let app = routed(&fleet);

    let (status, body) = call(
        &app,
        "POST",
        &format!("/manifest/start_verify?repository={root}"),
        "",
    )
    .await;
    let said = String::from_utf8_lossy(&body);
    assert_eq!(status, StatusCode::UNPROCESSABLE_ENTITY, "{said}");
    assert!(said.contains("no armada.yml at its root"), "{said}");

    let both = format!("?manifest_id=01FIXTUREMANIFEST&repository={root}");
    for (method, path, body) in [
        (
            "POST",
            "/manifest/start_verify",
            r#"{"workspace":"apps/web"}"#,
        ),
        ("GET", "/manifest/run_sheet", ""),
        ("POST", "/manifest/stop_run", r#"{"id":"01RUN"}"#),
    ] {
        let (status, answered) = call(&app, method, &format!("{path}{both}"), body).await;
        assert_eq!(status, StatusCode::UNPROCESSABLE_ENTITY, "{path}");
        assert!(
            String::from_utf8_lossy(&answered).contains("fleet.repository_named_twice"),
            "{path}"
        );
    }
    let sheet = fleet
        .checkout_run_sheet(fleet.first())
        .await
        .expect("a sheet");
    assert_eq!(sheet.verify, None, "nothing was begun");
}
