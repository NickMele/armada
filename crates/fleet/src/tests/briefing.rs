//! What a Drone is told, and the one thing it is never told.
//!
//! The wording is the Agent Prompt Contract's and is not asserted line by line
//! here — a test that pinned the copy would make every contract edit a Rust
//! change. What is asserted is the structure the contract's M1 rendering
//! requires, and the rule `docs/concepts/drone.md` puts on every Drone-facing
//! surface: **a Drone is never told what the Checks are.**
//!
//! The last group is about a moment rather than a rendering: a step advancing
//! under a Drone that is already running. It drives a whole Fleet, because the
//! defect it is about was not in any block's text — every block was right, and
//! the turn that carries them was never sent.

use std::time::Duration;

use adapter_traits::DroneEvent;
use core_model::{
    AcceptanceCriterion, CriterionId, CriterionSource, Facts, Job, JobId, ManifestId, ModelName,
    NewJob, StepId, StepSeed, Timestamp, TopLevelOrigin, Ulid, Urgency,
};
use ipc::mcp::DeclareScope;
use testkit::{FakeHarness, FakeVcs, FakeWorkProduct, Gate, Scoped, Sketch};
use verification::{Claimed, NotClaimed, ShownBy};

use crate::briefing::{first_turn, BASELINE};
use crate::daemon::Fleet;
use crate::evidence::Call;
use crate::gate::Ruling;
use crate::tests::daemon::{a_fleet_holding, a_proposal, worktree_directory};
use crate::tests::tmp::TempDir;

fn a_job() -> Job {
    Job::create_top_level(
        NewJob {
            id: JobId::carried(Ulid::carried("01TEST00000000000000000001")),
            title: core_model::Title::new("fix the off-by-one").expect("a title"),
            workflow: a_workflow(),
            owner_manifest_id: ManifestId::carried(Ulid::carried("01MF")),
            urgency: Urgency::Normal,
            atomic: false,
            model: ModelName::new("a-model").expect("a model name"),
            acceptance_criteria: vec![AcceptanceCriterion {
                criterion_id: CriterionId::new("c1"),
                text: "the log reader stops one line later".into(),
                source: CriterionSource::Check,
            }],
            steps: vec![
                StepSeed {
                    step_id: StepId::new("implement"),
                    ordinal: 0,
                },
                StepSeed {
                    step_id: StepId::new("summarise"),
                    ordinal: 1,
                },
            ],
            dependencies: Vec::new(),
            gate_manifests: Vec::new(),
            write_targets: None,
            subject: None,
            redispatched_from: None,
            facts: Facts::new("the reader is off by one"),
            scope_revisions: Vec::new(),
            attachments: Vec::new(),
        },
        TopLevelOrigin::Manual,
        Timestamp::from_rfc3339("2026-08-26T09:00:00.000Z"),
    )
}

fn a_workflow() -> core_model::FrozenWorkflow {
    testkit::frozen(&[
        Sketch {
            id: "implement",
            label: "Implement",
            evidence_type: Some("diff"),
            gates: &[Gate::Check {
                name: "suite",
                run: "cargo nextest run --workspace",
                expect_exit_code: 0,
            }],
            judged_on: &[],
            scope: None,
            gaming: None,
        },
        Sketch {
            id: "summarise",
            label: "Summarise",
            evidence_type: Some("facts_note"),
            gates: &[],
            judged_on: &[],
            scope: None,
            gaming: None,
        },
    ])
}

fn turn_at(step: &str) -> String {
    first_turn(&a_job(), &a_workflow(), &StepId::new(step))
        .expect("a prompt")
        .as_str()
        .to_string()
}

/// **The measured one.** Given a tool and a task and told nothing about
/// reporting, a Drone fixed the code and wrote a tidy sentence saying so, four
/// times out of four. The reporting clause is the difference between a working
/// gate and a Job that stalls on every step.
#[test]
fn every_turn_carries_the_reporting_clause() {
    for step in ["implement", "summarise"] {
        assert!(turn_at(step).contains(BASELINE), "layer 1 is a constant");
    }
    assert!(BASELINE.contains("evidence submission tool"));
    assert!(BASELINE.contains("recorded"));
}

