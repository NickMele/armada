//! `armada manifest status` — what's running, what's mine, what's stale.
//!
//! **A read verb.** It takes no lease, it mutates nothing, and **its exit code
//! describes the query, not the thing queried**: `0` for answered, whatever the
//! workspaces it reports are doing. A gate uses `armada manifest check --wait`; it never
//! reads a query's exit code as a verdict.
//!
//! It also asks **no daemon**. Everything it reports comes from `manifest.db`, a
//! port probe, the run directories under `.armada/run/` and a `ps` for the
//! process groups the store already names — which is what keeps it cheap enough
//! to poll, and it is enough for §6.1's own `status --all` example, since a
//! declared `release:` command is a recorded row rather than a labelled
//! resource. Reaping is `init`'s and `clean`'s job.
//!
//! # What it enumerates, and why that is not the registry
//!
//! **`workspaces` is not the set of workspaces that have state.** It records a
//! *port claim*, written by exactly one code path — `Db::claim_block` under
//! `armada manifest init`. `owned` records a *resource that exists*, written by
//! `up`, by `check --detach` and by `fleet spawn`, all of which key on
//! `WorkspaceId::derive` and need no claim. Measured on this project's own
//! checkout: `workspaces` held **0** rows while `owned` held **6**, every one a
//! `pgid` from `check --detach` and four of them from a previous boot. This verb
//! enumerated the registry, asked each of its zero rows what it owned, and
//! reported `OK` — the one verb whose job is finding stale resources, blind to
//! six leaked process groups. So it enumerates `Db::known_workspaces`, the union,
//! and [`docs/reserved/023`] records why the alternative fix was refused.
//!
//! # What it reports about a run
//!
//! **Only runs that have not reached a verdict, and only from this boot.** A run
//! that decided is history and `check --status <id>` is the verb that reads it
//! back; a run from a previous boot cannot be executing, and whatever it leaked
//! is already named in `stale[]` as the `pgid` that `clean` will drop. What is
//! left is the question the verb's own first line promises and could not
//! answer: is anything running here right now.
//!
//! [`docs/reserved/023`]: ../../../../docs/reserved/023-status-shows-what-is-running.md

use armada_core::ctx::{Clock, Fetch, Run};
use armada_core::envelope::{Envelope, PortReport, ResultRow, RunView, StatusData, Unreclaimed};
use armada_core::error::{ArmadaError, Status};
use armada_core::id::WorkspaceId;
use armada_core::lease::is_cold;
use armada_core::ports::PortState;
use armada_core::reap::{pgid_is_ours, PathStat};
use armada_core::registry::{OwnedKind, OwnedRow, WorkspaceRow};
use armada_core::scope::{self, Lens};
use armada_core::workspace::Workspace;
use armada_manifest::{fs, machine, net, runs};
use std::collections::BTreeMap;
use std::path::Path;

use crate::app::App;
use crate::args::Common;
use crate::verbs::{load_foreign_config, Output};

