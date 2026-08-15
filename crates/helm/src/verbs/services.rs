//! The shell behind `up` and `down`: the drivers, and the loop that performs
//! what [`armada_core::lifecycle`] proposes.
//!
//! **Nothing here decides anything.** The reducer decides the order, the
//! cascade, the deadline and the verdict; this file attempts what it asked for
//! and feeds the answer back as an event (`ARCHITECTURE.md` §1.2).
//!
//! # The two drivers, and why there are only two
//!
//! `compose` and `command`. No `tilt`, no `bazel`, no `make`: a driver plugin
//! system means a public lifecycle contract, error semantics and a versioning
//! story — permanent API surface for a third driver that may never arrive — and
//! `driver: command` already *is* the plugin system. The one thing a
//! first-class driver gives you that a bare command cannot is knowing what it
//! created, and `owns:` gives a `command` component exactly that (PLAN.md §6.1).
//!
//! # Recording, and the two shapes it takes
//!
//! *"Everything `up` starts must be recorded as owned, at the moment it starts,
//! before it is confirmed working."* The two drivers satisfy that differently,
//! and the difference is worth stating because only one of them needs a row:
//!
//! | Driver | What makes it reclaimable | When that exists |
//! |---|---|---|
//! | `compose` | the three `armada.*` labels, stamped into the document | **before compose creates anything** — the document is transformed and then handed over, so no container can exist unlabelled |
//! | `command` | the recorded pgid, and nothing else | a pgid carries no label, so the `owned` row is the only record there will ever be |
//!
//! So a compose service's record-before-create is satisfied by construction,
//! and a `command` service gets an **intent row** — `pgid:pending:<component>` —
//! written before the fork and replaced with the real group id straight after
//! it. If Armada is killed in between, that row survives and says so; nothing
//! else could. It is dropped on every ordinary path, so it only ever appears
//! when it is the truth.
//!
//! The compose ids are still recorded, immediately after `up -d` returns, both
//! because `results[].owns[]` is specified to name them and because a row costs
//! nothing next to a leak.
//!
//! # A compose component is a project, not a service
//!
//! **Stated because it is a limit rather than a choice.** `run:` has a `file:`
//! and no `service:` key, so nothing in the model maps a component name to a
//! compose service name — and `docker compose up -d` brings the whole project
//! up regardless. So `armada manifest up postgres` on a compose component
//! starts every service in that file, and `down postgres` stops every one of
//! them. Two components declaring the same `file:` are two rows over one
//! project: the second `up -d` is a no-op compose performs quickly, and each
//! gets its own ready-check, which is the part that differs between them.
//!
//! Inventing a `service:` key to narrow this is a config change and belongs to
//! whoever decides the schema, not here. `driver: command` is the granular
//! driver today, and its selector is honoured exactly.

use armada_core::compose;
use armada_core::config::{ReadyKind, ResolvedConfig, ResolvedRun, ResolvedService};
use armada_core::ctx::{Clock, Fetch, Run, RunRequest, SpawnErrorKind, StdioMode};
use armada_core::envelope::{aggregate, Envelope, ResultRow, ServicesData, UpDryRun};
use armada_core::error::{ArmadaError, ConfigWhere, ErrClass, Status};
use armada_core::lease::{LeaseId, Policy};
use armada_core::lifecycle::{self, Action, Attempt, Direction, Event, State};
use armada_core::ports::{self, PortBlock};
use armada_core::registry::{OwnedKind, OwnedRow};
use armada_core::service;
use armada_core::template::{self, Site, Vars};
use armada_core::workspace::Workspace;
use armada_manifest::docker;
use armada_manifest::{fs, net, posix, process};
use std::collections::BTreeMap;
use std::time::Duration;

use crate::app::{self, App};
use crate::verbs::{load_config, Output};

/// How a `command` service's intent row is spelled before its group exists.
const PENDING: &str = "pending:";

