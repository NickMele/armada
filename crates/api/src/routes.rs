//! The route table, by hand, and the state every handler is given.
//!
//! # Hand-written, and that is the accepted cost
//!
//! A typo in a path here is a runtime 404, not a compile error. That trade was
//! made deliberately against carrying a codegen toolchain, and it is why
//! [`SERVED`] exists: every route is declared once as data beside the operation
//! name `crates/ipc/operations.toml` keys it under, so a test can walk the
//! table and prove each row is actually routed. **A route that exists in the
//! inventory and nowhere in the router is exactly the failure this shape is
//! paying for.**
//!
//! # One listener
//!
//! The WebSocket upgrade is an extractor in this same `Router`. There is no
//! second port: queries and commands answer over HTTP because they are
//! request-response, and only the unsolicited push needs the socket.
//!
//! # The handlers are next door, and the table is one module down
//!
//! `crate::queries` reads, `crate::commands` writes, `crate::sockets` upgrades.
//! [`SERVED`] is [`served`](mod@served)'s, having moved there when this file
//! crossed the 900-line rule; the gate reads the two halves as one text.

use axum::routing::{get, post};
use axum::Router;

use crate::amending::edit_manifest;
use crate::attention::{get_activity_feed, list_alerts, list_job_board, list_reviews};
use crate::commands::{
    add_task, answer_command, answer_judge, answer_question, approve_dispatch, approve_review,
    ask_person_to_approve, delete_branch, dismiss_finding, drop_task, examine_job,
    file_finding_issue, file_report, forget_job, kill_drone, kill_job, merge_pull_request,
    override_verdict, propose_from_request, propose_job, queue_after_finding, raise_cost_cap,
    raise_turn_cap, reclaim_worktree, redirect_drone, redispatch_job, reject_job, request_changes,
    rerun_checks, rerun_gate, restart_step, set_when_blocked, set_when_refused, show_again,
    stop_proposal, take_up_remarks,
};
use crate::conversing::{
    answer_helm_call, ask_helm, ask_the_person, list_helm_calls, observe_helm, start_helm_fresh,
};
use crate::daemon::Daemon;
use crate::editing::{get_manifest_file, save_manifest_file};
use crate::fleetwide::{
    get_drone, get_events_since, get_health, get_manifest, get_manifest_spend, get_usage,
    list_drones,
};
use crate::limiting::{get_limits, save_limits};
use crate::preferring::{get_preferences, save_preferences};
use crate::queries::{
    explain_command, get_call, get_capacity, get_check_output, get_diff, get_evidence, get_frame,
    get_job, get_job_events, get_job_log, get_job_resources, get_manifest_drift,
    get_manifest_reading, get_remarks, list_jobs, list_manifests, list_models, list_reports,
    list_workflows, list_worktrees, search_files,
};
use crate::rehearsing::{
    get_checkout_run_diff, get_checkout_run_output, get_checkout_run_sheet, get_run_output,
    get_run_sheet, list_checkout_runs, list_runs, observe_checkout_run, observe_run,
    start_checkout_run, start_checkout_verify, start_run, stop_checkout_run, stop_run,
    undo_checkout_run, undo_run,
};
use crate::repositories::{add_repository, clone_repository, list_repositories};
use crate::repository_allow::{get_repository_allowed_commands, remove_repository_allowed_command};
use crate::served::Served;
use crate::servers::{list_servers, observe_server, start_server, stop_server};
use crate::sockets::{events, job_log, observe_check_output, observe_job};
use crate::studios::{
    add_studio_node, ask_scout, capture_studio_note, create_studio, decide_studio_edge,
    defer_on_studio, delete_studio, dispatch_studio_draft, edit_studio_draft, edit_studio_link,
    get_studio, get_studio_frame, group_studio_nodes, list_studios, move_studio_node,
    propose_studio_edge, read_in_link, remove_studio_node, remove_studio_nodes, rename_studio,
    settle_contradiction, start_scout, start_studio_run, stop_scout, write_up_studio_node,
};

/// The inventory this router is compared against, row by row.
mod served;

pub use served::{Route, SERVED};