/// Run it.
pub fn run<R: Run, C: Clock, F: Fetch>(
    app: &mut App<R, C, F>,
    common: Common,
) -> Result<Output, ArmadaError> {
    let me = app.ctx.workspace.as_ref().map(|w| w.id.clone());
    let project = app.ctx.workspace.as_ref().and_then(|w| w.project.clone());

    let rows = enumerate(
        &app.db.workspaces()?,
        &app.db.known_workspaces()?,
        app.ctx.workspace.as_ref(),
    );
    let selected: Vec<WorkspaceRow> = match (&me, common.lens) {
        // `armada manifest status --all` is one of the two invocations that run without a
        // `armada.yml` at all: asking about *this workspace* requires one, asking
        // about *the machine* does not.
        (None, Lens::All) => rows.clone(),
        (None, _) => return Err(app.ctx.workspace().unwrap_err()),
        (Some(me), lens) => scope::select(&rows, lens, me, project.as_ref())
            .into_iter()
            .cloned()
            .collect(),
    };

    let now = app.ctx.now.mono();
    let leases = app.db.leases()?;
    // **One query, grouped here, where it used to be one query per row.** The
    // old shape asked `owned(Some(id))` inside the loop, which is the same N+1
    // that made the registry look authoritative in the first place: a workspace
    // the loop never reached was a workspace whose resources were never read.
    let mut by_workspace: BTreeMap<WorkspaceId, Vec<OwnedRow>> = BTreeMap::new();
    for row in app.db.owned(None)? {
        by_workspace
            .entry(row.workspace.clone())
            .or_default()
            .push(row);
    }
    let no_rows: Vec<OwnedRow> = Vec::new();

    let mut results = Vec::new();
    let mut unreclaimed = Vec::new();

    for row in &selected {
        let root = known_root(row);
        let exists = root.is_some_and(|root| fs::stat(root) != PathStat::Missing);
        let mut result = ResultRow::new(row.id.to_string(), Status::Ok);
        result.path = root.map(|root| root.display().to_string());
        result.project = row.project.clone();
        result.port_block = row.ports;

        // A live lease is the only thing that distinguishes "running" from
        // "claimed and idle", and a cold one is a holder that died — which is
        // why the row names it either way rather than filtering it out.
        result.leases = leases
            .iter()
            .filter(|lease| lease.workspace.as_ref() == Some(&row.id))
            .map(|lease| {
                let cold = is_cold(lease.heartbeat_mono, now, lease.boot_id == app.boot_id);
                format!(
                    "{}:{}{}",
                    lease.kind,
                    lease.key,
                    if cold { " (cold)" } else { "" }
                )
            })
            .collect();

        let owned = by_workspace.get(&row.id).unwrap_or(&no_rows);

        // **The ids, not a count** (PLAN.md §3.1): what `clean` will remove, and
        // what to go and look at by hand. Sorted, because the store returns
        // insertion order and an envelope that differs between two identical
        // runs is one the golden snapshots cannot hold still.
        //
        // A `Release` row is left out: it is a command rather than a resource,
        // Armada never executes it, and `unreclaimed` below already carries it
        // with the fact that matters — whether its workspace still exists.
        result.owns = owned
            .iter()
            .filter(|owned| owned.kind != OwnedKind::Release)
            .map(|owned| format!("{}:{}", owned.kind, owned.reference))
            .collect();
        result.owns.sort();

        // **The third of the three states the doc has always promised.** `owns`
        // said what was recorded and nothing said what was still there, so four
        // dead process groups and two live ones printed identically — measured,
        // on this repository, at six rows for one workspace. The rule is
        // `pgid_is_ours`, the same one `clean` kills on, so nothing lands here
        // that `clean` would decline to reclaim.
        result.stale = owned
            .iter()
            .filter(|owned| !pgid_is_live(app, owned))
            .map(|owned| format!("{}:{}", owned.kind, owned.reference))
            .collect();
        result.stale.sort();

        if let Some(root) = root.filter(|_| exists) {
            result.runs = undecided_runs(app, root);
            if let Some(config) = load_foreign_config(root, &app.machine) {
                if let Ok(ports) = row.ports.map_or_else(
                    || Ok(std::collections::BTreeMap::new()),
                    |block| armada_core::ports::assign_ports(&config, block, "armada.yml"),
                ) {
                    // Whether Armada started anything here is what tells
                    // `LISTENING` from `CONFLICT`. A bound port with nothing
                    // owned is bound by a process Armada did not start — which
                    // is the only way that reaches a caller instead of surfacing
                    // later as a mysterious bind failure.
                    //
                    // **A group Armada has just proved dead does not count.**
                    // Answering `LISTENING` off a `pgid` row from a previous
                    // boot would attribute a port to a process that no longer
                    // exists, which is exactly the mystery bind failure this
                    // distinction was written to prevent.
                    let armada_started_something = owned.iter().any(|o| {
                        matches!(o.kind, OwnedKind::Pgid | OwnedKind::Container)
                            && pgid_is_live(app, o)
                    });
                    result.ports = ports
                        .into_iter()
                        .map(|(name, port)| {
                            (
                                name,
                                PortReport {
                                    port,
                                    // Probed at report time, never remembered:
                                    // a claim recorded at `init` says nothing
                                    // about what is bound days later.
                                    state: match (
                                        net::port_is_taken(port),
                                        armada_started_something,
                                    ) {
                                        (false, _) => PortState::Reserved,
                                        (true, true) => PortState::Listening,
                                        (true, false) => PortState::Conflict,
                                    },
                                },
                            )
                        })
                        .collect();
                }
            }
        }

        for owned in owned {
            if owned.kind == OwnedKind::Release {
                unreclaimed.push(Unreclaimed {
                    workspace: row.id.clone(),
                    command: owned.reference.clone(),
                    workspace_exists: exists,
                });
            }
        }

        results.push(result);
    }

    Ok(Output::Status(Box::new(Envelope::ok(
        "status",
        me,
        Status::Ok,
        StatusData {
            scope: match common.lens {
                Lens::Workspace => "workspace",
                Lens::Project => "project",
                Lens::All => "all",
            }
            .to_string(),
            results,
            unreclaimed,
        },
    ))))
}