/// Run `up` or `down` end to end.
pub fn run<R: Run, C: Clock, F: Fetch>(
    app: &mut App<R, C, F>,
    direction: Direction,
    selector: Option<&str>,
    dry_run: bool,
) -> Result<Output, ArmadaError> {
    let (workspace, config) = load_config(app)?;
    let verb = match direction {
        Direction::Up => "up",
        Direction::Down => "down",
    };

    // **`up` will not claim a block.** It is not `init`, and claiming one here
    // would make the two verbs interchangeable in a way that hides a missing
    // `init` until something else fails for a reason two steps away.
    let block = held_block(app, &workspace, verb)?;
    // No block means nothing to assign, which is the ordinary state of a
    // workspace whose services publish nothing.
    let assigned = match block {
        Some(block) => ports::assign_ports(&config, block, &workspace.config_label)?,
        None => BTreeMap::new(),
    };

    let selected = service::select(&config, selector, verb)?;
    let order = match direction {
        Direction::Up => service::start_order(&config, &selected, &workspace.config_label)?,
        Direction::Down => service::stop_order(&config, &selected, &workspace.config_label)?,
    };

    if dry_run {
        let preview = dry(app, &workspace, &config, &order, &assigned)?;
        return Ok(Output::UpDryRun(Box::new(Envelope::ok(
            verb,
            Some(workspace.id.clone()),
            Status::Up,
            preview,
        ))));
    }

    // The run lease covers everything that mutates. Two agents in one worktree
    // is the ordinary case, and without it one's `down` tears down what the
    // other's `up` is mid-way through starting.
    let lease = LeaseId::run(workspace.id.clone());
    let envelope = app::with_lease(app, lease.clone(), Policy::FailFast, None, |app| {
        drive(
            app, &workspace, &config, direction, order, &assigned, block, &lease,
        )
    })?;

    Ok(match direction {
        Direction::Up => Output::Up(Box::new(envelope)),
        Direction::Down => Output::Down(Box::new(envelope)),
    })
}

/// This workspace's block, or the refusal that names the missing verb.
///
/// **Registered-with-no-block and not-registered-at-all are different answers**,
/// and only the second is a refusal. A workspace whose components declare no
/// `ports:` is initialised, has services that publish nothing, and starts
/// perfectly well; refusing it would make `armada manifest up` unusable for
/// every repository that runs something without exposing a port.
fn held_block<R: Run, C: Clock, F: Fetch>(
    app: &mut App<R, C, F>,
    workspace: &Workspace,
    verb: &str,
) -> Result<Option<PortBlock>, ArmadaError> {
    app.db
        .workspaces()?
        .into_iter()
        .find(|row| row.id == workspace.id)
        .map(|row| row.ports)
        .ok_or_else(|| ArmadaError {
            class: ErrClass::BadInvocation,
            r#where: verb.to_string(),
            message: "this workspace is not initialised".to_string(),
            next_action: Some("`armada manifest init` claims one; `up` will not".to_string()),
        })
}

/// The loop: step the reducer, perform what it asks, feed the answer back.
#[allow(clippy::too_many_arguments)]
fn drive<R: Run, C: Clock, F: Fetch>(
    app: &mut App<R, C, F>,
    workspace: &Workspace,
    config: &ResolvedConfig,
    direction: Direction,
    order: Vec<String>,
    assigned: &BTreeMap<String, u16>,
    block: Option<PortBlock>,
    lease: &LeaseId,
) -> Result<Envelope<ServicesData>, ArmadaError> {
    let mut state = State::new(direction, order, needs_map(config), &ready_timeouts(config));
    let verb = match direction {
        Direction::Up => "up",
        Direction::Down => "down",
    };

    let mut results: Vec<ResultRow> = Vec::new();
    let mut event = Event::Tick {
        now_mono: app.ctx.now.mono(),
    };

    loop {
        let (next, actions) = lifecycle::step(state, event);
        state = next;
        let mut pending: Option<Event> = None;

        for action in actions {
            match action {
                Action::Record { service } => {
                    pending = Some(match record(app, workspace, config, &service) {
                        Ok(()) => Event::Recorded { service },
                        Err(error) => Event::RecordFailed { service, error },
                    });
                }
                Action::Start { service } => {
                    pending = Some(start(app, workspace, config, &service, assigned, lease));
                }
                Action::Stop { service } => {
                    pending = Some(stop(app, workspace, config, &service, assigned, lease));
                }
                Action::Probe { service } => {
                    pending = Some(probe(app, workspace, config, &service, assigned));
                }
                Action::Sleep { until_mono } => {
                    app.ctx.now.sleep_until(until_mono);
                    app.renew_held();
                    pending = Some(Event::Tick {
                        now_mono: app.ctx.now.mono(),
                    });
                }
                Action::Emit { result } => {
                    let mut row = ResultRow::from(&result);
                    row.ports = probed_ports(config, &result.id, assigned);
                    results.push(row);
                }
                Action::Finish { status } => {
                    let data = ServicesData {
                        port_block: block,
                        results,
                    };
                    let error = aggregate(&data.results, direction.subject());
                    return Ok(match error {
                        None => Envelope::ok(verb, Some(workspace.id.clone()), status, data),
                        Some(error) => {
                            let mut envelope =
                                Envelope::failed(verb, Some(workspace.id.clone()), error, data);
                            // The terminal state describes *what happened*; the
                            // class states *why*, and the exit code follows the
                            // class and never the state (PLAN.md §3.1).
                            envelope.status = status;
                            envelope
                        }
                    });
                }
            }
        }

        // A signal reaches the run through the same door every other fact does.
        if posix::interrupted() && !state.interrupted {
            event = Event::Interrupted;
            continue;
        }

        event = match pending {
            Some(event) => event,
            None => Event::Tick {
                now_mono: app.ctx.now.mono(),
            },
        };
    }
}

