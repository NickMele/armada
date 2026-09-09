//! `evidence:` — the section a repository declares its own harness in.
//!
//! **Four commands and a boundary, and not one of them is Armada's.** What is
//! checked here is that each key is read as what it is, that the three ways a
//! `frames` path is not a directory inside the worktree are caught at load
//! rather than after a server has been started, and that a `run` with nowhere
//! to put the spec is refused — a template that ignores its argument runs the
//! same command whichever spec was named, which is a capture scoped to nothing
//! and saying so nowhere.

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

    assert_eq!(harness.serve(), "pnpm dev --port 6006");
    assert_eq!(harness.ready(), "curl -sf http://localhost:6006");
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
/// run a `visual` step, and that is refused where the workflow is resolved —
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

/// Every required key is required on its own, so a section with three faults is
/// one edit rather than three loads.
#[test]
fn each_missing_command_is_refused_on_its_own() {
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

    for key in [
        "evidence.serve",
        "evidence.ready",
        "evidence.run",
        "evidence.frames",
    ] {
        assert!(
            matches!(fault_at(&refused, key), Fault::Missing),
            "`{key}` is required and says so on its own"
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
