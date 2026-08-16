//! `armada manifest check` — lint, format, test. Scoped, scheduled, leased, ceilinged.
//!
//! **This file is the shell, and it decides nothing.** The scheduler is a
//! reducer (`ARCHITECTURE.md` §1.2): the core proposes actions, this performs
//! them, and what happens comes back as events. Every branch below is either
//! "carry out what was proposed" or "report what was observed" — if a decision
//! appears here it belongs in [`armada_core::schedule`] instead, where a unit
//! test can reach it.
//!
//! ```text
//! Tick ──▶ step ──▶ Acquire ──▶ (manifest.db)   ──▶ LeaseGranted / LeaseDenied
//!                   Spawn    ──▶ (a child)  ──▶ ChildSpawned / SpawnFailed
//!                   Kill     ──▶ (killpg)   ──▶ ChildExited
//!                   Emit / Finish
//! ```
//!
//! **The loop never blocks**, which is why [`ProcessGroup::poll`] exists — and
//! why [`acquire`] is one attempt rather than a wait. Both are the same rule:
//! deadlines, child exits and the interrupt are all observed on this one
//! thread, so anything that sits inside a single action stops all three. An
//! acquisition that waited here stopped them for up to `acquire_timeout`.
//!
//! The lease heartbeat is renewed from this loop and from no background timer,
//! precisely so that a wedged loop is a loop that stopped renewing and the
//! cold-heartbeat path reclaims it (PLAN.md §4.3).

use armada_core::config::{ResolvedCheck, ResolvedComponent, ResolvedConfig, Scope};
use armada_core::ctx::{Clock, Fetch, Run, RunRequest, SpawnErrorKind, StdioMode};
use armada_core::dispatch::{Dispatch, Journal, Scrub};
use armada_core::envelope::{CheckData, CheckDryRun, DetachedView, Envelope};
use armada_core::error::{ArmadaError, ConfigWhere, ErrClass, Status};
use armada_core::lease::{self, LeaseId, LeaseKind, Policy};
use armada_core::reap::PathStat;
use armada_core::registry::{OwnedKind, OwnedRow};
use armada_core::run::{Detached, RunId, RunRecord};
use armada_core::schedule::{
    self, Action, CheckId, CheckResult, EnvDelta, Event, Phase, Plan, State, Waiting,
};
use armada_core::select::{self, Selection, Selector};
use armada_core::template::{self, Site, Vars};
use armada_core::workspace::Workspace;
use armada_manifest::process::ProcessGroup;
use armada_manifest::{fs, git, runs};
use std::collections::BTreeMap;
use std::path::PathBuf;

use crate::app::{self, App, Claim};
use crate::args::Check;
use crate::render::progress::{Planned, Progress, Shape, Verdict};
use crate::secrets::{self, Vault};
use crate::verbs::{load_config, Output};

/// How long the loop sleeps between turns when it has nothing else to do.
///
/// The scheduler's `Sleep` action names an *upper* bound — the nearest deadline
/// or the next heartbeat — and sleeping less than it is always safe. Sleeping
/// the whole of it is not: children are polled on this loop, so a run with a
/// fifteen-minute deadline would notice a child that exited in a second only
/// fifteen minutes later.
const TURN_MS: u64 = 20;

/// Run it.
///
/// `progress` is told what is happening as it happens. **It writes to stderr or
/// to nothing at all** (`render::progress`) — stdout carries the result, and a
/// spinner on it would reach `| jq`. It is a parameter rather than something
/// this module reaches for, because whether there is a person watching is a fact
/// about the invocation, not about the run.
pub fn run<R: Run, C: Clock, F: Fetch>(
    app: &mut App<R, C, F>,
    args: &Check,
    progress: &mut dyn Progress,
) -> Result<Output, ArmadaError> {
    // **Answered before the config is even read.** `--status` is a query about
    // a run that already happened, and a workspace whose `armada.yml` has been
    // edited — or broken — since is exactly when somebody needs to read what
    // the last run decided. Refusing to answer because the *current* config no
    // longer parses would lose the record at the moment it is worth most.
    if args.status {
        return status(app, args).map(|e| Output::Check(Box::new(e)));
    }

    let (workspace, config) = load_config(app)?;
    let selection = select_checks(&config, args)?;
    let candidates = candidate_files(app, &workspace, args, &selection)?;
    let ports = assigned_ports(app, &workspace, &config)?;
    let plans = build_plans(&config, &selection, &candidates, &ports, &workspace, args)?;

    // **Before any provider is invoked, and that is the ordering `--dry-run`
    // needs.** Reporting what *would* run is not a reason to make somebody
    // touch a hardware key, so the preview returns from above the resolution
    // rather than being carved out of it (PLAN.md §4.7).
    if args.dry_run {
        return dry(app, &workspace, plans).map(|e| Output::CheckDryRun(Box::new(e)));
    }

    // **Resolved here, in the caller's terminal, whether or not this run is
    // about to detach** (PLAN.md §4.7, `docs/reserved/013`). Putting this in
    // the run loop at `Action::Spawn` — where the value is actually needed — is
    // right for an attached run and wrong for a detached one, and it fails
    // *only* in the detached case: the `setsid`'d child has no terminal, so `op`
    // would wait on a biometric nobody is looking at and the run would hang
    // until its ceiling instead of failing. `crates/helm/src/secrets.rs` is the
    // whole of the reasoning, including how a value reaches that child.
    let vault = resolve_secrets(app, &config, &workspace, &plans)?;

    // **Everything above ran in the caller's terminal, and that is the point.**
    // Selection, the diff, the port block and the substitution of every argv
    // are the failures a caller can act on, and a `--detach` that deferred them
    // would report a run id for a run that was already doomed — the caller
    // would then poll it to learn what a synchronous error had been ready to
    // say. What detaches is the part that takes time.
    if args.detach {
        return detach(app, &workspace, plans, args, &vault).map(|e| Output::Check(Box::new(e)));
    }

    // **Fail fast, unless the caller asked to queue** (PLAN.md §3.2.1).
    // Blocking by default would mean an agent expecting a quick lint silently
    // waiting out a fifteen-minute test suite with no output; `--wait` is there
    // when queueing is what you meant, and is exempt from the ceiling because
    // it is the caller asking rather than a wait Armada imposed.
    let policy = if args.wait {
        Policy::Block
    } else {
        Policy::FailFast
    };
    let lease = LeaseId::run(workspace.id.clone());
    let envelope = app::with_lease(app, lease, policy, None, |app| {
        execute(app, &workspace, plans, args, progress, &vault)
    });
    // **The line is erased whichever way the run ended**, including the lease
    // acquisition failing before anything was drawn. A failure report printed
    // over half a spinner frame is the one moment it would matter most.
    progress.finish();

    Ok(Output::Check(Box::new(envelope?)))
}

// ------------------------------------------------------------------ selection

fn select_checks(config: &ResolvedConfig, args: &Check) -> Result<Selection, ArmadaError> {
    let selector = match (&args.component, &args.selector, args.files.is_empty()) {
        (Some(component), _, _) => Selector::Component(component.clone()),
        (None, Some(word), _) => select::classify(word),
        (None, None, false) => Selector::Paths(args.files.clone()),
        (None, None, true) => Selector::Everything,
    };
    select::resolve(config, &selector)
}

/// The file set the per-check `${files}` are filtered out of.
///
/// Three sources, and which one applies is the caller's choice rather than a
/// fallback chain — **Armada never silently falls back to the whole tree**
/// (PLAN.md §4.1), because that is the same hole `--all-files` exists to close
/// with an extra step.
fn candidate_files<R: Run, C: Clock, F: Fetch>(
    app: &App<R, C, F>,
    workspace: &Workspace,
    args: &Check,
    selection: &Selection,
) -> Result<Vec<String>, ArmadaError> {
    // An explicit list wins: the caller said exactly which files.
    if let Some(files) = &selection.files {
        return Ok(files.clone());
    }
    if args.all_files {
        return git::tracked_files(&app.ctx.run, &workspace.root);
    }
    match git::merge_base(&app.ctx.run, &workspace.root) {
        Some((_, base)) => git::changed_files(&app.ctx.run, &workspace.root, &base),
        None => Err(select::no_merge_base(&git::BASE_REFS)),
    }
}