/// The tool is **described, not named**: the MCP tool's own description carries
/// its name, so the prompt and the tool cannot drift apart.
#[test]
fn the_baseline_describes_the_tool_rather_than_naming_it() {
    assert!(!BASELINE.contains("mcp__"));
    assert!(!BASELINE.to_lowercase().contains("submit_evidence"));
}

/// The rule that governs every Drone-facing surface. A Drone told the Check
/// satisfies the Check.
#[test]
fn a_drone_is_never_told_what_the_checks_are() {
    let said = turn_at("implement");
    assert!(!said.contains("cargo nextest run"), "the command");
    assert!(!said.contains("suite"), "the Check's name");
    assert!(!said.contains("exit"), "what it is expected to exit with");
}

/// The stop sits inside the list, and later parts carry the prohibition. Where
/// the line falls **is** the boundary.
#[test]
fn the_stop_falls_where_the_step_is() {
    let said = turn_at("implement");
    let stop = said.find("STOP").expect("a stop");
    let later = said.find("Summarise").expect("the later part");
    assert!(stop < later, "later parts are below the line");
    assert!(said.contains("not yours"));

    let last = turn_at("summarise");
    assert!(last.contains("Summarise"));
    assert!(
        last.contains("2. Summarise — you are here"),
        "the rail says where it is: {last}"
    );
}

/// Armada's vocabulary is not taught. "Parts", not "steps".
#[test]
fn the_rail_says_parts_rather_than_steps() {
    let said = turn_at("implement");
    assert!(said.contains("This task runs in 2 parts."));
}

/// The requester's words reach the Drone: what the Job is, and what done means.
#[test]
fn the_brief_carries_the_facts_and_the_criteria() {
    let said = turn_at("implement");
    assert!(said.contains("fix the off-by-one"));
    assert!(said.contains("the reader is off by one"));
    assert!(said.contains("the log reader stops one line later"));
}

/// A step that asks for no scope gets no block, because there is no call it
/// could make — telling every Drone about a tool most of them cannot use is how
/// an instruction stops being read.
#[test]
fn a_step_with_no_scope_is_told_nothing_about_declaring_one() {
    for step in ["implement", "summarise"] {
        assert!(!turn_at(step).contains("BEFORE YOU START"));
    }
}

/// A step that asks for one says so, and says what a plan that turned out wrong
/// is fixed by. **The obligation is in the prompt** rather than only in the
/// tool description, for the reason the reporting clause is: spike 6 measured
/// that a description alone does not make a Drone call a tool.
#[test]
fn a_scoped_step_is_told_to_declare_before_it_starts() {
    let workflow = testkit::frozen(&[Sketch {
        id: "implement",
        label: "Implement",
        evidence_type: Some("diff"),
        gates: &[],
        judged_on: &[],
        scope: Some(testkit::Scoped {
            diff_check: true,
            at_step_start: true,
            exclude: &["secrets"],
        }),
        gaming: None,
    }]);
    let said = first_turn(&a_job(), &workflow, &StepId::new("implement"))
        .expect("a prompt")
        .as_str()
        .to_string();

    assert!(said.contains("BEFORE YOU START"));
    assert!(
        said.contains("call the tool"),
        "a wrong plan has a way out: {said}"
    );
    assert!(said.contains("secrets"), "the denylist is named: {said}");
    assert!(
        !said.contains("mcp__") && !said.to_lowercase().contains("declare_scope"),
        "described rather than named, like the Evidence tool: {said}"
    );
}

// ------------------------------------------------------ the step boundary

/// The two-step Job the boundary cases run: the first step scoped, the second
/// one scoped or not according to the argument.
fn plan_then_do(second_is_scoped: bool) -> config::ResolvedWorkflow {
    let scoped = Some(Scoped {
        diff_check: true,
        at_step_start: true,
        exclude: &[],
    });
    testkit::resolved(&[
        Sketch {
            id: "plan",
            label: "Plan",
            evidence_type: Some("diff"),
            gates: &[],
            judged_on: &[],
            scope: scoped,
            gaming: None,
        },
        Sketch {
            id: "implement",
            label: "Implement",
            evidence_type: Some("diff"),
            gates: &[],
            judged_on: &[],
            scope: second_is_scoped.then_some(scoped).flatten(),
            gaming: None,
        },
    ])
}

