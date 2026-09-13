//! A person asking a Job to show its work.
//!
//! **Every case starts from a real captured step.** The Job is proposed,
//! approved, worked and submitted, and the step's own harness runs while it
//! settles — so the spec a press reruns is one a Drone named and the gate
//! recorded, and the step's own frames are there to be left alone.
//!
//! The harness is `sh`, which is the point: Armada grows no capture stack, so a
//! spec that copies a file into `evidence.frames` exercises exactly the seam a
//! Playwright one does. It copies whatever `marker` holds, which is how a test
//! tells the step's picture from a press's without reading a digest.

use std::path::PathBuf;
use std::sync::Arc;
use std::time::Duration;

use adapter_traits::WorktreeSpec;
use config::{EvidenceType, Manifest, ResolvedWorkflow, Roster, WorkflowDef};
use core_model::{Job, JobStatus};
use testkit::FakeWorkProduct;
use verification::{Claimed, NotClaimed, ShownBy};

use crate::daemon::Fleet;
use crate::evidence::Call;
use crate::gate::{CheckBudget, Ruling};
use crate::showing_again::Unshowable;
use crate::tests::admitted::dispatched;
use crate::tests::daemon::{a_proposal_for, fittings, one, shown_step, worktree_directory};
use crate::tests::tmp::TempDir;
use crate::tests::tools::{submitted_by_the_one, Fixture};
use crate::Adrift;

const SPEC: &str = "e2e/panel.spec.ts";
const OTHER_SPEC: &str = "e2e/board.spec.ts";

/// Writes the spec it was handed into the frame, so a test reads back which one
/// actually ran. `{}` is substituted with the spec, and `sh -c` hands it to the
/// script as `$0`.
const WROTE_THE_SPEC: &str = "sh -c 'mkdir -p shots && printf %s \"$0\" > shots/ran.txt'";

/// Waits for as long as the worktree holds `hold`, then copies `marker` into
/// the frames directory under the name `name` holds. **The hold is the test's
/// to lift**, so a press is out for exactly as long as a case needs and no
/// clock decides whether it passed.
const HELD_WHILE_ASKED: &str =
    "sh -c 'while [ -f hold ]; do sleep 0.05; done; mkdir -p shots && cp marker shots/$(cat name)'";

fn a_fleet_showing(home: &TempDir, run: &str) -> Arc<Fixture> {
    let (workflow, armada_yml) = shown_step(run, "shots", None);
    let mut fittings = fittings(home, FakeWorkProduct::untouched());
    fittings.workflows = one(workflow);
    fittings.manifest = armada_yml;
    // A press held on a file is bounded by the Check budget, and the fixture's
    // five seconds would end one on a loaded machine before the test lifts it.
    fittings.budget = CheckBudget::of(Duration::from_secs(600));
    Arc::new(Fleet::assembled(fittings))
}