/// The workspace's port assignments, for `${port.NAME}`.
///
/// **A workspace with no claimed block has not been initialised**, and saying so
/// is better than letting the substitution fail: `${port.api}` would report "no
/// port named api is declared", which sends the caller to edit a config that is
/// correct.
fn assigned_ports<R: Run, C: Clock, F: Fetch>(
    app: &App<R, C, F>,
    workspace: &Workspace,
    config: &ResolvedConfig,
) -> Result<BTreeMap<String, u16>, ArmadaError> {
    // The same predicate `init` decides with, so the two cannot disagree about
    // what "needs ports" means — which is what would otherwise produce a check
    // demanding a block that `init` deliberately did not claim.
    let declares_ports = armada_core::ports::needs_block(config);

    let block = app
        .db
        .workspaces()?
        .into_iter()
        .find(|row| row.id == workspace.id)
        .and_then(|row| row.ports);

    match (block, declares_ports) {
        (Some(block), _) => {
            armada_core::ports::assign_ports(config, block, &workspace.config_label)
        }
        (None, false) => Ok(BTreeMap::new()),
        (None, true) => Err(ArmadaError {
            class: ErrClass::BadInvocation,
            r#where: "port_block".to_string(),
            message: "this workspace holds no port block, so `${port.…}` cannot resolve"
                .to_string(),
            next_action: Some("`armada manifest init` claims one".to_string()),
        }),
    }
}

// ------------------------------------------------------------------- planning

/// Turn each selected check into the immutable half the scheduler works over.
///
/// **The argv is split and substituted here, once.** That is a pure decision
/// (PLAN.md §4.1.1), so the seam never re-parses anything, a test can assert the
/// exact vector, and the dispatch record carries what actually ran rather than a
/// reconstruction of it.
fn build_plans(
    config: &ResolvedConfig,
    selection: &Selection,
    candidates: &[String],
    ports: &BTreeMap<String, u16>,
    workspace: &Workspace,
    args: &Check,
) -> Result<Vec<Plan>, ArmadaError> {
    let mut plans = Vec::new();
    for id in &selection.checks {
        let Some((component, check)) = find_check(config, id) else {
            continue;
        };
        plans.push(plan_for(
            component, check, id, candidates, ports, workspace, args,
        )?);
    }
    Ok(plans)
}

fn find_check<'a>(
    config: &'a ResolvedConfig,
    id: &CheckId,
) -> Option<(&'a ResolvedComponent, &'a ResolvedCheck)> {
    config.components.values().find_map(|component| {
        component
            .checks
            .values()
            .find(|check| check.id == id.as_str())
            .map(|check| (component, check))
    })
}

fn plan_for(
    component: &ResolvedComponent,
    check: &ResolvedCheck,
    id: &CheckId,
    candidates: &[String],
    ports: &BTreeMap<String, u16>,
    workspace: &Workspace,
    args: &Check,
) -> Result<Plan, ArmadaError> {
    let at = ConfigWhere::Path {
        file: workspace.config_label.clone(),
        path: format!("components.*.checks.{}", id),
    };

    // **`--fix` runs `fix:` instead of `cmd:`, and skips those that do not
    // declare one** (PLAN.md §3.2). Skipping rather than falling back to `cmd:`
    // is the point: `--fix` asks Armada to change files, and running the checking
    // command instead would report a failure the caller asked Armada to repair.
    let (command, skipped_by_fix) = match (args.fix, &check.fix) {
        (true, Some(fix)) => (fix.clone(), None),
        (true, None) => (
            check.cmd.clone(),
            Some("no fix: declared for this check".to_string()),
        ),
        (false, _) => (check.cmd.clone(), None),
    };

    let files = match check.scope {
        Scope::File => select::files_for(&component.match_globs, candidates),
        // A component-scoped check has no `${files}`, and the schema already
        // rejects one that mentions it — the two say opposite things, and
        // quietly honouring one is how a suite appears to run and covers
        // nothing.
        Scope::Component => Vec::new(),
    };

    let empty: BTreeMap<String, String> = BTreeMap::new();
    let vars = Vars {
        workspace_id: workspace.id.as_str(),
        ports,
        component_root: component.root.as_deref(),
        files: Some(&files),
        env: &empty,
    };

    let argv = if check.shell {
        template::shell_argv(&command, &vars, &at)?
    } else {
        template::expand_argv(&command, &vars, &at)?
    };

    let mut env = EnvDelta::default();
    for (name, value) in &check.env {
        env.set.insert(
            name.clone(),
            template::substitute(value, &vars, Site::EnvValue, &at)?,
        );
    }
    // **Names only.** Resolution happens in the shell at spawn and the value
    // never enters the core (`ARCHITECTURE.md` §1.8); the injection itself is
    // phase 4's, which is the phase where there is finally something to inject.
    env.secrets = check.secrets.clone();

    let skip = skipped_by_fix.or_else(|| select::skip_reason(check.scope, &files));

    Ok(Plan {
        blocked: blocked_on_a_service(check),
        id: id.clone(),
        argv,
        env,
        files: files.clone(),
        timeout_ms: u64::from(check.timeout) * 1_000,
        cost: check.cost,
        exclusives: check.exclusive.clone(),
        needs: check
            .needs
            .iter()
            .filter_map(|need| match need {
                armada_core::config::Need::Check(check) => Some(CheckId::new(check)),
                armada_core::config::Need::Component(_) => None,
            })
            .collect(),
        log: None,
        skip,
    })
}

/// **`needs:` gates in this phase and starts in phase 4** (`PHASES.md` phase 3).
///
/// The end state is that a check needing `postgres` brings it up — one command
/// instead of three, which matters when the caller is an agent. `armada manifest up` does
/// not exist yet, so the honest answer is a `bad_invocation` naming the service
/// and saying how to start it. Phase 4 replaces the error with the start; this
/// is **one behaviour built in two steps, not two behaviours**.
///
/// **Nothing Armada started is running, and that is a fact rather than an
/// assumption.** `up` is the only verb that records a service as `owned`, and
/// it is not built — so there is no state in which this answer is wrong today.
/// Phase 4 is where liveness becomes a question worth asking, because that is
/// the phase where something can answer it.
///
/// `in:` implies `needs:` on the enclosing component (PLAN.md §4.1): the
/// container has to be running before Armada can exec into it, so a check that
/// declares one is gated exactly like a check that names the component.
fn blocked_on_a_service(check: &ResolvedCheck) -> Option<ArmadaError> {
    let mut services: Vec<String> = check
        .needs
        .iter()
        .filter_map(|need| match need {
            armada_core::config::Need::Component(name) => Some(name.clone()),
            armada_core::config::Need::Check(_) => None,
        })
        .collect();
    if let Some(service) = &check.in_service {
        // Named separately so the message says the service the caller wrote
        // rather than the component Armada inferred it from.
        services.push(service.clone());
    }
    services.sort();
    services.dedup();
    if services.is_empty() {
        return None;
    }

    let named = services.join(", ");
    Some(ArmadaError {
        class: ErrClass::BadInvocation,
        r#where: check.id.clone(),
        message: format!("`{}` needs {named}, which is not running", check.id),
        next_action: Some(format!(
            "`armada manifest up {named}` starts it — not built until phase 4, so start it by hand for now"
        )),
    })
}

// -------------------------------------------------------------------- secrets

/// Every secret this run's checks were granted, resolved — or inherited from
/// the parent that already resolved them.
///
/// **Two callers of this function and only one of them ever runs a provider.**
/// An ordinary invocation holds the terminal and resolves; a detached child
/// reads what its parent resolved off stdin and **must never invoke a provider
/// at all**, because it has no terminal for one to prompt on. That branch is
/// the enforcement of `PLAN.md` §4.7's *"resolved before the process detaches"*,
/// and it is written as a branch rather than as a comment precisely because the
/// wrong version passes every attached test.
///
/// **Called before the run lease is taken, and that ordering is load-bearing.**
/// A provider may wait two minutes for a person to find a hardware key
/// ([`secrets::PROVIDER_TIMEOUT_MS`]); holding the workspace's run lease across
/// that would block every other agent in the worktree on a prompt they cannot
/// see, and a heartbeat renewed from a loop that is parked in `read` is the
/// shape PLAN.md §4.3 exists to make impossible. Nothing is claimed until the
/// answers are in hand.
///
/// **Only what this run wants.** `check --component api` resolves `api`'s
/// grants and nothing else, so a repository with four providers does not
/// produce four prompts to lint one component. Skipped checks are included:
/// a `--fix` that skips a check has still selected it, and a plan is not the
/// place to decide that a grant was hypothetical.
fn resolve_secrets<R: Run, C: Clock, F: Fetch>(
    app: &App<R, C, F>,
    config: &ResolvedConfig,
    workspace: &Workspace,
    plans: &[Plan],
) -> Result<Vault, ArmadaError> {
    if let Some(payload) = &app.handoff {
        return Ok(secrets::inherit(payload));
    }

    let wanted = armada_core::secrets::wanted(plans.iter().map(|plan| plan.env.secrets.as_slice()));
    if wanted.is_empty() {
        // **No calls, so no `op`, so no prompt.** The overwhelming majority of
        // runs are this one, and a repository that declares providers but grants
        // nothing to the selected checks must be indistinguishable from one that
        // declares none.
        return Ok(Vault::default());
    }
    let calls = armada_core::secrets::plan(config, &wanted, &workspace.config_label)?;
    secrets::resolve(
        &app.ctx.run,
        &app.cwd(),
        &calls,
        secrets::PROVIDER_TIMEOUT_MS,
    )
}

