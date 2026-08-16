//! Labelled resources, reaped by stamp — against a real Docker daemon.
//!
//! **This is the founding bug of the project**: 29 leftover per-worktree Docker
//! networks exhausted the default bridge address pool and broke Postgres
//! startup for every subsequently allocated worktree, *"accumulated exactly
//! because nothing ever called this."*
//!
//! Networks and volumes are covered unconditionally, because
//! `docker network create` and `docker volume create` need **no image** — so
//! that half depends on a daemon and not on a registry. They are also the
//! harder case: measured, compose does not propagate a service's labels to
//! either, so a `clean` that finds resources by label finds the containers and
//! leaves the network and the volumes behind, with no verb that can ever locate
//! them again.
//!
//! A **container** created with `docker run --label` — the phase's own wording —
//! is covered too, gated on being able to obtain an image. The code path is
//! shared and parameterised by kind, so the container adds little on that
//! argument alone; it is here because "the code path is shared" *is* an
//! argument, and this corpus is explicit that arguments lose to measurements.
//!
//! Skipped, loudly, when no daemon is reachable. A test that silently passes
//! without a daemon is worse than one that is absent.

mod support;

use serde_json::Value;
use std::process::Command;
use support::Machine;

fn daemon_is_up() -> bool {
    Command::new("docker")
        .args(["version", "--format", "{{.Server.Version}}"])
        .output()
        .map(|out| out.status.success())
        .unwrap_or(false)
}

fn docker(args: &[&str]) -> String {
    let output = Command::new("docker")
        .args(args)
        .output()
        .expect("docker runs");
    assert!(
        output.status.success(),
        "docker {args:?} failed: {}",
        String::from_utf8_lossy(&output.stderr)
    );
    String::from_utf8_lossy(&output.stdout).trim().to_string()
}

fn exists(kind: &str, name: &str) -> bool {
    let listed = docker(&[kind, "ls", "-q", "--filter", &format!("name=^{name}$")]);
    !listed.is_empty()
}

/// `docker ps -a`, because a stopped container still holds its name and its
/// volumes and a `clean` that leaves it behind leaves the leak.
fn container_exists(name: &str) -> bool {
    let listed = docker(&["ps", "-a", "-q", "--filter", &format!("name=^{name}$")]);
    !listed.is_empty()
}

/// `docker`, without asserting: for the calls whose failure is an answer rather
/// than a broken test.
fn docker_succeeds(args: &[&str]) -> bool {
    Command::new("docker")
        .args(args)
        .output()
        .map(|out| out.status.success())
        .unwrap_or(false)
}

fn local_images() -> Vec<String> {
    let Ok(listed) = Command::new("docker")
        .args(["image", "ls", "--format", "{{.Repository}}:{{.Tag}}"])
        .output()
    else {
        return Vec::new();
    };
    String::from_utf8_lossy(&listed.stdout)
        .lines()
        .map(str::trim)
        .filter(|line| !line.is_empty() && !line.contains("<none>"))
        .map(str::to_string)
        .collect()
}

/// Images to try running a throwaway container from, best first: one already on
/// this machine, then a small one pulled if the network allows.
///
/// **Local before the registry, because the module's claim is that this suite
/// depends on a daemon and not on a registry** — a pull on every run with a
/// daemon would make that false. The list is a list rather than one image
/// because the fallbacks are whatever happens to be on the machine, and a
/// distroless or scratch-based image cannot run the command below; the caller
/// tries them in order and skips loudly when none can.
fn images_for_testing() -> Vec<String> {
    let local = local_images();
    let has_busybox = local.iter().any(|image| image == "busybox:latest");

    let mut candidates: Vec<String> = Vec::new();
    if has_busybox {
        candidates.push("busybox:latest".to_string());
    }
    candidates.extend(
        local
            .into_iter()
            .filter(|image| image != "busybox:latest")
            .take(3),
    );
    if !has_busybox && docker_succeeds(&["pull", "-q", "busybox:latest"]) {
        candidates.push("busybox:latest".to_string());
    }
    candidates
}

/// A labelled container, from the first image that can actually run the
/// command. `None` means the container half of the test cannot run here, and it
/// is skipped loudly rather than silently.
///
/// A `docker run -d` that fails still leaves the named container behind, so
/// each unsuccessful attempt is removed before the next one is tried.
fn labelled_container(name: &str, labels: &[String]) -> Option<String> {
    for image in images_for_testing() {
        let mut args: Vec<&str> = vec!["run", "-d", "--name", name];
        for label in labels {
            args.push("--label");
            args.push(label);
        }
        args.push(&image);
        args.extend(["sleep", "300"]);
        if docker_succeeds(&args) {
            return Some(name.to_string());
        }
        let _ = Command::new("docker").args(["rm", "-f", name]).output();
    }
    None
}

