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

use armada_core::config::LandMerge;
use armada_core::ctx::{Clock, Run, RunRequest};
use armada_core::envelope::{
    AnswerData, AskData, BoardData, DaemonActRow, Disposition, Envelope, Evidence, FleetLsData,
    GateRow, InboxData, InboxRow, JobRow, KillData, Killed, NoteRow, PauseData, ProbeData,
    ProposeData, ReapCandidate, ReapPlanData, Recorded, ReportData, ResumeData, ShowData,
    SpawnData, StepRow, TickData, TickRow, TransitionRow, VerdictData,
};
use armada_core::error::{ArmadaError, ErrClass, Status};
use armada_core::fleet::classify::Classification;
use armada_core::fleet::drone::Reading;
use armada_core::fleet::job::{self, Handle, Job, Observed, Spend};
use armada_core::fleet::workflow::{self, Workflow};
use armada_core::fleet::{advance, drone as argv, gate, Acting, JobState, Subject, Verdict};
use armada_fleet::drone;
use armada_fleet::jobs::Store;
use armada_fleet::machine as fleet_machine;
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

    /// Write this Job's `Stop` hook and the settings document that registers
    /// it, and answer with what `--settings` should be handed.
    ///
    /// **Rewritten before every exchange rather than written once at spawn**,
    /// for the reason `armada helm` rewrites its own wiring on every launch:
    /// the hook names the `armada` binary on this machine, and a machine whose
    /// Armada moved would otherwise keep relaying through a path that is not
    /// there. Regenerating is two small writes.
    ///
    /// **A hook that could not be written does not fail the exchange.** The
    /// relay is one of two mechanisms (`020` §2) and the sweep is the other, so
    /// a Job that starts without a relay still advances — later, and through
    /// somebody else's tick, which is precisely what a backstop is for. Failing
    /// the spawn instead would trade a slow Job for no Job.
    fn relay(&self, uuid: &str) -> Option<String> {
        let hook = home::stop_hook(&self.armada_home, uuid);
        let settings = home::drone_settings(&self.armada_home, uuid);
        let exe = self.exe.display().to_string();
        std::fs::create_dir_all(hook.parent()?).ok()?;
        std::fs::write(&hook, argv::stop_hook(&exe)).ok()?;
        // **A `Stop` hook that is not executable is a relay that silently is
        // not one** — Claude Code runs it as a command, and a file without the
        // bit set fails to start with nothing on the machine reporting it.
        #[cfg(unix)]
        {
            use std::os::unix::fs::PermissionsExt;
            std::fs::set_permissions(&hook, std::fs::Permissions::from_mode(0o755)).ok()?;
        }
        std::fs::write(&settings, argv::settings_json(&hook.display().to_string())).ok()?;
        Some(settings.display().to_string())
    }

    /// The `--mcp-config` document that attaches Armada's own server, written
    /// beside the relay and for the same reason.
    ///
    /// **Without it a Drone cannot report at all.** Measured 2026-08-16: a
    /// Drone's session advertised 103 tools and none of them Armada's, so the
    /// four `mcp__armada__fleet_*` tools its brief names — and that
    /// `drone::ALLOW` grants — did not exist to be called. The Job did its
    /// work, said so in prose, and stopped `SILENT`.
    ///
    /// The document is `helm::mcp_json`'s, unchanged: one server, one command,
    /// and `ARMADA_JOB` in the environment is what makes `armada mcp serve`
    /// answer with the Drone's belt rather than Helm's. A second document
    /// spelling the same server differently is the drift `glossary.md` exists
    /// to prevent.
    fn drone_mcp(&self, uuid: &str) -> Option<String> {
        let path = home::drone_mcp(&self.armada_home, uuid);
        std::fs::create_dir_all(path.parent()?).ok()?;
        let exe = self.exe.display().to_string();
        std::fs::write(&path, armada_core::helm::mcp_json(&exe)).ok()?;
        Some(path.display().to_string())
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
fn look<R: Run>(run: &R, place: &Where, record: &Job, now_ms: u64) -> (Observed, Reading, bool) {
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
    // **`alive` comes back out rather than being asked a second time.**
    // `armada fleet tick` needs it as a fact of its own — a Job whose Drone is
    // still working is not gated — and [`observe`] folds it into a state, which
    // is lossy: `RUNNING` is both "a Drone is alive" and "a turn finished
    // cleanly and nothing is running". Those are the two cases the loop has to
    // tell apart, and a second `ps` to re-ask would be a second answer that
    // could disagree with this one.
    (observed, reading, alive)
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
                // **The entry says the keystroke that clears it**, which is
                // `027`'s rule and the one this message used to break: *reached
                // its iterations ceiling* was the sentence with no way out, and
                // a reader who found it had to already know that `board` and
                // `kill` were the only two moves. Now there is a third and it is
                // the one they want, so the row carries it.
                // **The keystroke leads, because the sentence is elided from the
                // right.** This said the fact first and the action last, and
                // `armada fleet inbox` truncates a body to its column — so the
                // row read *"reached its wall c…"* and the half telling you what
                // to type was the half that got cut. Reported by the owner, who
                // could see that a Job wanted something and not what.
                //
                // An action a reader cannot see is `027`'s whole subject, and
                // putting it after the explanation is how a message obeys that
                // rule in full text and breaks it on screen.
                &format!(
                    "answer `{}=…` — it reached its {} ceiling on the {} step",
                    match ceiling {
                        job::Ceiling::Attempts => "max_attempts",
                        job::Ceiling::Cost => "max_cost",
                        job::Ceiling::WallClock => "max_wall_clock",
                    },
                    ceiling.word(),
                    record.step,
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
            "its Drone is gone and nothing ticked it",
        )?;
    } else if observed.state == JobState::Silent && was != JobState::Silent {
        // **The same rule, and a different sentence** (`020` §6). A silent Job
        // is not a stalled one: there is no signal to act on anywhere, which is
        // the thing the reader has to be told rather than left to infer from a
        // word that means the other failure.
        raise(
            place,
            now,
            record,
            inbox::Kind::Blocked,
            "its Drone ended an exchange without a verdict or a question",
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
    // **Read and refused here, before anything is created.** `fleet.carry`
    // in `~/.armada/machine.yml` is this machine's declaration of which
    // untracked paths this repository needs in every worktree; a path that
    // escapes the repository root is `bad_config` at the moment the
    // declaration is read, not discovered later while copying into a
    // worktree that already exists.
    let carry = fleet_machine::carry_for(&place.armada_home, &place.shown(&repo_root))?;

    // **All four rows from the first frame**, exactly as `check` plans its
    // table by name rather than by count: a spawn stuck making a worktree
    // should show which step that is and that three more are coming, not a
    // number. None of them has a configured deadline, so none claims one —
    // `render/live.rs` leaves that cell empty rather than printing `timeout 0s`.
    //
    // **Except under `--dry-run`, which plans the one step it takes.** A live
    // table listing three steps a preview will never reach is the same untruth
    // the final table used to tell, drawn a few hundred milliseconds earlier.
    let planned: Vec<Planned<'_>> = match options.dry_run {
        true => vec![Planned {
            id: SpawnStep::Classify.id(),
            timeout_ms: None,
        }],
        false => SpawnStep::ALL
            .iter()
            .map(|step| Planned {
                id: step.id(),
                timeout_ms: None,
            })
            .collect(),
    };

    let classify_from = now.mono();
    // **The table opens before the classifying call, not after it.** Every other
    // step of a spawn finishes in well under a second; classification is the
    // whole of the wait — 7.5s in the run that reported this, a measured 20.6s
    // for a one-line task — and until this it happened before there was any
    // table, so the one part of a spawn a person waits through was the one part
    // that reported nothing. `armada fleet spawn` looked hung and then printed
    // everything at once, which is exactly what it looked like.
    progress.begin(Shape::Spawn, &planned, classify_from);
    progress.started(SpawnStep::Classify.id());
    let (guessed, classify_ms) = classify(run, now, &repo_root, options, &mut || {
        progress.tick(now.mono())
    })?;
    let classify_to = now.mono();
    progress.tick(classify_to);

    // **The table is given back before anybody is asked, and taken again after.**
    // `ask/select.rs` reserves an inline viewport on stderr of its own, and two
    // of them on one stream is a corrupted screen — which is why this used to be
    // solved by not opening the table until the interview was over. Closing it
    // for the duration keeps that property without paying for it with silence on
    // every confident spawn, which is nearly all of them.
    let interviewing = !guessed.is_confident(settle_threshold(options));
    if interviewing {
        progress.finish();
    }
    // Blocks on a person when the classification was a guess, which is why the
    // reading above it is taken before this line and not after: the time
    // somebody took to answer is not time spent classifying.
    let classification = settle_workflow(guessed, options, ask)?;
    if interviewing {
        // Reopened from the same starting reading, so the classify row still
        // reports the interval the call took rather than restarting its clock.
        progress.begin(Shape::Spawn, &planned, classify_from);
        progress.started(SpawnStep::Classify.id());
        progress.tick(classify_to);
    }
    let (classified, role) =
        crate::render::spawn_classified(&classification.workflow, classification.confidence);
    progress.finished(
        SpawnStep::Classify.id(),
        Reached::Word(SpawnStep::Classify.done(), role),
        Some(&classified),
    );
    let workflow = read_workflow(place, &classification.workflow)?;
    // **Every workflow it will reach has to exist before anything is spawned.**
    // A `review` or a sub-Job step resolves its workflow at the moment its gate
    // is reached, which is several paid exchanges in: a `bug` Job finds out its
    // guild has no `review` only after `reproduce` and `fix` have run, and the
    // money is spent whatever the reader does next. The check costs a directory
    // read; the alternative costs a Job.
    reachable_workflows_exist(place, &workflow)?;
    reachable_skills_exist(place, &repo_root, &workflow)?;
    the_projection_is_current(place)?;
    let budget = workflow::override_budget(workflow.budget, &options.budget)?;
    // The keys, not the values: a value is this Job's ceiling and a key is the
    // caller saying *this tree, not the default*.
    let budget_set: Vec<String> = options
        .budget
        .iter()
        .filter_map(|pair| pair.split_once('=').map(|(key, _)| key.to_string()))
        .collect();

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
        budget_set,
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
        waited_ms: 0,
        waiting_from_ms: None,
        transitions: Vec::new(),
        pending: None,
        facts: options.set.clone(),
        // A Job a person asked for has no parent and nothing to credit.
        kin: job::Kin::default(),
        ticked_turns: 0,
        doing: None,
        daemon_acts: Vec::new(),
        main_moved_at: None,
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
    progress.tick(started);
    progress.started(SpawnStep::Worktree.id());
    // **From the repository's own `HEAD`.** A Job a person asked for starts
    // where their checkout is; only a sub-Job names a start point, because only
    // a sub-Job is about work that is already on another branch
    // ([`spawn_child`]).
    if let Err(error) = worktree::add(run, &repo_root, &path, &branch, None) {
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
    // **Carried before `armada manifest init` runs**, because `setup:` may
    // depend on a path this repository declared (`docs/commands/fleet/spawn.md`).
    // `carry` was already refused at read time if it named anything unsafe
    // (above), so this only copies what is trusted and present.
    if let Err(error) = worktree::carry(&repo_root, &path, &carry) {
        let _ = manifest::clean(run, &place.exe, &path);
        let _ = worktree::remove(run, &repo_root, &path);
        let _ = worktree::delete_branch(run, &repo_root, &branch);
        record.state = JobState::Aborted;
        record.verdict = Some(Verdict::Failed);
        store.save(&record)?;
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
    // **A failed `init` aborts the spawn and cleans up, exactly as a failed
    // worktree does** (`docs/commands/fleet/spawn.md`: *"`1` `tool_failed` — the
    // worktree, the carry copy, or `manifest init` failed"*).
    //
    // It never did, because `manifest::init` read `data.port_block` and dropped
    // the envelope's `error` — so a repository whose `setup:` failed was
    // reported here as a workspace that claims no ports, this row was drawn
    // grey-but-fine, and a Drone was started into a worktree with no working
    // environment. Which is what happened to this repository on every spawn it
    // has ever done (job `armada-failed`, 2026-08-17).
    let block = match manifest::init(run, &place.exe, &path) {
        Ok(block) => block,
        Err(error) => {
            let _ = manifest::clean(run, &place.exe, &path);
            let _ = worktree::remove(run, &repo_root, &path);
            let _ = worktree::delete_branch(run, &repo_root, &branch);
            record.state = JobState::Aborted;
            record.verdict = Some(Verdict::Failed);
            store.save(&record)?;
            close_entries(place, &record)?;
            return Err(error);
        }
    };
    record.port_block = block;
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
            &prompt(&workflow, &step, &options.task, &options.set),
            &place.posture()?,
            // **The relay, written before the first Drone starts** (`020` §1).
            // Nothing observed an exchange ending until this line existed.
            place.relay(&uuid).as_deref(),
            place.drone_mcp(&uuid).as_deref(),
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
    tick: &mut dyn FnMut(),
) -> Result<(Classification, Option<u64>), ArmadaError> {
    match &options.workflow {
        // **No call at all for an override.** Classification is one cheap call
        // per spawn and its cost is the one that compounds; spending it to
        // confirm an answer the caller already gave would be the one avoidable
        // token in the whole verb.
        Some(named) => Ok((Classification::overridden(named), None)),
        None => {
            let started = now.mono();
            let classified = drone::classify(run, repo_root, &options.task, tick)?;
            Ok((classified, Some(now.mono().saturating_sub(started))))
        }
    }
}

/// The confidence this spawn settles at, and the one place it is decided.
///
/// **Read twice and therefore not typed twice.** [`settle_workflow`] uses it to
/// decide whether to ask, and [`spawn`] uses it to decide whether to give the
/// terminal back before it does — and a spawn that closed its live table for an
/// interview that never happened, or held it open through one, would be the two
/// halves disagreeing about a number.
fn settle_threshold(options: &Spawn) -> f64 {
    options
        .confidence
        .unwrap_or(armada_core::fleet::classify::CONFIDENT)
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
    if guessed.is_confident(settle_threshold(options)) {
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

/// What gates one step, when the guild can be read.
///
/// **Best-effort, and the absence is recorded rather than defaulted.** A
/// workflow lives in the guild and a guild can be absent, half-synced or
/// renamed; answering `always` for a step nobody could look up would invent the
/// one fact the caller wanted. Neither `show` nor `verdict` fails over this —
/// a Job whose guild has gone is exactly the Job somebody is trying to read.
fn gate_of(place: &Where, workflow: &str, step: &str) -> Option<workflow::Verify> {
    read_workflow(place, workflow)
        .ok()?
        .steps
        .iter()
        .find(|candidate| candidate.id == step)
        .map(|candidate| candidate.verify.clone())
}

/// Refuse before spawning if any workflow this one can reach is missing.
///
/// **Transitive, because a referenced workflow may reference another.** The
/// depth bound is the same one the gate enforces at run time; a guild whose
/// workflows form a cycle is refused there and this walk simply stops rather
/// than duplicating that argument in a second place.
///
/// The error is [`read_workflow`]'s own, unchanged — it already names
/// `armada guild upgrade`, and a second wording for one condition is the drift
/// `docs/glossary.md` exists to prevent.
fn reachable_workflows_exist(place: &Where, from: &Workflow) -> Result<(), ArmadaError> {
    let mut seen: Vec<String> = vec![from.name.clone()];
    let mut queue: Vec<Workflow> = vec![from.clone()];
    while let Some(flow) = queue.pop() {
        for step in &flow.steps {
            // A step names its sub-Job's workflow outright; `review_clean`
            // names none and means the reviewer, which is a constant so that a
            // Job under review cannot choose its own examiner.
            let named = step.workflow.clone().or_else(|| {
                (step.verify.must == workflow::Predicate::ReviewClean)
                    .then(|| workflow::REVIEWER.to_string())
            });
            let Some(name) = named else { continue };
            if seen.contains(&name) {
                continue;
            }
            seen.push(name.clone());
            queue.push(read_workflow(place, &name)?);
        }
    }
    Ok(())
}

/// Refuse to spawn under a projection the guild has moved past.
///
/// # A Drone reads the projection, not the guild
///
/// `~/.armada/guild/` is the source and `~/.claude/` is what Claude Code
/// actually loads, and `armada guild project` is what carries one to the other.
/// A Drone resolving `implement-change` reads the projected copy, so a guild
/// edited and not projected is a guild whose change no Drone will ever see.
///
/// **Measured 2026-08-17, on a fix that mattered.** The `review-diff` skill was
/// changed so a blocking review calls `fleet.ask_human` and stops the Job —
/// `review_clean` could not fail without it. The change was written to the
/// template and to the guild, verified in both, checked and merged. The next
/// reviewer wrote *"One thing blocks landing this"*, named a real defect, and
/// finished `PASS` anyway. The projected copy was eight hours old and contained
/// none of it. The fix was inert and nothing said so.
///
/// # Refuse rather than project
///
/// Projecting here would have `armada fleet spawn` write `~/.claude/` as a side
/// effect, which is the operator's own configuration and not this verb's to
/// touch. So this refuses and names the one command that fixes it — the same
/// shape as the missing workflow and missing skill guards above, and for the
/// same reason: the check costs a directory read and the alternative costs a
/// Job that runs under instructions nobody meant to give it.
///
/// **`armada doctor` has reported this all along** — *"STALE ~/.claude: 1 file
/// not what the guild says"* — and a diagnostic nobody runs at the moment it
/// matters is a diagnostic that gets read past. This is the same finding, at
/// the moment it costs something.
fn the_projection_is_current(place: &Where) -> Result<(), ArmadaError> {
    let guild = Guild::at(&place.armada_home);
    let claude = place.home.join(".claude");
    let behind: Vec<String> = armada_guild::projector::survey(&guild, &claude, &place.armada_home)
        .into_iter()
        .filter(|step| step.writes() || step.deletes())
        .map(|step| step.at)
        .collect();
    match behind.is_empty() {
        true => Ok(()),
        false => Err(ArmadaError {
            class: ErrClass::BadConfig,
            r#where: place.shown(&claude),
            message: format!(
                "{} in your guild {} not been projected, and a Drone reads the projection",
                crate::render::format::count(behind.len(), "change"),
                match behind.len() {
                    1 => "has",
                    _ => "have",
                }
            ),
            next_action: Some(format!(
                "`armada guild project` writes {}",
                behind
                    .iter()
                    .take(3)
                    .cloned()
                    .collect::<Vec<_>>()
                    .join(", ")
            )),
        }),
    }
}

/// Every skill a reachable workflow names, and where each was looked for.
///
/// # Why this exists beside the workflow check and not inside it
///
/// `reachable_workflows_exist` refuses a Job whose reachable *workflow* is
/// missing, because a missing `review` workflow stopped Jobs dead. There was no
/// equivalent for a **skill**, and the shipped starters named seven while
/// `guild init` wrote two — so every step of every workflow handed its Drone a
/// prompt reading *"Use the `implement-change` skill"* for a file that was not
/// there, in six cases out of seven.
///
/// Nothing broke visibly, because the step's task text also describes the work
/// and a Drone proceeds. What it did instead was teach every Drone to ignore
/// the first line of its own prompt. Found by a Drone that went looking for the
/// file and said so through `fleet.propose`.
///
/// # Two places, because a skill legitimately comes from either
///
/// **Armada never parses a skill file** — `prompt` hands the Drone a name and
/// the Drone resolves it in its own worktree, *"which is what makes the repo's
/// version win a collision"* (PLAN.md §14.5). So a name absent from the guild
/// is not missing if the repository being worked on supplies it, and a guard
/// that looked only at the guild would refuse Jobs that would have worked.
///
/// Both places are named in the refusal, because *"no such skill"* about a name
/// the repository was expected to provide sends the reader to the wrong file.
fn reachable_skills_exist(
    place: &Where,
    repo_root: &std::path::Path,
    from: &Workflow,
) -> Result<(), ArmadaError> {
    let guild = Guild::at(&place.armada_home);
    let mut seen: Vec<String> = Vec::new();
    let mut queue: Vec<Workflow> = vec![from.clone()];
    let mut flows: Vec<String> = vec![from.name.clone()];
    while let Some(flow) = queue.pop() {
        for step in &flow.steps {
            if let Some(skill) = &step.skill {
                if seen.contains(skill) {
                    continue;
                }
                seen.push(skill.clone());
                let in_guild = guild.path(&format!("skills/{skill}/SKILL.md"));
                let in_repo = repo_root.join(format!(".claude/skills/{skill}/SKILL.md"));
                if !in_guild.exists() && !in_repo.exists() {
                    return Err(ArmadaError {
                        class: ErrClass::BadConfig,
                        r#where: format!("workflows/{}.yml", flow.name),
                        message: format!(
                            "the `{}` step names a skill called `{skill}`, and there is none",
                            step.id
                        ),
                        next_action: Some(format!(
                            "looked in {} and {}; `armada guild upgrade` adds what a later                              release added",
                            place.shown(&in_guild),
                            place.shown(&in_repo)
                        )),
                    });
                }
            }
            let named = step.workflow.clone().or_else(|| {
                (step.verify.must == workflow::Predicate::ReviewClean)
                    .then(|| workflow::REVIEWER.to_string())
            });
            let Some(name) = named else { continue };
            if flows.contains(&name) {
                continue;
            }
            flows.push(name.clone());
            queue.push(read_workflow(place, &name)?);
        }
    }
    Ok(())
}

/// Whether a workflow declares a step by this id.
///
/// **Unreadable guild means yes.** A workflow file that will not parse is a
/// separate failure with its own message, and turning it into *"no such step"*
/// would send a reader to the wrong file — so the guard opens only when it can
/// actually answer.
fn flow_has_step(place: &Where, workflow: &str, step: &str) -> bool {
    match read_workflow(place, workflow) {
        Ok(flow) => flow.steps.iter().any(|declared| declared.id == step),
        Err(_) => true,
    }
}

/// The step ids a workflow declares, in order, for a refusal to name.
fn flow_steps(place: &Where, workflow: &str) -> Vec<String> {
    read_workflow(place, workflow)
        .map(|flow| {
            flow.steps
                .into_iter()
                .map(|declared| format!("`{}`", declared.id))
                .collect()
        })
        .unwrap_or_default()
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
        // **`upgrade` is named as well as `init`, and `review` is why.** A
        // guild made before the reviewer workflow shipped has the four
        // starters and not the fifth, and the Job that finds out is one whose
        // `review_clean` gate has just gone looking for it — so a next action
        // that only offered `init` would send somebody with a working guild to
        // a verb that refuses to touch it (`docs/reserved/006`).
        next_action: Some(format!(
            "`armada guild init` writes the starters ({}, {}); \
             `armada guild upgrade` adds what a later release added",
            workflow::STARTERS.join(", "),
            workflow::REVIEWER
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
///
/// # The gate's own resolved terms, stated
///
/// **A Drone was judged against a name it was never given.** A `bug` Job's
/// `reproduce` step gates on `failing_test_exists` with `test: ${task.test}`,
/// which [`gate::resolve`] fills from the Job's facts — and this prompt carried
/// the step, the skill and the task and *not the filled value*. So the Drone
/// wrote whatever test it thought reasonable, the gate went looking for the one
/// the caller had named, found nothing, and reported that the Drone had not
/// written a test. Both were doing exactly as told; only one of them had been
/// told.
///
/// Every `task.*` fact a step's own `verify:` names is stated **resolved** when
/// the Job holds it, and **asked for** when it does not.
///
/// # Why asking is the other half, and why nothing refuses at spawn
///
/// A `${task.<key>}` is a fact by that name, and two things can supply one: the
/// caller, with `armada fleet spawn --set test=…`, and — since this — the Drone
/// itself, with `fleet.report`. So a fact the Job does not hold at spawn is not a
/// misconfiguration to refuse; it is a fact nobody knows yet.
///
/// **Which is the only reading that works for the callers that exist.** `armada
/// failures fix` and `armada tasks start` both put a Job on the `bug` workflow,
/// and neither can name a test: a recorded failure is a stack trace and a report
/// is somebody's observation. Refusing them at spawn would make the primary way
/// into the fleet unusable, and asking a person to invent a test name for a bug
/// nobody has looked at yet is asking them for the answer to the step.
///
/// The caller still wins where there is one: a `--set test=` is a contract stated
/// before the work, so it is stated to the Drone as a contract. Where there is
/// none, the Drone writes the test it thinks reproduces the bug and names it, and
/// Armada verifies that name — `PLAN.md` §5's sandwich, rather than the caller
/// guessing.
fn prompt(
    workflow: &Workflow,
    step: &str,
    task: &str,
    facts: &std::collections::BTreeMap<String, String>,
) -> String {
    let declared = workflow.steps.iter().find(|candidate| candidate.id == step);
    let skill = declared.and_then(|candidate| candidate.skill.clone());
    let mut ask = match skill {
        Some(skill) => format!(
            "Use the `{skill}` skill for the `{step}` step of the `{}` workflow.\n\n\
             Task: {task}",
            workflow.name
        ),
        None => format!(
            "Work the `{step}` step of the `{}` workflow.\n\nTask: {task}",
            workflow.name
        ),
    };

    let Some(declared) = declared else { return ask };
    // The *resolved* verify, so what is stated is the value the gate will look
    // for rather than the placeholder the document was written with.
    let resolved = gate::resolve(declared, facts);
    let unfilled = |value: &Option<String>| value.as_ref().is_some_and(|v| v.contains("${"));

    match (&resolved.verify.test, unfilled(&resolved.verify.test)) {
        // **The Drone was judged against a name it was never given.** The prompt
        // carried the step, the skill and the task and not the filled value, so
        // the Drone wrote whatever test looked reasonable while the gate hunted
        // for the one the caller had named. Both did as told; one was told.
        (Some(test), false) => ask.push_str(&format!(
            "\n\nThis step's gate is `failing_test_exists` and it looks for a test called \
             `{test}`. Write that test, under that name, and make it fail for the reason the \
             task describes — a test by another name does not satisfy it."
        )),
        // Nobody named one, so name it yourself. Without this the gate had
        // nothing to look for and the step could not be decided at all, which is
        // where a live Job stopped after 39 turns.
        (Some(_), true) => ask.push_str(
            "\n\nThis step's gate is `failing_test_exists` and **nobody has named the test \
             yet**. Write the test that fails for the reason the task describes, then name it \
             when you report this step: `fleet.report` takes a `set` of \
             `[\"test=<the test function you wrote>\"]`. Armada searches your worktree for that \
             name and requires the check run to be red — so the name has to be the one in the \
             tree, and the test has to actually fail.",
        ),
        (None, _) => {}
    }

    match (
        &resolved.verify.artifact,
        unfilled(&resolved.verify.artifact),
    ) {
        (Some(path), false) => ask.push_str(&format!(
            "\n\nThis step's gate is `artifact_exists` and it looks for `{path}`. That exact \
             path is what satisfies it."
        )),
        (Some(_), true) => ask.push_str(
            "\n\nThis step's gate is `artifact_exists` and **nobody has named the path yet**. \
             Write the artifact, then name it when you report this step: `fleet.report` takes a \
             `set` of `[\"artifact=<the path you wrote>\"]`, relative to the worktree.",
        ),
        (None, _) => {}
    }
    ask
}

/// Raise an entry, and hand back the id it was given.
///
/// **The id is returned rather than discarded** because `fleet.ask_human` has to
/// name the thing it is waiting on — and because every item a person is asked to
/// act on needs an identity they can acknowledge one row at a time (PLAN.md
/// §15.3.1). The callers that only raise ignore it, which costs nothing.
///
/// # The Job's clock stops here
///
/// **A wait begins when the question is raised**, which is this function and
/// nothing else, and it ends where the entry is answered. Binding it to the
/// *entry* rather than inferring it from the Job's state is what makes it
/// reliable: the first attempt stamped it inside `settle`, which only the
/// mutating verbs call and which `answer` bypasses on its own success paths, so
/// the clock started and never stopped.
///
/// Measured 2026-08-17 across three live Jobs, three different wrong answers:
/// one was `RUNNING` with the wait still open, so its wall clock had frozen
/// permanently; one had sat `PAUSED` for fifty minutes with nothing recorded at
/// all; and one was right. A ceiling that stops being enforced is worse than one
/// that fires early.
///
/// The Job is written by the caller — this only stamps the record it is handed.
fn raise<C: Clock>(
    place: &Where,
    now: &C,
    record: &mut Job,
    kind: inbox::Kind,
    body: &str,
) -> Result<String, ArmadaError> {
    let at_ms = now.wall_ms();
    record.began_waiting(at_ms);
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
    // **Every window this fleet has seen, kept so one can be chosen.** The
    // event is per-transcript — each Job's last turn reports the window *it*
    // passed through — but the window is the account's, so the choice is over
    // the whole listing and is made once, below, by
    // [`armada_core::fleet::drone::window`].
    let mut windows: Vec<armada_core::fleet::drone::RateLimit> = Vec::new();
    // Accumulated per row, because the rows that are filtered out never spent
    // anything this listing is accounting for.
    let mut own_spend: f64 = 0.0;
    for record in place.store().all()? {
        if !all && record.state.is_over() {
            continue;
        }
        let (observed, reading, alive) = look(run, place, &record, wall);
        windows.extend(reading.rate_limits);
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
        // **What this Job spent by itself, before its children are added in.**
        //
        // `observe` reports `own + kin` so that a ceiling bounds the whole tree
        // ([`job::Kin::spend`]), which is right for a ceiling and wrong for a
        // total: summing every row's figure counted each sub-Job twice, once on
        // itself and once inside its parent, and the `$… today` on the Bridge and
        // at the foot of every listing over-reported the fleet by the cost of its
        // entire second generation. Measured on this machine: $180.56 against a
        // real $133.64.
        //
        // **Subtracted rather than summing the roots only.** A parent's roll-up
        // happens when a tick sees the child over, so a parent that was killed
        // while its child was still running never received it — and counting only
        // roots would then lose that child's spend entirely. Every Job's own
        // share is on its own record, so this needs neither assumption.
        own_spend += (observed.spend.cost_usd - record.kin.spend.cost_usd).max(0.0);
        rows.push(JobRow {
            uuid: record.uuid.clone(),
            name: record.name.clone(),
            workflow: record.workflow.clone(),
            state: observed.state,
            detail: detail(&record, observed.state, waiting, alive),
            // **The step on its own, because `detail` is already a fold.** A
            // Job with an open inbox entry shows the entry's body there, so a
            // `STEP` column reading `detail` would go blank on exactly the rows
            // somebody is looking at.
            step: match observed.state {
                JobState::Queued => String::new(),
                _ => record.step.clone(),
            },
            on_step_s: job::on_step_since_ms(&record.transitions, &record.step)
                .map(|since| wall.saturating_sub(since) / 1_000),
            // **Carried, never re-read.** The Bridge draws a `TASK` column and
            // is a renderer over this listing — a second pass over the Job index
            // to fill one column would be the second source
            // `commands/helm/bridge.md` rules out.
            task: record.task.clone(),
            runtime_s: run_time / 1_000,
            cost_usd: observed.spend.cost_usd,
            tokens: observed.spend.tokens,
            turns: observed.spend.turns,
            budget_remaining: job::remaining(
                &record.budget,
                &observed.spend,
                run_time,
                record.attempts_on_step(),
            ),
            needs_attention: wants_you,
            // **Carried off the record, which is the only place it exists.**
            // `observe` cannot derive it: an action with a duration is a thing
            // somebody is doing, not a thing the transcript or the process
            // table can be asked about.
            acting: record.doing.clone(),
            // **Measured here, against this reader's clock**, for the reason
            // `on_step_s` two fields up is: the process performing the action is
            // blocked inside it and cannot know when anybody will look, so it
            // writes down when the stage began and the process *looking* does
            // the subtraction. That is the whole of `020` §5's mechanism — the
            // terminal running the abort is stuck in `manifest clean`, so the
            // only thing that can report `docker 12s` is another reader of the
            // same record.
            acting_for_s: record
                .doing
                .as_ref()
                // **Only once a stage has been named.** A `Doing` with no
                // `slow` has a `since_ms` that dates the action rather than a
                // stage, and ageing an unnamed thing is a number with nothing
                // to attach it to.
                .filter(|doing| doing.slow.is_some())
                .map(|doing| doing.elapsed_ms(wall) / 1_000),
            // **Off the record, where `016` put it.** A sub-Job knows which Job
            // started it, which step of that Job's workflow it satisfies and
            // which attempt it belongs to; the parent keeps no list of children,
            // because *"what did I start"* is asked while walking the whole index
            // anyway — and a listing is exactly that walk.
            parent: record.kin.parent.clone(),
            // Set by `as_a_tree`, which is where the order is decided.
            depth: 0,
        });
    }

    let rows = armada_core::envelope::as_a_tree(rows);
    let needs_you = rows.iter().filter(|row| row.needs_attention).count();
    let spent_usd = own_spend;
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
            // **Turned into a countdown here, in the shell, because this is
            // where the clock is.** The core picks which window is in force and
            // the payload carries `resets_in_s`, so neither the renderer nor a
            // `--json` consumer has to know what epoch second it is to read the
            // one number that says when you can work again.
            windows: armada_core::fleet::drone::windows(&windows, wall / 1_000)
                .into_iter()
                .map(|limit| armada_core::envelope::Window {
                    kind: limit.kind.clone(),
                    used_percent: limit.percent(),
                    resets_in_s: limit.resets_at.map(|at| at.saturating_sub(wall / 1_000)),
                })
                .collect(),
        },
    ))))
}

/// The one thing a state word cannot say: which step, and what it is waiting on.
fn detail(record: &Job, state: JobState, waiting: Option<&inbox::Entry>, alive: bool) -> String {
    match waiting {
        Some(entry) => entry.body.clone(),
        None if state == JobState::Queued => String::new(),
        // **A Job holding a gate with no Drone to relay it says what it needs.**
        //
        // Measured 2026-08-17: `job-drives-the-drone` sat at `implement` for over
        // an hour holding a check that had passed 5 of 5. Nothing was wrong with
        // the Job, the gate or the check — the answer was there and nobody had
        // asked for it, and one `armada fleet tick` advanced it at once. The row
        // said `RUNNING` and `implement`, which is a Job working on a step, so
        // there was nothing on the screen to suggest a keystroke would end it.
        //
        // **The relay is the Drone's `Stop` hook** (`docs/reserved/024`), which
        // fires when a Drone stops *cleanly*. A `SIGKILL`, a crash and a failing
        // hook all break the chain in silence, so this is the ordinary end of a
        // Job rather than a rare one.
        //
        // **It does not ask whether the gate has settled.** That means
        // `armada manifest check --status`, which is a process, and this line is
        // drawn for every row of a listing the Bridge redraws every two seconds —
        // `verbs/bridge.rs` promises that redraw is a directory read, a
        // transcript tail and a `ps`. The keystroke is right either way: a
        // settled gate advances and an unsettled one is reported as still going.
        //
        // **Naming the sweep rather than performing it.** Nothing calls the
        // backstop sweep and `ARCHITECTURE.md` forbids a daemon, so what
        // invokes `armada fleet tick` is a decision for the machine's owner
        // (task 34a8afa8). Until it is made, the screen at least stops implying
        // that waiting is the answer.
        None if !alive => match record.pending.as_ref().map(|pending| &pending.on) {
            // **"waiting on", not "holding".** `holding` was the first word and it
            // over-promises: it reads as *the answer is here*, and the common
            // case is a check that is still running, where a tick reports
            // `WAITING` and the reader has learnt nothing. Found by using it —
            // `wire-bridge` drew `holding a check` while its check had 20s left.
            // The keystroke is still right, because it is right either way; what
            // changed is the claim beside it.
            Some(job::Waiting::Check(_)) => "waiting on a check — `arm fleet tick`".to_string(),
            Some(job::Waiting::SubJob(_)) => "waiting on a sub-Job — `arm fleet tick`".to_string(),
            // An `Answer` is the one gate a tick cannot settle: it is waiting on a
            // person, the inbox entry above is how they answer, and this row is
            // reached only when that entry is already closed.
            Some(job::Waiting::Answer(_)) | None => record.step.clone(),
        },
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
    // # An inbox entry id resolves here, exactly as it does at `answer`
    //
    // **An id you can act on should be an id you can read.** `answer` tries the
    // handle as an entry before trying it as a Job — that is deliberate and
    // `docs/reserved/001` argues it: naming the row says exactly which question
    // you mean, where naming the Job says it only by accident. `show` never
    // learned the same trick, so an id copied off the `ID` column that
    // `armada fleet inbox` prints was taken by one verb and rejected by its
    // neighbour, with a refusal reading *"no Job called c530ce2a"* about
    // something nobody claimed was a Job.
    //
    // Five recorded failures on one machine were this — `c530ce2a`,
    // `a058890c`, `f05ac0bf`, `b316971b` — each one a person or an agent
    // reading the inbox and reaching for the wrong verb, and each one sent
    // looking for the wrong mistake.
    //
    // **Entry first, Job second, and never both**, which is `answer`'s own
    // ordering: the two id spaces cannot be merged, so the order is the
    // decision. Nothing that worked stops working, because a Job handle has
    // never been an entry's uuid.
    let handle = match inbox::find_open(&entries(place)?, handle) {
        Ok(Some(entry)) => entry.job_uuid.clone().unwrap_or_else(|| handle.to_string()),
        _ => handle.to_string(),
    };
    let record = place.store().find(&handle)?;
    let wall = now.wall_ms();
    let (observed, _, _) = look(run, place, &record, wall);
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
            // **Off the record and aged against this reader's clock**, exactly
            // as `ls` does it two hundred lines up — the pane is what opens on
            // `d` over a row that has just started saying `ABORTING`, and it
            // reads the same field for the same reason.
            acting: record.doing.clone(),
            acting_for_s: record
                .doing
                .as_ref()
                .filter(|doing| doing.slow.is_some())
                .map(|doing| doing.elapsed_ms(wall) / 1_000),
            drone_pgid: record.drone.as_ref().map(|handle| handle.pgid),
            drone_alive: drone::alive(
                run,
                &place.armada_home,
                record.drone.as_ref(),
                &place.boot_id,
            ),
            step: record.step.clone(),
            attempt: record.attempts.get(&record.step).copied().unwrap_or(0),
            on_step_s: job::on_step_since_ms(&record.transitions, &record.step)
                .map(|since| wall.saturating_sub(since) / 1_000),
            // **Why it is still here**, which no other field answers: a step
            // advances when its predicate holds, and a stuck Job read without
            // its gate is a symptom with the cause missing.
            gate: gate_of(place, &record.workflow, &record.step).map(|verify| GateRow {
                must: verify.must.word().to_string(),
                test: verify.test.clone(),
                artifact: verify.artifact.clone(),
                answered_by_a_person: verify.must.answered_by_a_person(),
            }),
            // **Newest first, the same way `progress` is** — both are logs, and
            // the useful end of a log is the last thing that happened.
            transitions: record
                .transitions
                .iter()
                .rev()
                .map(|entry| TransitionRow {
                    at: entry.at.clone(),
                    ago_s: wall.saturating_sub(entry.at_ms) / 1_000,
                    step: entry.step.clone(),
                    event: entry.event.word().to_string(),
                    attempt: entry.attempt,
                    must: entry
                        .gate
                        .as_ref()
                        .and_then(|gate| gate.must)
                        .map(|must| must.word().to_string()),
                    evidence: entry
                        .gate
                        .as_ref()
                        .map(|gate| gate.evidence.clone())
                        .unwrap_or_default(),
                })
                .collect(),
            // **Newest first, the same as `transitions`** — both are logs of
            // what already happened, off the Job's own record and no new I/O
            // (`034` §6.5: "surfaced by `armada fleet show`").
            daemon_acts: record
                .daemon_acts
                .iter()
                .rev()
                .map(|act| DaemonActRow {
                    at: act.at.clone(),
                    ago_s: wall.saturating_sub(act.at_ms) / 1_000,
                    act: act.act.word().to_string(),
                    target: act.target.clone(),
                    outcome: act
                        .outcome
                        .as_ref()
                        .map(|outcome| outcome.word().to_string()),
                    outcome_detail: act
                        .outcome
                        .as_ref()
                        .and_then(|outcome| outcome.detail())
                        .map(str::to_string),
                    outcome_at: act.outcome_at.clone(),
                })
                .collect(),
            main_moved_at: record.main_moved_at.clone(),
            task: record.task.clone(),
            runtime_s: run_time / 1_000,
            created_at: record.created_at.clone(),
            cost_usd: observed.spend.cost_usd,
            tokens: observed.spend.tokens,
            turns: observed.spend.turns,
            budget: record.budget,
            budget_remaining: job::remaining(
                &record.budget,
                &observed.spend,
                run_time,
                record.attempts_on_step(),
            ),
            repo: record.repo.clone(),
            branch: record.branch.clone(),
            worktree: record.worktree.clone(),
            port_block: record.port_block,
            needs_attention: waiting_on_you,
            asked,
            progress,
            steps: step_rows(
                place,
                &record.workflow,
                &record.step,
                record
                    .doing
                    .as_ref()
                    .map_or_else(|| observed.state.word(), |doing| doing.acting.word()),
                &record.transitions,
            ),
        },
    ))))
}