// -------------------------------------------------------------------- dry run

fn dry<R: Run, C: Clock, F: Fetch>(
    app: &mut App<R, C, F>,
    workspace: &Workspace,
    plans: Vec<Plan>,
) -> Result<Envelope<CheckDryRun>, ArmadaError> {
    let mut preview = CheckDryRun::default();
    for plan in &plans {
        match &plan.skip {
            Some(reason) => preview.would_skip.push(format!("{}: {reason}", plan.id)),
            None => preview
                .would_run
                .push(format!("{}: {}", plan.id, plan.argv.join(" "))),
        }
    }
    // Listing changes nothing, so the preview is exactly the decision the real
    // pass makes a moment later rather than an approximation of it.
    let present = runs::present(&workspace.root)?;
    preview.would_reap = armada_core::run::runs_to_reap(&present, app.machine.run_retention, &[])
        .iter()
        .map(RunId::to_string)
        .collect();

    Ok(Envelope::ok(
        "check",
        Some(workspace.id.clone()),
        Status::Skipped,
        preview,
    ))
}

// --------------------------------------------------------- detach and status

/// How a detached child is told which run it is.
///
/// **Its own variable rather than `ARMADA_RUN_ID`**, and the difference is what
/// each one claims. `ARMADA_RUN_ID` marks a child *inside* a run, and the rule
/// that goes with it is that such a child joins the outer run and inherits its
/// lease (PLAN.md §3.2.1, `armada_core::run::nesting`). A detached child
/// inherits nothing: the invocation that started it has already exited and
/// holds no lease, so the child takes the run lease itself exactly as a
/// foreground run does. Two different claims, two different variables — one
/// variable meaning both is how a detached run would come to skip the lease
/// that stops two runs sharing one workspace.
pub const DETACH_RUN_VAR: &str = "ARMADA_DETACH_RUN";

/// The run this invocation was detached to carry out, if it is one.
///
/// **Validated rather than trusted, because the id becomes a path.** The same
/// rule `ARMADA_RUN_ID` is read under: Armada sets this variable and Armada
/// reads it back, so a value that is not a run id means something in between
/// rewrote it — and a value of `../../etc` reaching `.armada/run/<id>/` is a
/// directory traversal. A malformed one starts an ordinary run of its own,
/// which is the same forgiving answer `nesting` gives.
fn adopted_run<R: Run, C: Clock, F: Fetch>(app: &App<R, C, F>) -> Option<RunId> {
    app.inherited
        .get(DETACH_RUN_VAR)
        .and_then(|value| RunId::parse(value).ok())
}

/// The command line the detached child is given.
///
/// **Rebuilt from what was parsed, not replayed from `argv`.** A replay would
/// have to strip `--detach` out of a vector it did not parse, and would carry
/// through whatever spelling the caller used — `--color=always` from a terminal
/// that is about to go away. What the child needs is the same *selection*,
/// which is exactly what the parsed struct holds.
///
/// `--json` and `--color never` because the child's output goes to a file: an
/// envelope is what a later reader can act on, and a spinner's escape codes in
/// a log are noise nobody asked for.
fn detached_argv(program: &str, args: &Check) -> Vec<String> {
    let mut argv = vec![
        program.to_string(),
        "manifest".to_string(),
        "check".to_string(),
        "--json".to_string(),
        "--color".to_string(),
        "never".to_string(),
    ];
    if args.all_files {
        argv.push("--all-files".to_string());
    }
    if args.fix {
        argv.push("--fix".to_string());
    }
    // **Carried through, and it is the caller's own instruction.** A detached
    // run that fails fast on a lease has nowhere to report it but its log;
    // `--wait` is how a caller says to queue instead, and dropping it here
    // would silently turn the one shape that always succeeds into the one that
    // sometimes does not.
    if args.wait {
        argv.push("--wait".to_string());
    }
    if let Some(jobs) = args.jobs {
        argv.push("--concurrency".to_string());
        argv.push(jobs.to_string());
    }
    if let Some(component) = &args.component {
        argv.push("--component".to_string());
        argv.push(component.clone());
    }
    if let Some(selector) = &args.selector {
        argv.push(selector.clone());
    }
    // **Last, because it consumes until the next flag.** Its own parser reads
    // paths until it meets one, so anything placed after them would be read as
    // a path.
    if !args.files.is_empty() {
        argv.push("--files".to_string());
        argv.extend(args.files.iter().cloned());
    }
    argv
}

/// Start the run in its own session and answer with its id.
///
/// **The same shape Fleet gives a Drone** (`fleet::drone::start`), against a
/// run directory instead of a transcript: `setsid` so the child is its own
/// process group, a real file rather than a pipe so it survives this process,
/// the handle dropped without a `wait`, and the group recorded as owned so
/// `armada manifest clean` can reclaim it. Reusing the shape is the point — an
/// orphaned detached run is then reaped by the same pass that reaps an orphaned
/// Drone, and *"nothing is killed that Armada cannot prove is its own"* stays
/// one rule with one implementation.
///
/// **What is written before the child starts is what makes the id answerable.**
/// The record goes down first, so a `--status` racing this call reads a run
/// that is planned rather than one that does not exist.
fn detach<R: Run, C: Clock, F: Fetch>(
    app: &mut App<R, C, F>,
    workspace: &Workspace,
    plans: Vec<Plan>,
    args: &Check,
    vault: &Vault,
) -> Result<Envelope<CheckData>, ArmadaError> {
    let run_id = RunId::mint(app.ctx.now.wall_ms(), entropy(app));
    let (reaped, skipped) = runs::reap(&workspace.root, app.machine.run_retention, &[])?;
    fs::create_armada_dir(&workspace.root)?;
    runs::prepare(&workspace.root, &run_id)?;

    let slots = args.jobs.unwrap_or(app.machine.cpu_slots);
    let plans: Vec<Plan> = plans
        .into_iter()
        .map(|mut plan| {
            plan.log = Some(runs::log_reference(&run_id, &plan.id));
            plan
        })
        .collect();
    let state = State::new(workspace.root.clone(), slots, plans);
    let planned = state.snapshot();
    runs::write_record(
        &workspace.root,
        &RunRecord::new(
            run_id.clone(),
            workspace.id.clone(),
            app.ctx.now.wall_rfc3339(),
            state,
        ),
    )?;

    // **This invocation's own binary, resolved rather than assumed.** A child
    // spawned as `armada` would be whichever `armada` is on the child's `PATH`,
    // which on a machine mid-upgrade is a different build deciding a run this
    // one planned.
    let program = std::env::current_exe().map_err(|e| ArmadaError {
        class: ErrClass::Environment,
        r#where: "detach".to_string(),
        message: format!("Armada cannot find its own binary to detach: {e}"),
        next_action: Some("run `armada manifest check` in the foreground".to_string()),
    })?;
    let argv = detached_argv(&program.to_string_lossy(), args);
    let request = RunRequest::new(argv, workspace.root.clone())
        .env(BTreeMap::from([(
            DETACH_RUN_VAR.to_string(),
            run_id.to_string(),
        )]))
        // **A real file, not a pipe.** The run outlives this process, and a
        // captured stream would leave Armada holding the read end of a pipe it
        // is about to drop — the first write after that is `EPIPE`, which would
        // kill the run moments after this call reported it started.
        .stdio(StdioMode::Log(runs::detach_log(&workspace.root, &run_id)))
        // **The secret handoff, and the reason it goes here rather than into
        // `.env()` above.** PLAN.md §4.7 rule 5: §4.5 inherits Armada's own
        // environment wholesale to every child, so a resolved value put in it
        // would grant every secret to every child and void per-entry grants
        // entirely. This is a pipe between exactly two processes, written and
        // closed inside `ProcessGroup::spawn` before this call returns, and it
        // touches no disk on the way — which is the constraint
        // `ARCHITECTURE.md` §1.8 puts on the whole design. See
        // `crates/helm/src/secrets.rs` for the alternatives and why each fails.
        .with_stdin(vault.handoff());

    // **Recorded before it is spawned, and settled afterwards** — the order
    // `armada manifest up` established, for its reason: the failure mode must
    // be a stale row and never an untracked process group. Spawn-then-record
    // leaks a group if this process dies in between; this leaves a row pointing
    // at nothing, which the next reap drops for free.
    let pending = format!("detach:{run_id}");
    app.db.record_owned(&OwnedRow {
        workspace: workspace.id.clone(),
        kind: OwnedKind::Pgid,
        reference: pending.clone(),
        boot_id: Some(app.boot_id.clone()),
        pid_started_at: None,
        component: None,
    })?;

    let spawned = ProcessGroup::spawn(&request);
    let _ = app
        .db
        .delete_owned(&workspace.id, OwnedKind::Pgid, &pending);
    let group = spawned.map_err(|e| ArmadaError {
        class: ErrClass::Environment,
        r#where: "detach".to_string(),
        message: format!("the detached run would not start: {}", e.message),
        next_action: Some("run `armada manifest check` in the foreground".to_string()),
    })?;
    let pgid = group.pgid();
    // **The handle is dropped and the child is not waited on**, which is the
    // one place that is correct: the run is meant to outlive this invocation.
    // `setsid` has already put it in its own session, so init reaps it — the
    // zombie rule `docs/traps.md` states applies to the children Armada keeps.
    drop(group);

    let detached = Detached {
        pgid,
        boot_id: app.boot_id.clone(),
        // **Sampled after the spawn rather than remembered from it**, because
        // this is the value a later poll compares against, and it has to be
        // the one the machine reports rather than the one Armada guessed.
        // Without it a recycled pid is indistinguishable from a live run, so
        // the row is a permanent phantom (PLAN.md §2.3.1).
        started_at: armada_manifest::machine::process_start_at(&app.ctx.run, &app.cwd(), pgid),
        log: runs::detach_log_reference(&run_id),
    };
    app.db.record_owned(&OwnedRow {
        workspace: workspace.id.clone(),
        kind: OwnedKind::Pgid,
        reference: pgid.to_string(),
        boot_id: Some(detached.boot_id.clone()),
        pid_started_at: detached.started_at.clone(),
        component: None,
    })?;
    runs::write_detached(&workspace.root, &run_id, &detached)?;

    let mut reaped_runs: Vec<String> = reaped.iter().map(RunId::to_string).collect();
    reaped_runs.extend(skipped);

    Ok(Envelope {
        schema_version: armada_core::envelope::SCHEMA_VERSION,
        verb: "check".to_string(),
        workspace: Some(workspace.id.clone()),
        // **`RUNNING`, which is progress and not a verdict** (PLAN.md §3.1).
        // It carries no error, so this exits 0 — and a gate that treated that
        // as a pass would be reading the exit code of the *start*. The verdict
        // is what `--status` returns when the run has one.
        status: Status::Running,
        error: None,
        data: CheckData {
            run_id: run_id.to_string(),
            results: planned.iter().map(Into::into).collect(),
            reaped_runs,
            detached: Some(DetachedView {
                pgid,
                alive: true,
                log: detached.log,
            }),
        },
    })
}

