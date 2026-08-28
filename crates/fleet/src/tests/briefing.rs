//! What a Drone is told, and the one thing it is never told.
//!
//! The wording is the Agent Prompt Contract's and is not asserted line by line
//! here — a test that pinned the copy would make every contract edit a Rust
//! change. What is asserted is the structure the contract's M1 rendering
//! requires, and the rule `docs/concepts/drone.md` puts on every Drone-facing
//! surface: **a Drone is never told what the Checks are.**

use core_model::{
    AcceptanceCriterion, CriterionId, CriterionSource, Facts, Job, JobId, ManifestId, ModelName,
    NewJob, StepId, StepSeed, Timestamp, TopLevelOrigin, Ulid, Urgency,
};
use testkit::{Gate, Sketch};

use crate::briefing::{first_turn, BASELINE};

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
            references: &[],
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