/// A one-step workflow that asks to be captured, holding at
/// `awaiting_review` once the gate runs — `advance_gate: human_always`, unlike
/// `shown_step`'s own `auto` — so the port span the step's harness ran against
/// is still claimed when a press follows, rather than released the instant a
/// one-step `auto` workflow reaches `completed_success`.
///
/// **`ports: storybook: {}`, and a harness that names it three times.**
/// `serve`, `ready` and the substituted `run` each carry `${port.storybook}`
/// and `$ARMADA_PORT_STORYBOOK`, mirroring `armada.yml`'s own `storybook_dev`
/// Command — `#753`'s whole point being that a repository names its port once.
fn a_fleet_showing_a_claimed_port(home: &TempDir) -> Arc<Fixture> {
    let def = WorkflowDef::parse(
        std::path::Path::new("fixture-shown-port.yml"),
        "version: 1\nworkflow_id: fixture-shown-port\nname: fixture\nstructure: linear\nsteps:\n  \
         - id: show\n    label: \"Show\"\n    evidence: {submitted: {type: diff}, captured: true}\n    delivers: false\n    \
         advance_gate: human_always\n",
        &Roster::offering_nothing(),
    )
    .unwrap_or_else(|refused| panic!("the fixture workflow did not parse: {refused}"));
    let armada_yml = Manifest::parse(
        std::path::Path::new("fixture-shown-port-armada.yml"),
        r#"version: 1
id: 01FIXTUREMANIFEST
ports:
  storybook: {}
evidence:
  serve: >-
    /bin/sh -c "echo ${port.storybook} $ARMADA_PORT_STORYBOOK > shots/serve.txt; sleep 30"
  ready: >-
    /bin/sh -c "test -s shots/serve.txt"
  run: >-
    /bin/sh -c "echo ${port.storybook} $ARMADA_PORT_STORYBOOK > shots/run.txt" {}
  frames: shots
"#,
    )
    .unwrap_or_else(|refused| panic!("the fixture manifest did not parse: {refused}"));
    let resolved = ResolvedWorkflow::resolve(&def, &armada_yml)
        .unwrap_or_else(|refused| panic!("the fixture did not resolve: {refused}"));

    let mut fittings = fittings(home, FakeWorkProduct::untouched());
    fittings.workflows = one(resolved);
    fittings.manifest = armada_yml;
    // A press held while `ready` is asked is bounded by the Check budget.
    fittings.budget = CheckBudget::of(Duration::from_secs(600));
    Arc::new(Fleet::assembled(fittings))
}

/// A two-step workflow whose steps are both captured, so one Job's Drones name
/// two specs and a person has something to pick between. Its harness writes the
/// spec it was handed — [`WROTE_THE_SPEC`] — which is how a test tells the one
/// that ran from the one that did not.
fn a_fleet_showing_two_captured_steps(home: &TempDir) -> Arc<Fixture> {
    let def = WorkflowDef::parse(
        std::path::Path::new("fixture-shown-two.yml"),
        "version: 1\nworkflow_id: fixture-shown-two\nname: fixture\nstructure: linear\nsteps:\n  \
         - id: show\n    label: \"Show\"\n    evidence: {submitted: {type: diff}, captured: \
         true}\n    delivers: false\n    advance_gate: auto\n  - id: show_more\n    label: \"Show \
         more\"\n    evidence: {submitted: {type: diff}, captured: true}\n    delivers: false\n    \
         advance_gate: auto\n",
        &Roster::offering_nothing(),
    )
    .unwrap_or_else(|refused| panic!("the fixture workflow did not parse: {refused}"));
    let armada_yml = Manifest::parse(
        std::path::Path::new("fixture-shown-two-armada.yml"),
        &format!("version: 1\nid: 01FIXTUREMANIFEST\nevidence:\n  run: {WROTE_THE_SPEC} {{}}\n  frames: shots\n"),
    )
    .unwrap_or_else(|refused| panic!("the fixture manifest did not parse: {refused}"));
    let resolved = ResolvedWorkflow::resolve(&def, &armada_yml)
        .unwrap_or_else(|refused| panic!("the fixture did not resolve: {refused}"));

    let mut fittings = fittings(home, FakeWorkProduct::untouched());
    fittings.workflows = one(resolved);
    fittings.manifest = armada_yml;
    fittings.budget = CheckBudget::of(Duration::from_secs(600));
    Arc::new(Fleet::assembled(fittings))
}

fn worktree_of(home: &TempDir, job: &Job) -> PathBuf {
    let spec =
        WorktreeSpec::for_job(&home.path().to_string_lossy(), &job.handle()).expect("a legal spec");
    PathBuf::from(spec.worktree_path())
}

