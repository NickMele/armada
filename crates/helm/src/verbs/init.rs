//! `armada manifest init` — make this workspace ready.
//!
//! **Exactly one thing.** An earlier draft also gave it §5's layer-1 evidence
//! scan, which by definition runs where no `armada.yml` exists — so the verb had
//! two unrelated behaviours, two output shapes, and could only fail in the state
//! half of it existed to serve. That scan is `armada manifest config scan`.
//!
//! The order is **reap, then claim**, and it is the whole answer to the plan's
//! one piece of empirical evidence: a sweep function that existed and was never
//! called. `init` is the right primary hook because it is where the outage
//! actually originated — repeated worktree create/destroy always runs `init` in
//! the new one — it already holds the database open to claim a port block, and
//! it is infrequent enough that a docker call costs nothing noticeable.

use armada_core::config::ResolvedConfig;
use armada_core::ctx::{Clock, Fetch, Run, RunRequest, StdioMode};
use armada_core::envelope::{aggregate, Envelope, InitData, InitDryRun, PortReport, ResultRow};
use armada_core::error::{ArmadaError, ConfigWhere, ErrClass, Status};
use armada_core::lease::{LeaseId, Policy};
use armada_core::ports::{self, PortBlock, PortState};
use armada_core::registry::{OwnedKind, OwnedRow};
use armada_core::template::{self, Site, Vars};
use armada_core::workspace::Workspace;
use armada_manifest::db::ClaimOutcome;
use armada_manifest::{fs, net};
use std::collections::BTreeMap;

use crate::app::{self, App};
use crate::verbs::{load_config, Output};

/// How many times a lost port-block claim is re-decided before Armada gives up.
///
/// Losing means another workspace took the block Armada chose between the read
/// and the write, so the answer has changed and re-deciding is correct. A bound
/// exists because an unbounded retry against a pathological machine is a spin.
const CLAIM_ATTEMPTS: usize = 16;

/// Run it.
pub fn run<R: Run, C: Clock, F: Fetch>(
    app: &mut App<R, C, F>,
    dry_run: bool,
) -> Result<Output, ArmadaError> {
    let (workspace, config) = load_config(app)?;

    if dry_run {
        return dry(app, &workspace, &config).map(|e| Output::InitDryRun(Box::new(e)));
    }

    let lease = LeaseId::run(workspace.id.clone());
    let envelope = app::with_lease(app, lease.clone(), Policy::FailFast, None, |app| {
        claim_and_prepare(app, &workspace, &config, &lease)
    })?;

    Ok(Output::Init(Box::new(envelope)))
}

fn claim_and_prepare<R: Run, C: Clock, F: Fetch>(
    app: &mut App<R, C, F>,
    workspace: &Workspace,
    config: &ResolvedConfig,
    lease: &LeaseId,
) -> Result<Envelope<InitData>, ArmadaError> {
    let reaped = app.reap()?;

    let claimed_at = app.ctx.now.wall_rfc3339();
    // **A block only for a workspace that has something to collide over.** The
    // span exists so parallel worktrees do not fight over a service's port
    // (PLAN.md §2.2); a repository that declares no `ports:` has no service, so
    // a block reserved for it reserves nothing — and takes ten ports of a finite
    // pool from a workspace that does need them. The workspace is registered
    // either way: that row is what makes it reclaimable once its directory is
    // gone, which has nothing to do with ports.
    let block = claim(app, workspace, config, &claimed_at)?;
    let ports = match block {
        Some(block) => ports::assign_ports(config, block, &workspace.config_label)?,
        None => BTreeMap::new(),
    };

    fs::create_armada_dir(&workspace.root)?;
    record_release_commands(app, workspace, config, &ports)?;

    let results = run_setup(app, workspace, config, &ports, lease)?;

    // Through `aggregate`, not by finding the first failed row: a `setup:`
    // command that is not on `PATH` fails `bad_config`, and picking the first
    // failure while hardcoding `tool_failed` reported exit 1 — "that is a real
    // result, report it" — for a config the caller has to edit before anything
    // else means anything. The precedence rule exists so that two
    // implementations cannot disagree, which is only true when there is one.
    let error = aggregate(&results, "components");

    let data = InitData {
        port_block: block,
        claimed_at,
        ports,
        reaped,
        results,
    };

    Ok(match error {
        Some(error) => Envelope::failed("init", Some(workspace.id.clone()), error, data),
        None => Envelope::ok("init", Some(workspace.id.clone()), Status::Ready, data),
    })
}

