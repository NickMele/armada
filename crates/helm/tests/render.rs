//! The render layer, through the real binary — which is the only place the
//! three audiences of PLAN.md §3.1.1 are distinguishable.
//!
//! **A test harness captures output, so stdout here is never a terminal.** That
//! is not a limitation of the suite; it is the audience that matters most. An
//! agent calling `armada manifest status` and reading what comes back is in
//! exactly this position, and everything below asserts what it receives.
//!
//! The unit tests in `render/style.rs` prove the decision table. These prove the
//! decision reaches the bytes.

mod support;

use support::Machine;

/// Anything a terminal would interpret rather than display.
fn has_ansi(bytes: &[u8]) -> bool {
    bytes.contains(&0x1b)
}

/// **The non-TTY human audience gets no escape sequences, anywhere.**
///
/// Not a nicety: an agent captures stdout into a string, and `\x1b[38;2;…m`
/// wrapped around a status word is worse than the colour being absent — it is a
/// value the agent has to learn to strip before comparing anything.
#[test]
fn nothing_a_captured_stdout_receives_contains_an_ansi_escape() {
    let machine = Machine::new();
    let repo = machine.repo("main", CONFIG);
    machine.run(&repo, &["manifest", "init"]);

    for args in [
        &["--help"][..],
        &["--version"][..],
        &[][..],
        &["manifest", "status"][..],
        &["manifest", "init"][..],
        &["manifest", "check", "--dry-run"][..],
    ] {
        let output = machine.run(&repo, args);
        assert!(
            !has_ansi(&output.stdout),
            "`armada {}` painted a captured stdout: {:?}",
            args.join(" "),
            String::from_utf8_lossy(&output.stdout)
        );
    }
}

/// A refusal goes to stderr, and stderr is captured just as often — frequently
/// into a log file even when someone is watching stdout.
#[test]
fn a_refusal_on_a_captured_stderr_contains_no_ansi_escape() {
    let machine = Machine::new();
    let repo = machine.repo("main", CONFIG);

    let output = machine.run(&repo, &["manifest", "init", "--turbo"]);
    assert_eq!(output.status.code(), Some(2));
    assert!(!output.stderr.is_empty(), "the refusal was reported");
    assert!(
        !has_ansi(&output.stderr),
        "{:?}",
        String::from_utf8_lossy(&output.stderr)
    );
}

/// `--color always` is the flag for pushing colour through a pager, so it has to
/// actually reach a pipe — otherwise the test above proves nothing about the
/// `auto` path, only that colour is broken everywhere.
#[test]
fn color_always_paints_a_pipe() {
    let machine = Machine::new();
    let repo = machine.repo("main", CONFIG);
    machine.run(&repo, &["manifest", "init"]);

    let output = machine.run(&repo, &["manifest", "init", "--color", "always"]);
    assert!(
        has_ansi(&output.stdout),
        "--color always did not reach the pipe: {:?}",
        String::from_utf8_lossy(&output.stdout)
    );
}

/// **The documented tie-break, end to end: `NO_COLOR` beats `--color always`.**
///
/// The two rules genuinely conflict — `render.md` lists `NO_COLOR` as a peer of
/// the three `--color` values, and PLAN.md §3.1.1 says arguing with it costs a
/// bug report from someone who set it deliberately. `render/style.rs` records
/// why this is the direction the conflict resolves in.
///
/// **Whatever its value**, per the standard: `NO_COLOR=0` is still `NO_COLOR`,
/// because the variable's presence is the signal.
#[test]
fn no_color_suppresses_colour_even_under_color_always() {
    let machine = Machine::new();
    let repo = machine.repo("main", CONFIG);
    machine.run(&repo, &["manifest", "init"]);

    for value in ["1", "0", ""] {
        let output = machine.run_with_env(
            &repo,
            &["manifest", "init", "--color", "always"],
            &[("NO_COLOR", value)],
        );
        assert!(
            !has_ansi(&output.stdout),
            "NO_COLOR={value:?} was overruled: {:?}",
            String::from_utf8_lossy(&output.stdout)
        );
    }
}

/// `--color never` and a pipe produce the same bytes, which is what makes the
/// no-colour path a supported mode rather than a degraded one.
#[test]
fn color_never_and_an_ordinary_pipe_are_byte_identical() {
    let machine = Machine::new();
    let repo = machine.repo("main", CONFIG);
    machine.run(&repo, &["manifest", "init"]);

    let piped = machine.run(&repo, &["manifest", "status"]);
    let never = machine.run(&repo, &["manifest", "status", "--color", "never"]);
    assert_eq!(piped.stdout, never.stdout);
}

/// `--json` is a third audience and the renderer never touches it. The payload
/// is bytes a parser reads, so a colour flag must not reach it whatever it says.
#[test]
fn json_is_never_painted_whatever_color_asks_for() {
    let machine = Machine::new();
    let repo = machine.repo("main", CONFIG);

    let plain = machine.run(&repo, &["manifest", "init", "--json"]);
    let asked = machine.run(
        &repo,
        &["manifest", "status", "--json", "--color", "always"],
    );
    assert!(!has_ansi(&plain.stdout));
    assert!(
        !has_ansi(&asked.stdout),
        "--color always reached the envelope: {:?}",
        String::from_utf8_lossy(&asked.stdout)
    );
    assert!(serde_json::from_slice::<serde_json::Value>(&asked.stdout).is_ok());
}

const CONFIG: &str = "\
manifest:
  version: 1
  components:
    api:
      root: services/api
      setup: [\"true\"]
      run:
        driver: command
        cmd: ./serve
        ports: { api: 3000 }
      checks:
        lint: { cmd: \"./exiter.sh 0\", scope: component }
";
