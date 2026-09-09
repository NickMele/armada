//! Running a repository's own harness, and keeping what it produced.
//!
//! **Nothing here fakes a filesystem or a process.** The subject is which
//! directory a byte ends up in and which name reaches it, and both are settled
//! by the real thing — the same reason [`keeping`](super::keeping) removes a
//! real worktree.
//!
//! The commands are `sh` builtins rather than a browser, which is the point:
//! Armada grows no capture stack, so a harness that serves with `sleep` and
//! captures with `cp` exercises exactly the seam a Playwright one does.

use std::path::Path;
use std::time::Duration;

use config::Manifest;
use core_model::{Attempt, Side, StepFrame, StepId};

use crate::showing::{frame_bytes, kept, named, show, tail, Aimed, ComingUp, NotShown, Shown};
use crate::tests::tmp::TempDir;

const JOB: &str = "01J0000000000000000000JOB0";
const HANDLE: &str = "12-show-me-the-outcome";

/// A harness that serves with a sleep, is ready immediately, and captures by
/// copying a file the spec names into the frames directory.
fn harness(manifest_text: &str) -> config::Harness {
    Manifest::parse(Path::new("armada.yml"), manifest_text)
        .expect("a manifest with a harness")
        .harness()
        .expect("the section was declared")
        .clone()
}

/// The identity a caller names a frame by: the run's directory and the file
/// name, joined.
///
/// **Two components and not one.** A frame's name is the harness's own, so two
/// steps of one Job may both have written `home.png` and the name alone
/// identifies no row.
#[test]
fn a_frame_is_named_by_its_run_and_its_file_together() {
    assert_eq!(
        tail(".armada/frames/12-a-job/implement.1/home.png"),
        "implement.1/home.png"
    );
    assert_eq!(
        tail(".armada/frames/12-a-job/review.2/home.png"),
        "review.2/home.png",
        "the same file name from another run is another id"
    );
    // Derived from the path rather than stored beside it, so a path with no
    // directory in front of it still answers something rather than panicking.
    assert_eq!(tail("home.png"), "home.png");
}

/// **The record is the allowlist**, which is the whole of what makes a
/// caller-supplied name safe to open a file with. A name no row of this Job
/// holds resolves to nothing, whatever it spells — so nothing decides what may
/// be read except the rows, and a name is never opened and then judged.
#[test]
fn a_name_no_row_holds_resolves_to_no_file_whatever_it_spells() {
    let frames = vec![store::KeptFrame {
        step: StepId::new("implement"),
        attempt: 1,
        frame: StepFrame {
            name: "home.png".to_string(),
            path: format!(".armada/frames/{HANDLE}/implement.1/home.png"),
            bytes: 8,
            side: Side::Branch,
        },
    }];

    assert!(named("implement.1/home.png", &frames).is_some());

    for invented in [
        "implement.1/gone.png",
        "never.1/home.png",
        "home.png",
        "../../../etc/passwd",
        "implement.1/../../../etc/passwd",
    ] {
        assert!(
            named(invented, &frames).is_none(),
            "`{invented}` names no row of this Job"
        );
    }
}

/// A row and its file, or neither.
///
/// **A row whose file will not open answers nothing**, for the reason a frame
/// that would not copy is never recorded: a frame's whole content is the image,
/// so a row without it is nothing a caller can do anything with. A
/// `.armada/frames` directory reclaimed after the record was written is exactly
/// this case, and it is the case the refusal was written for.
#[test]
fn a_row_whose_file_is_gone_is_the_same_answer_as_a_name_that_was_never_one() {
    let dir = TempDir::new();
    let root = dir.path();
    let at = format!(".armada/frames/{HANDLE}/implement.1");
    std::fs::create_dir_all(root.join(&at)).expect("the frames directory");
    std::fs::write(root.join(&at).join("home.png"), b"\x89PNG\r\n\x1a\n").expect("a frame");

    let frames = vec![
        store::KeptFrame {
            step: StepId::new("implement"),
            attempt: 1,
            frame: StepFrame {
                name: "home.png".to_string(),
                path: format!("{at}/home.png"),
                bytes: 8,
                side: Side::Branch,
            },
        },
        store::KeptFrame {
            step: StepId::new("implement"),
            attempt: 1,
            frame: StepFrame {
                name: "reclaimed.png".to_string(),
                path: format!("{at}/reclaimed.png"),
                bytes: 8,
                side: Side::Branch,
            },
        },
    ];

    let root = root.to_string_lossy().into_owned();
    let (held, bytes) =
        frame_bytes(&root, "implement.1/home.png", &frames).expect("the row and its file");
    assert_eq!(held.frame.name, "home.png");
    assert_eq!(bytes, b"\x89PNG\r\n\x1a\n");

    assert!(
        frame_bytes(&root, "implement.1/reclaimed.png", &frames).is_none(),
        "the row is there and the file is not, which is nothing to answer with"
    );
}

