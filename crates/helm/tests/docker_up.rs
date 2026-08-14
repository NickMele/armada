//! The compose driver, against a **real** Docker daemon.
//!
//! `services.rs` covers the `command` driver, which needs nothing but a shell.
//! This is the other half, and it is the half that cannot be faked: every
//! measured fact PLAN.md §6.0 rests on is a property of compose and the daemon,
//! so a test with a fake `Run` would prove that Armada builds the argv it
//! intended and nothing about whether the argv works.
//!
//! Three claims, and the middle one is the founding bug of this project:
//!
//! 1. **The published port lands in the claimed block**, so two workspaces do
//!    not collide. An override cannot do this — compose *appends* to `ports:` —
//!    which is why Armada generates a whole document.
//! 2. **The network and the volumes carry Armada's labels.** Measured, compose
//!    does not propagate a service's labels to either, so stamping services is
//!    not stamping the stack: a `clean` that finds by label would find the
//!    containers and leave 29 per-worktree networks behind, exhausting the
//!    default bridge address pool. That is the outage this repository exists
//!    because of.
//! 3. **Nothing is orphaned.** After `clean`, no container, network or volume
//!    of this workspace's remains — which is the only claim that matters.
//!
//! Skipped, loudly, when no daemon is reachable. A test that silently passes
//! without a daemon is worse than one that is absent.

mod support;

use serde_json::Value;
use std::path::Path;
use std::process::Command;
use support::Machine;

fn daemon_is_up() -> bool {
    Command::new("docker")
        .args(["version", "--format", "{{.Server.Version}}"])
        .output()
        .map(|out| out.status.success())
        .unwrap_or(false)
}

fn compose_is_there() -> bool {
    Command::new("docker")
        .args(["compose", "version"])
        .output()
        .map(|out| out.status.success())
        .unwrap_or(false)
}

fn docker(args: &[&str]) -> String {
    let out = Command::new("docker").args(args).output();
    out.map(|out| String::from_utf8_lossy(&out.stdout).trim().to_string())
        .unwrap_or_default()
}

fn docker_succeeds(args: &[&str]) -> bool {
    Command::new("docker")
        .args(args)
        .output()
        .map(|out| out.status.success())
        .unwrap_or(false)
}

/// An image already on this machine, or `busybox` pulled if the network allows.
///
/// **Local before the registry**, because the claim is that this suite depends
/// on a daemon and not on one — a pull on every run would make that false.
fn runnable_image() -> Option<String> {
    let listed = docker(&["image", "ls", "--format", "{{.Repository}}:{{.Tag}}"]);
    if listed.lines().any(|line| line.trim() == "busybox:latest") {
        return Some("busybox:latest".to_string());
    }
    if docker_succeeds(&["pull", "-q", "busybox:latest"]) {
        return Some("busybox:latest".to_string());
    }
    None
}

/// Everything of one kind carrying this workspace's label.
fn labelled(kind: &[&str], workspace: &str) -> Vec<String> {
    let mut args: Vec<&str> = kind.to_vec();
    let filter = format!("label=armada.workspace={workspace}");
    args.extend(["--filter", &filter]);
    docker(&args)
        .lines()
        .map(str::trim)
        .filter(|line| !line.is_empty())
        .map(str::to_string)
        .collect()
}

/// The environment `docker compose` needs and the scratch machine deliberately
/// does not have.
///
/// **A harness fact, not a product one.** [`Machine`] clears the environment and
/// points `$HOME` at a temporary directory, which is what makes
/// `~/.armada/manifest.db` a fresh machine-global store rather than the
/// developer's. `docker compose` is a **CLI plugin**, and the CLI looks for
/// plugins under `$DOCKER_CONFIG` — defaulting to `$HOME/.docker` — so under a
/// scratch `$HOME` the daemon is reachable and `docker compose` is
/// *"unknown command"*. Measured on darwin against Docker 29.6.2 / Compose
/// v5.3.1. `DOCKER_HOST` rides along for a daemon that is not on the default
/// socket.
fn docker_env() -> Vec<(String, String)> {
    let mut env = Vec::new();
    let config = std::env::var("DOCKER_CONFIG").ok().or_else(|| {
        std::env::var("HOME")
            .ok()
            .map(|home| format!("{home}/.docker"))
    });
    if let Some(config) = config {
        env.push(("DOCKER_CONFIG".to_string(), config));
    }
    if let Ok(host) = std::env::var("DOCKER_HOST") {
        env.push(("DOCKER_HOST".to_string(), host));
    }
    env
}

