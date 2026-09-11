//! `evidence:` — the section a repository declares its own harness in.
//!
//! **A run, a place to look, and a boundary — and not one of them is
//! Armada's.** What is checked here is that each key is read as what it is,
//! that `serve` and `ready` are a pair rather than a default, that the three
//! ways a `frames` path is not a directory inside the worktree are caught at
//! load rather than after a server has been started, and that a `run` with
//! nowhere to put the spec is refused — a template that ignores its argument
//! runs the same command whichever spec was named, which is a capture scoped
//! to nothing and saying so nowhere.

use crate::error::Fault;
use crate::manifest::Manifest;
use crate::tests::{fault_at, named, refusals};

/// A manifest with a harness in it, and the four keys filled the way a
/// repository would fill them.
const WITH_A_HARNESS: &str = r#"
version: 1
id: armada
checks:
  test:
    run: cargo nextest run --workspace
evidence:
  serve: pnpm dev --port 6006
  ready: curl -sf http://localhost:6006
  run: pnpm exec playwright test {}
  frames: .playwright/frames
  never:
    - /settings
    - /admin
"#;

fn parse(text: &str) -> Result<Manifest, crate::LoadError> {
    Manifest::parse(&named("armada.yml"), text)
}

/// The whole section, read back through the accessors — which is where what
/// each value means is written down.
#[test]
fn a_repository_declares_how_it_shows_its_own_work() {
    let manifest = parse(WITH_A_HARNESS).expect("a manifest with a harness");
    let harness = manifest.harness().expect("the section was declared");

    assert_eq!(harness.serve(), Some("pnpm dev --port 6006"));
    assert_eq!(harness.ready(), Some("curl -sf http://localhost:6006"));
    assert_eq!(harness.frames().as_str(), ".playwright/frames");
    assert_eq!(harness.never(), ["/settings", "/admin"]);
}

/// **The substitution is what makes the template a command**, and it happens
/// in one place — a caller composing the string itself would be a second place
/// the two characters are known.
#[test]
fn the_spec_goes_where_the_run_template_says_it_goes() {
    let manifest = parse(WITH_A_HARNESS).expect("a manifest with a harness");
    let harness = manifest.harness().expect("the section was declared");

    assert_eq!(
        harness.running("e2e/job-detail.spec.ts"),
        "pnpm exec playwright test e2e/job-detail.spec.ts",
        "the spec is substituted, and nothing else in the line moves"
    );
}

/// **Absent is not a default.** A repository that declares no harness cannot
/// run a `shown` step, and that is refused where the workflow is resolved —
/// so absence reaches a Job as a refusal a person reads, never as a capture
/// that quietly produced nothing.
#[test]
fn a_repository_that_says_nothing_has_no_harness_rather_than_an_empty_one() {
    let manifest = parse(
        r#"
version: 1
id: armada
checks:
  test:
    run: cargo nextest run --workspace
"#,
    )
    .expect("a manifest with no evidence section");

    assert!(manifest.harness().is_none());
}

/// A `run` with nowhere to put the spec would run the same command whichever
/// spec was named. It is refused rather than run, at load.
#[test]
fn a_run_template_with_nothing_to_substitute_is_refused() {
    let refused = refusals(parse(
        r#"
version: 1
id: armada
checks:
  test:
    run: cargo nextest run --workspace
evidence:
  serve: pnpm dev
  ready: curl -sf http://localhost:6006
  run: pnpm exec playwright test
  frames: .playwright/frames
"#,
    ));

    assert!(matches!(
        fault_at(&refused, "evidence.run"),
        Fault::NothingToSubstitute
    ));
}

