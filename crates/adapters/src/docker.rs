//! docker, as an ordinary adapter module: it builds argv and calls `ctx.run`.
//!
//! **Stamp at creation, reap by stamp.** Every container, network, volume and
//! built image char creates carries three labels, and all three are load-bearing:
//!
//! | Label | Why it is not redundant |
//! |---|---|
//! | `char.workspace` | the owner |
//! | `char.workspace_path` | `workspace_id` is a **one-way hash**, so from the id alone char cannot recover the path and cannot ask whether that workspace still exists. Without this, a missing row is indistinguishable from a dead workspace — and the database is per-`$HOME` while the daemon is per-machine, so that mistake deletes a *running* workspace's containers |
//! | `char.namespace` | scopes the mechanism to one filesystem view. Two char installations sharing a daemon is the ordinary devcontainer setup, and `/workspaces/repo` is `ENOENT` from the host |
//!
//! **char's own filters use both the id and the path**, never the id alone:
//! `workspace_id` is 32 bits, and a collision would have `char clean` in one
//! workspace destroy another's live containers.

use charkit_core::ctx::{Run, RunRequest, SpawnErrorKind};
use charkit_core::error::{CharError, ErrClass};
use charkit_core::id::WorkspaceId;
use charkit_core::reap::LabelledResource;
use charkit_core::registry::OwnedKind;
use std::collections::BTreeMap;
use std::path::{Path, PathBuf};
use std::time::Duration;

/// The owning workspace's id.
pub const LABEL_WORKSPACE: &str = "char.workspace";
/// The owning workspace's realpath, so a reap can `stat` it.
pub const LABEL_WORKSPACE_PATH: &str = "char.workspace_path";
/// The installation, so a shared daemon does not become a shared fate.
pub const LABEL_NAMESPACE: &str = "char.namespace";

/// The four things char stamps and reaps.
///
/// **Volumes were absent from this vocabulary entirely until a review found
/// it**, and a named volume outlives `down`, outlives the container, and is
/// invisible to every filter char would otherwise use to find it. Images are
/// here because the source repo records roughly 2.1 GB per production app
/// build — the single biggest thing a stale workspace holds — but **only images
/// char causes to be built**: a pulled `postgres:16` is shared with everything
/// else on the machine and was never char's to remove.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Kind {
    /// `docker ps`.
    Container,
    /// `docker network ls`.
    Network,
    /// `docker volume ls`.
    Volume,
    /// `docker image ls`.
    Image,
}

impl Kind {
    /// Every kind, in the order `clean` removes them: containers first,
    /// because a network with a container attached will not go.
    pub const ALL: [Kind; 4] = [Kind::Container, Kind::Network, Kind::Volume, Kind::Image];

    fn owned_kind(self) -> OwnedKind {
        match self {
            Kind::Container => OwnedKind::Container,
            Kind::Network => OwnedKind::Network,
            Kind::Volume => OwnedKind::Volume,
            Kind::Image => OwnedKind::Image,
        }
    }

    fn list_argv(self) -> Vec<String> {
        let words: &[&str] = match self {
            // `ps -a`, not `ps`: a stopped container still holds its name and
            // its volumes, and `clean` that leaves it behind leaves the leak.
            Kind::Container => &["docker", "ps", "-a", "-q"],
            Kind::Network => &["docker", "network", "ls", "-q"],
            Kind::Volume => &["docker", "volume", "ls", "-q"],
            Kind::Image => &["docker", "image", "ls", "-q"],
        };
        words.iter().map(|s| s.to_string()).collect()
    }

    fn remove_argv(self) -> Vec<String> {
        let words: &[&str] = match self {
            Kind::Container => &["docker", "rm", "-f", "-v"],
            Kind::Network => &["docker", "network", "rm"],
            Kind::Volume => &["docker", "volume", "rm", "-f"],
            Kind::Image => &["docker", "image", "rm", "-f"],
        };
        words.iter().map(|s| s.to_string()).collect()
    }