// ------------------------------------------------------------------ recording

/// Write what has to exist before anything is created.
///
/// See this module's header: a compose service is made reclaimable by the
/// labels already in the document, and a `command` service by a row that is the
/// only record its process group will ever have.
fn record<R: Run, C: Clock, F: Fetch>(
    app: &mut App<R, C, F>,
    workspace: &Workspace,
    config: &ResolvedConfig,
    service: &str,
) -> Result<(), ArmadaError> {
    match run_of(config, service) {
        Some(ResolvedRun::Command { .. }) => app.db.record_owned(&OwnedRow {
            workspace: workspace.id.clone(),
            kind: OwnedKind::Pgid,
            reference: format!("{PENDING}{service}"),
            boot_id: Some(app.boot_id.clone()),
            pid_started_at: None,
            component: Some(service.to_string()),
        }),
        // Nothing to write: the stamp is in the document, and the document is
        // handed to compose before compose creates anything.
        Some(ResolvedRun::Compose { .. }) | None => Ok(()),
    }
}

/// Replace a `command` service's intent row with the group it actually got.
fn settle_pgid<R: Run, C: Clock, F: Fetch>(
    app: &mut App<R, C, F>,
    workspace: &Workspace,
    service: &str,
    pgid: Option<i32>,
) -> Vec<String> {
    let _ = app.db.delete_owned(
        &workspace.id,
        OwnedKind::Pgid,
        &format!("{PENDING}{service}"),
    );
    let Some(pgid) = pgid.filter(|pgid| *pgid > 0) else {
        return Vec::new();
    };
    let started = armada_manifest::machine::process_start_at(&app.ctx.run, &app.cwd(), pgid);
    let _ = app.db.record_owned(&OwnedRow {
        workspace: workspace.id.clone(),
        kind: OwnedKind::Pgid,
        reference: pgid.to_string(),
        component: Some(service.to_string()),
        boot_id: Some(app.boot_id.clone()),
        // **Both halves, or the row is a permanent phantom.** Without a start
        // time a recycled pid is indistinguishable from an orphaned service, so
        // `clean` can only report it — forever, across reboots (PLAN.md §2.3.1).
        pid_started_at: started,
    });
    vec![format!("pgid:{pgid}")]
}

// -------------------------------------------------------------------- drivers

/// Start one service through its driver.
fn start<R: Run, C: Clock, F: Fetch>(
    app: &mut App<R, C, F>,
    workspace: &Workspace,
    config: &ResolvedConfig,
    service: &str,
    assigned: &BTreeMap<String, u16>,
    lease: &LeaseId,
) -> Event {
    let outcome = match run_of(config, service) {
        Some(ResolvedRun::Compose { file, common }) => {
            let file = file.clone();
            let common = common.clone();
            compose_up(app, workspace, config, service, &file, &common, assigned)
        }
        Some(ResolvedRun::Command {
            cmd, shell, common, ..
        }) => {
            let (cmd, shell, common) = (cmd.clone(), *shell, common.clone());
            command_up(
                app, workspace, config, service, &cmd, shell, &common, assigned,
            )
        }
        None => Err(Box::new((
            Attempt::default(),
            ArmadaError {
                class: ErrClass::ArmadaBug,
                r#where: service.to_string(),
                message: "a service with no `run:` reached the start path".to_string(),
                next_action: None,
            },
        ))),
    };
    let _ = app.db.renew(lease, app.ctx.now.mono());

    match outcome {
        Ok(attempt) => Event::Spawned {
            service: service.to_string(),
            attempt,
        },
        Err(boxed) => {
            let (attempt, error) = *boxed;
            Event::SpawnFailed {
                service: service.to_string(),
                attempt,
                error,
            }
        }
    }
}

