//! The human renderer.
//!
//! **The renderer is the only thing that differs between human and agent
//! output** (`ARCHITECTURE.md` §1.6). Both read the same envelope; this one
//! flattens it into lines, and `--json` emits it whole.
//!
//! Nothing here decides anything. If a rule is being applied — which state is a
//! failure, which resources were skipped and why — it was decided upstream and
//! this file is reading a field.

use armada_core::envelope::{
    CheckData, CheckDryRun, CleanData, CleanDryRun, DispatchData, Envelope, InitData, InitDryRun,
    StatusData,
};
use armada_core::error::ArmadaError;
use armada_core::reap::ReapPlan;

use crate::verbs::Output;

/// Render for a terminal.
pub fn human(output: &Output) -> String {
    match output {
        Output::Init(envelope) => init(envelope),
        Output::InitDryRun(envelope) => init_dry(envelope),
        Output::Clean(envelope) => clean(envelope),
        Output::CleanDryRun(envelope) => clean_dry(envelope),
        Output::Status(envelope) => status(envelope),
        Output::Check(envelope) => check(envelope),
        Output::CheckDryRun(envelope) => check_dry(envelope),
        Output::Dispatch(envelope) => dispatch(envelope),
    }
}

fn init(envelope: &Envelope<InitData>) -> String {
    let data = &envelope.data;
    let mut out = String::new();
    out.push_str(&format!(
        "workspace {}  ports {}-{}\n",
        envelope
            .workspace
            .as_ref()
            .map(|w| w.to_string())
            .unwrap_or_default(),
        data.port_block.from,
        data.port_block.to
    ));
    for (name, port) in &data.ports {
        out.push_str(&format!("  {name:<16} {port}\n"));
    }
    for row in &data.results {
        out.push_str(&format!("  {:<16} {}\n", row.id, row.status));
        if let Some(error) = &row.error {
            out.push_str(&format!("    {}\n", error.message));
        }
    }
    out.push_str(&reaped(&data.reaped));
    if let Some(error) = &envelope.error {
        out.push_str(&error_lines(error));
    }
    out
}

/// `armada manifest check`.
///
/// One line per check, because that is the shape an agent scans and a human
/// reads: the id, the verdict, how long, and where the output went. The
/// **`waiting_on`** line is the one worth having — it names the workspace that
/// is in the way, which is the thing the caller cannot work out for itself.
fn check(envelope: &Envelope<CheckData>) -> String {
    let data = &envelope.data;
    let mut out = format!("run {}\n", data.run_id);
    for row in &data.results {
        out.push_str(&format!("  {:<24} {}", row.id, row.status));
        if let Some(ms) = row.duration_ms {
            out.push_str(&format!("  {ms}ms"));
        }
        out.push('\n');
        if let Some(reason) = &row.reason {
            out.push_str(&format!("    {reason}\n"));
        }
        if let Some(waiting) = &row.waiting_on {
            out.push_str(&format!(
                "    waiting on {}\n",
                serde_json::to_string(waiting).unwrap_or_default()
            ));
        }
        if let Some(error) = &row.error {
            out.push_str(&format!("    {}\n", error.message));
        }
        if let Some(log) = &row.log {
            out.push_str(&format!("    {log}\n"));
        }
    }
    for reaped in &data.reaped_runs {
        out.push_str(&format!("  reaped run {reaped}\n"));
    }
    if let Some(error) = &envelope.error {
        out.push_str(&error_lines(error));
    }
    out
}

fn check_dry(envelope: &Envelope<CheckDryRun>) -> String {
    let data = &envelope.data;
    let mut out = String::new();
    for line in &data.would_run {
        out.push_str(&format!("  would_run       {line}\n"));
    }
    for line in &data.would_skip {
        out.push_str(&format!("  would_skip      {line}\n"));
    }
    for line in &data.would_reap {
        out.push_str(&format!("  would_reap      run {line}\n"));
    }
    out
}