/// `armada`, with the environment above layered on.
fn armada(machine: &Machine, repo: &Path, args: &[&str]) -> std::process::Output {
    let env = docker_env();
    let pairs: Vec<(&str, &str)> = env
        .iter()
        .map(|(key, value)| (key.as_str(), value.as_str()))
        .collect();
    machine.run_with_env(repo, args, &pairs)
}

fn workspace_id(machine: &Machine, repo: &Path) -> String {
    let out = armada(machine, repo, &["manifest", "status", "--json"]);
    let envelope: Value = serde_json::from_slice(&out.stdout).expect("the envelope parses");
    envelope["workspace"]
        .as_str()
        .expect("a workspace id")
        .to_string()
}

/// `docker inspect`, as JSON, for one resource's labels.
fn labels_of(kind: &str, reference: &str) -> Value {
    let path = match kind {
        "container" | "image" => "{{json .Config.Labels}}",
        _ => "{{json .Labels}}",
    };
    let text = docker(&[
        "inspect",
        &format!("--type={kind}"),
        "--format",
        path,
        reference,
    ]);
    serde_json::from_str(&text).unwrap_or(Value::Null)
}

/// **The whole compose driver, end to end, against a real daemon.**
#[test]
fn up_publishes_into_the_claimed_block_stamps_the_stack_and_leaves_no_orphans() {
    if !daemon_is_up() || !compose_is_there() {
        eprintln!(
            "SKIPPED: no reachable Docker daemon with `docker compose`. \
             The compose driver is unverified in this run."
        );
        return;
    }
    let Some(image) = runnable_image() else {
        eprintln!(
            "SKIPPED: no image on this machine and none could be pulled. \
             The compose driver is unverified in this run."
        );
        return;
    };

    let machine = Machine::new();
    let repo = machine.repo(
        "composed",
        "\
manifest:
  version: 1
  components:
    cache:
      run:
        driver: compose
        file: [docker-compose.yml]
        ports: { cache: 6379 }
        ready: { tcp: cache, timeout: 60 }
",
    );

    // **A bare `ports:` entry, deliberately, and it is the harder case.**
    // Measured: a bare entry resolves to `{mode: ingress, target: 6379}` with no
    // `published` key, and Docker then publishes it on an *ephemeral* host port
    // — so a transform that skipped it would leave the service somewhere the
    // claimed block does not cover, and the `tcp:` ready-check below would wait
    // on 5460 until it timed out. That is exactly what happened before this test
    // existed (`docs/traps.md`).
    //
    // A named volume and no `networks:` block, because those are the two the
    // daemon creates and does *not* inherit the service's labels.
    std::fs::write(
        repo.join("docker-compose.yml"),
        format!(
            "services:\n  \
               cache:\n    \
                 image: {image}\n    \
                 command: [\"sleep\", \"300\"]\n    \
                 ports:\n      \
                   - \"6379\"\n    \
                 volumes:\n      \
                   - cachedata:/data\n\
             volumes:\n  \
               cachedata:\n"
        ),
    )
    .unwrap();

    armada(&machine, &repo, &["manifest", "init"]);
    let id = workspace_id(&machine, &repo);

    // Whatever happens below, this workspace's resources go. Written before the
    // assertions rather than after them, because an assertion that fails must
    // not leave a labelled stack on the machine running the suite.
    let cleanup = || {
        armada(&machine, &repo, &["manifest", "clean", "--force"]);
    };

    let out = armada(&machine, &repo, &["manifest", "up", "--json"]);
    let envelope: Value = serde_json::from_slice(&out.stdout).expect("the envelope parses");
    if envelope["status"] != "UP" {
        cleanup();
        panic!("`up` did not bring the stack up: {envelope:#}");
    }

    let outcome = std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| {
        // 1. The published port is the claimed one, not the base one.
        let assigned = envelope["data"]["results"][0]["ports"]["cache"]["port"]
            .as_u64()
            .expect("an assigned port");
        assert_ne!(assigned, 6379, "the base port survived the transform");
        // **The ready-check passed, which is the half a label assertion cannot
        // prove.** `tcp: cache` waits on the *claimed* port, so `UP` above is
        // already evidence the service is reachable there rather than on an
        // ephemeral one — and the port row says so too.
        // **The ready-check passed and the port reads as bound**, which is the
        // half a label assertion cannot prove. `tcp: cache` waits on the
        // *claimed* port, so `UP` is already evidence the service is reachable
        // there rather than on an ephemeral one — and `LISTENING` is the probe
        // agreeing, which it could not do until it learned to see a wildcard
        // holder.
        assert_eq!(
            envelope["data"]["results"][0]["ports"]["cache"]["state"], "LISTENING",
            "the claimed port does not read as bound: {envelope:#}"
        );
        let published = docker(&["port", &format!("armada-{id}-cache-1"), "6379/tcp"]);
        assert!(
            published.contains(&assigned.to_string()),
            "published {published:?}, expected the claimed port {assigned}"
        );

        // 2. The stack is stamped, and the network and the volume separately
        //    from the service. This is the founding bug.
        let containers = labelled(&["ps", "-aq"], &id);
        let networks = labelled(&["network", "ls", "-q"], &id);
        let volumes = labelled(&["volume", "ls", "-q"], &id);
        assert_eq!(containers.len(), 1, "the container is not labelled");
        assert_eq!(
            networks.len(),
            1,
            "the network carries no armada.workspace label — \
             `clean` would find the containers and leave 29 networks behind"
        );
        assert_eq!(volumes.len(), 1, "the volume carries no label");

        // All three labels, on all three kinds. The path label is what makes a
        // reap self-sufficient — the id is a one-way hash — and the namespace
        // is what stops two installations sharing a daemon sharing a fate.
        for (kind, reference) in [
            ("container", &containers[0]),
            ("network", &networks[0]),
            ("volume", &volumes[0]),
        ] {
            let labels = labels_of(kind, reference);
            assert_eq!(labels["armada.workspace"], id, "{kind} {reference}");
            assert!(
                labels["armada.workspace_path"].is_string(),
                "{kind} carries no path label: {labels}"
            );
            assert!(
                labels["armada.namespace"].is_string(),
                "{kind} carries no namespace label: {labels}"
            );
        }

        // The ids Armada reports are the ids that exist.
        let owns: Vec<String> = envelope["data"]["results"][0]["owns"]
            .as_array()
            .expect("owns[]")
            .iter()
            .map(|id| id.as_str().unwrap().to_string())
            .collect();
        assert!(
            owns.iter().any(|id| id.starts_with("container:")),
            "up reported no container id: {owns:?}"
        );
        assert!(
            owns.iter().any(|id| id.starts_with("network:")),
            "up reported no network id: {owns:?}"
        );

        // 3. `down` stops the containers and **keeps the volume**: it is pause,
        //    and a named volume is the workspace's data.
        let out = armada(&machine, &repo, &["manifest", "down", "--json"]);
        let stopped: Value = serde_json::from_slice(&out.stdout).expect("the envelope parses");
        assert_eq!(stopped["status"], "DOWN", "{stopped:#}");
        assert!(
            labelled(&["ps", "-q"], &id).is_empty(),
            "a container survived `down`"
        );
        assert_eq!(
            labelled(&["volume", "ls", "-q"], &id).len(),
            1,
            "`down` removed the volume: that is `clean`'s job, and the data was the workspace's"
        );
    }));

    let cleaned: Value = serde_json::from_slice(
        &armada(&machine, &repo, &["manifest", "clean", "--force", "--json"]).stdout,
    )
    .expect("the envelope parses");

    // **`clean` does not report as `kept` what it then removed.** The reap pass
    // runs first and answers a narrower question than the verb does — *did the
    // reaper remove this?* — so a live workspace's own containers are correctly
    // left alone by it and removed a moment later by the release. Both halves
    // are right; the envelope describes the run, so a `kept` row for a container
    // that is gone by the time anyone reads it is a statement the run has
    // already falsified.
    let kept: Vec<&Value> = cleaned["data"]["reaped"]["reported"]
        .as_array()
        .map(|rows| rows.iter().collect())
        .unwrap_or_default();
    assert!(
        kept.is_empty(),
        "`clean` reported {} resource(s) as kept and then removed them: {:#}",
        kept.len(),
        cleaned["data"]["reaped"]
    );

    // 4. **No orphans.** The claim the whole ownership layer exists for.
    for (what, remaining) in [
        ("container", labelled(&["ps", "-aq"], &id)),
        ("network", labelled(&["network", "ls", "-q"], &id)),
        ("volume", labelled(&["volume", "ls", "-q"], &id)),
    ] {
        assert!(
            remaining.is_empty(),
            "`clean` left {} {what}(s) behind: {remaining:?}",
            remaining.len()
        );
    }

    if let Err(panic) = outcome {
        std::panic::resume_unwind(panic);
    }
}
