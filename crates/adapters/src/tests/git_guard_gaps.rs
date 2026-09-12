//! The two cases #786 named as not yet closed: a bare `git branch <name>`
//! still creating a branch, and a global flag ahead of the subcommand still
//! moving it out of a verb-keyed rule's reach. Kept apart from `harness` so
//! that file does not grow past the argument that keeps it one file.

use adapter_traits::{AgentHarness, Grant, Toolbelt};

use crate::harness::{HarnessRefused, HeadlessAgent};
use crate::tests::harness::{config, rendered, value_after};

/// The gap itself: nothing granted `Bash(git branch:*)` before, so a
/// bare-create declared command rendered as an ordinary allow, and the Job
/// would have gone quiet at the point [`crate::git_guard::disallowed_git_rules`]
/// denied it at runtime with no refusal at render to say why.
#[test]
fn a_bare_branch_create_is_refused_at_render_not_left_to_go_quiet() {
    for run in ["git branch new-feature", "git branch --list"] {
        let refused = HeadlessAgent::at("/usr/local/bin/agent").render(&config(
            Toolbelt::evidence_only().and(Grant::RunADeclaredCommand(String::from(run))),
        ));
        assert!(
            matches!(refused, Err(HarnessRefused::CommandWouldMutateGit { .. })),
            "`{run}` was not refused: {refused:?}"
        );
    }
}

/// The other gap: a global flag that takes a value and is not one of the
/// three the old parser named (`--namespace`, unlike `-c` and `--git-dir`)
/// shifts the subcommand two words rather than one, so the old parser landed
/// on the value itself rather than the real subcommand behind it. Named on
/// the flag, since the word after it is not reliably the subcommand.
#[test]
fn a_value_taking_global_flag_outside_the_named_three_is_refused_at_render() {
    let refused =
        HeadlessAgent::at("/usr/local/bin/agent").render(&config(Toolbelt::evidence_only().and(
            Grant::RunADeclaredCommand(String::from("git --namespace n commit -m x")),
        )));
    assert!(
        matches!(
            refused,
            Err(HarnessRefused::CommandCarriesAGitGlobalFlag { .. })
        ),
        "{refused:?}"
    );
}

/// A boolean global flag (`--no-pager`, `-p`, `--bare`) does not shift the
/// subcommand at all, so the existing mutating-verb check already named it
/// correctly — this is the control that the whole class is still refused,
/// whichever check catches a given flag.
#[test]
fn every_global_flag_is_refused_however_it_is_parsed() {
    for run in [
        "git --namespace n commit -m x",
        "git --no-pager commit -m x",
        "git -p commit -m x",
        "git --bare add file",
    ] {
        let refused = HeadlessAgent::at("/usr/local/bin/agent").render(&config(
            Toolbelt::evidence_only().and(Grant::RunADeclaredCommand(String::from(run))),
        ));
        assert!(refused.is_err(), "`{run}` was not refused: {refused:?}");
    }
}

/// The floor denies a global flag wholesale, so even a declared command that
/// only reads is refused at render — a Drone told it had `git --no-pager
/// log` would find it denied at runtime with nothing at render to say so.
#[test]
fn a_global_flag_before_a_read_only_verb_is_refused_too() {
    let refused =
        HeadlessAgent::at("/usr/local/bin/agent").render(&config(Toolbelt::evidence_only().and(
            Grant::RunADeclaredCommand(String::from("git --namespace n log")),
        )));
    assert!(
        matches!(
            refused,
            Err(HarnessRefused::CommandCarriesAGitGlobalFlag { .. })
        ),
        "{refused:?}"
    );
}

/// The control for both: a command that only mentions `branch` or a lone `-`
/// in passing, on a program that is not git, is not refused.
#[test]
fn a_command_that_only_looks_like_either_gap_is_not_refused() {
    for run in [
        "cargo build --features branch",
        "npm run -s build",
        "make -j4",
    ] {
        let rendered = HeadlessAgent::at("/usr/local/bin/agent").render(&config(
            Toolbelt::evidence_only().and(Grant::RunADeclaredCommand(String::from(run))),
        ));
        assert!(rendered.is_ok(), "`{run}` was refused and should not be");
    }
}

/// The Done-when for the global-flag gap: one rule, and it is wide enough to
/// replace the three named flags without narrowing what a read-only grant
/// still renders.
#[test]
fn the_rendered_deny_list_carries_one_rule_for_every_global_flag() {
    let args = rendered(Toolbelt::evidence_only().and(Grant::ReadTheRepository));
    let denied = value_after(&args, "--disallowedTools").expect("a deny list is rendered");
    let entries: Vec<&str> = denied.split(',').collect();
    assert!(entries.contains(&"Bash(git -*)"), "{denied}");
    assert!(
        !entries.contains(&"Bash(git -C:*)")
            && !entries.contains(&"Bash(git --git-dir:*)")
            && !entries.contains(&"Bash(git --work-tree:*)"),
        "the three named flags should be subsumed by the one rule: {denied}"
    );
    let allowed = value_after(&args, "--allowedTools").expect("an allowlist is rendered");
    for rule in [
        "Bash(git status:*)",
        "Bash(git log:*)",
        "Bash(git rev-parse:*)",
    ] {
        assert!(
            allowed.split(',').any(|entry| entry == rule),
            "a bare read-only verb should still render: missing `{rule}`: {allowed}"
        );
    }
}
