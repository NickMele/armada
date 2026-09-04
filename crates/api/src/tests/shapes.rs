//! The shapes the fake answers with, fixed and built out of `ipc` alone.
//!
//! Split from [`fake`](super::fake) at the 900-line refusal, along the seam
//! that module's header already names: it holds Jobs in a `Vec` and moves them
//! on request, and these are the values it hands back when it does.
//!
//! **This half carries the claim.** Every shape below is constructed without
//! `core-model` and every spelling goes through `from_wire`, so a shape a test
//! can build here is a shape a Bridge can build, and a spelling this can check
//! against the registry is one a client can check against the same registry.
//!
//! Nothing here reads the record and nothing here refuses. A value that had to
//! know which Job was asked for, or whether there was one, belongs on the other
//! side of the seam — which is what keeps these readable as the answer alone.

use ipc::mcp::{CheckRan, CheckReport};
use ipc::{
    Actor, Asked, CallArguments, EvidenceType, Finding, FleetCapacity, Held, Instant, JobDetail,
    JobDiff, JobEvidence, JobExamined, JobHistory, JobId, JobProcess, JobResources, JobStatus,
    JobSummary, Look, ManifestId, ManifestSummary, ModelChoices, Movement, NotedField, Origin,
    ReclaimedBranch, ReclaimedWorktree, Recorded, RunId, StatusMoved, StepId, Submitted, Urgency,
    Work, WorkflowId, WorkflowSummary, WorktreeReclaimed,
};

/// A spelling the registry has. Panics in a test rather than returning an
/// `Option` nobody would handle.
pub fn status(spelling: &str) -> JobStatus {
    JobStatus::from_wire(spelling).expect("a status the registry has")
}

pub fn run_id() -> RunId {
    RunId::carried("01RUN")
}

/// The one call id this fake's record holds. Anything else is a call the
/// transcripts do not carry, which is a different answer from a missing Job.
pub const THE_CALL: &str = "toolu_01Haa";

/// An argument longer than a row carries, so a test can prove the whole of it
/// comes back rather than the line the socket sent.
pub const THE_ARGUMENT: &str = "cat <<'EOF' > notes.md\n  one\n  two\n  three\nEOF";

/// A proposal body, as Bridge would send it.
pub const A_PROPOSAL: &str = r#"{
    "title": "fix the off-by-one in the log reader",
    "workflow_id": "01WF",
    "owner_manifest_id": "01MF",
    "origin": "manual",
    "urgency": "normal",
    "atomic": false,
    "model": "a-model",
    "acceptance_criteria": [{"text": "the symptom is gone", "source": "check"}]
}"#;

/// A check outcome by its registry spelling. **`from_wire` rather than a
/// variant**, because this crate names no domain type — the same reason
/// `origin` and `urgency` below are built this way.
fn outcome(spelling: &str) -> ipc::CheckOutcome {
    ipc::CheckOutcome::from_wire(spelling).expect("a check outcome")
}

/// One step of the rail, declared and not yet run.
///
/// **Built out of `ipc` and nothing else**, which is this module's whole claim:
/// a gate and a judge declaration a Bridge can construct are a gate and a judge
/// declaration a Bridge can read.
fn step_rail(
    step_id: &str,
    ordinal: u32,
    gate: &str,
    judge_checks: Vec<ipc::DeclaredJudge>,
) -> ipc::StepDetail {
    ipc::StepDetail {
        step_id: StepId::carried(step_id),
        label: step_id.to_string(),
        ordinal,
        state: ipc::StepState::from_wire("not_started").expect("a step state the registry has"),
        checks: Some(Vec::new()),
        check_runs: Vec::new(),
        judge_checks: Some(judge_checks),
        advance_gate: Some(ipc::AdvanceGate::from_wire(gate).expect("a gate the registry has")),
        last_verdict: None,
        overridden: false,
        judged: Vec::new(),
        flagged: Vec::new(),
        deliverables: Vec::new(),
        attempts: Vec::new(),
        verdicts: Vec::new(),
        judging: None,
        entered_at: Instant::carried("2026-08-26T09:00:00.000Z"),
        updated_at: Instant::carried("2026-08-26T09:00:00.000Z"),
    }
}

