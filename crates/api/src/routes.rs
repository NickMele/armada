//! The route table, by hand, and the handlers under it.
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
//! second port and no assembly step: queries and commands answer over HTTP
//! because they are request-response, and only the unsolicited push needs the
//! socket. Who initiates is the whole rule.

use axum::body::Bytes;
use axum::extract::ws::{Message, WebSocket, WebSocketUpgrade};
use axum::extract::{Path, State};
use axum::http::{header, StatusCode};
use axum::response::{IntoResponse, Response};
use axum::routing::{get, post};
use axum::Router;
use ipc::{
    Answer, ChangesRequested, FileReport, JobId, JobRequest, Missed, Overruled, ProposeJob,
    Redirection, Resync, RunId, StreamMessage, WireError, PROTOCOL_VERSION,
};
use serde::Serialize;
use std::sync::Arc;

use crate::daemon::{Daemon, Refusal};
use crate::stream::{Broadcaster, Next};

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
    // Its own route rather than a shape `redirect` also takes: that one carries
    // a person's own words and this carries one of a closed set the Drone
    // offered, and one route taking either would make the closed set optional.
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
    // and not listed here is a kind no rule can see.
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
        operation: "job.files_changed",
        method: "GET",
        path: "/events",
    },
    Route {
        operation: "job.asking",
        method: "GET",
        path: "/events",
    },
];

/// A request body that would not parse.
///
/// The code is declared beside the thing that raises it, which is what makes
/// the set closed by collection rather than by a registry somebody has to keep.
const UNDECODABLE_REQUEST: &str = "api.undecodable_request";

/// A response that would not serialise. Unreachable for plain data, and
/// answered rather than panicked: a panic here drops a socket mid-Job.
const UNENCODABLE_RESPONSE: &str = "api.unencodable_response";

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
        .route("/jobs/:job_id", get(get_job::<D>))
        .route("/jobs/:job_id/events", get(get_job_events::<D>))
        .route("/jobs/:job_id/evidence", get(get_evidence::<D>))
        .route("/jobs/:job_id/diff", get(get_diff::<D>))
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

// ------------------------------------------------------------------ queries

async fn list_jobs<D: Daemon>(State(served): State<Served<D>>) -> Response {
    match served.daemon.list_jobs().await {
        Ok(jobs) => answer(StatusCode::OK, &jobs, &served.run_id),
        Err(refusal) => refused(refusal),
    }
}

/// One Job in full. **The Board row plus what the list redacts** — the steps
/// and where each got to, the criteria, the branch, the brief.
async fn get_job<D: Daemon>(
    State(served): State<Served<D>>,
    Path(job_id): Path<String>,
) -> Response {
    match served.daemon.get_job(JobId::carried(job_id)).await {
        Ok(detail) => answer(StatusCode::OK, &detail, &served.run_id),
        Err(refusal) => refused(refusal),
    }
}

/// Every move one Job made, oldest first. **The path taken**, which `get_job`
/// answers nothing about — it says where a Job is, and this says how it got
/// there.
///
/// Its own route because a history has no bound and a detail view is fetched to
/// draw a summary. Nothing here folds: the rows are read and rendered, and
/// `crates/store/src/fold.rs` stays the only thing that replays them.
async fn get_job_events<D: Daemon>(
    State(served): State<Served<D>>,
    Path(job_id): Path<String>,
) -> Response {
    match served.daemon.get_job_events(JobId::carried(job_id)).await {
        Ok(history) => answer(StatusCode::OK, &history, &served.run_id),
        Err(refusal) => refused(refusal),
    }
}

/// Every claim a Job's Drones have submitted. **What the work says about
/// itself**, which the gate ruled on and a person reads before deciding.
///
/// Its own route beside the diff rather than folded into it: this is a few
/// sentences per step and that is however large the work is.
async fn get_evidence<D: Daemon>(
    State(served): State<Served<D>>,
    Path(job_id): Path<String>,
) -> Response {
    match served.daemon.get_evidence(JobId::carried(job_id)).await {
        Ok(evidence) => answer(StatusCode::OK, &evidence, &served.run_id),
        Err(refusal) => refused(refusal),
    }
}

