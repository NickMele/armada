//! The apparatus Overview's claim is asserted against.
//!
//! **Two Manifests, and nothing else the shared bench already gives.**
//! [`Bench::created`] mints every Job under one hard-coded Manifest, because
//! M1 never had a second one to place a Job under — Overview's whole claim is
//! that a person reads two repositories from one list, so this file builds
//! Jobs directly on `core_model::Job::create_top_level` and drives them with
//! `core_model::Job::transition`, the same two calls `bench::Bench` wraps, so
//! that `owner_manifest_id` is a parameter rather than a constant.
//!
//! No worktree is made and no gate is run: every status a tile or a Board
//! section needs — `awaiting_approval`, `queued`, `running`, `completed_success`
//! — is reachable by an unguarded edge of `domain/job-transitions.toml`, so
//! nothing here touches a Drone, a Vcs or a Judge.

use core_model::{
    AcceptanceCriterion, Actor, Facts, Job, JobId, JobNumber, ManifestId, ModelName, NewJob,
    StepSeed, Target, Timestamp, Title, TopLevelOrigin, Ulid, Urgency,
};
use ipc::{
    AnswerCommand, ChosenAnswer, FleetCapacity, FleetHealth, JobDetail, JobList, JobSummary,
    JudgeAnswered, ManifestDrift, QuestionInFlight,
};

use super::bug_workflow_as_far_as_m1_expresses_it;

/// A Job at the approval gate, under the Manifest named.
///
/// **The one call every case below starts from.** `awaiting_approval` is the
/// entry status of `Job::create_top_level`, so a Job that is meant to stay
/// there is simply never transitioned further.
pub fn created_under(manifest_id: &str, ulid: &str, title: &str) -> Job {
    let workflow = bug_workflow_as_far_as_m1_expresses_it();
    let frozen = workflow.frozen();
    let new = NewJob {
        id: JobId::carried(Ulid::carried(ulid.to_string())),
        title: Title::new(title).expect("a title somebody could pick out of a list"),
        workflow: frozen.clone(),
        owner_manifest_id: ManifestId::carried(Ulid::carried(manifest_id.to_string())),
        urgency: Urgency::Normal,
        atomic: false,
        model: ModelName::new("a-model").expect("a model name"),
        acceptance_criteria: Vec::<AcceptanceCriterion>::new(),
        steps: frozen
            .steps()
            .iter()
            .enumerate()
            .map(|(ordinal, step)| StepSeed {
                step_id: step.id().clone(),
                ordinal: ordinal as u32,
            })
            .collect(),
        dependencies: Vec::new(),
        gate_manifests: Vec::new(),
        write_targets: None,
        subject: None,
        redispatched_from: None,
        number: JobNumber::carried(1),
        proposal_id: None,
        facts: Facts::new("an Overview fixture, placed rather than worked"),
        scope_revisions: Vec::new(),
        attachments: Vec::new(),
    };
    Job::create_top_level(new, TopLevelOrigin::Manual, at())
}

/// One legal move, by the actor `domain/job-transitions.toml` names for it.
///
/// **No gate, no ruling** — every edge this file drives is unguarded, so the
/// machine admits the move on the strength of the edge alone. A workflow whose
/// entry edges needed a guard would not belong in this bench; `bug`'s do not.
pub fn moved(job: Job, to: Target, by: Actor) -> Job {
    job.transition(to, by, at())
        .expect("an unguarded edge Overview's fixtures rely on")
        .job
}

/// A Job approved and admitted, with a Drone on it. `queued -> running` is
/// [`edge`](core_model) and unguarded — the frozen workflow's steps stay
/// `not_started`, which is honest: nothing here ever dispatches one.
pub fn running_under(manifest_id: &str, ulid: &str, title: &str) -> Job {
    let job = created_under(manifest_id, ulid, title);
    let job = moved(job, Target::Queued, Actor::Human);
    moved(job, Target::Running, Actor::Fleet)
}

/// A Job approved and left for a Drone, never admitted.
pub fn queued_under(manifest_id: &str, ulid: &str, title: &str) -> Job {
    let job = created_under(manifest_id, ulid, title);
    moved(job, Target::Queued, Actor::Human)
}

/// A Job that is over. `running -> completed_success` is guarded on
/// [`core_model::Guard::EveryStepAdvanced`] and this fixture never advances a
/// step, so this reaches `killed` instead — `running -> killed` is unguarded
/// and terminal, and terminal is the one fact this bench needs from it: proof
/// that Overview's sections leave a Job like this out of all three, not which
/// terminal status it stopped at.
pub fn done_under(manifest_id: &str, ulid: &str, title: &str) -> Job {
    let job = running_under(manifest_id, ulid, title);
    moved(job, Target::Killed, Actor::Human)
}

