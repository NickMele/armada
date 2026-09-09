//! `mechanical_checks`: the three types carried, the two refused by name, and
//! what a `target` has to be to name one file.
//!
//! The refusals here are the ones measured to be worth catching at parse time
//! rather than at the gate — each is a step no Drone could ever pass, and the
//! cost of finding out later is a worktree, a Drone and a retry budget.

use super::{bug_with, MANIFEST};
use crate::error::{BadTarget, Fault};
use crate::manifest::Manifest;
use crate::resolve::ResolvedWorkflow;
use crate::tests::{fault_at, named, refusals};
use crate::workflow::MechanicalCheck;

#[test]
fn the_two_unimplemented_check_types_are_refused_by_name() {
    for kind in ["test_run", "pr_merged"] {
        let refused = refusals(bug_with(&format!(
            "  - id: close\n    label: Close\n    delivers: false\n    advance_gate: auto\n    mechanical_checks:\n      - {{ type: {kind} }}\n"
        )));
        assert_eq!(
            fault_at(&refused, "steps[3].mechanical_checks[0].type"),
            &Fault::NotYetCarried {
                value: kind.to_string(),
                carried: &[
                    "manifest_check",
                    "every_manifest_check",
                    "diff_nonempty",
                    "artifact_exists"
                ],
            }
        );
    }
}

/// **`artifact_exists` is carried, and it needs the path.** It was refused
/// beside `test_run` and `pr_merged` because the schema's samples name a
/// registry entry and there is no artifact registry. What there is on every
/// step is a worktree, so the target is a path in it — and a check declaring
/// none is refused rather than read as "some file, somewhere".
#[test]
fn an_artifact_check_needs_the_path_it_is_looking_for() {
    let refused = refusals(bug_with(
        "  - id: close\n    label: Close\n    delivers: false\n    advance_gate: auto\n    mechanical_checks:\n      - { type: artifact_exists }\n",
    ));
    assert_eq!(
        fault_at(&refused, "steps[3].mechanical_checks[0].target"),
        &Fault::Missing
    );
}

/// **The four targets that name no single file inside the worktree.**
///
/// Each is refused where the workflow is parsed rather than met at the gate,
/// and the glob is the one that was measured: v1's `design` workflow named
/// `docs/design/*.md`, its gate probed the string as a literal path, and the
/// step was unpassable whatever the Drone wrote — the Job ran until it hit its
/// token ceiling. Refusing a pattern is also what lets Fleet quote the path to
/// the next step's Drone, since "whichever file matched" is not a path.
#[test]
fn an_artifact_target_that_cannot_name_one_file_is_refused_where_it_is_written() {
    for (target, why) in [
        ("docs/design/*.md", BadTarget::Globbed),
        ("report-?.md", BadTarget::Globbed),
        ("/etc/passwd", BadTarget::Absolute),
        ("../elsewhere/plan.md", BadTarget::Escapes),
        (".armada/artifacts/", BadTarget::ADirectory),
    ] {
        let refused = refusals(bug_with(&format!(
            "  - id: close\n    label: Close\n    delivers: false\n    advance_gate: auto\n    mechanical_checks:\n      - {{ type: artifact_exists, target: \"{target}\" }}\n"
        )));
        assert_eq!(
            fault_at(&refused, "steps[3].mechanical_checks[0].target"),
            &Fault::NotAnArtifactPath {
                value: target.to_string(),
                why,
            },
            "`{target}`"
        );
    }
}

/// **A step delivers one file.** Fleet reads the declared path into the Judge's
/// brief as *the document this step produced*, and points the next step's Drone
/// at it. Two targets would make both of those a choice nothing records, so the
/// second is refused where it is written rather than resolved by whichever a
/// reader reached first.
#[test]
fn a_step_declaring_two_artifacts_is_refused_because_it_has_one_deliverable() {
    let refused = refusals(bug_with(
        "  - id: close\n    label: Close\n    delivers: false\n    advance_gate: auto\n    mechanical_checks:\n      - { type: artifact_exists, target: a.md }\n      - { type: artifact_exists, target: b.md }\n",
    ));
    assert_eq!(
        fault_at(&refused, "steps[3].mechanical_checks[1].target"),
        &Fault::TwoDeliverables {
            first: "a.md".to_string()
        }
    );
}

/// A target that names one file in the worktree loads, and the path is carried
/// through resolution rather than reduced to the fact that a check was
/// declared.
#[test]
fn an_artifact_check_carries_its_path_onto_the_step() {
    let def = bug_with(
        "  - id: close\n    label: Close\n    delivers: false\n    advance_gate: auto\n    mechanical_checks:\n      - { type: artifact_exists, target: .armada/artifacts/close.md }\n",
    )
    .expect("the definition loads");
    assert_eq!(
        def.steps()[3].mechanical_checks(),
        [MechanicalCheck::ArtifactExists {
            target: ".armada/artifacts/close.md".to_string()
        }]
    );
    // One place answers "what does this step deliver", because the gate, the
    // brief and the mechanical tier all ask.
    let manifest = Manifest::parse(&named("armada.yml"), MANIFEST).expect("a manifest");
    let resolved = ResolvedWorkflow::resolve(&def, &manifest).expect("it resolves");
    assert_eq!(
        resolved.steps()[3].deliverable(),
        Some(".armada/artifacts/close.md")
    );
    assert_eq!(
        resolved.steps()[0].deliverable(),
        None,
        "a step declaring no artifact delivers no file"
    );
}