/// The workflow's declared step order, each one's gate and where it stands —
/// [`ShowData::steps`], the WORKFLOW panel's own table
/// (`docs/reserved/033-the-command-centre-designed.md`).
///
/// **A count in the list, the declared order here** — never a fraction: a step
/// is `PASS`, the current one carries the Job's own status word, and
/// everything after it is `QUEUED` because it has not been entered.
///
/// Empty when the workflow document could not be read, the same case
/// [`gate_of`] is already `None` for.
fn step_rows(
    place: &Where,
    workflow: &str,
    current_step: &str,
    current_word: &str,
    transitions: &[job::Transition],
) -> Vec<StepRow> {
    let Ok(document) = read_workflow(place, workflow) else {
        return Vec::new();
    };
    document
        .steps
        .iter()
        .map(|step| {
            let completed = transitions
                .iter()
                .any(|entry| entry.step == step.id && entry.event == job::StepEvent::Completed);
            let status = if completed {
                "PASS".to_string()
            } else if step.id == current_step {
                current_word.to_string()
            } else {
                "QUEUED".to_string()
            };
            StepRow {
                id: step.id.clone(),
                status,
                must: step.verify.must.word().to_string(),
            }
        })
        .collect()
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
        // **A Job's children go with it, and they go first**
        // (`docs/reserved/016` §2). A killed parent's sub-Job would otherwise
        // keep a Drone, a worktree and a port block, spending a budget for a
        // verdict nothing will ever read — and it would go on reporting to a
        // record that says `ABORTED`. Children first, because the teardown
        // order inside [`end`] is the point: the child's Drone is stopped and
        // its worktree released before the branch it was created from is.
        Some(handle) => {
            // **The one lookup that refuses a shared name rather than resolving
            // it.** Every other verb takes the live Job, because that is the one
            // a person reading the table means; ending the wrong Job cannot be
            // undone by re-running the command, so this one asks.
            let named = store.find_to_end(handle)?;
            let mut family = descendants(&store.all()?, &named.uuid);
            family.retain(|child| !child.state.is_over());
            family.reverse();
            family.push(named);
            family
        }
        // **`--all-finished` asks the observation, not the record.** A Job whose
        // Drone finished while nobody was looking is finished, and a record that
        // still says `RUNNING` is only what a verb last wrote.
        None => store
            .all()?
            .into_iter()
            .filter(|record| {
                let (observed, _, _) = look(run, place, record, wall);
                observed.state.is_over()
                    || matches!(observed.state, JobState::Paused | JobState::Stalled)
            })
            .collect(),
    };

    end(
        run,
        now,
        place,
        Acting::Aborting,
        targets,
        keep_branch,
        keep_worktree,
    )
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
    acting: Acting,
    targets: Vec<Job>,
    keep_branch: bool,
    keep_worktree: bool,
) -> Result<Output, ArmadaError> {
    let mut results: Vec<Killed> = Vec::new();
    for mut record in targets {
        // Read before the record is borrowed for the teardown.
        let already_over = record.state.is_over().then_some(record.state);
        results.push(tear_down(
            run,
            now,
            place,
            &mut record,
            Ending {
                // **A Job that already finished keeps its verdict.**
                //
                // `kill` gave up on this Job, whatever it was doing, so
                // `ABORTED` is the truth for it. `reap` shares this path and
                // means something else entirely: it releases what a Job that is
                // *already over* still holds. Writing `ABORTED` over a `DONE`
                // record destroys the one durable answer to *did this work* —
                // and it is the answer a person reads weeks later, long after
                // the worktree is gone.
                //
                // Measured 2026-08-17: one `armada fleet reap --yes` turned
                // seven finished Jobs into aborted ones, including
                // `table-binds-cells-to-columns`, which had passed its
                // `check_passes` gate, passed `review_clean`, landed on its
                // branch, and been merged into `main` an hour earlier. Its
                // record said `ABORTED/land` while its work was in the history.
                //
                // Releasing resources and rewriting a verdict are different
                // acts, and only one of them was asked for.
                state: already_over.unwrap_or(JobState::Aborted),
                keep_branch,
                keep_worktree,
                observe: true,
                acting: Some(acting),
            },
        )?);
    }

    let error = results.iter().find_map(|killed| killed.error.clone());
    let data = KillData { results };
    Ok(Output::Kill(Box::new(match error {
        Some(error) => Envelope::failed("fleet kill", None, error, data),
        None => Envelope::ok("fleet kill", None, Status::Clean, data),
    })))
}