/// A Job parked at any status the registry has, without a transition to get it
/// there. What a fake is for: the transport is the thing under test.
pub fn job_at(id: &str, spelling: &str) -> JobSummary {
    JobSummary {
        id: JobId::carried(id),
        title: format!("a Job called {id}"),
        status: status(spelling),
        created_at: Instant::carried("2026-01-01T00:00:00.000Z"),
        branch: Some(format!("armada/{id}")),
        reason: None,
        queued_reason: None,
        resumption: None,
        workflow_id: WorkflowId::carried("01WF"),
        owner_manifest_id: ManifestId::carried("01MF"),
        origin: Origin::from_wire("manual").expect("an origin"),
        urgency: Urgency::from_wire("normal").expect("an urgency"),
        atomic: false,
        model: "a-model".to_string(),
        current_step_id: None,
        assigned_drone: None,
        redispatched_from: None,
        asking: false,
        landed: None,
    }
}

/// A fleet of two, one place taken, and the disk short.
///
/// **Fixed, and built through `ipc::AdmissionHold::from_wire`** — which is this
/// module's claim one type further on: a spelling a client can check against
/// the registry is a spelling a Bridge can check. The value exercises the shape
/// a caller has to handle, which is all three fields present at once.
pub fn capacity() -> FleetCapacity {
    FleetCapacity {
        bound: 2,
        occupied: 1,
        held_by: ipc::AdmissionHold::from_wire("disk"),
    }
}

/// A refused re-read: the shape with the most in it, because a refusal carries
/// what an adoption does not.
pub fn manifest_reading() -> ipc::ManifestReading {
    ipc::ManifestReading {
        path: "armada.yml".to_string(),
        at: ipc::Instant::carried("2026-09-04T09:00:00.000Z"),
        moved: Vec::new(),
        at_restart: Vec::new(),
        refused: Some(ipc::ManifestRefused {
            summary: "armada.yml was refused; `drone.poke_limit` is not a number".to_string(),
            faults: vec![ipc::ManifestFault {
                key: "drone.poke_limit".to_string(),
                fault: "is not a number".to_string(),
            }],
        }),
    }
}

/// One Job, with nothing the fake does not hold — except a step rail, which it
/// holds because the rail is the shape a client draws a Job from.
///
/// **Two steps, and they are the pair the rail turns on**: one the Judge gates,
/// one a person does. Every other list is empty and every other option absent,
/// because that daemon holds `JobSummary` and nothing beneath it, and the real
/// values are asserted against a real Fleet in `fleet`'s own suite.
pub fn detail(job: JobSummary) -> JobDetail {
    JobDetail {
        spend: None,
        delivery: None,
        job,
        created_at: Instant::carried("2026-08-26T09:00:00.000Z"),
        branch: None,
        steps: vec![
            step_rail(
                "implement",
                0,
                "auto_if_judge_passes",
                vec![ipc::DeclaredJudge {
                    criteria: 2,
                    panel_size: Some(3),
                    gaming_check: true,
                }],
            ),
            step_rail("handoff", 1, "human_always", Vec::new()),
        ],
        acceptance_criteria: Vec::new(),
        facts: None,
        write_targets: None,
        subject: None,
        dependencies: Vec::new(),
        // Absent, like every other option here: this daemon reads no store
        // and a footprint is a store's answer.
        footprint: None,
        // Absent for the same reason, one layer further in: an outstanding
        // redirect is read from a working slot, and this daemon holds none.
        redirecting: None,
        // Absent for the redirect's reason exactly: an unanswered question
        // lives on a working slot, and this daemon holds none.
        asking: None,
        // Absent again: a waiting note is a column on `jobs`, and this
        // daemon's Jobs are wire summaries rather than records.
        redirect_waiting: None,
        // Absent, not empty: no other Job's record is held, so nothing looked.
        write_scope_overlaps: None,
        // Absent for the same reason again: a classification is read from
        // a slot, a filesystem and a store, and this daemon has none of the
        // three.
        stuck: None,
    }
}