    /// The `inspect` type, and where that type keeps its labels.
    ///
    /// **Measured, because the obvious uniform version does not exist:**
    /// `docker image ls --format` supports neither `{{.Labels}}` nor
    /// `{{.Label "k"}}` — it errors with *can't evaluate field Labels in type
    /// `*formatter.imageContext`* — while `ps`, `network ls` and `volume ls`
    /// all support `.Labels`. So char reads labels through `inspect`, which
    /// works for every type, and the label path differs there too: containers
    /// and images keep them under `.Config.Labels`, networks and volumes under
    /// `.Labels`.
    fn inspect(self) -> (&'static str, &'static str, &'static str) {
        match self {
            Kind::Container => ("container", ".Config.Labels", ".Id"),
            Kind::Image => ("image", ".Config.Labels", ".Id"),
            Kind::Network => ("network", ".Labels", ".Id"),
            Kind::Volume => ("volume", ".Labels", ".Name"),
        }
    }
}

/// Whether the daemon is reachable, as an `environment` question.
///
/// **Measured: `docker compose config` succeeds with no daemon at all** — it is
/// client-side — so a design that discovers the daemon is gone at the end does
/// its whole resolve-and-transform first. Probing up front is what turns "the
/// stack failed to start" into "Docker is not running", which is the difference
/// between an agent fixing a repo that is fine and a human starting Docker.
pub fn daemon_ready(
    run: &impl Run,
    cwd: &Path,
    timeout: Duration,
    tick: &mut dyn FnMut(),
) -> Result<(), CharError> {
    let request = RunRequest::new(
        ["docker", "version", "--format", "{{.Server.Version}}"]
            .iter()
            .map(|s| s.to_string())
            .collect(),
        cwd.to_path_buf(),
    )
    .timeout(timeout);

    let outcome = run.call_with_tick(&request, tick);
    tick();
    match outcome {
        Ok(output) if output.ok() => Ok(()),
        Ok(output) if output.timed_out => Err(CharError {
            class: ErrClass::Environment,
            r#where: "docker".to_string(),
            message: format!("the Docker daemon did not answer within {timeout:?}"),
            next_action: Some("check that Docker is running and responsive".to_string()),
        }),
        Ok(output) => Err(CharError {
            class: ErrClass::Environment,
            r#where: "docker".to_string(),
            message: format!("the Docker daemon is unreachable: {}", output.stderr.trim()),
            next_action: Some("start Docker, then retry unchanged".to_string()),
        }),
        Err(e) if e.kind == SpawnErrorKind::NotFound => Err(CharError {
            class: ErrClass::Environment,
            r#where: "docker".to_string(),
            message: "docker is not on PATH".to_string(),
            next_action: Some("install Docker, then retry unchanged".to_string()),
        }),
        Err(e) => Err(CharError {
            class: ErrClass::Environment,
            r#where: "docker".to_string(),
            message: format!("cannot run docker: {}", e.message),
            next_action: None,
        }),
    }
}

/// Every resource of one kind carrying `char.workspace`, with its three labels.
pub fn list_labelled(
    run: &impl Run,
    cwd: &Path,
    timeout: Duration,
    kind: Kind,
    tick: &mut dyn FnMut(),
) -> Result<Vec<LabelledResource>, CharError> {
    let mut argv = kind.list_argv();
    argv.push("--filter".to_string());
    argv.push(format!("label={LABEL_WORKSPACE}"));

    let listed = call(run, cwd, timeout, argv, tick)?;
    let ids: Vec<String> = listed
        .lines()
        .map(str::trim)
        .filter(|line| !line.is_empty())
        .map(str::to_string)
        .collect();
    if ids.is_empty() {
        return Ok(Vec::new());
    }

    let (kind_flag, labels_path, id_path) = kind.inspect();
    let mut argv: Vec<String> = vec![
        "docker".to_string(),
        "inspect".to_string(),
        format!("--type={kind_flag}"),
        "--format".to_string(),
        // JSON rather than a tab-separated list of label values: a workspace
        // path may legally contain a tab or a newline, and a delimiter a path
        // can contain is a delimiter that eventually mis-attributes a resource.
        format!("{{{{{id_path}}}}}\t{{{{json {labels_path}}}}}"),
    ];
    argv.extend(ids);

    let inspected = call(run, cwd, timeout, argv, tick)?;
    Ok(inspected
        .lines()
        .filter_map(|line| parse_inspect_line(line, kind))
        .collect())
}