/// The three ways `frames` does not name a directory inside the worktree.
///
/// **Each is a harness that fails on every Job in the same way**, and each is
/// cheaper to catch here than after a server has been started and a spec has
/// run. A glob cannot name a directory to read; an absolute path is not in the
/// worktree the run happens in; a path climbing out of it reaches the machine
/// rather than the checkout.
#[test]
fn a_frames_path_that_is_not_a_directory_in_the_worktree_is_refused() {
    for written in [".playwright/*", "/tmp/frames", "../frames"] {
        let refused = refusals(parse(&format!(
            r#"
version: 1
id: armada
checks:
  test:
    run: cargo nextest run --workspace
evidence:
  serve: pnpm dev
  ready: curl -sf http://localhost:6006
  run: pnpm exec playwright test {{}}
  frames: {written}
"#
        )));

        assert!(
            matches!(
                fault_at(&refused, "evidence.frames"),
                Fault::NotAnArtifactPath { .. }
            ),
            "`{written}` names no directory inside the worktree"
        );
    }
}

/// `run` and `frames` are required on their own, so a section missing both is
/// one edit rather than two loads. `serve` and `ready` are not among them —
/// naming neither is the shape a repository with nothing to serve declares.
#[test]
fn each_required_key_is_refused_on_its_own() {
    let refused = refusals(parse(
        r#"
version: 1
id: armada
checks:
  test:
    run: cargo nextest run --workspace
evidence:
  never:
    - /admin
"#,
    ));

    for key in ["evidence.run", "evidence.frames"] {
        assert!(
            matches!(fault_at(&refused, key), Fault::Missing),
            "`{key}` is required and says so on its own"
        );
    }
    assert!(
        !crate::tests::refused(&refused, "evidence.serve"),
        "`serve` is optional, and naming neither it nor `ready` is not a fault"
    );
}

/// **The common denominator.** A repository with nothing to serve — a desktop
/// app, a CLI, a library — declares only the two keys every kind of software
/// shares.
#[test]
fn a_repository_with_nothing_to_serve_declares_only_run_and_frames() {
    let manifest = parse(
        r#"
version: 1
id: armada
checks:
  test:
    run: cargo nextest run --workspace
evidence:
  run: node run-and-capture.js {}
  frames: .armada/frames
"#,
    )
    .expect("a manifest with a harness and no server");
    let harness = manifest.harness().expect("the section was declared");

    assert_eq!(harness.serve(), None);
    assert_eq!(harness.ready(), None);
    assert_eq!(harness.frames().as_str(), ".armada/frames");
}

/// **`serve` and `ready` are a pair, not two independent optionals.** A server
/// nothing confirms came up is a frame of a blank page that looks exactly like
/// one of a broken page, and a readiness probe with nothing to probe answers a
/// question nobody asked.
#[test]
fn serve_and_ready_are_refused_apart() {
    for (present, absent, written) in [
        ("serve", "ready", "  serve: pnpm dev --port 6006\n"),
        (
            "ready",
            "serve",
            "  ready: curl -sf http://localhost:6006\n",
        ),
    ] {
        let refused = refusals(parse(&format!(
            r#"
version: 1
id: armada
checks:
  test:
    run: cargo nextest run --workspace
evidence:
{written}  run: pnpm exec playwright test {{}}
  frames: .playwright/frames
"#
        )));

        assert!(
            matches!(
                fault_at(&refused, &format!("evidence.{absent}")),
                Fault::ServeReadyMustPair { present: found } if *found == present
            ),
            "`{present}` alone names the key that is missing, and which one is there"
        );
    }
}

/// **`never: []` is a key to delete**, not a repository that forbids nothing —
/// which is what an absent key already says. Same rule `requires` follows.
#[test]
fn a_never_list_with_nothing_in_it_is_refused_rather_than_read_as_empty() {
    let refused = refusals(parse(
        r#"
version: 1
id: armada
checks:
  test:
    run: cargo nextest run --workspace
evidence:
  serve: pnpm dev
  ready: curl -sf http://localhost:6006
  run: pnpm exec playwright test {}
  frames: .playwright/frames
  never: []
"#,
    ));

    assert!(crate::tests::refused(&refused, "evidence.never"));
}
