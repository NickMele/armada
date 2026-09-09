//! Reading one side's frames against the other's.
//!
//! **The aim, the reap, and the reading.** Which tree serves and which one
//! shoots is settled by the real thing — `sh` for all four command lines, the
//! same way [`super::showing`] settles the one-sided run — and what a set of
//! frames then *means* is a reading over rows with no process in it.

use std::path::Path;
use std::time::Duration;

use config::Manifest;
use core_model::{Attempt, Side, StepFrame, StepId};
use verification::{Exit, NeverRan};

use crate::basing::{paired, NoBase, WhyNoPair};
use crate::showing::{kept, run_dir, show, tail, Aimed, ComingUp, NotShown, Shown};
use crate::tests::tmp::TempDir;

const HANDLE: &str = "12-show-me-the-outcome";

/// A harness whose four command lines are `sh` builtins, which is the point:
/// Armada grows no capture stack.
fn harness(manifest_text: &str) -> config::Harness {
    Manifest::parse(Path::new("armada.yml"), manifest_text)
        .expect("a manifest with a harness")
        .harness()
        .expect("the section was declared")
        .clone()
}

fn frame(name: &str, side: Side) -> StepFrame {
    StepFrame {
        name: name.to_string(),
        path: format!(
            ".armada/frames/12-a-job/implement.1.{}/{name}",
            side.as_wire()
        ),
        bytes: 8,
        side,
        // The same digest on both sides, so a caller that folds a matching pair
        // folds this one. A test whose subject is the pairing overrides it.
        digest: String::from("a1b2c3d4e5f60718"),
    }
}

/// **A name on one side only is an answer, not a gap.** A frame the change
/// added has no before and a frame it removed has no after, and neither is a
/// fault — treating either as one would draw the commonest case `#209` exists
/// for, a brand-new screen, as the feature being broken.
#[test]
fn a_name_on_one_side_only_is_added_or_removed_and_never_an_error() {
    let counted = paired(&[
        frame("home.png", Side::Base),
        frame("home.png", Side::Branch),
        frame("settings.png", Side::Branch),
        frame("legacy.png", Side::Base),
    ]);

    assert_eq!(counted.both, 1, "home.png was photographed on both sides");
    assert_eq!(
        counted.added, 1,
        "settings.png is a screen the change added"
    );
    assert_eq!(counted.removed, 1, "legacy.png is one it removed");
}

/// The commonest shape of all: a step whose harness ran only on the branch,
/// because there was no base to serve. Every frame is an addition, and the
/// count says so rather than reporting three failures.
#[test]
fn a_run_with_no_base_at_all_reads_as_every_frame_added() {
    let counted = paired(&[
        frame("home.png", Side::Branch),
        frame("settings.png", Side::Branch),
    ]);
    assert_eq!((counted.both, counted.added, counted.removed), (0, 2, 0));
}

#[test]
fn the_line_says_what_the_change_did_to_the_screen() {
    let counted = paired(&[
        frame("home.png", Side::Base),
        frame("home.png", Side::Branch),
    ]);
    assert_eq!(counted.said(), "1 paired, 0 added, 0 removed");
}

/// **The one reading that matters, and the only thing that can make it.** Both
/// sides run the same spec through the same harness in the same turn, so a spec
/// that fails at base and succeeds on the branch failed on the one difference
/// between them — the code being served. That is a screen the change adds.
/// Failing on both sides is a harness nobody can trust, and saying *there was
/// nothing here before* about it would hide exactly that.
#[test]
fn a_base_run_that_failed_alone_is_a_new_screen_and_one_that_failed_with_the_branch_is_not() {
    let why = WhyNoPair::NotShown(NotShown::SpecFailed(Exit::Code(1)));

    let alone = why.said(true);
    assert!(
        alone.contains("nothing here before"),
        "only the base run failed, so the screen is one the change adds: {alone}"
    );

    let together = why.said(false);
    assert!(
        together.contains("either side"),
        "neither run worked, so this says nothing about the base: {together}"
    );
    assert!(
        !together.contains("nothing here before"),
        "a harness that fails everywhere must not read as a new screen: {together}"
    );
}

/// **A repository with no base is not a fault**, and its sentence must not read
/// like one: there is nothing a change is measured against, and the branch
/// frames are the whole honest answer.
#[test]
fn a_repository_that_names_no_base_says_so_without_naming_a_failure() {
    let said = WhyNoPair::NoBase(NoBase::Unnamed).said(true);
    assert!(said.contains("names no base branch"));
    assert!(
        !said.contains("failed") && !said.contains("could not"),
        "nothing went wrong here: {said}"
    );
}

/// Each of the three ways there is no base sends a person somewhere different,
/// so each names the thing they would go and change.
#[test]
fn every_way_the_base_is_missing_names_what_to_go_and_look_at() {
    assert!(NoBase::NotCheckedOut {
        why: "the repository would not open".to_string(),
    }
    .said()
    .contains("checked out"));
    assert!(NoBase::NotPrepared {
        command: "bootstrap".to_string(),
        why: "exited 1".to_string(),
    }
    .said()
    .contains("setup.requires"));
}

/// A spec that ran against the base and photographed nothing is the spec's
/// answer, not the harness's — the distinction `Shown::Nothing` already draws
/// one layer down, kept here so a person is not sent to fix a harness that
/// worked.
#[test]
fn a_base_run_that_captured_nothing_is_not_reported_as_a_harness_that_failed() {
    let said = WhyNoPair::CapturedNothing.said(true);
    assert!(said.contains("photographed nothing"));
    assert!(!said.contains("nothing here before"));
}