/// One Job's whole patch. **The expensive read, on the one route that asks for
/// it** — `get_job` is fetched on every open to draw a summary, and the bytes
/// are what a person reading a diff needs and nothing else does.
async fn get_diff<D: Daemon>(
    State(served): State<Served<D>>,
    Path(job_id): Path<String>,
) -> Response {
    match served.daemon.get_diff(JobId::carried(job_id)).await {
        Ok(diff) => answer(StatusCode::OK, &diff, &served.run_id),
        Err(refusal) => refused(refusal),
    }
}

/// The workflows a proposal may name. **The set Fleet will accept**, which is
/// why it is served rather than left to a caller to know.
async fn list_workflows<D: Daemon>(State(served): State<Served<D>>) -> Response {
    match served.daemon.list_workflows().await {
        Ok(workflows) => answer(StatusCode::OK, &workflows, &served.run_id),
        Err(refusal) => refused(refusal),
    }
}

async fn list_manifests<D: Daemon>(State(served): State<Served<D>>) -> Response {
    match served.daemon.list_manifests().await {
        Ok(manifests) => answer(StatusCode::OK, &manifests, &served.run_id),
        Err(refusal) => refused(refusal),
    }
}

async fn list_models<D: Daemon>(State(served): State<Served<D>>) -> Response {
    match served.daemon.list_models().await {
        Ok(models) => answer(StatusCode::OK, &models, &served.run_id),
        Err(refusal) => refused(refusal),
    }
}

// ----------------------------------------------------------------- commands

async fn propose_job<D: Daemon>(State(served): State<Served<D>>, body: Bytes) -> Response {
    let proposal: ProposeJob = match ipc::decode("proposed Job", &body) {
        Ok(proposal) => proposal,
        // 400 is the transport's own refusal and never the daemon's — the
        // bytes did not become a request, so nothing downstream was asked.
        Err(why) => return undecodable(&why.to_string(), &served.run_id),
    };
    match served.daemon.propose_job(proposal).await {
        // 201: the Job now exists, at the approval gate. It is not running, and
        // nothing here approves it.
        Ok(job) => answer(StatusCode::CREATED, &job, &served.run_id),
        Err(refusal) => refused(refusal),
    }
}

/// The other way a Job reaches the gate: a person describes the work and the
/// proposer reads it. **The same 201 and the same gate** — what differs is who
/// filled the workflow in, and that one request can be several Jobs.
async fn propose_from_request<D: Daemon>(State(served): State<Served<D>>, body: Bytes) -> Response {
    let request: JobRequest = match ipc::decode("request", &body) {
        Ok(request) => request,
        Err(why) => return undecodable(&why.to_string(), &served.run_id),
    };
    match served.daemon.propose_from_request(request).await {
        Ok(job) => answer(StatusCode::CREATED, &job, &served.run_id),
        Err(refusal) => refused(refusal),
    }
}

async fn approve_dispatch<D: Daemon>(
    State(served): State<Served<D>>,
    Path(job_id): Path<String>,
) -> Response {
    match served.daemon.approve_dispatch(JobId::carried(job_id)).await {
        Ok(job) => answer(StatusCode::OK, &job, &served.run_id),
        Err(refusal) => refused(refusal),
    }
}

/// The person takes the work. **The counterpart to `approve_dispatch`**, at the
/// other end of the Job: that is the gate before anything runs, and this is the
/// decision after it has.
///
/// 409 anywhere but `awaiting_review`, which is what keeps it from becoming the
/// dispatch gate under a second name.
async fn approve_review<D: Daemon>(
    State(served): State<Served<D>>,
    Path(job_id): Path<String>,
) -> Response {
    match served.daemon.approve_review(JobId::carried(job_id)).await {
        Ok(job) => answer(StatusCode::OK, &job, &served.run_id),
        Err(refusal) => refused(refusal),
    }
}

/// Send the work back with a note. **The Job comes back `running`**, at the
/// same step, with the same Drone — nothing was thrown away and nothing was
/// spawned.
///
/// 409 where the Drone is gone: there is nobody to tell.
async fn request_changes<D: Daemon>(
    State(served): State<Served<D>>,
    Path(job_id): Path<String>,
    body: Bytes,
) -> Response {
    let note: ChangesRequested = match ipc::decode("a review note", &body) {
        Ok(note) => note,
        Err(why) => return undecodable(&why.to_string(), &served.run_id),
    };
    match served
        .daemon
        .request_changes(JobId::carried(job_id), note)
        .await
    {
        Ok(job) => answer(StatusCode::OK, &job, &served.run_id),
        Err(refusal) => refused(refusal),
    }
}