fn at() -> Timestamp {
    Timestamp::from_rfc3339("2026-09-13T09:00:00.000Z")
}

// ---------------------------------------------------------------------------
// Which of Overview's three drawn sections a row belongs to
// ---------------------------------------------------------------------------

/// Copied from `domain/job-statuses.toml`'s `who_is_acting`/`mode` columns:
/// `core_model` exposes neither to read, so the lists below are hand kept —
/// `overview.rs`'s
/// `every_status_the_registry_names_is_classified_or_excluded_as_terminal` is
/// what catches them drifting from `JobStatus::ALL`.
pub const NEEDS_YOU: &[&str] = &[
    "awaiting_approval",
    "awaiting_attestation",
    "awaiting_repair",
    "awaiting_review",
    "escalated",
];
pub const RUNNING: &[&str] = &["running", "piloted"];
pub const QUEUED: &[&str] = &["queued"];

/// Overview's three drawn sections — Needs you, Running and Queued — read off
/// one row, with **Other named as the section this build never populates.**
/// `asking` is read first because it is not a status at all — `board.ts`
/// reads it the same way, ahead of every status-derived rule.
pub fn section_of(row: &JobSummary) -> Option<&'static str> {
    if row.asking {
        return Some("needs-you");
    }
    let status = row.status.as_wire();
    if NEEDS_YOU.contains(&status) {
        Some("needs-you")
    } else if RUNNING.contains(&status) {
        Some("running")
    } else if QUEUED.contains(&status) {
        Some("queued")
    } else {
        None
    }
}

// ---------------------------------------------------------------------------
// The JobDetail a Job's open question is served on
// ---------------------------------------------------------------------------

/// A `JobDetail` over one bench Job, with no step facts and no classification
/// — everything `board.rs` and `recovery.rs` already assert is out of scope
/// here, so only `asking` is a constructor argument.
pub fn detail_of(job: &Job, asking: Option<QuestionInFlight>) -> JobDetail {
    JobDetail::of(
        job,
        None,
        None,
        None,
        None,
        None,
        None,
        &[],
        None,
        None,
        asking,
        None,
        None,
        None,
        None,
        None,
    )
}

// ---------------------------------------------------------------------------
// The round trip every assertion in `overview.rs` is made through
// ---------------------------------------------------------------------------

/// Written once per type rather than generically: `ipc::encode` and
/// `ipc::decode` already carry the bounds, and naming `serde` here would be a
/// dependency this crate does not otherwise need.
pub fn round_trip_jobs(value: &JobList) -> JobList {
    let body = ipc::encode(value).expect("a list that serialises");
    ipc::decode("a Job list", body.as_bytes()).expect("a list that reads back")
}

pub fn round_trip_capacity(value: &FleetCapacity) -> (FleetCapacity, String) {
    let body = ipc::encode(value).expect("a capacity that serialises");
    let read =
        ipc::decode("a capacity reading", body.as_bytes()).expect("a reading that reads back");
    (read, body)
}

pub fn round_trip_health(value: &FleetHealth) -> FleetHealth {
    let body = ipc::encode(value).expect("a health reading that serialises");
    ipc::decode("a health reading", body.as_bytes()).expect("a reading that reads back")
}

pub fn round_trip_drift(value: &ManifestDrift) -> ManifestDrift {
    let body = ipc::encode(value).expect("a drift reading that serialises");
    ipc::decode("a drift reading", body.as_bytes()).expect("a reading that reads back")
}

pub fn round_trip_detail(value: &JobDetail) -> JobDetail {
    let body = ipc::encode(value).expect("a detail that serialises");
    ipc::decode("a Job detail", body.as_bytes()).expect("a detail that reads back")
}

pub fn round_trip_chosen_answer(value: &ChosenAnswer) -> ChosenAnswer {
    let body = ipc::encode(value).expect("a chosen answer that serialises");
    ipc::decode("a chosen answer", body.as_bytes()).expect("an answer that reads back")
}

pub fn round_trip_answer_command(value: &AnswerCommand) -> AnswerCommand {
    let body = ipc::encode(value).expect("an answered command that serialises");
    ipc::decode("an answered command", body.as_bytes()).expect("an answer that reads back")
}

pub fn round_trip_judge_answered(value: &JudgeAnswered) -> JudgeAnswered {
    let body = ipc::encode(value).expect("a judge answer that serialises");
    ipc::decode("a judge answer", body.as_bytes()).expect("an answer that reads back")
}
