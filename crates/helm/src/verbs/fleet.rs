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

use armada_core::ctx::{Clock, Run};
use armada_core::envelope::{
    AnswerData, BoardData, Disposition, Envelope, FleetLsData, InboxData, InboxRow, JobRow,
    KillData, Killed, SpawnData,
};
use armada_core::error::{ArmadaError, ErrClass, Status};
use armada_core::fleet::classify::Classification;
use armada_core::fleet::job::{self, Job, Spend};
use armada_core::fleet::workflow::{self, Workflow};
use armada_core::fleet::{drone as argv, JobState, Verdict};
use armada_fleet::drone::{self, Ended};
use armada_fleet::jobs::Store;
use armada_fleet::{home, inbox, manifest, worktree};
use armada_guild::layout::Guild;
use std::path::{Path, PathBuf};
use std::time::Duration;

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

    /// A path as a person writes it.
    pub fn shown(&self, path: &Path) -> String {
        home::tilde(path, &self.home)
    }
}

// ----------------------------------------------------------------------- spawn

/// `armada fleet spawn` — classify, worktree, `manifest init`, budgeted turn.
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
        created_at: now.wall_rfc3339(),
        created_ms: now.wall_ms(),
        spend: Spend::default(),
        task: options.task.clone(),
    };

    if options.dry_run {
        // Nothing is written, nothing is claimed, and no record is left behind:
        // a preview that minted a Job would be the destructive path it was
        // previewing (`ARCHITECTURE.md` §2.1.2).
        return Ok(envelope(
            &record,
            Status::Skipped,
            classify_ms,
            0,
            None,
            place,
        ));
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

    // **One bounded headless turn**, and the deadline is the workflow's own
    // wall clock so a Drone cannot outlive the ceiling it was given.
    let ended = drone::turn(
        run,
        &path,
        argv::spawn_argv(&uuid, &prompt(&workflow, &step, &options.task)),
        drone::job_env(&name, &uuid),
        Duration::from_millis(budget.wall_clock_ms),
    )?;
    let spend = ended.spend();
    record.spend.add(&spend);
    settle(&mut record, &ended, now, place)?;
    store.save(&record)?;

    Ok(envelope(
        &record,
        Status::Ready,
        classify_ms,
        prepare_ms,
        Some(spend),
        place,
    ))
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

/// What the Job is after its turn.
///
/// **Exhaustion is an outcome, never a silent stop** (PLAN.md §14.3): the Job
/// records what it spent and where it reached, and it is raised to the inbox.
fn settle<C: Clock>(
    record: &mut Job,
    ended: &Ended,
    now: &C,
    place: &Where,
) -> Result<(), ArmadaError> {
    let run_time = record.run_time_ms(now.wall_ms());
    if let Some(ceiling) = job::exhausted(&record.budget, &record.spend, run_time) {
        record.state = JobState::Paused;
        record.verdict = Some(Verdict::NeedsHuman);
        return raise(
            place,
            now,
            record,
            inbox::Kind::NeedsHuman,
            &format!(
                "reached its {} ceiling on the {} step",
                ceiling.word(),
                record.step
            ),
        );
    }

    match ended {
        Ended::Turn(_) => record.state = JobState::Running,
        // Armada's own deadline. The wall-clock ceiling by another name, and it
        // ends at a person for the same reason.
        Ended::Timeout(_) => {
            record.state = JobState::Paused;
            record.verdict = Some(Verdict::NeedsHuman);
            raise(
                place,
                now,
                record,
                inbox::Kind::NeedsHuman,
                "its turn ran past the workflow's wall clock",
            )?;
        }
        // **A Drone that died does not end its Job** (PLAN.md §14.1). The Job is
        // `STALLED` — the observer's word — and boarding it is how you find out
        // why.
        Ended::Died { code, stderr } => {
            record.state = JobState::Stalled;
            raise(
                place,
                now,
                record,
                inbox::Kind::Blocked,
                &format!(
                    "its Drone ended without finishing a turn ({}){}",
                    match code {
                        Some(code) => format!("exit {code}"),
                        None => "killed".to_string(),
                    },
                    if stderr.is_empty() {
                        String::new()
                    } else {
                        format!(": {stderr}")
                    }
                ),
            )?;
        }
    }
    Ok(())
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
    spend: Option<Spend>,
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
            spend,
        },
    )))
}