fn a_diff_call<'a>() -> Call<'a> {
    Call {
        evidence_type: config::EvidenceType::Diff,
        claimed: Claimed("The plan is written."),
        shown_by: ShownBy("docs/plan.md"),
        not_claimed: NotClaimed(""),
    }
}

/// Every turn Fleet has written down the pipe, as the Drone echoed it back.
///
/// The fake Drone is `/bin/cat`, so a turn Fleet sends comes back as a line of
/// transcript — which is the only way to read an injected turn from outside the
/// process that sent it. It comes back on the reader's own task, so this waits
/// for `turns` of them rather than reading once and hoping.
async fn turns_sent(
    fleet: &Fleet<FakeHarness, FakeVcs, FakeWorkProduct>,
    turns: usize,
) -> Vec<String> {
    for _ in 0..600 {
        let echoed: Vec<String> = {
            let slot = fleet.slot().lock().await;
            slot.as_ref()
                .map(|at_work| {
                    at_work
                        .heard()
                        .into_iter()
                        .filter_map(|event| match event {
                            DroneEvent::Said { text } => Some(text),
                            _ => None,
                        })
                        .collect()
                })
                .unwrap_or_default()
        };
        if echoed.len() >= turns {
            return echoed;
        }
        tokio::time::sleep(Duration::from_millis(5)).await;
    }
    panic!("the Drone never echoed {turns} turns back");
}

/// Drive a Job through its first step and answer with what its Drone was told.
async fn told_across_the_boundary(
    home: &TempDir,
    second_is_scoped: bool,
) -> (Fleet<FakeHarness, FakeVcs, FakeWorkProduct>, Vec<String>) {
    let fleet = a_fleet_holding(
        home,
        FakeWorkProduct::changed(&["docs/plan.md"]),
        plan_then_do(second_is_scoped),
        1,
    );
    let job = fleet
        .propose(a_proposal("plan then do"))
        .await
        .expect("a proposal");
    worktree_directory(home, job.id());
    fleet.approve(job.id()).await.expect("it is approved");
    fleet
        .declare_scope(&DeclareScope {
            context_paths: vec!["docs".to_string()],
        })
        .await
        .expect("the first step's plan");
    fleet
        .submit_evidence(a_diff_call())
        .await
        .expect("evidence lands");
    let turned = fleet.turn().await.expect("a turn");
    assert!(
        matches!(turned.ruled, Some(Ruling::Advanced { .. })),
        "the first step advanced: {:?}",
        turned.ruled
    );
    let sent = turns_sent(&fleet, 2).await;
    (fleet, sent)
}

/// **The one that cost twenty-two minutes of correct work.**
///
/// `Working::now_on` clears the declaration at the boundary, which is right: a
/// plan inherited from the step before is not a plan. Nothing said so. The
/// Drone declared once, on the step that asked at spawn, worked the next step
/// for sixty-eight turns and failed `evidence_scope` on a call nobody had
/// requested. The ask belongs on every boundary that clears one.
#[tokio::test]
async fn a_step_boundary_asks_again_for_the_declaration_it_just_cleared() {
    let home = TempDir::new();
    let (_fleet, sent) = told_across_the_boundary(&home, true).await;

    assert!(
        sent[1].contains("BEFORE YOU START"),
        "the boundary turn asks for the next part's plan: {}",
        sent[1]
    );
    assert!(
        sent[1].contains("scope tool"),
        "described rather than named, as the first turn describes it: {}",
        sent[1]
    );
    assert!(
        sent[1].contains("does not carry over"),
        "and says why it is being asked again: {}",
        sent[1]
    );
}

/// **The cold switch, at the boundary this time.** A step with no evidence
/// scope is told exactly what it was told before any of this existed, so the
/// turn that moves a Drone on to one is the outcome and nothing else.
#[tokio::test]
async fn a_step_boundary_says_nothing_where_the_next_step_wants_no_plan() {
    let home = TempDir::new();
    let (_fleet, sent) = told_across_the_boundary(&home, false).await;

    assert!(
        !sent[1].contains("BEFORE YOU START"),
        "no tool is put in front of a Drone that has nothing to declare: {}",
        sent[1]
    );
    assert!(
        sent[1].contains("Implement"),
        "it is still told where it is going: {}",
        sent[1]
    );
}