/// How a teardown ends, which is the only thing that differs between its
/// callers.
#[derive(Debug, Clone, Copy)]
struct Ending {
    /// The state to write. `ABORTED` when somebody gave up on the Job,
    /// `DONE` when it reached its last step — **and a teardown that wrote one
    /// for the other would be the record lying about what happened**.
    state: JobState,
    /// Keep the commits. Always true for a Job that finished: its branch is the
    /// entire reason it was run.
    keep_branch: bool,
    /// Keep the directory.
    keep_worktree: bool,
    /// Look at the Job and persist what was seen before ending it.
    ///
    /// `kill` needs this: it may be ending a Job that stalled or hit a ceiling
    /// while nobody was looking, and after this the observation is no longer
    /// derivable. The loop does not — it has just gated the step and written the
    /// verdict, so a second observation would only be a chance to disagree with
    /// itself.
    observe: bool,
    /// The word a row shows while this teardown runs (`020` §5).
    ///
    /// **It differs by caller and nothing else does**: `kill` is `ABORTING`,
    /// `reap` is `REAPING`, and the loop's own finishing pass is neither —
    /// there is nothing to watch on a Job whose last step just passed, and a
    /// `REAPING` on it would name an action nobody took.
    acting: Option<Acting>,
}

/// **One Job's teardown, and the one of it.**
///
/// The order is the point (`commands/fleet/kill.md`): the Drone first because it
/// is still working, then `manifest clean` so resources are released while the
/// config describing them is still present, then the worktree, then the branch.
/// A second copy of this would be a second answer to what Armada orders and what
/// it tolerates — so `kill`, `reap` and the loop's own finishing pass all arrive
/// here, and differ only in [`Ending`].
///
/// **Nothing here is raised.** `kill`'s documented contract is that the Job is
/// marked ended either way; what would not release is carried on the row, and
/// ownership is recorded machine-globally so `armada manifest clean --all`
/// reclaims the remainder.
fn tear_down<R: Run, C: Clock>(
    run: &R,
    now: &C,
    place: &Where,
    record: &mut Job,
    ending: Ending,
) -> Result<Killed, ArmadaError> {
    let store = place.store();
    let wall = now.wall_ms();
    {
        let path = place.expand(&record.worktree);

        // **What it was doing is recorded before it is ended.** A Job that
        // stalled or hit a ceiling while nobody was looking has that written
        // down and raised here, because after this the observation is no longer
        // derivable.
        if ending.observe {
            let (observed, _, _) = look(run, place, record, wall);
            settle(record, &observed, place, now)?;
        }

        // **The row says what is happening to it, from here on** (`020` §5).
        // The abort that started this rule took several seconds inside docker
        // and said nothing; every stage below now names itself in the record
        // before it starts, so a second reader — the Bridge, another terminal —
        // draws `ABORTING · docker 12s…` instead of a row that looks hung.
        //
        // **Written before the slow thing, not after.** A stage announced on
        // completion is a stage nobody could see while it was the problem.
        let started = ending
            .acting
            .map(|acting| job::Doing::started(acting, now.wall_ms()));
        let mut doing = stage(&store, record, started.as_ref(), "drone", now.wall_ms());

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
                // The stage the reader watched in silence for several seconds.
                doing = stage(&store, record, doing.as_ref(), "docker", now.wall_ms());
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
        let disposition = match (ending.keep_worktree, path.exists()) {
            (_, false) => Disposition::Gone,
            (true, true) => Disposition::Kept,
            (false, true) => {
                // The last stage, so its answer is not carried on — the write
                // that settles the Job clears the transient a few lines below.
                stage(&store, record, doing.as_ref(), "worktree", now.wall_ms());
                match worktree::remove(run, &repo_root, &path) {
                    Ok(()) => Disposition::Removed,
                    Err(error) => {
                        // Reported, never raised. The Job is ended either way,
                        // and a `kill` that bailed out here would need a second
                        // `kill` to do the same thing again.
                        failure.get_or_insert(error);
                        Disposition::Kept
                    }
                }
            }
        };

        let branch = if ending.keep_branch || disposition != Disposition::Removed {
            Disposition::Kept
        } else {
            match worktree::delete_branch(run, &repo_root, &record.branch) {
                Ok(()) => Disposition::Removed,
                // The branch may already be gone, or the repository may be. A
                // Job cannot be un-killed, so this is reported and not raised.
                Err(_) => Disposition::Gone,
            }
        };

        let killed = Killed {
            job: record.name.clone(),
            uuid: record.uuid.clone(),
            released: cleaned.released,
            port_block: record.port_block,
            worktree: disposition,
            worktree_path: record.worktree.clone(),
            branch,
            branch_name: record.branch.clone(),
            error: failure,
        };

        // **The Job is marked ended whatever happened above.** A `kill` that
        // left a Job live because one container refused to stop would need a
        // second `kill` to do the same thing again.
        //
        // Its spend is settled from the transcript on the way out, because the
        // transcript is about to be the only thing left that knows.
        //
        // **Plus what its sub-Jobs spent, which the transcript does not hold.**
        // A parent's children run in sessions of their own, so re-deriving its
        // ledger from its own transcript alone silently subtracts every one of
        // them at the last moment — and this is the last moment: the figure
        // written here is the one `armada fleet ls --all` shows for ever after.
        // It cost the `feature` Job that finished with a `plan` sub-Job exactly
        // the sub-Job's whole spend. `job::observe` sums the same two halves
        // for a live Job ([`job::Kin::spend`]); this is that sum at the end.
        record.spend = drone::transcript(&place.stream(&record.uuid)).spend;
        record.spend.add(&record.kin.spend);
        record.state = ending.state;
        record.port_block = None;
        // **A Job whose worktree is gone must not go on claiming it.** The
        // record is what `armada fleet show` reads a held worktree from, and a
        // Job that reads as holding a directory nothing can open is the same
        // shape of lie as a `RUNNING` Job with a dead Drone — which this fleet
        // has already shown a reader once.
        if disposition == Disposition::Removed {
            record.worktree = String::new();
        }
        // **The transient is cleared by the write that settles the Job.** An
        // action word left behind would say a kill is still running against a
        // Job that is already `ABORTED` — the opposite of the silence `020` §5
        // is about, and just as misleading.
        record.doing = None;
        store.save(record)?;

        // **Its inbox entries end with it.** Both of the user's Jobs reached
        // `ABORTED` and five entries stayed open against Jobs that no longer
        // existed, still advertising `armada fleet answer` — the two
        // consequences `docs/reserved/005-inbox-label-not-identity.md`
        // records, and this line is where the first of them is closed.
        //
        // **After the save, and never instead of it.** A `kill` that failed
        // here must still have ended the Job; an entry left open is a stale
        // row, an unsaved record is a Job nothing can end.
        close_entries(place, record)?;
        Ok(killed)
    }
}

// --------------------------------------------------------------- pause/resume

