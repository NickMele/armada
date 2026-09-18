//! A Check narrowed by the runner that drives it, where it declares no
//! narrowing of its own. #1456.
//!
//! **The subject is precedence and the fallbacks.** What a runner's shipped
//! description answers, what it must not override, and the four ways it can
//! have nothing to say — every one of which has to come out as the Check
//! running whole rather than as a command naming nothing.

use core_model::{Narrowing, ResolvedCheck, Runner, RunsAt};

use crate::checking::{by_its_runner, narrowed, Planned};

fn paths(of: &[&str]) -> Vec<String> {
    of.iter().map(|path| path.to_string()).collect()
}

/// One vitest-driven Check, narrowing declared or not.
fn check(runner: Option<Runner>, narrow: Option<Narrowing>) -> ResolvedCheck {
    ResolvedCheck::ManifestCheck {
        name: "screens_test".to_string(),
        run: "pnpm --dir packages/screens exec vitest run".to_string(),
        expect_exit_code: 0,
        when: None,
        requires: Vec::new(),
        narrow,
        one_test: None,
        runs_at: RunsAt::Everywhere,
        places: std::num::NonZeroU32::MIN,
        width: None,
        runner,
    }
}

fn vitest() -> Runner {
    Runner::declared("vitest".to_string(), Some("packages/screens".to_string()))
}

#[test]
fn a_check_naming_vitest_narrows_to_the_tests_reaching_what_changed() {
    let narrowed_to = by_its_runner(&check(Some(vitest()), None), &paths(&["src/overview.ts"]))
        .expect("vitest ships a run_changed");
    assert_eq!(
        narrowed_to,
        "pnpm --dir packages/screens exec vitest related src/overview.ts \
         --run --passWithNoTests=false"
    );
}

/// **The declaration in the file wins.** A repository that wrote how it wants
/// this Check narrowed has said so, and a shipped description of the runner
/// answers only where it has not — the precedence the concept page states for
/// a repository's own description against a shipped one, one tier down.
#[test]
fn a_checks_own_narrowing_is_answered_before_its_runners() {
    let own = Narrowing::declared(
        "pnpm --dir packages/screens exec vitest run".to_string(),
        "{}".to_string(),
        None,
        None,
        Vec::new(),
    );
    let planned = narrowed(
        &check(Some(vitest()), Some(own)),
        "screens_test",
        "pnpm --dir packages/screens exec vitest run",
        &paths(&["src/overview.ts"]),
        true,
    );
    let Planned::Command { narrowed_to, .. } = planned else {
        panic!("a Check with a narrowing of its own runs narrowed");
    };
    let narrowed_to = narrowed_to.expect("narrowed by its own declaration");
    assert!(
        !narrowed_to.contains("related"),
        "the runner's shape overrode the file's own: {narrowed_to}"
    );
}

/// Each of these is the Check running whole, which is never less than narrow.
#[test]
fn a_runner_with_nothing_to_say_leaves_the_check_whole() {
    assert_eq!(
        by_its_runner(&check(None, None), &paths(&["src/a.ts"])),
        None,
        "a Check naming no runner"
    );
    assert_eq!(
        by_its_runner(
            &check(Some(Runner::declared("nose".to_string(), None)), None),
            &paths(&["src/a.ts"])
        ),
        None,
        "a runner nobody ships a description of"
    );
    assert_eq!(
        by_its_runner(&check(Some(vitest()), None), &[]),
        None,
        "no changed paths to narrow to"
    );
    assert_eq!(
        by_its_runner(
            &check(Some(Runner::declared("vitest".to_string(), None)), None),
            &paths(&["src/a.ts"])
        ),
        None,
        "a template naming a package the Check does not declare"
    );
}

/// A narrowed run is never carried to a gate, so what this produces can only
/// ever tell a Drone where it stands. `reuse::KeptDryRun` is what holds that.
#[test]
fn what_a_runner_narrows_to_is_still_a_narrowed_run() {
    let planned = narrowed(
        &check(Some(vitest()), None),
        "screens_test",
        "pnpm --dir packages/screens exec vitest run",
        &paths(&["src/overview.ts"]),
        true,
    );
    let Planned::Command { narrowed_to, .. } = planned else {
        panic!("it narrows");
    };
    assert!(narrowed_to.is_some(), "recorded as narrowed, not as whole");
}
