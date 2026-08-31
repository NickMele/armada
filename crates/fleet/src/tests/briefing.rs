//! What a Drone is told, and the one thing it is never told.
//!
//! The wording is the Agent Prompt Contract's and is not asserted line by line
//! here — a test that pinned the copy would make every contract edit a Rust
//! change. What is asserted is the structure the contract's M1 rendering
//! requires, and the rule `docs/concepts/drone.md` puts on every Drone-facing
//! surface: **a Drone is never told what the Checks are.**
//!
//! The last two groups are about moments rather than renderings. A step
//! boundary drives a whole Fleet, because the defect it is about was not in any
//! block's text — every block was right and the turn carrying them was never
//! sent. A restart asserts the opposite failure: **the sentence has to be the
//! one the record supports**, because a Drone told its work did not pass will
//! change work that was correct.

use std::time::Duration;

use adapter_traits::DroneEvent;
use core_model::{
    AcceptanceCriterion, CriterionId, CriterionSource, EscalationTrigger, Facts, Job, JobId,
    JudgeVerdict, Judgment, ManifestId, ModelName, NewJob, RepoPath, StepId, StepLevelTrigger,
    StepSeed, StepVerdict, Timestamp, TopLevelOrigin, Ulid, Urgency,
};
use ipc::mcp::DeclareScope;
use testkit::{FakeHarness, FakeVcs, FakeWorkProduct, Gate, Scoped, Sketch};
use verification::{Claimed, NotClaimed, ShownBy, TheBaseMoved};

use crate::briefing::{first_turn, resuming_turn, Opening, Redeclaring, Stopped, BASELINE};
use crate::crossing::Crossed;
use crate::daemon::Fleet;
use crate::evidence::Call;
use crate::gate::Ruling;
use crate::tests::daemon::{a_fleet_holding, a_proposal, worktree_directory};
use crate::tests::tmp::TempDir;

