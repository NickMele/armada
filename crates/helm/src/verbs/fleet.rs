//! `armada fleet <verb>` — the sequencing, and nothing that decides anything.
//!
//! Every rule this file appears to apply was decided somewhere else: which
//! workflow a task is, what a ceiling is, what a Job's argv looks like and what
//! a turn cost are all `armada_core::fleet`'s; the Job index, the worktrees, the
//! inbox and the subprocesses are `armada_fleet`'s. What lives here is the order
//! the adapter calls go in (`ARCHITECTURE.md` §1.3).
//!
//! **These verbs are machine-scoped.** They run before workspace resolution and
//! before `app::build`, for the reason `commands/fleet/ls.md` states outright:
//! `ls` "does not need the repository the Jobs branched from". A Fleet routed
//! through workspace resolution would refuse to list the fleet from any
//! directory that is not one of its worktrees, which is most directories.
//!
//! **`spawn` returns while the Drone is still working**, and every other verb is
//! written around that. Nothing updates a Job's record when its turn ends — a
//! Drone reports to nobody — so the record holds what was true when a verb last
//! wrote it, and the truth is the transcript plus the process table.
//! [`armada_core::fleet::job::observe`] reconciles the two, in one place, and
//! `ls` renders that while `kill` and `answer` persist it.

use armada_core::ctx::{Clock, Run};
use armada_core::envelope::{
    AnswerData, BoardData, Disposition, Envelope, FleetLsData, InboxData, InboxRow, JobRow,
    KillData, Killed, SpawnData,
};
use armada_core::error::{ArmadaError, ErrClass, Status};
use armada_core::fleet::classify::Classification;
use armada_core::fleet::drone::Reading;
use armada_core::fleet::job::{self, Handle, Job, Observed, Spend};
use armada_core::fleet::workflow::{self, Workflow};
use armada_core::fleet::{drone as argv, JobState, Verdict};
use armada_fleet::drone;
use armada_fleet::jobs::Store;
use armada_fleet::{home, inbox, manifest, own, worktree};
use armada_guild::layout::Guild;
use std::path::{Path, PathBuf};

use crate::args::Spawn;
use crate::verbs::Output;

/// Everything a Fleet verb needs from the machine, gathered at the entrypoint.
///
/// **`$HOME` and the current directory arrive as values** — nothing below the
/// entrypoint reads either (`ARCHITECTURE.md` §1.4) — and that is also what lets
/// the whole suite point `armada fleet spawn` at a `TempDir` instead of at
/// somebody's real `~/.armada/`.
pub struct Where {
    /// `$HOME`, for writing a path the way a person writes one.
    pub home: PathBuf,
    /// `~/.armada/`.
    pub armada_home: PathBuf,
    /// Where the command was typed.
    pub cwd: PathBuf,
    /// The `armada` binary itself, for the Manifest verbs run in a worktree.
    pub exe: PathBuf,
    /// This boot.
    ///
    /// **Required rather than optional**, for the reason `app::build` requires
    /// it: without one, every Drone handle looks stale across a reboot, so
    /// Armada would either refuse to stop its own Drones forever or signal a
    /// recycled pid. Refusing to run is better than either.
    pub boot_id: String,
}

impl Where {
    /// The Job index.
    pub fn store(&self) -> Store {
        Store::at(&self.armada_home)
    }

    /// The inbox file.
    pub fn inbox(&self) -> PathBuf {
        home::inbox(&self.armada_home)
    }

    /// Where a Job's transcript is.
    pub fn stream(&self, uuid: &str) -> PathBuf {
        home::stream(&self.armada_home, uuid)
    }

    /// A path as a person writes it.
    pub fn shown(&self, path: &Path) -> String {
        home::tilde(path, &self.home)
    }

    /// Turn a `~/…` back into a real path.
    pub fn expand(&self, shown: &str) -> PathBuf {
        match shown.strip_prefix("~/") {
            Some(rest) => self.home.join(rest),
            None => PathBuf::from(shown),
        }
    }
}

