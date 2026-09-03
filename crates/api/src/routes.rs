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
//! # The handlers are next door
//!
//! `crate::queries` reads, `crate::commands` writes, `crate::sockets` upgrades.
//! **The table and the router stay here together**, because the rule comparing
//! them to the inventory reads this file.

use axum::routing::{get, post};
use axum::Router;
use ipc::RunId;
use std::sync::Arc;

use crate::commands::{
    answer_question, approve_dispatch, approve_review, file_report, forget_job, kill_drone,
    kill_job, override_verdict, propose_from_request, propose_job, reclaim_worktree,
    redirect_drone, redispatch_job, reject_job, request_changes, rerun_gate, restart_step,
};
use crate::daemon::Daemon;
use crate::queries::{
    get_call, get_capacity, get_diff, get_evidence, get_job, get_job_events, list_jobs,
    list_manifests, list_models, list_reports, list_workflows,
};
use crate::sockets::{events, observe_job};
use crate::stream::Broadcaster;

/// One operation, and where it is served.
///
/// `operation` is the key in `crates/ipc/operations.toml`, spelled exactly as
/// that file spells it, so comparing the two needs a set lookup and no mapping.
pub struct Route {
    pub operation: &'static str,
    pub method: &'static str,
    pub path: &'static str,
}

