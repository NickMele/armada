//! Naming which tests failed, off a run's own summary.
//!
//! Every fixture below is text captured from a real run rather than invented
//! — `cargo nextest run -p checks-runner scratch_capture` over a scratch
//! module with two failing tests, and `vitest run` over a scratch file under
//! `packages/screens` with the same shape, both reverted after capture — so a
//! change to either runner's wording is what would break these.

use crate::failing::{failing_tests, failing_tests_in};
use crate::run::Output;

fn printed(stdout: &str, stderr: &str) -> Output {
    Output {
        stdout: stdout.to_string(),
        stderr: stderr.to_string(),
        truncated: false,
    }
}

/// `cargo nextest run -p checks-runner scratch_capture`, two failing tests,
/// one of them nested in a module.
const NEXTEST_TWO_FAILURES: &str = "\
────────────
     Summary [   0.021s] 2 tests run: 0 passed, 2 failed, 38 skipped
        FAIL [   0.016s] (1/2) checks-runner tests::scratch_capture::scratch_fails_one
        FAIL [   0.019s] (2/2) checks-runner tests::scratch_capture::scratch_nested::scratch_fails_two
error: test run failed
";

#[test]
fn nextests_own_summary_names_every_failing_test() {
    let output = printed("", NEXTEST_TWO_FAILURES);
    assert_eq!(
        failing_tests(&output),
        vec![
            "tests::scratch_capture::scratch_fails_one",
            "tests::scratch_capture::scratch_nested::scratch_fails_two",
        ]
    );
}

/// A line printed mid-run, before the summary, must not be read twice or read
/// at all: only the summary block is the one nextest means to be read back.
#[test]
fn a_fail_line_before_the_summary_is_not_counted() {
    let text = "        FAIL [   0.030s] (4/4) nt tests::mid_run_only\n".to_string()
        + NEXTEST_TWO_FAILURES;
    assert_eq!(
        failing_tests_in(&text),
        vec![
            "tests::scratch_capture::scratch_fails_one",
            "tests::scratch_capture::scratch_nested::scratch_fails_two",
        ]
    );
}

#[test]
fn nextests_own_no_tests_to_run_names_nothing() {
    let output = printed(
        "",
        "error: no tests to run\nSummary [   0.001s] 0 tests run: 0 passed, 32 skipped\n",
    );
    assert!(failing_tests(&output).is_empty());
}

/// `vitest run` over a scratch file under `packages/screens`, inside this
/// repository's own pnpm workspace — which is why the file carries a
/// `|screens|` project prefix a single-package run would not print.
const VITEST_TWO_FAILURES: &str = "
⎯⎯⎯⎯⎯⎯⎯ Failed Tests 2 ⎯⎯⎯⎯⎯⎯⎯

 FAIL  |screens| src/__scratch__/capture.test.ts > scratch capture > scratch fails one
AssertionError: expected 1 to be 2

 FAIL  |screens| src/__scratch__/capture.test.ts > scratch capture > nested > scratch fails two
Error: boom

 Test Files  1 failed (1)
      Tests  2 failed (2)
";

#[test]
fn vitests_own_failed_tests_block_names_every_leaf() {
    let output = printed(VITEST_TWO_FAILURES, "");
    assert_eq!(
        failing_tests(&output),
        vec!["scratch fails one", "scratch fails two"]
    );
}

/// A file that failed to load prints `FAIL` with no `>` in the line — a
/// transform error rather than a named test failing.
#[test]
fn vitests_own_file_load_failure_names_nothing() {
    let output = printed(
        " FAIL  broken.test.ts [ broken.test.ts ]\nReferenceError\n",
        "",
    );
    assert!(failing_tests(&output).is_empty());
}

#[test]
fn vitests_own_every_test_skipped_names_nothing() {
    let output = printed(
        " Test Files  1 skipped (1)\n      Tests  4 skipped (4)\n",
        "",
    );
    assert!(failing_tests(&output).is_empty());
}

#[test]
fn a_check_that_is_neither_runner_names_nothing() {
    let output = printed(
        "",
        "assertion `left == right` failed\n  left: 1\n right: 2\n",
    );
    assert!(failing_tests(&output).is_empty());
}

#[test]
fn the_same_name_from_both_readers_is_not_duplicated() {
    let text = format!("{VITEST_TWO_FAILURES}\n{VITEST_TWO_FAILURES}");
    assert_eq!(
        failing_tests_in(&text),
        vec!["scratch fails one", "scratch fails two"]
    );
}
