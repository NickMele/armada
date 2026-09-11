//! Writes. Every one moves something, and each says what it moves it to.
//!
//! **Split from the route table rather than from a line count.** What separates
//! these from `queries` is not size: a command decodes a body, may answer 201,
//! and has refusals that mean the machine would not admit the move. A caller
//! reading one of these needs to know what the Job looks like afterwards, which
//! is what each doc below states and no query has to.
//!
//! The 400 for a body that would not parse is the transport's own and never the
//! daemon's — see `crate::answers::undecodable`. Nothing downstream was asked.

use axum::body::Bytes;
use axum::extract::State;
use axum::http::StatusCode;
use axum::response::Response;
use ipc::{
    CapRaise, ChangesRequested, ChosenAnswer, FileReport, JobRequest, Overruled, ProposeJob,
    Redirection, RemarksTakenUp, RestartRequested, StopProposal, TurnRaise,
};

use crate::answers::{answer, refused, undecodable};
use crate::daemon::Commands;
use crate::reference::Resolved;
use crate::routes::Served;

pub(crate) async fn propose_job<D: Commands>(
    State(served): State<Served<D>>,
    body: Bytes,
) -> Response {
    let proposal: ProposeJob = match ipc::decode("proposed Job", &body) {
        Ok(proposal) => proposal,
        // 400 is the transport's own refusal and never the daemon's — the
        // bytes did not become a request, so nothing downstream was asked.
        Err(why) => return undecodable(&why.to_string(), served.run_id()),
    };
    match served.daemon().propose_job(proposal).await {
        // 201: the Job now exists, at the approval gate. It is not running, and
        // nothing here approves it.
        Ok(job) => answer(StatusCode::CREATED, &job, served.run_id()),
        Err(refusal) => refused(refusal),
    }
}

/// The other way a Job reaches the gate: a person describes the work and the
/// proposer reads it. **The same 201 and the same gate** — what differs is who
/// filled the workflow in, and that one request can be several Jobs.
pub(crate) async fn propose_from_request<D: Commands>(
    State(served): State<Served<D>>,
    body: Bytes,
) -> Response {
    let request: JobRequest = match ipc::decode("request", &body) {
        Ok(request) => request,
        Err(why) => return undecodable(&why.to_string(), served.run_id()),
    };
    match served.daemon().propose_from_request(request).await {
        Ok(job) => answer(StatusCode::CREATED, &job, served.run_id()),
        Err(refusal) => refused(refusal),
    }
}

/// Stop a proposal that is still out.
///
/// **200 on both arms**, including the one where there was nothing to stop.
/// `ipc::ProposalStopped` carries which, and a 404 for a proposal that has just
/// landed would report a failure to somebody whose Jobs are on the board.
pub(crate) async fn stop_proposal<D: Commands>(
    State(served): State<Served<D>>,
    body: Bytes,
) -> Response {
    let stopping: StopProposal = match ipc::decode("a proposal to stop", &body) {
        Ok(stopping) => stopping,
        Err(why) => return undecodable(&why.to_string(), served.run_id()),
    };
    match served.daemon().stop_proposal(stopping.proposal_id).await {
        Ok(stopped) => answer(StatusCode::OK, &stopped, served.run_id()),
        Err(refusal) => refused(refusal),
    }
}

pub(crate) async fn approve_dispatch<D: Commands>(
    State(served): State<Served<D>>,
    job: Resolved,
) -> Response {
    match served.daemon().approve_dispatch(job.id()).await {
        Ok(job) => answer(StatusCode::OK, &job, served.run_id()),
        Err(refusal) => refused(refusal),
    }
}

/// The person takes the work. **The counterpart to `approve_dispatch`**, at the
/// other end of the Job: that is the gate before anything runs, and this is the
/// decision after it has.
///
/// 409 anywhere but `awaiting_review`, which is what keeps it from becoming the
/// dispatch gate under a second name.
pub(crate) async fn approve_review<D: Commands>(
    State(served): State<Served<D>>,
    job: Resolved,
) -> Response {
    match served.daemon().approve_review(job.id()).await {
        Ok(job) => answer(StatusCode::OK, &job, served.run_id()),
        Err(refusal) => refused(refusal),
    }
}