/// The operations M1 serves — a deliberate subset of the inventory, not all of it.
///
/// The rest of the inventory, the `/v0` lifeboat and version-skew handling are
/// the Ship milestone's. Nothing here stubs them: a route that answers with a
/// placeholder is worse than one that 404s, because a client cannot tell the
/// difference between not built and not working.
///
/// A path is checkable against its inventory key by eye. `list_jobs` and
/// `get_job` are the collection and the member and name no act; every other row
/// spells its key in the last segment, except `redispatch`, which drops `_job`
/// because no `redispatch_drone` exists to tell it apart from. The lifeboat's
/// `POST /v0/jobs/:id/kill` is the same act as `kill_job` under a frozen prefix
/// that shares nothing with this table, by design.
pub const SERVED: &[Route] = &[
    Route {
        operation: "list_jobs",
        method: "GET",
        path: "/jobs",
    },
    Route {
        operation: "get_job",
        method: "GET",
        path: "/jobs/:job_id",
    },
    // Fleet's own capacity, not a Job's. It is `/capacity` and not
    // `/jobs/capacity` because nothing about it is scoped to a Job — the whole
    // point is the answer no per-Job field could give.
    Route {
        operation: "get_capacity",
        method: "GET",
        path: "/capacity",
    },
    // The path taken, under the Job that took it. `get_job_events` drops
    // `get_` and `job_` for the reason `redispatch` drops `_job`: the segment
    // before it already names the Job. It is not `/events`, which is the
    // global stream and carries no history at all.
    Route {
        operation: "get_job_events",
        method: "GET",
        path: "/jobs/:job_id/events",
    },
    // The two halves of the work product, on two routes and not one. Evidence
    // is a handful of sentences per step and the patch is however large the
    // work is, so a surface wanting only the claims does not fetch the bytes.
    // Neither is on `get_job`, which is read on every open to draw a summary.
    Route {
        operation: "get_evidence",
        method: "GET",
        path: "/jobs/:job_id/evidence",
    },
    Route {
        operation: "get_diff",
        method: "GET",
        path: "/jobs/:job_id/diff",
    },
    // One call's arguments, and a member read rather than an act — the same
    // shape as `get_job` under `/jobs`, which is why the last segment is the id
    // and not the key. The socket sends the line and the size; this is what a
    // person opening that row asks for, once, about one call.
    Route {
        operation: "get_call",
        method: "GET",
        path: "/jobs/:job_id/calls/:call_id",
    },
    Route {
        operation: "list_workflows",
        method: "GET",
        path: "/workflows",
    },
    Route {
        operation: "list_manifests",
        method: "GET",
        path: "/manifests",
    },
    Route {
        operation: "list_models",
        method: "GET",
        path: "/models",
    },
    Route {
        operation: "propose_job",
        method: "POST",
        path: "/jobs",
    },
    // A second POST under `/jobs`, and a segment rather than a mode flag on the
    // first: what a caller sends is a different shape, and one route taking
    // either would be the two paths sharing a body that means two things.
    Route {
        operation: "propose_from_request",
        method: "POST",
        path: "/jobs/from_request",
    },
    Route {
        operation: "approve_dispatch",
        method: "POST",
        path: "/jobs/:job_id/approve_dispatch",
    },
    // The three answers at a human gate, and three routes rather than one with
    // a decision in the body: each does something different to the Job, and one
    // route taking any of them would be a body that means three things. The
    // reject path drops `_job` for the reason `redispatch` does.
    Route {
        operation: "approve_review",
        method: "POST",
        path: "/jobs/:job_id/approve_review",
    },
    Route {
        operation: "request_changes",
        method: "POST",
        path: "/jobs/:job_id/request_changes",
    },
    Route {
        operation: "reject_job",
        method: "POST",
        path: "/jobs/:job_id/reject",
    },
    // The answer at a gate that refused, which is a different place from the
    // three above: those answer `awaiting_review` and this answers `escalated`.
    // Its own route rather than a flag on `approve_review` — one route taking
    // either would let a refusal be taken with the act built for work nothing
    // objected to, which is exactly the confusion the record has to keep apart.
    Route {
        operation: "override_verdict",
        method: "POST",
        path: "/jobs/:job_id/override_verdict",
    },
    // The answer at a gate that could not rule, which is a different place
    // again: `override_verdict` lifts a decision and this asks for one. Its own
    // route rather than a flag on that one — the two triggers do not overlap,
    // and one route taking either would let a step nothing weighed be advanced
    // by the act built for disagreeing with a machine that did.
    Route {
        operation: "rerun_gate",
        method: "POST",
        path: "/jobs/:job_id/rerun_gate",
    },
    Route {
        operation: "kill_drone",
        method: "POST",
        path: "/jobs/:job_id/kill_drone",
    },
    Route {
        operation: "kill_job",
        method: "POST",
        path: "/jobs/:job_id/kill_job",
    },
    // Real deletion, and the only route on this table that is: every other
    // command moves a Job further, and this removes the row.
    Route {
        operation: "forget_job",
        method: "POST",
        path: "/jobs/:job_id/forget_job",
    },
    // The other half of the row above, and its own route because that row says
    // why: one call with two unrelated things to fail at is worse than two
    // calls. This one takes the disk and leaves the record.
    Route {
        operation: "reclaim_worktree",
        method: "POST",
        path: "/jobs/:job_id/reclaim_worktree",
    },
    Route {
        operation: "redispatch_job",
        method: "POST",
        path: "/jobs/:job_id/redispatch",
    },
    // The two acts that resume a step without redispatching. Two routes and
    // not one with a mode: which applies is decided by whether the Job holds a
    // Drone, and a caller that asked for the wrong one is told which is right
    // rather than silently given it.
    Route {
        operation: "redirect_drone",
        method: "POST",
        path: "/jobs/:job_id/redirect",
    },
    Route {
        operation: "restart_step",
        method: "POST",
        path: "/jobs/:job_id/restart_step",
    },
    // Its own route and not a shape `redirect` also takes: one carries a
    // person's words and this a label the Drone offered, and one route taking
    // either would make the closed set optional.
    Route {
        operation: "answer_question",
        method: "POST",
        path: "/jobs/:job_id/answer_question",
    },
    // What a person says went wrong, under the Job it is about, and every
    // report filed, which is not under one — a report outlives the Job it
    // names, so a listing reachable only through a Job would lose exactly the
    // reports that most need reading.
    Route {
        operation: "file_report",
        method: "POST",
        path: "/jobs/:job_id/report",
    },
    Route {
        operation: "list_reports",
        method: "GET",
        path: "/reports",
    },
    // A socket rather than a body: it opens with what has already happened and
    // then continues, which no request-response shape carries. The path drops
    // `_job` for the reason `redispatch` does — the segment before it already
    // names the Job.
    Route {
        operation: "observe_job",
        method: "GET",
        path: "/jobs/:job_id/observe",
    },
    // Every event kind is served on the one socket, and every one is named:
    // `SERVED` is what a rule compares to the inventory, so a kind published
    // and not listed here is a kind no rule can see. The rule also compares
    // this group to `crates/ipc/src/event.rs`'s `Event` enum, which is the
    // closed set of kinds that can actually be published — a variant there
    // with no row here now fails the gate rather than reading as complete
    // while two kinds went unlisted.
    Route {
        operation: "job.created",
        method: "GET",
        path: "/events",
    },
    Route {
        operation: "job.state_changed",
        method: "GET",
        path: "/events",
    },
    Route {
        operation: "job.step_advanced",
        method: "GET",
        path: "/events",
    },
    Route {
        operation: "drone.spawned",
        method: "GET",
        path: "/events",
    },
    Route {
        operation: "drone.exited",
        method: "GET",
        path: "/events",
    },
    Route {
        operation: "job.files_changed",
        method: "GET",
        path: "/events",
    },
    Route {
        operation: "job.judging",
        method: "GET",
        path: "/events",
    },
    Route {
        operation: "job.asking",
        method: "GET",
        path: "/events",
    },
    Route {
        operation: "job.forgotten",
        method: "GET",
        path: "/events",
    },
    Route {
        operation: "job.landed",
        method: "GET",
        path: "/events",
    },
];