/// Register the workspace, claiming a block if it needs one, and re-deciding
/// when a concurrent claimant wins the race.
fn claim<R: Run, C: Clock, F: Fetch>(
    app: &mut App<R, C, F>,
    workspace: &Workspace,
    config: &ResolvedConfig,
    claimed_at: &str,
) -> Result<Option<PortBlock>, ArmadaError> {
    // **The size is what says whether a block is wanted at all.** `None` still
    // writes the registry row; it just reserves nothing.
    let size = ports::needs_block(config).then_some(app.machine.port_block_size);
    for _ in 0..CLAIM_ATTEMPTS {
        let outcome = app.db.claim_block(
            &workspace.id,
            &workspace.root,
            workspace.project.as_ref(),
            size,
            app.machine.port_base,
            claimed_at,
        )?;
        match outcome {
            ClaimOutcome::Claimed(block) | ClaimOutcome::AlreadyHeld(block) => return Ok(block),
            ClaimOutcome::Lost => continue,
            ClaimOutcome::Exhausted => {
                return Err(ArmadaError {
                    class: ErrClass::Environment,
                    r#where: "ports".to_string(),
                    // **The machine's own floor, not the default one.** A
                    // report naming 5460 on a machine configured to start at
                    // 21000 sends the reader to look at the wrong range, and
                    // this message exists to say which range ran out.
                    message: format!(
                        "no free block of {} ports between {} and {}",
                        app.machine.port_block_size, app.machine.port_base, ports::PORT_CEILING
                    ),
                    next_action: Some(
                        "`armada manifest clean --all --orphaned` releases blocks whose workspaces are gone, \
                         or lower `port_base` in ~/.armada/machine.yml"
                            .to_string(),
                    ),
                });
            }
        }
    }
    Err(ArmadaError {
        class: ErrClass::Aborted,
        r#where: "ports".to_string(),
        message: format!("lost the port-block race {CLAIM_ATTEMPTS} times"),
        next_action: Some("retry; another Armada is claiming blocks right now".to_string()),
    })
}

/// Record every declared `owns.release:`, resolved.
///
/// **Recorded at `init` rather than read from the repo at `clean` time**,
/// because a teardown script symmetric with `setup:` would live *in the
/// workspace* — so in the orphan case, the one that actually matters, it has
/// been deleted along with everything else. A resolved command string in the
/// machine-global store runs from anywhere. It is **reported, never executed**:
/// a stale `DROP DATABASE` is strictly more dangerous than a stale `kill`.
fn record_release_commands<R: Run, C: Clock, F: Fetch>(
    app: &mut App<R, C, F>,
    workspace: &Workspace,
    config: &ResolvedConfig,
    ports: &BTreeMap<String, u16>,
) -> Result<(), ArmadaError> {
    // Cleared first so a declaration removed from `armada.yml` stops being
    // reported, rather than accumulating every version the repo ever had.
    app.db.clear_kind(&workspace.id, OwnedKind::Release)?;

    let vars = Vars::new(workspace.id.as_str(), ports, &app.inherited);
    let mut declared: Vec<String> = Vec::new();

    for (name, component) in &config.components {
        let at = ConfigWhere::Path {
            file: workspace.config_label.clone(),
            path: format!("components.{name}.owns.release"),
        };
        for command in &component.owns_release {
            declared.push(template::substitute(command, &vars, Site::Argv, &at)?);
        }
        if let Some(run) = &component.run {
            for command in &run.common().owns_release {
                declared.push(template::substitute(command, &vars, Site::Argv, &at)?);
            }
        }
    }
    for (name, entry) in &config.commands {
        let at = ConfigWhere::Path {
            file: workspace.config_label.clone(),
            path: format!("commands.{name}.owns.release"),
        };
        for command in &entry.owns_release {
            declared.push(template::substitute(command, &vars, Site::Argv, &at)?);
        }
    }

    for command in declared {
        app.db.record_owned(&OwnedRow {
            workspace: workspace.id.clone(),
            kind: OwnedKind::Release,
            reference: command,
            boot_id: None,
            pid_started_at: None,
            // A `release:` command is reported and never executed, so nothing
            // ever selects one by component.
            component: None,
        })?;
    }
    Ok(())
}