/// The person merges the pull request their Job opened, and Fleet runs what
/// follows. **`approve_review` with a write to somebody else's repository in
/// front of it** — and the write is what makes the after-merge Checks run.
///
/// 409 anywhere but `awaiting_review`, and 409 where the record holds no pull
/// request. A forge that would not merge answers with the kind it refused on,
/// each its own code: a protected base, a conflict and a required check are
/// 409s, and a missing tool or an unreadable refusal are 500s. Nothing retries.
pub(crate) async fn merge_pull_request<D: Commands>(
    State(served): State<Served<D>>,
    job: Resolved,
) -> Response {
    match served.daemon().merge_pull_request(job.id()).await {
        Ok(job) => answer(StatusCode::OK, &job, served.run_id()),
        Err(refusal) => refused(refusal),
    }
}

/// Send the work back with a note. **The Job comes back `running`**, at the
/// same step, with the same Drone — nothing was thrown away and nothing was
/// spawned.
///
/// 409 where the Drone is gone: there is nobody to tell.
pub(crate) async fn request_changes<D: Commands>(
    State(served): State<Served<D>>,
    job: Resolved,
    body: Bytes,
) -> Response {
    let note: ChangesRequested = match ipc::decode("a review note", &body) {
        Ok(note) => note,
        Err(why) => return undecodable(&why.to_string(), served.run_id()),
    };
    match served.daemon().request_changes(job.id(), note).await {
        Ok(job) => answer(StatusCode::OK, &job, served.run_id()),
        Err(refusal) => refused(refusal),
    }
}

/// The comments a person picked off the pull request reach a Drone.
///
/// **The same answer `request_changes` gives**, because that is what it calls:
/// the Job comes back `queued` with the words waiting for the Drone
/// re-admission puts on it.
///
/// 409 where a comment named is one a Drone on this Job has already been
/// handed, or one the pull request no longer has. 422 where the press named no
/// comment at all.
pub(crate) async fn take_up_remarks<D: Commands>(
    State(served): State<Served<D>>,
    job: Resolved,
    body: Bytes,
) -> Response {
    let picked: RemarksTakenUp = match ipc::decode("the comments picked", &body) {
        Ok(picked) => picked,
        Err(why) => return undecodable(&why.to_string(), served.run_id()),
    };
    match served.daemon().take_up_remarks(job.id(), picked).await {
        Ok(job) => answer(StatusCode::OK, &job, served.run_id()),
        Err(refusal) => refused(refusal),
    }
}