fn namespace_of(machine: &Machine) -> String {
    let db = machine.home.path().join(".armada/manifest.db");
    let conn = rusqlite::Connection::open(db).unwrap();
    conn.query_row(
        "SELECT value FROM meta WHERE key = 'namespace'",
        [],
        |row| row.get(0),
    )
    .unwrap()
}

/// The whole reap contract in one pass: an orphan goes, a live workspace's
/// resource stays, and a foreign namespace's resource stays even though its
/// path is missing here.
#[test]
fn init_reaps_an_orphans_labelled_resources_and_leaves_everyone_elses_alone() {
    if !daemon_is_up() {
        eprintln!("skipping: no Docker daemon reachable");
        return;
    }

    let machine = Machine::new();
    let main = machine.repo("main", CONFIG);
    let doomed = machine.worktree(&main, "doomed");
    machine.run(&main, &["manifest", "init"]);

    let live: Value =
        serde_json::from_slice(&machine.run(&main, &["manifest", "status", "--json"]).stdout)
            .unwrap();
    let live_id = live["data"]["results"][0]["id"]
        .as_str()
        .unwrap()
        .to_string();
    let doomed_json = machine.run(&doomed, &["manifest", "init", "--json"]);
    let doomed_payload: Value = serde_json::from_slice(&doomed_json.stdout).unwrap();
    let doomed_id = doomed_payload["workspace"].as_str().unwrap().to_string();
    let namespace = namespace_of(&machine);

    let suffix = std::process::id();
    let orphan_net = format!("armada-test-orphan-{suffix}");
    let orphan_vol = format!("armada-test-orphanvol-{suffix}");
    let live_net = format!("armada-test-live-{suffix}");
    let foreign_net = format!("armada-test-foreign-{suffix}");

    // The orphan's, stamped exactly as Armada stamps its own.
    docker(&[
        "network",
        "create",
        "--label",
        &format!("armada.workspace={doomed_id}"),
        "--label",
        &format!("armada.workspace_path={}", doomed.display()),
        "--label",
        &format!("armada.namespace={namespace}"),
        &orphan_net,
    ]);
    // **Stamped in the pre-M1 namespace on purpose.** `clean` and `init`'s reap
    // pass must recognise `char.*` for one release, or every resource created
    // before the rename is orphaned on the machine it is sitting on
    // (PHASES.md §8.3). Covered here rather than only in the docker adapter's
    // unit tests because the filter is built for a real daemon and the failure
    // — a `--filter` that ands rather than ors — is invisible to a fake.
    docker(&[
        "volume",
        "create",
        "--label",
        &format!("char.workspace={doomed_id}"),
        "--label",
        &format!("char.workspace_path={}", doomed.display()),
        "--label",
        &format!("char.namespace={namespace}"),
        &orphan_vol,
    ]);
    // A live workspace's, which must survive: `clean` must never cascade into a
    // workspace another agent is using.
    docker(&[
        "network",
        "create",
        "--label",
        &format!("armada.workspace={live_id}"),
        "--label",
        &format!("armada.workspace_path={}", main.display()),
        "--label",
        &format!("armada.namespace={namespace}"),
        &live_net,
    ]);
    // A different installation sharing this daemon — the ordinary devcontainer
    // setup. Its path is `ENOENT` from here, which under the pre-namespace
    // design is exactly how a host-side `armada manifest init` reaped a live workspace's
    // containers.
    docker(&[
        "network",
        "create",
        "--label",
        "armada.workspace=deadbeef",
        "--label",
        "armada.workspace_path=/workspaces/repo",
        "--label",
        "armada.namespace=some-other-installation",
        &foreign_net,
    ]);

    // A container, created the way the phase words it. Skipped rather than
    // failed when no image can be obtained, so the suite depends on a daemon
    // and not on a registry.
    let orphan_container = labelled_container(
        &format!("armada-test-orphan-c-{suffix}"),
        &[
            format!("armada.workspace={doomed_id}"),
            format!("armada.workspace_path={}", doomed.display()),
            format!("armada.namespace={namespace}"),
        ],
    );
    if orphan_container.is_none() {
        eprintln!("note: no image could run a container, so the container case was skipped");
    }

    std::fs::remove_dir_all(&doomed).unwrap();

    let third = machine.worktree(&main, "third");
    let output = machine.run(&third, &["manifest", "init", "--json"]);
    let payload: Value = serde_json::from_slice(&output.stdout).unwrap();

    let removed: Vec<String> = payload["data"]["reaped"]["resources"]
        .as_array()
        .cloned()
        .unwrap_or_default()
        .iter()
        .map(|target| target["reference"].as_str().unwrap_or_default().to_string())
        .collect();

    // Reported, never silent.
    assert!(
        !removed.is_empty(),
        "the reap reported nothing: {}",
        payload["data"]["reaped"]
    );

    assert!(
        !exists("network", &orphan_net),
        "the orphan network survived"
    );
    assert!(!exists("volume", &orphan_vol), "the orphan volume survived");
    if let Some(container) = &orphan_container {
        assert!(
            !container_exists(container),
            "the orphan container survived — this is the case the phase words as \
             `docker run --label`, and it is here because \"the code path is shared\" \
             is an argument, and arguments lose to measurements"
        );
    }
    assert!(
        exists("network", &live_net),
        "a live workspace's network was destroyed"
    );
    assert!(
        exists("network", &foreign_net),
        "another installation's network was destroyed — the namespace label did not hold"
    );

    let reported = payload["data"]["reaped"]["reported"].to_string();
    assert!(
        reported.contains("foreign_namespace"),
        "the foreign resource must be reported rather than merely ignored: {reported}"
    );

    // Clean up whatever this test still owns.
    for name in [&live_net, &foreign_net] {
        let _ = Command::new("docker")
            .args(["network", "rm", name])
            .output();
    }
}