/// What a Job is really doing, worked out from its transcript and the process
/// table.
///
/// **One function, called by every verb that needs the answer.** `ls` renders
/// it, `kill` and `answer` persist it, and nothing computes it a second way —
/// which is what stops `ls` and `kill` disagreeing about whether a Job is
/// stalled.
fn look<R: Run>(run: &R, place: &Where, record: &Job, now_ms: u64) -> (Observed, Reading) {
    let reading = drone::transcript(&place.stream(&record.uuid));
    // **The process table is only consulted for a Job that could still be
    // running.** A finished Job costs no `ps`, which is what keeps `armada
    // fleet ls --all` cheap on a machine with a long history.
    let alive = !record.state.is_over()
        && drone::alive(
            run,
            &place.armada_home,
            record.drone.as_ref(),
            &place.boot_id,
        );
    let observed = job::observe(
        record,
        reading.spend,
        reading.turns.len(),
        reading.last().is_some_and(|turn| turn.is_error),
        alive,
        record.run_time_ms(now_ms),
    );
    (observed, reading)
}

/// Write an observation back into the record.
///
/// **The verbs that change something persist what they saw**, so that a Job
/// which reached a ceiling while nobody was looking is `PAUSED` on disk rather
/// than only on a screen. `ls` deliberately does not do this: a read verb that
/// wrote would make `armada fleet ls | head` a mutation.
fn settle<C: Clock>(
    record: &mut Job,
    observed: &Observed,
    place: &Where,
    now: &C,
) -> Result<(), ArmadaError> {
    let was = record.state;
    record.spend = observed.spend;
    record.state = observed.state;

    if let Some(ceiling) = observed.ceiling {
        // **Exhaustion is an outcome, never a silent stop** (PLAN.md §14.3):
        // the Job records what it spent and where it reached, and is raised.
        if record.verdict != Some(Verdict::NeedsHuman) {
            record.verdict = Some(Verdict::NeedsHuman);
            raise(
                place,
                now,
                record,
                inbox::Kind::NeedsHuman,
                &format!(
                    "reached its {} ceiling on the {} step",
                    ceiling.word(),
                    record.step
                ),
            )?;
        }
    } else if observed.state == JobState::Stalled && was != JobState::Stalled {
        // **Raised once, when it first stalls.** A stall re-raised on every `ls`
        // would turn the inbox into a poll, and a diluted signal gets ignored at
        // the moment it matters (PLAN.md §15.4).
        raise(
            place,
            now,
            record,
            inbox::Kind::Blocked,
            "its Drone stopped without finishing a turn",
        )?;
    }
    Ok(())
}

// ----------------------------------------------------------------------- spawn