/// **`pub(super)` so `dry_run` can use it**: the briefing block that offers the
/// dry run is assembled here, and a second Job fixture would be a second answer
/// to what a Job is.
pub(super) fn a_job() -> Job {
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
                when: &[],
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
    first_turn(
        &a_job(),
        &a_workflow(),
        &StepId::new(step),
        &Crossed::nothing(),
    )
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

/// **The promise is conditional, and the condition is the ruling that carries
/// a message.** `Ruling::HandedBack` has a `tell`; `Refused`, `Suspect` and
/// `CouldNotDecide` have no message field, and an advancing step ends its
/// Drone too. Promised unconditionally, a refused Drone waits for a turn
/// nothing sends — nineteen hours on one Job, twice.
///
/// **Both halves are asserted**, because "you may not hear back" satisfies the
/// second alone and still leaves a Drone unable to tell waiting from stalling.
/// The last assertion holds the promise to an attempt it does not number: a
/// Drone shown the arithmetic has reason to satisfy it rather than work.
#[test]
fn the_baseline_promises_a_turn_only_where_the_part_comes_back() {
    assert!(!BASELINE.contains("Wait for that turn"), "{BASELINE}");
    assert!(
        BASELINE.contains("A later turn comes only where the part is coming back to you"),
        "the outcome that does send one"
    );
    assert!(
        BASELINE.contains("Every other outcome ends your part where it stands"),
        "the outcomes that do not"
    );
    for counted in ["attempts left", "attempt of", "last try", "retry"] {
        let said = BASELINE.to_lowercase();
        assert!(!said.contains(counted), "the baseline counts: {counted}");
    }
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

/// **Every other per-Job artifact is keyed by Job id and what a Drone writes
/// for itself now is too.** A file at the repository root is one slot every
/// worktree shares, and a plan left in it has no way to say it is finished
/// with — which is what `PLAN.md` was until it was deleted. The path answers
/// "is this mine" without anything having to read the file.
///
/// On the resuming turn as well: a restarted Drone is the one with no history
/// at all, and it writes the same notes as any other.
#[test]
fn every_turn_says_where_a_drones_own_files_go() {
    let job = a_job();
    let keyed = format!(".armada/{}/", job.id().as_str());
    for step in ["implement", "summarise"] {
        let said = turn_at(step);
        assert!(said.contains(&keyed), "the Job's own path: {said}");
    }

    let resumed = resuming_turn(
        &job,
        &a_workflow(),
        &StepId::new("implement"),
        &Stopped::default(),
        &Crossed::nothing(),
    )
    .expect("a prompt");
    assert!(resumed.as_str().contains(&keyed), "{}", resumed.as_str());
}

/// Two Jobs, two paths, and neither of them the root. **The staleness question
/// disappears rather than being answered** — there is no shared slot for a
/// finished Job's plan to sit in, so nothing has to mark one stale.
#[test]
fn what_a_drone_writes_for_itself_cannot_land_where_another_job_would_read_it() {
    let mine = turn_at("implement");
    assert!(
        !mine.contains("PLAN.md"),
        "nothing points a Drone back at the root slot: {mine}"
    );

    let other = ".armada/01TEST00000000000000000002/";
    assert!(
        !mine.contains(other),
        "one Job's turn names one Job's path: {mine}"
    );
}

/// It offers a place; it does not ask for a plan. A block read as an
/// instruction would put every Job through a planning step nobody requested,
/// and the cost lands on the Jobs that had nothing to write down.
#[test]
fn where_a_file_goes_is_not_an_instruction_to_write_one() {
    let said = turn_at("summarise");
    assert!(
        said.contains("Nothing here is asking you to write"),
        "{said}"
    );
    assert!(
        !said.contains("Write a plan") && !said.contains("write a plan before"),
        "no imperative: {said}"
    );
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
    let said = first_turn(
        &a_job(),
        &workflow,
        &StepId::new("implement"),
        &Crossed::nothing(),
    )
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

// --------------------------------------------- what a drifting Drone is told

/// One step, watched or not, for the drift notice to be built from.
fn watching(live: bool) -> config::ResolvedWorkflow {
    testkit::resolved(&[Sketch {
        id: "implement",
        label: "Implement",
        evidence_type: Some("diff"),
        gates: &[],
        judged_on: &[],
        scope: Some(Scoped {
            diff_check: true,
            at_step_start: live,
            exclude: &[],
            references: &[],
        }),
        gaming: None,
    }])
}

fn drift_notice(workflow: &config::ResolvedWorkflow, paths: &[&str]) -> Option<Redeclaring> {
    Redeclaring::at(
        workflow.steps().first().expect("a first step"),
        &paths
            .iter()
            .map(|path| RepoPath::new(*path))
            .collect::<Vec<RepoPath>>(),
    )
}

/// **The mechanism nothing could reach.** Drift has been compared against the
/// plan on every turn since the scope tool existed, and every finding went to
/// the Job's log — which no Drone reads. The notice names the file and names
/// the call, because "you edited outside your scope" does not make re-declaring
/// obvious to anything reading it.
#[test]
fn a_drone_that_drifted_is_told_which_file_and_which_call() {
    let workflow = watching(true);
    let notice = drift_notice(&workflow, &["crates/ipc/src/lib.rs"]).expect("a watched step");

    let said = notice.text();
    assert!(said.contains("crates/ipc/src/lib.rs"), "{said}");
    assert!(
        said.contains("call the scope tool again"),
        "the call that fixes it, not just the finding: {said}"
    );
    assert!(
        !said.contains("mcp__") && !said.to_lowercase().contains("declare_scope"),
        "described rather than named, like every other tool: {said}"
    );
}

/// **Not an accusation and not a stop-work order.** Drift is a signal because
/// investigation legitimately moves the work, so a Drone that reads this and
/// carries on has done nothing wrong. A notice that read like the thrashing
/// directive would have a Drone down tools over a file it was right to touch.
#[test]
fn the_drift_notice_asks_for_nothing_but_the_call() {
    let workflow = watching(true);
    let notice = drift_notice(&workflow, &["src/lib.rs"]).expect("a watched step");

    let said = notice.text();
    assert!(said.contains("Nothing has failed"), "{said}");
    assert!(said.contains("not being asked to stop"), "{said}");
    assert!(
        !said.contains("Stop and report"),
        "that is the thrashing directive, and this is not it: {said}"
    );
    assert!(
        !said.contains("failed to") && !said.contains("should have"),
        "nothing here is put to the Drone as a fault: {said}"
    );
}

/// The cold switch, on the block this time. A step whose plan is measured only
/// at the gate has no live plan to correct, and a Drone told to call a tool it
/// was never asked to call goes looking for one.
#[test]
fn a_step_whose_edits_are_not_watched_has_no_drift_notice_to_send() {
    assert_eq!(drift_notice(&watching(false), &["src/lib.rs"]), None);
}

/// Nothing new is nothing to say. The once-per-path rule is
/// `Working::drifting`'s and this rides it rather than keeping a second memory
/// of what a Drone has already been told.
#[test]
fn nothing_seen_outside_the_plan_is_nothing_to_send() {
    assert_eq!(drift_notice(&watching(true), &[]), None);
}

// ------------------------------------------------------ the step boundary

/// The two-step Job the boundary cases run: the first step scoped, the second
/// one scoped or not according to the argument.
fn plan_then_do(second_is_scoped: bool) -> config::ResolvedWorkflow {
    let scoped = Some(Scoped {
        diff_check: true,
        at_step_start: true,
        exclude: &[],
        references: &[],
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

pub(super) fn a_diff_call<'a>() -> Call<'a> {
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
pub(crate) async fn turns_sent(
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
///
/// `submitted` is part one's evidence, because part two's opening brief is
/// assembled from it: the boundary reads the record `crate::settling` wrote and
/// hands what it finds to the Drone it starts.
pub(super) async fn told_across_the_boundary(
    home: &TempDir,
    second_is_scoped: bool,
    submitted: Call<'_>,
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
        .submit_evidence(submitted)
        .await
        .expect("evidence lands");
    let turned = fleet.turn().await.expect("a turn");
    assert!(
        matches!(turned.ruled, Some(Ruling::Advanced { .. })),
        "the first step advanced: {:?}",
        turned.ruled
    );
    // **The slot holds a different Drone now.** The boundary ended the one that
    // worked part one and put a fresh one on part two, so what part two's Drone
    // was told is its *opening* turn rather than a second turn injected into a
    // session — and its transcript starts empty. One echo, not two.
    let sent = turns_sent(&fleet, 1).await;
    (fleet, sent)
}

/// **The one that cost twenty-two minutes of correct work.**
///
/// A declaration is about one part, so it does not cross a boundary. Nothing
/// said so: the Drone declared once, on the part that asked at spawn, worked
/// the next part for sixty-eight turns and failed `evidence_scope` on a call
/// nobody had requested. The ask belongs on every boundary.
///
/// **It is answered differently now, and the claim is unchanged.** It used to
/// be an injected turn, because the process carried on and the declaration it
/// was holding had just been cleared underneath it. The process does not carry
/// on: the ask is in the opening brief of the Drone that starts part two, which
/// is where every other thing a Drone is told before it starts already was.
#[tokio::test]
async fn a_step_boundary_asks_again_for_the_declaration_it_just_cleared() {
    let home = TempDir::new();
    let (_fleet, sent) = told_across_the_boundary(&home, true, a_diff_call()).await;

    assert!(
        sent[0].contains("BEFORE YOU START"),
        "the next part's Drone is asked for its plan: {}",
        sent[0]
    );
    assert!(
        sent[0].contains("scope tool"),
        "described rather than named, as the first turn describes it: {}",
        sent[0]
    );
    assert!(
        sent[0].contains("does not carry over"),
        "and says why it is being asked again: {}",
        sent[0]
    );
    // And it is a Drone that was not there for part one, so the two blocks
    // `crate::crossing` builds are what it knows about it.
    assert!(
        sent[0].contains("What part 1 produced"),
        "it is told what the part before it produced: {}",
        sent[0]
    );
    assert!(
        sent[0].contains("THE PART BEFORE THIS ONE"),
        "and that the part is closed: {}",
        sent[0]
    );
}

/// **The cold switch, at the boundary this time.** A part with no evidence
/// scope is told exactly what it was told before any of this existed, so the
/// brief the next part's Drone opens with puts no tool in front of it.
#[tokio::test]
async fn a_step_boundary_says_nothing_where_the_next_step_wants_no_plan() {
    let home = TempDir::new();
    let (_fleet, sent) = told_across_the_boundary(&home, false, a_diff_call()).await;

    assert!(
        !sent[0].contains("BEFORE YOU START"),
        "no tool is put in front of a Drone that has nothing to declare: {}",
        sent[0]
    );
    assert!(
        sent[0].contains("Implement"),
        "it is still told where it is going: {}",
        sent[0]
    );
}

// ------------------------------------------- what a restarted Drone is told

fn stopped_by(trigger: EscalationTrigger) -> Stopped {
    Stopped {
        verdict: Some(StepVerdict::Failed(
            StepLevelTrigger::of(trigger).expect("a step-level trigger"),
        )),
        judged: Vec::new(),
        flagged: Vec::new(),
    }
}

fn restarted(stopped: &Stopped) -> String {
    resuming_turn(
        &a_job(),
        &a_workflow(),
        &StepId::new("implement"),
        stopped,
        &Crossed::nothing(),
    )
    .expect("a prompt")
    .as_str()
    .to_string()
}

/// **The one that changes work that was right.** `gate_undecided` is the gate
/// saying it could not read what it needed, which is the whole point of
/// `CouldNotDecide`: a machine that cannot answer must not produce a verdict in
/// either direction. The briefing then produced one anyway and handed it over
/// as fact, and a Drone told its work did not pass goes looking for what was
/// wrong with it.
#[test]
fn a_restart_after_a_gate_that_could_not_decide_is_not_told_its_work_failed() {
    let said = restarted(&stopped_by(EscalationTrigger::GateUndecided));

    assert!(
        !said.contains("did not pass"),
        "nothing was checked, so nothing failed: {said}"
    );
    assert!(
        said.contains("never checked"),
        "it is told what actually happened: {said}"
    );
    assert!(
        said.contains("nothing was decided about the work"),
        "and that no verdict exists to go looking for: {said}"
    );
    assert!(
        !said.contains("Address this"),
        "there is nothing cited to address: {said}"
    );
}

/// The ordinary failure still reads as one, and still carries the two fields
/// the refusal reprompt specifies.
#[test]
fn a_restart_after_a_gate_failure_is_told_its_work_did_not_pass() {
    let mut stopped = stopped_by(EscalationTrigger::GateFailure);
    stopped.judged = vec![Judgment {
        criterion_id: CriterionId::new("c1"),
        verdict: JudgeVerdict::NotMet,
        expected: Some(String::from("the reader stops one line later")),
        produced: Some(String::from("the reader stops where it did")),
        consequence: Some(String::from("every caller still reads short")),
    }];
    let said = restarted(&stopped);

    assert!(said.contains("checked and did not pass"), "{said}");
    assert!(said.contains("the reader stops one line later"), "{said}");
    assert!(said.contains("the reader stops where it did"), "{said}");
    assert!(
        !said.contains("every caller still reads short"),
        "consequence is the person's field: {said}"
    );
    assert!(said.contains("Address this and submit again"), "{said}");
}

/// **Four stops, four sentences.** A Drone acts on what it is told, so a
/// restart after thrashing and a restart after a refusal cannot read the same
/// — and neither may claim a check ran where none did.
#[test]
fn no_two_triggers_hand_a_drone_the_same_sentence() {
    let four = [
        EscalationTrigger::GateFailure,
        EscalationTrigger::EvidenceSuspect,
        EscalationTrigger::GateUndecided,
        EscalationTrigger::Thrashing,
    ];
    let mut said: Vec<String> = four
        .iter()
        .map(|trigger| {
            let block = restarted(&stopped_by(*trigger));
            block
                .rsplit("WHY THIS PART IS BEING DONE AGAIN")
                .next()
                .expect("the block is there")
                .to_string()
        })
        .collect();
    said.sort();
    said.dedup();
    assert_eq!(said.len(), 4, "one sentence each: {said:#?}");

    for trigger in [
        EscalationTrigger::GateUndecided,
        EscalationTrigger::Thrashing,
    ] {
        let block = restarted(&stopped_by(trigger));
        assert!(
            !block.contains("did not pass"),
            "{trigger:?} weighed nothing: {block}"
        );
    }
}

/// A `Stopped` carrying no verdict says so rather than inventing one. It is
/// what `Default` builds, and a record that lost its `last_verdict` must not
/// become a refusal on the way to a Drone.
#[test]
fn a_stop_with_no_verdict_recorded_claims_none() {
    let said = restarted(&Stopped::default());

    assert!(said.contains("holds no verdict against its work"), "{said}");
    assert!(!said.contains("did not pass"), "{said}");
}

// ------------------------------ what an opening turn says about the branch

fn opened(opening: &Opening, moved: Option<&TheBaseMoved>) -> String {
    opening
        .turn(&a_job(), &a_workflow(), &StepId::new("implement"), moved)
        .expect("a prompt")
        .as_str()
        .to_string()
}

/// **The no-op stays silent**, which is the same argument `caught_up` makes at
/// a boundary: a paragraph saying nothing happened costs the Drone a read and
/// tells it nothing.
#[test]
fn a_branch_that_did_not_move_is_not_mentioned_in_the_opening_turn() {
    assert!(!opened(&Opening::fresh(), None).contains("THE BRANCH YOU ARE ON"));
}

/// **The tense is the whole reason this block is not
/// `TheBaseMoved::told`.** That one opens "while you worked", which is true of
/// a Drone standing at a boundary and false of one that has not started — and a
/// first turn describing work the reader has no memory of is a first turn it
/// has to reconcile before it can begin.
#[test]
fn a_clean_catch_up_before_a_spawn_does_not_claim_the_drone_was_working() {
    let said = opened(
        &Opening::fresh(),
        Some(&TheBaseMoved::BroughtUpToDate {
            base: String::from("main"),
            commits: 4,
        }),
    );

    assert!(said.contains("THE BRANCH YOU ARE ON"), "{said}");
    assert!(!said.contains("While you worked"), "{said}");
    assert!(said.contains("since this branch was cut"), "{said}");
    assert!(said.contains("4 commit(s)"), "{said}");
}

/// **The reader `#180` had to find.** A restart has no session at the moment
/// the rebase runs, so a conflict is carried by the opening brief and named as
/// the Drone's first piece of work — beside the reason the step is being run
/// again rather than instead of it.
#[test]
fn a_conflicted_catch_up_before_a_spawn_is_named_as_the_first_piece_of_work() {
    let said = opened(
        &Opening::resuming(stopped_by(EscalationTrigger::GateFailure)),
        Some(&TheBaseMoved::Conflicted {
            base: String::from("main"),
            files: vec![String::from("src/log.rs"), String::from("src/parse.rs")],
        }),
    );

    assert!(said.contains("conflict markers in them"), "{said}");
    assert!(said.contains("the first piece of your work"), "{said}");
    assert!(said.contains("- src/log.rs"), "{said}");
    assert!(said.contains("- src/parse.rs"), "{said}");
    assert!(
        said.contains("WHY THIS PART IS BEING DONE AGAIN"),
        "the branch block sits beside the reason, not in place of it: {said}"
    );
}

/// A rebase that would not replay is **not the Drone's to fix**, and the block
/// says so rather than handing it work it has no git to do.
#[test]
fn a_catch_up_that_would_not_replay_tells_the_drone_to_carry_on() {
    let said = opened(
        &Opening::fresh(),
        Some(&TheBaseMoved::CouldNotFollow {
            base: String::from("main"),
        }),
    );

    assert!(said.contains("exactly where it was"), "{said}");
    assert!(said.contains("Nothing here is yours to fix"), "{said}");
    assert!(!said.contains("conflict markers"), "{said}");
}

/// A step that declares an artifact, and the turn its Drone is given.
fn turn_delivering(target: &str) -> String {
    let workflow = testkit::frozen(&[Sketch {
        id: "implement",
        label: "Implement",
        evidence_type: Some("facts_note"),
        gates: &[Gate::ArtifactExists { target }],
        judged_on: &[],
        scope: None,
        gaming: None,
    }]);
    first_turn(
        &a_job(),
        &workflow,
        &StepId::new("implement"),
        &Crossed::nothing(),
    )
    .expect("a prompt")
    .as_str()
    .to_string()
}

/// **The path is named, because a check nobody was told about fails every
/// time.** The step's product is a file now; a Drone that is not told which
/// file does the work and loses it.
#[test]
fn a_step_that_must_write_a_file_is_told_which_file() {
    let said = turn_delivering(".armada/artifacts/plan.md");
    assert!(said.contains(".armada/artifacts/plan.md"), "{said}");
    assert!(said.contains("WHAT THIS PART DELIVERS"), "{said}");
}

/// **The deliverable and the scratch directory are two things and the turn
/// says which.** This is the Job measured on 2026-08-29: the Drone put its
/// plan under `.armada/<job-id>/`, which this repository ignores, so the file
/// never entered the diff and the Judge refused the step for a root cause
/// written on page one of it.
#[test]
fn the_deliverable_is_not_offered_the_scratch_directory() {
    let said = turn_delivering(".armada/artifacts/plan.md");
    let scratch = format!(".armada/{}/", a_job().id().as_str());
    let delivers = said
        .split("WHAT THIS PART DELIVERS")
        .nth(1)
        .expect("the block");

    assert!(
        !delivers.contains(&scratch),
        "the deliverable block points at the scratch path: {delivers}"
    );
    assert!(
        said.contains("not one of them"),
        "the scratch block does not say a deliverable is excluded: {said}"
    );
    // A plan is no longer one of the scratch examples, because on a `plan` step
    // it read as an instruction to file the deliverable out of the Judge's
    // sight.
    assert!(
        !said.contains("A plan, a checklist"),
        "a plan is still offered as scratch: {said}"
    );
}

/// A step that declares no artifact gets no block. **An empty block reads as an
/// answered one**, and every step whose product is the diff would otherwise be
/// asked for a file nothing looks for.
#[test]
fn a_step_that_declares_no_artifact_is_not_asked_for_one() {
    for step in ["implement", "summarise"] {
        let said = turn_at(step);
        assert!(!said.contains("WHAT THIS PART DELIVERS"), "{step}: {said}");
    }
}

/// **The block says the path is the one that is read**, which is the fact a
/// Drone acts on. Fleet opens exactly this path, so nothing goes looking for a
/// file written well somewhere else — "write it here" is not a filing
/// convention and the wording must not read as one.
#[test]
fn the_deliverable_block_says_this_path_is_the_one_that_is_read() {
    let said = turn_delivering(".armada/artifacts/plan.md");
    assert!(
        said.contains("This exact path is the one that is read"),
        "{said}"
    );
    assert!(
        said.contains("an empty file or no file stops this part"),
        "{said}"
    );
    assert!(
        said.contains("a file somewhere else is not this part's work"),
        "{said}"
    );
}