/// Read a run out of the run directory.
///
/// **It asks the directory, never the process.** A run that is still going is
/// writing its record as each check settles, and that record is the run — so
/// the answer to *"what has it decided"* comes from what it wrote down. The one
/// thing the directory cannot answer is whether anything is still deciding, and
/// that is the only question the process probe is put.
///
/// The three answers, in the order they are ruled out:
///
/// | the record | the group | answer |
/// |---|---|---|
/// | has a verdict | either | the verdict, exactly as the run reported it |
/// | has none | alive | `RUNNING`, with the rows settled so far |
/// | has none | gone | `DEAD` — it stopped without deciding |
fn status<R: Run, C: Clock, F: Fetch>(
    app: &mut App<R, C, F>,
    args: &Check,
) -> Result<Envelope<CheckData>, ArmadaError> {
    let workspace = app.ctx.workspace()?.clone();
    let run_id = match &args.run {
        Some(named) => RunId::parse(named)?,
        None => runs::present(&workspace.root)?
            .pop()
            .ok_or_else(|| no_runs(&workspace.id))?,
    };
    let mut record = runs::read_record(&workspace.root, &run_id)?
        .ok_or_else(|| unknown_run(&run_id, &workspace.id))?;
    let detached = runs::read_detached(&workspace.root, &run_id)?;

    let alive = detached.as_ref().is_some_and(|detached| {
        let observed = armada_manifest::machine::process_start_at(
            &app.ctx.run,
            &workspace.root,
            detached.pgid,
        );
        detached.is_ours(&app.boot_id, observed.as_deref())
    });

    // **A running check's elapsed time is measured against the reader's clock,
    // not the writer's.** `State::now_mono` is the last reading the *shell*
    // reported, and a detached shell writes its journal at step boundaries — so
    // a check that has been running for thirteen minutes without finishing
    // reported `duration_ms: 786`, frozen at the moment it started, and every
    // subsequent `--status` repeated that number. A duration that does not move
    // while the thing it measures does is worse than no duration: it reads as
    // evidence the run is wedged.
    //
    // **Only while it is alive**, and the guard is not incidental. `alive` is
    // already `is_ours(boot_id, …)`, so it is exactly the condition under which
    // the two processes' readings are comparable: `mono()` is `clock_gettime`
    // on `CLOCK_UPTIME_RAW`/`CLOCK_MONOTONIC`, which is machine-wide since boot
    // rather than process-relative, and meaningless across a reboot. A finished
    // or dead run keeps what its record says, which is the honest final figure
    // and the one the foreground run printed.
    if alive {
        record.state.now_mono = record.state.now_mono.max(app.ctx.now.mono());
    }

    let (status, error) = match record.state.verdict() {
        // **The record wins over the probe, and the order matters.** A run that
        // has written its verdict has finished deciding whether or not its
        // process has got as far as exiting, and re-deciding here on the
        // strength of a `ps` would make a detached run answer differently from
        // the attached one that produced the same rows.
        Some(verdict) => verdict,
        None if alive => (Status::Running, None),
        None => (
            Status::Dead,
            Some(died_without_deciding(&run_id, &detached)),
        ),
    };

    Ok(Envelope {
        schema_version: armada_core::envelope::SCHEMA_VERSION,
        verb: "check".to_string(),
        workspace: Some(workspace.id.clone()),
        status,
        error,
        data: CheckData {
            run_id: run_id.to_string(),
            results: record.state.snapshot().iter().map(Into::into).collect(),
            // Nothing was reaped by reading.
            reaped_runs: Vec::new(),
            detached: detached.map(|detached| DetachedView {
                pgid: detached.pgid,
                alive,
                log: detached.log,
            }),
        },
    })
}

fn no_runs(workspace: &armada_core::id::WorkspaceId) -> ArmadaError {
    ArmadaError {
        class: ErrClass::BadInvocation,
        r#where: workspace.as_str().to_string(),
        message: "this workspace has kept no runs".to_string(),
        next_action: Some("`armada manifest check --detach` starts one".to_string()),
    }
}

fn unknown_run(run: &RunId, workspace: &armada_core::id::WorkspaceId) -> ArmadaError {
    ArmadaError {
        class: ErrClass::BadInvocation,
        r#where: run.to_string(),
        message: format!("no run `{run}` in workspace {workspace}"),
        // **Not "list them", because nothing lists them.** `armada manifest
        // status` answers about ports, leases and what is owned; run
        // directories are reaped on a retention count and are not among what it
        // reports. The one thing that is true and actionable is that `--status`
        // with no id reads the most recent run there is.
        next_action: Some(
            "`armada manifest check --status` reads the most recent run this workspace kept"
                .to_string(),
        ),
    }
}

/// A run whose process is gone and whose record holds no verdict.
///
/// **`aborted`, which is retryable**, because that is what actually happened:
/// something stopped the run — a reboot, a `clean`, a SIGKILL — rather than any
/// check reaching a conclusion. Reporting `tool_failed` would send a reader to
/// hunt a broken test that never ran.
fn died_without_deciding(run: &RunId, detached: &Option<Detached>) -> ArmadaError {
    ArmadaError {
        class: ErrClass::Aborted,
        r#where: run.to_string(),
        message: "the run stopped without reaching a verdict".to_string(),
        next_action: Some(match detached {
            Some(detached) => format!("{} says why, if anything does", detached.log),
            // No detach record at all: this was a foreground run, and nothing
            // recorded a group to ask about. Saying so is better than implying
            // Armada probed and found nothing.
            None => "this run was not detached, so nothing recorded a process to ask".to_string(),
        }),
    }
}

// ---------------------------------------------------------------- the record

/// The run record as it stands right now.
///
/// **A projection of the loop's own state and nothing else.** The record is not
/// a second copy of the run kept alongside it; it is the reducer's `State` and
/// the journal, serialized. That is what makes `--status` able to answer with
/// what the run decided rather than with what a writer remembered to say.
fn record_of(workspace: &Workspace, it: &Loop<'_>) -> RunRecord {
    RunRecord {
        schema_version: armada_core::envelope::SCHEMA_VERSION,
        run_id: it.run_id.clone(),
        workspace: workspace.id.clone(),
        started_at: it.started_at.clone(),
        state: it.state.clone(),
        journal: it.journal.clone(),
    }
}

