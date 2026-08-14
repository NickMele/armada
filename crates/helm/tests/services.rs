//! `armada manifest up` and `armada manifest down`, through the real binary.
//!
//! **The claim under test is reclaimability, not liveness.** A service that
//! starts and works is the easy half; the half this project exists for is that
//! *everything `up` starts is recorded before it is confirmed working*, so
//! `clean` can reclaim a container or a process group after the workspace
//! directory is gone. Every assertion below is aimed at that.
//!
//! `command` services are covered here because they need no daemon and are the
//! driver whose ownership record is load-bearing — a pgid carries no label, so
//! the row in `manifest.db` is the only record there will ever be. The compose
//! driver against a real daemon is [`docker_up.rs`](docker_up.rs).

mod support;

use serde_json::Value;
use std::path::Path;
use support::Machine;

/// A repo with two `command` services, the second needing the first.
///
/// `sleep` is the service: it starts, it stays up, and it says nothing — which
/// is exactly the shape whose only handle is its process group.
const TWO_SERVICES: &str = "\
manifest:
  version: 1
  components:
    db:
      run:
        driver: command
        cmd: sleep 300
        ports: { pg: 5432 }
        ready: { none: true }
    api:
      run:
        driver: command
        cmd: sleep 300
        ports: { api: 3000 }
        ready: { none: true }
        needs: [db]
";

fn json(bytes: &[u8]) -> Value {
    serde_json::from_slice(bytes).expect("the envelope parses")
}

fn statuses(envelope: &Value) -> Vec<(String, String)> {
    envelope["data"]["results"]
        .as_array()
        .expect("results[]")
        .iter()
        .map(|row| {
            (
                row["id"].as_str().unwrap().to_string(),
                row["status"].as_str().unwrap().to_string(),
            )
        })
        .collect()
}

/// Every `pgid:` row this workspace holds, as numbers.
fn owned_pgids(machine: &Machine, repo: &Path) -> Vec<i32> {
    let out = machine.run(repo, &["manifest", "status", "--json"]);
    let envelope = json(&out.stdout);
    envelope["data"]["results"]
        .as_array()
        .unwrap()
        .iter()
        .flat_map(|row| row["owns"].as_array().cloned().unwrap_or_default())
        .filter_map(|id| {
            id.as_str()?
                .strip_prefix("pgid:")
                .and_then(|n| n.parse::<i32>().ok())
        })
        .collect()
}

fn alive(pgid: i32) -> bool {
    // A signal-0 probe against the group. `ps` rather than `killpg` because the
    // test is a different process than the one that parented it, so it never
    // sees the zombie ambiguity `docs/traps.md` records.
    std::process::Command::new("ps")
        .args(["-o", "pid=", "-g", &pgid.to_string()])
        .output()
        .map(|out| !String::from_utf8_lossy(&out.stdout).trim().is_empty())
        .unwrap_or(false)
}

/// **The whole point.** A process group Armada started is in `manifest.db`
/// before `up` returns, so `clean` can reclaim it even once the directory is
/// gone — and the row carries the boot id and the start time, without which a
/// recycled pid is indistinguishable from an orphaned service and the row is a
/// permanent phantom (PLAN.md §2.3.1).
#[test]
fn up_records_every_process_group_it_started() {
    let machine = Machine::new();
    let repo = machine.repo("recorded", TWO_SERVICES);
    machine.run(&repo, &["manifest", "init"]);

    let out = machine.run(&repo, &["manifest", "up", "--json"]);
    let envelope = json(&out.stdout);
    assert_eq!(envelope["status"], "UP", "{}", envelope);
    assert_eq!(
        statuses(&envelope),
        vec![
            ("db".to_string(), "UP".to_string()),
            ("api".to_string(), "UP".to_string())
        ],
        "dependency order, and both up"
    );

    let groups = owned_pgids(&machine, &repo);
    assert_eq!(groups.len(), 2, "two services, two recorded groups");
    for pgid in &groups {
        assert!(alive(*pgid), "group {pgid} was recorded and is not running");
    }

    // Reclaimable is the claim, so it is exercised rather than asserted about.
    machine.run(&repo, &["manifest", "clean", "--force"]);
    for pgid in &groups {
        assert!(!alive(*pgid), "group {pgid} outlived `clean`");
    }
}