/// The one move the fake records: the Job reaching the status it is at.
///
/// **Not a fold and not a machine.** `fake`'s header comment holds for this too
/// — it asserts nothing about either machine, and what it answers is the shape
/// rather than a real history.
pub fn history(job_id: JobId, at: JobStatus) -> JobHistory {
    JobHistory {
        job_id,
        moves: vec![Recorded {
            seq: 1,
            status: status("awaiting_approval"),
            moved: Movement::Status(StatusMoved {
                to: at,
                reason: None,
            }),
            actor: Actor::from_wire("human").expect("an actor the envelope has"),
            at: Instant::carried("2026-08-26T09:00:00.000Z"),
        }],
    }
}

/// One step's evidence. **A step that submitted nothing is absent** rather than
/// blank, which is the shape the store answers in and the one the fake is here
/// to prove crosses.
pub fn evidence(job_id: JobId) -> JobEvidence {
    JobEvidence {
        job_id,
        steps: vec![Submitted {
            step_id: StepId::carried("implement"),
            evidence_type: EvidenceType::from_wire("diff").expect("a type the registry has"),
            claimed: "the log reader stops one line early".to_string(),
            shown_by: "a failing test that now passes".to_string(),
            // Absent, not blank. The rule the whole DTO turns on.
            not_claimed: None,
        }],
    }
}

/// A reading with a patch in it. **Present because there is something in it** —
/// the absent case is a Job with no worktree, and the fake has no worktrees to
/// be absent.
pub fn diff(job_id: JobId) -> JobDiff {
    JobDiff {
        job_id,
        work: Some(Work {
            files: vec![ipc::ChangedFile {
                path: "crates/store/src/read.rs".to_string(),
                change: ipc::ChangeKind::Modified,
                outside_plan: false,
            }],
            plan_declared: false,
            patch: Some(
                "--- a/crates/store/src/read.rs\n+++ b/crates/store/src/read.rs\n".to_string(),
            ),
        }),
    }
}

/// What one Job holds on the machine. **A reading with something in it** —
/// the fake spawns nothing, so what it proves is the shape a panel draws from
/// and not any figure.
pub fn resources(job_id: JobId) -> JobResources {
    JobResources {
        job_id,
        read_at: Instant::carried("2026-09-04T04:07:00.366Z"),
        held: Held::Running,
        processes: vec![JobProcess {
            pid: 41233,
            command: "node".to_string(),
            cpu_percent: 12.4,
            memory_bytes: 402_653_184,
            running_for: "06:12".to_string(),
            recorded: true,
        }],
        worktree: Some(ipc::WorktreeOnDisk {
            path: "/repo/.armada/worktrees/01JOB".to_string(),
            branch: "armada/01JOB".to_string(),
            bytes: Some(1_073_741_824),
        }),
        wrote_last_at: Some(Instant::carried("2026-09-04T04:06:12.001Z")),
    }
}

/// What Fleet found when somebody asked it to look. **One look that could not
/// tell**, so the whole answer is `cannot_tell` — which is the rule the DTO
/// exists to make visible.
pub fn examined(job_id: JobId) -> JobExamined {
    JobExamined {
        job_id: job_id.clone(),
        looked_at: Instant::carried("2026-09-04T04:07:00.366Z"),
        found: Finding::CannotTell,
        looks: vec![
            Look {
                asked: Asked::Process,
                found: Finding::Working,
                said: "the process Fleet recorded is running".to_string(),
                fields: vec![NotedField {
                    name: "pid".to_string(),
                    value: "41233".to_string(),
                }],
            },
            Look {
                asked: Asked::Writing,
                found: Finding::CannotTell,
                said: "nothing has been written to this Job's log lately, which settles nothing on its own"
                    .to_string(),
                fields: vec![],
            },
        ],
        resources: resources(job_id),
    }
}