/// `armada fleet spawn` — classify, worktree, `manifest init`, start a Drone,
/// **return**.
///
/// **It does not wait for the Drone, and that is the point of the verb.** A
/// `spawn` that ran the turn to completion could only ever run one Job at a
/// time, and running several is the whole of Fleet. What comes back is the
/// handle — a uuid, a name and a process group — and everything the Job goes on
/// to do is read afterwards from its transcript by `armada fleet ls`.
///
/// **The uuid is minted and the record written before the worktree exists.** The
/// durable handle exists before the process does, so a spawn that dies halfway
/// leaves a Job `armada fleet kill` can still find and release — which is the
/// whole reason PLAN.md §14.1 puts the minting first.
pub fn spawn<R: Run, C: Clock>(
    run: &R,
    now: &C,
    place: &Where,
    options: &Spawn,
) -> Result<Output, ArmadaError> {
    let repo_root = repository(run, place, options)?;
    let repo = home::repo_name(&repo_root);

    let (classification, classify_ms) = classify(run, now, &repo_root, options)?;
    let workflow = read_workflow(place, &classification.workflow)?;
    let budget = workflow::override_budget(workflow.budget, &options.budget)?;

    let store = place.store();
    let wanted = options
        .name
        .clone()
        .unwrap_or_else(|| job::derive_name(&options.task));
    // **A name a person passed is refused when it is taken; a derived one is
    // numbered.** The flag is a statement about which Job this is, and silently
    // renaming it would answer a different question than the one asked.
    let name = match &options.name {
        Some(named) if store.name_is_taken(named)? => {
            return Err(ArmadaError {
                class: ErrClass::BadInvocation,
                r#where: named.clone(),
                message: format!("a live Job is already called `{named}`"),
                next_action: Some("pick another --name, or kill that one".to_string()),
            })
        }
        _ => store.free_name(&wanted)?,
    };

    let uuid = job::mint_uuid(&format!(
        "{repo}|{name}|{}|{}",
        now.wall_ms(),
        armada_manifest::posix::pid()
    ));
    let path = home::worktree(&place.armada_home, &repo, &name);
    let branch = worktree::branch_for(&name);
    let step = workflow.first_step().id.clone();

    let mut record = Job {
        uuid: uuid.clone(),
        name: name.clone(),
        workflow: workflow.name.clone(),
        confidence: classification.confidence,
        repo: repo.clone(),
        repo_root: place.shown(&repo_root),
        worktree: place.shown(&path),
        branch: branch.clone(),
        port_block: None,
        budget,
        state: JobState::Queued,
        step: step.clone(),
        verdict: None,
        drone: None,
        created_at: now.wall_rfc3339(),
        created_ms: now.wall_ms(),
        spend: Spend::default(),
        task: options.task.clone(),
    };

    if options.dry_run {
        // Nothing is written, nothing is claimed, and no record is left behind:
        // a preview that minted a Job would be the destructive path it was
        // previewing (`ARCHITECTURE.md` §2.1.2).
        return Ok(envelope(&record, Status::Skipped, classify_ms, 0, place));
    }

    // **Recorded before anything is created.** Everything after this line can
    // fail, and every one of those failures leaves a Job on disk rather than an
    // orphaned worktree holding a port block nobody can name.
    store.save(&record)?;

    let started = now.mono();
    if let Err(error) = worktree::add(run, &repo_root, &path, &branch) {
        // **A failed spawn cleans up after itself** (`commands/fleet/spawn.md`).
        // A half-created worktree holding a claimed block is released before the
        // error returns — and if any of that also fails, ownership is recorded
        // machine-globally and `armada manifest clean --all` reclaims the rest.
        let _ = manifest::clean(run, &place.exe, &path);
        let _ = worktree::remove(run, &repo_root, &path);
        let _ = worktree::delete_branch(run, &repo_root, &branch);
        record.state = JobState::Aborted;
        record.verdict = Some(Verdict::Failed);
        store.save(&record)?;
        return Err(error);
    }
    record.port_block = manifest::init(run, &place.exe, &path)?;
    record.state = JobState::Running;
    store.save(&record)?;
    let prepare_ms = now.mono().saturating_sub(started);

    // **The Drone is started detached and `spawn` returns.** That is the whole
    // purpose of Fleet — five Jobs at once with one thing to watch — and it is
    // why nothing here waits, reads a ledger, or reports a spend: the Drone is
    // still working when this function ends.
    record.drone = Some(start_drone(
        run,
        place,
        &record,
        &path,
        argv::spawn_argv(&uuid, &prompt(&workflow, &step, &options.task)),
    )?);
    store.save(&record)?;

    Ok(envelope(
        &record,
        Status::Ready,
        classify_ms,
        prepare_ms,
        place,
    ))
}

/// Start a Drone for a Job, and record its group where Manifest's reaper looks.
///
/// **Two records of one process group, and both are needed.** The Job's own
/// carries the handle so `armada fleet ls` and `kill` can reach it without
/// opening the machine-global store; the `owned` row is what makes an *orphaned*
/// Drone — Armada died, the Drone did not — reapable by the same pass that
/// reaps an orphaned service, which is the whole reason not to invent a second
/// mechanism.
fn start_drone<R: Run>(
    run: &R,
    place: &Where,
    record: &Job,
    worktree: &Path,
    argv: Vec<String>,
) -> Result<Handle, ArmadaError> {
    let handle = drone::start(
        run,
        worktree,
        &place.stream(&record.uuid),
        argv,
        drone::job_env(&record.name, &record.uuid),
        &place.boot_id,
    )?;
    // Best-effort: the Job's own record already carries the handle, so a
    // workspace that will not resolve costs the machine-global backstop and not
    // the Job.
    let _ = own::record_drone(run, &place.armada_home, worktree, &handle);
    Ok(handle)
}

/// Which repository this Job branches from.
///
/// **`environment`, not `bad_config`**, when there is no repository: nothing is
/// wrong with any file, and the answer is to run the command somewhere else.
fn repository<R: Run>(run: &R, place: &Where, options: &Spawn) -> Result<PathBuf, ArmadaError> {
    let from = match &options.at {
        Some(at) => place.cwd.join(at),
        None => place.cwd.clone(),
    };
    armada_manifest::git::root(run, &from).ok_or_else(|| ArmadaError {
        class: ErrClass::Environment,
        r#where: from.display().to_string(),
        message: "a Job needs a git repository to branch from".to_string(),
        next_action: Some("run it inside a repository, or pass -C <path>".to_string()),
    })
}

