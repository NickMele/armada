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
//! **The loop never blocks**, which is why [`ProcessGroup::poll`] exists. The
//! lease heartbeat is renewed from this loop and from no background timer,
//! precisely so that a wedged loop is a loop that stopped renewing and the
//! cold-heartbeat path reclaims it (PLAN.md §4.3).

use armada_core::config::{ResolvedCheck, ResolvedComponent, ResolvedConfig, Scope};
use armada_core::ctx::{Clock, Fetch, Run, RunRequest, SpawnErrorKind, StdioMode};
use armada_core::dispatch::{Dispatch, Journal, Scrub};
use armada_core::envelope::{CheckData, CheckDryRun, Envelope};
use armada_core::error::{ArmadaError, ConfigWhere, ErrClass, Status};
use armada_core::lease::{self, LeaseId, LeaseKind, Policy};
use armada_core::reap::PathStat;
use armada_core::run::{RunId, RunRecord};
use armada_core::schedule::{
    self, Action, CheckId, CheckResult, EnvDelta, Event, Phase, Plan, State,
};
use armada_core::select::{self, Selection, Selector};
use armada_core::template::{self, Site, Vars};
use armada_core::workspace::Workspace;
use armada_manifest::process::ProcessGroup;
use armada_manifest::{fs, git, runs};
use std::collections::BTreeMap;
use std::path::PathBuf;

use crate::app::{self, App};
use crate::args::Check;
use crate::render::progress::Progress;
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
    let (workspace, config) = load_config(app)?;
    let selection = select_checks(&config, args)?;
    let candidates = candidate_files(app, &workspace, args, &selection)?;
    let ports = assigned_ports(app, &workspace, &config)?;
    let plans = build_plans(&config, &selection, &candidates, &ports, &workspace, args)?;

    if args.dry_run {
        return dry(app, &workspace, plans).map(|e| Output::CheckDryRun(Box::new(e)));
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
        execute(app, &workspace, plans, args, progress)
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
    let declares_ports = config
        .components
        .values()
        .filter_map(|component| component.run.as_ref())
        .any(|run| !run.common().ports.is_empty());

    let block = app
        .db
        .workspaces()?
        .into_iter()
        .find(|row| row.id == workspace.id)
        .map(|row| row.ports);

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

// ---------------------------------------------------------------- the run

fn execute<R: Run, C: Clock, F: Fetch>(
    app: &mut App<R, C, F>,
    workspace: &Workspace,
    plans: Vec<Plan>,
    args: &Check,
    progress: &mut dyn Progress,
) -> Result<Envelope<CheckData>, ArmadaError> {
    let run_id = RunId::mint(app.ctx.now.wall_ms(), entropy(app));
    app.run = Some(run_id.clone());

    // Reap first, then create: the new directory must not count toward its own
    // retention budget, and reaping is reported rather than silent.
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
    // **By name, before anything runs.** The live table has a row per check
    // from its first frame, so a run that is waiting on a lease still shows
    // what it is waiting to run rather than an empty count.
    let ids: Vec<&str> = plans.iter().map(|plan| plan.id.as_str()).collect();
    progress.begin(&ids, app.ctx.now.mono());
    drop(ids);

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
        ceiling_ms: app.machine.acquire_ceiling_ms(),
        held: BTreeMap::new(),
        slots: app.machine.cpu_slots,
    };

    drive(app, workspace, &mut loop_state)?;

    let record = RunRecord {
        schema_version: armada_core::envelope::SCHEMA_VERSION,
        run_id: run_id.clone(),
        workspace: workspace.id.clone(),
        started_at: app.ctx.now.wall_rfc3339(),
        state: loop_state.state,
        journal: loop_state.journal,
    };
    runs::write_record(&workspace.root, &record)?;

    let (status, error) = loop_state
        .finish
        .unwrap_or((Status::Failed, Some(never_finished())));

    let mut reaped_runs: Vec<String> = reaped.iter().map(RunId::to_string).collect();
    reaped_runs.extend(skipped);

    let data = CheckData {
        run_id: run_id.to_string(),
        results: loop_state.rows.values().map(Into::into).collect(),
        reaped_runs,
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

        // Observe: children first, then deadlines, then the clock. A child that
        // finished is more informative than a deadline that has not.
        collect_children(workspace, it, &mut queue);
        collect_deadlines(app, it, &mut queue);
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
                .finished(result.id.as_str(), result.status, detail);
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

/// **The ordering rule, performed rather than re-decided.**
///
/// `lease::acquisition_order` is the single statement of "exclusives first, in
/// sorted name order, then slots" — the shell asks it which concrete leases a
/// class means and takes them in the order it returns. Sorting here as well
/// would be a second implementation of the property that makes a cross-workspace
/// cycle impossible.
fn acquire<R: Run, C: Clock, F: Fetch>(
    app: &mut App<R, C, F>,
    workspace: &Workspace,
    it: &mut Loop<'_>,
    check: &CheckId,
    kind: LeaseKind,
    queue: &mut Vec<Event>,
) -> Result<(), ArmadaError> {
    let cost = it
        .state
        .checks
        .get(check)
        .map(|entry| entry.plan.cost)
        .unwrap_or(0);

    // The wait is reported once per claim rather than once per poll: the claim
    // loop reports on every turn, and a fifteen-minute wait would otherwise put
    // eighteen hundred identical rows in the record.
    let mut denials: Vec<armada_core::lease::WaitingOn> = Vec::new();
    let outcome: Result<Vec<LeaseId>, ArmadaError> = match kind {
        LeaseKind::Exclusive => {
            let mut taken = Vec::new();
            let mut failure = None;
            for id in exclusives_of(workspace, it, check) {
                match app.acquire_reporting(
                    id.clone(),
                    Policy::Block,
                    Some(it.ceiling_ms),
                    &mut |w| denials.push(w),
                ) {
                    Ok(held) => taken.push(held),
                    Err(error) => {
                        failure = Some(error);
                        break;
                    }
                }
            }
            match failure {
                Some(error) => Err(error),
                None => Ok(taken),
            }
        }
        LeaseKind::CpuSlot => app.acquire_slots(
            &workspace.id,
            cost,
            it.slots,
            Some(it.ceiling_ms),
            &mut |w| denials.push(w),
        ),
        // The scheduler asks for neither, and saying so is cheaper than a
        // silent no-op that would look like a granted claim.
        LeaseKind::Run | LeaseKind::Machine => Ok(Vec::new()),
    };

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

    match outcome {
        Ok(taken) => {
            for lease in &taken {
                app.held.push(lease.clone());
            }
            it.held.entry(check.clone()).or_default().extend(taken);
            queue.push(Event::LeaseGranted {
                check: check.clone(),
                kind,
            });
            Ok(())
        }
        // The ceiling expiring is this check's answer, not the run's:
        // retryable, because the actionable fact is that the machine was busy
        // rather than that this check is slow.
        Err(error) if error.class == ErrClass::Aborted => {
            queue.push(Event::AcquireCeiling {
                check: check.clone(),
            });
            Ok(())
        }
        // Anything else is the store being broken, which is not this check's
        // problem and not something the next check will survive either.
        Err(error) => Err(error),
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

        let text = format!("{}{}", output.stdout, output.stderr);
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
fn collect_deadlines<R: Run, C: Clock, F: Fetch>(
    app: &App<R, C, F>,
    it: &Loop<'_>,
    queue: &mut Vec<Event>,
) {
    let now = app.ctx.now.mono();
    for (check, entry) in &it.state.checks {
        if let Phase::Running(running) = &entry.phase {
            if now >= running.deadline_mono && running.stopping.is_none() {
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