/// The one listener: the HTTP surface, the Drone's endpoint and the agent's
/// door, on one port.
///
/// **The surface is built twice on purpose.** The agent's door makes every
/// tool call against it rather than against the daemon, and a router that held
/// itself is not a value — so it is handed its own copy of the same table.
pub fn router<D: Daemon>(served: Served<D>) -> Router {
    surface(served.clone()).merge(crate::door::mounted::<D>(served.clone(), surface(served)))
}

/// The HTTP surface and the Drone's endpoint: every route but the door.
fn surface<D: Daemon>(served: Served<D>) -> Router {
    Router::new()
        .route("/jobs", get(list_jobs::<D>).post(propose_job::<D>))
        .route("/jobs/from_request", post(propose_from_request::<D>))
        .route("/proposals/stop", post(stop_proposal::<D>))
        .route("/workflows", get(list_workflows::<D>))
        .route(
            "/workflows/left_out",
            get(crate::repositories::list_left_out_workflows::<D>),
        )
        .route("/manifests", get(list_manifests::<D>))
        .route("/repositories", get(list_repositories::<D>))
        .route("/repositories/add", post(add_repository::<D>))
        .route("/repositories/clone", post(clone_repository::<D>))
        .route("/models", get(list_models::<D>))
        .route("/capacity", get(get_capacity::<D>))
        .route("/limits", get(get_limits::<D>))
        .route("/limits/save", post(save_limits::<D>))
        .route("/preferences", get(get_preferences::<D>))
        .route("/preferences/save", post(save_preferences::<D>))
        .route("/health", get(get_health::<D>))
        .route("/usage", get(get_usage::<D>))
        .route("/alerts", get(list_alerts::<D>))
        .route("/drones", get(list_drones::<D>))
        .route("/drones/:drone_id", get(get_drone::<D>))
        .route("/manifests/:manifest_id", get(get_manifest::<D>))
        .route("/events/since", get(get_events_since::<D>))
        .route("/jobs/board", get(list_job_board::<D>))
        .route("/jobs/reviews", get(list_reviews::<D>))
        .route("/jobs/activity", get(get_activity_feed::<D>))
        .route("/manifest/reading", get(get_manifest_reading::<D>))
        .route("/manifest/drift", get(get_manifest_drift::<D>))
        .route("/manifest/spend", get(get_manifest_spend::<D>))
        .route(
            "/repository/scan",
            get(crate::queries::get_repository_scan::<D>),
        )
        .route(
            "/repository/proposals",
            get(crate::manifest_proposals::get_manifest_proposals::<D>),
        )
        .route(
            "/repository/edit_proposal",
            post(crate::manifest_proposals::edit_manifest_proposal::<D>),
        )
        .route(
            "/repository/write_proposal",
            post(crate::manifest_proposals::write_manifest_proposal::<D>),
        )
        .route("/manifest/file", get(get_manifest_file::<D>))
        .route("/manifest/save_file", post(save_manifest_file::<D>))
        .route("/manifest/edit", post(edit_manifest::<D>))
        .route("/manifest/files", get(search_files::<D>))
        .route(
            "/manifest/allowed_commands",
            get(get_repository_allowed_commands::<D>),
        )
        .route(
            "/manifest/allowed_commands/remove",
            post(remove_repository_allowed_command::<D>),
        )
        .route("/jobs/:job_id", get(get_job::<D>))
        .route("/jobs/:job_id/events", get(get_job_events::<D>))
        .route("/jobs/:job_id/evidence", get(get_evidence::<D>))
        .route("/jobs/:job_id/diff", get(get_diff::<D>))
        .route("/jobs/:job_id/remarks", get(get_remarks::<D>))
        .route("/jobs/:job_id/resources", get(get_job_resources::<D>))
        .route("/jobs/:job_id/examine", post(examine_job::<D>))
        .route(
            "/jobs/:job_id/ask_person_to_approve",
            post(ask_person_to_approve::<D>),
        )
        .route("/jobs/:job_id/calls/:call_id", get(get_call::<D>))
        .route(
            "/jobs/:job_id/calls/:call_id/explain",
            get(explain_command::<D>),
        )
        .route(
            "/jobs/:job_id/checks/:kept/output",
            get(get_check_output::<D>),
        )
        .route("/jobs/:job_id/frames/:run/:name", get(get_frame::<D>))
        .route("/jobs/:job_id/approve_review", post(approve_review::<D>))
        .route("/jobs/:job_id/merge", post(merge_pull_request::<D>))
        .route(
            "/jobs/:job_id/rerun_failed_checks",
            post(crate::commands::rerun_failed_checks::<D>),
        )
        .route(
            "/jobs/:job_id/investigate_failed_checks",
            post(crate::commands::investigate_failed_checks::<D>),
        )
        .route("/jobs/:job_id/request_changes", post(request_changes::<D>))
        .route("/jobs/:job_id/reject", post(reject_job::<D>))
        .route("/jobs/:job_id/take_up_remarks", post(take_up_remarks::<D>))
        .route("/jobs/:job_id/dismiss_finding", post(dismiss_finding::<D>))
        .route(
            "/jobs/:job_id/queue_after_finding",
            post(queue_after_finding::<D>),
        )
        .route(
            "/jobs/:job_id/file_finding_issue",
            post(file_finding_issue::<D>),
        )
        .route(
            "/jobs/:job_id/override_verdict",
            post(override_verdict::<D>),
        )
        .route("/jobs/:job_id/rerun_gate", post(rerun_gate::<D>))
        .route("/jobs/:job_id/rerun_checks", post(rerun_checks::<D>))
        .route("/jobs/:job_id/show_again", post(show_again::<D>))
        .route("/jobs/:job_id/run_sheet", get(get_run_sheet::<D>))
        .route("/jobs/:job_id/runs", get(list_runs::<D>))
        .route(
            "/jobs/:job_id/runs/:run_id/output",
            get(get_run_output::<D>),
        )
        .route("/jobs/:job_id/runs/:run_id/observe", get(observe_run::<D>))
        .route("/jobs/:job_id/start_run", post(start_run::<D>))
        .route("/jobs/:job_id/stop_run", post(stop_run::<D>))
        .route("/jobs/:job_id/undo_run", post(undo_run::<D>))
        .route("/manifest/run_sheet", get(get_checkout_run_sheet::<D>))
        .route("/manifest/runs", get(list_checkout_runs::<D>))
        .route(
            "/manifest/runs/:run_id/output",
            get(get_checkout_run_output::<D>),
        )
        .route(
            "/manifest/runs/:run_id/diff",
            get(get_checkout_run_diff::<D>),
        )
        .route(
            "/manifest/runs/:run_id/observe",
            get(observe_checkout_run::<D>),
        )
        .route("/manifest/start_run", post(start_checkout_run::<D>))
        .route("/manifest/stop_run", post(stop_checkout_run::<D>))
        .route("/manifest/undo_run", post(undo_checkout_run::<D>))
        .route("/manifest/start_verify", post(start_checkout_verify::<D>))
        .route("/servers", get(list_servers::<D>))
        .route("/servers/start", post(start_server::<D>))
        .route("/servers/stop", post(stop_server::<D>))
        .route("/servers/:server_id/observe", get(observe_server::<D>))
        .route(
            "/jobs/:job_id/approve_dispatch",
            post(approve_dispatch::<D>),
        )
        .route("/jobs/:job_id/raise_cost_cap", post(raise_cost_cap::<D>))
        .route("/jobs/:job_id/raise_turn_cap", post(raise_turn_cap::<D>))
        .route("/jobs/:job_id/kill_drone", post(kill_drone::<D>))
        .route("/jobs/:job_id/kill_job", post(kill_job::<D>))
        .route("/jobs/:job_id/forget_job", post(forget_job::<D>))
        .route(
            "/jobs/:job_id/reclaim_worktree",
            post(reclaim_worktree::<D>),
        )
        .route("/jobs/:job_id/delete_branch", post(delete_branch::<D>))
        .route("/jobs/:job_id/redispatch", post(redispatch_job::<D>))
        .route("/jobs/:job_id/add_task", post(add_task::<D>))
        .route("/jobs/:job_id/drop_task", post(drop_task::<D>))
        .route("/jobs/:job_id/redirect", post(redirect_drone::<D>))
        .route("/jobs/:job_id/restart_step", post(restart_step::<D>))
        .route("/jobs/:job_id/answer_question", post(answer_question::<D>))
        .route("/jobs/:job_id/answer_command", post(answer_command::<D>))
        .route(
            "/jobs/:job_id/set_when_blocked",
            post(set_when_blocked::<D>),
        )
        .route("/jobs/:job_id/answer_judge", post(answer_judge::<D>))
        .route(
            "/jobs/:job_id/set_when_refused",
            post(set_when_refused::<D>),
        )
        .route(
            "/jobs/:job_id/set_model",
            post(crate::commands::set_model::<D>),
        )
        .route(
            "/jobs/:job_id/set_review_model",
            post(crate::commands::set_review_model::<D>),
        )
        .route(
            "/jobs/:job_id/remove_allowed_command",
            post(crate::commands::remove_allowed_command::<D>),
        )
        .route("/jobs/:job_id/report", post(file_report::<D>))
        .route("/reports", get(list_reports::<D>))
        .route("/worktrees", get(list_worktrees::<D>))
        .route("/jobs/:job_id/observe", get(observe_job::<D>))
        .route("/jobs/:job_id/log", get(job_log::<D>))
        .route("/jobs/:job_id/log/read", get(get_job_log::<D>))
        .route(
            "/jobs/:job_id/checks/:kept/observe",
            get(observe_check_output::<D>),
        )
        .route("/helm/observe", get(observe_helm::<D>))
        .route("/helm/ask", post(ask_helm::<D>))
        .route("/helm/start_fresh", post(start_helm_fresh::<D>))
        .route("/helm/permission", post(ask_the_person::<D>))
        .route("/helm/calls", get(list_helm_calls::<D>))
        .route("/helm/calls/answer", post(answer_helm_call::<D>))
        .route("/studios", get(list_studios::<D>))
        .route("/studios/create", post(create_studio::<D>))
        .route("/studios/:studio_id", get(get_studio::<D>))
        .route(
            "/studios/:studio_id/frames/:node_id",
            get(get_studio_frame::<D>),
        )
        .route("/studios/:studio_id/rename", post(rename_studio::<D>))
        .route("/studios/:studio_id/delete", post(delete_studio::<D>))
        .route("/studios/:studio_id/add_node", post(add_studio_node::<D>))
        .route(
            "/studios/:studio_id/capture_note",
            post(capture_studio_note::<D>),
        )
        .route("/studios/:studio_id/move_node", post(move_studio_node::<D>))
        .route(
            "/studios/:studio_id/remove_node",
            post(remove_studio_node::<D>),
        )
        .route(
            "/studios/:studio_id/remove_nodes",
            post(remove_studio_nodes::<D>),
        )
        .route(
            "/studios/:studio_id/propose_edge",
            post(propose_studio_edge::<D>),
        )
        .route(
            "/studios/:studio_id/decide_edge",
            post(decide_studio_edge::<D>),
        )
        .route(
            "/studios/:studio_id/group_nodes",
            post(group_studio_nodes::<D>),
        )
        .route("/studios/:studio_id/defer", post(defer_on_studio::<D>))
        .route(
            "/studios/:studio_id/write_up",
            post(write_up_studio_node::<D>),
        )
        .route(
            "/studios/:studio_id/edit_draft",
            post(edit_studio_draft::<D>),
        )
        .route("/studios/:studio_id/edit_link", post(edit_studio_link::<D>))
        .route(
            "/studios/:studio_id/settle",
            post(settle_contradiction::<D>),
        )
        .route(
            "/studios/:studio_id/dispatch_draft",
            post(dispatch_studio_draft::<D>),
        )
        .route("/studios/:studio_id/ask_scout", post(ask_scout::<D>))
        .route("/studios/:studio_id/start_scout", post(start_scout::<D>))
        .route("/studios/:studio_id/stop_scout", post(stop_scout::<D>))
        .route("/studios/:studio_id/read_in", post(read_in_link::<D>))
        .route("/studios/:studio_id/start_run", post(start_studio_run::<D>))
        .route("/events", get(events::<D>))
        // The Evidence endpoint, on the same listener and deliberately not in
        // `SERVED`: it is the Fleet/Drone seam rather than the Fleet/Bridge
        // one, and the inventory this table is checked against is Bridge's.
        .merge(crate::mcp::mounted::<D>())
        .with_state(served)
}