/// The four steps of PLAN.md §6.0, in order.
fn compose_up<R: Run, C: Clock, F: Fetch>(
    app: &mut App<R, C, F>,
    workspace: &Workspace,
    config: &ResolvedConfig,
    service: &str,
    files: &[String],
    common: &ResolvedService,
    assigned: &BTreeMap<String, u16>,
) -> Result<Attempt, Box<(Attempt, ArmadaError)>> {
    // **The daemon is probed before any of the work.** Measured: `docker
    // compose config` returns 0 against a dead daemon, so steps 1 through 3 all
    // succeed and Armada would discover the daemon is gone only at step 4 —
    // reporting "the stack failed to start" for "Docker is not running".
    app.docker_ready()
        .map_err(|e| Box::new((Attempt::default(), e)))?;

    let project = compose::project(workspace.id.as_str());
    let root = workspace.root.display().to_string();

    let document = call_docker(
        app,
        compose::resolve_argv(files, &project, &root),
        None,
        app.machine.docker_deadline(),
        ErrClass::Environment,
        service,
    )
    .map_err(|e| Box::new((Attempt::default(), e)))?;

    let names = compose::port_names(config, &workspace.config_label)
        .map_err(|e| Box::new((Attempt::default(), e)))?;
    let labels = docker::stamp(&workspace.id, &workspace.root, &app.namespace);
    let transformed = compose::transform(
        &document,
        &names,
        assigned,
        &labels,
        &workspace.config_label,
    )
    .map_err(|e| Box::new((Attempt::default(), e)))?;

    let argv = compose::up_argv(&project, &root);
    let started = call_docker(
        app,
        argv.clone(),
        Some(transformed),
        app.machine.up_deadline(),
        // **A slow build is a slow build**, not a broken machine: only the
        // question calls are `environment` on expiry (PLAN.md §6).
        ErrClass::Timeout,
        service,
    );

    // **Whatever exists is collected either way.** A `compose up` that created
    // two containers and could not create the third leaves two that are
    // Armada's, and a failure path that skipped this would strand them.
    let attempt = Attempt {
        owns: adopt_compose(app, workspace, &project),
        argv,
        log: None,
        ready: Some(describe_ready(common, assigned)),
    };
    match started {
        Ok(_) => Ok(attempt),
        Err(error) => Err(Box::new((attempt, error))),
    }
}

/// Find every resource the compose project created, record it, and name it.
///
/// **Containers, networks and volumes by the compose project label; images by
/// Armada's own.** Compose applies its project label to what it creates, which
/// is the precise filter here — while an *image* is only Armada's when Armada
/// caused it to be built, which is what `build.labels` marks and a pulled
/// `postgres:16` does not carry.
fn adopt_compose<R: Run, C: Clock, F: Fetch>(
    app: &mut App<R, C, F>,
    workspace: &Workspace,
    project: &str,
) -> Vec<String> {
    let by_project = format!("label=com.docker.compose.project={project}");
    let by_workspace = format!(
        "label={}={}",
        docker::LABEL_WORKSPACE,
        workspace.id.as_str()
    );

    let mut owns = Vec::new();
    for (kind, owned, selector) in [
        (docker::Kind::Container, OwnedKind::Container, &by_project),
        (docker::Kind::Network, OwnedKind::Network, &by_project),
        (docker::Kind::Volume, OwnedKind::Volume, &by_project),
        (docker::Kind::Image, OwnedKind::Image, &by_workspace),
    ] {
        // An enumeration that fails proves nothing about what Armada owns, and
        // must not fail the row: the resource is labelled whether or not this
        // call answered, so the next `init` reaps it regardless.
        let Ok(found) = app.docker_list_by_selector(kind, selector) else {
            continue;
        };
        for reference in found {
            let _ = app.db.record_owned(&OwnedRow {
                workspace: workspace.id.clone(),
                kind: owned,
                reference: reference.clone(),
                boot_id: None,
                pid_started_at: None,
                // **Unattributed on purpose.** One `docker compose up` brings
                // the whole project up, so a container belongs to the stack
                // rather than to the component that happened to trigger it —
                // and claiming otherwise would have `down api` remove the
                // postgres container `db` is the component for.
                component: None,
            });
            owns.push(format!("{owned}:{reference}"));
        }
    }
    owns
}

