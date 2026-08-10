//! The human renderer.
//!
//! **The renderer is the only thing that differs between human and agent
//! output** (`ARCHITECTURE.md` §1.6). Both read the same envelope; this one
//! flattens it into lines, and `--json` emits it whole.
//!
//! Nothing here decides anything. If a rule is being applied — which state is a
//! failure, which resources were skipped and why — it was decided upstream and
//! this file is reading a field.

use charkit_core::envelope::{
    CleanData, CleanDryRun, DispatchData, Envelope, InitData, InitDryRun, StatusData,
};
use charkit_core::error::CharError;
use charkit_core::reap::ReapPlan;

use crate::verbs::Output;

/// Render for a terminal.
pub fn human(output: &Output) -> String {
    match output {
        Output::Init(envelope) => init(envelope),
        Output::InitDryRun(envelope) => init_dry(envelope),
        Output::Clean(envelope) => clean(envelope),
        Output::CleanDryRun(envelope) => clean_dry(envelope),
        Output::Status(envelope) => status(envelope),
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
            "  char did not reclaim, and will not: {}\n",
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
            "  workspace {}{} declared an external resource char did not reclaim:\n      {}\n",
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
        // The child wrote its own output; char adds nothing. Saying "exited 0"
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
pub fn error_lines(error: &CharError) -> String {
    let mut out = format!("error: {}\n", error.message);
    out.push_str(&format!("  where: {}\n", error.r#where));
    out.push_str(&format!("  class: {}\n", error.class));
    if let Some(next) = &error.next_action {
        out.push_str(&format!("  next:  {next}\n"));
    }
    out
}
