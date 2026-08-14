//! `commands:` — repo-local verbs Armada does not own (PLAN.md §4.5).
//!
//! The six verbs are universal. Every repo also has commands that are **only**
//! meaningful in that repo, and Armada must not swallow them or force them
//! elsewhere. Armada is a dispatcher here and nothing more:
//!
//! - **remaining argv passes through untouched** — `Armada worktrees prune
//!   --dry-run` runs `… prune --dry-run`, and that `--dry-run` is the child's
//!   even though Armada defines a flag by that name;
//! - **the child's exit code is returned verbatim**, not mapped into Armada's own
//!   codes. Armada did not decide the outcome, so it does not get to classify it,
//!   and scripts return meaningful codes their own callers already depend on;
//! - **`env:` is additive** — the parent environment is inherited wholesale and
//!   these are layered on top, so a command needing `$HOME` already has it.
//!
//! The collision with Armada's own `1`–`6` is resolved by the envelope rather
//! than by renumbering: **Armada's own error codes can only occur when the child
//! never ran**, and `data.dispatched` says which happened.

use armada_core::ctx::{Clock, Fetch, Run, RunRequest, StdioMode};
use armada_core::envelope::{DispatchData, Envelope};
use armada_core::error::{ArmadaError, ConfigWhere, ErrClass, Status};
use armada_core::lease::{LeaseId, Policy};
use armada_core::template::{self, Site, Vars};
use std::collections::BTreeMap;
use std::io::Write;

use crate::app::{self, App};
use crate::verbs::{load_config, Output};