/// A verdict on the work, and the Job is over. **Terminal**, which is what
/// separates it from `request_changes` — and it is not `kill_job`, which clears
/// the Board and carries no verdict at all.
async fn reject_job<D: Daemon>(
    State(served): State<Served<D>>,
    Path(job_id): Path<String>,
) -> Response {
    match served.daemon.reject_job(JobId::carried(job_id)).await {
        Ok(job) => answer(StatusCode::OK, &job, &served.run_id),
        Err(refusal) => refused(refusal),
    }
}

/// The Judge refused, a person disagrees, and the step advances anyway. **The
/// Job comes back `running`** at the step that follows, with everything the
/// refused Drone did still on the branch.
///
/// 409 anywhere but an `escalated` Job stopped on `gate_failure`: a gate that
/// never weighed the work and a gaming flag are not opinions to be overruled,
/// and a failed mechanical Check is terminal and reaches this route as a Job
/// with no stopped step. 422 on a blank reason.
async fn override_verdict<D: Daemon>(
    State(served): State<Served<D>>,
    Path(job_id): Path<String>,
    body: Bytes,
) -> Response {
    let overruling: Overruled = match ipc::decode("an override", &body) {
        Ok(overruling) => overruling,
        Err(why) => return undecodable(&why.to_string(), &served.run_id),
    };
    match served
        .daemon
        .override_verdict(JobId::carried(job_id), overruling)
        .await
    {
        Ok(job) => answer(StatusCode::OK, &job, &served.run_id),
        Err(refusal) => refused(refusal),
    }
}

/// The gate could not decide, and a person asks it again on the evidence the
/// step already submitted. **The Job comes back wherever the second reading
/// left it** — running at the next step where it ruled, escalated again where
/// it could not.
///
/// No body, because nothing is being disagreed with. 409 anywhere but an
/// `escalated` Job stopped on `gate_undecided`, and 409 again on a Job this
/// Fleet is no longer standing at, where the baseline the first reading used
/// went with the slot and `restart_step` is what applies.
async fn rerun_gate<D: Daemon>(
    State(served): State<Served<D>>,
    Path(job_id): Path<String>,
) -> Response {
    match served.daemon.rerun_gate(JobId::carried(job_id)).await {
        Ok(job) => answer(StatusCode::OK, &job, &served.run_id),
        Err(refusal) => refused(refusal),
    }
}

/// The process, not the unit of work. What comes back is the Job the Drone was
/// on, which is still there.
async fn kill_drone<D: Daemon>(
    State(served): State<Served<D>>,
    Path(job_id): Path<String>,
) -> Response {
    match served.daemon.kill_drone(JobId::carried(job_id)).await {
        Ok(job) => answer(StatusCode::OK, &job, &served.run_id),
        Err(refusal) => refused(refusal),
    }
}

/// The unit of work, not the process. A separate operation from `kill_drone`
/// because two of the edges into `killed` leave a status no
/// Drone has ever existed under, and neither one can be spelled as killing a
/// Drone.
async fn kill_job<D: Daemon>(
    State(served): State<Served<D>>,
    Path(job_id): Path<String>,
) -> Response {
    match served.daemon.kill_job(JobId::carried(job_id)).await {
        Ok(job) => answer(StatusCode::OK, &job, &served.run_id),
        Err(refusal) => refused(refusal),
    }
}

/// Delete the Job's whole record. **Real deletion, not a status** — the row
/// and everything beneath it are gone, and there is no undo.
///
/// 409 where the Job is not yet terminal: `kill_job` is the act that ends one
/// still in flight, and this only clears a Board of work that has already
/// finished. It does not touch the worktree or the branch — `armada clean`
/// keeps that concern.
async fn forget_job<D: Daemon>(
    State(served): State<Served<D>>,
    Path(job_id): Path<String>,
) -> Response {
    match served.daemon.forget_job(JobId::carried(job_id)).await {
        Ok(forgotten) => answer(StatusCode::OK, &forgotten, &served.run_id),
        Err(refusal) => refused(refusal),
    }
}

/// Mint a replacement for a stopped Job. **Two Jobs come back**, and
/// the answer is 200 rather than 201 because the act a caller asked for is the
/// recovery, not the creation — the new Job's id is in the body.
async fn redispatch_job<D: Daemon>(
    State(served): State<Served<D>>,
    Path(job_id): Path<String>,
) -> Response {
    match served.daemon.redispatch_job(JobId::carried(job_id)).await {
        Ok(both) => answer(StatusCode::OK, &both, &served.run_id),
        Err(refusal) => refused(refusal),
    }
}