fn parse_inspect_line(line: &str, kind: Kind) -> Option<LabelledResource> {
    let (reference, labels) = line.split_once('\t')?;
    let labels: BTreeMap<String, String> = serde_json::from_str(labels).unwrap_or_default();
    let workspace = labels.get(LABEL_WORKSPACE)?;
    Some(LabelledResource {
        kind: kind.owned_kind(),
        reference: reference.trim().to_string(),
        workspace: WorkspaceId::from_stored(workspace.clone()),
        // A resource stamped by a char too old to write the path label has no
        // path to stat. `PathBuf::new()` stats as `Missing`, which would make
        // it reapable — so it is stamped with a path that cannot exist and is
        // reported instead, because "labelled, unknowable" is exactly the case
        // the rule says never to remove.
        workspace_path: labels
            .get(LABEL_WORKSPACE_PATH)
            .map(PathBuf::from)
            .unwrap_or_else(|| PathBuf::from("/nonexistent/char-unstamped")),
        namespace: labels.get(LABEL_NAMESPACE).cloned(),
    })
}

/// Every handle matching a **declared** `owns:` selector.
///
/// This is a distinct path from [`list_labelled`], and deliberately so. A
/// `commands:` entry's `owns:` is a *selector, not a record*: the command runs
/// ad hoc, so there is no "while it was up" window to record against, and
/// `clean` evaluates the declaration against docker at the time it runs. The
/// selector is passed to `--filter` verbatim after substitution, because the
/// repo is naming resources by *its own* labels — `label=com.example.worktree=…`
/// — which char has no vocabulary for and no business rewriting.
pub fn list_by_selector(
    run: &impl Run,
    cwd: &Path,
    timeout: Duration,
    kind: Kind,
    selector: &str,
    tick: &mut dyn FnMut(),
) -> Result<Vec<String>, CharError> {
    let mut argv = kind.list_argv();
    argv.push("--filter".to_string());
    argv.push(selector.to_string());
    Ok(call(run, cwd, timeout, argv, tick)?
        .lines()
        .map(str::trim)
        .filter(|line| !line.is_empty())
        .map(str::to_string)
        .collect())
}

/// Remove resources by handle. Removal is best-effort per handle: one
/// container that will not go must not strand the network behind it.
pub fn remove(
    run: &impl Run,
    cwd: &Path,
    timeout: Duration,
    kind: Kind,
    references: &[String],
    tick: &mut dyn FnMut(),
) -> Vec<(String, Option<CharError>)> {
    references
        .iter()
        .map(|reference| {
            let mut argv = kind.remove_argv();
            argv.push(reference.clone());
            (reference.clone(), call(run, cwd, timeout, argv, tick).err())
        })
        .collect()
}

/// The label arguments char stamps onto everything it creates.
///
/// Returned as pairs rather than as `--label k=v` strings because the compose
/// driver (phase 4) has to write them into a generated document instead —
/// measured, `docker compose up` has no `--label` flag at all, so stamping
/// services is done through the document and stamping the *stack* means
/// stamping the top-level `networks:` and `volumes:` separately.
pub fn stamp(workspace: &WorkspaceId, path: &Path, namespace: &str) -> BTreeMap<String, String> {
    BTreeMap::from([
        (LABEL_WORKSPACE.to_string(), workspace.as_str().to_string()),
        (LABEL_WORKSPACE_PATH.to_string(), path.display().to_string()),
        (LABEL_NAMESPACE.to_string(), namespace.to_string()),
    ])
}