/// Write the record where it stands, and carry on if it cannot be written.
///
/// **A checkpoint is evidence and never a step of the run.** It is taken every
/// time a check moves, so a run that is SIGKILLed halfway has the verdicts it
/// did reach on disk rather than nothing at all — and so `--status` reports a
/// check that is running as `RUNNING` rather than as the `WAITING` it was when
/// the last verdict landed. `write_record` renames into place, so a poll
/// arriving mid-write reads the previous checkpoint and never half of one.
///
/// **Failing to write one does not fail the run.** The final write is the one
/// that must succeed and is still checked; a disk that rejected an intermediate
/// snapshot is not a reason to throw away checks that have already passed.
fn checkpoint(workspace: &Workspace, it: &mut Loop<'_>) {
    it.checkpointed = phases(&it.state);
    let _ = runs::write_record(&workspace.root, &record_of(workspace, it));
}

/// Where every check has got to, and nothing else.
///
/// **What decides whether the record is rewritten**, so that "on every state
/// change" is a handful of writes per check rather than one per turn of a loop
/// that turns every twenty milliseconds. Phases are the whole of what a reader
/// of the record can act on; the clock reading and the byte counts move
/// constantly and change no answer.
///
/// **Not a copy of anything the reducer owns.** It is a memo of what has
/// already been persisted — the same kind of bookkeeping as [`Loop::polled`],
/// and the reducer neither writes files nor knows one was written.
fn phases(state: &State) -> String {
    let mut out = String::new();
    for entry in state.checks.values() {
        match &entry.phase {
            Phase::Pending => out.push('p'),
            Phase::Waiting(waiting) => {
                out.push('w');
                out.push_str(&waiting.kind.to_string());
            }
            // A stop in progress is a state change worth recording: the run has
            // decided to end this check and has not yet.
            Phase::Running(running) => {
                out.push('r');
                if running.stopping.is_some() {
                    out.push('!');
                }
            }
            Phase::Done(_) => out.push('d'),
            Phase::Skipped => out.push('s'),
        }
        out.push(',');
    }
    out
}

// ---------------------------------------------------------------- the run

fn execute<R: Run, C: Clock, F: Fetch>(
    app: &mut App<R, C, F>,
    workspace: &Workspace,
    plans: Vec<Plan>,
    args: &Check,
    progress: &mut dyn Progress,
    vault: &Vault,
) -> Result<Envelope<CheckData>, ArmadaError> {
    // **A detached child adopts the run its parent already opened** rather
    // than minting one of its own, because the parent has already told the
    // caller which run this is. A second id here would leave `--detach`
    // printing one directory and `--status` reading another.
    let adopted = adopted_run(app);
    let run_id = adopted
        .clone()
        .unwrap_or_else(|| RunId::mint(app.ctx.now.wall_ms(), entropy(app)));
    app.run = Some(run_id.clone());

    // Reap first, then create: the new directory must not count toward its own
    // retention budget, and reaping is reported rather than silent.
    //
    // **The adopting child reaps nothing**, and not merely to save the work:
    // its own directory is already on disk, so a reap here would be a retention
    // pass that counts the run it is about to write into — and the parent has
    // already done the pass and reported what it removed.
    let (reaped, skipped) = match adopted {
        Some(_) => (Vec::new(), Vec::new()),
        None => runs::reap(&workspace.root, app.machine.run_retention, &[])?,
    };
    fs::create_armada_dir(&workspace.root)?;
    runs::prepare(&workspace.root, &run_id)?;

    let slots = args.jobs.unwrap_or(app.machine.cpu_slots);
    let plans: Vec<Plan> = plans
        .into_iter()
        .map(|mut plan| {
            plan.log = Some(runs::log_reference(&run_id, &plan.id));
            plan
        })
        .collect();
    // **By name, before anything runs.** The live table has a row per check
    // from its first frame, so a run that is waiting on a lease still shows
    // what it is waiting to run rather than an empty count.
    // Each carries its own deadline, so a row that has been running a while can
    // say whether that is close to its budget or nowhere near it.
    let planned: Vec<Planned<'_>> = plans
        .iter()
        .map(|plan| Planned {
            id: plan.id.as_str(),
            timeout_ms: Some(plan.timeout_ms),
        })
        .collect();
    progress.begin(Shape::Check, &planned, app.ctx.now.mono());
    drop(planned);

    let mut loop_state = Loop {
        progress,
        state: State::new(workspace.root.clone(), slots, plans),
        journal: Journal::default(),
        children: BTreeMap::new(),
        rows: BTreeMap::new(),
        finish: None,
        interrupted: false,
        scrub: Scrub::new(&workspace.root, &workspace.id),
        run_id: run_id.clone(),
        // **Read once, before the run.** It used to be read where the record is
        // written, which is after the last check — so a fifteen-minute run
        // recorded a start time fifteen minutes after it started.
        started_at: app.ctx.now.wall_rfc3339(),
        checkpointed: String::new(),
        ceiling_ms: app.machine.acquire_ceiling_ms(),
        held: BTreeMap::new(),
        slots: app.machine.cpu_slots,
        polled: BTreeMap::new(),
        vault,
    };

    // **The record exists before the first check does.** `--status` may be
    // asked a millisecond after `--detach` returns, and the honest answer then
    // is the run's plan; a missing file would read as *"no such run"* about a
    // run that had just been reported by id.
    checkpoint(workspace, &mut loop_state);

    let drove = drive(app, workspace, &mut loop_state);
    // **Nothing the shell took is left behind**, on every path out including
    // the failing one. The reducer releases what it knows about, and it knows
    // about *classes*: a check whose claim never completed has an empty `held`
    // there, so an exclusive it did take — `browser`, while `gpu` was still
    // held elsewhere — would be released by nobody and sit in `manifest.db`
    // until its heartbeat went cold sixty seconds later.
    for lease in std::mem::take(&mut loop_state.held).into_values().flatten() {
        let _ = app.release(&lease);
        app.held.retain(|h| h != &lease);
    }
    drove?;

    runs::write_record(&workspace.root, &record_of(workspace, &loop_state))?;

    let (status, error) = loop_state
        .finish
        .unwrap_or((Status::Failed, Some(never_finished())));

    let mut reaped_runs: Vec<String> = reaped.iter().map(RunId::to_string).collect();
    reaped_runs.extend(skipped);

    let data = CheckData {
        run_id: run_id.to_string(),
        results: loop_state.rows.values().map(Into::into).collect(),
        reaped_runs,
        // The run is this process. A caller holding this envelope has already
        // waited for it, so there is nothing to say about a group to poll.
        detached: None,
    };

    Ok(Envelope {
        schema_version: armada_core::envelope::SCHEMA_VERSION,
        verb: "check".to_string(),
        workspace: Some(workspace.id.clone()),
        status,
        error,
        data,
    })
}

/// Everything the loop carries that is not the scheduler's.
struct Loop<'a> {
    /// Told what is happening as it happens, and writes to stderr or nowhere.
    /// Borrowed rather than owned so the caller can erase the line after the
    /// loop has ended — including when it ended by failing.
    progress: &'a mut dyn Progress,
    state: State,
    journal: Journal,
    children: BTreeMap<CheckId, ProcessGroup>,
    /// Keyed by id so a `WAITING` row is replaced by the verdict that follows
    /// it. `results[]` is one row per check, and the last thing Armada knew is
    /// the one worth reporting.
    rows: BTreeMap<CheckId, CheckResult>,
    finish: Option<(Status, Option<ArmadaError>)>,
    /// When the run began, RFC 3339. Read once at the top, because it is what
    /// the record says the run started at.
    started_at: String,
    /// [`phases`] as of the last record written, so the next write happens when
    /// something moved and not because a turn went by.
    checkpointed: String,
    /// Whether `Event::Interrupted` has already been delivered. The handler's
    /// flag stays set once tripped, so without this the loop would re-deliver
    /// on every turn.
    interrupted: bool,
    scrub: Scrub,
    run_id: RunId,
    ceiling_ms: u64,
    /// What each check actually holds. **Tracked rather than re-derived**: the
    /// store chooses which CPU slots a claim gets, so `acquisition_order` can
    /// say how many and in what order but not which — and releasing a slot Armada
    /// does not hold would hand another workspace's budget away.
    held: BTreeMap<CheckId, Vec<LeaseId>>,
    /// The machine's slot count, which bounds what the store can ever grant.
    slots: u32,
    /// When each check last attempted its claim, so a polled acquisition is a
    /// poll rather than a spin.
    ///
    /// **Not a copy of anything the reducer owns.** The reducer knows a check
    /// is `Waiting` and when it started; how often the shell goes back to the
    /// store is the shell's own business, and there is nowhere else it could
    /// be recorded.
    polled: BTreeMap<CheckId, u64>,
    /// What this run resolved, **borrowed rather than owned**, because it was
    /// resolved before the run started and outlives it.
    ///
    /// The loop uses it for exactly two things and neither is a decision: the
    /// per-entry grant handed to a child at [`spawn`], and the mask applied to
    /// captured output on its way to disk in [`collect_children`]. Nothing in
    /// [`armada_core::schedule`] can see it — a pure function that has never
    /// seen a value cannot leak one (`ARCHITECTURE.md` §1.8).
    vault: &'a Vault,
}