/// A verdict on the work, and the Job is over. **Terminal**, which is what
/// separates it from `request_changes` — and it is not `kill_job`, which clears
/// the Board and carries no verdict at all.
pub(crate) async fn reject_job<D: Commands>(
    State(served): State<Served<D>>,
    job: Resolved,
) -> Response {
    match served.daemon().reject_job(job.id()).await {
        Ok(job) => answer(StatusCode::OK, &job, served.run_id()),
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
pub(crate) async fn override_verdict<D: Commands>(
    State(served): State<Served<D>>,
    job: Resolved,
    body: Bytes,
) -> Response {
    let overruling: Overruled = match ipc::decode("an override", &body) {
        Ok(overruling) => overruling,
        Err(why) => return undecodable(&why.to_string(), served.run_id()),
    };
    match served.daemon().override_verdict(job.id(), overruling).await {
        Ok(job) => answer(StatusCode::OK, &job, served.run_id()),
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
pub(crate) async fn rerun_gate<D: Commands>(
    State(served): State<Served<D>>,
    job: Resolved,
) -> Response {
    match served.daemon().rerun_gate(job.id()).await {
        Ok(job) => answer(StatusCode::OK, &job, served.run_id()),
        Err(refusal) => refused(refusal),
    }
}

/// Ask a Job to show its work: the repository's harness runs the last spec a
/// Drone named, in the Job's worktree, and what it captured comes back as a set
/// of its own. **The Job does not move**, so what comes back is the set and not
/// the Job.
///
/// 409 before anything runs where it cannot — no harness, no worktree, no spec,
/// the spec gone, a Drone working, a press already out. 200 with `nothing` set
/// where the harness ran and captured nothing, which is an answer rather than a
/// refusal. **It may take as long as the app takes to start**: the daemon runs
/// it off the turn loop, and the run finishes whether or not this request is
/// still waiting.
pub(crate) async fn show_again<D: Commands>(
    State(served): State<Served<D>>,
    job: Resolved,
) -> Response {
    match served.shared().show_again(job.id()).await {
        Ok(shown) => answer(StatusCode::OK, &shown, served.run_id()),
        Err(refusal) => refused(refusal),
    }
}

/// Give this Job a higher cost ceiling than the tier above it allows. **The
/// Job comes back exactly where it was** — the raise moves a number and nothing
/// else, and the field a caller reads it for is `queued_reason`: `over_budget`
/// before, absent or `waiting_on_resources` after.
///
/// 409 on a terminal Job, where nothing is left to spend. 422 on a figure at or
/// under the cap in force — the act only ever raises — and 422 again where the
/// surface asked past its own ceiling, which a person never meets.
pub(crate) async fn raise_cost_cap<D: Commands>(
    State(served): State<Served<D>>,
    job: Resolved,
    body: Bytes,
) -> Response {
    let raise: CapRaise = match ipc::decode("a cost cap", &body) {
        Ok(raise) => raise,
        Err(why) => return undecodable(&why.to_string(), served.run_id()),
    };
    match served.daemon().raise_cost_cap(job.id(), raise).await {
        Ok(job) => answer(StatusCode::OK, &job, served.run_id()),
        Err(refusal) => refused(refusal),
    }
}

/// The other ceiling, and the same three refusals in the same order. 409 on a
/// terminal Job, 422 on a figure at or under the cap in force, 422 again where
/// the surface asked past its own ceiling.
///
/// **A route of its own and never a field on the one above.** The two ceilings
/// are refused separately and cleared separately, so a caller that raised one
/// and was told about the other could not act on the answer.
pub(crate) async fn raise_turn_cap<D: Commands>(
    State(served): State<Served<D>>,
    job: Resolved,
    body: Bytes,
) -> Response {
    let raise: TurnRaise = match ipc::decode("a turn cap", &body) {
        Ok(raise) => raise,
        Err(why) => return undecodable(&why.to_string(), served.run_id()),
    };
    match served.daemon().raise_turn_cap(job.id(), raise).await {
        Ok(job) => answer(StatusCode::OK, &job, served.run_id()),
        Err(refusal) => refused(refusal),
    }
}

/// The process, not the unit of work. What comes back is the Job the Drone was
/// on, which is still there.
pub(crate) async fn kill_drone<D: Commands>(
    State(served): State<Served<D>>,
    job: Resolved,
) -> Response {
    match served.daemon().kill_drone(job.id()).await {
        Ok(job) => answer(StatusCode::OK, &job, served.run_id()),
        Err(refusal) => refused(refusal),
    }
}

/// The unit of work, not the process. A separate operation from `kill_drone`
/// because two of the edges into `killed` leave a status no
/// Drone has ever existed under, and neither one can be spelled as killing a
/// Drone.
pub(crate) async fn kill_job<D: Commands>(
    State(served): State<Served<D>>,
    job: Resolved,
) -> Response {
    match served.daemon().kill_job(job.id()).await {
        Ok(job) => answer(StatusCode::OK, &job, served.run_id()),
        Err(refusal) => refused(refusal),
    }
}

/// Go and look at this Job now, and answer with what was found.
///
/// **200 on every answer, including `not_working`.** The examination ran and
/// this is what it found; a status code saying the request failed would make a
/// finding indistinguishable from Fleet being unable to look.
///
/// **A POST, and an act rather than a read.** It writes what it found into the
/// Job's own log, so the answer is on the record beside everything else Fleet
/// did — and it does real work, which is not what a GET promises.
pub(crate) async fn examine_job<D: Commands>(
    State(served): State<Served<D>>,
    job: Resolved,
) -> Response {
    match served.daemon().examine_job(job.id()).await {
        Ok(examined) => answer(StatusCode::OK, &examined, served.run_id()),
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
pub(crate) async fn forget_job<D: Commands>(
    State(served): State<Served<D>>,
    job: Resolved,
) -> Response {
    match served.daemon().forget_job(job.id()).await {
        Ok(forgotten) => answer(StatusCode::OK, &forgotten, served.run_id()),
        Err(refusal) => refused(refusal),
    }
}

/// Give the Job's worktree and branch back. **The record survives** — this is
/// disk, not the row, and `forget_job` is the act that clears the Board.
///
/// 409 where the Job is not yet terminal, for `forget_job`'s reason. 200 where
/// either half was left standing: a branch holding commits the base cannot
/// reach is kept on purpose, and the answer names it rather than failing.
pub(crate) async fn reclaim_worktree<D: Commands>(
    State(served): State<Served<D>>,
    job: Resolved,
) -> Response {
    match served.daemon().reclaim_worktree(job.id()).await {
        Ok(reclaimed) => answer(StatusCode::OK, &reclaimed, served.run_id()),
        Err(refusal) => refused(refusal),
    }
}

/// Mint a replacement for a stopped Job. **Two Jobs come back**, and
/// the answer is 200 rather than 201 because the act a caller asked for is the
/// recovery, not the creation — the new Job's id is in the body.
pub(crate) async fn redispatch_job<D: Commands>(
    State(served): State<Served<D>>,
    job: Resolved,
) -> Response {
    match served.daemon().redispatch_job(job.id()).await {
        Ok(both) => answer(StatusCode::OK, &both, served.run_id()),
        Err(refusal) => refused(refusal),
    }
}

/// Say something to the Drone that is there. **The Job comes back `running`**,
/// at the same step, with the same Drone — nothing was spawned and nothing was
/// thrown away.
///
/// 409 where the Drone is gone, naming `restart_step` as the act that applies.
pub(crate) async fn redirect_drone<D: Commands>(
    State(served): State<Served<D>>,
    job: Resolved,
    body: Bytes,
) -> Response {
    let instruction: Redirection = match ipc::decode("a redirect", &body) {
        Ok(instruction) => instruction,
        Err(why) => return undecodable(&why.to_string(), served.run_id()),
    };
    match served.daemon().redirect_drone(job.id(), instruction).await {
        Ok(job) => answer(StatusCode::OK, &job, served.run_id()),
        Err(refusal) => refused(refusal),
    }
}

/// Answer the question a Drone asked. **The Job comes back unchanged**: what
/// moved is the Drone, handed the answer as a turn. 409 where nothing waits,
/// where the id names an answered question, or where the label was not offered.
pub(crate) async fn answer_question<D: Commands>(
    State(served): State<Served<D>>,
    job: Resolved,
    body: Bytes,
) -> Response {
    let chosen: ChosenAnswer = match ipc::decode("an answer", &body) {
        Ok(chosen) => chosen,
        Err(why) => return undecodable(&why.to_string(), served.run_id()),
    };
    let job_id = job.id();
    match served.daemon().answer_question(job_id, chosen).await {
        Ok(job) => answer(StatusCode::OK, &job, served.run_id()),
        Err(refusal) => refused(refusal),
    }
}
/// Put a new Drone on the worktree the last one left, and say what to do
/// differently. **One Job comes back**, not two — this is the same Job
/// resuming, which is the whole of what makes it different from a redispatch.
///
/// **The body is optional, and no body is the plain restart.** It is the one
/// route here that reads its bytes conditionally, and the condition is that the
/// act existed without them: a restart with nothing to say sends exactly what
/// it sent before this route learned to read anything, so the commonest use of
/// it did not get a step longer. Bytes that arrive are read as a note and a
/// note that will not parse is the transport's 400 like every other body.
///
/// 409 where the Drone is alive, where the worktree is gone, and where a note
/// is already waiting on the Job. 422 on a note with nothing in it — which is
/// not the same request as one with no note.
pub(crate) async fn restart_step<D: Commands>(
    State(served): State<Served<D>>,
    job: Resolved,
    body: Bytes,
) -> Response {
    // Emptiness is read here rather than through an `Option<Json<_>>`
    // extractor, because the question this asks is about the bytes and not
    // about their shape: a caller that sends no body and one that sends a
    // zero-length one have both said the same thing, and both predate this.
    let note: Option<RestartRequested> = if body.is_empty() {
        None
    } else {
        match ipc::decode("a restart note", &body) {
            Ok(note) => Some(note),
            Err(why) => return undecodable(&why.to_string(), served.run_id()),
        }
    };
    match served.daemon().restart_step(job.id(), note).await {
        Ok(job) => answer(StatusCode::OK, &job, served.run_id()),
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
pub(crate) async fn file_report<D: Commands>(
    State(served): State<Served<D>>,
    job: Resolved,
    body: Bytes,
) -> Response {
    let filing: FileReport = match ipc::decode("a report", &body) {
        Ok(filing) => filing,
        Err(why) => return undecodable(&why.to_string(), served.run_id()),
    };
    match served.daemon().file_report(job.id(), filing).await {
        Ok(report) => answer(StatusCode::CREATED, &report, served.run_id()),
        Err(refusal) => refused(refusal),
    }
}
