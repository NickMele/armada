//! A workspace's own Command in a repository with no root `armada.yml`: run,
//! listed, read and undone, every route naming the repository by its root.

use std::path::Path;
use std::time::Duration;

use axum::http::StatusCode;

use crate::tests::http::call;
use crate::tests::tmp::TempDir;
use crate::tests::workspace_verify::{a_checkout, only_a_workspace, routed};

/// **A workspace's Command runs in its directory by `?repository=`**, and its
/// history, output, diff and Undo are read the same way — none of it in the
/// first repository's.
#[tokio::test]
async fn a_workspace_command_runs_and_undoes_by_repository_where_the_root_has_no_manifest() {
    let home = TempDir::new();
    let fleet = a_checkout(&home);
    let root = only_a_workspace(&home, &fleet);
    let app = routed(&fleet);
    let scope = format!("?repository={root}");

    let (status, body) = call(
        &app,
        "POST",
        &format!("/manifest/start_run{scope}"),
        r#"{"name":"say","workspace":"apps/web"}"#,
    )
    .await;
    let said = String::from_utf8_lossy(&body);
    assert_eq!(status, StatusCode::ACCEPTED, "{said}");
    let underway: ipc::CheckoutRunUnderway = ipc::decode("underway", &body).expect("underway");

    let record = tokio::time::timeout(Duration::from_secs(30), async {
        loop {
            let (status, body) = call(&app, "GET", &format!("/manifest/runs{scope}"), "").await;
            assert_eq!(status, StatusCode::OK, "{}", String::from_utf8_lossy(&body));
            let list: ipc::CheckoutRunList = ipc::decode("a history", &body).expect("a history");
            if let Some(record) = list.runs.into_iter().find(|run| run.id == underway.id) {
                return record;
            }
            tokio::time::sleep(Duration::from_millis(20)).await;
        }
    })
    .await
    .expect("the run wrote its record");
    assert_eq!(record.exit_code, Some(0), "{}", record.ended);
    let web = Path::new(&root).join("apps/web");
    assert!(web.join("said.txt").is_file(), "it ran in the workspace");
    assert!(record.undoable);

    let uri = format!("/manifest/runs/{}/output{scope}", underway.id);
    let (status, body) = call(&app, "GET", &uri, "").await;
    assert_eq!(status, StatusCode::OK, "{}", String::from_utf8_lossy(&body));
    let output: ipc::RunOutput = ipc::decode("its output", &body).expect("its output");
    let web_real = web.canonicalize().expect("the workspace");
    assert!(
        output
            .lines
            .iter()
            .any(|line| Path::new(line.trim()) == web_real),
        "{:?}",
        output.lines
    );

    let uri = format!("/manifest/runs/{}/diff{scope}", underway.id);
    let (status, body) = call(&app, "GET", &uri, "").await;
    assert_eq!(status, StatusCode::OK, "{}", String::from_utf8_lossy(&body));
    let diff: ipc::CheckoutRunDiff = ipc::decode("its diff", &body).expect("its diff");
    let ipc::RunDiffReading::Read { files, .. } = diff.reading else {
        panic!("the snapshot is kept: {:?}", diff.reading);
    };
    let paths: Vec<_> = files.iter().map(|file| file.path.as_str()).collect();
    assert_eq!(paths, vec!["apps/web/said.txt"]);

    let named = format!(r#"{{"id":"{}"}}"#, underway.id);
    let (status, body) = call(&app, "POST", &format!("/manifest/undo_run{scope}"), &named).await;
    assert_eq!(status, StatusCode::OK, "{}", String::from_utf8_lossy(&body));
    assert!(
        !web.join("said.txt").exists(),
        "Undo put the workspace back"
    );

    let own = fleet
        .checkout_rehearsal_history(fleet.first())
        .await
        .expect("the first repository's history");
    assert!(own.runs.iter().all(|run| run.id != underway.id));
}

/// **With no workspace named, the root's missing file is refused plainly.**
#[tokio::test]
async fn a_run_with_no_workspace_is_refused_where_the_root_has_no_manifest() {
    let home = TempDir::new();
    let fleet = a_checkout(&home);
    let root = only_a_workspace(&home, &fleet);
    let app = routed(&fleet);

    let (status, body) = call(
        &app,
        "POST",
        &format!("/manifest/start_run?repository={root}"),
        r#"{"name":"say"}"#,
    )
    .await;
    let said = String::from_utf8_lossy(&body);
    assert_eq!(status, StatusCode::UNPROCESSABLE_ENTITY, "{said}");
    assert!(said.contains("no armada.yml at its root"), "{said}");
}

/// **Each run route refuses a request naming both `manifest_id` and
/// `repository`**, rather than quietly choosing one.
#[tokio::test]
async fn every_run_route_refuses_both_names() {
    let home = TempDir::new();
    let fleet = a_checkout(&home);
    let root = only_a_workspace(&home, &fleet);
    let app = routed(&fleet);

    let both = format!("?manifest_id=01FIXTUREMANIFEST&repository={root}");
    for (method, path, body) in [
        (
            "POST",
            "/manifest/start_run",
            r#"{"name":"say","workspace":"apps/web"}"#,
        ),
        ("POST", "/manifest/undo_run", r#"{"id":"01RUN"}"#),
        ("GET", "/manifest/runs", ""),
        ("GET", "/manifest/runs/01RUN/output", ""),
        ("GET", "/manifest/runs/01RUN/diff", ""),
    ] {
        let (status, answered) = call(&app, method, &format!("{path}{both}"), body).await;
        let said = String::from_utf8_lossy(&answered);
        assert_eq!(status, StatusCode::UNPROCESSABLE_ENTITY, "{path}: {said}");
        assert!(
            said.contains("fleet.repository_named_twice"),
            "{path}: {said}"
        );
    }
    assert!(!Path::new(&root).join("apps/web/said.txt").exists());
}