/// Say something to the Drone that is there. **The Job comes back `running`**,
/// at the same step, with the same Drone — nothing was spawned and nothing was
/// thrown away.
///
/// 409 where the Drone is gone, naming `restart_step` as the act that applies.
async fn redirect_drone<D: Daemon>(
    State(served): State<Served<D>>,
    Path(job_id): Path<String>,
    body: Bytes,
) -> Response {
    let instruction: Redirection = match ipc::decode("a redirect", &body) {
        Ok(instruction) => instruction,
        Err(why) => return undecodable(&why.to_string(), &served.run_id),
    };
    match served
        .daemon
        .redirect_drone(JobId::carried(job_id), instruction)
        .await
    {
        Ok(job) => answer(StatusCode::OK, &job, &served.run_id),
        Err(refusal) => refused(refusal),
    }
}

/// Answer the question a Drone asked. **The Job comes back unchanged** — it was
/// `running` while it waited and is `running` now; what moved is the Drone,
/// handed the answer as a turn. 409 where nothing is waiting, where the id
/// names a question already answered, and where the label was not offered.
async fn answer_question<D: Daemon>(
    State(served): State<Served<D>>,
    Path(job_id): Path<String>,
    body: Bytes,
) -> Response {
    let chosen: Answer = match ipc::decode("an answer", &body) {
        Ok(chosen) => chosen,
        Err(why) => return undecodable(&why.to_string(), &served.run_id),
    };
    match served
        .daemon
        .answer_question(JobId::carried(job_id), chosen)
        .await
    {
        Ok(job) => answer(StatusCode::OK, &job, &served.run_id),
        Err(refusal) => refused(refusal),
    }
}

/// A body that would not parse, as the 400 it is.
///
/// **Written once.** This arm was spelled out at every command that takes a
/// body, seven times, each one four lines of `WireError` construction that has
/// to agree with the other six. Adding the eighth is what made it a function.
fn undecodable(why: &str, run_id: &RunId) -> Response {
    problem(
        StatusCode::BAD_REQUEST,
        &WireError::raised(UNDECODABLE_REQUEST, why.to_string(), run_id.clone())
            .caused_by(vec![why.to_string()]),
    )
}

/// Put a new Drone on the worktree the last one left. **One Job comes back**,
/// not two — this is the same Job resuming, which is the whole of what makes it
/// different from a redispatch.
///
/// 409 where the Drone is alive, and where the worktree is gone.
async fn restart_step<D: Daemon>(
    State(served): State<Served<D>>,
    Path(job_id): Path<String>,
) -> Response {
    match served.daemon.restart_step(JobId::carried(job_id)).await {
        Ok(job) => answer(StatusCode::OK, &job, &served.run_id),
        Err(refusal) => refused(refusal),
    }
}

/// A person says this Job failed in error. **201, because a report now
/// exists** — and nothing else does: no Job is proposed, no Drone is spawned,
/// and the Job it names is exactly where it was.
///
/// 422 on a blank sentence. The record was already served by three other
/// routes before anybody pressed anything, so a filing with the bundle and no
/// sentence has added nothing at all.
async fn file_report<D: Daemon>(
    State(served): State<Served<D>>,
    Path(job_id): Path<String>,
    body: Bytes,
) -> Response {
    let filing: FileReport = match ipc::decode("a report", &body) {
        Ok(filing) => filing,
        Err(why) => return undecodable(&why.to_string(), &served.run_id),
    };
    match served
        .daemon
        .file_report(JobId::carried(job_id), filing)
        .await
    {
        Ok(report) => answer(StatusCode::CREATED, &report, &served.run_id),
        Err(refusal) => refused(refusal),
    }
}

/// Every report filed, newest first, with the counts they are read beside.
///
/// **Not under `/jobs`**, and that is the shape of the claim: a report survives
/// `armada clean` taking its Job away, so it is a record of its own rather than
/// a row beneath one.
async fn list_reports<D: Daemon>(State(served): State<Served<D>>) -> Response {
    match served.daemon.list_reports().await {
        Ok(reports) => answer(StatusCode::OK, &reports, &served.run_id),
        Err(refusal) => refused(refusal),
    }
}