/// Run it.
pub fn run<R: Run, C: Clock, F: Fetch>(
    app: &mut App<R, C, F>,
    name: &str,
    passthrough: &[String],
    json: bool,
) -> Result<Output, ArmadaError> {
    let (workspace, config) = load_config(app)?;

    let Some(entry) = config.commands.get(name).cloned() else {
        let mut available: Vec<&str> = config.commands.keys().map(String::as_str).collect();
        available.sort();
        return Err(ArmadaError {
            class: ErrClass::BadInvocation,
            r#where: name.to_string(),
            message: format!(
                "`armada manifest {name}` is not a verb and this repo declares no such command"
            ),
            next_action: Some(if available.is_empty() {
                "this repo declares no commands: entries; `armada --help` lists the built-ins"
                    .to_string()
            } else {
                format!("declared commands: {}", available.join(", "))
            }),
        });
    };

    let block = app
        .db
        .workspaces()?
        .into_iter()
        .find(|row| row.id == workspace.id)
        .map(|row| row.ports);
    let ports = match block {
        Some(block) => armada_core::ports::assign_ports(&config, block, &workspace.config_label)?,
        // No block means `armada manifest init` has not run here. `${port.NAME}` then has
        // no answer, and substitution says so by name rather than expanding to
        // something plausible.
        None => BTreeMap::new(),
    };

    let at = ConfigWhere::Path {
        file: workspace.config_label.clone(),
        path: format!("commands.{name}.cmd"),
    };
    let vars = Vars {
        workspace_id: workspace.id.as_str(),
        ports: &ports,
        component_root: None,
        // `${files}` in a `commands:` entry is `bad_config` rather than an
        // empty expansion: a command that quietly checks nothing is the exact
        // failure the empty-set rule exists to prevent, and a `commands:` entry
        // runs ad hoc so there is no scope to compute.
        files: None,
        env: &app.inherited,
    };

    let mut argv = if entry.shell {
        // Under a shell the passthrough has to be quoted, because it is the
        // *caller's* text: concatenating it raw would let
        // `Armada worktrees ';rm -rf /'` mean something the repo never wrote.
        let mut line = template::substitute(&entry.cmd, &vars, Site::Shell, &at)?;
        for arg in passthrough {
            line.push(' ');
            line.push_str(&template::shell_quote(arg));
        }
        vec!["/bin/sh".to_string(), "-c".to_string(), line]
    } else {
        template::expand_argv(&entry.cmd, &vars, &at)?
    };
    if !entry.shell {
        argv.extend(passthrough.iter().cloned());
    }

    let mut env = app.child_env();
    for (key, value) in &entry.env {
        let at = ConfigWhere::Path {
            file: workspace.config_label.clone(),
            path: format!("commands.{name}.env.{key}"),
        };
        env.insert(
            key.clone(),
            template::substitute(value, &vars, Site::EnvValue, &at)?,
        );
    }

    // **`--json` overrides `stdio:` and forces `pipe`.** With `inherit` the
    // child writes to Armada's own stdout and Armada then writes the envelope to
    // the same descriptor, so the one consumer the envelope exists for receives
    // interleaved child output and JSON.
    let stdio = if json || entry.stdio == armada_core::config::Stdio::Pipe {
        StdioMode::Capture
    } else {
        StdioMode::Inherit
    };

    let request = RunRequest::new(argv.clone(), workspace.root.clone())
        .env(env)
        .stdio(stdio.clone());
    // No timeout: how long a repo's own command takes is the repo's business,
    // and Armada's deadlines exist for the calls Armada makes on its own account.
    //
    // **Known limit, stated rather than hidden.** The child runs in its own
    // session, so Armada can reach its whole tree — but no `owned` pgid row is
    // written for it, because the dispatch is synchronous and Armada waits. If
    // Armada itself is SIGKILLed mid-dispatch, that group survives with no record
    // and only the port probe can find it. Recording it would mean the `Run`
    // seam reporting a pgid back, which is the extension `armada manifest up` needs in
    // phase 4 anyway; doing it there once beats doing it twice differently.

    let lease = LeaseId::run(workspace.id.clone());
    let outcome = app::with_lease(app, lease.clone(), Policy::FailFast, None, |app| {
        let db = &mut app.db;
        let clock = &app.ctx.now;
        Ok(app.ctx.run.call_with_tick(&request, &mut || {
            let _ = db.renew(&lease, clock.mono());
        }))
    })?;

    match outcome {
        Ok(output) => {
            if stdio == StdioMode::Capture {
                // Captured output still has to go somewhere, and **which
                // descriptor depends on why it was captured**. Under `--json`
                // both streams go to stderr, because stdout carries the
                // envelope and nothing else. A `stdio: pipe` entry without
                // `--json` was captured only so Armada could hold it, not so Armada
                // could move it: sending its stdout to Armada's stderr would make
                // `Armada mycmd > out.txt` capture nothing, which is the opposite
                // of what redirecting a command means.
                let mut err = std::io::stderr();
                if json {
                    let _ = err.write_all(output.stdout.as_bytes());
                } else {
                    let mut out = std::io::stdout();
                    let _ = out.write_all(output.stdout.as_bytes());
                    let _ = out.flush();
                }
                let _ = err.write_all(output.stderr.as_bytes());
                let _ = err.flush();
            }
            Ok(Output::Dispatch(Box::new(Envelope::ok(
                "commands",
                Some(workspace.id.clone()),
                if output.code == Some(0) {
                    Status::Ok
                } else {
                    Status::Failed
                },
                DispatchData {
                    command: name.to_string(),
                    dispatched: true,
                    child_exit: output.code,
                    argv,
                },
            ))))
        }
        // The child never ran, so this is Armada's failure to report and Armada's
        // code to exit with — which is exactly the case `data.dispatched: false`
        // marks. A declared command that is not on `PATH` is the repo's
        // statement being wrong, so it is `bad_config` rather than
        // `environment`.
        Err(spawn) => Ok(Output::Dispatch(Box::new(Envelope::failed(
            "commands",
            Some(workspace.id.clone()),
            ArmadaError::bad_config(
                at,
                format!(
                    "`{}` could not be started: {}",
                    spawn.program, spawn.message
                ),
                format!(
                    "install {}, or correct `commands.{name}.cmd`",
                    spawn.program
                ),
            ),
            DispatchData {
                command: name.to_string(),
                dispatched: false,
                child_exit: None,
                argv,
            },
        )))),
    }
}
