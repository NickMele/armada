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

/// A path under the system's temporary directory that no other test names.
fn scratch(name: &str) -> std::path::PathBuf {
    std::env::temp_dir().join(format!(
        "armada-{name}-{}-{}",
        std::process::id(),
        std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .map(|at| at.as_nanos())
            .unwrap_or_default()
    ))
}

/// Stop is a person's, and it ends the group the way the budget does: the
/// child the command left behind never gets to write its marker.
#[tokio::test]
async fn a_stopped_run_ends_its_group_and_keeps_what_printed() {
    let dir = scratch("stopped");
    std::fs::create_dir_all(&dir).expect("a directory to run in");
    let started = Instant::now();
    let ran = crate::run::run_until(
        "/bin/sh -c '(sleep 1; touch marker) & echo before; wait'",
        &dir,
        Duration::from_secs(60),
        crate::run::Writing::Nowhere,
        &[],
        tokio::time::sleep(Duration::from_millis(300)),
    )
    .await;
    assert!(started.elapsed() < Duration::from_secs(10));
    tokio::time::sleep(Duration::from_millis(1_500)).await;
    let marker = dir.join("marker").exists();
    let _ = std::fs::remove_dir_all(&dir);

    assert_eq!(
        ran.exit,
        Exit::Signalled {
            signal: libc::SIGKILL
        }
    );
    assert_eq!(ran.output.stdout, "before\n", "what printed before is kept");
    assert!(!marker, "the child outlived the stop");
}

/// A Check's prerequisites and the Check itself read as one log.
#[tokio::test]
async fn an_appended_log_keeps_what_was_there() {
    let live = scratch("appended.log");
    std::fs::write(&live, "earlier\n").expect("a log with a line in it");
    let ran = crate::run::run_until(
        "/bin/echo later",
        anywhere(),
        Duration::from_secs(10),
        crate::run::Writing::Appending(&live),
        &[],
        std::future::pending(),
    )
    .await;
    let whole = std::fs::read_to_string(&live).unwrap_or_default();
    let _ = std::fs::remove_file(&live);
    assert_eq!(ran.exit, Exit::Code(0));
    assert_eq!(whole, "earlier\nlater\n");
}

#[tokio::test]
async fn output_comes_back_for_a_person_to_read() {
    let ran = attempt("/bin/echo the suite is unhappy", Duration::from_secs(10)).await;
    assert_eq!(ran.output.stdout.trim(), "the suite is unhappy");
    assert!(ran.output.stderr.is_empty());
    assert!(!ran.output.truncated);
}