// ------------------------------------------------------------- the upgrade

/// The event stream. **Global, and a client subscribes to nothing** — one
/// socket carries every Job, because Bridge holds exactly one connection.
///
/// Nothing is read from the socket. The stream is one-directional by design:
/// there is no subscribe message to read, and a connection that carried state
/// would be a connection that is expensive to drop and remake.
/// One Job's turns. **Per-Job, and not on `/events`** — that stream is one
/// drop-oldest channel carrying every Job, and a transcript row on it would
/// evict the state changes a Board is drawn from.
///
/// The daemon is asked **before** the upgrade, so a Job that does not exist is
/// a 404 the caller reads at the moment they asked rather than a socket that
/// opens and says nothing. What comes back already holds the subscription and
/// the history, in that order.
async fn observe_job<D: Daemon>(
    State(served): State<Served<D>>,
    Path(job_id): Path<String>,
    upgrade: WebSocketUpgrade,
) -> Response {
    match served.daemon.observe_job(JobId::carried(job_id)).await {
        Ok(observed) => upgrade.on_upgrade(move |socket| crate::observing::relay(socket, observed)),
        Err(refusal) => refused(refusal),
    }
}

async fn events<D: Daemon>(State(served): State<Served<D>>, upgrade: WebSocketUpgrade) -> Response {
    upgrade.on_upgrade(move |socket| watch(socket, served))
}

async fn watch<D: Daemon>(mut socket: WebSocket, served: Served<D>) {
    // Subscribe first, then snapshot. The other order drops whatever lands in
    // between; this order can only repeat, and a repeat is detectable.
    let mut subscription = served.events.subscribe();
    if !resync(&mut socket, &served).await {
        return;
    }
    while let Some(next) = subscription.next().await {
        let delivered = match next {
            Next::Send(delivered) => send(&mut socket, &StreamMessage::Event(delivered)).await,
            // The count alone cannot repair what the client holds, so the drop
            // is always followed by current state.
            Next::Missed(dropped) => {
                send(&mut socket, &StreamMessage::Missed(Missed { dropped })).await
                    && resync(&mut socket, &served).await
            }
        };
        if !delivered {
            return;
        }
    }
}

/// Current state, whole. `false` where the socket or the daemon is gone, and
/// the caller stops.
///
/// A daemon that cannot answer closes the socket rather than sending a partial
/// snapshot: there is no error message on this stream, and a client that
/// reconnects gets a whole answer or none.
async fn resync<D: Daemon>(socket: &mut WebSocket, served: &Served<D>) -> bool {
    let cursor = served.events.cursor();
    let Ok(jobs) = served.daemon.list_jobs().await else {
        return false;
    };
    send(
        socket,
        &StreamMessage::Resync(Resync {
            protocol_version: PROTOCOL_VERSION,
            cursor,
            jobs,
        }),
    )
    .await
}

async fn send(socket: &mut WebSocket, message: &StreamMessage) -> bool {
    let Ok(text) = ipc::encode(message) else {
        return false;
    };
    // Awaited, not queued: this is what makes a slow client slow *this* task
    // rather than Fleet's memory, so the bound upstream is the thing that gives.
    socket.send(Message::Text(text)).await.is_ok()
}

// ------------------------------------------------------------------ answers

fn answer<T: Serialize>(status: StatusCode, value: &T, run_id: &RunId) -> Response {
    match ipc::encode(value) {
        Ok(body) => (status, [(header::CONTENT_TYPE, "application/json")], body).into_response(),
        Err(why) => problem(
            StatusCode::INTERNAL_SERVER_ERROR,
            &WireError::raised(UNENCODABLE_RESPONSE, why.to_string(), run_id.clone())
                .caused_by(vec![why.to_string()]),
        ),
    }
}

fn refused(refusal: Refusal) -> Response {
    let status =
        StatusCode::from_u16(refusal.status()).unwrap_or(StatusCode::INTERNAL_SERVER_ERROR);
    problem(status, refusal.error())
}

fn problem(status: StatusCode, error: &WireError) -> Response {
    match ipc::encode(error) {
        Ok(body) => (status, [(header::CONTENT_TYPE, "application/json")], body).into_response(),
        // The error would not serialise, which leaves nothing true to say with
        // a body. The status is still the answer.
        Err(_) => status.into_response(),
    }
}
