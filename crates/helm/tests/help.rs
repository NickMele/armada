//! **Every verb has a `--help`, through the binary a person actually types.**
//!
//! The report that opened this milestone was one command:
//!
//! ```text
//! $ armada fleet spawn --help
//! error: unknown flag `--help`
//! ```
//!
//! The page existed for one verb out of twenty-one. `render::help`'s own tests
//! prove that a page can be *drawn* for each of them; this file proves the other
//! half — that typing `--help` after the verb **reaches** it, exits `0`, and puts
//! it on stdout. The two failures are different: a page nothing routes to is
//! still `unknown flag` to the reader.
//!
//! **The list is the parser's, not this file's.** `args::every_verb` enumerates
//! what the grammar accepts, so a verb added without a page fails here on the day
//! it ships rather than on the day somebody remembers to extend a hand-kept list
//! in a test. That is the whole reason the roster exists.

mod support;

use armada_helm::args::every_verb;
use support::Machine;

/// The words to type, for a verb named as `args::every_verb` names it.
fn line(path: &str) -> Vec<String> {
    path.split(' ').map(str::to_string).collect()
}

/// `armada <verb> --help`, for every verb, in both spellings.
///
/// **Run in a scratch `$HOME` like everything else in this suite.** Drawing a
/// page touches nothing, but a test that proves it by running against the
/// developer's `~/.armada/` proves it about their machine and not about Armada.
#[test]
fn every_verb_answers_help_on_stdout_and_exits_zero() {
    let machine = Machine::new();
    let cwd = machine.outside();

    for path in every_verb() {
        for spelling in ["--help", "-h"] {
            let mut words = line(&path);
            words.push(spelling.to_string());
            let argv: Vec<&str> = words.iter().map(String::as_str).collect();
            let output = machine.run(&cwd, &argv);

            assert!(
                output.status.success(),
                "`armada {path} {spelling}` exited {:?}: {}",
                output.status.code(),
                String::from_utf8_lossy(&output.stderr)
            );
            let page = String::from_utf8_lossy(&output.stdout);
            assert!(
                page.starts_with(&format!("armada {path} ")),
                "`armada {path} {spelling}` did not print that verb's page:\n{page}"
            );
            assert!(
                page.contains("USAGE"),
                "`armada {path} {spelling}` printed no USAGE"
            );
            assert!(
                page.contains("FLAGS"),
                "`armada {path} {spelling}` printed no FLAGS"
            );
            assert!(
                String::from_utf8_lossy(&output.stderr).is_empty(),
                "`armada {path} {spelling}` wrote to stderr"
            );
        }
    }
}

/// **`--help` is answered before the verb's own grammar refuses the line.**
///
/// `fleet spawn` needs a task and `fleet kill` needs a Job; the reader typing
/// `--help` is asking *what* to give it, so refusing them for the missing
/// argument would answer the question with the question.
#[test]
fn help_beats_a_missing_required_argument() {
    let machine = Machine::new();
    let cwd = machine.outside();

    for argv in [
        ["fleet", "spawn", "--help"],
        ["fleet", "kill", "--help"],
        ["fleet", "answer", "--help"],
        ["fleet", "board", "--help"],
        ["guild", "import", "--help"],
        ["manifest", "config", "--help"],
    ] {
        let output = machine.run(&cwd, &argv);
        assert!(
            output.status.success(),
            "`armada {}` was refused rather than answered: {}",
            argv.join(" "),
            String::from_utf8_lossy(&output.stderr)
        );
    }
}

/// The module pages, and the bare module name that is as incomplete as they are.
#[test]
fn every_module_answers_its_own_page_with_or_without_the_flag() {
    let machine = Machine::new();
    let cwd = machine.outside();

    for module in ["manifest", "guild", "fleet"] {
        for argv in [vec![module], vec![module, "--help"]] {
            let output = machine.run(&cwd, &argv);
            assert!(
                output.status.success(),
                "`armada {}` did not exit 0",
                argv.join(" ")
            );
            let page = String::from_utf8_lossy(&output.stdout);
            assert!(
                page.starts_with(&format!("armada {module} ")),
                "`armada {}` did not draw the {module} page",
                argv.join(" ")
            );
            assert!(
                page.contains("VERBS"),
                "`armada {}` listed no verbs",
                argv.join(" ")
            );
        }
    }
}

// A `commands:` child keeping its own `--help` is the other half of this rule,
// and `e2e.rs::help_answers_at_every_level_of_the_grammar` already runs it
// against a repository that declares one. Asserting it twice would mean two
// tests to update when the grammar changes and one of them getting missed.

/// A claimed name that is not built has no page, and says which milestone builds
/// it rather than drawing one it cannot honour.
#[test]
fn an_unbuilt_verb_has_no_page() {
    let machine = Machine::new();
    let cwd = machine.outside();

    // `guild edit` used to be here. It was reserved as *open a guild file,
    // validate it, commit it* and is now built to that contract (PLAN.md
    // §15.3.4), so `guild verify` is the claimed Guild name still unbuilt.
    for argv in [
        ["guild", "verify", "--help"],
        ["manifest", "explain", "--help"],
    ] {
        let output = machine.run(&cwd, &argv);
        assert!(
            !output.status.success(),
            "`armada {}` drew a page for a verb that does not exist",
            argv.join(" ")
        );
        let said = String::from_utf8_lossy(&output.stderr);
        assert!(
            said.contains("not built yet"),
            "`armada {}` said {said}",
            argv.join(" ")
        );
    }
}