/// Spawn a `command` service, detached, with its output in a real file.
#[allow(clippy::too_many_arguments)]
fn command_up<R: Run, C: Clock, F: Fetch>(
    app: &mut App<R, C, F>,
    workspace: &Workspace,
    config: &ResolvedConfig,
    service: &str,
    cmd: &str,
    shell: bool,
    common: &ResolvedService,
    assigned: &BTreeMap<String, u16>,
) -> Result<Attempt, Box<(Attempt, ArmadaError)>> {
    let at = ConfigWhere::Path {
        file: workspace.config_label.clone(),
        path: format!("components.{service}.run.cmd"),
    };
    let argv = expand(
        workspace,
        config,
        service,
        cmd,
        shell,
        assigned,
        &app.inherited,
        &at,
    )
    .map_err(|e| Box::new((Attempt::default(), e)))?;

    let mut env = app.child_env();
    for (key, value) in &common.env {
        let substituted = template::substitute(
            value,
            &vars(workspace, config, service, assigned, &app.inherited),
            Site::Argv,
            &at,
        )
        .map_err(|e| Box::new((Attempt::default(), e)))?;
        env.insert(key.clone(), substituted);
    }

    let log = fs::service_log(&workspace.root, service);
    let request = RunRequest::new(argv.clone(), workspace.root.clone())
        .env(env)
        // **A real file, not a pipe.** The service outlives this process, and
        // a captured stream would leave Armada holding the read end of a pipe
        // it is about to drop — the first write after that is `EPIPE`, which
        // kills the service moments after `up` reported it healthy.
        .stdio(StdioMode::Log(log));

    let attempt = |owns: Vec<String>| Attempt {
        owns,
        argv: argv.clone(),
        log: Some(fs::service_log_reference(service)),
        ready: Some(describe_ready(common, assigned)),
    };

    match process::ProcessGroup::spawn(&request) {
        Ok(group) => {
            let pgid = group.pgid();
            // **The handle is dropped and the child is not waited on**, which
            // is the one place that is correct: a service is meant to outlive
            // this process. `setsid` has already put it in its own session, so
            // it is reparented to init, which reaps it — the zombie rule
            // `docs/traps.md` states applies to children Armada keeps.
            drop(group);
            let owns = settle_pgid(app, workspace, service, Some(pgid));
            Ok(attempt(owns))
        }
        Err(spawn) => {
            settle_pgid(app, workspace, service, None);
            let error = match spawn.kind {
                // A service's own `cmd:` missing from `PATH` is the repo's
                // statement being wrong, not the machine's.
                SpawnErrorKind::NotFound | SpawnErrorKind::PermissionDenied => {
                    ArmadaError::bad_config(
                        at,
                        format!(
                            "`{}` could not be started: {}",
                            spawn.program, spawn.message
                        ),
                        format!("install {}, or correct `run.cmd`", spawn.program),
                    )
                }
                SpawnErrorKind::Other => ArmadaError {
                    class: ErrClass::Environment,
                    r#where: service.to_string(),
                    message: format!("cannot start `{}`: {}", spawn.program, spawn.message),
                    next_action: None,
                },
            };
            Err(Box::new((attempt(Vec::new()), error)))
        }
    }
}

/// Stop one service through its driver, and confirm it is gone.
fn stop<R: Run, C: Clock, F: Fetch>(
    app: &mut App<R, C, F>,
    workspace: &Workspace,
    config: &ResolvedConfig,
    service: &str,
    assigned: &BTreeMap<String, u16>,
    lease: &LeaseId,
) -> Event {
    let outcome = match run_of(config, service) {
        Some(ResolvedRun::Compose { file, .. }) => {
            let file = file.clone();
            compose_down(app, workspace, config, service, &file, assigned)
        }
        Some(ResolvedRun::Command { stop, shell, .. }) => {
            let (stop, shell) = (stop.clone(), *shell);
            command_down(
                app,
                workspace,
                config,
                service,
                stop.as_deref(),
                shell,
                assigned,
            )
        }
        None => Ok(Attempt::default()),
    };
    let _ = app.db.renew(lease, app.ctx.now.mono());

    match outcome {
        Ok(attempt) => Event::Stopped {
            service: service.to_string(),
            attempt,
        },
        Err(error) => Event::StopFailed {
            service: service.to_string(),
            error,
        },
    }
}

fn compose_down<R: Run, C: Clock, F: Fetch>(
    app: &mut App<R, C, F>,
    workspace: &Workspace,
    config: &ResolvedConfig,
    service: &str,
    files: &[String],
    assigned: &BTreeMap<String, u16>,
) -> Result<Attempt, ArmadaError> {
    app.docker_ready()?;

    let project = compose::project(workspace.id.as_str());
    let root = workspace.root.display().to_string();

    // The same document, for the same reason `up` generates one: `-f -` is how
    // compose is told which project this is without a file on disk.
    let document = call_docker(
        app,
        compose::resolve_argv(files, &project, &root),
        None,
        app.machine.docker_deadline(),
        ErrClass::Environment,
        service,
    )?;
    let names = compose::port_names(config, &workspace.config_label)?;
    let labels = docker::stamp(&workspace.id, &workspace.root, &app.namespace);
    let transformed = compose::transform(
        &document,
        &names,
        assigned,
        &labels,
        &workspace.config_label,
    )?;

    let argv = compose::down_argv(&project, &root);
    call_docker(
        app,
        argv.clone(),
        Some(transformed),
        app.machine.up_deadline(),
        ErrClass::Timeout,
        service,
    )?;

    // **The `owned` rows for the containers go, and the volumes stay.** `down`
    // removed the containers and the network; a named volume outlives it by
    // design, and dropping its row would leave `clean` nothing to release.
    for kind in [OwnedKind::Container, OwnedKind::Network] {
        let _ = app.db.clear_kind(&workspace.id, kind);
    }

    Ok(Attempt {
        argv,
        ..Attempt::default()
    })
}