/// The workflow, classified or named.
fn classify<R: Run, C: Clock>(
    run: &R,
    now: &C,
    repo_root: &Path,
    options: &Spawn,
) -> Result<(Classification, Option<u64>), ArmadaError> {
    match &options.workflow {
        // **No call at all for an override.** Classification is one cheap call
        // per spawn and its cost is the one that compounds; spending it to
        // confirm an answer the caller already gave would be the one avoidable
        // token in the whole verb.
        Some(named) => Ok((Classification::overridden(named), None)),
        None => {
            let started = now.mono();
            let classified = drone::classify(run, repo_root, &options.task)?;
            Ok((classified, Some(now.mono().saturating_sub(started))))
        }
    }
}

/// Read one workflow out of the guild.
fn read_workflow(place: &Where, name: &str) -> Result<Workflow, ArmadaError> {
    let guild = Guild::at(&place.armada_home);
    let relative = format!("workflows/{name}.yml");
    let path = guild.path(&relative);
    let text = std::fs::read_to_string(&path).map_err(|_| ArmadaError {
        class: ErrClass::BadInvocation,
        r#where: name.to_string(),
        message: format!("no workflow called `{name}` in your guild"),
        next_action: Some(format!(
            "`armada guild init` writes the starters: {}",
            workflow::STARTERS.join(", ")
        )),
    })?;
    workflow::parse(&text, &relative)
}

/// What the Drone is asked to do.
///
/// **The step's skill by name, not its prose.** A skill is a named grant plus a
/// pointer to a markdown file (`glossary.md`), and Armada never parses that file
/// — the Drone resolves the name in its own worktree, which is what makes the
/// repo's version win a collision (PLAN.md §14.5).
fn prompt(workflow: &Workflow, step: &str, task: &str) -> String {
    let skill = workflow
        .steps
        .iter()
        .find(|candidate| candidate.id == step)
        .and_then(|candidate| candidate.skill.clone());
    match skill {
        Some(skill) => format!(
            "Use the `{skill}` skill for the `{step}` step of the `{}` workflow.\n\n\
             Task: {task}",
            workflow.name
        ),
        None => format!(
            "Work the `{step}` step of the `{}` workflow.\n\nTask: {task}",
            workflow.name
        ),
    }
}

fn raise<C: Clock>(
    place: &Where,
    now: &C,
    record: &Job,
    kind: inbox::Kind,
    body: &str,
) -> Result<(), ArmadaError> {
    let at_ms = now.wall_ms();
    inbox::raise(
        &place.inbox(),
        &job::mint_uuid(&format!("{}|{at_ms}|{body}", record.uuid)),
        &record.name,
        kind,
        &now.wall_rfc3339(),
        at_ms,
        body,
    )?;
    Ok(())
}

fn envelope(
    record: &Job,
    status: Status,
    classify_ms: Option<u64>,
    prepare_ms: u64,
    place: &Where,
) -> Output {
    let _ = place;
    Output::Spawn(Box::new(Envelope::ok(
        "fleet spawn",
        None,
        status,
        SpawnData {
            uuid: record.uuid.clone(),
            name: record.name.clone(),
            workflow: record.workflow.clone(),
            confidence: record.confidence,
            worktree: record.worktree.clone(),
            branch: record.branch.clone(),
            port_block: record.port_block,
            budget: record.budget,
            step: record.step.clone(),
            state: record.state,
            classify_ms,
            prepare_ms,
            pgid: record.drone.as_ref().map(|drone| drone.pgid),
        },
    )))
}

// -------------------------------------------------------------------------- ls

