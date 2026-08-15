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
    AnswerData, AskData, BoardData, Disposition, Envelope, Evidence, FleetLsData, InboxData,
    InboxRow, JobRow, KillData, Killed, NoteRow, PauseData, ProbeData, ReapCandidate, ReapPlanData,
    ReportData, ResumeData, ShowData, SpawnData, VerdictData,
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
use crate::ask::{Ask, Choice};
use crate::render::palette::Role;
// `Verdict` is already Fleet's own, so the progress one is `Reached` here.
use crate::render::progress::{Planned, Progress, Shape, SpawnStep, Verdict as Reached};
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

    /// **What a Drone started from here may do unattended.**
    ///
    /// Read from this machine's guild on every spawn, resume and answer, and
    /// deliberately not cached on the record. A posture is a preference, and a
    /// preference you changed should take effect on the next Drone rather than
    /// on the next Job — freezing it at spawn would mean a person who narrowed
    /// their permissions after a surprise still had every running Job's next
    /// turn ignore them.
    ///
    /// A machine with no guild at all gets the shipped default, for the same
    /// reason a guild with no `permissions.yml` does: a Job must work before
    /// anybody has configured anything.
    pub fn posture(&self) -> Result<armada_core::fleet::drone::Posture, ArmadaError> {
        armada_guild::permissions::read(Guild::at(&self.armada_home).root())
    }

    /// A path as a person writes it.
    pub fn shown(&self, path: &Path) -> String {
        home::tilde(path, &self.home)
    }

    /// Turn a `~/…` back into a real path.
    ///
    /// **Tilde-form is the stored form, and this is the only reader.** A Job
    /// record keeps `repo_root` and `worktree` as `~/…`
    /// ([`armada_core::fleet::job::Job`]), written by [`Where::shown`] at spawn
    /// and expanded here on every use. The decision, so that nothing has to
    /// re-take it:
    ///
    /// - **A record is portable.** `~/.armada/workspaces/api/rate-limit` is the
    ///   same Job on a laptop and on a workstation, and survives a `$HOME` that
    ///   moves — `/Users/x` to `/home/x` on a migration, or a machine whose
    ///   accounts were renamed. An absolute path baked in at spawn is a record
    ///   that silently names a directory belonging to somebody else.
    /// - **It is the form the record already had**, and the two audiences
    ///   already read it: `armada fleet ls` and `--json` both print
    ///   `~/.armada/…`, so making the stored form absolute would either change
    ///   what every reader sees or add a second conversion in the other
    ///   direction.
    /// - **The cost is that every reader must expand**, and a reader that
    ///   forgets gets `ENOENT` on a path with a literal `~` in it. That is the
    ///   defect this doc comment exists to stop recurring: `armada fleet board`
    ///   handed `~/.armada/…` to `chdir` and could not board any Job at all.
    ///
    /// **Both forms are accepted**, because records written before this was
    /// settled are on disk and a record is not migrated for a question that has
    /// one right answer either way: an absolute path is already expanded, and a
    /// bare `~` is `$HOME`.
    pub fn expand(&self, shown: &str) -> PathBuf {
        match shown {
            "~" => self.home.clone(),
            _ => match shown.strip_prefix("~/") {
                Some(rest) => self.home.join(rest),
                None => PathBuf::from(shown),
            },
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
///
/// **A guess stops and asks.** `ask` is `Some` only when a person is at the
/// other end; see [`settle_workflow`] for why a low-confidence spawn refuses
/// rather than proceeding when nobody is.
///
/// **It reports itself as it goes, in the table it will answer in.** A spawn
/// makes a worktree, runs `armada manifest init` and starts a Drone, and until
/// this it printed nothing at all until every one of them was done — the same
/// silence `armada manifest check` had before `render/live.rs`, and answered
/// the same way rather than a second way. `progress` is the same trait,
/// `Shape::Spawn` picks the same columns the final table uses, and
/// [`SpawnStep`] is the one place either table learns the words.
///
/// **The table opens after the classification is settled, never before.** A
/// low-confidence spawn asks, and `ask/select.rs` reserves an inline viewport
/// on stderr of its own — two of them on one stream is a corrupted screen. The
/// classify row is not lost to that: it is reported the moment the table opens,
/// with the interval it actually took, so all four rows are present from the
/// first frame and the steps that made the run feel hung are the ones drawn
/// live.
pub fn spawn<R: Run, C: Clock>(
    run: &R,
    now: &C,
    place: &Where,
    options: &Spawn,
    ask: Option<&mut dyn Ask>,
    progress: &mut dyn Progress,
) -> Result<Output, ArmadaError> {
    let repo_root = repository(run, place, options)?;
    let repo = home::repo_name(&repo_root);

    let classify_from = now.mono();
    let (guessed, classify_ms) = classify(run, now, &repo_root, options)?;
    let classify_to = now.mono();
    // Blocks on a person when the classification was a guess, which is why the
    // reading above it is taken before this line and not after: the time
    // somebody took to answer is not time spent classifying.
    let classification = settle_workflow(guessed, options, ask)?;
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
        progress: Vec::new(),
        attempts: std::collections::BTreeMap::new(),
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

    // **All four rows from the first frame**, exactly as `check` plans its
    // table by name rather than by count: a spawn stuck making a worktree
    // should show which step that is and that three more are coming, not a
    // number. None of them has a configured deadline, so none claims one —
    // `render/live.rs` leaves that cell empty rather than printing `timeout 0s`.
    let planned: Vec<Planned<'_>> = SpawnStep::ALL
        .iter()
        .map(|step| Planned {
            id: step.id(),
            timeout_ms: None,
        })
        .collect();
    progress.begin(Shape::Spawn, &planned, classify_from);
    // Already done by the time the table can safely open, and reported with the
    // interval it took rather than as a row that mysteriously starts complete.
    progress.started(SpawnStep::Classify.id());
    progress.tick(classify_to);
    let (classified, role) = crate::render::spawn_classified(&record.workflow, record.confidence);
    progress.finished(
        SpawnStep::Classify.id(),
        Reached::Word(SpawnStep::Classify.done(), role),
        Some(&classified),
    );

    let started = now.mono();
    progress.tick(started);
    progress.started(SpawnStep::Worktree.id());
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
        // Nothing has been raised this early, but the close travels with every
        // terminal write rather than with the ones that happen to need it —
        // an entry that outlives its Job is the defect, and remembering by
        // hand at each site is how it comes back.
        close_entries(place, &record)?;
        return Err(error);
    }
    progress.tick(now.mono());
    progress.finished(
        SpawnStep::Worktree.id(),
        Reached::Word(SpawnStep::Worktree.done(), Role::BeaconGreen),
        Some(&place.shown(&path)),
    );

    progress.started(SpawnStep::Ports.id());
    record.port_block = manifest::init(run, &place.exe, &path)?;
    record.state = JobState::Running;
    store.save(&record)?;
    let prepare_ms = now.mono().saturating_sub(started);
    progress.tick(now.mono());
    // A workspace with nothing to collide over claims nothing, and the row says
    // so in grey rather than reporting a green success at claiming no ports —
    // which is the same distinction `render.rs`'s final row draws.
    progress.finished(
        SpawnStep::Ports.id(),
        Reached::Word(
            SpawnStep::Ports.done(),
            match record.port_block {
                Some(_) => Role::BeaconGreen,
                None => Role::SteelGrey,
            },
        ),
        record
            .port_block
            .map(|b| format!("{}-{}", b.from, b.to))
            .as_deref(),
    );

    // **The Drone is started detached and `spawn` returns.** That is the whole
    // purpose of Fleet — five Jobs at once with one thing to watch — and it is
    // why nothing here waits, reads a ledger, or reports a spend: the Drone is
    // still working when this function ends.
    progress.started(SpawnStep::Drone.id());
    record.drone = Some(start_drone(
        run,
        place,
        &record,
        &path,
        argv::spawn_argv(
            &uuid,
            &prompt(&workflow, &step, &options.task),
            &place.posture()?,
        ),
    )?);
    store.save(&record)?;
    progress.tick(now.mono());
    progress.finished(
        SpawnStep::Drone.id(),
        Reached::Word(SpawnStep::Drone.done(), Role::BeaconGreen),
        Some(&format!("job {}, {} step", job::short(&uuid), step)),
    );

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

/// A guess, settled by a person — or refused.
///
/// **Printing the word `a guess` was not enough.** A real spawn read
/// `classified  workflow  feature, confidence 0.15, a guess` and went straight
/// on to make a worktree, claim a block and start a Drone on a budget. §14.2
/// puts the confidence on the screen *"so a guess is visible as a guess"*, and a
/// guess that is visible for one line and then acted on regardless has not been
/// surfaced — it has been narrated.
///
/// So below the threshold the four workflows are put to the person, with the
/// model's guess already selected: one keypress confirms it and one arrow key
/// changes it. That is cheaper than the thing it prevents by an order of
/// magnitude — the wrong workflow is a whole budget spent looking busy, and a
/// `design` Job working on a bug does not look wrong until somebody reads it.
///
/// **With nobody there it refuses, and does not hang.** An agent driving Armada
/// through a pipe cannot answer, and waiting on an answer that will never arrive
/// is worse than either alternative. `bad_invocation` naming `--workflow` is the
/// honest one: a Job started on a coin flip costs a worktree and a budget to
/// discover.
fn settle_workflow(
    guessed: Classification,
    options: &Spawn,
    ask: Option<&mut dyn Ask>,
) -> Result<Classification, ArmadaError> {
    let threshold = options
        .confidence
        .unwrap_or(armada_core::fleet::classify::CONFIDENT);
    if guessed.is_confident(threshold) {
        return Ok(guessed);
    }

    let Some(ask) = ask else {
        return Err(ArmadaError {
            class: ErrClass::BadInvocation,
            r#where: "classify".to_string(),
            message: format!(
                "`{}` is a guess at {:.2}, and there is nobody to ask",
                guessed.workflow,
                guessed.confidence.unwrap_or(0.0)
            ),
            next_action: Some(format!(
                "name it: --workflow {}",
                workflow::STARTERS.join("|")
            )),
        });
    };

    // The guess is the default, so confirming it is `enter` and changing it is
    // one arrow key. `select.rs` handles the keys, the digits and the
    // non-widget fallback; this only decides what is offered.
    let options_offered: Vec<Choice> = workflow::STARTERS
        .iter()
        .map(|name| Choice::new(name, describe(name)))
        .collect();
    let default = workflow::STARTERS
        .iter()
        .position(|name| *name == guessed.workflow)
        .map_or(1, |at| at + 1);

    let chosen = ask.choose(
        &format!(
            "Which workflow is this? (guessed {} at {:.2})",
            guessed.workflow,
            guessed.confidence.unwrap_or(0.0)
        ),
        &options_offered,
        default,
    );
    let picked = workflow::STARTERS
        .get(chosen.saturating_sub(1))
        .unwrap_or(&workflow::STARTERS[0]);

    // **Answered by a person is an override, not a confident model.** Recording
    // it any other way would put a confidence on the screen that nobody
    // measured — the distinction `Classification::overridden` exists for.
    Ok(Classification::overridden(picked))
}

/// One clause per workflow, for the selector's aside.
///
/// **The same four sentences the classifier's prompt uses**, so the person and
/// the model are choosing between the same descriptions.
fn describe(workflow: &str) -> &'static str {
    match workflow {
        "design" => "deciding an approach, no code",
        "plan" => "writing down how, before building",
        "bug" => "something is broken, reproduce it first",
        _ => "building something new",
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

/// Raise an entry, and hand back the id it was given.
///
/// **The id is returned rather than discarded** because `fleet.ask_human` has to
/// name the thing it is waiting on — and because every item a person is asked to
/// act on needs an identity they can acknowledge one row at a time (PLAN.md
/// §15.3.1). The callers that only raise ignore it, which costs nothing.
fn raise<C: Clock>(
    place: &Where,
    now: &C,
    record: &Job,
    kind: inbox::Kind,
    body: &str,
) -> Result<String, ArmadaError> {
    let at_ms = now.wall_ms();
    inbox::raise(
        &place.inbox(),
        &job::mint_uuid(&format!("{}|{at_ms}|{body}", record.uuid)),
        // **The uuid is the identity and the name only travels beside it**
        // (`docs/reserved/005-inbox-label-not-identity.md`). Raising against
        // the name was the defect: `free_name` hands a name out again once the
        // Job holding it is over, so two Jobs called `this-test` produced five
        // entries that belonged to neither.
        &record.uuid,
        &record.name,
        kind,
        &now.wall_rfc3339(),
        at_ms,
        body,
    )
}

/// Every inbox entry, with any legacy one given the identity it was written
/// without.
///
/// **The migration is here rather than in a verb of its own**, because an
/// inbox full of name-keyed entries is on a real machine right now and a fix
/// that needed a command run first is a fix most machines never get. It is
/// append-only, idempotent, and triggered only while a legacy entry remains —
/// so it converges after one read and every read after that is a read.
///
/// **The Job index is loaded only when there is something to migrate.** A
/// machine already migrated pays a `read_to_string` and nothing else, which is
/// what keeps `armada fleet inbox` as cheap as it was.
fn entries(place: &Where) -> Result<Vec<inbox::Entry>, ArmadaError> {
    let entries = inbox::read(&place.inbox())?;
    if !entries.iter().any(inbox::Entry::is_legacy) {
        return Ok(entries);
    }
    let jobs: Vec<(String, String, bool)> = place
        .store()
        .all()?
        .into_iter()
        .map(|record| (record.name, record.uuid, record.state.is_over()))
        .collect();
    inbox::migrate(&place.inbox(), &jobs)?;
    inbox::read(&place.inbox())
}

/// Close everything a Job had open, because the Job has ended.
///
/// **Called beside the write that ends it, never from a read.** `DONE` and
/// `ABORTED` are the two states a verb writes deliberately —
/// [`armada_core::fleet::job::observe`] never invents either — so this is
/// complete, and `armada fleet ls` stays a read that changes nothing.
fn close_entries(place: &Where, record: &Job) -> Result<(), ArmadaError> {
    inbox::close(&place.inbox(), &record.uuid, inbox::Closed::Ended)?;
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
    let entries = entries(place)?;
    let wall = now.wall_ms();

    let mut rows: Vec<JobRow> = Vec::new();
    for record in place.store().all()? {
        if !all && record.state.is_over() {
            continue;
        }
        let (observed, _) = look(run, place, &record, wall);
        // **By uuid, and never for a Job that is over.** The first is the
        // defect `005` records; the second is its first consequence, and it is
        // asserted here as well as written at the close because `--all` draws
        // finished Jobs and an ended Job must not still be advertising a
        // question nobody can answer.
        let waiting = match observed.state.is_over() {
            true => None,
            false => inbox::open_for(&entries, &record.uuid),
        };
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
            // **Carried, never re-read.** The Bridge draws a `TASK` column and
            // is a renderer over this listing — a second pass over the Job index
            // to fill one column would be the second source
            // `commands/helm/bridge.md` rules out.
            task: record.task.clone(),
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

// ------------------------------------------------------------------------ show

/// `armada fleet show` — **one Job, and why it wants you.**
///
/// **The verb behind the Bridge's detail view, and it exists as a verb for that
/// reason.** `commands/helm/bridge.md` says every key maps to a verb that
/// already exists and is reachable from a shell — that is what keeps the Bridge
/// a rendering choice rather than an architectural one — so the pane renders
/// this payload rather than growing a read of its own. It is also what gives the
/// view all three audiences (PLAN.md §3.1.1) instead of only the one at a
/// terminal.
///
/// **Nothing here explains anything twice.** The state is [`look`]'s, the same
/// one `ls` renders; the reason it wants you is the inbox entry's own body, the
/// same one `armada fleet inbox` prints; the step is the record's. This verb
/// gathers and never rephrases — a second wording of one state is a bug that
/// only shows up when the two are read side by side.
///
/// **Read-only, like every other view.** No `settle`, so `show` on a Job that
/// reached a ceiling reports it without persisting it or raising a second inbox
/// entry: watching something must not change it (PLAN.md §15.2), and the Bridge
/// re-reads this every interval.
pub fn show<R: Run, C: Clock>(
    run: &R,
    now: &C,
    place: &Where,
    handle: &str,
) -> Result<Output, ArmadaError> {
    let record = place.store().find(handle)?;
    let wall = now.wall_ms();
    let (observed, _) = look(run, place, &record, wall);
    let run_time = record.run_time_ms(wall);

    // **Every entry this Job raised, not only the open one.** `ls` folds the
    // oldest open entry into one `detail` cell and drops the rest; a reader
    // asking why a Job wants them is often looking at the second question, and
    // an answered one is the record of what was already decided.
    //
    // **Selected by uuid, which is what makes the cut exact.** It used to
    // filter on the name and then on `raised_ms >= created_ms` to keep a fresh
    // `nightly-flake` from inheriting its namesake's questions — an
    // approximation that fails the moment two Jobs of one name overlap, which
    // is exactly what `docs/reserved/005-inbox-label-not-identity.md` was
    // raised about. The uuid needs no window.
    let asked: Vec<InboxRow> = entries(place)?
        .into_iter()
        .filter(|entry| entry.job_uuid.as_deref() == Some(record.uuid.as_str()))
        .map(|entry| row(&entry, wall))
        .collect();

    // **Newest first, which is the opposite of the inbox's order and is meant
    // to be.** An inbox is a queue and is answered oldest first; progress is a
    // log, and the useful end of a log is the last thing that happened.
    let mut progress: Vec<NoteRow> = record
        .progress
        .iter()
        .map(|note| NoteRow {
            at: note.at.clone(),
            ago_s: wall.saturating_sub(note.at_ms) / 1_000,
            step: note.step.clone(),
            body: note.body.clone(),
        })
        .collect();
    progress.reverse();

    let waiting_on_you = observed.state.needs_a_person() || asked.iter().any(InboxRow::is_open);

    Ok(Output::Show(Box::new(Envelope::ok(
        "fleet show",
        None,
        // **The Job's state is not the command's**, and `show` succeeds whenever
        // the record is readable — the same rule `ls` follows. A `BLOCKED` Job
        // reported successfully is exit 0.
        Status::Ok,
        ShowData {
            job: record.name.clone(),
            uuid: record.uuid.clone(),
            workflow: record.workflow.clone(),
            state: observed.state,
            recorded_state: record.state,
            drone_pgid: record.drone.as_ref().map(|handle| handle.pgid),
            drone_alive: drone::alive(
                run,
                &place.armada_home,
                record.drone.as_ref(),
                &place.boot_id,
            ),
            step: record.step.clone(),
            attempt: record.attempts.get(&record.step).copied().unwrap_or(0),
            task: record.task.clone(),
            runtime_s: run_time / 1_000,
            created_at: record.created_at.clone(),
            cost_usd: observed.spend.cost_usd,
            tokens: observed.spend.tokens,
            turns: observed.spend.turns,
            budget: record.budget,
            budget_remaining: job::remaining(&record.budget, &observed.spend, run_time),
            repo: record.repo.clone(),
            branch: record.branch.clone(),
            worktree: record.worktree.clone(),
            port_block: record.port_block,
            needs_attention: waiting_on_you,
            asked,
            progress,
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

    end(run, now, place, targets, keep_branch, keep_worktree)
}

/// End every one of these Jobs, and report what each released.
///
/// **One teardown, shared by `kill` and `reap`.** A second copy of this loop
/// would be a second answer to what `kill` orders and what it tolerates — and
/// the order is the point (`commands/fleet/kill.md`), so there is one of it.
///
/// **A Job that will not clean does not stop the rest.** The failure is carried
/// on that Job's row and the loop carries on, for the same reason
/// [`manifest::Cleaned::error`] is carried rather than raised: one container
/// that refuses to stop must not leave four other Jobs holding their worktrees.
fn end<R: Run, C: Clock>(
    run: &R,
    now: &C,
    place: &Where,
    targets: Vec<Job>,
    keep_branch: bool,
    keep_worktree: bool,
) -> Result<Output, ArmadaError> {
    let store = place.store();
    let wall = now.wall_ms();
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
        //
        // **Nothing here is raised, and that is the contract rather than a
        // tolerance** (`armada_fleet::manifest::Cleaned::error`): the Job is
        // marked ended either way, and a `kill` that bailed out would leave the
        // worktree as well as the Job. It was raised, once, and the symptom was
        // the one this is written against — `x` on a Job whose worktree had
        // been deleted failed with a message about reinstalling Armada, and
        // left the Job `RUNNING` in the record with no way to end it.
        //
        // **A worktree that is gone is not asked to clean itself.** There is no
        // directory to resolve `armada.yml` in and no config left to describe
        // what to release; what the Job owned is recorded machine-globally, so
        // `armada manifest clean --all` reclaims it. Running the subprocess
        // anyway would spend a spawn to be told the directory is missing, which
        // is what `disposition` below is about to say.
        let cleaned = match path.is_dir() {
            true => {
                manifest::clean(run, &place.exe, &path).unwrap_or_else(manifest::Cleaned::failed)
            }
            false => manifest::Cleaned::default(),
        };
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

        // **Its inbox entries end with it.** Both of the user's Jobs reached
        // `ABORTED` and five entries stayed open against Jobs that no longer
        // existed, still advertising `armada fleet answer` — the two
        // consequences `docs/reserved/005-inbox-label-not-identity.md`
        // records, and this line is where the first of them is closed.
        //
        // **After the save, and never instead of it.** A `kill` that failed
        // here must still have ended the Job; an entry left open is a stale
        // row, an unsaved record is a Job nothing can end.
        close_entries(place, &record)?;
    }

    let error = results.iter().find_map(|killed| killed.error.clone());
    let data = KillData { results };
    Ok(Output::Kill(Box::new(match error {
        Some(error) => Envelope::failed("fleet kill", None, error, data),
        None => Envelope::ok("fleet kill", None, Status::Clean, data),
    })))
}

// --------------------------------------------------------------- pause/resume

/// `armada fleet pause` — stop the Drone, keep the Job.
///
/// **A Job is durable and a Drone is not, which is what makes this a verb rather
/// than a signal** (PLAN.md §14.1). Pausing stops the process that is working
/// and leaves everything the Job *is* exactly where it was: the worktree, the
/// branch, the port block and the transcript. [`resume`] starts a new Drone on
/// the same session, and the transcript — which is the ledger — carries on being
/// appended to, so a Job held for an hour has not spent anything in that hour
/// and has not had its budget reset either.
///
/// **`SIGSTOP` was the other candidate and is the wrong one.** A stopped process
/// still answers `ps`, so [`armada_core::fleet::job::observe`] would go on
/// calling it `RUNNING` — the pause would not stick — and a Claude Code session
/// frozen mid-request holds a connection open for as long as the person is away.
pub fn pause<R: Run, C: Clock>(
    run: &R,
    now: &C,
    place: &Where,
    handle: &str,
) -> Result<Output, ArmadaError> {
    let store = place.store();
    let mut record = store.find(handle)?;
    let (observed, _) = look(run, place, &record, now.wall_ms());

    if observed.state.is_over() {
        return Err(ArmadaError {
            class: ErrClass::BadInvocation,
            r#where: record.name.clone(),
            message: format!(
                "`{}` has already ended — it is {}",
                record.name, observed.state
            ),
            next_action: Some("`armada fleet ls --all` lists the ended ones".to_string()),
        });
    }
    if observed.state == JobState::Paused {
        return Err(ArmadaError {
            class: ErrClass::BadInvocation,
            r#where: record.name.clone(),
            message: format!("`{}` is already paused", record.name),
            next_action: Some(format!(
                "`armada fleet resume {}` starts it again",
                record.name
            )),
        });
    }

    // **The Drone goes, and nothing else does.** Ownership of the group is
    // forgotten in the same breath, because a group that is gone must not stay
    // in the machine-global store for `armada manifest clean` to find later and
    // decide about.
    let stopped = drone::stop(
        run,
        &place.armada_home,
        record.drone.as_ref(),
        &place.boot_id,
    );
    let pgid = record.drone.as_ref().map(|handle| handle.pgid);
    if let Some(handle) = &record.drone {
        own::forget_drone(
            run,
            &place.armada_home,
            &place.expand(&record.worktree),
            handle.pgid,
        );
    }
    record.drone = None;

    // Settled from the transcript on the way in, because nothing is going to
    // write to it again until the Job is resumed.
    record.spend = drone::transcript(&place.stream(&record.uuid)).spend;
    record.state = JobState::Paused;
    store.save(&record)?;

    let failure = (stopped == drone::Stopped::Survived).then(|| ArmadaError {
        class: ErrClass::ToolFailed,
        r#where: record.name.clone(),
        message: "the Drone was still running after SIGKILL".to_string(),
        next_action: Some("look for it by hand; `armada fleet ls` names it".to_string()),
    });

    let data = PauseData {
        job: record.name.clone(),
        uuid: record.uuid.clone(),
        state: record.state,
        // Reported only when there was one to stop: a Job between turns has no
        // live Drone, and holding it is still something a person can ask for.
        stopped: (stopped == drone::Stopped::Stopped)
            .then_some(pgid)
            .flatten(),
        spend: record.spend,
    };
    Ok(Output::Pause(Box::new(match failure {
        // **The Job is held either way**, for `kill`'s reason: a pause that
        // bailed out because a group would not die would need a second pause to
        // do the same thing again.
        Some(error) => Envelope::failed("fleet pause", None, error, data),
        None => Envelope::ok("fleet pause", None, Status::Ok, data),
    })))
}

/// `armada fleet resume` — start a new Drone on the same session.
///
/// **Two refusals, and both are somebody else's verb.** A Job that reached a
/// ceiling is not resumed past it — `on_exhausted: needs_human` means a person
/// decides what happens next, and silently continuing is how a budget stops
/// being one. A Job with an open question is *answered* rather than resumed:
/// continuing it with [`armada_core::fleet::drone::CONTINUE`] would leave the
/// inbox entry open forever and put words into a conversation that asked for
/// different ones.
pub fn resume<R: Run, C: Clock>(
    run: &R,
    now: &C,
    place: &Where,
    handle: &str,
) -> Result<Output, ArmadaError> {
    let store = place.store();
    let mut record = store.find(handle)?;
    let (observed, _) = look(run, place, &record, now.wall_ms());

    if observed.state != JobState::Paused {
        return Err(ArmadaError {
            class: ErrClass::BadInvocation,
            r#where: record.name.clone(),
            message: format!("`{}` is not paused — it is {}", record.name, observed.state),
            next_action: Some("`armada fleet ls` says what each Job is doing".to_string()),
        });
    }

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

    let entries = entries(place)?;
    if inbox::open_for(&entries, &record.uuid).is_some() {
        return Err(ArmadaError {
            class: ErrClass::BadInvocation,
            r#where: record.name.clone(),
            message: format!("`{}` is waiting on an answer, not on a resume", record.name),
            next_action: Some(format!(
                "`armada fleet answer {} \"<your answer>\"`",
                record.name
            )),
        });
    }

    // **A worktree that is gone is said so before a Drone is started in it.**
    // `chdir` would fail inside the detached child, where the only evidence is a
    // Drone that recorded a group and died immediately.
    let path = place.expand(&record.worktree);
    if !path.is_dir() {
        return Err(ArmadaError {
            class: ErrClass::Environment,
            r#where: record.worktree.clone(),
            message: format!(
                "`{}` has no worktree left to run in: `{}` is gone",
                record.name, record.worktree
            ),
            next_action: Some(format!(
                "`armada fleet kill {}` ends it and releases what it holds",
                record.name
            )),
        });
    }

    record.spend = observed.spend;
    record.state = JobState::Running;
    record.drone = Some(start_drone(
        run,
        place,
        &record,
        &path,
        argv::continue_argv(&record.uuid, &place.posture()?),
    )?);
    store.save(&record)?;

    Ok(Output::Resume(Box::new(Envelope::ok(
        "fleet resume",
        None,
        Status::Ok,
        ResumeData {
            job: record.name.clone(),
            uuid: record.uuid.clone(),
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

// ------------------------------------------------------------------------ reap

/// `armada fleet reap --dry-run` — every Job a reap would offer, and what each
/// is still holding.
///
/// **The preview is the feature.** A bulk delete that only listed names would be
/// asking a person to approve a decision on less information than the machine
/// already has; what makes the answer possible is the second half of every row.
/// A port block held by a Job whose Drone died months ago is a span nothing can
/// use and nothing will report — and it is invisible until something puts it
/// beside the Job's name.
///
/// **Observed rather than recorded**, which is what makes it useful at all: a
/// record that still says `RUNNING` with a dead process group is exactly the Job
/// this verb exists to find, and asking the record would file it under the one
/// state that is never offered.
pub fn reap_plan<R: Run, C: Clock>(run: &R, now: &C, place: &Where) -> Result<Output, ArmadaError> {
    let wall = now.wall_ms();
    let mut results: Vec<ReapCandidate> = Vec::new();
    for record in place.store().all()? {
        let (observed, _) = look(run, place, &record, wall);
        let reaping = observed.state.reaping();
        if !reaping.is_offered() {
            continue;
        }
        results.push(ReapCandidate {
            job: record.name.clone(),
            uuid: record.uuid.clone(),
            state: observed.state,
            selected: reaping.is_default(),
            port_block: record.port_block,
            worktree_exists: place.expand(&record.worktree).is_dir(),
            worktree_path: record.worktree.clone(),
            branch: record.branch.clone(),
            cost_usd: observed.spend.cost_usd,
        });
    }

    let selected = results.iter().filter(|row| row.selected).count();
    Ok(Output::ReapPlan(Box::new(Envelope::ok(
        "fleet reap",
        None,
        // A read verb reports rather than judges: it exits 0 whenever the index
        // is readable, and `SKIPPED` is what "there was nothing to do" is called.
        match results.is_empty() {
            true => Status::Skipped,
            false => Status::Ok,
        },
        ReapPlanData { results, selected },
    ))))
}

/// `armada fleet reap` — end exactly these Jobs.
///
/// **Named, never inferred.** The plan decides what is *offered*; what is taken
/// is a list, so the preview a person read and the reap that follows cannot
/// disagree because the fleet moved between them. A Job named here that is no
/// longer reapable — its Drone came back to life between the preview and the
/// `enter` — is refused rather than killed.
pub fn reap<R: Run, C: Clock>(
    run: &R,
    now: &C,
    place: &Where,
    jobs: &[String],
) -> Result<Output, ArmadaError> {
    let store = place.store();
    let wall = now.wall_ms();
    let mut targets: Vec<Job> = Vec::new();
    for handle in jobs {
        let record = store.find(handle)?;
        let (observed, _) = look(run, place, &record, wall);
        if !observed.state.reaping().is_offered() {
            return Err(ArmadaError {
                class: ErrClass::BadInvocation,
                r#where: record.name.clone(),
                message: format!(
                    "`{}` is {} and a reap does not take a Job that is working",
                    record.name, observed.state
                ),
                next_action: Some(format!(
                    "`armada fleet kill {}` ends it deliberately",
                    record.name
                )),
            });
        }
        targets.push(record);
    }

    end(run, now, place, targets, false, false)
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
    // **`--job` resolves through the Job index rather than matching the entry's
    // label.** Typing a name that means two Jobs is refused here exactly as
    // `armada fleet show` refuses it, instead of quietly returning both Jobs'
    // entries in one undifferentiated list — which is what it did, and is the
    // reading half of `docs/reserved/005-inbox-label-not-identity.md`.
    let only = match job {
        Some(handle) => Some(place.store().find(handle)?.uuid),
        None => None,
    };
    let rows: Vec<InboxRow> = entries(place)?
        .into_iter()
        .filter(|entry| {
            only.as_deref()
                .is_none_or(|uuid| entry.job_uuid.as_deref() == Some(uuid))
        })
        .filter(|entry| all || entry.is_open())
        .map(|entry| row(&entry, wall))
        .collect();

    let open = rows.iter().filter(|row| row.is_open()).count();
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

/// One entry, as every view reports it.
///
/// **One conversion, so `inbox` and `show` cannot describe one entry
/// differently.** They already promise to carry [`InboxRow`] unchanged
/// (`envelope.rs`), and two copies of this mapping is how that promise stops
/// being true.
fn row(entry: &inbox::Entry, wall: u64) -> InboxRow {
    InboxRow {
        uuid: entry.uuid.clone(),
        job_uuid: entry.job_uuid.clone(),
        job: entry.job.clone(),
        kind: entry.kind.word().to_string(),
        raised_at: entry.raised_at.clone(),
        waiting_s: wall.saturating_sub(entry.raised_ms) / 1_000,
        body: entry.body.clone(),
        answered: entry.answered.clone(),
        closed: entry.closed.map(|why| why.word().to_string()),
    }
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

    // **A Job that has ended is refused before anything is read.** Its entries
    // were closed when it ended, so there is nothing to find — but the message
    // a reader gets should name the reason rather than say "nothing open",
    // which is the misleading half of
    // `docs/reserved/005-inbox-label-not-identity.md`'s second consequence.
    if record.state.is_over() {
        return Err(ArmadaError {
            class: ErrClass::BadInvocation,
            r#where: record.name.clone(),
            message: format!(
                "`{}` has ended — it is {}, and a finished Job has nothing to answer",
                record.name,
                record.state.word()
            ),
            next_action: Some("`armada fleet inbox --all` shows what it asked".to_string()),
        });
    }

    let entries = entries(place)?;
    let Some(entry) = inbox::open_for(&entries, &record.uuid) else {
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
        argv::resume_argv(&record.uuid, said, &place.posture()?),
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

// ----------------------------------------------------------------------- probe

/// `fleet.probe` — one Job's transcript, summarised, **without a word to the
/// Drone** (PLAN.md §15.2).
///
/// **The only verb here that reads and never writes.** It does not `settle` what
/// it observed, which every other verb that opens a record does: probing is what
/// the orchestrator does *constantly* — five Jobs, asked about between every
/// exchange — and a read that mutated would make "how is it going" a state
/// transition. The record it reports is the record as it stands.
///
/// The summary itself is [`armada_fleet::drone::probe`], one turn of the cheapest
/// model over the tail of the transcript. A Job that has written nothing gets a
/// sentence rather than a call, because that is the ordinary state a moment
/// after `spawn` returns and paying a model to say so is waste.
pub fn probe<R: Run>(run: &R, place: &Where, handle: &str) -> Result<Output, ArmadaError> {
    let record = place.store().find(handle)?;
    let stream = std::fs::read_to_string(place.stream(&record.uuid)).unwrap_or_default();
    let (tail, events) = armada_core::fleet::probe::tail(&stream);

    let summary = if events == 0 {
        armada_core::fleet::probe::NOTHING_YET.to_string()
    } else {
        drone::probe(run, &place.armada_home, &record.task, &tail)?
    };

    Ok(Output::Probe(Box::new(Envelope::ok(
        "fleet probe",
        None,
        Status::Ok,
        ProbeData {
            job: record.name.clone(),
            uuid: record.uuid.clone(),
            state: record.state,
            step: record.step.clone(),
            summary,
            events,
            model: armada_core::fleet::classify::MODEL.to_string(),
        },
    ))))
}

// ---------------------------------------------------------------------- report

/// `fleet.report` — a Drone appends progress to **its own** Job record.
///
/// **Its own, and nothing else's.** The Job is named by `handle`, which the MCP
/// layer fills from `ARMADA_JOB` — the variable [`armada_fleet::drone::job_env`]
/// sets on every child of a Job and that a Drone therefore cannot choose. A
/// worker able to write another worker's record is a worker that can rewrite the
/// evidence a verdict rests on.
pub fn report<C: Clock>(
    now: &C,
    place: &Where,
    handle: &str,
    body: &str,
) -> Result<Output, ArmadaError> {
    let store = place.store();
    let mut record = store.find(handle)?;

    // **A Job that is over does not gain notes.** The record is the durable
    // half, and appending to a finished one would put progress after the
    // verdict it was meant to justify.
    if record.state.is_over() {
        return Err(ArmadaError {
            class: ErrClass::BadInvocation,
            r#where: record.name.clone(),
            message: format!("`{}` is {}", record.name, record.state.word()),
            next_action: Some("a Job that has ended takes no more progress".to_string()),
        });
    }

    record.progress.push(armada_core::fleet::job::Note {
        at: now.wall_rfc3339(),
        at_ms: now.wall_ms(),
        step: record.step.clone(),
        body: body.to_string(),
    });
    store.save(&record)?;

    Ok(Output::Report(Box::new(Envelope::ok(
        "fleet report",
        None,
        Status::Ok,
        ReportData {
            job: record.name.clone(),
            step: record.step.clone(),
            notes: record.progress.len(),
        },
    ))))
}

// ------------------------------------------------------------------- ask_human

/// How often the wait looks for an answer.
///
/// **A poll of one local file, not of a service.** The inbox is append-only on
/// disk, so a read costs a `read_to_string` — and two seconds is short enough
/// that a person who answers does not sit watching a Job that has not noticed.
pub const ASK_POLL_MS: u64 = 2_000;

/// `fleet.ask_human` — raise an entry, and wait for the answer.
///
/// **The waiting has a ceiling and the ceiling is the caller's.** An unbounded
/// wait inside a tool call is a Drone that holds a request open for as long as
/// its person is at lunch; expiring returns the entry's id with `answered: null`,
/// which is a true answer — *it is still open* — rather than a failure. The
/// entry stays in the inbox either way, so nothing is lost by the wait ending:
/// `armada fleet answer` closes it whenever a person gets to it.
///
/// **This is why the tool belongs behind the tasks extension.** A long operation
/// with polling and a durable handle is exactly what the extension is for, and
/// inventing a second polling protocol beside it is what
/// `commands/helm/mcp.md` refuses.
pub fn ask_human<C: Clock>(
    now: &C,
    place: &Where,
    handle: &str,
    question: &str,
    wait_ms: u64,
) -> Result<Output, ArmadaError> {
    let record = place.store().find(handle)?;
    let entry = raise(place, now, &record, inbox::Kind::NeedsHuman, question)?;

    let deadline = now.mono().saturating_add(wait_ms);
    let mut answered = None;
    loop {
        // **Read before sleeping**, so a `wait_ms` of zero still reports an
        // entry somebody had already answered — and so the first look costs no
        // latency.
        answered = inbox::read(&place.inbox())?
            .into_iter()
            .find(|row| row.uuid == entry)
            .and_then(|row| row.answered)
            .or(answered);
        if answered.is_some() || now.mono() >= deadline {
            break;
        }
        now.sleep_until((now.mono() + ASK_POLL_MS).min(deadline));
    }

    Ok(Output::Ask(Box::new(Envelope::ok(
        "fleet ask_human",
        None,
        Status::Ok,
        AskData {
            job: record.name.clone(),
            entry,
            question: question.to_string(),
            answered,
        },
    ))))
}

// --------------------------------------------------------------------- verdict

/// `fleet.verdict` — how a step ended (PLAN.md §14.3).
///
/// **A `PASS` with no evidence is refused rather than recorded.** *"A verdict is
/// only `PASS` if it carries evidence an external command produced"* — an agent
/// asserting that the tests pass is not evidence, an `armada manifest check`
/// exit code is. Refusing here is what makes that rule structural rather than a
/// sentence in a workflow's prompt, and it is the reason the loop genuinely
/// depends on Manifest's `check` verb.
///
/// `BLOCKED` and `NEEDS_HUMAN` raise to the inbox on their way out, because both
/// mean the Job has stopped and neither can be discovered by a person who is not
/// looking.
pub fn verdict<C: Clock>(
    now: &C,
    place: &Where,
    handle: &str,
    step: &str,
    reached: Verdict,
    evidence: Vec<Evidence>,
) -> Result<Output, ArmadaError> {
    if reached == Verdict::Pass && evidence.is_empty() {
        return Err(ArmadaError {
            class: ErrClass::BadInvocation,
            r#where: step.to_string(),
            message: "a PASS carries evidence an external command produced".to_string(),
            next_action: Some(
                "run `manifest.check` and pass its result ids and exit codes as evidence"
                    .to_string(),
            ),
        });
    }

    let store = place.store();
    let mut record = store.find(handle)?;
    if record.state.is_over() {
        return Err(ArmadaError {
            class: ErrClass::BadInvocation,
            r#where: record.name.clone(),
            message: format!("`{}` is {}", record.name, record.state.word()),
            next_action: Some("a Job that has ended reaches no further verdict".to_string()),
        });
    }

    let attempts = record.attempts.entry(step.to_string()).or_insert(0);
    *attempts += 1;
    let attempts = *attempts;

    record.step = step.to_string();
    record.verdict = Some(reached);
    record.state = reached.settles_to();

    match reached {
        Verdict::Blocked => {
            raise(
                place,
                now,
                &record,
                inbox::Kind::Blocked,
                &format!("`{step}` is blocked and cannot proceed without an external change"),
            )?;
        }
        Verdict::NeedsHuman => {
            raise(
                place,
                now,
                &record,
                inbox::Kind::NeedsHuman,
                &format!("`{step}` reached a judgement call"),
            )?;
        }
        Verdict::Pass | Verdict::Failed => {}
    }
    store.save(&record)?;

    Ok(Output::Verdict(Box::new(Envelope::ok(
        "fleet verdict",
        None,
        Status::Ok,
        VerdictData {
            job: record.name.clone(),
            step: step.to_string(),
            verdict: reached,
            evidence,
            attempts,
            state: record.state,
        },
    ))))
}