/// The empty spec case, which cannot happen through `showed` — `shown_by` is
/// refused when blank — and is carried anyway, because what it produces is a
/// command with a hole in it rather than a refusal.
#[test]
fn a_spec_that_never_ran_is_still_told_apart_by_whether_the_branch_ran() {
    let why = WhyNoPair::NotShown(NotShown::SpecFailed(Exit::NeverRan(NeverRan::NothingToRun)));
    assert_ne!(why.said(true), why.said(false));
}

/// **The old code serves and the new spec shoots**, which is the whole of how a
/// base run is possible: `shown_by` names a spec that lands in the patch, so it
/// does not exist at base and a run from there would fail every time.
///
/// The harness proves both halves at once — `serve` writes a file into the tree
/// it is started in, and `run` copies whatever it can see from the tree it is
/// started in. So a frame that comes back holding the base's marker was shot
/// from the branch against a server the base started, and nothing else produces
/// that pair.
#[tokio::test]
async fn the_base_serves_and_the_branch_shoots_so_the_spec_never_has_to_exist_at_base() {
    let dir = TempDir::new();
    let base = dir.path().join("base");
    let branch = dir.path().join("branch");
    std::fs::create_dir_all(base.join("shots")).expect("the base checkout");
    std::fs::create_dir_all(branch.join("shots")).expect("the worktree");
    // Only the branch has the spec, which is the situation the split exists
    // for: it is code in the patch.
    std::fs::write(branch.join("the-spec"), b"the branch's own spec").expect("the spec");

    let declared = harness(
        r#"
version: 1
id: armada
evidence:
  serve: sh -c 'printf served > served-here; sleep 30'
  ready: "true"
  run: sh -c 'cp the-spec shots/home.png' {}
  frames: shots
"#,
    );

    let shown = show(
        &declared,
        "e2e/home.spec.ts",
        Aimed::at_base(&base, &branch),
        Side::Base,
        ComingUp::of(Duration::from_secs(5)),
        Duration::from_secs(30),
    )
    .await;

    let Shown::Frames(frames) = shown else {
        panic!("the base served and the branch's spec shot it: {shown:?}");
    };
    assert_eq!(frames.len(), 1);
    assert_eq!(
        frames[0].side,
        Side::Base,
        "the frame is a photograph of the base, whichever tree ran the spec"
    );
    assert!(
        base.join("served-here").exists(),
        "`evidence.serve` ran in the base checkout"
    );
    assert!(
        !branch.join("served-here").exists(),
        "and never in the worktree, which is what makes the frame a `before`"
    );
    assert!(
        branch.join("shots/home.png").exists(),
        "`evidence.run` ran in the worktree, where the spec is"
    );
}

/// **The frames a run left are taken once they are kept**, and this is what
/// makes the second listing honest: both sides shoot into the one directory
/// `evidence.frames` names, so a shot the base took and the branch never
/// reached would still be sitting there and would be filed as the branch's.
#[test]
fn what_the_base_run_left_behind_is_not_counted_as_the_branch_s() {
    let dir = TempDir::new();
    let root = dir.path();
    let worktree = root.join("worktree");
    let wrote = worktree.join("shots");
    std::fs::create_dir_all(&wrote).expect("where the harness wrote");
    std::fs::write(wrote.join("legacy.png"), b"\x89PNG").expect("a base frame");
    // A file the repository put there that no run produced. Nothing may take
    // it: `evidence.frames` is a path the repository named.
    std::fs::write(wrote.join("README"), b"ours").expect("the repository's own file");

    let took = kept(
        &root.to_string_lossy(),
        HANDLE,
        &StepId::new("implement"),
        Attempt::FIRST,
        &[StepFrame {
            name: "legacy.png".to_string(),
            path: String::new(),
            bytes: 4,
            side: Side::Base,
            digest: String::from("a1b2c3d4e5f60718"),
        }],
        &worktree,
        "shots",
        Side::Base,
    );
    assert_eq!(took.len(), 1);
    crate::showing::reaped(&worktree, "shots", &took);

    assert!(
        !wrote.join("legacy.png").exists(),
        "the base's frame is kept, so the branch run's listing must not see it"
    );
    assert!(
        wrote.join("README").exists(),
        "only the names it was given, and never the directory"
    );
    assert!(
        root.join(&took[0].path).exists(),
        "and the copy under .armada is still there"
    );
}

/// The two sides land in two run directories, so a spec that named a screen the
/// same way on both sides does not have one copy overwrite the other — and the
/// id a caller sends back is still two components.
#[test]
fn one_name_on_two_sides_is_two_files_and_two_ids() {
    let base = run_dir(
        HANDLE,
        &StepId::new("implement"),
        Attempt::FIRST,
        Side::Base,
    )
    .expect("a run directory");
    let branch = run_dir(
        HANDLE,
        &StepId::new("implement"),
        Attempt::FIRST,
        Side::Branch,
    )
    .expect("a run directory");

    assert_ne!(base, branch);
    assert_eq!(
        tail(&format!("{base}/home.png")),
        "implement.1.base/home.png"
    );
    assert_eq!(
        tail(&format!("{branch}/home.png")),
        "implement.1.branch/home.png"
    );
}