/// `armada fleet ls` — read-only, and it never resumes or interrupts a Job.
///
/// **Every column comes from data Claude Code already emits** (PHASES.md §9.1
/// F2). Nothing here estimates a cost, a token count or a remaining budget.
///
/// **What it reports is an observation, not the record.** A Drone runs detached
/// and updates nothing when its turn ends, so the state on disk is what a verb
/// last wrote — and `ls` is the thing that looks at the transcript and the
/// process table and says what is actually true. It writes none of it back: a
/// read verb that mutated would make `armada fleet ls | head` a change to the
/// fleet.
pub fn ls<R: Run, C: Clock>(
    run: &R,
    now: &C,
    place: &Where,
    all: bool,
    needs_attention: bool,
) -> Result<Output, ArmadaError> {
    let entries = inbox::read(&place.inbox())?;
    let wall = now.wall_ms();

    let mut rows: Vec<JobRow> = Vec::new();
    for record in place.store().all()? {
        if !all && record.state.is_over() {
            continue;
        }
        let (observed, _) = look(run, place, &record, wall);
        let waiting = inbox::open_for(&entries, &record.name);
        let wants_you = observed.state.needs_a_person() || waiting.is_some();
        if needs_attention && !wants_you {
            continue;
        }
        let run_time = record.run_time_ms(wall);
        rows.push(JobRow {
            uuid: record.uuid.clone(),
            name: record.name.clone(),
            workflow: record.workflow.clone(),
            state: observed.state,
            detail: detail(&record, observed.state, waiting),
            runtime_s: run_time / 1_000,
            cost_usd: observed.spend.cost_usd,
            tokens: observed.spend.tokens,
            turns: observed.spend.turns,
            budget_remaining: job::remaining(&record.budget, &observed.spend, run_time),
            needs_attention: wants_you,
        });
    }

    let needs_you = rows.iter().filter(|row| row.needs_attention).count();
    let spent_usd = rows.iter().map(|row| row.cost_usd).sum();
    let running = rows.iter().any(|row| row.state == JobState::Running);

    Ok(Output::FleetLs(Box::new(Envelope::ok(
        "fleet ls",
        None,
        // **A progress state, which is what a read verb is allowed** (PLAN.md
        // §3.1). `ls` reports rather than judges: it exits 0 whenever the index
        // is readable, so the word describes the fleet and not the command.
        if running { Status::Running } else { Status::Ok },
        FleetLsData {
            results: rows,
            needs_you,
            spent_usd,
        },
    ))))
}

/// The one thing a state word cannot say: which step, and what it is waiting on.
fn detail(record: &Job, state: JobState, waiting: Option<&inbox::Entry>) -> String {
    match waiting {
        Some(entry) => entry.body.clone(),
        None if state == JobState::Queued => String::new(),
        None => record.step.clone(),
    }
}

// ----------------------------------------------------------------------- board

/// `armada fleet board` — the two facts needed to enter a Job.
///
/// **It does not attach and it does not stop a running Drone first.** Boarding
/// hands you the conversation; if a turn is in flight, resuming interactively
/// while it runs is a conflict, and `ls` is where you check for that
/// (`commands/fleet/board.md`).
pub fn board(place: &Where, handle: &str) -> Result<Output, ArmadaError> {
    let record = place.store().find(handle)?;
    Ok(Output::Board(Box::new(Envelope::ok(
        "fleet board",
        None,
        Status::Ok,
        BoardData {
            job: record.name.clone(),
            worktree: record.worktree.clone(),
            uuid: record.uuid.clone(),
            branch: record.branch.clone(),
            command: argv::board_argv(&record.uuid).join(" "),
        },
    ))))
}

// ------------------------------------------------------------------------ kill

