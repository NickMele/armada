//! What a Check's run reports, especially when it does not run.
//!
//! These spawn real processes, which is the point: the three failures that
//! matter here — a command that is not installed, a command that hangs, a
//! command killed by a signal — are all operating system behaviour, and a fake
//! that returned the right enum would be asserting that the test author knows
//! what the operating system does.
//!
//! Every program used is one that ships with the machine, and every one of them
//! finishes in milliseconds or is killed.

use std::path::Path;
use std::time::{Duration, Instant};

use verification::{Exit, NeverRan};

use crate::run::{run, Attempt};

/// A directory that certainly exists and that no test writes to.
fn anywhere() -> &'static Path {
    Path::new("/")
}

async fn attempt(command: &str, budget: Duration) -> Attempt {
    run(command, anywhere(), budget).await
}

#[tokio::test]
async fn a_command_that_succeeds_reports_its_code() {
    let ran = attempt("/usr/bin/true", Duration::from_secs(10)).await;
    assert_eq!(ran.exit, Exit::Code(0));
}

#[tokio::test]
async fn a_command_that_fails_reports_its_code_rather_than_an_error() {
    let ran = attempt("/usr/bin/false", Duration::from_secs(10)).await;
    assert_eq!(ran.exit, Exit::Code(1));
}

#[tokio::test]
async fn a_command_that_does_not_exist_never_ran() {
    let ran = attempt("armada-no-such-program", Duration::from_secs(10)).await;
    assert_eq!(
        ran.exit,
        Exit::NeverRan(NeverRan::NoSuchCommand {
            program: "armada-no-such-program".to_string()
        })
    );
}

/// The failure a fresh machine actually produces, and the one a shell would
/// have reported as exit `127`. A step declaring `expect_exit_code: 127` must
/// not pass because the program was missing.
#[tokio::test]
async fn a_missing_command_is_not_reported_as_any_exit_code() {
    let ran = attempt("armada-no-such-program", Duration::from_secs(10)).await;
    assert!(!matches!(ran.exit, Exit::Code(_)));
}

#[tokio::test]
async fn an_empty_command_never_ran() {
    let ran = attempt("   ", Duration::from_secs(10)).await;
    assert_eq!(ran.exit, Exit::NeverRan(NeverRan::NothingToRun));
}

/// The assertion the milestone step names: a hanging Check fails rather than
/// hangs. The budget is short and the command would run for an hour.
#[tokio::test]
async fn a_hanging_check_fails_rather_than_hanging() {
    let budget = Duration::from_millis(300);
    let started = Instant::now();
    let ran = attempt("/bin/sleep 3600", budget).await;
    let took = started.elapsed();

    assert_eq!(ran.exit, Exit::TimedOut { after: budget });
    assert!(
        took < Duration::from_secs(10),
        "the run took {took:?}, so the budget did not end it"
    );
}

#[tokio::test]
async fn a_timed_out_check_is_not_reported_as_an_exit_code() {
    let ran = attempt("/bin/sleep 3600", Duration::from_millis(200)).await;
    assert!(!matches!(ran.exit, Exit::Code(_)));
}

/// A Check that spawns something slower than itself. The parent exits at once
/// and the group is what the budget has to end — killing only the process Fleet
/// started would leave the child holding the worktree.
#[tokio::test]
async fn a_check_whose_child_outlives_it_is_ended_with_it() {
    let budget = Duration::from_millis(400);
    let started = Instant::now();
    // The shell exits immediately; `sleep` inherits the pipe and keeps it open,
    // so the read only finishes when the whole group is gone.
    let ran = attempt("/bin/sh -c 'sleep 3600 & exit 0'", budget).await;
    let took = started.elapsed();

    assert_eq!(ran.exit, Exit::TimedOut { after: budget });
    assert!(
        took < Duration::from_secs(10),
        "the run took {took:?}, so the group outlived the budget"
    );
}

#[tokio::test]
async fn output_comes_back_for_a_person_to_read() {
    let ran = attempt("/bin/echo the suite is unhappy", Duration::from_secs(10)).await;
    assert_eq!(ran.output.stdout.trim(), "the suite is unhappy");
    assert!(ran.output.stderr.is_empty());
    assert!(!ran.output.truncated);
}

#[tokio::test]
async fn a_command_is_split_into_a_program_and_its_arguments() {
    let ran = attempt("/bin/echo 'one two' three", Duration::from_secs(10)).await;
    assert_eq!(ran.output.stdout.trim(), "one two three");
}

#[tokio::test]
async fn the_worktree_is_where_the_check_runs() {
    let ran = run("/bin/pwd", Path::new("/tmp"), Duration::from_secs(10)).await;
    assert_eq!(ran.exit, Exit::Code(0));
    // macOS resolves /tmp through a symlink, so the assertion is on the tail.
    assert!(
        ran.output.stdout.trim().ends_with("/tmp"),
        "ran in {}",
        ran.output.stdout.trim()
    );
}

#[tokio::test]
async fn a_worktree_that_is_not_there_never_ran() {
    let ran = run(
        "/bin/pwd",
        Path::new("/armada-no-such-worktree"),
        Duration::from_secs(10),
    )
    .await;
    assert_eq!(
        ran.exit,
        Exit::NeverRan(NeverRan::WorktreeGone {
            worktree: "/armada-no-such-worktree".to_string()
        })
    );
}

#[tokio::test]
async fn a_check_killed_by_a_signal_has_no_exit_code_at_all() {
    let ran = attempt("/bin/sh -c 'kill -9 $$'", Duration::from_secs(10)).await;
    assert_eq!(ran.exit, Exit::Signalled { signal: 9 });
}