/// Every workspace `status` may report on: the registry, plus every id the store
/// holds any other state for, plus the one this invocation is standing in.
///
/// **The synthesised rows carry an empty `path`, and that is the honest answer
/// rather than a sentinel.** A workspace id is `sha1(realpath)` and the hash is
/// one-way (`core/src/id.rs`), so an `owned` row against an unregistered
/// workspace names a directory nothing can recover — the store never wrote it
/// down, because writing it down is what a `workspaces` row *is*. The single
/// exception is the workspace this process is inside, whose root it already has
/// in `Ctx`: reporting that one as pathless would hide the run directory of the
/// repository the caller is standing in, which is the whole case the fix exists
/// for.
///
/// **The invoking workspace always gets a row, even with nothing to say.**
/// `owns  resources  —` means Armada looked and found nothing; no row at all
/// means nothing whatsoever, and a caller cannot tell those apart — the same
/// argument the renderer already makes for keeping an empty resource row.
fn enumerate(
    registered: &[WorkspaceRow],
    known: &[WorkspaceId],
    here: Option<&Workspace>,
) -> Vec<WorkspaceRow> {
    let mut rows = registered.to_vec();
    let missing = |rows: &[WorkspaceRow], id: &WorkspaceId| !rows.iter().any(|row| &row.id == id);

    for id in known {
        if missing(&rows, id) {
            rows.push(synthesise(id.clone(), here));
        }
    }
    if let Some(here) = here {
        if missing(&rows, &here.id) {
            rows.push(synthesise(here.id.clone(), Some(here)));
        }
    }
    // Sorted, so the payload of two identical invocations is byte-identical
    // whatever order the union and the registry happened to return.
    rows.sort_by(|a, b| a.id.as_str().cmp(b.id.as_str()));
    rows
}

/// A row for a workspace the registry never heard of.
fn synthesise(id: WorkspaceId, here: Option<&Workspace>) -> WorkspaceRow {
    let here = here.filter(|here| here.id == id);
    WorkspaceRow {
        id,
        path: here.map(|here| here.root.clone()).unwrap_or_default(),
        project: here.and_then(|here| here.project.clone()),
        // **No block, because there is no claim.** A workspace with no registry
        // row has reserved no ports by construction, so reporting a span here
        // would invent a reservation that nothing holds.
        ports: None,
        claimed_at: String::new(),
    }
}

/// The workspace's directory, when anything can name it.
fn known_root(row: &WorkspaceRow) -> Option<&Path> {
    (!row.path.as_os_str().is_empty()).then_some(row.path.as_path())
}