fn init_dry(envelope: &Envelope<InitDryRun>) -> String {
    let data = &envelope.data;
    let mut out = String::from("dry run — nothing was changed\n");
    if let Some(block) = data.would_claim {
        out.push_str(&format!(
            "  would_claim     ports {}-{}\n",
            block.from, block.to
        ));
    }
    for step in &data.would_run {
        out.push_str(&format!("  would_run       {step}\n"));
    }
    out.push_str(&reaped(&data.would_reap));
    out
}

fn clean(envelope: &Envelope<CleanData>) -> String {
    let data = &envelope.data;
    let mut out = String::new();
    for row in &data.results {
        let released = row.released.as_ref();
        out.push_str(&format!(
            "  {:<10} {}{}\n",
            row.id,
            row.status,
            released
                .map(|r| format!(
                    "  {} processes, {} containers, {} networks, {} volumes, {} images",
                    r.processes, r.containers, r.networks, r.volumes, r.images
                ))
                .unwrap_or_default()
        ));
    }
    for id in &data.skipped {
        out.push_str(&format!("  skipped    {id} — it holds a live lease\n"));
    }
    for external in &data.unreclaimed {
        // Reported, never executed. A stale `DROP DATABASE` is strictly more
        // dangerous than a stale `kill`.
        out.push_str(&format!(
            "  Armada did not reclaim, and will not: {}\n",
            external.command
        ));
    }
    out.push_str(&reaped(&data.reaped));
    if let Some(error) = &envelope.error {
        out.push_str(&error_lines(error));
    }
    out
}

fn clean_dry(envelope: &Envelope<CleanDryRun>) -> String {
    let data = &envelope.data;
    let mut out = String::from("dry run — nothing was changed\n");
    for line in &data.would_release {
        out.push_str(&format!("  would_release   {line}\n"));
    }
    for line in &data.would_remove {
        out.push_str(&format!("  would_remove    {line}\n"));
    }
    for line in &data.would_delete {
        out.push_str(&format!("  would_delete    {line}\n"));
    }
    for line in &data.would_report {
        out.push_str(&format!("  would_report    {line}\n"));
    }
    out
}

fn status(envelope: &Envelope<StatusData>) -> String {
    let data = &envelope.data;
    let mut out = format!("scope {}\n", data.scope);
    for row in &data.results {
        out.push_str(&format!(
            "  {}  {}  ports {}\n",
            row.id,
            row.path.as_deref().unwrap_or(""),
            row.port_block
                .map(|b| format!("{}-{}", b.from, b.to))
                .unwrap_or_default()
        ));
        for (name, report) in &row.ports {
            out.push_str(&format!(
                "      {name:<12} {} {}\n",
                report.port,
                serde_json::to_string(&report.state)
                    .unwrap_or_default()
                    .trim_matches('"')
            ));
        }
        for lease in &row.leases {
            out.push_str(&format!("      holds {lease}\n"));
        }
    }
    for external in &data.unreclaimed {
        out.push_str(&format!(
            "  workspace {}{} declared an external resource Armada did not reclaim:\n      {}\n",
            external.workspace,
            if external.workspace_exists {
                ""
            } else {
                " (directory deleted)"
            },
            external.command
        ));
    }
    out
}

fn dispatch(envelope: &Envelope<DispatchData>) -> String {
    match &envelope.error {
        Some(error) => error_lines(error),
        // The child wrote its own output; Armada adds nothing. Saying "exited 0"
        // after a command that already printed its result is noise, and saying
        // it on stdout would corrupt a pipeline the repo owns.
        None => String::new(),
    }
}