/// Release what a Job holds because it has **finished**.
///
/// # The happy path was the leak
///
/// Every other way a Job ends reclaimed what it held. `spawn`'s rollback did,
/// `kill` did, `reap` did — and the loop, on the one path a Job takes when it
/// *succeeds*, did not. So a Job that failed was tidied up and a Job that worked
/// left its containers, its networks, its **named volumes**, its port block and
/// its worktree behind for ever. Nobody runs `clean` afterwards, which is how a
/// machine comes to hold 171 volumes and 12.0 GB.
///
/// # What a finished Job keeps, and why
///
/// | | `kill` / `reap` | finishing |
/// |---|---|---|
/// | containers, networks, **volumes**, images | released | released |
/// | port block | released | released |
/// | **branch** | deleted | **kept — it is the whole reason the Job was run** |
/// | **worktree** | removed | removed **only when there is nothing in it to lose** |
///
/// **The branch is never touched.** A Job that reached its last step produced
/// commits, and those commits are the deliverable; deleting the branch would
/// make the loop's success indistinguishable from its failure.
///
/// # The worktree goes, unless removing it would destroy work
///
/// This was the decision, and both answers were arguable.
///
/// *Keep it and let `reap` take it* has a real case: `reap` exists for exactly
/// this, it already offers a `DONE` Job and already ticks it by default, and a
/// reader who wants to see what the Job did has somewhere to look. It loses on
/// the evidence, and the evidence is this project's own: **nobody runs the
/// deferred verb.** That is the identical argument the user made about `clean`,
/// and answering it with "run `reap`" would be answering "why is my disk full"
/// with a command he was already not running.
///
/// *Remove it always* is what `kill` does, and it is wrong here for one reason:
/// [`worktree::remove`] forces, and forcing is right when a caller asked for it
/// by name and wrong when a background pass decided it. Uncommitted work
/// destroyed by a loop nobody was watching is work nobody agreed to lose.
///
/// **So it goes when git says the tree is clean, and stays when it is not** —
/// and the row says which, because a directory that survives with nothing
/// explaining it reads as a broken removal. A Job whose tree is dirty stays
/// offered to `armada fleet reap`, where taking it is a deliberate act in front
/// of a preview. [`worktree::holds_uncommitted_work`] answers `true` when it
/// cannot tell, for the same reason the reaper never removes on an errno that is
/// not `ENOENT`.
///
/// **A `FAILED` or halted Job never reaches here.** `Next::Halt` leaves a Job
/// `PAUSED` and asks a person; its worktree is the evidence for the question it
/// just raised, and this function is only ever called on `Next::Finish`.
pub(crate) fn release_on_finish<R: Run, C: Clock>(
    run: &R,
    now: &C,
    place: &Where,
    record: &mut Job,
) -> Result<Killed, ArmadaError> {
    let path = place.expand(&record.worktree);
    // **A tree that is not there is not dirty**, and `tear_down` reports it as
    // `Gone` rather than removing anything.
    let keep_worktree = path.is_dir() && worktree::holds_uncommitted_work(run, &path);
    tear_down(
        run,
        now,
        place,
        record,
        Ending {
            state: JobState::Done,
            keep_branch: true,
            keep_worktree,
            // The loop has just gated the step and written the verdict. A second
            // observation here could only disagree with the one already
            // recorded.
            observe: false,
            // Nothing to watch: the step has passed and the Job is `DONE`
            // whatever this releases.
            acting: None,
        },
    )
}

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
    let (observed, _, _) = look(run, place, &record, now.wall_ms());

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

    // **`PAUSING` while the Drone is being stopped** (`020` §5). A pause
    // SIGTERMs a group and waits before escalating, which is the same several
    // seconds of silence an abort had — and the same repair: the row says what
    // is happening before the slow part starts.
    let pausing = job::Doing::started(Acting::Pausing, now.wall_ms());
    stage(&store, &mut record, Some(&pausing), "drone", now.wall_ms());

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
    record.doing = None;
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
    let (observed, _, _) = look(run, place, &record, now.wall_ms());

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
        argv::continue_argv(
            &record.uuid,
            &place.posture()?,
            place.relay(&record.uuid).as_deref(),
            place.drone_mcp(&record.uuid).as_deref(),
        ),
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
                record.attempts_on_step(),
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
        let (observed, _, _) = look(run, place, &record, wall);
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
        let (observed, _, _) = look(run, place, &record, wall);
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

    end(run, now, place, Acting::Reaping, targets, false, false)
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
/// **The handle is an entry id or a Job**, in that order, and the entry id is
/// the one this verb is really about
/// (`docs/reserved/001-raised-items-need-identity.md`). The Job form is kept
/// because it is what every existing caller types and because a Job with one
/// open question needs no id to disambiguate it; the entry form is what makes a
/// row in a table something you can act on one at a time, which is the whole
/// complaint.
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
    let entries = entries(place)?;

    // **The handle is tried as an entry id before it is tried as a Job**, which
    // is `docs/reserved/001-raised-items-need-identity.md` arriving in the verb
    // 005 fixed the store half of. A Job that asked twice has two open rows and
    // naming the Job says which one you mean only by accident — [`open_for`]
    // picks the oldest. Naming the row says it exactly, and the row is what the
    // table you are reading it off draws.
    //
    // **Entry first, Job second, and never both.** The two spaces cannot be
    // merged — a Job's uuid and an entry's are different identities of
    // different things — so the order is the decision: an id that names an open
    // entry means that entry, and anything else falls through to the Job index
    // exactly as it did before. Nothing that worked stops working, because a
    // Job handle has never been an open entry's uuid.
    let picked = inbox::find_open(&entries, handle)?.cloned();
    let mut record = match &picked {
        // `job_uuid` is `Some` for every open entry — `is_open` requires it
        // (`inbox.rs`) — so this cannot fall through to the Job index by
        // accident. The `unwrap_or` is the total spelling of that invariant.
        Some(entry) => store.load(entry.job_uuid.as_deref().unwrap_or_default())?,
        None => store.find(handle)?,
    };

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

    let entry = match &picked {
        Some(entry) => entry,
        None => {
            let Some(entry) = inbox::open_for(&entries, &record.uuid) else {
                return Err(ArmadaError {
                    class: ErrClass::BadInvocation,
                    r#where: record.name.clone(),
                    message: format!("`{}` has nothing open to answer", record.name),
                    next_action: Some("`armada fleet inbox` lists what is waiting".to_string()),
                });
            };
            entry
        }
    };

    // # A ceiling is a checkpoint, not a death
    //
    // **The answer to a Job that ran out of rope is how much more it gets.**
    // This used to refuse outright — *"board it to take it over, or kill it"* —
    // on the argument that `on_exhausted: needs_human` means a person decides
    // what happens next. That argument is right and the refusal did not follow
    // from it: a person deciding *give it three more attempts* is the person
    // deciding. What the old path actually offered was two ways to abandon the
    // work, and it stranded every Job that reached a ceiling on a branch nobody
    // went back to.
    //
    // **The answer is a `--budget` pair, in `--budget`'s own grammar**, so there
    // is one spelling of a ceiling in Armada rather than two:
    //
    // ```text
    // armada fleet answer 47fb4860 max_attempts=6
    // armada fleet answer 47fb4860 max_cost=25.00
    // ```
    //
    // [`workflow::override_budget`] already parses it, already refuses an
    // unknown key, and already says what the three ceilings are — so an answer
    // that is prose gets the grammar back rather than a second, differently
    // worded complaint. And the raise is still a person's: nothing here raises
    // a ceiling on its own, and a Job that is not answered stays stopped.
    //
    // **The key is recorded in `budget_set`** for [`carved`]'s reason — a
    // ceiling a person typed is an instruction, and a sub-Job spawned after this
    // must inherit the raise rather than its workflow's default.
    let (observed, _, _) = look(run, place, &record, now.wall_ms());
    if let Some(ceiling) = observed.ceiling {
        let raised = match workflow::override_budget(record.budget, &[said.to_string()]) {
            Ok(raised) => raised,
            Err(error) => {
                // **Persisted and raised on the way out**, so the ceiling is a
                // durable fact rather than something this invocation noticed and
                // forgot. The answer was prose rather than a raise, so nothing
                // about the Job has changed except that it is now written down
                // as stopped.
                settle(&mut record, &observed, place, now)?;
                store.save(&record)?;
                return Err(ArmadaError {
                    message: format!(
                        "`{}` reached its {} ceiling, and `{}` does not say how much more it gets: {}",
                        record.name,
                        ceiling.word(),
                        said,
                        error.message
                    ),
                    ..error
                });
            }
        };
        record.budget = raised;
        if let Some((key, _)) = said.split_once('=') {
            let key = key.trim().to_string();
            if !record.budget_set.contains(&key) {
                record.budget_set.push(key);
            }
        }

        // **Re-observed against the new ceiling before anything is closed.** A
        // raise that does not actually clear the ceiling it was given for — six
        // attempts to a Job that has made six — would otherwise close the entry
        // and leave the Job stopped with nothing open to answer, which is the
        // silent-empty-answer shape this codebase has produced three times.
        let (observed, _, _) = look(run, place, &record, now.wall_ms());
        if let Some(still) = observed.ceiling {
            return Err(ArmadaError {
                class: ErrClass::BadInvocation,
                r#where: record.name.clone(),
                message: format!(
                    "`{}` is still at its {} ceiling after `{}`",
                    record.name,
                    still.word(),
                    said
                ),
                next_action: Some(format!(
                    "`armada fleet show {}` says what it has spent",
                    record.name
                )),
            });
        }
        // **Cleared, so the verdict goes with it.** `NeedsHuman` is what stopped
        // the Job; leaving it set would have `attention` decline to gate a Job
        // whose question has just been answered.
        record.verdict = None;
        store.save(&record)?;

        // # A raise is not an answer to whatever else the Job was asking
        //
        // **Measured within the hour this branch was written, on the author's
        // own fleet.** A planning Job sat on `human_approves` asking *"does this
        // look right to you?"*, and while it waited its wall clock ran out. The
        // raise that gave it more time fell through to `inbox::answer` below and
        // closed that question with the string `max_wall_clock=6h` — which the
        // gate then read as the reviewer's verdict, decided it was not an
        // approval, and recorded a failed attempt. Three of those and the Job was
        // out of attempts as well.
        //
        // So a raise settles the ceiling and nothing else. The gate's question
        // stays open and is answered on its own terms, by a second call that
        // carries the actual decision.
        if let Some(pending) = &record.pending {
            if matches!(&pending.on, job::Waiting::Answer(waiting_on) if waiting_on == &entry.uuid)
            {
                return Ok(Output::Answer(Box::new(Envelope::ok(
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
                            record.attempts_on_step(),
                        ),
                        pgid: record.drone.as_ref().map(|drone| drone.pgid),
                    },
                ))));
            }
        }
    }

    // **Whose question was this?** A gate's and a Drone's arrive in one inbox
    // and are answered by one verb, and they want opposite things done next.
    //
    // A Drone that asked is stuck and needs the answer to carry on, so the
    // answer is a continuation — resume the session, below.
    //
    // A gate that asked is the *workflow* asking whether the step is accepted.
    // The answer belongs to the gate, and resuming a Drone here is what made
    // `human_approves` unsettleable: the resumed Drone did more work, asked its
    // own question, and a new entry replaced the pending one, so the next tick
    // halted on that instead. The approval was recorded every time and read
    // never, `Next::Finish` was unreachable, and no Job has ever been `DONE`
    // (`docs/reserved/026`).
    let settles_a_gate = record
        .pending
        .as_ref()
        .is_some_and(|pending| match &pending.on {
            job::Waiting::Answer(waiting_on) => waiting_on == &entry.uuid,
            // **A sub-Job's gate is settled by the child's verdict and by
            // nothing a person types.** The entry being answered here belongs
            // to the *child* — a reviewer that raised a blocking finding — and
            // answering it releases that Job, whose verdict this one then
            // reads. Treating it as this gate's answer would settle the parent
            // on a sentence rather than on the Job it is waiting for.
            job::Waiting::Check(_) | job::Waiting::SubJob(_) => false,
        });

    inbox::answer(&place.inbox(), &entry.uuid, said)?;
    // **The wait ends here, at the entry that was being waited on.** Paired with
    // [`raise`], which starts it; between them they are the only two places a
    // Job's wait clock moves, so it cannot start without stopping.
    record.stopped_waiting(now.wall_ms());

    if settles_a_gate {
        // **Ticked rather than resumed**, and ticked here rather than left for
        // the relay: no Drone is going to run, so no `Stop` hook is going to
        // fire, and a gate that is settled but never gated is a Job that stops
        // exactly as dead as before.
        //
        // `tick` takes the pass lock itself and this function holds none, so
        // the call nests nothing. Its own output is discarded: the reader ran
        // `fleet answer` and the envelope they get back says so.
        // **Out of `PAUSED` before the tick, and `RUNNING` is the right word
        // for it.** `advance::attention` reads `PAUSED` as *"it is waiting on
        // you"* and declines to gate — correctly, because a paused Job has an
        // open question. Answering closed that question, so the Job is back in
        // what `JobState::Running` actually names here: *the ordinary resting
        // state between turns, a turn finished cleanly and no Drone is
        // running*. Nothing is started; the word is about the Job, not a
        // process, and `alive` is observed separately.
        //
        // The old path reached the same state by resuming a Drone, which is why
        // this was never missing before — and resuming is the thing that made
        // the gate unsettleable.
        record.state = JobState::Running;
        store.save(&record)?;
        // **The Job's own name, never the caller's `handle`.** `handle` is
        // whatever was typed, and this verb deliberately accepts an *entry* id
        // as well as a Job's — so passing it on gave `tick` an inbox id and the
        // refusal `no Job called a058890c`, raised *after* the answer had
        // already been written. The answer landed, the gate was never told, and
        // the error named the wrong noun.
        let name = record.name.clone();
        tick(run, now, place, Some(&name), false)?;
        // Re-read, because the tick has just rewritten the record it acted on.
        // **By uuid, which is what the store is keyed by** — `handle` is what
        // the caller typed and may be a name or an entry id, and `load` builds
        // a path out of what it is given.
        let record = store.load(&record.uuid)?;
        return Ok(Output::Answer(Box::new(Envelope::ok(
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
                    record.attempts_on_step(),
                ),
                pgid: record.drone.as_ref().map(|drone| drone.pgid),
            },
        ))));
    }

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
        argv::resume_argv(
            &record.uuid,
            said,
            &place.posture()?,
            place.relay(&record.uuid).as_deref(),
            place.drone_mcp(&record.uuid).as_deref(),
        ),
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
                record.attempts_on_step(),
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

/// `fleet.report` — a Drone appends progress to **its own** Job record, and
/// records the step boundary it just crossed.
///
/// **Its own, and nothing else's.** The Job is named by `handle`, which the MCP
/// layer fills from `ARMADA_JOB` — the variable [`armada_fleet::drone::job_env`]
/// sets on every child of a Job and that a Drone therefore cannot choose. A
/// worker able to write another worker's record is a worker that can rewrite the
/// evidence a verdict rests on.
///
/// **A boundary at a time, and nothing polls.** *"It doesn't have to be in real
/// time"* — so a report when a step starts and one when the Drone stops trying
/// is enough, and nothing here messages a running Drone to ask how it is going.
/// The probe never interrupts a Drone (PLAN.md §15.2) and this verb adds no
/// second channel that would.
///
/// **What a Drone may say, and what it may not.** `entered` and `attempted` are
/// facts about the Drone. `completed` and `failed` are the step's predicate
/// holding or not — [`fleet.verdict`](verdict) writes those, and this verb
/// refuses them rather than recording an assertion dressed as a gate. The
/// refusal is the feature, not a validation detail:
/// [`StepEvent::is_a_drones_to_report`] is where the rule lives.
pub fn report<C: Clock>(
    now: &C,
    place: &Where,
    handle: &str,
    body: &str,
    step: Option<&str>,
    event: Option<&str>,
    set: &std::collections::BTreeMap<String, String>,
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

    let crossing = event.map(reportable).transpose()?;
    let step = step.unwrap_or(&record.step).to_string();

    // # A step this workflow does not declare is refused, not stored
    //
    // **Measured four times on 2026-08-17, and the fourth corrupted a Job.**
    // A Drone reported `verify`, then `approve` twice, then `approve` again —
    // none of which is a step in the workflow it was running. The name was
    // written to the record every time, and `bridge-command-centre` ended up
    // with `step: approve` while running `feature`, whose steps are `plan`,
    // `approval`, `implement`, `review` and `land`. The gate then had nothing to
    // look up, so `tick` reported *"its Drone is still working"* for ever and
    // the Job could not advance, fail, or reach a ceiling. $14 of work stranded
    // in a state machine pointing at a state that does not exist.
    //
    // **The real fix deletes the parameter** — `docs/reserved/032` says a Drone
    // may not name a step at all, because the Job already holds it — and that is
    // being built. This is the guard that belongs here regardless: a state
    // machine must refuse a state it does not have, whoever proposed it. Even
    // once the argument is gone, the same refusal protects a resumed Job whose
    // guild workflow has since been edited.
    //
    // The refusal names the steps that do exist, because the caller is an agent
    // that guessed and a list is what stops it guessing again.
    if !flow_has_step(place, &record.workflow, &step) {
        let known = flow_steps(place, &record.workflow);
        return Err(ArmadaError {
            class: ErrClass::BadInvocation,
            r#where: step.clone(),
            message: format!(
                "the `{}` workflow has no step called `{step}`",
                record.workflow
            ),
            next_action: Some(match known.is_empty() {
                true => format!("`{}` is on `{}`", record.name, record.step),
                false => format!("its steps are {}", known.join(", ")),
            }),
        });
    }

    // **Entering a step is what moves the Job onto it.** The record's `step` is
    // what every other surface reads, and a boundary that left it pointing at
    // the previous step would put the note, the attempt count and the `STEP`
    // column each on a different step.
    if crossing.is_some_and(job::StepEvent::opens) {
        record.step.clone_from(&step);
    }

    // **What the gate should look for, from the only party that can know it.**
    //
    // A step's `verify:` may be written `test: ${task.test}`, which
    // [`gate::resolve`] fills from these facts. Until now the *only* source was
    // `armada fleet spawn --set`, and the callers that put a Job on the `bug`
    // workflow cannot supply one: a recorded failure is a stack trace and
    // `armada report` is somebody's observation, so neither knows the name of a
    // test nobody has written yet. The gate then had nothing to search for and
    // the step could not be decided at all — measured 2026-08-17 on job
    // `armada-failed`, 39 turns and a real failing test in the tree, stopped on
    // *"the `reproduce` step names its test as `${task.test}`, which nothing has
    // substituted"*.
    //
    // **The caller still wins, and that ordering is the point.** A `--set test=`
    // is a contract stated before the work began; this is a fact discovered
    // during it. Letting a Drone overwrite the first would let it move its own
    // goalposts — and what it cannot do either way is decide whether the gate
    // holds: Armada still searches the tree for the name and still requires the
    // check run to be red (`PLAN.md` §5).
    for (key, value) in set {
        record.facts.entry(key.clone()).or_insert(value.clone());
    }

    let crossed = crossing.map(|event| {
        let entry = job::record(
            &mut record.transitions,
            now.wall_rfc3339(),
            now.wall_ms(),
            &step,
            event,
            None,
        );
        // **An attempt is counted when it begins.** `verdict` counts one for a
        // Drone that never reported entering; it must not count this one twice,
        // which is what `attempt_open` decides there.
        if entry.event.opens() {
            record
                .attempts
                .insert(step.clone(), job::step_attempts(&record.transitions, &step));
        }
        entry
    });

    record.progress.push(armada_core::fleet::job::Note {
        at: now.wall_rfc3339(),
        at_ms: now.wall_ms(),
        step: step.clone(),
        body: body.to_string(),
    });
    store.save(&record)?;

    Ok(Output::Report(Box::new(Envelope::ok(
        "fleet report",
        None,
        Status::Ok,
        ReportData {
            job: record.name.clone(),
            step,
            notes: record.progress.len(),
            event: crossed.as_ref().map(|entry| entry.event.word().to_string()),
            attempt: crossed.as_ref().map(|entry| entry.attempt),
        },
    ))))
}