/// The claim `#628` rests on: a Check's output can be read while it runs.
///
/// The first line is in the file while the Check is still sleeping, and what
/// comes back at the end is exactly what [`run`] would have captured — the
/// file is a view of the run and changes nothing the gate reads.
#[tokio::test]
async fn output_is_written_down_while_the_check_is_still_running() {
    let live = std::env::temp_dir().join(format!(
        "armada-live-{}-{}.log",
        std::process::id(),
        std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .map(|at| at.as_nanos())
            .unwrap_or_default()
    ));
    let writing = live.clone();
    let running = tokio::spawn(async move {
        crate::run::run_writing(
            "/bin/sh -c 'echo first; sleep 1; echo second'",
            anywhere(),
            Duration::from_secs(10),
            Some(&writing),
        )
        .await
    });

    tokio::time::sleep(Duration::from_millis(400)).await;
    let so_far = std::fs::read_to_string(&live).unwrap_or_default();
    assert_eq!(so_far, "first\n", "read while the Check slept");
    assert!(!running.is_finished(), "the Check had not ended yet");

    let ran = running.await.expect("the run");
    let whole = std::fs::read_to_string(&live).unwrap_or_default();
    let _ = std::fs::remove_file(&live);
    assert_eq!(ran.exit, Exit::Code(0));
    assert_eq!(ran.output.stdout, "first\nsecond\n");
    assert_eq!(whole, "first\nsecond\n");
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

// ---------------------------------------------------------------- narrowing
//
// **No process here**, unlike everything above: assembling a command line is a
// function over strings, and the half that spawns is tested by what spawns.

mod narrowing {
    use core_model::{Covers, Narrowing, PathPattern};

    use crate::narrow::{narrowed, Narrowed};

    fn covers(patterns: &[&str]) -> Option<Covers> {
        Covers::of(
            patterns
                .iter()
                .map(|written| PathPattern::parse(written).expect("a pattern"))
                .collect(),
        )
    }

    fn paths(paths: &[&str]) -> Vec<String> {
        paths.iter().map(|path| path.to_string()).collect()
    }

    /// The derived case: a package name, from the layout the Manifest declared.
    fn by_package() -> Narrowing {
        Narrowing::declared(
            "cargo nextest run".to_string(),
            "-p {}".to_string(),
            None,
            Some("crates".to_string()),
            Vec::new(),
        )
    }

    /// The same, restating the exclusion the whole run makes.
    fn by_package_except(dropped: &str) -> Narrowing {
        Narrowing::declared(
            "cargo nextest run".to_string(),
            "-p {}".to_string(),
            None,
            Some("crates".to_string()),
            vec![dropped.to_string()],
        )
    }

    /// The verbatim case: the changed paths, filtered to the ones the command
    /// can read.
    fn by_file() -> Narrowing {
        Narrowing::declared(
            "rustfmt --check".to_string(),
            "{}".to_string(),
            covers(&["**/*.rs"]),
            None,
            Vec::new(),
        )
    }

    /// **The default, and the one that must never move.** A Check the Manifest
    /// said nothing about runs exactly as it always did.
    #[test]
    fn a_check_that_declares_no_narrowing_runs_whole() {
        assert_eq!(
            narrowed(None, &paths(&["crates/ipc/src/lib.rs"])),
            Narrowed::Whole
        );
    }

    #[test]
    fn a_path_under_the_declared_directory_becomes_the_directorys_child() {
        assert_eq!(
            narrowed(
                Some(&by_package()),
                &paths(&["crates/ipc/src/mcp/tools.rs"])
            ),
            Narrowed::To("cargo nextest run -p ipc".to_string())
        );
    }

    /// **One argument per crate and not per file.** Two files in one crate are
    /// one `-p`, because the second would cost a package resolution and buy
    /// nothing.
    #[test]
    fn two_files_in_one_crate_are_one_argument() {
        assert_eq!(
            narrowed(
                Some(&by_package()),
                &paths(&["crates/ipc/src/lib.rs", "crates/ipc/src/mcp/mod.rs"])
            ),
            Narrowed::To("cargo nextest run -p ipc".to_string())
        );
    }

    /// **Sorted, so two identical runs read identically.** A Drone comparing
    /// this run with the last one is comparing text.
    #[test]
    fn several_crates_arrive_in_one_order_however_the_diff_came_out() {
        let forwards = narrowed(
            Some(&by_package()),
            &paths(&["crates/store/src/lib.rs", "crates/fleet/src/lib.rs"]),
        );
        let backwards = narrowed(
            Some(&by_package()),
            &paths(&["crates/fleet/src/lib.rs", "crates/store/src/lib.rs"]),
        );
        assert_eq!(
            forwards,
            Narrowed::To("cargo nextest run -p fleet -p store".to_string())
        );
        assert_eq!(forwards, backwards);
    }

    /// **A change that touches nothing the Check narrows to is not a whole
    /// run.** The Drone asked what its own change broke, and the answer is that
    /// this Check has nothing to say about it.
    #[test]
    fn a_change_outside_the_declared_directory_narrows_to_nothing() {
        assert_eq!(
            narrowed(
                Some(&by_package()),
                &paths(&["docs/OPEN.md", "xtask/src/lib.rs"])
            ),
            Narrowed::Nothing
        );
    }

    /// The directory itself, with nothing under it, names no child.
    #[test]
    fn the_declared_directory_on_its_own_names_nothing() {
        assert_eq!(
            narrowed(Some(&by_package()), &paths(&["crates"])),
            Narrowed::Nothing
        );
    }

    #[test]
    fn the_verbatim_case_appends_the_changed_paths_themselves() {
        assert_eq!(
            narrowed(
                Some(&by_file()),
                &paths(&["crates/ipc/src/lib.rs", "xtask/src/rules.rs"])
            ),
            Narrowed::To("rustfmt --check crates/ipc/src/lib.rs xtask/src/rules.rs".to_string())
        );
    }

    /// **`from` is not `when`.** `format` covers every path in the repository
    /// and still reads only the Rust ones, so what feeds the narrowing is a
    /// separate list from what decides whether the Check runs at all.
    #[test]
    fn a_path_the_narrowing_does_not_read_contributes_nothing() {
        assert_eq!(
            narrowed(
                Some(&by_file()),
                &paths(&["docs/OPEN.md", "crates/ipc/src/lib.rs"])
            ),
            Narrowed::To("rustfmt --check crates/ipc/src/lib.rs".to_string())
        );
    }

    /// A path with a space is one argument, spelled the way `run`'s splitter
    /// takes quotes off — the reason this function lives in this crate.
    #[test]
    fn a_path_with_a_space_in_it_is_one_argument() {
        assert_eq!(
            narrowed(Some(&by_file()), &paths(&["docs/a note.rs"])),
            Narrowed::To("rustfmt --check \"docs/a note.rs\"".to_string())
        );
    }

    /// **A path this cannot spell abandons the narrowing rather than dropping
    /// the path.** Dropping it would run the Check over a subset nobody was
    /// told was a subset; running whole is never less than running narrow.
    #[test]
    fn a_path_holding_a_quote_falls_back_to_the_whole_check() {
        assert_eq!(
            narrowed(Some(&by_file()), &paths(&["docs/it's.rs"])),
            Narrowed::Whole
        );
    }

    /// **The one case where a narrowed run would be misleading rather than
    /// merely smaller.** The whole `test` run excludes `acceptance` because a
    /// milestone's claim is read on its own, and dropping `--workspace` so `-p`
    /// means anything drops `--exclude` with it. Without this the Drone is
    /// shown a bar the gate does not apply to it, and a Drone shown a red test
    /// chases it green.
    #[test]
    fn an_excluded_value_is_dropped_and_the_rest_of_the_change_is_not() {
        assert_eq!(
            narrowed(
                Some(&by_package_except("acceptance")),
                &paths(&["crates/acceptance/tests/board.rs", "crates/ipc/src/lib.rs"])
            ),
            Narrowed::To("cargo nextest run -p ipc".to_string())
        );
    }

    /// **A change that is nothing but excluded values narrows to nothing**, and
    /// is skipped rather than run whole: what the Drone touched is what the
    /// whole run would not have measured either.
    #[test]
    fn a_change_only_in_an_excluded_value_narrows_to_nothing() {
        assert_eq!(
            narrowed(
                Some(&by_package_except("acceptance")),
                &paths(&["crates/acceptance/tests/board.rs"])
            ),
            Narrowed::Nothing
        );
    }
}