/// Whether an `owned` row still points at something.
///
/// **Only a `pgid` can be answered, and `true` is the answer for everything
/// else.** A container, a network, a volume or an image would take a daemon
/// call, and `status` asks no daemon — that is what makes it cheap enough to
/// poll (`docs/commands/manifest/status.md`). Reporting an unprobed container as
/// stale would be a guess dressed as a measurement, and the field's contract is
/// proof.
///
/// **A boot id that is not this boot short-circuits the probe.**
/// [`pgid_is_ours`] requires `boot == current_boot`, so a row from a previous
/// boot is already decided and the `ps` would only confirm it — four of the six
/// rows measured on this machine, four spawns saved on every poll.
fn pgid_is_live<R: Run, C: Clock, F: Fetch>(app: &App<R, C, F>, row: &OwnedRow) -> bool {
    if row.kind != OwnedKind::Pgid {
        return true;
    }
    if row.boot_id.as_deref() != Some(app.boot_id.as_str()) {
        return false;
    }
    // A pgid of zero or less is not a pgid; `stop_owned_processes` drops such a
    // row rather than signalling with it, so nothing can be alive behind it.
    let Ok(pgid) = row.reference.parse::<i32>() else {
        return false;
    };
    if pgid <= 0 {
        return false;
    }
    let observed = machine::process_start_at(&app.ctx.run, &app.cwd(), pgid);
    pgid_is_ours(
        row.boot_id.as_deref(),
        row.pid_started_at.as_deref(),
        &app.boot_id,
        observed.as_deref(),
    )
}

/// The detached runs in this workspace that have not reached a verdict, newest
/// first.
///
/// **Every filesystem failure here answers "no runs" rather than failing the
/// verb**, and that is the exit-code contract rather than laziness: `status`
/// exits `0` whenever *the store* could be read, and `.armada/run/` is not the
/// store. A run directory a root-owned CI job wrote, or one being reaped by a
/// concurrent `check` as this reads it, must not turn the machine-wide status
/// query into an `environment` failure. `check --status` is the verb that
/// reports a specific run's directory as broken, and it says so precisely.
fn undecided_runs<R: Run, C: Clock, F: Fetch>(app: &App<R, C, F>, root: &Path) -> Vec<RunView> {
    let mut found = Vec::new();
    for run in runs::present(root).unwrap_or_default().into_iter().rev() {
        // **`detached.json` first, and it is the cheap half of the pair.** It is
        // five fields; `state.json` carries the whole journal, so reading it for
        // every run a workspace has kept would put a retention-count-sized parse
        // on a verb meant to be polled. A run with no detach record was a
        // foreground run whose caller was watching it — there is no group to ask
        // about and nothing to report.
        let Ok(Some(detached)) = runs::read_detached(root, &run) else {
            continue;
        };
        // A run from a previous boot cannot be executing, and its group is
        // already named in `stale[]` — see this module's header.
        if detached.boot_id != app.boot_id {
            continue;
        }
        let Ok(Some(record)) = runs::read_record(root, &run) else {
            continue;
        };
        if record.state.verdict().is_some() {
            continue;
        }
        // **Asked from a different process than the one that spawned it**, which
        // every `status` invocation is by construction — a start-time probe
        // answers for a zombie the spawning process has not reaped
        // (`docs/traps.md`).
        let observed = machine::process_start_at(&app.ctx.run, root, detached.pgid);
        found.push(RunView {
            run_id: run.to_string(),
            status: match detached.is_ours(&app.boot_id, observed.as_deref()) {
                true => Status::Running,
                // The record holds no verdict and the group is gone: it stopped
                // without deciding. `check --status` reaches the same word by
                // the same two questions, in the same order.
                false => Status::Dead,
            },
            pgid: detached.pgid,
            log: detached.log,
            started_at: record.started_at,
        });
    }
    found
}