// -------------------------------------------------------------------------- ls

/// `armada fleet ls` — read-only, and it never resumes or interrupts a Job.
///
/// **Every column comes from data Claude Code already emits** (PHASES.md §9.1
/// F2). Nothing here estimates a cost, a token count or a remaining budget.
pub fn ls<C: Clock>(
    now: &C,
    place: &Where,
    all: bool,
    needs_attention: bool,
) -> Result<Output, ArmadaError> {
    let entries = inbox::read(&place.inbox())?;
    let wall = now.wall_ms();

    let mut rows: Vec<JobRow> = Vec::new();
    for record in place.store().all()? {
        let waiting = inbox::open_for(&entries, &record.name);
        let wants_you = record.state.needs_a_person() || waiting.is_some();
        if !all && record.state.is_over() {
            continue;
        }
        if needs_attention && !wants_you {
            continue;
        }
        let run_time = record.run_time_ms(wall);
        rows.push(JobRow {
            uuid: record.uuid.clone(),
            name: record.name.clone(),
            workflow: record.workflow.clone(),
            state: record.state,
            detail: detail(&record, waiting),
            runtime_s: run_time / 1_000,
            cost_usd: record.spend.cost_usd,
            tokens: record.spend.tokens,
            turns: record.spend.turns,
            budget_remaining: job::remaining(&record.budget, &record.spend, run_time),
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
fn detail(record: &Job, waiting: Option<&inbox::Entry>) -> String {
    match waiting {
        Some(entry) => entry.body.clone(),
        None if record.state == JobState::Queued => String::new(),
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

/// `armada fleet kill` — clean, drop the worktree, mark the Job ended.
///
/// **Three steps, in this order, and the order is the point**
/// (`commands/fleet/kill.md`). Cleaning before removing means resources are
/// released while the config that describes them is still present. If the order
/// is ever reversed, nothing is lost — ownership is recorded machine-globally,
/// so `armada manifest clean --all` still reclaims it afterwards. That safety
/// net is the reason Manifest sits underneath Fleet.
pub fn kill<R: Run>(
    run: &R,
    place: &Where,
    handle: Option<&str>,
    keep_branch: bool,
    keep_worktree: bool,
) -> Result<Output, ArmadaError> {
    let store = place.store();
    let targets = match handle {
        Some(handle) => vec![store.find(handle)?],
        None => store
            .all()?
            .into_iter()
            .filter(|record| record.state.is_over() || record.state == JobState::Paused)
            .collect(),
    };

    let mut results: Vec<Killed> = Vec::new();
    for mut record in targets {
        let path = expand(place, &record.worktree);
        // **Step one, and it is first for a reason**: resources are released
        // while the config that describes them is still present.
        let cleaned = manifest::clean(run, &place.exe, &path)?;
        let mut failure = cleaned.error;

        // **`git worktree remove` is run from the repository, not the
        // worktree.** git refuses to remove the tree it is standing in, and by
        // this point the record is the only thing that knows where the
        // repository was.
        let repo_root = expand(place, &record.repo_root);

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

/// Turn a `~/…` back into a real path.
fn expand(place: &Where, shown: &str) -> PathBuf {
    match shown.strip_prefix("~/") {
        Some(rest) => place.home.join(rest),
        None => PathBuf::from(shown),
    }
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
/// Job that asks a question (`commands/fleet/answer.md`).
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

    inbox::answer(&place.inbox(), &entry.uuid, said)?;

    let run_time = record.run_time_ms(now.wall_ms());
    let left = job::remaining(&record.budget, &record.spend, run_time);
    let ended = drone::turn(
        run,
        &expand(place, &record.worktree),
        argv::resume_argv(&record.uuid, said),
        drone::job_env(&record.name, &record.uuid),
        Duration::from_millis(left.wall_clock_ms),
    )?;
    let spend = ended.spend();
    record.spend.add(&spend);
    record.verdict = None;
    settle(&mut record, &ended, now, place)?;
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
            spend: Some(spend),
        },
    ))))
}
