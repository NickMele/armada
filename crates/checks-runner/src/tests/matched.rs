//! Reading what [`one_test`](crate::one_test)'s command came to: a pass, a
//! failure, or the name matching nothing at all. #1204.
//!
//! Every fixture below is text captured from a real run rather than
//! invented — `cargo nextest run -p checks-runner -E
//! 'test(=this_test_name_does_not_exist_anywhere)'` and `pnpm --dir
//! packages/components exec vitest run -t "..."`, both ways — so a change to
//! either runner's wording is what would break these, not a guess about it.

use verification::Exit;

use crate::matched::{one_test_ran, OneTestRan};
use crate::run::Output;

fn printed(stdout: &str, stderr: &str) -> Output {
    Output {
        stdout: stdout.to_string(),
        stderr: stderr.to_string(),
        truncated: false,
    }
}

#[test]
fn nextests_own_no_tests_to_run_is_not_a_pass() {
    let output = printed(
        "Starting 0 tests across 1 binary (32 tests skipped)\n\
         Summary [   0.001s] 0 tests run: 0 passed, 32 skipped\n",
        "error: no tests to run\n(hint: use `--no-tests` to customize)\n",
    );
    // The current binary in this environment exits 4 here; an older one
    // reported 0 (#1204's own repro). Both are covered because this does
    // not gate on the code.
    for exit in [Exit::Code(0), Exit::Code(4)] {
        assert_eq!(one_test_ran(&exit, &output, 0), OneTestRan::NoMatch);
    }
}

/// A real pass, isolated to one test: `cargo nextest run -p checks-runner -E
/// 'test(=tests::a_command_that_succeeds_reports_its_code)'`.
#[test]
fn nextests_own_one_test_run_one_passed_is_a_pass() {
    let output = printed(
        "Starting 1 test across 1 binary (31 tests skipped)\n\
         Summary [   0.022s] 1 test run: 1 passed, 31 skipped\n",
        "",
    );
    assert_eq!(one_test_ran(&Exit::Code(0), &output, 0), OneTestRan::Passed);
}

/// vitest's own words for a `-t` filter matching nothing: every test in the
/// run is skipped, and vitest still exits `0`.
#[test]
fn vitests_own_every_test_skipped_is_not_a_pass() {
    let output = printed(
        " Test Files  149 skipped (149)\n      Tests  1059 skipped (1059)\n",
        "",
    );
    assert_eq!(
        one_test_ran(&Exit::Code(0), &output, 0),
        OneTestRan::NoMatch
    );
}

/// A real pass, isolated to one test: `pnpm --dir packages/components exec
/// vitest run -t "The id and version, read-only"`.
#[test]
fn vitests_own_one_passed_is_a_pass() {
    let output = printed(
        " Test Files  1 passed | 148 skipped (149)\n      Tests  1 passed | 1058 skipped (1059)\n",
        "",
    );
    assert_eq!(one_test_ran(&Exit::Code(0), &output, 0), OneTestRan::Passed);
}

/// A real match that fails is neither of the other two.
#[test]
fn a_real_match_that_fails_is_a_failure_not_a_no_match() {
    let output = printed(" Tests  1 failed | 1058 skipped (1059)\n", "");
    assert_eq!(one_test_ran(&Exit::Code(1), &output, 0), OneTestRan::Failed);
}

/// No output at all — neither runner's summary — falls back to the exit code
/// exactly as before this existed.
#[test]
fn no_output_falls_back_to_the_exit_code() {
    let output = printed("", "");
    assert_eq!(one_test_ran(&Exit::Code(0), &output, 0), OneTestRan::Passed);
    assert_eq!(one_test_ran(&Exit::Code(1), &output, 0), OneTestRan::Failed);
}