fn command_down<R: Run, C: Clock, F: Fetch>(
    app: &mut App<R, C, F>,
    workspace: &Workspace,
    config: &ResolvedConfig,
    service: &str,
    declared_stop: Option<&str>,
    shell: bool,
    assigned: &BTreeMap<String, u16>,
) -> Result<Attempt, ArmadaError> {
    let groups = owned_pgids(app, workspace, service)?;

    // A repo's own `stop:` gets to go first — `pumactl stop` knows how to end a
    // Puma cleanly and Armada does not. It is not trusted to have worked:
    // the group is confirmed gone afterwards either way.
    let mut argv = Vec::new();
    if let Some(declared) = declared_stop {
        let at = ConfigWhere::Path {
            file: workspace.config_label.clone(),
            path: format!("components.{service}.run.stop"),
        };
        argv = expand(
            workspace,
            config,
            service,
            declared,
            shell,
            assigned,
            &app.inherited,
            &at,
        )?;
        let request = RunRequest::new(argv.clone(), workspace.root.clone())
            .env(app.child_env())
            .timeout(app.machine.up_deadline());
        let _ = app.ctx.run.call(&request);
    }

    // **SIGTERM, grace, then SIGKILL — unconditional, not a retry.** A leader
    // running `trap '' TERM` immunises its whole group, because children
    // inherit an ignored disposition across `fork` and `exec`, and a second
    // SIGTERM achieves exactly what the first did (`docs/traps.md`).
    let mut survived = Vec::new();
    for pgid in groups {
        let report = posix::stop_group(pgid, process::GRACE);
        app.renew_held();
        if report.existed && !report.gone {
            survived.push(pgid);
        }
        let _ = app
            .db
            .delete_owned(&workspace.id, OwnedKind::Pgid, &pgid.to_string());
    }
    let _ = app.db.delete_owned(
        &workspace.id,
        OwnedKind::Pgid,
        &format!("{PENDING}{service}"),
    );

    if survived.is_empty() {
        return Ok(Attempt {
            argv,
            ..Attempt::default()
        });
    }
    Err(ArmadaError {
        class: ErrClass::ToolFailed,
        r#where: service.to_string(),
        message: format!(
            "process group {} survived SIGKILL",
            survived
                .iter()
                .map(i32::to_string)
                .collect::<Vec<_>>()
                .join(", ")
        ),
        next_action: Some(
            "the group is uninterruptible or unreachable; `armada manifest status` re-probes the port"
                .to_string(),
        ),
    })
}

/// The process groups **this component** owns, as numbers Armada may signal.
///
/// **Scoped to the component, and that is not a refinement.** The rows are
/// workspace-scoped, so an unscoped read would have `armada manifest down api`
/// stop the `postgres` group that `db` owns — over-reach the "pause, not
/// release" contract forbids, and the reason `owned.component` exists at all.
///
/// **A pgid of zero is not a pgid**: `killpg(0, …)` signals the caller's own
/// group, so a `0` reaching here would have `down` SIGTERM and then SIGKILL
/// Armada itself and everything sharing its foreground group.
///
/// A row with no component was written before that was recorded. It is left
/// alone here and reclaimed by `clean`, which takes everything: acting on it
/// would be guessing, and guessing wrong stops a live service.
fn owned_pgids<R: Run, C: Clock, F: Fetch>(
    app: &mut App<R, C, F>,
    workspace: &Workspace,
    service: &str,
) -> Result<Vec<i32>, ArmadaError> {
    Ok(app
        .db
        .owned(Some(&workspace.id))?
        .into_iter()
        .filter(|row| row.kind == OwnedKind::Pgid)
        .filter(|row| row.component.as_deref() == Some(service))
        .filter_map(|row| row.reference.parse::<i32>().ok())
        .filter(|pgid| *pgid > 0)
        .collect())
}

// --------------------------------------------------------------- ready-checks