fn drive<R: Run, C: Clock, F: Fetch>(
    app: &mut App<R, C, F>,
    workspace: &Workspace,
    it: &mut Loop<'_>,
) -> Result<(), ArmadaError> {
    // **The clock before the run, and that ordering is not cosmetic.** A pure
    // reducer cannot read a clock, so `Tick` is where it learns one — and a
    // `Started` that arrives first computes every deadline from a `now_mono` of
    // zero. Measured on the first real run: every check timed out immediately,
    // because the first tick then jumped the clock past a deadline set at
    // `0 + timeout`, and each reported a duration of eleven days.
    let mut queue: Vec<Event> = vec![
        Event::Tick {
            now_mono: app.ctx.now.mono(),
        },
        Event::Started,
    ];

    while it.finish.is_none() {
        while !queue.is_empty() {
            let batch = std::mem::take(&mut queue);
            for event in batch {
                it.journal.observed(&event);
                let (next, actions) =
                    schedule::step(std::mem::replace(&mut it.state, empty()), event);
                it.state = next;
                for action in actions {
                    perform(app, workspace, it, action, &mut queue)?;
                }
            }
        }
        // **Where "the record is rewritten on every state change" is actually
        // true.** The batch above is what moved the run; this is the first
        // moment after it that the state is settled and nothing is mid-step. A
        // check that has just spawned reads as `RUNNING` from here on, which is
        // what a poll is asking about — before this, a run's record went from
        // its plan straight to its verdicts and a `--status` in between
        // reported a check that had been running for two minutes as waiting.
        if phases(&it.state) != it.checkpointed {
            checkpoint(workspace, it);
        }

        if it.finish.is_some() {
            break;
        }

        // **The interrupt is observed before anything else**, because every
        // other observation costs time the operator has just asked to stop
        // spending. `Event::Interrupted` is delivered once — the reducer moves
        // the run to `Ending::Interrupted`, and delivering it again would step
        // a state that has already ended.
        if !it.interrupted && armada_manifest::posix::interrupted() {
            it.interrupted = true;
            queue.push(Event::Interrupted);
            continue;
        }

        // Observe: children first, then deadlines, then the queued claims, then
        // the clock. A child that finished is more informative than a deadline
        // that has not, and both are more informative than a lease that is
        // probably still held — but the ordering is also what keeps a claim
        // from ever being the reason a deadline went unnoticed.
        collect_children(workspace, it, &mut queue);
        collect_deadlines(app, it, &mut queue);
        poll_claims(app, workspace, it, &mut queue)?;
        // One turn of the loop went by. The reading is handed to the watcher
        // rather than read by it: the real clock is injected, and the live
        // table's elapsed column must be the same clock the deadlines are.
        let now_mono = app.ctx.now.mono();
        it.progress.tick(now_mono);
        queue.push(Event::Tick { now_mono });
    }
    Ok(())
}

/// A placeholder while the real state is being stepped. `step` takes ownership
/// (`ARCHITECTURE.md` §1.2: `State` is owned and returned, never mutated in
/// place), so the loop hands it over and takes the answer back.
fn empty() -> State {
    State::new(PathBuf::new(), 1, Vec::new())
}

fn perform<R: Run, C: Clock, F: Fetch>(
    app: &mut App<R, C, F>,
    workspace: &Workspace,
    it: &mut Loop<'_>,
    action: Action,
    queue: &mut Vec<Event>,
) -> Result<(), ArmadaError> {
    match action {
        Action::Acquire { check, kind } => acquire(app, workspace, it, &check, kind, queue),
        Action::Release { check, kind } => {
            let held = it.held.entry(check).or_default();
            let (releasing, keeping): (Vec<LeaseId>, Vec<LeaseId>) =
                held.iter().cloned().partition(|lease| lease.kind == kind);
            *held = keeping;
            for lease in releasing {
                let _ = app.release(&lease);
                app.held.retain(|h| h != &lease);
            }
            Ok(())
        }
        Action::Spawn {
            check,
            argv,
            env,
            cwd,
        } => spawn(app, workspace, it, &check, argv, env, cwd, queue),
        Action::Kill { check, escalate } => {
            if let Some(child) = it.children.get_mut(&check) {
                child.signal(escalate);
            }
            Ok(())
        }
        Action::Renew => {
            app.renew_held();
            Ok(())
        }
        Action::Sleep { until_mono } => {
            // Bounded by a turn, because children are polled on this loop.
            let now = app.ctx.now.mono();
            let until = until_mono.min(now.saturating_add(TURN_MS));
            app.ctx.now.sleep_until(until);
            Ok(())
        }
        // Reaping is what `poll` does; there is no second mechanism, which is
        // what keeps "every spawned child is waited on" true by construction.
        Action::Reap => Ok(()),
        Action::Emit { result } => {
            // **Reported here rather than at the spawn's exit**, because this is
            // the point the scheduler considers the check answered — and a
            // `WAITING` row emitted before a verdict is progress the watcher
            // wants to see change.
            // **The same detail the final table will print**, chosen the same
            // way (`render.rs`): its own failure, else the prose, else nothing.
            // A watcher that showed a verdict without the line explaining it
            // would make the reader wait for the end to learn what went wrong,
            // which is the wait this table exists to remove.
            let detail = result
                .error
                .as_ref()
                .map(|e| e.message.as_str())
                .or(result.reason.as_deref());
            it.progress
                .finished(result.id.as_str(), Verdict::Status(result.status), detail);
            it.rows.insert(result.id.clone(), result);
            Ok(())
        }
        Action::Finish { status, error } => {
            it.finish = Some((status, error));
            Ok(())
        }
    }
}

/// The exclusives one check must take, **in the order that makes a cycle
/// impossible**.
///
/// `lease::acquisition_order` is the single statement of the rule; asking it
/// rather than sorting here is what stops a second implementation of the
/// property. Its CPU-slot entries are deliberately not used for *identity* —
/// see [`Loop::held`].
fn exclusives_of(workspace: &Workspace, it: &Loop<'_>, check: &CheckId) -> Vec<LeaseId> {
    let Some(entry) = it.state.checks.get(check) else {
        return Vec::new();
    };
    lease::acquisition_order(&workspace.id, &entry.plan.exclusives, entry.plan.cost)
        .into_iter()
        .filter(|id| id.kind == LeaseKind::Exclusive)
        .collect()
}