fn reaped(plan: &ReapPlan) -> String {
    let mut out = String::new();
    for id in &plan.workspaces {
        out.push_str(&format!("  reaped     workspace {id} (directory gone)\n"));
    }
    for target in &plan.resources {
        out.push_str(&format!(
            "  reaped     {} {} ({})\n",
            target.kind, target.reference, target.workspace
        ));
    }
    for lease in &plan.leases {
        out.push_str(&format!("  reaped     lease {lease} (heartbeat cold)\n"));
    }
    for report in &plan.reported {
        out.push_str(&format!(
            "  left alone {} {} ({}) — {}\n",
            report.kind,
            report.reference,
            report.workspace,
            serde_json::to_string(&report.reason)
                .unwrap_or_default()
                .trim_matches('"')
        ));
    }
    for skipped in &plan.skipped {
        out.push_str(&format!("  not swept  {skipped}\n"));
    }
    out
}

/// The error, in the shape PLAN.md §3.2.1 prints it.
pub fn error_lines(error: &ArmadaError) -> String {
    let mut out = format!("error: {}\n", error.message);
    out.push_str(&format!("  where: {}\n", error.r#where));
    out.push_str(&format!("  class: {}\n", error.class));
    if let Some(next) = &error.next_action {
        out.push_str(&format!("  next:  {next}\n"));
    }
    out
}

#[cfg(test)]
mod tests {
    use super::*;
    use armada_core::envelope::{PortReport, Released, ResultRow, Unreclaimed};
    use armada_core::error::{ErrClass, Status};
    use armada_core::id::WorkspaceId;
    use armada_core::ports::{PortBlock, PortState};
    use armada_core::reap::{LeftAlone, ReapTarget, Reported};
    use armada_core::registry::OwnedKind;
    use std::collections::BTreeMap;

    fn workspace() -> WorkspaceId {
        WorkspaceId::from_stored("a3f91c02")
    }

    fn block() -> PortBlock {
        PortBlock {
            from: 5460,
            to: 5469,
        }
    }

    fn failure() -> ArmadaError {
        ArmadaError {
            class: ErrClass::ToolFailed,
            r#where: "api".to_string(),
            message: "`npm ci` exited 1".to_string(),
            next_action: Some("run it by hand".to_string()),
        }
    }

    /// Every kind of decision a reap can report, so the block below renders all
    /// five of its lists rather than whichever one a scenario happened to fill.
    fn plan() -> ReapPlan {
        ReapPlan {
            workspaces: vec![WorkspaceId::from_stored("deadbeef")],
            resources: vec![ReapTarget {
                kind: OwnedKind::Container,
                reference: "Armada-api-1".to_string(),
                workspace: WorkspaceId::from_stored("deadbeef"),
            }],
            leases: vec!["run:a3f91c02".to_string()],
            reported: vec![Reported {
                kind: OwnedKind::Volume,
                reference: "Armada-data".to_string(),
                workspace: workspace(),
                reason: LeftAlone::WorkspaceLive,
            }],
            skipped: vec!["labelled resources: docker daemon unreachable".to_string()],
        }
    }

    #[test]
    fn init_prints_the_block_the_ports_and_every_row() {
        let mut row = ResultRow::new("api", Status::Failed);
        row.error = Some(failure());
        let envelope = Envelope::ok(
            "init",
            Some(workspace()),
            Status::Ready,
            InitData {
                port_block: block(),
                claimed_at: "2026-08-09T14:02:11Z".to_string(),
                ports: BTreeMap::from([("web".to_string(), 5460)]),
                reaped: ReapPlan::default(),
                results: vec![row],
            },
        );

        let text = human(&Output::Init(Box::new(envelope)));
        assert!(text.starts_with("workspace a3f91c02  ports 5460-5469\n"));
        assert!(text.contains("  web              5460\n"));
        assert!(text.contains("  api              FAILED\n"));
        // The row's own error is printed under it; the envelope's is not, and
        // there is none here.
        assert!(text.contains("    `npm ci` exited 1\n"));
        assert!(!text.contains("error: "));
    }

    /// The envelope's error is printed **as well as** the rows, because a verb
    /// that failed as a whole and a row that failed are different facts.
    #[test]
    fn a_failed_init_prints_the_envelopes_error_too() {
        let envelope = Envelope::failed(
            "init",
            Some(workspace()),
            failure(),
            InitData {
                port_block: block(),
                claimed_at: "2026-08-09T14:02:11Z".to_string(),
                ports: BTreeMap::new(),
                reaped: plan(),
                results: Vec::new(),
            },
        );

        let text = human(&Output::Init(Box::new(envelope)));
        assert!(text.contains("error: `npm ci` exited 1\n"));
        assert!(text.contains("  reaped     workspace deadbeef (directory gone)\n"));
        assert!(text.contains("  reaped     container Armada-api-1 (deadbeef)\n"));
        assert!(text.contains("  reaped     lease run:a3f91c02 (heartbeat cold)\n"));
        assert!(text.contains("  left alone volume Armada-data (a3f91c02) — workspace_live\n"));
        assert!(text.contains("  not swept  labelled resources: docker daemon unreachable\n"));
    }

    #[test]
    fn a_dry_run_says_so_before_anything_it_would_do() {
        let envelope = Envelope::ok(
            "init",
            Some(workspace()),
            Status::Ready,
            InitDryRun {
                would_claim: Some(block()),
                would_run: vec!["api: npm ci".to_string()],
                would_reap: ReapPlan::default(),
            },
        );

        let text = human(&Output::InitDryRun(Box::new(envelope)));
        assert!(text.starts_with("dry run — nothing was changed\n"));
        assert!(text.contains("  would_claim     ports 5460-5469\n"));
        assert!(text.contains("  would_run       api: npm ci\n"));
    }

    #[test]
    fn clean_prints_what_it_released_what_it_skipped_and_what_it_will_never_reclaim() {
        let mut row = ResultRow::new("a3f91c02", Status::Clean);
        row.released = Some(Released {
            processes: 1,
            containers: 2,
            networks: 3,
            volumes: 4,
            images: 5,
            port_block: true,
            files: 6,
        });
        let envelope = Envelope::ok(
            "clean",
            Some(workspace()),
            Status::Clean,
            CleanData {
                reaped: ReapPlan::default(),
                results: vec![row],
                unreclaimed: vec![Unreclaimed {
                    workspace: workspace(),
                    command: "psql -c 'DROP DATABASE app_a3f91c02'".to_string(),
                    workspace_exists: true,
                }],
                skipped: vec!["b7c20d11".to_string()],
            },
        );

        let text = human(&Output::Clean(Box::new(envelope)));
        assert!(text.contains(
            "  a3f91c02   CLEAN  1 processes, 2 containers, 3 networks, 4 volumes, 5 images\n"
        ));
        assert!(text.contains("  skipped    b7c20d11 — it holds a live lease\n"));
        assert!(text.contains(
            "  Armada did not reclaim, and will not: psql -c 'DROP DATABASE app_a3f91c02'\n"
        ));
    }

    /// A row Armada never got as far as releasing anything for prints its state
    /// and nothing else — no empty tally.
    #[test]
    fn a_clean_row_with_nothing_released_prints_no_counts() {
        let envelope = Envelope::failed(
            "clean",
            Some(workspace()),
            failure(),
            CleanData {
                reaped: ReapPlan::default(),
                results: vec![ResultRow::new("a3f91c02", Status::Failed)],
                unreclaimed: Vec::new(),
                skipped: Vec::new(),
            },
        );

        let text = human(&Output::Clean(Box::new(envelope)));
        assert!(text.contains("  a3f91c02   FAILED\n"));
        assert!(!text.contains("processes"));
        assert!(text.contains("error: `npm ci` exited 1\n"));
    }

    #[test]
    fn a_clean_preview_labels_all_four_of_its_lists() {
        let envelope = Envelope::ok(
            "clean",
            Some(workspace()),
            Status::Clean,
            CleanDryRun {
                would_release: vec!["ports 5460-5469 (a3f91c02)".to_string()],
                would_remove: vec!["container Armada-api-1".to_string()],
                would_delete: vec!["node_modules".to_string()],
                would_report: vec!["psql -c 'DROP DATABASE app_a3f91c02'".to_string()],
            },
        );

        let text = human(&Output::CleanDryRun(Box::new(envelope)));
        assert!(text.starts_with("dry run — nothing was changed\n"));
        assert!(text.contains("  would_release   ports 5460-5469 (a3f91c02)\n"));
        assert!(text.contains("  would_remove    container Armada-api-1\n"));
        assert!(text.contains("  would_delete    node_modules\n"));
        assert!(text.contains("  would_report    psql -c 'DROP DATABASE app_a3f91c02'\n"));
    }

    /// **The human spelling is the JSON spelling** (`ARCHITECTURE.md` §1.6): a
    /// port state reads `CONFLICT` in the terminal because that is what the
    /// envelope calls it, and a reader who learned one knows the other.
    #[test]
    fn status_spells_a_port_state_the_way_the_envelope_does() {
        let mut row = ResultRow::new("a3f91c02", Status::Ok);
        row.path = Some("/scratch/repo".to_string());
        row.port_block = Some(block());
        row.ports = BTreeMap::from([(
            "web".to_string(),
            PortReport {
                port: 5460,
                state: PortState::Conflict,
            },
        )]);
        row.leases = vec!["run:a3f91c02".to_string()];
        let envelope = Envelope::ok(
            "status",
            Some(workspace()),
            Status::Ok,
            StatusData {
                scope: "workspace".to_string(),
                results: vec![row],
                unreclaimed: vec![Unreclaimed {
                    workspace: WorkspaceId::from_stored("deadbeef"),
                    command: "psql -c 'DROP DATABASE app_deadbeef'".to_string(),
                    workspace_exists: false,
                }],
            },
        );

        let text = human(&Output::Status(Box::new(envelope)));
        assert!(text.starts_with("scope workspace\n"));
        assert!(text.contains("  a3f91c02  /scratch/repo  ports 5460-5469\n"));
        assert!(
            text.contains("      web          5460 CONFLICT\n"),
            "the state is spelled as the JSON spells it, unquoted: {text}"
        );
        assert!(text.contains("      holds run:a3f91c02\n"));
        assert!(
            text.contains("  workspace deadbeef (directory deleted) declared an external"),
            "a deleted directory is said so rather than left to be inferred: {text}"
        );
    }

    /// A dispatched command that ran prints **nothing**: the child already
    /// wrote its own output, and "exited 0" on top of it is noise — on stdout,
    /// it is corruption of a pipeline the repo owns.
    #[test]
    fn a_dispatched_command_that_ran_adds_nothing_of_chars_own() {
        let ran = Envelope::ok(
            "commands",
            Some(workspace()),
            Status::Ok,
            DispatchData {
                command: "echoer".to_string(),
                dispatched: true,
                child_exit: Some(0),
                argv: vec!["echo".to_string()],
            },
        );
        assert_eq!(human(&Output::Dispatch(Box::new(ran))), "");

        // A command that never ran is Armada's failure, and Armada says so.
        let refused = Envelope::failed(
            "commands",
            Some(workspace()),
            failure(),
            DispatchData {
                command: "missing".to_string(),
                dispatched: false,
                child_exit: None,
                argv: Vec::new(),
            },
        );
        assert_eq!(
            human(&Output::Dispatch(Box::new(refused))),
            "error: `npm ci` exited 1\n  where: api\n  class: tool_failed\n  next:  run it by hand\n"
        );
    }

    /// The error class is spelled exactly as the envelope spells it, and the
    /// remedy line is omitted rather than printed empty when there is none.
    #[test]
    fn an_error_without_a_remedy_prints_three_lines_and_not_four() {
        let text = error_lines(&ArmadaError {
            class: ErrClass::BadInvocation,
            r#where: "--force-rebuild".to_string(),
            message: "not built yet".to_string(),
            next_action: None,
        });
        assert_eq!(
            text,
            "error: not built yet\n  where: --force-rebuild\n  class: bad_invocation\n"
        );
    }
}