/// The boundary word a Drone wrote, or the refusal that says who owns it.
///
/// **Two different refusals, because they are two different mistakes.** A word
/// outside the five is a typo; `completed` is a worker trying to grade its own
/// work, and telling it *"one of entered, attempted"* would leave it hunting for
/// a synonym. It is sent to `fleet.verdict` instead — the verb that already
/// refuses a `PASS` carrying no evidence.
fn reportable(word: &str) -> Result<job::StepEvent, ArmadaError> {
    let refuse = |message: String, next: String| ArmadaError {
        class: ErrClass::BadInvocation,
        r#where: "event".to_string(),
        message,
        next_action: Some(next),
    };
    let Some(event) = job::StepEvent::from_word(word) else {
        return Err(refuse(
            format!("`{word}` is not a step boundary"),
            "one of `entered`, `attempted`".to_string(),
        ));
    };
    if !event.is_a_drones_to_report() {
        return Err(refuse(
            format!("`{}` is not a Drone's to report", event.word()),
            match event {
                job::StepEvent::Restarted => {
                    "report `entered`; Armada records the restart from the attempts it already has"
                        .to_string()
                }
                _ => "a step is done when its `verify:` predicate holds — record it with \
                      `fleet.verdict`, carrying the evidence"
                    .to_string(),
            },
        ));
    }
    Ok(event)
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
    let mut record = place.store().find(handle)?;
    let entry = raise(place, now, &mut record, inbox::Kind::NeedsHuman, question)?;
    // **Persisted, because the wait started on it.** `raise` stamps the record;
    // a caller that dropped it would leave the clock running on a Job that is
    // demonstrably stopped in front of a question.
    place.store().save(&record)?;

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

// --------------------------------------------------------------------- propose

/// `fleet.propose` — **what a Drone noticed and is not blocked on**
/// ([`docs/reserved/008`](../../../../docs/reserved/008-armada-injects-its-own-skills.md)).
///
/// # Why this is not `ask_human`, and not an `armada.yml` edit
///
/// `PLAN.md` §5's sandwich is *Armada reports facts, an agent authors, Armada
/// verifies*. The middle layer was expected to know how to hold the tools, and a
/// Drone that had learned something the manifest does not say had two bad
/// options and no good one:
///
/// | It could | And that is |
/// |---|---|
/// | edit `armada.yml` itself | a claim nobody checked, landing inside a diff about something else — the one thing *Armada verifies* rules out |
/// | call [`ask_human`] | a five-minute wait, against its own ceiling, for a decision it is not blocked on |
/// | say nothing | what it did, because the third option did not exist |
///
/// So this raises and returns. **No wait, no poll, no ceiling spent** — the
/// difference from [`ask_human`] is the whole reason it is a second verb rather
/// than a flag on the first, because a proposal that waited would be a Drone
/// stopping work to tell somebody something they can read tomorrow.
///
/// # It writes an inbox entry, and there is no fifth origin
///
/// `docs/reserved/001` settled that every item Helm surfaces is an entry with an
/// id, across four origins and one id space. A proposal is
/// [`armada_core::failure::Origin::Raised`] — *a Drone asked for you* — which it
/// already is, because it is the inbox. Nothing new is stored, nothing new is
/// resolved, and `armada fleet answer <id>`, `armada failures show <id>` and the
/// Bridge all reach it on the day it is written.
///
/// # What Armada verifies here, and what it does not
///
/// It verifies that the subject is one of two words ([`Subject`]) and that the
/// proposal is not empty. **It does not check whether the claim is true**, and
/// could not: the Drone is the only thing that ran the command. Verification of
/// the content is the person reading the row, or a Job they start from it —
/// through a path that already exists. A proposal is not a change.
pub fn propose<C: Clock>(
    now: &C,
    place: &Where,
    handle: &str,
    subject: Subject,
    proposal: &str,
) -> Result<Output, ArmadaError> {
    let proposal = proposal.trim();
    // **An empty proposal is refused rather than filed.** A row whose body says
    // nothing is a row a person opens, learns nothing from, and cannot close
    // with any confidence — and the inbox is append-only, so it is there for
    // good. `bad_invocation` because the caller can fix it by saying what it
    // noticed.
    if proposal.is_empty() {
        return Err(ArmadaError {
            class: ErrClass::BadInvocation,
            r#where: "proposal".to_string(),
            message: "a proposal with no body is a row nobody can act on".to_string(),
            next_action: Some(format!(
                "say what about the {subject} you would change, and why"
            )),
        });
    }
    let mut record = place.store().find(handle)?;
    // **`NEEDS_HUMAN` rather than a fourth [`inbox::Kind`].** The kind answers
    // *why does the fleet want you*, and the answer here is the one it already
    // has: a judgement call is yours. A `PROPOSAL` kind would be a second word
    // for the same fact, and every table that draws the column would have to
    // learn it to say nothing new.
    let entry = raise(
        place,
        now,
        &mut record,
        inbox::Kind::NeedsHuman,
        // **The subject is in the body, not beside it.** An inbox entry is read
        // out of context and possibly hours later, and a bare sentence about a
        // port would leave a reader guessing which file it was about. It is not
        // a field because that would change the on-disk line shape for every
        // entry ever written, to carry one word.
        &format!("proposes a change to the {subject}: {proposal}"),
    )?;
    // Persisted for [`ask_human`]'s reason: the wait began on this record.
    place.store().save(&record)?;

    Ok(Output::Propose(Box::new(Envelope::ok(
        "fleet propose",
        None,
        Status::Ok,
        ProposeData {
            job: record.name.clone(),
            entry,
            subject: subject.word().to_string(),
            proposal: proposal.to_string(),
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
///
/// **`why` is the entry's own words, and `None` is the Drone's case.** A Drone
/// calling `fleet.verdict` has already said what it was doing through
/// `fleet.report`, so the generic sentence is enough; the loop
/// ([`tick`]) knows precisely which ceiling was reached or which predicate could
/// not be decided, and an inbox that said *"reached a judgement call"* about a
/// step nobody could gate would send the reader to read the transcript to find
/// out what Armada already knew.
/// Record a Drone's status or tick's verdict. When called by a Drone via MCP tool,
/// takes only the status. When called internally by tick, uses the old signature
/// (via a separate internal path if needed).
pub fn verdict<C: Clock>(
    now: &C,
    place: &Where,
    handle: &str,
    status: armada_core::fleet::DroneStatus,
) -> Result<Output, ArmadaError> {
    use armada_core::fleet::DroneStatus;

    let store = place.store();
    let mut record = store.find(handle)?;
    if record.state.is_over() {
        return Err(ArmadaError {
            class: ErrClass::BadInvocation,
            r#where: record.name.clone(),
            message: format!("`{}` is {}", record.name, record.state.word()),
            next_action: Some("a Job that has finished reaches no further verdict".to_string()),
        });
    }

    let step = record.step.clone();

    // **Counted here only when nobody counted it at entry.** A Drone that
    // reported entering the step already opened the attempt; bumping again on
    // the way out would halve the rope the workflow declared.
    if !job::attempt_open(&record.transitions, &step) {
        *record.attempts.entry(step.clone()).or_insert(0) += 1;
    }
    let attempts = record.attempts.get(&step).copied().unwrap_or(1);

    let (recorded, settled) = match status {
        DroneStatus::Done => {
            // **The Drone reports done; tick will gate.** Record only the Attempted
            // event. Don't set a verdict — tick will gate and call record_gate_verdict
            // with the actual gate outcome.
            job::record(
                &mut record.transitions,
                now.wall_rfc3339(),
                now.wall_ms(),
                &step,
                job::StepEvent::Attempted,
                None,
            );
            // **Nothing has been decided, and the answer says so.** This
            // returned `Verdict::Pass` as what its own comment called a
            // "response marker" — correctly kept out of the record, and
            // serialised to the Drone anyway as `"verdict":"PASS"` with an
            // empty evidence list, before any gate had run. `032` says only
            // the Job may say a step passed; saying it in Armada's own tool
            // response is that rule broken with Armada's authority behind it.
            // Found by a reviewer Job reading the change that introduced it.
            (Recorded::Attempted, None)
        }
        DroneStatus::Stuck => {
            // **The Drone hit a blocker.** Mark that it attempted and then stopped.
            // Record Attempted to close the attempt, then set Blocked verdict.
            job::record(
                &mut record.transitions,
                now.wall_rfc3339(),
                now.wall_ms(),
                &step,
                job::StepEvent::Attempted,
                None,
            );
            (Recorded::Blocked, Some(Verdict::Blocked))
        }
    };

    record.step = step.clone();

    // **A verdict is written only when the Drone really produced one.** A
    // `done` report produces none: the attempt is closed and `tick` gates it.
    if let Some(verdict) = settled {
        record.verdict = Some(verdict);
        record.state = verdict.settles_to();
    }

    let entry = match settled {
        Some(Verdict::Blocked) => Some(raise(
            place,
            now,
            &mut record,
            inbox::Kind::Blocked,
            &format!("`{step}` is blocked and cannot proceed without an external change"),
        )?),
        _ => None,
    };
    store.save(&record)?;

    Ok(Output::Verdict(Box::new(Envelope::ok(
        "fleet verdict",
        None,
        Status::Ok,
        VerdictData {
            job: record.name.clone(),
            step: step.clone(),
            recorded,
            verdict: None,
            evidence: vec![],
            attempts,
            state: record.state,
            entry,
        },
    ))))
}

/// Internal function for tick to record a gate outcome as a verdict.
/// This is called after `advance::after` determines what to do next.
/// It writes the step event and inbox entry for a reached verdict.
pub fn record_gate_verdict<C: Clock>(
    now: &C,
    place: &Where,
    handle: &str,
    step: &str,
    reached: Verdict,
    evidence: Vec<Evidence>,
    why: Option<&str>,
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

    // **Counted here only when nobody counted it at entry.** A Drone that
    // reported entering the step already opened the attempt; bumping again on
    // the way out would halve the rope the workflow declared.
    if !job::attempt_open(&record.transitions, step) {
        *record.attempts.entry(step.to_string()).or_insert(0) += 1;
    }
    let attempts = record.attempts.get(step).copied().unwrap_or(1);

    // **The gate's own record of what settled the step.** `PASS` and `FAILED`
    // are the predicate holding or not; `BLOCKED` and `NEEDS_HUMAN` are neither
    // — the step is still open and the inbox is what says why, so no boundary is
    // written for them and the attempt stays the one that was already under way.
    if let Some(event) = match reached {
        Verdict::Pass => Some(job::StepEvent::Completed),
        Verdict::Failed => Some(job::StepEvent::Failed),
        Verdict::Blocked | Verdict::NeedsHuman => None,
    } {
        let gated = gate_of(place, &record.workflow, step);
        job::record(
            &mut record.transitions,
            now.wall_rfc3339(),
            now.wall_ms(),
            step,
            event,
            Some(job::Gate {
                must: gated.as_ref().map(|verify| verify.must),
                test: gated.as_ref().and_then(|verify| verify.test.clone()),
                artifact: gated.as_ref().and_then(|verify| verify.artifact.clone()),
                evidence: evidence.clone(),
            }),
        );
    }

    record.step = step.to_string();
    record.verdict = Some(reached);
    record.state = reached.settles_to();

    // **The entry id is kept, not discarded.** A gate waiting on a person has to
    // read *that* answer and not whichever one is newest, so the id the raise
    // minted travels back out in the envelope.
    let entry = match reached {
        Verdict::Blocked => Some(raise(
            place,
            now,
            &mut record,
            inbox::Kind::Blocked,
            &match why {
                Some(why) => why.to_string(),
                None => {
                    format!("`{step}` is blocked and cannot proceed without an external change")
                }
            },
        )?),
        Verdict::NeedsHuman => Some(raise(
            place,
            now,
            &mut record,
            inbox::Kind::NeedsHuman,
            &match why {
                Some(why) => why.to_string(),
                None => format!("`{step}` reached a judgement call"),
            },
        )?),
        Verdict::Pass | Verdict::Failed => None,
    };
    store.save(&record)?;

    Ok(Output::Verdict(Box::new(Envelope::ok(
        "fleet verdict",
        None,
        Status::Ok,
        VerdictData {
            job: record.name.clone(),
            step: step.to_string(),
            recorded: match reached {
                Verdict::Blocked => Recorded::Blocked,
                Verdict::NeedsHuman => Recorded::NeedsHuman,
                Verdict::Pass | Verdict::Failed => Recorded::Attempted,
            },
            verdict: Some(reached),
            evidence,
            attempts,
            state: record.state,
            entry,
        },
    ))))
}

// ------------------------------------------------------------------------ tick

/// How often `--watch` looks again.
///
/// **Two seconds, the Bridge's cadence and `fleet.ask_human`'s**, and for the
/// same reason: a pass over an idle fleet is a directory listing, a transcript
/// tail and a `ps`, none of which any Drone notices. A third number here would
/// be a third thing to tune.
pub const TICK_POLL_MS: u64 = 2_000;

/// `armada fleet tick` — **the workflow loop** (PHASES.md §8.6).
///
/// # The gap this closes
///
/// A Drone runs one exchange under `--print` and exits. That is correct — it is
/// what lets `spawn` return and several Jobs run at once — but **nothing
/// observed the exchange ending**, so a Job went `RUNNING` and stayed there for
/// ever beside a process group that was gone. This is the thing that observes
/// it, gates the step, and then advances, retries or stops.
///
/// # The shape, and why the decisions are not here
///
/// Everything this function decides was decided in
/// [`armada_core::fleet::advance`] and [`armada_core::fleet::gate`]. What lives
/// here is the order the adapter calls go in:
///
/// 1. [`look`] — the transcript and the process table, reconciled once.
/// 2. `advance::attention` — is there anything to do at all.
/// 3. [`gather`] — start or poll a check, search the tree, stat a path, read the
///    inbox. **The only I/O in the gate**, and it is driven by `gate::needs`
///    rather than by a second copy of the mapping.
/// 4. `gate::decide` — does the predicate hold.
/// 5. `advance::after` — advance, retry, ask or stop.
/// 6. [`verdict`] — the record, written by the verb that already refuses a
///    `PASS` with no evidence. **`fleet.report` still writes only `entered` and
///    `attempted`**; the two words a gate owns are still written by one thing,
///    and that thing is now something that exists.
///
/// # Why this is a verb rather than a daemon
///
/// Armada owns no long-lived process and this milestone does not add one. A pass
/// is idempotent and cheap, so a timer, a `Stop` hook, the Bridge or a person
/// typing it are all valid drivers — and `--watch` is one of them rather than
/// the only one. A daemon would need its own lease, its own crash recovery and
/// its own answer to *"what happened while it was down"*, all to replace a
/// command that can simply be run again.
pub fn tick<R: Run, C: Clock>(
    run: &R,
    now: &C,
    place: &Where,
    handle: Option<&str>,
    watch: bool,
) -> Result<Output, ArmadaError> {
    // **One pass at a time on this machine** (`armada_fleet::pass`). Every
    // Drone's `Stop` hook sweeps the whole fleet, so five exchanges ending
    // together start five passes — and two passes gating one step would both
    // `claude --resume` one session. Declining is the right answer rather than
    // a slower yes: the pass that holds the lock is walking the same records.
    let Some(_held) = armada_fleet::pass::take(&place.armada_home, &place.boot_id, now.wall_ms())?
    else {
        return Ok(Output::Tick(Box::new(Envelope::ok(
            "fleet tick",
            None,
            Status::Ok,
            TickData {
                results: Vec::new(),
                moved: 0,
            },
        ))));
    };
    loop {
        let rows = pass(run, now, place, handle)?;
        let moved = rows.iter().filter(|row| did_something(&row.did)).count();
        // **`--watch` ends when there is nothing left that could move.** A Job
        // that is finished, halted or waiting on a person is `idle`, and a Job
        // over its ceiling becomes one — so the Job's own budget is what stops
        // this loop, and a second ceiling here would be a second thing that has
        // to agree with the first.
        // **Live means *could still move on its own*.** A Job whose Drone is
        // working will produce something; one waiting on a check will decide;
        // one that just advanced or retried has a Drone starting. The four that
        // are not live are the four that need a person or need nothing.
        let live = rows.iter().any(|row| {
            !matches!(
                row.did.as_str(),
                TICK_IDLE | TICK_HALTED | TICK_FINISHED | TICK_ASKED
            )
        });
        if !watch || !live {
            return Ok(Output::Tick(Box::new(Envelope::ok(
                "fleet tick",
                None,
                Status::Ok,
                TickData {
                    results: rows,
                    moved,
                },
            ))));
        }
        now.sleep_until(now.mono().saturating_add(TICK_POLL_MS));
    }
}

/// The words a row's `did` may take.
///
/// **Named constants rather than literals at seven call sites**, because two of
/// them decide whether `--watch` goes round again and a typo in one of those
/// would be a loop that never ends or one that ends immediately.
pub const TICK_IDLE: &str = "idle";
/// Something is still running; nothing was settled.
pub const TICK_WAITING: &str = "waiting";
/// Its Drone is mid-exchange. **Not `idle`**, because `--watch` has to tell
/// *nothing is happening* from *something is happening elsewhere*.
pub const TICK_WORKING: &str = "working";
/// The step passed and the Job moved to the next one.
pub const TICK_ADVANCED: &str = "advanced";
/// The step did not pass and it was started again.
pub const TICK_RETRIED: &str = "retried";
/// The last step passed and the Job is `DONE`.
pub const TICK_FINISHED: &str = "finished";
/// It stopped and put a question in the inbox.
pub const TICK_ASKED: &str = "asked";
/// It stopped: a ceiling, or a gate nothing can decide.
pub const TICK_HALTED: &str = "halted";

/// Whether a row is one where the loop actually did something.
fn did_something(did: &str) -> bool {
    !matches!(did, TICK_IDLE | TICK_WAITING | TICK_WORKING)
}

/// One pass over the Jobs in scope.
///
/// **A named Job means it and everything it started.** A parent waiting on a
/// sub-Job moves only when the child does, and the child moves only when
/// something ticks it — so `armada fleet tick <job> --watch` scoped to the
/// parent alone would poll a Job whose answer was sitting one record away,
/// waiting for a pass nobody was going to make. The parent comes first, because
/// it is what was asked about.
fn pass<R: Run, C: Clock>(
    run: &R,
    now: &C,
    place: &Where,
    handle: Option<&str>,
) -> Result<Vec<TickRow>, ArmadaError> {
    let store = place.store();
    let records = match handle {
        Some(handle) => {
            let named = store.find(handle)?;
            let mut family = vec![named.clone()];
            family.extend(descendants(&store.all()?, &named.uuid));
            family
        }
        None => {
            let mut all = store.all()?;
            all.sort_by(|a, b| a.name.cmp(&b.name));
            all
        }
    };
    records
        .into_iter()
        .map(|record| one(run, now, place, record))
        .collect()
}

/// Every Job started by this one, or by one of those, oldest first.
///
/// **Breadth first over the index in hand, rather than a query per record.**
/// The whole index is already loaded by every caller, a chain is at most
/// [`GENERATIONS`] deep, and a Job whose parent record has been deleted is
/// simply not reached — which is the right answer: it is nobody's child any
/// more, and `armada fleet ls` still lists it.
fn descendants(all: &[Job], root: &str) -> Vec<Job> {
    let mut found: Vec<Job> = Vec::new();
    let mut frontier = vec![root.to_string()];
    while let Some(uuid) = frontier.pop() {
        for child in all
            .iter()
            .filter(|job| job.kin.parent.as_ref().is_some_and(|up| up.uuid == uuid))
        {
            // A record that somehow names itself, or a pair that name each
            // other, would loop for ever otherwise — and this walks data on
            // disk that a person can edit.
            if found.iter().any(|seen| seen.uuid == child.uuid) || child.uuid == root {
                continue;
            }
            found.push(child.clone());
            frontier.push(child.uuid.clone());
        }
    }
    found
}

/// What the loop did about one Job.
fn one<R: Run, C: Clock>(
    run: &R,
    now: &C,
    place: &Where,
    record: Job,
) -> Result<TickRow, ArmadaError> {
    let wall = now.wall_ms();
    let (observed, reading, alive) = look(run, place, &record, wall);

    match advance::attention(&record, &observed, alive) {
        advance::Attention::Idle { why } => {
            // **What was observed is persisted on the way past.** `ls` refuses
            // to, because a read verb that wrote would make `armada fleet ls |
            // head` a mutation — but this verb's whole job is to move Jobs on,
            // and a Job whose Drone died is one whose record should say so
            // rather than waiting for somebody to run `kill`.
            let mut record = record;
            settle_if_changed(&mut record, &observed, place, now)?;
            Ok(tick_row(
                &record,
                TICK_IDLE,
                why.to_string(),
                None,
                Vec::new(),
                None,
            ))
        }
        advance::Attention::Ceiling(ceiling) => {
            let mut record = record;
            // `settle` records the ceiling and raises it: exhaustion is a
            // first-class outcome and `on_exhausted: needs_human` means stop and
            // ask, never abort.
            settle(&mut record, &observed, place, now)?;
            place.store().save(&record)?;
            let verdict = record.verdict;
            Ok(tick_row(
                &record,
                TICK_HALTED,
                format!("it reached its {} ceiling", ceiling.word()),
                None,
                Vec::new(),
                verdict,
            ))
        }
        advance::Attention::Working => {
            let mut record = record;
            settle_if_changed(&mut record, &observed, place, now)?;
            Ok(tick_row(
                &record,
                TICK_WORKING,
                "its Drone is still working".to_string(),
                None,
                Vec::new(),
                None,
            ))
        }
        advance::Attention::Gate => gate_step(run, now, place, record, &observed, &reading),
    }
}

/// Gate the step a Job is resting on, and act on the answer.
fn gate_step<R: Run, C: Clock>(
    run: &R,
    now: &C,
    place: &Where,
    mut record: Job,
    observed: &Observed,
    reading: &Reading,
) -> Result<TickRow, ArmadaError> {
    // **The spend the transcript sums, written down before anything else.** A
    // pass that advanced a step without persisting what the last exchange cost
    // would let a Job cross a ceiling and be gated anyway on the next pass.
    record.spend = observed.spend;
    // **The watermark, moved here and nowhere else** (`020` §6). This is the
    // one place an exchange is gated, so this is the one place that may record
    // that it was — and it is recorded *before* the gate answers, because a
    // pass that gated and then crashed has still consumed the exchange. The
    // alternative, moving it only on success, re-gates the same turn forever.
    record.ticked_turns = reading.turns.len();

    let flow = match read_workflow(place, &record.workflow) {
        Ok(flow) => flow,
        // **A guild that cannot be read stops the Job rather than failing the
        // pass.** One Job whose workflow is missing must not stop the loop
        // moving every other Job on.
        Err(error) => {
            let why = format!(
                "its `{}` workflow could not be read: {}",
                record.workflow, error.message
            );
            return halt(place, now, record, None, why);
        }
    };
    let step_id = record.step.clone();
    let Some(step) = flow
        .steps
        .iter()
        .find(|candidate| candidate.id == step_id)
        .cloned()
    else {
        let why = format!(
            "the `{}` workflow has no step called `{step_id}`",
            flow.name
        );
        return halt(place, now, record, None, why);
    };

    let want = gate::needs(&gate::resolve(&step, &record.facts));
    // Taken out and put back so the gatherer may record a run it started while
    // still reading the rest of the record.
    let mut pending = record.pending.take();
    let gathered = gather(
        run,
        now,
        place,
        &mut record,
        &want,
        &step_id,
        reading,
        &mut pending,
    );
    record.pending = pending;
    let predicate = Some(step.verify.must.word().to_string());
    let (facts, land_merge) = match gathered {
        Ok(gathered) => gathered,
        // **One Job's gate that cannot be gathered must not end the pass**, for
        // the reason [`stalled`] gives about a worktree that is gone: a fleet
        // of twenty would exit non-zero and move none of the other nineteen.
        // The two failures that reach here are a machine one — `armada manifest
        // check --detach` that would not start — and a guild one: a workflow
        // whose sub-Job would run a workflow already above it
        // ([`refuse_a_cycle`]). Both are somebody's to fix and neither is the
        // rest of the fleet's problem.
        Err(error) => {
            place.store().save(&record)?;
            return stalled(place, now, record, predicate, error);
        }
    };
    let waiting_on_a_person = facts
        .subjob
        .as_ref()
        .is_some_and(|child| child.state.needs_a_person());
    let outcome = gate::decide(&want, &facts, land_merge);
    // Whatever was started or read is recorded before anything acts on it: a
    // pass that started a check and then failed to save the run id would start
    // a second one on the next pass, for ever.
    place.store().save(&record)?;

    let attempts = job::step_failures(&record.transitions, &step_id).saturating_add(1);
    let next = advance::after(
        &outcome,
        &flow,
        &step_id,
        attempts,
        record.kin.parent.is_some(),
    );
    let evidence = match &outcome {
        gate::Outcome::Holds { evidence } | gate::Outcome::DoesNotHold { evidence, .. } => {
            evidence.clone()
        }
        gate::Outcome::NotYet { .. }
        | gate::Outcome::AsksAPerson { .. }
        | gate::Outcome::CannotDecide { .. } => Vec::new(),
    };

    // **Nothing is settled until the verdict is written, and the verdict is
    // written by `fleet.verdict`.** This is the one caller that legitimately
    // reaches it: the separation that shipped today — a Drone reports, the gate
    // decides — is preserved by using the same verb rather than by writing the
    // record here.
    // The inbox entry the verdict opened, if it opened one. Kept because a gate
    // waiting on a person has to read *that* answer back.
    let mut opened = None;
    if let Some(reached) = next.verdict() {
        let why = match &next {
            advance::Next::Ask { question } => Some(question.clone()),
            advance::Next::Halt { why, .. } => Some(why.clone()),
            _ => None,
        };
        let wrote = record_gate_verdict(
            now,
            place,
            &record.name,
            &step_id,
            reached,
            evidence.clone(),
            why.as_deref(),
        )?;
        if let Output::Verdict(envelope) = &wrote {
            opened.clone_from(&envelope.data.entry);
        }
        record = place.store().find(&record.name)?;
    }

    match next {
        advance::Next::Again { why } => {
            let verdict = record.verdict;
            Ok(tick_row(
                &record,
                // **A parent whose child is waiting on a person is `asked`, not
                // `waiting`, and the difference is whether `--watch` ever
                // returns.** `waiting` means *something will decide this on its
                // own* — a detached check finishes — and the loop keeps looking.
                // A sub-Job holding an unanswered inbox entry will not finish
                // until somebody answers it, so a `--watch` that called that
                // live would poll until the person it is waiting for gave up on
                // it. Nothing is raised here: the child already raised the
                // question, and a second entry about the same one is the
                // dilution PLAN.md §15.4 warns about.
                match waiting_on_a_person {
                    true => TICK_ASKED,
                    false => TICK_WAITING,
                },
                why,
                predicate,
                evidence,
                verdict,
            ))
        }
        advance::Next::Advance { to } => {
            record.pending = None;
            record.step.clone_from(&to);
            if let Err(error) = start_step(run, place, &mut record, &flow, &to, None) {
                return stalled(place, now, record, predicate, error);
            }
            place.store().save(&record)?;
            // **A step no Drone runs is gated in the pass that enters it,
            // because nothing will ever make it *due*.**
            //
            // `020` §6's watermark asks *has anything gated the exchange that
            // just ended?* — `finished > ticked_turns` — and that question has
            // no answer for a step Fleet satisfies: no Drone runs, so the
            // transcript never grows, so `due` is false for ever. The Job would
            // then be observed with no live Drone, nothing pending and nothing
            // due, which `job::observe_state`'s last arm calls `STALLED` and
            // `advance::attention` answers `Idle` — a Job dead in the water
            // that even the sweep would step over. Measured, not assumed: a
            // parent advanced into `review` observed
            // `state=Stalled due=false -> Idle`.
            //
            // Gating it here is the honest fix rather than a second watermark:
            // there is genuinely nothing to wait for, and the pass already
            // holds everything the gate needs. It terminates because
            // `Workflow::step_after` only ever moves forward.
            if !runs_a_drone(&flow, &to) {
                return gate_step(run, now, place, record, observed, reading);
            }
            Ok(tick_row(
                &record,
                TICK_ADVANCED,
                format!("`{step_id}` passed; it is on `{to}`"),
                predicate,
                evidence,
                Some(Verdict::Pass),
            ))
        }
        advance::Next::Retry { attempt, why } => {
            record.pending = None;
            if let Err(error) = start_step(run, place, &mut record, &flow, &step_id, Some(&why)) {
                return stalled(place, now, record, predicate, error);
            }
            place.store().save(&record)?;
            Ok(tick_row(
                &record,
                TICK_RETRIED,
                format!("`{step_id}` did not pass ({why}); attempt {attempt}"),
                predicate,
                evidence,
                Some(Verdict::Failed),
            ))
        }
        advance::Next::Finish => {
            record.pending = None;
            // **A Job releases what it holds when it ends.** Everything below
            // is the same teardown `kill` runs, in the same order, differing
            // only in where it lands and what it keeps.
            let ended = release_on_finish(run, now, place, &mut record)?;
            let mut why = format!("`{step_id}` was its last step");
            if let Some(summary) = ended.released.summary() {
                why.push_str(&format!("; released {summary}"));
            }
            match ended.worktree {
                Disposition::Removed => why.push_str(", and its worktree"),
                // **Said out loud, because it is the reason a directory is
                // still there.** A reader who finds a finished Job's worktree
                // on disk with nothing explaining it concludes the removal is
                // broken; a reader told it holds uncommitted work goes and
                // looks at it, which is the right next move.
                Disposition::Kept => why.push_str(
                    ", and kept its worktree: it holds uncommitted work, so `armada fleet reap` \
                     is where removing it is a deliberate act",
                ),
                Disposition::Gone => {}
            }
            let mut row = tick_row(
                &record,
                TICK_FINISHED,
                why,
                predicate,
                evidence,
                Some(Verdict::Pass),
            );
            row.released = Some(ended.released);
            Ok(row)
        }
        advance::Next::Hand { why } => {
            record.pending = None;
            record.state = JobState::Paused;
            record.verdict = Some(Verdict::NeedsHuman);
            raise(place, now, &mut record, inbox::Kind::NeedsHuman, &why)?;
            place.store().save(&record)?;
            Ok(tick_row(
                &record,
                TICK_ASKED,
                why,
                predicate,
                evidence,
                Some(Verdict::NeedsHuman),
            ))
        }
        // `verdict` already recorded `NEEDS_HUMAN` and raised the question.
        advance::Next::Ask { question } => {
            // **Which entry, remembered.** The answer is read back off this id
            // and no other — see [`gather`]'s `Person` arm for why an open-entry
            // lookup cannot work, and `job::Pending` for why the attempt travels
            // with it.
            if let Some(entry) = opened {
                record.pending = Some(job::Pending {
                    step: step_id.clone(),
                    on: job::Waiting::Answer(entry),
                    attempt: attempts,
                });
                place.store().save(&record)?;
            }
            let verdict = record.verdict;
            Ok(tick_row(
                &record, TICK_ASKED, question, predicate, evidence, verdict,
            ))
        }
        advance::Next::Halt { why, .. } => {
            let verdict = record.verdict;
            Ok(tick_row(
                &record,
                TICK_HALTED,
                why,
                predicate,
                evidence,
                verdict,
            ))
        }
    }
}

/// Stop a Job whose next exchange could not be started, and say why.
///
/// **One Job's broken machine must not end the pass.** `gate_step` already
/// refuses to fail the whole loop over a workflow it cannot read, for the reason
/// given there — one Job with a missing workflow must not stop the loop moving
/// every other Job on — and a Job whose worktree has been deleted under it is
/// the same failure arriving one step later. Propagating it would mean `armada
/// fleet tick` over a fleet of twenty exited `6` and moved none of the other
/// nineteen, and `--watch` would stop dead on it.
///
/// **The error's own words, not a summary.** `start_step` says which worktree is
/// gone and what to run about it; a row saying *"could not start"* would send
/// the reader back to the terminal to find out what Armada already knew.
fn stalled<C: Clock>(
    place: &Where,
    now: &C,
    record: Job,
    predicate: Option<String>,
    error: ArmadaError,
) -> Result<TickRow, ArmadaError> {
    let why = match &error.next_action {
        Some(next) => format!("{} — {next}", error.message),
        None => error.message.clone(),
    };
    halt(place, now, record, predicate, why)
}

/// Stop a Job the loop cannot gate at all, and say why.
fn halt<C: Clock>(
    place: &Where,
    now: &C,
    mut record: Job,
    predicate: Option<String>,
    why: String,
) -> Result<TickRow, ArmadaError> {
    record.state = JobState::Paused;
    record.verdict = Some(Verdict::NeedsHuman);
    raise(place, now, &mut record, inbox::Kind::NeedsHuman, &why)?;
    place.store().save(&record)?;
    Ok(tick_row(
        &record,
        TICK_HALTED,
        why,
        predicate,
        Vec::new(),
        Some(Verdict::NeedsHuman),
    ))
}

/// Name the slow part of an action, and write it where another reader can see
/// it (`020` §5).
///
/// **A write per stage, and it is cheap on purpose.** Three small saves during
/// an abort is the whole cost of a row that says which of `drone`, `docker` and
/// `worktree` is taking the time — against an abort that said nothing at all
/// for several seconds and was indistinguishable from a hung one.
///
/// **A failed write does not stop the action.** Reporting what an abort is
/// doing must never be able to prevent the abort; if the record will not take
/// the transient, the caller carries on and the reader is back where they were
/// rather than worse off.
///
/// **`None` in, `None` out, and no write at all.** A teardown with no action
/// word is the loop finishing a Job whose last step passed; there is nothing to
/// watch there, and stamping `ABORTING` on it would name an action nobody took.
fn stage(
    store: &Store,
    record: &mut Job,
    doing: Option<&job::Doing>,
    slow: &str,
    now_ms: u64,
) -> Option<job::Doing> {
    let moved = doing?.at(slow, now_ms);
    record.doing = Some(moved.clone());
    // Best effort: reporting what a teardown is doing must never be able to
    // stop the teardown.
    let _ = store.save(record);
    Some(moved)
}

/// Whether a Drone runs this step.
///
/// **The one place [`workflow::Runner`] is read in the loop**, because two
/// decisions turn on it and they must not disagree: whether [`start_step`]
/// starts a Drone at all, and whether [`gate_step`] may leave the step for a
/// later pass. A step nobody runs a Drone for produces no exchange, and a step
/// that produces no exchange is never *due* a gate (`020` §6) — so a Job left
/// resting on one is a Job nothing will ever look at again.
///
/// A step that is not in the workflow answers `false`, which sends it to
/// `gate_step`'s own refusal — *"the workflow has no step called …"* — rather
/// than starting a Drone on a step nobody declared.
fn runs_a_drone(flow: &Workflow, step: &str) -> bool {
    flow.steps
        .iter()
        .find(|candidate| candidate.id == step)
        .is_some_and(|candidate| matches!(candidate.runner(), workflow::Runner::Drone(_)))
}

/// Persist an observation only when it changed something.
///
/// **A save per pass on an idle fleet would rewrite every record every two
/// seconds**, which is a lot of writes to record that nothing happened — and it
/// would re-raise a stall the inbox has already reported.
fn settle_if_changed<C: Clock>(
    record: &mut Job,
    observed: &Observed,
    place: &Where,
    now: &C,
) -> Result<(), ArmadaError> {
    if record.state == observed.state && record.spend == observed.spend {
        return Ok(());
    }
    settle(record, observed, place, now)?;
    place.store().save(record)
}

/// One row of the pass.
fn tick_row(
    record: &Job,
    did: &str,
    why: String,
    predicate: Option<String>,
    evidence: Vec<Evidence>,
    verdict: Option<Verdict>,
) -> TickRow {
    TickRow {
        job: record.name.clone(),
        step: record.step.clone(),
        did: did.to_string(),
        state: record.state,
        verdict,
        predicate,
        evidence,
        why,
        released: None,
    }
}

/// Start a Drone on a step.
///
/// **`--resume`, not a fresh session.** A Job's conversation is one Claude Code
/// session across every step (PLAN.md §14.1); starting the next step in a new
/// one would throw away everything the last step established and pay to
/// rediscover it.
///
/// # The step that starts no Drone
///
/// A step names at most one of `skill:` and `workflow:`, and
/// [`workflow::Runner`] is what that means: a skill is a Drone's, a workflow is
/// a sub-Job's, and **neither is Fleet's** — as `review` is, by spawning a
/// reviewer. Starting this Job's Drone on one of those two would be asking the
/// Job under review to review itself, at the cost of a turn against its own
/// ceiling, and its answer would not be evidence in any case.
///
/// **Except on a retry, which is exactly when it is the Drone's to fix.** A
/// gate that did not hold hands back words — *"there is nothing committed for a
/// reviewer to read"*, *"the reviewer finished FAILED"* — and the only thing
/// that can act on either is the session that did the work. So `failed` is what
/// decides: nothing to say means nothing to start.
///
/// **The first step is never this**, because it is [`spawn`]'s and not this
/// function's — and it must not be, for a reason below the workflow entirely:
/// `--resume` needs a session that exists, and a Job whose first step started no
/// Drone would have no session for any later step to resume.
fn start_step<R: Run>(
    run: &R,
    place: &Where,
    record: &mut Job,
    flow: &Workflow,
    step: &str,
    failed: Option<&str>,
) -> Result<(), ArmadaError> {
    if failed.is_none() && !runs_a_drone(flow, step) {
        // The Job stays `RUNNING` with no Drone, and [`gate_step`] gates the
        // step in this same pass — it has to, because a step with no exchange
        // is never *due* one and would otherwise be observed as `STALLED`.
        record.state = JobState::Running;
        return Ok(());
    }
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
    let mut ask = prompt(flow, step, &record.task, &record.facts);
    if let Some(failed) = failed {
        // **The gate's own words, handed back.** A retry that started with the
        // same prompt as the first attempt is an agent asked to do the same
        // thing again with no idea what was wrong with the last answer.
        //
        // **And the log's own words with them**, which is the half that was
        // missing. The sentence names the check and its log path; a Drone that
        // has to open the file is a Drone that may not, and one measured Job
        // failed the same gate nine times in an hour on a single dead-code
        // warning it was never shown. The Job ran the check and holds the
        // output — handing down a path instead of the text is asking an agent
        // to rediscover a fact its Job already established.
        ask = format!(
            "{ask}\n\nThe previous attempt did not pass this step's gate: {failed}\n\
             Fix that. Armada re-runs the gate when your turn ends."
        );
        for tail in logs(&path, failed) {
            ask.push_str(&tail);
        }
    }
    record.state = JobState::Running;
    record.drone = Some(start_drone(
        run,
        place,
        record,
        &path,
        argv::resume_argv(
            &record.uuid,
            &ask,
            &place.posture()?,
            place.relay(&record.uuid).as_deref(),
            place.drone_mcp(&record.uuid).as_deref(),
        ),
    )?);
    Ok(())
}

// --------------------------------------------------------- gathering the facts

/// Look at whatever this step's predicate needs looked at.
///
/// **Driven by [`gate::Needs`], and it is the only I/O in the gate.** Nothing
/// here decides anything: every branch answers a question and hands the answer
/// back for [`gate::decide`] to weigh, which is what keeps the decision testable
/// with values and no filesystem.
///
/// **Eight arguments, and each of them is a seam rather than a parameter**:
/// three are the injected ones `ARCHITECTURE.md` §1.1 names — the process
/// runner, the clock and where the machine is — and the rest are the record,
/// what the gate wants, which step, the transcript already read, and the pending
/// slot. Bundling them into a struct would name the tuple `GatherArgs` and move
/// the same eight things one line up.
///
/// **The resolved `fleet.land.merge` travels back alongside the facts,
/// rather than living in [`gate::Facts`].** It answers a different question
/// from every other field there — not *what did a command find*, but *what
/// has this repository consented to* — and [`gate::decide`] takes it as its
/// own parameter for exactly that reason (gate.rs's own doc comment: "the
/// one place this module stops being config-blind, and only for this one
/// predicate"). It is read here rather than in `gate_step` because this is
/// still the only I/O in the gate — `armada_fleet::manifest::land_merge` is a
/// local file read, gated behind the one predicate that needs it.
#[allow(clippy::too_many_arguments)]
fn gather<R: Run, C: Clock>(
    run: &R,
    now: &C,
    place: &Where,
    record: &mut Job,
    want: &gate::Needs,
    step: &str,
    reading: &Reading,
    pending: &mut Option<job::Pending>,
) -> Result<(gate::Facts, LandMerge), ArmadaError> {
    let worktree = place.expand(&record.worktree);
    let mut facts = gate::Facts::default();
    let mut land_merge = LandMerge::Never;

    match want {
        gate::Needs::Nothing => {
            // **How the exchange ended, read off the transcript.** The turn's
            // own `is_error` is the fact; `--print` exits before anything could
            // ask the process, which is why this is the ledger and not a wait.
            facts.turn = reading.last().map(|turn| gate::Probed {
                scope: step.to_string(),
                exit: i32::from(turn.is_error),
            });
        }
        gate::Needs::GreenCheck { scope } => {
            facts.check = check(run, place, record, step, scope.as_deref(), pending)?;
        }
        gate::Needs::RedCheck { test, scope } => {
            facts.test = Some(searched(run, &worktree, test));
            // **The search first, and the check only if it found something.** A
            // check run costs a repository's whole suite; starting one to
            // decide a predicate whose other half has already failed is the one
            // avoidable expense in the loop.
            if facts.test.as_ref().is_some_and(gate::Probed::found) {
                facts.check = check(run, place, record, step, scope.as_deref(), pending)?;
            }
        }
        gate::Needs::Path { path } => {
            // **Joined onto the worktree, and an absolute path is refused by
            // `join` doing exactly what it is asked.** A workflow naming `/etc`
            // would be gating on a file outside the Job entirely, which is a
            // `armada guild verify` finding rather than something to silently
            // allow here — it is reported as *not on disk* until that verb
            // exists to refuse it (`docs/reserved/016`).
            // **A pattern, not only a path.** The shipped `design` workflow
            // names its artifact `docs/design/*.md`, and `join` + `exists`
            // looked for a file literally called `*.md` — so `articulate` could
            // never pass, whatever the Drone wrote. Measured 2026-08-16: a Job
            // wrote and committed `docs/design/hello-format.md`, the gate said
            // the artifact was absent, and the Job retried until it hit its
            // token ceiling. Every `design` Job was unpassable at that step.
            //
            // A literal path still takes the cheap route, which is both faster
            // and the only thing that can answer for a name containing no
            // wildcard at all.
            let found = if gate::is_glob(path) {
                let (dir, name) = gate::glob_parts(path);
                std::fs::read_dir(worktree.join(dir))
                    .map(|entries| {
                        entries.filter_map(Result::ok).any(|entry| {
                            entry
                                .file_name()
                                .to_str()
                                .is_some_and(|found| gate::glob_matches(found, name))
                        })
                    })
                    // **A directory that will not open is *not found*, not an
                    // error.** The commonest reason is that the step has not
                    // created it yet, which is exactly the answer the gate
                    // wants; failing the run instead would turn a step's first
                    // attempt into a fault.
                    .unwrap_or(false)
            } else {
                worktree.join(path).exists()
            };
            facts.artifact = Some(gate::Probed {
                scope: path.clone(),
                exit: i32::from(!found),
            });
        }
        gate::Needs::Branch => {
            facts.branch = Some(committed(run, &worktree, &record.branch));
        }
        gate::Needs::Person => {
            // **The entry this attempt asked, by id — never *an* open entry.**
            // `inbox::open_for` finds nothing the moment you reply, because
            // `Entry::is_open` is false once `answered` is set; a gate that
            // looked for an open entry would therefore never see the answer and
            // would ask the same question for ever. The id is remembered in
            // `pending` when the question is raised, and it carries the attempt
            // with it for the reason `job::Pending` gives: *"yes, ship it"*
            // about the second thing you were asked is not approval of the
            // third.
            let attempt = job::step_failures(&record.transitions, step).saturating_add(1);
            let asked = pending
                .as_ref()
                .filter(|open| open.step == step && open.attempt == attempt)
                .and_then(|open| match &open.on {
                    job::Waiting::Answer(entry) => Some(entry.clone()),
                    job::Waiting::Check(_) | job::Waiting::SubJob(_) => None,
                });
            facts.answer = match asked {
                None => None,
                Some(id) => entries(place)?
                    .into_iter()
                    .find(|entry| entry.uuid == id)
                    .and_then(|entry| entry.answered),
            };
        }
        gate::Needs::SubJob { workflow, kind } => {
            // **The commit first, and the reviewer only if there is one.** A
            // worktree branched from a branch with nothing on it hands the
            // reviewer an empty diff, and spawning a whole Job to read nothing
            // is the same avoidable expense the `RedCheck` arm refuses one line
            // up. `decide` is what turns the missing commit into words the
            // parent's next turn is given.
            if *kind == gate::SubJobKind::Review {
                facts.branch = Some(committed(run, &worktree, &record.branch));
                if !facts.branch.as_ref().is_some_and(gate::Probed::found) {
                    return Ok((facts, land_merge));
                }
            }
            facts.subjob = subjob(run, now, place, record, step, workflow, *kind, pending)?;
        }
        gate::Needs::Pr { .. } => {
            facts.pr = pr(run, &worktree, &record.branch);
            // **Read once policy is actually in question, not on every
            // step.** Every other step's gate has nothing to do with
            // `fleet.land.merge`, and reading a repository's `armada.yml` on
            // every pass over every step of every Job would be I/O this
            // module's whole design keeps out of everywhere but here.
            land_merge = manifest::land_merge(&worktree);
        }
        // Nothing to look at: `decide` says why rather than guessing.
        gate::Needs::Unstated { .. } => {}
    }
    Ok((facts, land_merge))
}

/// Start the sub-Job this attempt needs, or read the one already running.
///
/// **One child per attempt, and the attempt travels with the uuid** — the same
/// rule [`check`] states for a check run, for the same reason and with more at
/// stake. A reviewer started for attempt one read the diff *before* the fix;
/// settling attempt two with its verdict would pass a step on a reading of work
/// that no longer exists.
///
/// **What it does on the way past is bookkeeping the parent cannot do
/// afterwards**: it opens the wall-clock suspension when the child starts, and
/// closes it — rolling the child's spend into the parent's ledger — the moment
/// the child is over. Both are on the record before anything acts on the fact,
/// because `gate_step` saves immediately after this returns.
#[allow(clippy::too_many_arguments)]
fn subjob<R: Run, C: Clock>(
    run: &R,
    now: &C,
    place: &Where,
    record: &mut Job,
    step: &str,
    workflow: &str,
    kind: gate::SubJobKind,
    pending: &mut Option<job::Pending>,
) -> Result<Option<gate::SubjobFact>, ArmadaError> {
    let attempt = job::step_failures(&record.transitions, step).saturating_add(1);
    // **A pending check is not a pending sub-Job.** All three ids are opaque
    // strings and only [`job::Waiting`] tells them apart.
    let mine = pending
        .as_ref()
        .filter(|open| open.step == step && open.attempt == attempt)
        .and_then(|open| match &open.on {
            job::Waiting::SubJob(uuid) => Some(uuid.clone()),
            job::Waiting::Check(_) | job::Waiting::Answer(_) => None,
        });

    let Some(uuid) = mine else {
        let child = spawn_child(run, now, place, record, step, attempt, workflow, kind)?;
        *pending = Some(job::Pending {
            step: step.to_string(),
            on: job::Waiting::SubJob(child.uuid.clone()),
            attempt,
        });
        // **The parent's wall clock stops here.** PLAN.md §14.6: a plan sub-Job
        // ends at your approval, approval takes hours, and a parent whose clock
        // kept running would be killed because you went to lunch.
        record.kin.suspended_from_ms.get_or_insert(now.wall_ms());
        return Ok(None);
    };

    // A record that will not load is a child somebody deleted by hand. It is
    // reported as *ended without a verdict* rather than as a missing file,
    // because that is what it is from here: nothing is going to arrive.
    let Ok(child) = place.store().load(&uuid) else {
        return Ok(Some(gate::SubjobFact {
            uuid: uuid.clone(),
            name: job::short(&uuid),
            state: JobState::Aborted,
            verdict: None,
        }));
    };
    let (observed, _, _) = look(run, place, &child, now.wall_ms());

    if observed.state.is_over() {
        // **Counted once, on the pass that sees it over.** `decide` settles the
        // attempt from this same fact — `Holds` advances, `DoesNotHold` retries,
        // and both clear `pending` — so this cannot be reached twice for one
        // child. What it buys is a ceiling that bounds the whole tree: a parent
        // spends no turns of its own while a sub-Job works, so without this its
        // ledger would sit still however many children it started
        // ([`job::Kin::spend`]).
        record.kin.spend.add(&observed.spend);
        // **And the cached total with it.** `record.spend` is what
        // [`job::observe`] last answered, and `gate_step` set it from the
        // transcript at the top of this pass — before there was a child spend
        // to include. Leaving it would put a figure on disk that under-reports
        // the Job by exactly what its children cost, which is the number a
        // person reads off `armada fleet ls` between passes.
        record.spend.add(&observed.spend);
        if let Some(from) = record.kin.suspended_from_ms.take() {
            record.kin.suspended_ms = record
                .kin
                .suspended_ms
                .saturating_add(now.wall_ms().saturating_sub(from));
        }
    }

    Ok(Some(gate::SubjobFact {
        uuid: child.uuid.clone(),
        name: child.name.clone(),
        state: observed.state,
        verdict: child.verdict,
    }))
}

/// How many Jobs deep a chain of sub-Jobs may go.
///
/// **A rope, not a policy.** The thing that makes `feature → plan → feature`
/// wrong is that it is a cycle, and cycles are refused by name below — this is
/// the backstop for the other shape, a guild whose workflows fan out without
/// ever repeating one. Three is what the shipped set needs and one more:
/// `feature` starts a `plan`, and a `plan` that grew a `review` step would be
/// the third.
pub const GENERATIONS: usize = 3;

/// Start the Job a gate is waiting on.
///
/// # Why this is not [`spawn`]
///
/// It does three fewer things and three more. It does not classify — the gate
/// knows the workflow by name, and paying for a model call to be told what the
/// workflow file already says is the one avoidable token in the loop. It does
/// not report progress — nothing is watching a `tick`. It does not ask anybody
/// anything, for the same reason.
///
/// And it carries three facts `spawn` has no way to express: **a parent**, so
/// `kill` and the gate can find the child and the child cannot become an orphan
/// nobody is waiting on; **a start point**, because a reviewer branched from the
/// repository's `HEAD` would be reading the code as it was before the work; and
/// **a carved budget**, so the child cannot spend what the parent has already
/// spent.
///
/// The adapters underneath are the same ones `spawn` calls, in the same order,
/// which is the part that must not be answered twice.
#[allow(clippy::too_many_arguments)]
fn spawn_child<R: Run, C: Clock>(
    run: &R,
    now: &C,
    place: &Where,
    parent: &Job,
    step: &str,
    attempt: u32,
    named: &str,
    kind: gate::SubJobKind,
) -> Result<Job, ArmadaError> {
    let store = place.store();
    refuse_a_cycle(place, parent, named)?;
    let flow = read_workflow(place, named)?;
    // Read and refused here, before anything is created — the same repository
    // as the parent's, so the same `fleet.carry` entry applies (§[`spawn`]).
    let carry = fleet_machine::carry_for(&place.armada_home, &parent.repo_root)?;

    let name = store.free_name(&format!("{}-{named}", parent.name))?;
    let uuid = job::mint_uuid(&format!(
        "{}|{step}|{attempt}|{named}|{}",
        parent.uuid,
        now.wall_ms()
    ));
    let path = home::worktree(&place.armada_home, &parent.repo, &name);
    let branch = worktree::branch_for(&name);
    let first = flow.first_step().id.clone();
    let task = child_task(parent, kind);

    let mut record = Job {
        uuid: uuid.clone(),
        name: name.clone(),
        // A sub-Job inherits what the caller said about the tree, so a raise
        // reaches every generation rather than only the first.
        budget_set: parent.budget_set.clone(),
        workflow: flow.name.clone(),
        // **Not a guess and not a person's answer either.** A confidence here
        // would be a number nobody measured; the workflow was named by the
        // step, which is the same standing `Classification::overridden` has.
        confidence: None,
        repo: parent.repo.clone(),
        repo_root: parent.repo_root.clone(),
        worktree: place.shown(&path),
        branch: branch.clone(),
        port_block: None,
        budget: carved(
            flow.budget,
            flow.ends_at,
            &parent.budget,
            &parent.spend,
            &parent.budget_set,
        ),
        state: JobState::Queued,
        step: first.clone(),
        verdict: None,
        drone: None,
        created_at: now.wall_rfc3339(),
        created_ms: now.wall_ms(),
        spend: Spend::default(),
        task: task.clone(),
        progress: Vec::new(),
        attempts: std::collections::BTreeMap::new(),
        waited_ms: 0,
        waiting_from_ms: None,
        transitions: Vec::new(),
        pending: None,
        // **The parent's `--set` facts, inherited.** A `${task.test}` the
        // person named once at spawn means the same thing to a sub-Job working
        // the same task, and a child that had to be told again would be a
        // human turn in the middle of a workflow that promises none.
        facts: parent.facts.clone(),
        kin: job::Kin {
            parent: Some(job::Parent {
                uuid: parent.uuid.clone(),
                step: step.to_string(),
                attempt,
            }),
            ..job::Kin::default()
        },
        // Nothing has ticked it and nobody is doing anything to it: it is being
        // minted right now.
        ticked_turns: 0,
        doing: None,
        daemon_acts: Vec::new(),
        main_moved_at: None,
    };
    // Recorded before anything is created, for [`spawn`]'s reason: everything
    // after this line can fail, and each of those failures leaves a Job on disk
    // rather than an orphaned worktree holding a port block nobody can name.
    store.save(&record)?;

    let repo_root = place.expand(&parent.repo_root);
    if let Err(error) = worktree::add(run, &repo_root, &path, &branch, Some(&parent.branch)) {
        let _ = manifest::clean(run, &place.exe, &path);
        let _ = worktree::remove(run, &repo_root, &path);
        let _ = worktree::delete_branch(run, &repo_root, &branch);
        record.state = JobState::Aborted;
        record.verdict = Some(Verdict::Failed);
        store.save(&record)?;
        return Err(error);
    }
    // Carried before `armada manifest init` runs, for the same reason
    // `spawn`'s own copy step is: `setup:` may depend on it.
    if let Err(error) = worktree::carry(&repo_root, &path, &carry) {
        let _ = manifest::clean(run, &place.exe, &path);
        let _ = worktree::remove(run, &repo_root, &path);
        let _ = worktree::delete_branch(run, &repo_root, &branch);
        record.state = JobState::Aborted;
        record.verdict = Some(Verdict::Failed);
        store.save(&record)?;
        return Err(error);
    }

    // A failed `init` aborts and cleans up, for [`spawn`]'s reason: a Drone
    // started into a worktree whose `setup:` never ran is a Job that will fail
    // its first gate for a reason no agent can act on.
    let block = match manifest::init(run, &place.exe, &path) {
        Ok(block) => block,
        Err(error) => {
            let _ = manifest::clean(run, &place.exe, &path);
            let _ = worktree::remove(run, &repo_root, &path);
            let _ = worktree::delete_branch(run, &repo_root, &branch);
            record.state = JobState::Aborted;
            record.verdict = Some(Verdict::Failed);
            store.save(&record)?;
            return Err(error);
        }
    };
    record.port_block = block;
    record.state = JobState::Running;
    store.save(&record)?;

    record.drone = Some(start_drone(
        run,
        place,
        &record,
        &path,
        argv::spawn_argv(
            &uuid,
            &prompt(&flow, &first, &task, &record.facts),
            &place.posture()?,
            // **Both of these are load-bearing for a sub-Job in particular.**
            // Without the relay (`020` §1) nothing observes the child's
            // exchange ending, so the child never advances and the parent waits
            // on it for ever; without its own MCP config the child has no
            // `mcp__armada__fleet_verdict` to report through, and its verdict
            // is the entire thing the parent's gate reads.
            place.relay(&uuid).as_deref(),
            place.drone_mcp(&uuid).as_deref(),
        ),
    )?);
    store.save(&record)?;
    Ok(record)
}

/// What the child is told it is for.
///
/// **The reviewer is told the branch and the claim, and nothing about how the
/// work went.** PLAN.md §14.6: *"a reviewer that shares the implementer's
/// context shares its blind spots"* — so what it gets is what a colleague would
/// get, the diff and the thing it was supposed to do, in a worktree of its own
/// at that branch.
fn child_task(parent: &Job, kind: gate::SubJobKind) -> String {
    match kind {
        gate::SubJobKind::Review => format!(
            "Review the work `{}` did on `{}`. Your worktree is that branch: the commits it \
             added are the change, and `git log` and `git show` are how you read them. Say \
             what would block landing it.\n\nThe task it was given: {}",
            parent.name, parent.branch, parent.task
        ),
        gate::SubJobKind::Workflow => parent.task.clone(),
    }
}

/// The ceilings a child runs under.
///
/// # Only cost is carved, because only cost is a tree-wide resource
///
/// **Cost is the smaller of what the child's workflow asks for and what the
/// parent has left.** Money a child spends is money the tree spent — 016 §2 asks
/// for the parent's ceilings to bound the child's, and a child that could spend
/// a fresh $10 is how a parent gets exhausted by something it never counted.
///
/// **The other two are the child's own, for the same reason stated twice.**
/// The parent's clock is *suspended* while the child runs
/// ([`job::Kin::suspended_ms`]), so there is nothing of it to carve. And the
/// attempt ceiling counts retries **at one step** — the child is running a
/// different workflow with different steps, and its own first step has been
/// attempted zero times no matter what the parent has been through. Carving it
/// would hand a child two attempts because its parent had used one on an
/// unrelated gate.
///
/// This used to carve `iterations` as well, when that field held a whole-Job
/// turn count and so plausibly described a shared pool. It never really did:
/// see [`workflow::Budget::attempts`] for what it was actually counting.
///
/// # A key the caller set is not a default, so it is not what the child is held to
///
/// Measured 2026-08-16: a parent spawned with `--budget max_cost=25.00` gave
/// its planning sub-Job `plan.yml`'s $5.00, because `min` chose the smaller.
/// The raise never reached the work it was meant for. For a key in
/// [`job::Job::budget_set`] the child gets the parent's **remaining** instead —
/// still bounded by what the parent actually has, so the containment argument
/// above is untouched; what changes is which number is the default and which is
/// the instruction.
/// How much of a parent's remaining budget a step that produces a document may
/// take, when the caller has raised that budget.
///
/// **A third, and the number is a judgement rather than a measurement.** What is
/// measured is the failure it answers: one `plan` sub-Job took $12.37 of $20
/// before implementation began. A third leaves two thirds for the work the
/// planning was for, which is the ordering the `feature` workflow already
/// implies — `plan` is one step of five.
const PLANNING_SHARE: f64 = 1.0 / 3.0;

/// # A raise reaches a step that writes code; a step that writes a document gets a share
///
/// **Measured 2026-08-16, an hour after the raise above was built.** A `plan`
/// sub-Job under a parent spawned with `--budget max_cost=20.00` spent **$12.37
/// planning** — 177 turns and an hour of API time — and stopped at its approval
/// gate with no code written. The raise had reached it, exactly as intended, and
/// handed the whole remainder to the cheap half of the work.
///
/// So a child whose workflow [`workflow::EndsAt::Human`] — `design`, `plan`,
/// `review`, the ones whose output is a document rather than a change — gets
/// [`PLANNING_SHARE`] of what the parent has left rather than all of it.
/// Planning is meant to be the cheap part, and a plan step that can take the
/// whole budget inverts that before implementation begins.
///
/// **Never below what the workflow itself declares, and never above what the
/// parent has.** The share is a ceiling on a raise, not a second way to starve a
/// sub-Job — which was the original defect and must not come back.
fn carved(
    declared: workflow::Budget,
    ends_at: workflow::EndsAt,
    parent: &workflow::Budget,
    spent: &Spend,
    set: &[String],
) -> workflow::Budget {
    let left = job::remaining(parent, spent, 0, 0);
    let told = |key: &str| set.iter().any(|k| k == key);
    workflow::Budget {
        // Per-step and per-workflow, so there is nothing of the parent's to
        // carve — a raise the caller typed still reaches the child, because a
        // child spawned with `--budget max_attempts` gets it on its own record.
        attempts: declared.attempts,
        cost_usd: match told("max_cost") {
            true => match ends_at {
                workflow::EndsAt::Branch => left.cost_usd,
                workflow::EndsAt::Human => (left.cost_usd * PLANNING_SHARE)
                    .max(declared.cost_usd)
                    .min(left.cost_usd),
            },
            false => declared.cost_usd.min(left.cost_usd),
        },
        // The clock stays the child's own either way: the parent's is suspended
        // while the child runs, so there is nothing of it to carve or to raise.
        wall_clock_ms: declared.wall_clock_ms,
        on_exhausted: declared.on_exhausted,
    }
}

/// Refuse a sub-Job that would run a workflow already running above it.
///
/// **The acyclicity `guild verify` would have enforced, enforced where the edge
/// is actually taken.** `workflow.schema.json` says the graph *"must be
/// acyclic; `armada guild verify` rejects a cycle"* — and that verb is not
/// built (`AGENTS.md`), so `feature → plan → feature` would today be a fleet
/// that grows until every ceiling in it is reached. Checked against the chain
/// on disk rather than against the guild's documents, which is both cheaper —
/// the chain is at most [`GENERATIONS`] records — and stricter: it catches a
/// cycle somebody introduced by editing a workflow while a Job was running
/// through it.
fn refuse_a_cycle(place: &Where, parent: &Job, named: &str) -> Result<(), ArmadaError> {
    let mut chain = vec![parent.workflow.clone()];
    let mut above = parent.kin.parent.clone();
    while let Some(link) = above {
        let Ok(ancestor) = place.store().load(&link.uuid) else {
            break;
        };
        chain.push(ancestor.workflow.clone());
        above = ancestor.kin.parent.clone();
    }
    if chain.iter().any(|workflow| workflow == named) {
        chain.reverse();
        chain.push(named.to_string());
        return Err(ArmadaError {
            class: ErrClass::BadConfig,
            r#where: format!("workflows/{named}.yml"),
            message: format!("`{named}` would run inside itself: {}", chain.join(" → ")),
            next_action: Some(format!(
                "edit the `{named}` workflow, or the one that names it, so the two do not \
                 depend on each other"
            )),
        });
    }
    if chain.len() >= GENERATIONS {
        chain.reverse();
        chain.push(named.to_string());
        return Err(ArmadaError {
            class: ErrClass::BadConfig,
            r#where: format!("workflows/{named}.yml"),
            message: format!(
                "sub-Jobs are {GENERATIONS} deep already: {}",
                chain.join(" → ")
            ),
            next_action: Some(
                "flatten one of these workflows: a step that runs a workflow that runs a \
                 workflow is work nobody can follow"
                    .to_string(),
            ),
        });
    }
    Ok(())
}

/// Start a check for this step, or read the one already running.
///
/// **One run per attempt, and the attempt travels with the id.** A check started
/// for attempt one must never settle attempt two: the Drone has rewritten the
/// worktree in between, and a stale green would advance a step on a run that
/// predates the work it is judging.
fn check<R: Run>(
    run: &R,
    place: &Where,
    record: &Job,
    step: &str,
    scope: Option<&str>,
    pending: &mut Option<job::Pending>,
) -> Result<Option<gate::CheckFact>, ArmadaError> {
    let worktree = place.expand(&record.worktree);
    let attempt = job::step_failures(&record.transitions, step).saturating_add(1);

    // **A pending answer is not a pending check.** The two ids are both opaque
    // strings and only [`job::Waiting`] tells them apart, so matching the variant
    // is what stops an inbox entry id being handed to `check --status`.
    let mine = pending
        .as_ref()
        .filter(|open| open.step == step && open.attempt == attempt)
        .and_then(|open| match &open.on {
            job::Waiting::Check(run) => Some(run.clone()),
            job::Waiting::Answer(_) | job::Waiting::SubJob(_) => None,
        });

    let Some(run_id) = mine else {
        // **Started, and nothing is decided this pass.** `--detach` returns as
        // soon as the run is handed to its own session, so the answer arrives on
        // a later pass — which is the whole reason the loop is a repeatable verb
        // rather than one long call.
        let started = armada_fleet::manifest::check_detach(run, &place.exe, &worktree, scope)?;
        *pending = Some(job::Pending {
            step: step.to_string(),
            on: job::Waiting::Check(started),
            attempt,
        });
        return Ok(None);
    };

    let (status, exit, failed) =
        armada_fleet::manifest::check_status(run, &place.exe, &worktree, &run_id)?;
    if status.is_terminal() {
        // **Cleared the moment it decided.** A settled run left pending would be
        // read again on the next attempt and answer about the wrong worktree.
        *pending = None;
    }
    Ok(Some(gate::CheckFact {
        run: run_id,
        status,
        exit,
        // **Carried across the module boundary by shape, not by type.** Fleet's
        // reader and the gate's fact are two crates' spellings of one thing, and
        // the gate may not depend on the shell that produced it.
        failed: failed
            .into_iter()
            .map(|one| gate::FailedCheck {
                id: one.id,
                log: one.log,
                said: one.said,
            })
            .collect(),
    }))
}

/// The tail of each log the gate's sentence named, ready to append to a retry
/// prompt.
///
/// # Why the text and not the path
///
/// The `why` a failed gate produces already names the check and where it wrote
/// — `armada:lint exited 101, in .armada/run/01M0…/logs/armada.lint.log`. That
/// is enough for a person and demonstrably not enough for a Drone: measured
/// 2026-08-17, `job-drives-the-drone` failed its `implement` gate **nine
/// consecutive times over an hour** on one dead-code warning, committing clean
/// work each attempt, and was never shown the compiler's sentence. It cost $12.
///
/// The Job ran the check and the output is on its disk. Handing down a path
/// instead of the text asks an agent to rediscover a fact its Job already has —
/// which is the rule this codebase keeps relearning: **a Drone cannot be
/// trusted with anything the Job can establish itself.**
///
/// # Bounded, and from the end
///
/// [`LOG_TAIL`] lines from the bottom. A compiler puts the error last and the
/// summary after it, so the tail is where the answer is; the head is where the
/// noise is. Bounded because a failing test suite writes megabytes and a prompt
/// that carried all of it would spend the exchange on output nobody reads.
///
/// **Every read is best effort.** A log that cannot be opened is one absent
/// block, never a retry that fails to start — the gate has already decided, and
/// this is the explanation rather than the decision.
fn logs(worktree: &Path, why: &str) -> Vec<String> {
    why.split_whitespace()
        .filter_map(|word| word.strip_suffix(&[',', ';'][..]).or(Some(word)))
        .filter(|word| word.ends_with(".log"))
        .filter_map(|relative| {
            let text = std::fs::read_to_string(worktree.join(relative)).ok()?;
            let tail: Vec<&str> = text
                .lines()
                .rev()
                .take(LOG_TAIL)
                .collect::<Vec<_>>()
                .into_iter()
                .rev()
                .collect();
            match tail.iter().all(|line| line.trim().is_empty()) {
                true => None,
                false => Some(format!(
                    "\n\nThe last {} lines of `{relative}`:\n\n{}\n",
                    tail.len(),
                    tail.join("\n")
                )),
            }
        })
        .collect()
}

/// How many lines of a failed check's log a retry prompt carries.
///
/// **Forty, from the end.** Enough for a compiler error with its span, its note
/// and its help — the shape `-D warnings` produces is six lines and a summary —
/// and enough for the last few failures of a test suite. Not so many that a
/// megabyte of output becomes the exchange.
const LOG_TAIL: usize = 40;

/// Whether a named test is anywhere in the Job's worktree.
///
/// **`--untracked`, because the test the Drone just wrote is not committed
/// yet.** The `reproduce` step's whole output is a new failing test, and a
/// search that only looked at tracked files would answer *"you did not write
/// it"* about a file sitting in front of it.
///
/// **A fixed string, not a pattern.** A test name containing a `.` or a `[` is
/// ordinary in most languages and is a regular expression in none of the
/// workflows anybody writes.
fn searched<R: Run>(run: &R, worktree: &Path, test: &str) -> gate::Probed {
    let argv = vec![
        "git".to_string(),
        "grep".to_string(),
        "--untracked".to_string(),
        "--fixed-strings".to_string(),
        "-l".to_string(),
        "-e".to_string(),
        test.to_string(),
    ];
    let exit = run
        .call(&RunRequest::new(argv, worktree.to_path_buf()))
        .ok()
        .and_then(|output| output.code)
        .unwrap_or(1);
    gate::Probed {
        scope: test.to_string(),
        exit,
    }
}

/// Whether the work is committed on the Job's branch.
///
/// Two commands, and the second is what makes the predicate mean anything:
/// `spawn` creates the branch before the Drone starts, so the ref existing is
/// true of every Job from the moment it is minted.
fn committed<R: Run>(run: &R, worktree: &Path, branch: &str) -> gate::Probed {
    let exists = run
        .call(&RunRequest::new(
            vec![
                "git".to_string(),
                "rev-parse".to_string(),
                "--verify".to_string(),
                "--quiet".to_string(),
                format!("refs/heads/{branch}"),
            ],
            worktree.to_path_buf(),
        ))
        .ok()
        .and_then(|output| output.code)
        .unwrap_or(1);
    if exists != 0 {
        return gate::Probed {
            scope: branch.to_string(),
            exit: exists,
        };
    }
    let dirty = run
        .call(&RunRequest::new(
            vec![
                "git".to_string(),
                "status".to_string(),
                "--porcelain".to_string(),
            ],
            worktree.to_path_buf(),
        ))
        .ok()
        .is_some_and(|output| !output.stdout.trim().is_empty());
    match dirty {
        true => gate::Probed {
            scope: format!("{branch}, with changes still uncommitted"),
            exit: 1,
        },
        false => gate::Probed {
            scope: branch.to_string(),
            exit: 0,
        },
    }
}

/// What `gh pr view` reports about the branch's pull request.
///
/// **`None` on any failure, rather than a zeroed [`gate::PrFact`].** A
/// repository with no PR yet — `land-branch` has not pushed and opened one —
/// answers a non-zero exit on `gh pr view <branch>`, and that reads no
/// differently here from `gh` being unauthenticated or unreachable: all three
/// are *nothing found yet*, which is [`gate::decide`]'s `Needs::Pr` arm
/// reading `facts.pr` as `None` and answering [`gate::Outcome::NotYet`] —
/// the same shape every other unstarted probe in this module takes.
fn pr<R: Run>(run: &R, worktree: &Path, branch: &str) -> Option<gate::PrFact> {
    let output = run
        .call(&RunRequest::new(
            vec![
                "gh".to_string(),
                "pr".to_string(),
                "view".to_string(),
                branch.to_string(),
                "--json".to_string(),
                "number,state,mergedAt".to_string(),
            ],
            worktree.to_path_buf(),
        ))
        .ok()?;
    if output.code != Some(0) {
        return None;
    }
    let value: serde_json::Value = serde_json::from_str(&output.stdout).ok()?;
    let number = value.get("number")?.as_u64()?;
    let state = value
        .get("state")
        .and_then(serde_json::Value::as_str)
        .unwrap_or_default();
    let merged = value
        .get("mergedAt")
        .is_some_and(|merged_at| !merged_at.is_null());
    Some(gate::PrFact {
        number,
        open: state == "OPEN",
        merged,
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use armada_core::fleet::workflow::{Budget, EndsAt, OnExhausted};

    fn budget(attempts: u32, cost_usd: f64) -> Budget {
        Budget {
            attempts,
            cost_usd,
            wall_clock_ms: 90 * 60_000,
            on_exhausted: OnExhausted::NeedsHuman,
        }
    }

    /// A record holding a gate, with whatever `pending` names.
    fn holding(on: Option<job::Waiting>) -> Job {
        Job {
            budget_set: Vec::new(),
            uuid: "8077e742-e164-4d93-a496-391f947f550a".to_string(),
            name: "job-drives-the-drone".to_string(),
            workflow: "feature".to_string(),
            confidence: None,
            repo: "armada".to_string(),
            repo_root: "~/code/armada".to_string(),
            worktree: "~/.armada/workspaces/armada/job-drives-the-drone".to_string(),
            branch: "armada/job-drives-the-drone".to_string(),
            port_block: None,
            budget: armada_core::fleet::workflow::DEFAULT_BUDGET,
            state: JobState::Running,
            step: "implement".to_string(),
            verdict: None,
            drone: None,
            created_at: "2026-08-17T03:00:00Z".to_string(),
            created_ms: 1_000,
            spend: Default::default(),
            task: "make the Job drive the Drone".to_string(),
            progress: Vec::new(),
            attempts: Default::default(),
            waited_ms: 0,
            waiting_from_ms: None,
            transitions: Vec::new(),
            pending: on.map(|on| job::Pending {
                step: "implement".to_string(),
                on,
                attempt: 1,
            }),
            facts: Default::default(),
            kin: Default::default(),
            ticked_turns: 0,
            doing: None,
            daemon_acts: Vec::new(),
            main_moved_at: None,
        }
    }

    /// **A Job holding a gate with no Drone to relay it names the keystroke.**
    ///
    /// Measured 2026-08-17: `job-drives-the-drone` sat at `implement` for over an
    /// hour holding a check that had passed 5 of 5. Nothing was wrong with the
    /// Job, the gate or the check — the answer was there and nobody had asked for
    /// it, and one `armada fleet tick` advanced it at once. The row said `RUNNING`
    /// and `implement`, which is what a Job working on a step says, so there was
    /// nothing on the screen to suggest a keystroke would end it.
    ///
    /// The relay is the Drone's `Stop` hook (`docs/reserved/024`), which fires
    /// when a Drone stops *cleanly*; a `SIGKILL`, a crash and a failing hook all
    /// break the chain in silence.
    #[test]
    fn a_job_holding_a_gate_with_no_drone_says_what_it_needs() {
        let check = holding(Some(job::Waiting::Check("01M06YB0EZZ9JC5J".to_string())));
        assert_eq!(
            detail(&check, JobState::Running, None, false),
            "waiting on a check — `arm fleet tick`"
        );

        let subjob = holding(Some(job::Waiting::SubJob("uuid-of-reviewer".to_string())));
        assert_eq!(
            detail(&subjob, JobState::Running, None, false),
            "waiting on a sub-Job — `arm fleet tick`"
        );
    }

    /// **A live Drone is working, and says so.** The keystroke would be wrong
    /// advice: the gate is not waiting to be read, it is waiting to be reached.
    #[test]
    fn a_job_with_a_live_drone_still_reports_its_step() {
        let record = holding(Some(job::Waiting::Check("01M06YB0EZZ9JC5J".to_string())));
        assert_eq!(detail(&record, JobState::Running, None, true), "implement");
    }

    /// **An answer is the one gate a tick cannot settle.** It is waiting on a
    /// person, and the inbox entry is how they answer — telling them to tick
    /// would send them to the wrong verb.
    #[test]
    fn a_job_waiting_on_a_person_is_not_told_to_tick() {
        let record = holding(Some(job::Waiting::Answer("93438134".to_string())));
        assert_eq!(detail(&record, JobState::Paused, None, false), "implement");

        // And a Job holding nothing at all is just on its step.
        assert_eq!(
            detail(&holding(None), JobState::Running, None, false),
            "implement"
        );
    }

    /// **A failed check's own words reach the retry, not just its path.**
    ///
    /// Measured 2026-08-17: `job-drives-the-drone` failed its `implement` gate
    /// nine consecutive times over an hour on one dead-code warning. It
    /// committed clean work each attempt and was never shown what the compiler
    /// said — the gate handed back *"`armada manifest check` reached FAILED"*
    /// and nothing else. $12 on a one-line error the Job was holding the answer
    /// to.
    #[test]
    fn a_retry_is_handed_the_last_lines_of_the_log_that_failed_it() {
        let dir = tempfile::tempdir().unwrap();
        let log = dir.path().join(".armada/run/01M0/logs/armada.lint.log");
        std::fs::create_dir_all(log.parent().unwrap()).unwrap();
        std::fs::write(
            &log,
            "Compiling armada-helm\n\
             error: function `word_to_verdict` is never used\n   \
             --> crates/helm/src/mcp/drone.rs:302:4\n\
             error: could not compile `armada-helm` (lib) due to 1 previous error\n",
        )
        .unwrap();

        let why = "`armada manifest check` reached FAILED — armada:lint exited 101, \
                   in .armada/run/01M0/logs/armada.lint.log";
        let blocks = logs(dir.path(), why);
        assert_eq!(
            blocks.len(),
            1,
            "the log named in the sentence was not read"
        );
        assert!(
            blocks[0].contains("`word_to_verdict` is never used"),
            "the compiler's own sentence did not reach the Drone: {}",
            blocks[0]
        );
        assert!(
            blocks[0].contains("armada.lint.log"),
            "the block does not say which log it is: {}",
            blocks[0]
        );
    }

    /// **A log the sentence names but the disk does not have is one absent
    /// block, never a retry that fails to start.** The gate has already decided;
    /// this is the explanation, not the decision.
    #[test]
    fn a_log_that_cannot_be_read_costs_the_retry_nothing() {
        let dir = tempfile::tempdir().unwrap();
        let why = "`armada manifest check` reached FAILED — armada:lint exited 101, \
                   in .armada/run/gone/logs/armada.lint.log";
        assert!(logs(dir.path(), why).is_empty());
        // And a sentence naming no log at all is not an error either.
        assert!(logs(dir.path(), "`armada manifest check` reached FAILED").is_empty());
    }

    /// **A ceiling the caller set reaches the whole tree.**
    ///
    /// Measured 2026-08-16: `--budget max_cost=25.00` on a parent gave its
    /// planning sub-Job **$5.00**, because `plan.yml` declares 5 and [`carved`]
    /// took the smaller of the two. The child died on that $5 and the raise the
    /// caller asked for never reached the work it was asked for.
    #[test]
    fn a_budget_the_caller_set_reaches_a_sub_job() {
        let declared = budget(3, 5.00);
        let parent = budget(3, 30.00);
        let spent = Spend::default();

        // Nobody said anything, so the child's own declaration holds. A
        // default must never be raised in silence.
        assert_eq!(
            carved(declared, EndsAt::Branch, &parent, &spent, &[]).cost_usd,
            5.00
        );

        let told = carved(
            declared,
            EndsAt::Branch,
            &parent,
            &spent,
            &["max_cost".to_string()],
        );
        assert_eq!(told.cost_usd, 30.00, "the raise did not reach the sub-Job");
    }

    /// **A raise does not hand the whole budget to the planning step.**
    ///
    /// Measured 2026-08-16, an hour after the raise was built: a `plan` sub-Job
    /// under a parent spawned `--budget max_cost=20.00` spent $12.37 planning —
    /// 177 turns, an hour of API time — and stopped at its approval gate with no
    /// code written. The raise reached it exactly as intended and gave the cheap
    /// half of the work the expensive half's money.
    #[test]
    fn a_step_that_writes_a_document_gets_a_share_of_a_raise_rather_than_all_of_it() {
        let spent = Spend::default();
        let parent = budget(3, 30.00);

        let planning = carved(
            budget(3, 5.00),
            EndsAt::Human,
            &parent,
            &spent,
            &["max_cost".to_string()],
        );
        assert!(
            planning.cost_usd < 30.00,
            "the whole remainder went to a step that writes a document: {}",
            planning.cost_usd
        );
        assert!(
            (planning.cost_usd - 10.00).abs() < 1e-9,
            "expected a third of the parent's remainder, got {}",
            planning.cost_usd
        );

        // **The same parent, a step that writes code: the whole remainder.**
        // The share is a ceiling on planning, not a new way to starve a sub-Job
        // — which was the original defect and must not come back.
        let building = carved(
            budget(3, 5.00),
            EndsAt::Branch,
            &parent,
            &spent,
            &["max_cost".to_string()],
        );
        assert_eq!(building.cost_usd, 30.00);
    }

    /// **The share never drops a sub-Job below what its own workflow asks
    /// for.** A third of a nearly-spent parent is pennies, and a `plan.yml`
    /// declaring $5.00 means $5.00 is what planning is worth — the raise is
    /// being capped, not the declaration.
    #[test]
    fn the_planning_share_never_undercuts_the_workflows_own_declaration() {
        let spent = Spend {
            cost_usd: 21.00,
            ..Default::default()
        };
        let told = carved(
            budget(3, 5.00),
            EndsAt::Human,
            &budget(3, 30.00),
            &spent,
            &["max_cost".to_string()],
        );
        // A third of the $9.00 left is $3.00, which is less than plan.yml's
        // $5.00 — so the declaration wins, still bounded by the $9.00 remaining.
        assert_eq!(told.cost_usd, 5.00);
    }

    /// **The attempt ceiling is not carved, because it is not a shared pool.**
    ///
    /// It counts retries at *one step*, and a child runs a different workflow
    /// with different steps whose first attempt is its first. Carving it would
    /// hand a child two attempts because its parent had spent one on an
    /// unrelated gate — and a child that then failed twice would report having
    /// run out of something it never had.
    #[test]
    fn a_child_gets_its_own_attempts_rather_than_what_the_parent_has_left() {
        let spent = Spend::default();
        let told = carved(
            budget(3, 5.00),
            EndsAt::Branch,
            &budget(3, 30.00),
            &spent,
            &["max_cost".to_string()],
        );
        assert_eq!(told.attempts, 3, "the child's own step ceiling holds");
    }

    /// **A raise lifts the default; it does not mint budget.** The parent's
    /// remaining still bounds the child, so `016` §2's containment argument —
    /// *a child that could spend a fresh budget is how a parent gets exhausted
    /// by something it never counted* — is untouched.
    #[test]
    fn a_raised_sub_job_is_still_bounded_by_what_the_parent_has_left() {
        let spent = Spend {
            cost_usd: 28.00,
            ..Default::default()
        };
        let told = carved(
            budget(3, 5.00),
            EndsAt::Branch,
            &budget(3, 30.00),
            &spent,
            &["max_cost".to_string()],
        );
        assert!(
            told.cost_usd <= 2.00,
            "a child was handed more than the parent had left: {}",
            told.cost_usd
        );
    }
}