/// Ask one service's ready-check, once.
fn probe<R: Run, C: Clock, F: Fetch>(
    app: &mut App<R, C, F>,
    workspace: &Workspace,
    config: &ResolvedConfig,
    service: &str,
    assigned: &BTreeMap<String, u16>,
) -> Event {
    let Some(run) = run_of(config, service) else {
        return Event::Ready {
            service: service.to_string(),
        };
    };
    let ready = run.common().ready.clone();
    let at = ConfigWhere::Path {
        file: workspace.config_label.clone(),
        path: format!("components.{service}.run.ready"),
    };
    // A ready-check's own deadline bounds the probe as well as the wait: a
    // `connect()` to a host that blackholes packets takes the kernel's minute
    // otherwise, and that minute is spent inside one turn of a loop that is
    // supposed to be checking a deadline.
    let per_probe = Duration::from_secs(2);

    let answer: Result<bool, ArmadaError> = match &ready.kind {
        ReadyKind::None => Ok(true),
        ReadyKind::Tcp(name) => match assigned.get(name) {
            Some(port) => app.ctx.fetch.tcp_connect("localhost", *port, per_probe),
            None => Err(ArmadaError::bad_config(
                at.clone(),
                format!("`ready: {{tcp: {name}}}` names no declared port"),
                format!("declare `ports: {{ {name}: <port> }}` on this component"),
            )),
        },
        ReadyKind::Http(url) => {
            match template::substitute(
                url,
                &vars(workspace, config, service, assigned, &app.inherited),
                Site::Argv,
                &at,
            ) {
                Ok(url) => match app.ctx.fetch.http_status(&url, per_probe) {
                    // **2xx and nothing else.** A connection refused is a
                    // service still starting, so it is `NotReady` rather than a
                    // failure — the deadline is what ends the wait.
                    Ok(status) => Ok((200..300).contains(&status)),
                    Err(_) => Ok(false),
                },
                Err(error) => Err(error),
            }
        }
        ReadyKind::Exec(cmd) => {
            match expand(
                workspace,
                config,
                service,
                cmd,
                false,
                assigned,
                &app.inherited,
                &at,
            ) {
                Ok(argv) => {
                    let request = RunRequest::new(argv, workspace.root.clone())
                        .env(app.child_env())
                        .timeout(per_probe);
                    match app.ctx.run.call(&request) {
                        Ok(output) => Ok(output.ok()),
                        // A probe command that is not on `PATH` will never be,
                        // so waiting out the deadline learns nothing.
                        Err(spawn) => Err(ArmadaError::bad_config(
                            at.clone(),
                            format!(
                                "`ready.exec` cannot run `{}`: {}",
                                spawn.program, spawn.message
                            ),
                            format!("install {}, or correct `ready.exec`", spawn.program),
                        )),
                    }
                }
                Err(error) => Err(error),
            }
        }
        ReadyKind::Log(pattern) => {
            let text = fs::read_service_log(&workspace.root, service);
            lifecycle::log_matched(&text, pattern, at.clone())
        }
    };

    match answer {
        Ok(true) => Event::Ready {
            service: service.to_string(),
        },
        Ok(false) => Event::NotReady {
            service: service.to_string(),
        },
        Err(error) => Event::ReadyFailed {
            service: service.to_string(),
            error,
        },
    }
}

/// The ready-check in words, for `results[].reason`.
fn describe_ready(common: &ResolvedService, assigned: &BTreeMap<String, u16>) -> String {
    match &common.ready.kind {
        ReadyKind::None => "ready on spawn".to_string(),
        ReadyKind::Tcp(name) => match assigned.get(name) {
            Some(port) => format!("tcp {name} ({port})"),
            None => format!("tcp {name}"),
        },
        ReadyKind::Http(url) => format!("http {url}"),
        ReadyKind::Exec(cmd) => format!("exec {cmd}"),
        ReadyKind::Log(pattern) => format!("log /{pattern}/"),
    }
}

// ----------------------------------------------------------------- the shared
// small parts

fn run_of<'a>(config: &'a ResolvedConfig, service: &str) -> Option<&'a ResolvedRun> {
    config.components.get(service)?.run.as_ref()
}

fn needs_map(config: &ResolvedConfig) -> BTreeMap<String, Vec<String>> {
    config
        .components
        .iter()
        .filter_map(|(name, component)| {
            let run = component.run.as_ref()?;
            let needs = run
                .common()
                .needs
                .iter()
                .filter_map(|need| match need {
                    armada_core::config::Need::Component(target) => Some(target.clone()),
                    armada_core::config::Need::Check(_) => None,
                })
                .collect();
            Some((name.clone(), needs))
        })
        .collect()
}

fn ready_timeouts(config: &ResolvedConfig) -> BTreeMap<String, u32> {
    config
        .components
        .iter()
        .filter_map(|(name, component)| {
            let run = component.run.as_ref()?;
            Some((name.clone(), run.common().ready.timeout))
        })
        .collect()
}

fn vars<'a>(
    workspace: &'a Workspace,
    config: &'a ResolvedConfig,
    service: &str,
    assigned: &'a BTreeMap<String, u16>,
    inherited: &'a BTreeMap<String, String>,
) -> Vars<'a> {
    Vars {
        workspace_id: workspace.id.as_str(),
        ports: assigned,
        component_root: config
            .components
            .get(service)
            .and_then(|component| component.root.as_deref()),
        files: None,
        env: inherited,
    }
}

#[allow(clippy::too_many_arguments)]
fn expand(
    workspace: &Workspace,
    config: &ResolvedConfig,
    service: &str,
    cmd: &str,
    shell: bool,
    assigned: &BTreeMap<String, u16>,
    inherited: &BTreeMap<String, String>,
    at: &ConfigWhere,
) -> Result<Vec<String>, ArmadaError> {
    let vars = vars(workspace, config, service, assigned, inherited);
    match shell {
        true => template::shell_argv(cmd, &vars, at),
        false => template::expand_argv(cmd, &vars, at),
    }
}