/// **A declared `owns:` selector is reclaimed by `armada manifest clean` after the command
/// has already exited.**
///
/// This is a distinct code path from reading the `owned` table, and that is the
/// point: a `commands:` entry runs ad hoc, so there is no "while it was up"
/// window to record against. Armada stores the *declaration* and evaluates it
/// against docker at `clean` time — which works because every selector is
/// stamped with `${workspace.id}`.
#[test]
fn a_commands_owns_selector_is_reclaimed_after_the_command_has_exited() {
    if !daemon_is_up() {
        eprintln!("skipping: no Docker daemon reachable");
        return;
    }

    let machine = Machine::new();
    let repo = machine.repo("main", OWNS_CONFIG);
    let payload: Value =
        serde_json::from_slice(&machine.run(&repo, &["manifest", "init", "--json"]).stdout)
            .unwrap();
    let id = payload["workspace"].as_str().unwrap().to_string();

    let network = format!("armada-test-owns-{}", std::process::id());
    // The repo's own script, creating a resource Armada never sees created and
    // knows about only through the declaration.
    std::fs::write(
        repo.join("make-net.sh"),
        format!(
            "#!/bin/sh\ndocker network create --label com.example.wt=$ARMADA_WORKSPACE {network} >/dev/null\n"
        ),
    )
    .unwrap();
    #[cfg(unix)]
    {
        use std::os::unix::fs::PermissionsExt;
        std::fs::set_permissions(
            repo.join("make-net.sh"),
            std::fs::Permissions::from_mode(0o755),
        )
        .unwrap();
    }

    let output = machine.run(&repo, &["manifest", "worktrees"]);
    assert!(
        output.status.success(),
        "{}",
        String::from_utf8_lossy(&output.stderr)
    );
    assert!(exists("network", &network), "the command did not create it");

    // The command has exited. Nothing recorded it; only the declaration exists.
    let cleaned = machine.run(&repo, &["manifest", "clean", "--json"]);
    assert!(
        cleaned.status.success(),
        "{}",
        String::from_utf8_lossy(&cleaned.stderr)
    );
    assert!(
        !exists("network", &network),
        "the declared selector was not evaluated at clean time"
    );
    assert!(
        id.len() == 8,
        "the selector is stamped with the workspace id, which is what makes it safe"
    );

    let _ = Command::new("docker")
        .args(["network", "rm", &network])
        .output();
}

/// The other half of the preview contract, and the half only a daemon can
/// answer: `clean --dry-run --all --orphaned --force-rebuild` names the
/// resource it would remove **and leaves it there**. The rebuild path is the
/// one place Armada removes across namespaces, so a preview that removed anyway
/// would be the most expensive way to learn what a flag does.
#[test]
fn a_force_rebuild_dry_run_names_the_orphan_and_removes_nothing() {
    if !daemon_is_up() {
        eprintln!("skipping: no Docker daemon reachable");
        return;
    }

    let machine = Machine::new();
    let repo = machine.repo("main", CONFIG);
    machine.run(&repo, &["manifest", "init"]);
    let namespace = namespace_of(&machine);

    let orphan_net = format!("armada-test-dry-{}", std::process::id());
    docker(&[
        "network",
        "create",
        "--label",
        "armada.workspace=deadbeef",
        "--label",
        &format!("armada.workspace_path={}", repo.join("gone").display()),
        "--label",
        &format!("armada.namespace={namespace}"),
        &orphan_net,
    ]);
    // Armada names a network by its id, so that is what the preview will print.
    let orphan_id = docker(&[
        "network",
        "ls",
        "-q",
        "--filter",
        &format!("name=^{orphan_net}$"),
    ]);

    let db = machine.home.path().join(".armada/manifest.db");
    std::fs::write(&db, b"this is not a database").unwrap();

    let previewed = machine.run(
        &repo,
        &[
            "manifest",
            "clean",
            "--dry-run",
            "--all",
            "--orphaned",
            "--force-rebuild",
            "--json",
        ],
    );
    assert!(
        previewed.status.success(),
        "{}",
        String::from_utf8_lossy(&previewed.stderr)
    );
    let payload: Value = serde_json::from_slice(&previewed.stdout).unwrap();
    assert!(
        !orphan_id.is_empty()
            && payload["data"]["would_remove"]
                .to_string()
                .contains(&orphan_id),
        "the preview did not name the orphan it would remove: {payload}"
    );
    assert!(
        exists("network", &orphan_net),
        "the preview removed the orphan network"
    );

    let _ = Command::new("docker")
        .args(["network", "rm", &orphan_net])
        .output();
}