/// The copies go under `.armada/frames/`, which is built from the repository
/// root and so is not inside the thing `armada clean` deletes.
///
/// **A frame that will not copy is dropped rather than recorded with no path.**
/// A row pointing at nothing is a dead click, and unlike a Check's output —
/// where the row records a run that decided something — a frame's whole content
/// is the file. There is nothing left to record.
#[test]
fn what_is_kept_outlives_the_worktree_and_a_frame_that_would_not_copy_is_dropped() {
    let dir = TempDir::new();
    let root = dir.path();
    let worktree = root.join("worktree");
    let wrote = worktree.join(".playwright/frames");
    std::fs::create_dir_all(&wrote).expect("where the harness wrote");
    std::fs::write(wrote.join("home.png"), b"\x89PNG\r\n\x1a\n").expect("a frame");

    let found = kept(
        &root.to_string_lossy(),
        HANDLE,
        &StepId::new("implement"),
        Attempt::FIRST,
        &[
            StepFrame {
                name: "home.png".to_string(),
                path: String::new(),
                bytes: 8,
                side: Side::Branch,
            },
            StepFrame {
                name: "never-written.png".to_string(),
                path: String::new(),
                bytes: 0,
                side: Side::Branch,
            },
        ],
        &worktree,
        ".playwright/frames",
        Side::Branch,
    );

    assert_eq!(
        found.len(),
        1,
        "the one that copied, and not the one that did not"
    );
    assert_eq!(
        found[0].path,
        format!(".armada/frames/{HANDLE}/implement.1.branch/home.png"),
        "under .armada, keyed by the run and the side it was taken on"
    );

    // The worktree really goes, which is the whole subject: the copy is not
    // inside it.
    std::fs::remove_dir_all(&worktree).expect("armada clean takes the checkout");
    assert!(
        root.join(&found[0].path).exists(),
        "the frame is still readable after the checkout it came from is gone"
    );
}

/// **The harness runs, and the frames come back.** Four command lines and a
/// directory, and not one of them is Armada's — which is what this proves by
/// using `sh` for all four.
#[tokio::test]
async fn a_harness_that_serves_and_captures_answers_with_what_it_wrote() {
    let dir = TempDir::new();
    let worktree = dir.path().join("worktree");
    std::fs::create_dir_all(worktree.join("shots")).expect("the worktree");

    // `run` writes two files into the frames directory and names the spec, so
    // the substitution is exercised rather than assumed.
    let declared = harness(
        r#"
version: 1
id: armada
evidence:
  serve: sleep 30
  ready: "true"
  run: sh -c 'printf %s "$0" > shots/b.png; printf x > shots/a.png' {}
  frames: shots
"#,
    );

    let shown = show(
        &declared,
        "e2e/home.spec.ts",
        Aimed::at(&worktree),
        Side::Branch,
        ComingUp::of(Duration::from_secs(5)),
        Duration::from_secs(30),
    )
    .await;

    let Shown::Frames(frames) = shown else {
        panic!("the harness ran and captured, so this is frames: {shown:?}");
    };
    assert_eq!(
        frames.iter().map(|at| at.name.as_str()).collect::<Vec<_>>(),
        ["a.png", "b.png"],
        "every file the directory held, in a settled order"
    );
    assert_eq!(
        frames[1].bytes,
        "e2e/home.spec.ts".len() as u64,
        "the spec reached the command, so the substitution happened"
    );
}