/// **`down` keeps the block; `clean` releases it.** That distinction is the
/// entire reason both verbs exist, and it is what makes the next `up` give back
/// the same ports — so URLs, bookmarks and `.env` files stay valid.
#[test]
fn down_stops_the_services_and_keeps_the_port_block() {
    let machine = Machine::new();
    let repo = machine.repo("kept", TWO_SERVICES);
    machine.run(&repo, &["manifest", "init"]);
    machine.run(&repo, &["manifest", "up"]);

    let groups = owned_pgids(&machine, &repo);
    assert_eq!(groups.len(), 2);

    let out = machine.run(&repo, &["manifest", "down", "--json"]);
    let envelope = json(&out.stdout);
    assert_eq!(envelope["status"], "DOWN", "{}", envelope);
    assert_eq!(out.status.code(), Some(0));

    // **Dependents before dependencies**, so nothing is torn out from under a
    // live consumer.
    assert_eq!(
        statuses(&envelope)
            .into_iter()
            .map(|(id, _)| id)
            .collect::<Vec<_>>(),
        vec!["api", "db"]
    );

    for pgid in &groups {
        assert!(!alive(*pgid), "group {pgid} survived `down`");
    }
    assert!(
        owned_pgids(&machine, &repo).is_empty(),
        "a stopped group kept its row"
    );

    // The block is still this workspace's, and the same one.
    let block = &envelope["data"]["port_block"];
    let after = json(&machine.run(&repo, &["manifest", "status", "--json"]).stdout);
    assert_eq!(
        &after["data"]["results"][0]["port_block"], block,
        "`down` released the block: that is `clean`'s job"
    );
}

