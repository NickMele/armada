//! What a step's declaration looks like on the wire, and the one state it
//! exists to keep readable.
//!
//! **Two empty check lists that mean opposite things.** A step gating on every
//! Check its repository declares, met with a repository declaring none, freezes
//! nothing — exactly as a step that declared no gate does. The first verified
//! nothing while asking to and the second never asked, and the row this file
//! asserts is what tells a reader which happened without a transcript.
//!
//! Built from `ResolvedStep` directly rather than through a dispatched Job:
//! the subject is the drawing, and a whole Fleet around it would be asserting
//! the parser and the store again.

use core_model::{AdvanceGate, ResolvedCheck, ResolvedStep, StepId, EVERY_MANIFEST_CHECK};

use crate::wire::declared_checks;

/// A step with the checks given, and nothing else it does not need.
fn step(checks: Vec<ResolvedCheck>, every: bool) -> ResolvedStep {
    ResolvedStep::frozen(
        StepId::new("implement".to_string()),
        "Implement".to_string(),
        None,
        checks,
        AdvanceGate::Auto,
        Vec::new(),
        None,
        0,
        None,
    )
    .gating_on_every_check(every)
}

fn a_check(name: &str) -> ResolvedCheck {
    ResolvedCheck::ManifestCheck {
        name: name.to_string(),
        run: format!("run {name}"),
        expect_exit_code: 0,
        when: None,
        requires: Vec::new(),
        narrow: None,
    }
}

/// The state the row is for: the step asked for everything and the repository
/// declares nothing. `docs/concepts/manifest.md` sanctions an ungated
/// workspace, so this is a legitimate Job rather than a refused one — and a
/// person reading it back has to be able to see that a gate was asked for.
#[test]
fn a_step_gated_on_an_empty_checks_registry_still_draws_its_declaration() {
    let drawn = declared_checks(&step(Vec::new(), true));
    assert_eq!(drawn.len(), 1, "{drawn:?}");
    assert_eq!(drawn[0].kind, EVERY_MANIFEST_CHECK);
    // No name, no command and no paths: the row is the declaration, and there
    // was nothing for it to expand to.
    assert_eq!(drawn[0].name, None);
    assert_eq!(drawn[0].run, None);
    assert_eq!(drawn[0].when, None);
}

/// And the step that asked for nothing draws nothing, which is the half that
/// makes the half above mean something.
#[test]
fn a_step_that_declared_no_gate_draws_no_row_at_all() {
    assert!(declared_checks(&step(Vec::new(), false)).is_empty());
}

/// Where there was something to expand to, the declaration leads and the
/// Checks it came to follow — it is what those rows came from rather than a
/// Check beside them.
#[test]
fn the_declaration_leads_the_checks_it_expanded_to() {
    let drawn = declared_checks(&step(vec![a_check("build"), a_check("test")], true));
    let kinds: Vec<&str> = drawn.iter().map(|check| check.kind.as_str()).collect();
    assert_eq!(
        kinds,
        [EVERY_MANIFEST_CHECK, "manifest_check", "manifest_check"]
    );
    assert_eq!(drawn[1].name.as_deref(), Some("build"));
    assert_eq!(drawn[1].run.as_deref(), Some("run build"));
}

/// A step that named its Checks by hand draws them and nothing else, so the
/// row never appears on a workflow that did not ask for it.
#[test]
fn naming_checks_by_hand_draws_no_declaration_row() {
    let drawn = declared_checks(&step(vec![a_check("build")], false));
    let kinds: Vec<&str> = drawn.iter().map(|check| check.kind.as_str()).collect();
    assert_eq!(kinds, ["manifest_check"]);
}