/// **The name is required and the code is not**, which is the whole of what
/// moved. A `manifest_check` says which Check; what a passing run of it looks
/// like is the Check's own and lives in `armada.yml`.
#[test]
fn a_manifest_check_needs_the_check_name_and_no_longer_the_expected_code() {
    let refused = refusals(bug_with(
        "  - id: close\n    label: Close\n    delivers: false\n    advance_gate: auto\n    mechanical_checks:\n      - { type: manifest_check }\n",
    ));
    assert_eq!(
        fault_at(&refused, "steps[3].mechanical_checks[0].check"),
        &Fault::Missing
    );
    assert!(
        !refused
            .iter()
            .any(|refusal| refusal.key.ends_with("expect_exit_code")),
        "the key is optional now: {refused:?}"
    );

    let def = bug_with(
        "  - id: close\n    label: Close\n    delivers: false\n    advance_gate: auto\n    mechanical_checks:\n      - { type: manifest_check, check: build }\n",
    )
    .expect("a step may name a Check and say nothing about its code");
    assert_eq!(
        def.steps()[3].mechanical_checks(),
        [MechanicalCheck::ManifestCheck {
            check: "build".to_string(),
            expect_exit_code: None,
        }],
        "absent is carried as absent, not filled in with zero here"
    );
}

/// **A step says *every Check*; it never omits its way into meaning it.** The
/// spelling is an entry in `mechanical_checks` rather than a key on the step so
/// that it composes with `diff_nonempty` — which is exactly what `implement`
/// needs, since a build passes cleanly on an empty diff.
#[test]
fn a_step_may_declare_every_check_the_manifest_names() {
    let def = bug_with(
        "  - id: close\n    label: Close\n    delivers: false\n    advance_gate: auto\n    mechanical_checks:\n      - { type: every_manifest_check }\n      - { type: diff_nonempty }\n",
    )
    .expect("the definition loads");
    assert_eq!(
        def.steps()[3].mechanical_checks(),
        [
            MechanicalCheck::EveryManifestCheck,
            MechanicalCheck::DiffNonempty
        ]
    );
}

/// The set is the Manifest's to state, so there is no key here to trim it with.
#[test]
fn every_manifest_check_carries_nothing_else() {
    for extra in ["check: build", "expect_exit_code: 0"] {
        let refused = refusals(bug_with(&format!(
            "  - id: close\n    label: Close\n    delivers: false\n    advance_gate: auto\n    mechanical_checks:\n      - {{ type: every_manifest_check, {extra} }}\n"
        )));
        assert!(
            refused
                .iter()
                .any(|refusal| matches!(refusal.fault, Fault::Unknown { .. })),
            "`{extra}`: {refused:?}"
        );
    }
}

/// **A step names Checks or gates on all of them, never both.** The two
/// together would run one Check twice and report it twice, and which entry to
/// delete is the author's call rather than a reader's — so the second of the
/// pair is refused where it is written, in either order.
#[test]
fn every_check_and_a_named_one_on_one_step_is_refused() {
    for (entries, at, first_at) in [
        (
            "      - { type: every_manifest_check }\n      - { type: manifest_check, check: build }\n",
            "steps[3].mechanical_checks[1].type",
            0,
        ),
        (
            "      - { type: manifest_check, check: build }\n      - { type: every_manifest_check }\n",
            "steps[3].mechanical_checks[1].type",
            0,
        ),
        (
            "      - { type: diff_nonempty }\n      - { type: every_manifest_check }\n      - { type: every_manifest_check }\n",
            "steps[3].mechanical_checks[2].type",
            1,
        ),
    ] {
        let refused = refusals(bug_with(&format!(
            "  - id: close\n    label: Close\n    delivers: false\n    advance_gate: auto\n    mechanical_checks:\n{entries}"
        )));
        assert_eq!(
            fault_at(&refused, at),
            &Fault::EveryCheckAndByName { first_at },
            "{entries}"
        );
    }
}

/// Two named Checks on one step stay ordinary — `implement` gates on the build
/// and on the tests, and nothing about the rule above touches that.
#[test]
fn two_named_checks_on_one_step_are_not_the_refusal() {
    let def = bug_with(
        "  - id: close\n    label: Close\n    delivers: false\n    advance_gate: auto\n    mechanical_checks:\n      - { type: manifest_check, check: build }\n      - { type: manifest_check, check: test }\n",
    )
    .expect("a step may name two Checks");
    assert_eq!(def.steps()[3].mechanical_checks().len(), 2);
}

#[test]
fn diff_nonempty_carries_nothing_else() {
    let refused = refusals(bug_with(
        "  - id: close\n    label: Close\n    delivers: false\n    advance_gate: auto\n    mechanical_checks:\n      - { type: diff_nonempty, check: build }\n",
    ));
    assert!(matches!(
        fault_at(&refused, "steps[3].mechanical_checks[0].check"),
        Fault::Unknown { .. }
    ));
}