/// One Job's disk, given back. **The checkout goes and the branch is kept**,
/// which is the shape a caller most needs to be able to read: `UnmergedWork`
/// on this seam is always `Keep`, so a branch nothing has merged surviving its
/// own worktree is the ordinary answer and not the exceptional one.
///
/// The fake has no repository, so this asserts nothing about git. What it
/// carries is the two halves disagreeing, which is what the transport has to
/// be able to say.
pub fn reclaimed(job_id: JobId) -> WorktreeReclaimed {
    let branch = format!("armada/{}", job_id.as_str());
    WorktreeReclaimed {
        job_id,
        worktree: ReclaimedWorktree {
            path: format!(".armada/worktrees/{branch}"),
            removed: true,
            why: None,
        },
        branch: ReclaimedBranch {
            branch,
            deleted: false,
            tip: Some("0f1e2d3c4b5a69788796a5b4c3d2e1f001234567".to_string()),
            why: Some("main cannot reach 3 of its commits".to_string()),
            base: Some("main".to_string()),
            unmerged_commits: Some(3),
        },
    }
}

/// One call's arguments, whole. **The fake holds exactly one**, keyed by
/// [`THE_CALL`]: what the route has to prove is that the argument comes back at
/// its full size.
pub fn call(call_id: String) -> CallArguments {
    CallArguments {
        tool: "Bash".to_string(),
        call: call_id,
        arguments: THE_ARGUMENT.to_string(),
        whole: true,
        length: Some(THE_ARGUMENT.chars().count()),
    }
}

/// The one workflow the fake holds. A list, because the operation is one.
pub fn workflows() -> Vec<WorkflowSummary> {
    vec![WorkflowSummary {
        id: WorkflowId::carried("01WF"),
        name: "a-workflow".to_string(),
        version: 1,
        steps: vec![
            // Gated, and ungated. The pair is the distinction [`detail`]'s rail
            // turns on, so the fake carries both rather than one — and the
            // second is the step that stops for a person, which a preview drew
            // as a step with nothing on it.
            ipc::WorkflowStep {
                step_id: StepId::carried("implement"),
                label: "Implement the change".to_string(),
                checks: vec![ipc::DeclaredCheck {
                    kind: "manifest_check".to_string(),
                    name: Some("build".to_string()),
                    run: Some("cargo build --workspace --locked".to_string()),
                    expect_exit_code: Some(0),
                    // Covering everything, which is most Checks. The scoped
                    // case is asserted in `ipc`'s own round-trip.
                    when: None,
                }],
                judge_checks: vec![ipc::DeclaredJudge {
                    criteria: 2,
                    panel_size: None,
                    gaming_check: false,
                }],
                advance_gate: ipc::AdvanceGate::from_wire("auto_if_judge_passes")
                    .expect("a gate the registry has"),
            },
            ipc::WorkflowStep {
                step_id: StepId::carried("handoff"),
                label: "Hand the work back".to_string(),
                checks: Vec::new(),
                judge_checks: Vec::new(),
                advance_gate: ipc::AdvanceGate::from_wire("human_always")
                    .expect("a gate the registry has"),
            },
        ],
        manifest_id: ManifestId::carried("01MF"),
    }]
}

/// The one Manifest the fake holds.
pub fn manifests() -> Vec<ManifestSummary> {
    vec![ManifestSummary {
        id: ManifestId::carried("01MF"),
        repository: "a-repository".to_string(),
        path: "/a-repository/armada.yml".to_string(),
        version: 1,
        checks: vec!["build".to_string()],
    }]
}

/// Two models and a default, which is the shape a picker draws from.
pub fn models() -> ModelChoices {
    ModelChoices {
        models: vec!["a-model".to_string(), "another-model".to_string()],
        default: "a-model".to_string(),
    }
}

/// One passing Check and one failing one, so a test over the router can tell
/// that a report reaches the Drone **and** that a failure in it is still a
/// successful tool call.
pub fn check_report() -> CheckReport {
    CheckReport {
        ran: vec![
            CheckRan {
                name: "fmt".to_string(),
                outcome: outcome("passed"),
                detail: None,
                took: std::time::Duration::from_millis(300),
                log: Some(".armada/checks/a-job/implement.dry.0.log".to_string()),
            },
            CheckRan {
                name: "tests".to_string(),
                outcome: outcome("failed"),
                detail: Some("exit code 101, expected 0".to_string()),
                took: std::time::Duration::from_secs(12),
                log: Some(".armada/checks/a-job/implement.dry.1.log".to_string()),
            },
        ],
    }
}