/// One docker subprocess, with the caller's heartbeat driven **around and
/// through it**.
///
/// **The renewal belongs here rather than around the functions above**, because
/// those make more than one call each: `remove` runs one `docker rm` per
/// handle and `list_labelled` is an `ls` followed by an `inspect`, so a
/// renewal placed at the adapter's edge leaves a gap bounded by the *batch*
/// rather than by one call — three handles against a hung daemon is ninety
/// seconds of silence, and a lease goes cold at sixty. Both halves are needed:
/// `call_with_tick` covers one call that waits a long time (PLAN.md §4.3 puts
/// renewal in the loop that waits), and the tick after it covers many calls
/// that each return quickly, which no in-wait timer ever fires for.
fn call(
    run: &impl Run,
    cwd: &Path,
    timeout: Duration,
    argv: Vec<String>,
    tick: &mut dyn FnMut(),
) -> Result<String, CharError> {
    let request = RunRequest::new(argv.clone(), cwd.to_path_buf()).timeout(timeout);
    let output = run.call_with_tick(&request, tick);
    tick();
    let output = output.map_err(|e| CharError {
        class: ErrClass::Environment,
        r#where: "docker".to_string(),
        message: match e.kind {
            SpawnErrorKind::NotFound => "docker is not on PATH".to_string(),
            _ => format!("cannot run docker: {}", e.message),
        },
        next_action: None,
    })?;

    if output.timed_out {
        return Err(CharError {
            class: ErrClass::Environment,
            r#where: "docker".to_string(),
            message: format!("`{}` exceeded char's docker timeout", argv.join(" ")),
            next_action: Some("check that Docker is running and responsive".to_string()),
        });
    }
    if !output.ok() {
        return Err(CharError {
            class: ErrClass::Environment,
            r#where: "docker".to_string(),
            message: format!("`{}` failed: {}", argv.join(" "), output.stderr.trim()),
            next_action: None,
        });
    }
    Ok(output.stdout)
}

#[cfg(test)]
mod tests {
    use super::*;
    use charkit_core::ctx::{RunOutput, SpawnError};
    use std::cell::RefCell;

    #[derive(Default)]
    struct FakeRun {
        seen: RefCell<Vec<Vec<String>>>,
        replies: RefCell<Vec<String>>,
    }

    impl FakeRun {
        fn with(replies: &[&str]) -> Self {
            FakeRun {
                seen: RefCell::new(Vec::new()),
                replies: RefCell::new(replies.iter().rev().map(|s| s.to_string()).collect()),
            }
        }
    }

    impl Run for FakeRun {
        fn call(&self, request: &RunRequest) -> Result<RunOutput, SpawnError> {
            self.seen.borrow_mut().push(request.argv.clone());
            Ok(RunOutput {
                code: Some(0),
                signal: None,
                stdout: self.replies.borrow_mut().pop().unwrap_or_default(),
                stderr: String::new(),
                timed_out: false,
            })
        }
    }