/// **One attempt at a claim, and then back to the loop.**
///
/// `lease::acquisition_order` is the single statement of "exclusives first, in
/// sorted name order, then slots" — the shell asks it which concrete leases a
/// class means and takes them in the order it returns. Sorting here as well
/// would be a second implementation of the property that makes a cross-workspace
/// cycle impossible.
///
/// **What it must not do is wait.** An earlier version blocked here until the
/// lease came free, bounded only by `acquire_timeout` — forty minutes by
/// default — and the run loop is single-threaded, so for the whole of that wait
/// no `Deadline` fired, no `ChildExited` was observed and no interrupt was
/// seen. A check with a sixty-second `timeout:` running alongside sailed past
/// it silently and was killed whenever the *other* check's lease happened to
/// come free. So a denial returns here, the loop keeps turning, and
/// [`poll_claims`] comes back to it.
fn acquire<R: Run, C: Clock, F: Fetch>(
    app: &mut App<R, C, F>,
    workspace: &Workspace,
    it: &mut Loop<'_>,
    check: &CheckId,
    kind: LeaseKind,
    queue: &mut Vec<Event>,
) -> Result<(), ArmadaError> {
    it.polled.insert(check.clone(), app.ctx.now.mono());

    // **The ceiling is read off the scheduler's `Waiting::since_mono`**, not
    // accumulated here. A blocking claim could count its own sleeps because it
    // owned the whole wait; a polled one does not, and a counter in the shell
    // would be a second answer to "how long has this waited" that disagreed
    // with the reducer's the moment a poll was skipped or a turn ran long.
    if waited_ms(it, check).is_some_and(|waited| waited >= it.ceiling_ms) {
        // The claim is over, so whatever it managed to take goes back — the
        // reducer will not release it, because it never saw the class granted.
        release_claim(app, it, check);
        // The ceiling expiring is this check's answer, not the run's:
        // retryable, because the actionable fact is that the machine was busy
        // rather than that this check is slow.
        queue.push(Event::AcquireCeiling {
            check: check.clone(),
        });
        return Ok(());
    }

    let cost = it
        .state
        .checks
        .get(check)
        .map(|entry| entry.plan.cost)
        .unwrap_or(0);

    let mut denials: Vec<armada_core::lease::WaitingOn> = Vec::new();
    let mut taken: Vec<LeaseId> = Vec::new();
    // Anything that comes back an error is the store being broken, which is not
    // this check's problem and not something the next check will survive
    // either — so it ends the run rather than the claim.
    let granted = match kind {
        LeaseKind::Exclusive => {
            let already: Vec<LeaseId> = it.held.get(check).cloned().unwrap_or_default();
            let mut complete = true;
            for id in exclusives_of(workspace, it, check) {
                if already.contains(&id) {
                    continue;
                }
                match app.claim(&id, &mut |w| denials.push(w))? {
                    Claim::Granted(got) => taken.extend(got),
                    Claim::Waiting => {
                        complete = false;
                        break;
                    }
                }
            }
            complete
        }
        LeaseKind::CpuSlot => {
            match app.claim_slots(&workspace.id, cost, it.slots, &mut |w| denials.push(w))? {
                Claim::Granted(got) => {
                    taken.extend(got);
                    true
                }
                Claim::Waiting => false,
            }
        }
        // The scheduler asks for neither, and saying so is cheaper than a
        // silent no-op that would look like a granted claim.
        LeaseKind::Run | LeaseKind::Machine => true,
    };

    // **Recorded the moment it is held, whether or not the class is complete.**
    // A check holding `browser` while it waits for `gpu` is the ordering rule
    // working as specified; what must not happen is the shell forgetting it
    // holds `browser` between two polls, because then nothing gives it back.
    for lease in &taken {
        app.held.push(lease.clone());
    }
    it.held.entry(check.clone()).or_default().extend(taken);

    if granted {
        queue.push(Event::LeaseGranted {
            check: check.clone(),
            kind,
        });
        return Ok(());
    }

    // **Reported once per claim rather than once per poll**, and polling makes
    // that constraint sharper rather than softer: the shell now comes back
    // every half-second, and a fifteen-minute wait would otherwise put eighteen
    // hundred identical rows in the record. The gate is the reducer's own —
    // `Waiting` carries no holder until it has taken a `LeaseDenied` — so there
    // is no flag here to fall out of step with it.
    if !unreported(it, check) {
        return Ok(());
    }
    if let Some(first) = denials.first() {
        let holder = match first {
            armada_core::lease::WaitingOn::Exclusive { held_by, .. } => Some(held_by.clone()),
            armada_core::lease::WaitingOn::CpuSlot { .. }
            | armada_core::lease::WaitingOn::Run { .. } => None,
        };
        queue.push(Event::LeaseDenied {
            check: check.clone(),
            kind,
            holder: holder.clone().unwrap_or_else(|| workspace.id.clone()),
        });
        if let Some(dispatch) = it.journal.dispatches.get_mut(check) {
            dispatch.waited(kind, &kind.to_string(), holder, 0);
        }
    }
    Ok(())
}

/// Try again for every claim the scheduler is still waiting on.
///
/// **This is the half that makes a non-blocking [`acquire`] a queue rather than
/// a single try.** `Action::Acquire` is proposed once, when a check first
/// becomes ready; a denial leaves it in `Phase::Waiting`, which nothing else
/// re-proposes, so the loop comes back to it here — between the observations,
/// and with a full turn of ticking in between.
///
/// **Oldest first.** `acquisition_order` makes a cycle impossible, not a queue
/// fair, and a poll is a race: retrying in id order would let a check that has
/// waited one second beat one that has waited ten minutes, every single time
/// the lease came free. Sorting by when the wait started is FIFO by arrival,
/// and it is read out of the reducer rather than kept alongside it.
fn poll_claims<R: Run, C: Clock, F: Fetch>(
    app: &mut App<R, C, F>,
    workspace: &Workspace,
    it: &mut Loop<'_>,
    queue: &mut Vec<Event>,
) -> Result<(), ArmadaError> {
    let now = app.ctx.now.mono();
    let mut waiting: Vec<(u64, CheckId, LeaseKind)> = Vec::new();

    for (check, entry) in &it.state.checks {
        let Phase::Waiting(wait) = &entry.phase else {
            continue;
        };
        // `lease::POLL_INTERVAL_MS`, because it is the same question the
        // blocking claim answered with it: well under the renewal interval so a
        // released lease is picked up promptly, and far enough above zero that
        // a fifteen-minute wait is not a spin against `manifest.db`. The loop
        // itself turns every `TURN_MS`, so most turns skip every waiter.
        let due = it
            .polled
            .get(check)
            .map_or(0, |at| at.saturating_add(lease::POLL_INTERVAL_MS));
        if now < due {
            continue;
        }
        waiting.push((wait.since_mono, check.clone(), wait.kind));
    }

    waiting.sort();
    for (_, check, kind) in waiting {
        acquire(app, workspace, it, &check, kind, queue)?;
    }
    Ok(())
}

/// How long this check's claim has been outstanding, or `None` if it has none.
fn waited_ms(it: &Loop<'_>, check: &CheckId) -> Option<u64> {
    let entry = it.state.checks.get(check)?;
    let Phase::Waiting(wait) = &entry.phase else {
        return None;
    };
    Some(it.state.now_mono.saturating_sub(wait.since_mono))
}

/// Whether this check's claim is one nothing has reported yet.
///
/// `start_ready` opens a claim with no holder and `Event::LeaseDenied` fills
/// one in, so the absence of a holder *is* "not reported" — held in the one
/// place that already has to know, rather than in a second flag beside it.
fn unreported(it: &Loop<'_>, check: &CheckId) -> bool {
    matches!(
        it.state.checks.get(check).map(|entry| &entry.phase),
        Some(Phase::Waiting(Waiting { holder: None, .. }))
    )
}

/// Give back whatever this check took before its claim ended without being
/// granted.
fn release_claim<R: Run, C: Clock, F: Fetch>(
    app: &mut App<R, C, F>,
    it: &mut Loop<'_>,
    check: &CheckId,
) {
    for lease in it.held.remove(check).unwrap_or_default() {
        let _ = app.release(&lease);
        app.held.retain(|h| h != &lease);
    }
}

#[allow(clippy::too_many_arguments)]
fn spawn<R: Run, C: Clock, F: Fetch>(
    app: &mut App<R, C, F>,
    workspace: &Workspace,
    it: &mut Loop<'_>,
    check: &CheckId,
    argv: Vec<String>,
    env: EnvDelta,
    cwd: PathBuf,
    queue: &mut Vec<Event>,
) -> Result<(), ArmadaError> {
    // **One syscall, before each dispatch** (PLAN.md §2.3.1). A run whose
    // workspace is deleted under it must notice, because every symptom is
    // misleading: writes to an already-open log fd succeed silently into an
    // unlinked inode, `getcwd` gives `ENOENT`, and spawning gives an opaque git
    // error — so the run continues, its logs go nowhere, and every remaining
    // check reports `tool_failed` for the wrong reason.
    if fs::stat(&workspace.root) == PathStat::Missing {
        queue.push(Event::WorkspaceGone);
        return Ok(());
    }

    // **Written at dispatch, because most of it cannot be recovered.**
    let Some(entry) = it.state.checks.get(check) else {
        return Ok(());
    };
    let mut record = Dispatch::new(&entry.plan, &cwd, &entry.plan.files, it.state.now_mono);
    let held: Vec<LeaseId> = it.held.get(check).cloned().unwrap_or_default();
    for lease in &held {
        record.holding(lease.kind, &lease.key);
    }
    it.journal.dispatched(record);

    let mut environment = app.child_env();
    environment.extend(env.set.clone());
    // **The grant, applied at the last possible moment** (PLAN.md §4.7). The
    // vault holds everything this run resolved; this check gets only the names
    // its own `secrets:` listed, so a sibling with no grant sees none of them —
    // which is the assertion §4.7 rule 5 asks for by name and which
    // `crates/helm/tests/secrets.rs` makes.
    //
    // **Into the environment and never into argv**, because argv is
    // world-readable through `ps` — and the dispatch record written a few lines
    // above already carries `EnvDelta::names()` rather than values, so the run
    // directory learns that the variable was set and never what it was set to.
    environment.extend(it.vault.granted(&env.secrets));

    let request = RunRequest::new(argv, cwd)
        .env(environment)
        .stdio(StdioMode::Capture);

    match ProcessGroup::spawn(&request) {
        Ok(group) => {
            if let Some(pgid) = schedule::Pgid::new(group.pgid()) {
                queue.push(Event::ChildSpawned {
                    check: check.clone(),
                    pgid,
                });
            }
            it.children.insert(check.clone(), group);
            // Reported once the child actually exists. A check named the moment
            // it was *proposed* would appear in the running list and then
            // vanish when the spawn failed.
            it.progress.started(check.as_str());
        }
        // **The class is decided here and not in the core**, because the same
        // failure is a different class depending on who asked: `docker` missing
        // is `environment`, and a check's own `cmd:` missing is a config the
        // caller has to edit.
        Err(error) => queue.push(Event::SpawnFailed {
            check: check.clone(),
            err: match error.kind {
                SpawnErrorKind::NotFound | SpawnErrorKind::PermissionDenied => ErrClass::BadConfig,
                SpawnErrorKind::Other => ErrClass::Environment,
            },
        }),
    }
    Ok(())
}