/// **A run that captured nothing is its own answer**, and not an empty list of
/// frames. A runner that exits zero having photographed nothing was given a
/// spec that asserts and never captures, which is a spec to read rather than a
/// harness to fix — and a single empty list would say that about a broken
/// harness too.
#[tokio::test]
async fn a_spec_that_captured_nothing_says_so_rather_than_answering_with_no_frames() {
    let dir = TempDir::new();
    let worktree = dir.path().join("worktree");
    std::fs::create_dir_all(worktree.join("shots")).expect("the worktree");

    let shown = show(
        &harness(
            r#"
version: 1
id: armada
evidence:
  serve: sleep 30
  ready: "true"
  run: "true {}"
  frames: shots
"#,
        ),
        "e2e/home.spec.ts",
        Aimed::at(&worktree),
        Side::Branch,
        ComingUp::of(Duration::from_secs(5)),
        Duration::from_secs(30),
    )
    .await;

    assert_eq!(shown, Shown::Nothing);
}

/// A spec that fails is the Drone's spec, and it is reported as the exit rather
/// than as a sentence — a caller drawing this has the shapes `Exit` already
/// tells apart.
#[tokio::test]
async fn a_spec_that_failed_is_named_apart_from_a_harness_that_would_not_run() {
    let dir = TempDir::new();
    let worktree = dir.path().join("worktree");
    std::fs::create_dir_all(worktree.join("shots")).expect("the worktree");

    let shown = show(
        &harness(
            r#"
version: 1
id: armada
evidence:
  serve: sleep 30
  ready: "true"
  run: "false {}"
  frames: shots
"#,
        ),
        "e2e/home.spec.ts",
        Aimed::at(&worktree),
        Side::Branch,
        ComingUp::of(Duration::from_secs(5)),
        Duration::from_secs(30),
    )
    .await;

    assert!(
        matches!(shown, Shown::NotShown(NotShown::SpecFailed(_))),
        "the spec ran and did not exit zero: {shown:?}"
    );
}

/// **A serve that exits before it is ready is not the machine being slow.** A
/// port already in use ends the server in milliseconds, and reporting that as a
/// readiness timeout sends a person to look at the wrong thing.
#[tokio::test]
async fn a_serve_that_ends_before_it_is_ready_is_not_reported_as_a_timeout() {
    let dir = TempDir::new();
    let worktree = dir.path().join("worktree");
    std::fs::create_dir_all(worktree.join("shots")).expect("the worktree");

    let shown = show(
        &harness(
            r#"
version: 1
id: armada
evidence:
  serve: "false"
  ready: "false"
  run: "true {}"
  frames: shots
"#,
        ),
        "e2e/home.spec.ts",
        Aimed::at(&worktree),
        Side::Branch,
        ComingUp::of(Duration::from_secs(2)),
        Duration::from_secs(30),
    )
    .await;

    assert!(
        matches!(shown, Shown::NotShown(NotShown::ServeEnded)),
        "the server is gone, which is a different edit from a slow one: {shown:?}"
    );
}

/// Every `NotShown` names the key it is about, because the ways a harness fails
/// are different edits to `armada.yml` — and a sentence saying only *the
/// harness failed* would send a person to read all five.
#[test]
fn what_went_wrong_names_the_key_to_go_and_look_at() {
    assert!(NotShown::ServeEnded.said().contains("evidence.serve"));
    assert!(NotShown::NeverReady {
        after: Duration::from_secs(30),
    }
    .said()
    .contains("evidence.ready"));
    assert!(NotShown::FramesUnreadable {
        at: "shots".to_string(),
    }
    .said()
    .contains("shots"));
}

/// The Job id is on the module rather than in a signature nothing here takes,
/// so the constant does not read as unused.
#[test]
fn the_handle_is_what_the_run_directory_is_named_after() {
    assert!(JOB.starts_with("01J"));
    assert!(
        kept(
            "/nonexistent-root",
            "not/one/component",
            &StepId::new("implement"),
            Attempt::FIRST,
            &[StepFrame {
                name: "home.png".to_string(),
                path: String::new(),
                bytes: 8,
                side: Side::Branch,
            }],
            Path::new("/nonexistent"),
            "shots",
            Side::Branch,
        )
        .is_empty(),
        "a handle that is not a single path component keeps nothing"
    );
}