/// Everything a handler needs. Cloned per request, so nothing here may be
/// expensive to clone.
pub struct Served<D> {
    daemon: Arc<D>,
    /// **This process's** run id, minted at start and never Fleet's-by-
    /// assumption. Every error the transport raises carries it.
    run_id: RunId,
    events: Broadcaster,
}

impl<D> Served<D> {
    /// The daemon this listener answers from, handed over.
    ///
    /// The transport is the only holder. For a caller that also has to *drive*
    /// the daemon — anything calling a turn on an interval — see
    /// [`Served::sharing`].
    pub fn by(daemon: D, run_id: RunId, events: Broadcaster) -> Served<D> {
        Served::sharing(Arc::new(daemon), run_id, events)
    }

    /// The same daemon the caller keeps a reference to.
    ///
    /// **This is what lets a Job advance.** Serving is one of two things a
    /// process does with a daemon and driving it is the other, so a
    /// constructor that consumed it left nothing in the process able to call
    /// `turn` — a Job dispatched on approval and then never settled. The state
    /// was already an `Arc` for cloning per request; this only stops that
    /// `Arc` being made where nobody else can reach it.
    ///
    /// No `Daemon` implementation for `Arc<D>` is needed for this and none is
    /// stated: the handlers reach the daemon through the state's own `Arc`, so
    /// `D` stays the concrete daemon and one indirection stays one.
    pub fn sharing(daemon: Arc<D>, run_id: RunId, events: Broadcaster) -> Served<D> {
        Served {
            daemon,
            run_id,
            events,
        }
    }

    /// The stream this listener publishes from, for whoever holds the daemon.
    pub fn events(&self) -> Broadcaster {
        self.events.clone()
    }

    /// The daemon, for a handler in another module of this crate.
    pub(crate) fn daemon(&self) -> &D {
        &self.daemon
    }

    /// **This process's** run id, which every error the transport raises
    /// carries. Read by the handlers next door and by nothing outside.
    pub(crate) fn run_id(&self) -> &RunId {
        &self.run_id
    }
}

impl<D> Clone for Served<D> {
    fn clone(&self) -> Served<D> {
        Served {
            daemon: Arc::clone(&self.daemon),
            run_id: self.run_id.clone(),
            events: self.events.clone(),
        }
    }
}

/// The one listener. HTTP for queries and commands, an upgrade for events.
pub fn router<D: Daemon>(served: Served<D>) -> Router {
    Router::new()
        .route("/jobs", get(list_jobs::<D>).post(propose_job::<D>))
        .route("/jobs/from_request", post(propose_from_request::<D>))
        .route("/workflows", get(list_workflows::<D>))
        .route("/manifests", get(list_manifests::<D>))
        .route("/models", get(list_models::<D>))
        .route("/capacity", get(get_capacity::<D>))
        .route("/jobs/:job_id", get(get_job::<D>))
        .route("/jobs/:job_id/events", get(get_job_events::<D>))
        .route("/jobs/:job_id/evidence", get(get_evidence::<D>))
        .route("/jobs/:job_id/diff", get(get_diff::<D>))
        .route("/jobs/:job_id/calls/:call_id", get(get_call::<D>))
        .route("/jobs/:job_id/approve_review", post(approve_review::<D>))
        .route("/jobs/:job_id/request_changes", post(request_changes::<D>))
        .route("/jobs/:job_id/reject", post(reject_job::<D>))
        .route(
            "/jobs/:job_id/override_verdict",
            post(override_verdict::<D>),
        )
        .route("/jobs/:job_id/rerun_gate", post(rerun_gate::<D>))
        .route(
            "/jobs/:job_id/approve_dispatch",
            post(approve_dispatch::<D>),
        )
        .route("/jobs/:job_id/kill_drone", post(kill_drone::<D>))
        .route("/jobs/:job_id/kill_job", post(kill_job::<D>))
        .route("/jobs/:job_id/forget_job", post(forget_job::<D>))
        .route(
            "/jobs/:job_id/reclaim_worktree",
            post(reclaim_worktree::<D>),
        )
        .route("/jobs/:job_id/redispatch", post(redispatch_job::<D>))
        .route("/jobs/:job_id/redirect", post(redirect_drone::<D>))
        .route("/jobs/:job_id/restart_step", post(restart_step::<D>))
        .route("/jobs/:job_id/answer_question", post(answer_question::<D>))
        .route("/jobs/:job_id/report", post(file_report::<D>))
        .route("/reports", get(list_reports::<D>))
        .route("/jobs/:job_id/observe", get(observe_job::<D>))
        .route("/events", get(events::<D>))
        // The Evidence endpoint, on the same listener and deliberately not in
        // `SERVED`: it is the Fleet/Drone seam rather than the Fleet/Bridge
        // one, and the inventory this table is checked against is Bridge's.
        .merge(crate::mcp::mounted::<D>())
        .with_state(served)
}