    fn cwd() -> &'static Path {
        Path::new("/srv/repo")
    }

    /// The argv assertion this whole seam exists for: `--filter
    /// label=char.workspace` selects char's resources, and writing it without
    /// the `label=` prefix or against the wrong subcommand silently selects
    /// everything or nothing.
    #[test]
    fn listing_filters_on_the_workspace_label() {
        let run = FakeRun::with(&["", ""]);
        list_labelled(
            &run,
            cwd(),
            Duration::from_secs(30),
            Kind::Container,
            &mut || {},
        )
        .unwrap();
        assert_eq!(
            run.seen.borrow()[0],
            vec![
                "docker",
                "ps",
                "-a",
                "-q",
                "--filter",
                "label=char.workspace"
            ]
        );
    }

    #[test]
    fn a_stopped_container_is_still_listed() {
        // `-a` is the difference between reaping a stopped container and
        // leaving it holding its name and its volumes.
        let run = FakeRun::with(&[""]);
        list_labelled(
            &run,
            cwd(),
            Duration::from_secs(30),
            Kind::Container,
            &mut || {},
        )
        .unwrap();
        assert!(run.seen.borrow()[0].contains(&"-a".to_string()));
    }

    #[test]
    fn labels_are_read_through_inspect_because_image_ls_cannot_format_them() {
        let run = FakeRun::with(&[
            "sha256:abc\n",
            "sha256:abc\t{\"char.workspace\":\"a3f91c02\",\
             \"char.workspace_path\":\"/srv/repo\",\"char.namespace\":\"ns-1\"}\n",
        ]);
        let found = list_labelled(
            &run,
            cwd(),
            Duration::from_secs(30),
            Kind::Image,
            &mut || {},
        )
        .unwrap();
        assert_eq!(run.seen.borrow()[1][2], "--type=image");
        assert!(run.seen.borrow()[1][4].contains(".Config.Labels"));
        assert_eq!(found.len(), 1);
        assert_eq!(found[0].workspace, WorkspaceId::from_stored("a3f91c02"));
        assert_eq!(found[0].workspace_path, PathBuf::from("/srv/repo"));
        assert_eq!(found[0].namespace.as_deref(), Some("ns-1"));
    }

    #[test]
    fn networks_and_volumes_read_their_labels_from_a_different_path() {
        let run = FakeRun::with(&["net1\n", "net1\t{\"char.workspace\":\"a3f91c02\"}\n"]);
        list_labelled(
            &run,
            cwd(),
            Duration::from_secs(30),
            Kind::Network,
            &mut || {},
        )
        .unwrap();
        let argv = run.seen.borrow()[1].clone();
        assert!(argv[4].contains("json .Labels"), "{argv:?}");
        assert!(!argv[4].contains(".Config"), "{argv:?}");
    }

    /// A path containing a tab or a newline is legal, and a delimiter a path
    /// can contain is one that eventually mis-attributes a resource.
    #[test]
    fn a_label_value_containing_a_delimiter_survives_parsing() {
        let line = "c1\t{\"char.workspace\":\"a3f91c02\",\
                    \"char.workspace_path\":\"/srv/od\\td\\nname\"}";
        let parsed = parse_inspect_line(line, Kind::Container).unwrap();
        assert_eq!(parsed.workspace_path, PathBuf::from("/srv/od\td\nname"));
    }

    /// "Labelled, no path" is not "labelled, path missing". A resource stamped
    /// by a char too old to write the path label must be **reported**, and the
    /// stand-in path is what makes that fall out of the ordinary rule.
    #[test]
    fn a_resource_with_no_path_label_is_never_reapable() {
        let parsed =
            parse_inspect_line("c1\t{\"char.workspace\":\"a3f91c02\"}", Kind::Container).unwrap();
        assert_eq!(parsed.namespace, None);
        let (remove, reported) = charkit_core::reap::resource_pass(
            &[(parsed, charkit_core::reap::PathStat::Missing)],
            "ns-1",
        );
        assert!(remove.is_empty());
        assert_eq!(reported.len(), 1);
    }

    #[test]
    fn an_unlabelled_resource_is_skipped_rather_than_guessed_at() {
        assert!(parse_inspect_line("c1\t{}", Kind::Container).is_none());
        assert!(parse_inspect_line("c1\t<no value>", Kind::Container).is_none());
        assert!(parse_inspect_line("nonsense", Kind::Container).is_none());
    }

    #[test]
    fn removal_is_one_call_per_handle_so_one_failure_strands_nothing() {
        let run = FakeRun::with(&["", ""]);
        let results = remove(
            &run,
            cwd(),
            Duration::from_secs(30),
            Kind::Network,
            &["net1".to_string(), "net2".to_string()],
            &mut || {},
        );
        assert_eq!(results.len(), 2);
        assert_eq!(
            run.seen.borrow()[0],
            vec!["docker", "network", "rm", "net1"]
        );
        assert_eq!(
            run.seen.borrow()[1],
            vec!["docker", "network", "rm", "net2"]
        );
    }

    /// The heartbeat is bounded by **one subprocess**, not by the batch. Three
    /// handles is three `docker rm` calls, each with its own deadline, and a
    /// renewal that only happened when `remove` returned would leave ninety
    /// seconds of silence against a hung daemon — thirty past cold.
    #[test]
    fn every_subprocess_gets_a_tick_however_many_one_call_makes() {
        let run = FakeRun::with(&["", "", ""]);
        let mut ticks = 0;
        remove(
            &run,
            cwd(),
            Duration::from_secs(30),
            Kind::Network,
            &["net1".to_string(), "net2".to_string(), "net3".to_string()],
            &mut || ticks += 1,
        );
        assert!(ticks >= 3, "one tick per `docker rm` at least, got {ticks}");

        // Two calls inside one function, for the same reason.
        let run = FakeRun::with(&["c1\n", "c1\t{\"char.workspace\":\"a3f91c02\"}\n"]);
        let mut ticks = 0;
        list_labelled(
            &run,
            cwd(),
            Duration::from_secs(30),
            Kind::Container,
            &mut || ticks += 1,
        )
        .unwrap();
        assert!(
            ticks >= 2,
            "the `ls` and the `inspect` both tick, got {ticks}"
        );
    }

    /// A declared selector reaches `--filter` verbatim: the repo is naming
    /// resources by its own labels, which char has no vocabulary for and no
    /// business rewriting.
    #[test]
    fn a_declared_selector_reaches_the_filter_untouched() {
        let run = FakeRun::with(&["c1\nc2\n"]);
        let found = list_by_selector(
            &run,
            cwd(),
            Duration::from_secs(30),
            Kind::Container,
            "label=com.example.worktree=a3f91c02",
            &mut || {},
        )
        .unwrap();
        assert_eq!(found, vec!["c1", "c2"]);
        assert_eq!(
            run.seen.borrow()[0].last().unwrap(),
            "label=com.example.worktree=a3f91c02"
        );
    }

    #[test]
    fn the_stamp_carries_all_three_labels() {
        let stamp = stamp(
            &WorkspaceId::from_stored("a3f91c02"),
            Path::new("/srv/repo"),
            "ns-1",
        );
        assert_eq!(stamp[LABEL_WORKSPACE], "a3f91c02");
        assert_eq!(stamp[LABEL_WORKSPACE_PATH], "/srv/repo");
        assert_eq!(stamp[LABEL_NAMESPACE], "ns-1");
    }

    #[test]
    fn a_missing_docker_binary_is_environment_and_not_tool_failed() {
        struct Missing;
        impl Run for Missing {
            fn call(&self, request: &RunRequest) -> Result<RunOutput, SpawnError> {
                Err(SpawnError {
                    program: request.argv[0].clone(),
                    kind: SpawnErrorKind::NotFound,
                    message: "no such file".to_string(),
                })
            }
        }
        let err = daemon_ready(&Missing, cwd(), Duration::from_secs(1), &mut || {}).unwrap_err();
        assert_eq!(err.class, ErrClass::Environment);
        assert_eq!(err.class.exit_code(), 6);
    }

    #[test]
    fn a_hung_daemon_hits_chars_own_deadline_rather_than_blocking_forever() {
        struct Hung;
        impl Run for Hung {
            fn call(&self, _: &RunRequest) -> Result<RunOutput, SpawnError> {
                Ok(RunOutput {
                    code: None,
                    signal: Some(9),
                    stdout: String::new(),
                    stderr: String::new(),
                    timed_out: true,
                })
            }
        }
        let err = daemon_ready(&Hung, cwd(), Duration::from_secs(1), &mut || {}).unwrap_err();
        assert_eq!(err.class, ErrClass::Environment);
        assert!(err.message.contains("did not answer"));
    }
}