/// `armada fleet kill` — stop the Drone, clean, drop the worktree, mark the Job
/// ended.
///
/// **Four steps, in this order, and the order is the point**
/// (`commands/fleet/kill.md`). The Drone goes first because it is still working:
/// a live Drone mid-`docker compose up` would otherwise race the teardown of the
/// very resources it is creating, and lose. Cleaning before removing means
/// resources are released while the config that describes them is still present.
///
/// **If the order is ever reversed, nothing is lost.** Ownership is recorded
/// machine-globally — including the Drone's own process group — so `armada
/// manifest clean --all` still reclaims it afterwards. That safety net is the
/// reason Manifest sits underneath Fleet.
pub fn kill<R: Run, C: Clock>(
    run: &R,
    now: &C,
    place: &Where,
    handle: Option<&str>,
    keep_branch: bool,
    keep_worktree: bool,
) -> Result<Output, ArmadaError> {
    let store = place.store();
    let wall = now.wall_ms();
    let targets = match handle {
        Some(handle) => vec![store.find(handle)?],
        // **`--all-finished` asks the observation, not the record.** A Job whose
        // Drone finished while nobody was looking is finished, and a record that
        // still says `RUNNING` is only what a verb last wrote.
        None => store
            .all()?
            .into_iter()
            .filter(|record| {
                let (observed, _) = look(run, place, record, wall);
                observed.state.is_over()
                    || matches!(observed.state, JobState::Paused | JobState::Stalled)
            })
            .collect(),
    };

    let mut results: Vec<Killed> = Vec::new();
    for mut record in targets {
        let path = place.expand(&record.worktree);

        // **What it was doing is recorded before it is ended.** A Job that
        // stalled or hit a ceiling while nobody was looking has that written
        // down and raised here, because after this it is `ABORTED` and the
        // observation is no longer derivable.
        let (observed, _) = look(run, place, &record, wall);
        settle(&mut record, &observed, place, now)?;

        // **Step one: the Drone.** It is still working, and everything below
        // takes away what it is working with.
        let stopped = drone::stop(
            run,
            &place.armada_home,
            record.drone.as_ref(),
            &place.boot_id,
        );
        if let Some(handle) = &record.drone {
            own::forget_drone(run, &place.armada_home, &path, handle.pgid);
        }
        record.drone = None;

        // **Step two**: resources are released while the config that describes
        // them is still present.
        let cleaned = manifest::clean(run, &place.exe, &path)?;
        let mut failure = cleaned.error;
        if stopped == drone::Stopped::Survived {
            // A group still alive after SIGKILL is a real leak, and a reclaim
            // Armada could not complete must never be silent.
            failure.get_or_insert(ArmadaError {
                class: ErrClass::ToolFailed,
                r#where: record.name.clone(),
                message: "the Drone was still running after SIGKILL".to_string(),
                next_action: Some("look for it by hand; `armada fleet ls` names it".to_string()),
            });
        }

        // **`git worktree remove` is run from the repository, not the
        // worktree.** git refuses to remove the tree it is standing in, and by
        // this point the record is the only thing that knows where the
        // repository was.
        let repo_root = place.expand(&record.repo_root);

        // **A directory that is already gone is not a failure.** A Job whose
        // worktree somebody deleted by hand is exactly the Job the durable
        // record exists for (PLAN.md §14.1).
        let disposition = match (keep_worktree, path.exists()) {
            (_, false) => Disposition::Gone,
            (true, true) => Disposition::Kept,
            (false, true) => match worktree::remove(run, &repo_root, &path) {
                Ok(()) => Disposition::Removed,
                Err(error) => {
                    // Reported, never raised. The Job is ended either way, and a
                    // `kill` that bailed out here would need a second `kill` to
                    // do the same thing again.
                    failure.get_or_insert(error);
                    Disposition::Kept
                }
            },
        };

        let branch = if keep_branch || disposition != Disposition::Removed {
            Disposition::Kept
        } else {
            match worktree::delete_branch(run, &repo_root, &record.branch) {
                Ok(()) => Disposition::Removed,
                // The branch may already be gone, or the repository may be. A
                // Job cannot be un-killed, so this is reported and not raised.
                Err(_) => Disposition::Gone,
            }
        };

        results.push(Killed {
            job: record.name.clone(),
            uuid: record.uuid.clone(),
            released: cleaned.released,
            port_block: record.port_block,
            worktree: disposition,
            worktree_path: record.worktree.clone(),
            branch,
            branch_name: record.branch.clone(),
            error: failure,
        });

        // **The Job is marked ended whatever happened above.** A `kill` that
        // left a Job live because one container refused to stop would need a
        // second `kill` to do the same thing again.
        //
        // Its spend is settled from the transcript on the way out, because the
        // transcript is about to be the only thing left that knows.
        record.spend = drone::transcript(&place.stream(&record.uuid)).spend;
        record.state = JobState::Aborted;
        record.port_block = None;
        store.save(&record)?;
    }

    let error = results.iter().find_map(|killed| killed.error.clone());
    let data = KillData { results };
    Ok(Output::Kill(Box::new(match error {
        Some(error) => Envelope::failed("fleet kill", None, error, data),
        None => Envelope::ok("fleet kill", None, Status::Clean, data),
    })))
}

// ---------------------------------------------------------------- inbox/answer