fn shown(spec: &'static str) -> Call<'static> {
    Call {
        evidence_type: EvidenceType::Diff,
        claimed: Claimed("the panel now collapses"),
        shown_by: ShownBy(spec),
        not_claimed: NotClaimed(""),
    }
}

/// A Job whose one captured step was worked, submitted and ruled on, its own
/// harness having photographed `marker` as it stood — so the Job has finished
/// and its worktree is still on disk, which is the moment a reviewer presses.
async fn shown_once(fleet: &Fixture, home: &TempDir, title: &str, marker: &str) -> Job {
    let job = fleet
        .propose(a_proposal_for(title, "fixture-shown"))
        .await
        .expect("a Job at the approval gate");
    worktree_directory(home, &job);
    let tree = worktree_of(home, &job);
    std::fs::create_dir_all(tree.join("e2e")).expect("the spec's directory");
    std::fs::write(tree.join(SPEC), b"the Drone's spec").expect("the spec");
    std::fs::write(tree.join("marker"), marker).expect("what the harness photographs");
    std::fs::write(tree.join("name"), "a.txt").expect("what the spec calls its frame");
    dispatched(fleet, job.id()).await.expect("released to run");
    submitted_by_the_one(fleet, shown(SPEC))
        .await
        .expect("the Drone names its spec");
    let turned = fleet.turn().await.expect("the gate ran");
    assert!(
        matches!(
            turned.ruled(),
            Some(Ruling::Advanced { .. }) | Some(Ruling::Finished { .. })
        ),
        "a shown step with nothing else to gate on is ruled on: {:?}",
        turned.ruled()
    );
    fleet.load(job.id()).await.expect("the Job reads")
}

fn read(home: &TempDir, path: &str) -> String {
    std::fs::read_to_string(home.path().join(path)).expect("the kept copy is there")
}

// ---------------------------------------------------- `#753`: a claimed port

/// **`serve`, `ready` and `run` each reach the port this Job's own span
/// claimed — in the step's own run, and again on a press against the same
/// still-open claim.** `a_fleet_showing_a_claimed_port` holds the Job at
/// `awaiting_review` rather than letting it finish, so the span is still held
/// — `crate::dispatch::record` releases it only once the Job goes terminal —
/// which is the "Watch for" this issue names: a press against a Job whose
/// claim is already gone is a different case, not exercised here.
#[tokio::test]
async fn a_harness_naming_a_declared_port_reaches_the_claim_on_the_step_and_on_a_press() {
    let home = TempDir::new();
    let fleet = a_fleet_showing_a_claimed_port(&home);

    let job = fleet
        .propose(a_proposal_for(
            "show the panel on its own port",
            "fixture-shown-port",
        ))
        .await
        .expect("a Job at the approval gate");
    worktree_directory(&home, &job);
    let tree = worktree_of(&home, &job);
    std::fs::create_dir_all(tree.join("e2e")).expect("the spec's directory");
    std::fs::create_dir_all(tree.join("shots")).expect("where the harness writes");
    std::fs::write(tree.join(SPEC), b"the Drone's spec").expect("the spec");
    dispatched(&fleet, job.id()).await.expect("released to run");
    submitted_by_the_one(&fleet, shown(SPEC))
        .await
        .expect("the Drone names its spec");
    let turned = fleet.turn().await.expect("the gate ran");
    assert!(
        matches!(turned.ruled(), Some(Ruling::HeldForReview { .. })),
        "a human_always gate holds rather than finishing the Job: {:?}",
        turned.ruled()
    );
    let job = fleet.load(job.id()).await.expect("the Job reads");
    assert_eq!(job.status(), JobStatus::AwaitingReview, "the span is kept");

    let claim = fleet
        .store()
        .lock()
        .await
        .port_span_for_job(job.id())
        .expect("the read succeeds")
        .expect("the Job's span is still claimed");
    let claimed = format!("{} {}", claim.base, claim.base);

    let own = fleet
        .store()
        .lock()
        .await
        .step_frames_every_attempt(job.id())
        .expect("reads");
    let mut own_names: Vec<_> = own.iter().map(|held| held.frame.name.clone()).collect();
    own_names.sort();
    assert_eq!(
        own_names,
        vec!["run.txt".to_string(), "serve.txt".to_string()],
        "`evidence.serve` and `evidence.run` both wrote into `shots`"
    );
    for held in &own {
        assert_eq!(
            read(&home, &held.frame.path).trim(),
            claimed,
            "`{}` saw `${{port.storybook}}` and `$ARMADA_PORT_STORYBOOK` resolved to the claim",
            held.frame.name
        );
    }

    // The press, against the very claim the step's own run just proved.
    let pressed = Arc::clone(&fleet)
        .show_again(job.id(), None)
        .await
        .expect("the press ran");
    let set = pressed.set.expect("the harness captured a frame");
    assert_eq!(pressed.nothing, None);
    let mut pressed_names: Vec<_> = set.frames.iter().map(|frame| frame.name.clone()).collect();
    pressed_names.sort();
    assert_eq!(
        pressed_names,
        vec!["run.txt".to_string(), "serve.txt".to_string()],
        "a press reruns `evidence.serve` and `evidence.run` exactly as the step did"
    );
    for frame in &set.frames {
        assert_eq!(
            read(&home, &frame.path).trim(),
            claimed,
            "a press resolves against the same claim the step's own run did: `{}`",
            frame.name
        );
    }
}

// ------------------------------------------------------------ the press

/// **The whole of `#603`'s definition of done.** A press runs the harness in
/// the Job's worktree and its frames come back as a set of their own — and
/// while it runs, the Fleet's other Jobs keep turning.
///
/// **Order, not a clock.** The press is held on a file only its worktree holds
/// and only the test lifts. The second Job is proposed, worked and ruled on
/// while the press is still held — `Pressing::since` answers for that — and
/// only then is the hold lifted and the set read. A press on the turn loop
/// could never let that turn come back, so this would hang rather than pass.
#[tokio::test]
async fn a_press_keeps_a_set_of_its_own_while_the_other_jobs_keep_turning() {
    let home = TempDir::new();
    let fleet = a_fleet_showing(&home, HELD_WHILE_ASKED);
    let first = shown_once(&fleet, &home, "show the panel", "the step's own picture").await;
    assert!(
        first.status().is_terminal(),
        "a one-step Job finishes: {:?}",
        first.status()
    );

    // What the step photographed, before anything is pressed.
    let own = fleet
        .store()
        .lock()
        .await
        .step_frames_every_attempt(first.id())
        .expect("reads");
    assert_eq!(own.len(), 1, "the step kept its one frame");

    let tree = worktree_of(&home, &first);
    std::fs::write(tree.join("marker"), "a press's picture").expect("the screen changed");
    std::fs::write(tree.join("hold"), b"").expect("an app still starting");

    let id = first.id().clone();
    let pressing = tokio::spawn({
        let fleet = Arc::clone(&fleet);
        async move { fleet.show_again(&id, None).await }
    });
    // Until the press is out: taken for this Job, and waiting on the hold.
    while fleet.pressing().since(first.id()).is_none() {
        assert!(
            !pressing.is_finished(),
            "the press ended before it was ever held"
        );
        tokio::task::yield_now().await;
    }

    // The second Job, start to finish, while the press is held.
    let second = fleet
        .propose(a_proposal_for("show the other panel", "fixture-shown"))
        .await
        .expect("a second Job");
    worktree_directory(&home, &second);
    let other = worktree_of(&home, &second);
    std::fs::create_dir_all(other.join("e2e")).expect("its spec's directory");
    std::fs::write(other.join(SPEC), b"its spec").expect("its spec");
    std::fs::write(other.join("marker"), "the other Job").expect("its marker");
    std::fs::write(other.join("name"), "a.txt").expect("its frame's name");
    dispatched(&fleet, second.id())
        .await
        .expect("admitted beside the press");
    submitted_by_the_one(&fleet, shown(SPEC))
        .await
        .expect("the second Drone submits");
    let turned = fleet.turn().await.expect("a turn while the press is out");

    assert!(
        matches!(
            turned.ruled(),
            Some(Ruling::Advanced { .. }) | Some(Ruling::Finished { .. })
        ),
        "the other Job was ruled on while the press was held: {:?}",
        turned.ruled()
    );
    assert!(
        fleet.pressing().since(first.id()).is_some() && !pressing.is_finished(),
        "and the press was still held when that Job was ruled on"
    );

    std::fs::remove_file(tree.join("hold")).expect("the app is up");

    let answered = pressing
        .await
        .expect("the press's task ended")
        .expect("the press ran");
    let set = answered.set.expect("the harness captured a frame");
    assert_eq!(answered.nothing, None);
    assert_eq!(set.press, 1, "the Job's first press");
    assert_eq!(set.step_id.as_str(), "show");
    assert_eq!(set.frames.len(), 1);
    assert_eq!(
        read(&home, &set.frames[0].path),
        "a press's picture",
        "the press photographed the worktree as it is now"
    );
    assert!(
        set.frames[0].kept.starts_with("show.again1."),
        "in a run directory of its own: {}",
        set.frames[0].kept
    );

    let own_after = fleet
        .store()
        .lock()
        .await
        .step_frames_every_attempt(first.id())
        .expect("reads");
    assert_eq!(own_after, own, "the step's own frame record is untouched");
    assert_eq!(
        read(&home, &own_after[0].frame.path),
        "the step's own picture",
        "and so is the copy it points at"
    );

    let facts = fleet
        .showing_again_of(&fleet.load(first.id()).await.expect("reads"))
        .await
        .expect("the facts read");
    assert_eq!(facts.shown.len(), 1, "get_job carries the set");
    assert_eq!(facts.showing_since, None, "and no press is out any more");
}

/// **A second press is a second set.** The owner's decision: nothing a press
/// keeps replaces anything, including an earlier press.
#[tokio::test]
async fn two_presses_are_two_sets_told_apart_by_when_each_ran() {
    let home = TempDir::new();
    let fleet = a_fleet_showing(&home, HELD_WHILE_ASKED);
    let job = shown_once(&fleet, &home, "show the panel", "the step's own picture").await;
    let tree = worktree_of(&home, &job);

    for marker in ["first press", "second press"] {
        std::fs::write(tree.join("marker"), marker).expect("the screen changed");
        Arc::clone(&fleet)
            .show_again(job.id(), None)
            .await
            .expect("the press ran");
    }

    let sets = fleet
        .store()
        .lock()
        .await
        .shown_again_every_press(job.id())
        .expect("reads");
    assert_eq!(sets.len(), 2);
    assert_ne!(sets[0].pressed_at, sets[1].pressed_at);
    assert_eq!(read(&home, &sets[0].frames[0].path), "first press");
    assert_eq!(read(&home, &sets[1].frames[0].path), "second press");
}

/// **A press keeps what its own spec wrote and nothing an earlier run left.**
/// Every run shoots into the one `evidence.frames` directory, so the step's
/// frame, or an earlier press's, would otherwise be listed again and filed
/// under a press that never produced it. Both reaps are proved here: the
/// step's run writes `a.txt`, then two presses write `b.txt` and `c.txt`.
#[tokio::test]
async fn a_press_keeps_only_what_its_own_spec_wrote() {
    let home = TempDir::new();
    let fleet = a_fleet_showing(&home, HELD_WHILE_ASKED);
    let job = shown_once(&fleet, &home, "show the panel", "the step's own picture").await;
    let tree = worktree_of(&home, &job);

    for name in ["b.txt", "c.txt"] {
        std::fs::write(tree.join("name"), name).expect("the spec names another frame");
        Arc::clone(&fleet)
            .show_again(job.id(), None)
            .await
            .expect("the press ran");
    }

    let names = |frames: &[core_model::StepFrame]| {
        frames
            .iter()
            .map(|frame| frame.name.clone())
            .collect::<Vec<_>>()
    };
    let store = fleet.store().lock().await;
    let own = store.step_frames_every_attempt(job.id()).expect("reads");
    assert_eq!(own.len(), 1);
    assert_eq!(own[0].frame.name, "a.txt", "the step kept what it wrote");
    let sets = store.shown_again_every_press(job.id()).expect("reads");
    assert_eq!(
        sets.iter()
            .map(|set| names(&set.frames))
            .collect::<Vec<_>>(),
        vec![vec!["b.txt".to_string()], vec!["c.txt".to_string()]],
        "each press kept only the frame its own spec wrote"
    );
    assert!(
        !tree.join("shots").join("a.txt").exists(),
        "the step's frame left the worktree once it was kept"
    );
}

// ------------------------------------------------------ a spec a person picks

/// **`#619`.** A press runs the spec a person picked, a press that picks
/// nothing runs the last a Drone named, and a spec no Drone named is refused
/// before anything runs.
///
/// **The refusal is the whole of the path rule.** A spec reaches `evidence.run`
/// as an argument, and what keeps it inside the worktree is that the only
/// values Fleet will run are ones its own record holds — so `../../../etc/
/// passwd` is refused as a spec nobody named rather than by a check on its
/// shape.
#[tokio::test]
async fn a_press_runs_the_spec_that_was_picked_and_refuses_one_no_drone_named() {
    let home = TempDir::new();
    let fleet = a_fleet_showing_two_captured_steps(&home);
    let job = fleet
        .propose(a_proposal_for("show both panels", "fixture-shown-two"))
        .await
        .expect("a Job at the approval gate");
    worktree_directory(&home, &job);
    let tree = worktree_of(&home, &job);
    std::fs::create_dir_all(tree.join("e2e")).expect("the specs' directory");
    for spec in [SPEC, OTHER_SPEC] {
        std::fs::write(tree.join(spec), b"a Drone's spec").expect("the spec");
    }
    dispatched(&fleet, job.id()).await.expect("released to run");
    for spec in [SPEC, OTHER_SPEC] {
        while fleet.working_on().await.is_empty() {
            fleet.turn().await.expect("a turn brings the next step up");
        }
        submitted_by_the_one(&fleet, shown(spec))
            .await
            .expect("the Drone names its spec");
        fleet.turn().await.expect("the gate ran");
    }
    let job = fleet.load(job.id()).await.expect("the Job reads");

    let facts = fleet.showing_again_of(&job).await.expect("the facts read");
    assert_eq!(
        facts
            .specs
            .iter()
            .map(|named| named.spec.as_str())
            .collect::<Vec<_>>(),
        vec![OTHER_SPEC, SPEC],
        "both steps' specs are offered, latest first"
    );
    assert_eq!(
        facts.spec.as_ref().map(|named| named.spec.as_str()),
        Some(OTHER_SPEC),
        "and the first is what a press with no pick runs"
    );

    let picked = Arc::clone(&fleet)
        .show_again(job.id(), Some(SPEC))
        .await
        .expect("the press ran");
    let set = picked.set.expect("the harness captured a frame");
    assert_eq!(set.spec, SPEC, "the set says which spec produced it");
    assert_eq!(
        read(&home, &set.frames[0].path),
        SPEC,
        "and `evidence.run` was handed the spec that was picked"
    );

    let unpicked = Arc::clone(&fleet)
        .show_again(job.id(), None)
        .await
        .expect("the press ran");
    let set = unpicked.set.expect("the harness captured a frame");
    assert_eq!(set.spec, OTHER_SPEC);
    assert_eq!(
        read(&home, &set.frames[0].path),
        OTHER_SPEC,
        "picking nothing still runs the last spec a Drone named"
    );

    let out = "../../../etc/passwd";
    let why = why(Arc::clone(&fleet).show_again(job.id(), Some(out)).await);
    assert_eq!(
        why,
        Unshowable::SpecNotNamed {
            spec: out.to_string()
        }
    );
    assert!(
        why.said().contains("this Job's own record"),
        "a person is told what a press will run: {}",
        why.said()
    );
}

// -------------------------------------------------------- what refuses

fn why(refused: Result<ipc::ShownAgain, Adrift>) -> Unshowable {
    match refused {
        Err(Adrift::CannotShowAgain { why, .. }) => why,
        other => panic!("expected a refusal naming what is missing, got {other:?}"),
    }
}

/// **A repository with no `evidence:` has nothing to run**, and the refusal
/// says so through the wire the way a person reads it: a 409 with a sentence,
/// never a press that fails when the harness is asked for.
#[tokio::test]
async fn a_repository_that_declares_no_harness_is_told_so() {
    let home = TempDir::new();
    let fleet = Arc::new(Fleet::assembled(fittings(
        &home,
        FakeWorkProduct::untouched(),
    )));
    let job = fleet
        .propose(crate::tests::daemon::a_proposal("show the panel"))
        .await
        .expect("a Job");
    worktree_directory(&home, &job);

    let refused = api::Commands::show_again(Arc::clone(&fleet), ipc::JobId::from(job.id()), None)
        .await
        .expect_err("nothing to run");
    assert_eq!(refused.status(), 409);
    assert_eq!(refused.error().code, "fleet.cannot_show_again");
    assert!(
        refused
            .error()
            .message
            .contains("declares no evidence harness"),
        "a person is told what is missing: {}",
        refused.error().message
    );
}

#[tokio::test]
async fn a_job_whose_worktree_is_gone_is_told_so() {
    let home = TempDir::new();
    let fleet = a_fleet_showing(&home, HELD_WHILE_ASKED);
    let job = fleet
        .propose(a_proposal_for("show the panel", "fixture-shown"))
        .await
        .expect("a Job with no worktree yet");

    let refused = Arc::clone(&fleet).show_again(job.id(), None).await;
    let why = why(refused);
    assert_eq!(why, Unshowable::NoWorktree);
    assert!(why.said().contains("worktree is no longer on disk"));
}

#[tokio::test]
async fn a_job_whose_drone_never_named_a_spec_is_told_so() {
    let home = TempDir::new();
    let fleet = a_fleet_showing(&home, HELD_WHILE_ASKED);
    let job = fleet
        .propose(a_proposal_for("show the panel", "fixture-shown"))
        .await
        .expect("a Job");
    worktree_directory(&home, &job);

    let why = why(Arc::clone(&fleet).show_again(job.id(), None).await);
    assert_eq!(why, Unshowable::NoSpec);
    assert!(why.said().contains("no step of this Job named a spec"));
}

#[tokio::test]
async fn a_spec_a_later_run_deleted_is_named_rather_than_run() {
    let home = TempDir::new();
    let fleet = a_fleet_showing(&home, HELD_WHILE_ASKED);
    let job = shown_once(&fleet, &home, "show the panel", "the step's own picture").await;
    std::fs::remove_file(worktree_of(&home, &job).join(SPEC)).expect("the spec went");

    let why = why(Arc::clone(&fleet).show_again(job.id(), None).await);
    assert_eq!(
        why,
        Unshowable::SpecGone {
            spec: SPEC.to_string()
        }
    );
    assert!(why.said().contains(SPEC), "the sentence names the spec");
}

#[tokio::test]
async fn a_job_whose_drone_is_working_is_not_photographed_mid_edit() {
    let home = TempDir::new();
    let fleet = a_fleet_showing(&home, HELD_WHILE_ASKED);
    let job = fleet
        .propose(a_proposal_for("show the panel", "fixture-shown"))
        .await
        .expect("a Job");
    worktree_directory(&home, &job);
    let running = dispatched(&fleet, job.id()).await.expect("released");
    assert_eq!(running.status(), JobStatus::Running);

    assert_eq!(
        why(Arc::clone(&fleet).show_again(job.id(), None).await),
        Unshowable::DroneWorking
    );
}