/// Run each component's `setup:`, in declaration order within a component.
///
/// **Idempotent in Armada's own state, not in the repo's** — `armada manifest init` twice
/// claims one block and creates one `.armada/`, but it runs `setup:` again,
/// because whether `bundle install` is cheap the second time is the repo's
/// business and Armada has no way to know.
fn run_setup<R: Run, C: Clock, F: Fetch>(
    app: &mut App<R, C, F>,
    workspace: &Workspace,
    config: &ResolvedConfig,
    ports: &BTreeMap<String, u16>,
    lease: &LeaseId,
) -> Result<Vec<ResultRow>, ArmadaError> {
    let mut results = Vec::new();

    for (name, component) in &config.components {
        let mut row = ResultRow::new(name.clone(), Status::Ready);
        row.ports = component
            .run
            .as_ref()
            .map(|run| {
                run.common()
                    .ports
                    .keys()
                    .filter_map(|port_name| {
                        ports.get(port_name).map(|port| {
                            (
                                port_name.clone(),
                                PortReport {
                                    port: *port,
                                    // Probed at report time, on both families:
                                    // an IPv6-only listener is invisible to an
                                    // IPv4 probe, and `localhost` resolving to
                                    // `::1` is what modern Node does.
                                    state: if net::port_is_taken(*port) {
                                        PortState::Conflict
                                    } else {
                                        PortState::Reserved
                                    },
                                },
                            )
                        })
                    })
                    .collect()
            })
            .unwrap_or_default();

        if component.setup.is_empty() {
            row.status = Status::Skipped;
            results.push(row);
            continue;
        }

        let started = app.ctx.now.mono();
        for (index, step) in component.setup.iter().enumerate() {
            let at = ConfigWhere::Path {
                file: workspace.config_label.clone(),
                path: format!("components.{name}.setup[{index}]"),
            };
            let vars = Vars {
                workspace_id: workspace.id.as_str(),
                ports,
                component_root: component.root.as_deref(),
                files: None,
                env: &app.inherited,
            };
            let argv = if step.shell {
                template::shell_argv(&step.cmd, &vars, &at)?
            } else {
                template::expand_argv(&step.cmd, &vars, &at)?
            };

            let request = RunRequest::new(argv, workspace.root.clone())
                .env(app.child_env())
                .stdio(StdioMode::Capture);

            let lease = lease.clone();
            let outcome = {
                // The heartbeat renews from the loop that waits, never from a
                // timer: `bundle install` runs for minutes and a lease goes
                // cold in one, so without this a second agent takes the run
                // lease out from under a healthy init.
                let db = &mut app.db;
                let clock = &app.ctx.now;
                app.ctx.run.call_with_tick(&request, &mut || {
                    let _ = db.renew(&lease, clock.mono());
                })
            };

            match outcome {
                Ok(output) if output.ok() => {}
                Ok(output) => {
                    row.status = Status::Failed;
                    row.error = Some(ArmadaError {
                        class: ErrClass::ToolFailed,
                        r#where: name.clone(),
                        message: format!(
                            "`{}` exited {}",
                            step.cmd,
                            output
                                .code
                                .map(|c| c.to_string())
                                .unwrap_or("on a signal".into())
                        ),
                        next_action: None,
                    });
                    break;
                }
                Err(spawn) => {
                    row.status = Status::Failed;
                    // A `setup:` command that is not on `PATH` is the repo's
                    // statement being wrong, not the machine's — which is why
                    // the seam returns a typed spawn failure and lets each
                    // caller classify it.
                    row.error = Some(ArmadaError::bad_config(
                        at.clone(),
                        format!(
                            "`{}` could not be started: {}",
                            spawn.program, spawn.message
                        ),
                        format!("install {} , or correct the setup step", spawn.program),
                    ));
                    break;
                }
            }
        }
        row.duration_ms = Some(app.ctx.now.mono().saturating_sub(started));
        results.push(row);
    }

    Ok(results)
}

/// `--dry-run`: the ordinary envelope with `would_*` in place of `results[]`,
/// and nothing changed. It takes **no lease**, because it reads.
fn dry<R: Run, C: Clock, F: Fetch>(
    app: &mut App<R, C, F>,
    workspace: &Workspace,
    config: &ResolvedConfig,
) -> Result<Envelope<InitDryRun>, ArmadaError> {
    let rows = app.db.workspaces()?;
    let held = rows.iter().find(|row| row.id == workspace.id);
    let taken: Vec<PortBlock> = rows.iter().filter_map(|row| row.ports).collect();

    // **The preview says `none` for a workspace that needs none**, because that
    // is what the real pass would do — a preview that showed a range the run
    // would not claim is the one thing `--dry-run` must never do.
    let would_claim = match (held, ports::needs_block(config)) {
        (_, false) => None,
        (Some(row), true) => row.ports,
        (None, true) => {
            ports::choose_block(&taken, app.machine.port_block_size, app.machine.port_base)
        }
    };

    // Decided, not executed: listing and stat-ing change nothing, so the
    // preview is exactly the decision the real pass would make.
    let would_reap = app.plan_reap()?;

    let would_run = config
        .components
        .iter()
        .flat_map(|(name, component)| {
            component
                .setup
                .iter()
                .map(move |step| format!("{name}: {}", step.cmd))
        })
        .collect();

    Ok(Envelope::ok(
        "init",
        Some(workspace.id.clone()),
        Status::Ready,
        InitDryRun {
            would_claim,
            would_run,
            would_reap,
        },
    ))
}