/// `armada fleet inbox` — what the fleet needs from you.
///
/// **Reading does not mark anything answered**; [`answer`] does
/// (`commands/fleet/inbox.md`).
pub fn inbox<C: Clock>(
    now: &C,
    place: &Where,
    job: Option<&str>,
    all: bool,
) -> Result<Output, ArmadaError> {
    let wall = now.wall_ms();
    let rows: Vec<InboxRow> = inbox::read(&place.inbox())?
        .into_iter()
        .filter(|entry| job.is_none_or(|name| entry.job == name))
        .filter(|entry| all || entry.is_open())
        .map(|entry| InboxRow {
            uuid: entry.uuid,
            job: entry.job,
            kind: entry.kind.word().to_string(),
            raised_at: entry.raised_at,
            waiting_s: wall.saturating_sub(entry.raised_ms) / 1_000,
            body: entry.body,
            answered: entry.answered,
        })
        .collect();

    let open = rows.iter().filter(|row| row.answered.is_none()).count();
    Ok(Output::Inbox(Box::new(Envelope::ok(
        "fleet inbox",
        None,
        // **An empty inbox is a normal state, not a failure.** A caller checks
        // for an empty result set rather than reading the exit code.
        Status::Ok,
        InboxData {
            results: rows,
            open,
        },
    ))))
}

/// `armada fleet answer` — close the entry, and resume the Job with it.
///
/// **The budget is not reset.** An answer is a continuation rather than a new
/// run, and resetting the ceiling here would make budgets unenforceable for any
/// Job that asks a question (`commands/fleet/answer.md`). The resumed session
/// appends its `result` to the same transcript, so continuing costs what it
/// costs and the sum keeps counting.
///
/// **The resumed Drone is detached exactly as a fresh one is.** An answer starts
/// a turn; it does not wait for one. A Job you answered before lunch is working
/// while you are out, which is the behaviour the whole verb exists for.
pub fn answer<R: Run, C: Clock>(
    run: &R,
    now: &C,
    place: &Where,
    handle: &str,
    said: &str,
) -> Result<Output, ArmadaError> {
    let store = place.store();
    let mut record = store.find(handle)?;
    let entries = inbox::read(&place.inbox())?;
    let Some(entry) = inbox::open_for(&entries, &record.name) else {
        return Err(ArmadaError {
            class: ErrClass::BadInvocation,
            r#where: record.name.clone(),
            message: format!("`{}` has nothing open to answer", record.name),
            next_action: Some("`armada fleet inbox` lists what is waiting".to_string()),
        });
    };

    // **Refused before the entry is closed.** A Job whose rope has run out is
    // not continued by answering it: `on_exhausted: needs_human` means a person
    // decides what happens next, and silently resuming past a ceiling is how a
    // budget stops being one.
    let (observed, _) = look(run, place, &record, now.wall_ms());
    if let Some(ceiling) = observed.ceiling {
        // Persisted and raised on the way out, so the ceiling is a durable fact
        // rather than something this invocation noticed and forgot.
        settle(&mut record, &observed, place, now)?;
        store.save(&record)?;
        return Err(ArmadaError {
            class: ErrClass::BadInvocation,
            r#where: record.name.clone(),
            message: format!("`{}` reached its {} ceiling", record.name, ceiling.word()),
            next_action: Some(format!(
                "`armada fleet board {}` to take it over, or kill it",
                record.name
            )),
        });
    }

    inbox::answer(&place.inbox(), &entry.uuid, said)?;

    // A Drone left over from the previous turn is stopped first: two Drones on
    // one session is two writers on one transcript.
    let _ = drone::stop(
        run,
        &place.armada_home,
        record.drone.as_ref(),
        &place.boot_id,
    );
    if let Some(previous) = &record.drone {
        own::forget_drone(
            run,
            &place.armada_home,
            &place.expand(&record.worktree),
            previous.pgid,
        );
    }

    record.spend = observed.spend;
    record.verdict = None;
    record.state = JobState::Running;
    record.drone = Some(start_drone(
        run,
        place,
        &record,
        &place.expand(&record.worktree),
        argv::resume_argv(&record.uuid, said),
    )?);
    store.save(&record)?;

    Ok(Output::Answer(Box::new(Envelope::ok(
        "fleet answer",
        None,
        Status::Ok,
        AnswerData {
            job: record.name.clone(),
            uuid: record.uuid.clone(),
            entry: entry.uuid.clone(),
            answer: said.to_string(),
            state: record.state,
            budget_remaining: job::remaining(
                &record.budget,
                &record.spend,
                record.run_time_ms(now.wall_ms()),
            ),
            pgid: record.drone.as_ref().map(|drone| drone.pgid),
        },
    ))))
}