/// One docker call, with the held leases renewed from inside it.
fn call_docker<R: Run, C: Clock, F: Fetch>(
    app: &mut App<R, C, F>,
    argv: Vec<String>,
    stdin: Option<String>,
    timeout: Duration,
    on_timeout: ErrClass,
    service: &str,
) -> Result<String, ArmadaError> {
    let mut request = RunRequest::new(argv.clone(), app.cwd()).timeout(timeout);
    if let Some(text) = stdin {
        request = request.with_stdin(text);
    }

    let held = app.held.clone();
    let output = {
        let db = &mut app.db;
        let clock = &app.ctx.now;
        app.ctx.run.call_with_tick(&request, &mut || {
            let stamp = clock.mono();
            for lease in &held {
                let _ = db.renew(lease, stamp);
            }
        })
    };
    app.renew_held();

    let output = output.map_err(|e| ArmadaError {
        class: ErrClass::Environment,
        r#where: "docker".to_string(),
        message: match e.kind {
            SpawnErrorKind::NotFound => "docker is not on PATH".to_string(),
            _ => format!("cannot run docker: {}", e.message),
        },
        next_action: None,
    })?;

    if output.timed_out {
        return Err(ArmadaError {
            class: on_timeout,
            r#where: service.to_string(),
            message: format!("`{}` exceeded {}s", argv.join(" "), timeout.as_secs()),
            next_action: Some("raise `up_timeout` in ~/.armada/machine.yml".to_string()),
        });
    }
    if !output.ok() {
        return Err(ArmadaError {
            class: ErrClass::ToolFailed,
            r#where: service.to_string(),
            message: format!("`{}` failed: {}", argv.join(" "), last_line(&output.stderr)),
            next_action: None,
        });
    }
    Ok(output.stdout)
}

/// The last line a tool said, which is where the reason is.
fn last_line(text: &str) -> String {
    text.lines()
        .map(str::trim)
        .rfind(|line| !line.is_empty())
        .unwrap_or_default()
        .to_string()
}

/// A service's declared ports, probed **at report time and never remembered**.
fn probed_ports(
    config: &ResolvedConfig,
    service: &str,
    assigned: &BTreeMap<String, u16>,
) -> BTreeMap<String, armada_core::envelope::PortReport> {
    let Some(run) = run_of(config, service) else {
        return BTreeMap::new();
    };
    run.common()
        .ports
        .keys()
        .filter_map(|name| {
            let port = *assigned.get(name)?;
            Some((
                name.clone(),
                armada_core::envelope::PortReport {
                    port,
                    // `LISTENING` when something answers a bind probe, which
                    // after `up` is the service Armada started and after `down`
                    // is a service that did not stop — the case `docs/traps.md`
                    // says is detected rather than prevented.
                    state: if net::port_is_taken(port) {
                        armada_core::ports::PortState::Listening
                    } else {
                        armada_core::ports::PortState::Reserved
                    },
                },
            ))
        })
        .collect()
}

// ------------------------------------------------------------------ --dry-run

/// What `up` would run, and what it would then wait on. **Nothing is changed**,
/// and it takes no lease, because it reads.
fn dry<R: Run, C: Clock, F: Fetch>(
    app: &App<R, C, F>,
    workspace: &Workspace,
    config: &ResolvedConfig,
    order: &[String],
    assigned: &BTreeMap<String, u16>,
) -> Result<UpDryRun, ArmadaError> {
    let project = compose::project(workspace.id.as_str());
    let root = workspace.root.display().to_string();

    let mut would_run = Vec::new();
    let mut would_wait = Vec::new();
    for service in order {
        let Some(run) = run_of(config, service) else {
            continue;
        };
        let argv = match run {
            ResolvedRun::Compose { file, .. } => compose::up_argv(&project, &root)
                .into_iter()
                .chain(std::iter::once(format!("# from {}", file.join(", "))))
                .collect::<Vec<_>>(),
            ResolvedRun::Command { cmd, shell, .. } => expand(
                workspace,
                config,
                service,
                cmd,
                *shell,
                assigned,
                &app.inherited,
                &ConfigWhere::Path {
                    file: workspace.config_label.clone(),
                    path: format!("components.{service}.run.cmd"),
                },
            )?,
        };
        would_run.push(format!("{service}: {}", argv.join(" ")));
        let common = run.common();
        would_wait.push(format!(
            "{service}: {} ({}s)",
            describe_ready(common, assigned),
            common.ready.timeout
        ));
    }
    Ok(UpDryRun {
        would_run,
        would_wait,
    })
}