/// A ready-check that will never pass reports `TIMEOUT`, not `FAILED`. A gate
/// reading 1 goes looking for a broken service; reading 4 it raises a deadline.
#[test]
fn a_ready_check_that_never_passes_times_out_and_exits_four() {
    let machine = Machine::new();
    let repo = machine.repo(
        "slow",
        "\
manifest:
  version: 1
  components:
    web:
      run:
        driver: command
        cmd: sleep 300
        ports: { web: 3000 }
        ready: { exec: \"false\", timeout: 1 }
",
    );
    machine.run(&repo, &["manifest", "init"]);

    let out = machine.run(&repo, &["manifest", "up", "--json"]);
    let envelope = json(&out.stdout);
    assert_eq!(envelope["status"], "TIMEOUT", "{}", envelope);
    assert_eq!(envelope["error"]["class"], "timeout");
    assert_eq!(out.status.code(), Some(4), "not 1");

    // **Started and not ready is still owned.** The container or group most
    // likely to be broken is the one it is least acceptable to lose.
    let groups = owned_pgids(&machine, &repo);
    assert_eq!(
        groups.len(),
        1,
        "a service that never became ready lost its row"
    );
    machine.run(&repo, &["manifest", "clean", "--force"]);
    assert!(!alive(groups[0]));
}

/// A service whose dependency did not come up is `SKIPPED` naming the one that
/// stopped it — not started into a failure two levels from its own logs.
#[test]
fn a_service_whose_dependency_failed_is_skipped_and_never_started() {
    let machine = Machine::new();
    let repo = machine.repo(
        "cascade",
        "\
manifest:
  version: 1
  components:
    db:
      run:
        driver: command
        cmd: definitely-not-on-path
        ports: { pg: 5432 }
        ready: { none: true }
    api:
      run:
        driver: command
        cmd: sleep 300
        ports: { api: 3000 }
        ready: { none: true }
        needs: [db]
",
    );
    machine.run(&repo, &["manifest", "init"]);

    let out = machine.run(&repo, &["manifest", "up", "--json"]);
    let envelope = json(&out.stdout);
    assert_eq!(envelope["status"], "FAILED", "{}", envelope);

    let rows = statuses(&envelope);
    assert_eq!(
        rows,
        vec![
            ("db".to_string(), "FAILED".to_string()),
            ("api".to_string(), "SKIPPED".to_string())
        ]
    );
    let api = &envelope["data"]["results"][1];
    assert_eq!(api["reason"], "`db` did not start");

    // Nothing was started, so nothing is owned — and no intent row was left
    // behind for a fork that never happened.
    assert!(
        owned_pgids(&machine, &repo).is_empty(),
        "a service that never started left a group row"
    );
}

/// A `log:` ready-check reads the file the service is writing, which is also
/// the file a human reads. Both halves matter: `.armada/logs/<component>.log`
/// is where PLAN.md §4.2 puts it, and a pipe would have died with `up`.
#[test]
fn a_log_ready_check_matches_against_the_file_the_service_writes() {
    let machine = Machine::new();
    let repo = machine.repo(
        "logs",
        "\
manifest:
  version: 1
  components:
    web:
      run:
        driver: command
        cmd: ./say.sh
        ports: { web: 3000 }
        ready: { log: \"listening on\", timeout: 30 }
",
    );
    std::fs::write(
        repo.join("say.sh"),
        "#!/bin/sh\nsleep 0.3\necho 'listening on 3000'\nsleep 300\n",
    )
    .unwrap();
    #[cfg(unix)]
    {
        use std::os::unix::fs::PermissionsExt;
        std::fs::set_permissions(repo.join("say.sh"), std::fs::Permissions::from_mode(0o755))
            .unwrap();
    }
    machine.run(&repo, &["manifest", "init"]);

    let out = machine.run(&repo, &["manifest", "up", "--json"]);
    let envelope = json(&out.stdout);
    assert_eq!(envelope["status"], "UP", "{}", envelope);
    assert_eq!(
        envelope["data"]["results"][0]["log"],
        ".armada/logs/web.log"
    );

    let log = std::fs::read_to_string(repo.join(".armada/logs/web.log")).unwrap();
    assert!(log.contains("listening on 3000"), "{log:?}");

    // The service is still running after `up` returned, which is what a real
    // file descriptor buys over a pipe Armada was holding.
    let groups = owned_pgids(&machine, &repo);
    assert!(alive(groups[0]), "the service died when `up` exited");
    machine.run(&repo, &["manifest", "clean", "--force"]);
}

/// `up` is not `init`, and says so rather than claiming a block itself.
#[test]
fn up_before_init_refuses_and_names_the_verb_that_claims() {
    let machine = Machine::new();
    let repo = machine.repo("uninitialised", TWO_SERVICES);

    let out = machine.run(&repo, &["manifest", "up", "--json"]);
    let envelope = json(&out.stdout);
    assert_eq!(envelope["error"]["class"], "bad_invocation");
    assert_eq!(out.status.code(), Some(2));
    assert!(
        envelope["error"]["next_action"]
            .as_str()
            .unwrap()
            .contains("init"),
        "{envelope}"
    );
}

/// A selector that is not a service is a refusal that says which mistake it
/// was, and lists the names that would have worked.
#[test]
fn a_selector_that_names_no_service_is_refused_with_the_list() {
    let machine = Machine::new();
    let repo = machine.repo("selector", TWO_SERVICES);
    machine.run(&repo, &["manifest", "init"]);

    let out = machine.run(&repo, &["manifest", "up", "dbb", "--json"]);
    let envelope = json(&out.stdout);
    assert_eq!(envelope["error"]["class"], "bad_invocation");
    assert!(
        envelope["error"]["next_action"]
            .as_str()
            .unwrap()
            .contains("api, db"),
        "{envelope}"
    );
}

/// A selected service pulls its dependencies in, and `down` on one does not
/// take the rest with it.
#[test]
fn selecting_one_service_starts_its_dependencies_and_stops_only_itself() {
    let machine = Machine::new();
    let repo = machine.repo("selected", TWO_SERVICES);
    machine.run(&repo, &["manifest", "init"]);

    let envelope = json(
        &machine
            .run(&repo, &["manifest", "up", "api", "--json"])
            .stdout,
    );
    assert_eq!(
        statuses(&envelope),
        vec![
            ("db".to_string(), "UP".to_string()),
            ("api".to_string(), "UP".to_string())
        ],
        "`api` needs `db`, so starting one starts both"
    );

    let before = owned_pgids(&machine, &repo);
    assert_eq!(before.len(), 2);

    let envelope = json(
        &machine
            .run(&repo, &["manifest", "down", "api", "--json"])
            .stdout,
    );
    assert_eq!(
        statuses(&envelope),
        vec![("api".to_string(), "DOWN".to_string())],
        "`down api` must not stop the db something else may be using"
    );
    assert_eq!(
        owned_pgids(&machine, &repo).len(),
        1,
        "`down api` stopped more than it was asked to"
    );

    machine.run(&repo, &["manifest", "clean", "--force"]);
}

/// `--dry-run` reports the argv and the ready-check and starts nothing.
#[test]
fn a_dry_run_reports_the_argv_and_the_wait_and_starts_nothing() {
    let machine = Machine::new();
    let repo = machine.repo("preview", TWO_SERVICES);
    machine.run(&repo, &["manifest", "init"]);

    let out = machine.run(&repo, &["manifest", "up", "--dry-run", "--json"]);
    let envelope = json(&out.stdout);
    let would_run = envelope["data"]["would_run"].as_array().unwrap();
    assert_eq!(would_run.len(), 2, "{envelope}");
    assert!(would_run[0].as_str().unwrap().starts_with("db: sleep 300"));
    assert_eq!(
        envelope["data"]["would_wait"][0],
        "db: ready on spawn (60s)"
    );

    assert!(
        owned_pgids(&machine, &repo).is_empty(),
        "a dry run started something"
    );
    assert_eq!(out.status.code(), Some(0));
}

/// **A repo that declares no services is `SKIPPED`, not `PARTIAL`** — nothing
/// to do is exit 0 (PLAN.md §3).
#[test]
fn a_workspace_with_no_services_is_skipped() {
    let machine = Machine::new();
    let repo = machine.repo(
        "quiet",
        "manifest:\n  version: 1\n  components:\n    docs:\n      checks:\n        lint:\n          cmd: true\n",
    );
    machine.run(&repo, &["manifest", "init"]);

    let out = machine.run(&repo, &["manifest", "up", "--json"]);
    assert_eq!(json(&out.stdout)["status"], "SKIPPED");
    assert_eq!(out.status.code(), Some(0));

    let out = machine.run(&repo, &["manifest", "down", "--json"]);
    assert_eq!(json(&out.stdout)["status"], "DOWN");
    assert_eq!(out.status.code(), Some(0));
}

/// **A group that ignores SIGTERM still dies**, because the escalation is
/// unconditional rather than a retry: children inherit an *ignored* disposition
/// across `fork` and `exec`, so one uncooperative leader immunises its whole
/// group and a second SIGTERM achieves what the first did (`docs/traps.md`).
///
/// A cooperative `sleep` passes this while proving nothing, which is why the
/// service traps the signal.
#[test]
fn down_ends_a_service_that_ignores_sigterm() {
    let machine = Machine::new();
    let repo = machine.repo(
        "stubborn",
        "\
manifest:
  version: 1
  components:
    web:
      run:
        driver: command
        cmd: ./stubborn.sh
        ports: { web: 3000 }
        ready: { none: true }
",
    );
    std::fs::write(
        repo.join("stubborn.sh"),
        "#!/bin/sh\ntrap '' TERM\nsleep 300 &\nsleep 300\n",
    )
    .unwrap();
    #[cfg(unix)]
    {
        use std::os::unix::fs::PermissionsExt;
        std::fs::set_permissions(
            repo.join("stubborn.sh"),
            std::fs::Permissions::from_mode(0o755),
        )
        .unwrap();
    }
    machine.run(&repo, &["manifest", "init"]);
    machine.run(&repo, &["manifest", "up"]);

    let groups = owned_pgids(&machine, &repo);
    assert_eq!(groups.len(), 1);
    // Let the trap be installed before signalling it.
    std::thread::sleep(std::time::Duration::from_millis(400));

    let out = machine.run(&repo, &["manifest", "down", "--json"]);
    assert_eq!(json(&out.stdout)["status"], "DOWN", "{}", json(&out.stdout));
    assert!(
        !alive(groups[0]),
        "a group that ignores SIGTERM outlived `down`, so the escalation is a retry"
    );
}