const OWNS_CONFIG: &str = "\
manifest:
  version: 1
  commands:
    worktrees:
      cmd: ./make-net.sh
      owns:
        networks: label=com.example.wt=${workspace.id}
";

const CONFIG: &str = "\
manifest:
  version: 1
  components:
    app:
      run:
        driver: command
        cmd: ./serve
        ports: { web: 3000 }
";

/// **A live workspace's own named volume is released by `armada manifest
/// clean`** — which is the exact call a Job makes when it finishes.
///
/// # Why this is not covered by the orphan test above
///
/// That test proves the *reap* pass: a workspace whose directory is gone, found
/// by label and removed on `ENOENT`. This proves the other half, and it is the
/// half that leaks. A workspace that is alive and well, whose Job has just
/// reached its last step, is `PathStat::Present` — so the reaper correctly
/// leaves its resources alone, and the only thing that removes them is
/// `clean_one`'s per-workspace pass. Nothing exercised that pass against a real
/// volume, so nothing would have noticed if it had only ever removed containers.
///
/// **A named volume is the resource this is written for.** `down` stops the
/// containers and the networks and leaves the volume standing, by design — that
/// is what docker volumes are for. So it outlives the Job unless something goes
/// and gets it, and nothing did: the machine this was written after held 171 of
/// them and 12.0 GB.
///
/// The fixture is created by this test, carries this process's id in its name,
/// and is removed on the way out whatever happens. **Nothing here can touch a
/// volume the machine's owner created** — it is addressed by a name this test
/// minted, never by a prune, never by a filter that could match anything else.
#[test]
fn a_live_workspaces_own_volume_is_released_when_it_cleans() {
    if !daemon_is_up() {
        eprintln!("skipping: no Docker daemon reachable");
        return;
    }

    let machine = Machine::new();
    let repo = machine.repo("main", CONFIG);
    let payload: Value =
        serde_json::from_slice(&machine.run(&repo, &["manifest", "init", "--json"]).stdout)
            .unwrap();
    let id = payload["workspace"].as_str().unwrap().to_string();
    let namespace = namespace_of(&machine);

    // Stamped exactly as the compose transform stamps a top-level `volumes:`
    // entry — which is a separate stamp from the services', because measured,
    // a service's labels do not reach the volume.
    let volume = format!("armada-test-livevol-{}", std::process::id());
    docker(&[
        "volume",
        "create",
        "--label",
        &format!("armada.workspace={id}"),
        "--label",
        &format!("armada.workspace_path={}", repo.display()),
        "--label",
        &format!("armada.namespace={namespace}"),
        &volume,
    ]);
    assert!(exists("volume", &volume), "the fixture was not created");

    let cleaned = machine.run(&repo, &["manifest", "clean", "--json"]);
    assert!(
        cleaned.status.success(),
        "{}",
        String::from_utf8_lossy(&cleaned.stderr)
    );

    // **Reported, never silent.** A reclaim nobody can audit is one nobody
    // trusts, and the count is what `armada fleet tick` puts on the row when a
    // Job finishes.
    let payload: Value = serde_json::from_slice(&cleaned.stdout).unwrap();
    let released = &payload["data"]["results"][0]["released"];
    assert_eq!(
        released["volumes"], 1,
        "the volume was not counted as released: {payload}"
    );

    let survived = exists("volume", &volume);
    // Removed before the assertion, so a failure leaves nothing behind either:
    // stray `armada-test-*` resources from an earlier run have twice been
    // misdiagnosed as flakiness.
    let _ = Command::new("docker")
        .args(["volume", "rm", "-f", &volume])
        .output();
    assert!(
        !survived,
        "the workspace's own named volume survived its own `clean` — this is the leak"
    );
}
