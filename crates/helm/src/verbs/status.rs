//! `char status` — what's running, what's mine, what's stale.
//!
//! **A read verb.** It takes no lease, it mutates nothing, and **its exit code
//! describes the query, not the thing queried**: `0` for answered, whatever the
//! workspaces it reports are doing. A gate uses `char check --wait`; it never
//! reads a query's exit code as a verdict.
//!
//! It also asks **no daemon**. Everything it reports comes from `char.db` and a
//! port probe, which is what makes it cheap enough to poll — and it is enough
//! for §6.1's own `status --all` example, since a declared `release:` command is
//! a recorded row rather than a labelled resource. Reaping is `init`'s and
//! `clean`'s job.

use armada_core::ctx::{Clock, Fetch, Run};
use armada_core::envelope::{Envelope, PortReport, ResultRow, StatusData, Unreclaimed};
use armada_core::error::{CharError, Status};
use armada_core::lease::is_cold;
use armada_core::ports::PortState;
use armada_core::reap::PathStat;
use armada_core::registry::{OwnedKind, WorkspaceRow};
use armada_core::scope::{self, Lens};
use armada_manifest::{fs, net};

use crate::app::App;
use crate::args::Common;
use crate::verbs::{load_foreign_config, Output};

/// Run it.
pub fn run<R: Run, C: Clock, F: Fetch>(
    app: &mut App<R, C, F>,
    common: Common,
) -> Result<Output, CharError> {
    let me = app.ctx.workspace.as_ref().map(|w| w.id.clone());
    let project = app.ctx.workspace.as_ref().and_then(|w| w.project.clone());

    let rows = app.db.workspaces()?;
    let selected: Vec<WorkspaceRow> = match (&me, common.lens) {
        // `char status --all` is one of the two invocations that run without a
        // `char.yml` at all: asking about *this workspace* requires one, asking
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
    let mut results = Vec::new();
    let mut unreclaimed = Vec::new();

    for row in &selected {
        let exists = fs::stat(&row.path) != PathStat::Missing;
        let mut result = ResultRow::new(row.id.to_string(), Status::Ok);
        result.path = Some(row.path.display().to_string());
        result.project = row.project.clone();
        result.port_block = Some(row.ports);

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

        let owned = app.db.owned(Some(&row.id))?;
        // Whether char started anything here is what tells `LISTENING` from
        // `CONFLICT`. A bound port with nothing owned is bound by a process
        // char did not start — which is the only way that reaches a caller
        // instead of surfacing later as a mysterious bind failure.
        let char_started_something = owned
            .iter()
            .any(|o| matches!(o.kind, OwnedKind::Pgid | OwnedKind::Container));

        if exists {
            if let Some(config) = load_foreign_config(&row.path, &app.machine) {
                if let Ok(ports) = armada_core::ports::assign_ports(&config, row.ports, "char.yml")
                {
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
                                    state: match (net::port_is_taken(port), char_started_something)
                                    {
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
                    command: owned.reference,
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