fn collect_children(workspace: &Workspace, it: &mut Loop<'_>, queue: &mut Vec<Event>) {
    // Two passes on purpose: polling needs the child map mutably and recording
    // needs the journal, and taking both at once is the borrow the compiler is
    // right to refuse.
    let finished: Vec<(CheckId, armada_core::ctx::RunOutput)> = it
        .children
        .iter_mut()
        .filter_map(|(check, group)| group.poll().map(|output| (check.clone(), output)))
        .collect();

    for (check, output) in finished {
        it.children.remove(&check);

        // **Armada reads raw and writes scrubbed** (PLAN.md §4.7). The exit code
        // above is read off the real child and the mask is a filter on the way
        // *out*, applied at the value level before anything serializes it —
        // filtering the serialized form would fail the moment a value contained
        // a `"` or a `\`, because Armada's own encoder would have escaped it
        // first (§4.7 rule 2).
        //
        // A check granted a token and told to `set -x` is the case this exists
        // for: the run directory is a place agents are *expected* to read, which
        // is the whole reason §4.7 was written.
        let text = it
            .vault
            .mask(&format!("{}{}", output.stdout, output.stderr));
        let path = runs::log_path(&workspace.root, &it.run_id, &check);
        let _ = std::fs::write(&path, text.as_bytes());

        queue.push(Event::ChildOutput {
            check: check.clone(),
            bytes: text.len(),
        });
        // A child killed by a signal has no code. The shell convention is
        // `128 + N`, and it is the same carve-out `ARCHITECTURE.md` §1.6 makes
        // for Armada's own 130 and 141.
        let code = output
            .code
            .unwrap_or_else(|| 128 + output.signal.unwrap_or(0));
        if code != 0 {
            let _ = it.journal.failed(&check, code, &text, &it.scrub);
        }
        queue.push(Event::ChildExited {
            check: check.clone(),
            code,
        });
    }
}

/// Report the deadlines the scheduler is holding, once each.
///
/// The scheduler owns the deadline — it is computed from the plan's `timeout:`
/// and the reading at spawn — and this only says when one has passed. Reading
/// it out of `State` rather than keeping a second copy is what stops the two
/// from disagreeing about when a check is late.
///
/// **A check gets two of these, not one**, which is the whole of the SIGKILL
/// escalation: the first when its own deadline passes, and the second when
/// `schedule::KILL_GRACE_MS` has elapsed on top of the SIGTERM that answered
/// the first. The reducer escalates on the second and only on the second, so a
/// shell that reported the deadline once and then fell silent — which is what
/// this did while it guarded on `stopping.is_none()` — left a check that
/// ignores SIGTERM running forever with a scheduler that had already decided
/// to kill it. `schedule::due_at` is the same answer `next_wake` uses, so the
/// turn the loop wakes for is the turn that finds the check late.
fn collect_deadlines<R: Run, C: Clock, F: Fetch>(
    app: &App<R, C, F>,
    it: &Loop<'_>,
    queue: &mut Vec<Event>,
) {
    let now = app.ctx.now.mono();
    for (check, entry) in &it.state.checks {
        if let Phase::Running(running) = &entry.phase {
            if schedule::due_at(running).is_some_and(|due| now >= due) {
                queue.push(Event::Deadline {
                    check: check.clone(),
                });
            }
        }
    }
}

/// A run that fell out of the loop without the scheduler finishing it.
///
/// Unreachable by construction — the loop's only exit is `Finish` — and stated
/// as a `armada_bug` rather than as an `unwrap`, because a panic here would lose a
/// run that already happened along with its record.
fn never_finished() -> ArmadaError {
    ArmadaError {
        class: ErrClass::ArmadaBug,
        r#where: "check".to_string(),
        message: "the run loop ended without a verdict".to_string(),
        next_action: None,
    }
}

/// Entropy for the run id, from this process rather than from a crate.
///
/// Runs in one workspace are serialised by the run lease (PLAN.md §3.2.1), so
/// two ids in one `.armada/run/` cannot be minted in the same millisecond by
/// construction. This is belt to that braces, and mixing the pid with a
/// monotonic reading is enough for it.
fn entropy<R: Run, C: Clock, F: Fetch>(app: &App<R, C, F>) -> u64 {
    let pid = armada_manifest::posix::pid() as u64;
    pid.wrapping_mul(0x9E37_79B9_7F4A_7C15) ^ app.ctx.now.mono()
}

#[cfg(test)]
mod tests {
    use super::*;

    /// **The detached child is given the same selection, spelled once.**
    /// Rebuilding from the parsed struct rather than replaying the caller's
    /// argv is what stops `--detach` reaching the child, and what stops a
    /// `--color=always` typed at a terminal following the run into a log file.
    #[test]
    fn the_detached_child_is_given_the_same_selection_and_never_detach_itself() {
        let args = Check {
            detach: true,
            fix: true,
            all_files: true,
            wait: true,
            jobs: Some(3),
            component: Some("api".to_string()),
            ..Default::default()
        };
        let argv = detached_argv("/usr/local/bin/armada", &args);

        assert_eq!(argv[0], "/usr/local/bin/armada");
        assert_eq!(&argv[1..3], ["manifest", "check"]);
        assert!(
            !argv.iter().any(|word| word == "--detach"),
            "the child would detach again, forever: {argv:?}"
        );
        for expected in ["--json", "--all-files", "--fix", "--wait", "--component"] {
            assert!(argv.iter().any(|word| word == expected), "{expected}");
        }
        assert_eq!(
            argv.windows(2)
                .find(|pair| pair[0] == "--concurrency")
                .map(|pair| pair[1].as_str()),
            Some("3")
        );
        // The child writes to a file, so its output is an envelope rather than
        // a spinner's escape codes.
        assert_eq!(
            argv.windows(2)
                .find(|pair| pair[0] == "--color")
                .map(|pair| pair[1].as_str()),
            Some("never")
        );
    }

    /// **`--files` goes last**, because its own parser reads paths until it
    /// meets a flag — anything placed after them would be read as a path.
    #[test]
    fn the_file_list_is_the_last_thing_the_child_is_given() {
        let args = Check {
            files: vec!["src/main.rs".to_string(), "src/args.rs".to_string()],
            fix: true,
            ..Default::default()
        };
        let argv = detached_argv("armada", &args);
        let at = argv
            .iter()
            .position(|word| word == "--files")
            .expect("--files");
        assert_eq!(&argv[at + 1..], ["src/main.rs", "src/args.rs"]);
    }

    /// A bare positional survives as a positional, so `check --detach api:lint`
    /// runs the check the caller named and not every check in the repo.
    #[test]
    fn a_selector_reaches_the_child_as_a_selector() {
        let args = Check {
            selector: Some("api:lint".to_string()),
            ..Default::default()
        };
        assert_eq!(detached_argv("armada", &args).last().unwrap(), "api:lint");
    }

    /// **The fingerprint changes when a check moves and at no other time**,
    /// which is what makes "the record is rewritten on every state change" a
    /// handful of writes rather than one every twenty milliseconds. A clock
    /// reading is not a state change; a phase is.
    #[test]
    fn only_a_check_changing_phase_asks_for_a_new_record() {
        let plans = vec![Plan {
            id: CheckId::new("api:lint"),
            argv: vec!["true".to_string()],
            env: EnvDelta::default(),
            files: Vec::new(),
            timeout_ms: 900_000,
            cost: 1,
            exclusives: Vec::new(),
            needs: Vec::new(),
            log: None,
            skip: None,
            blocked: None,
        }];
        let mut state = State::new(PathBuf::from("/srv/repo"), 6, plans);
        let pending = phases(&state);

        state.now_mono = 5_000;
        assert_eq!(phases(&state), pending, "a clock reading moved the run");

        let (state, _) = schedule::step(state, Event::Started);
        assert_ne!(
            phases(&state),
            pending,
            "a check that left Pending did not ask to be recorded"
        );
    }
}
