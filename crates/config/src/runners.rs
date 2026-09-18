//! The runners Armada ships a description of.
//!
//! **Data, not code** — `docs/concepts/runner-adapter.md`. A description holds
//! file signatures and template strings; nothing in one runs as a program, and
//! Fleet only ever interpolates into a fixed template. That is what makes a
//! learned one safe to keep and, later, to publish.
//!
//! **Beside , not under .** These are Armada's own
//! source, and everything under  but the workflows is a record of one
//! run on one machine and is ignored — a shipped description put there compiles
//! in the tree that wrote it and in no fresh checkout. A repository's own
//! description, when that is built, is what belongs under .
//!
//! **Shipped ones are compiled in, for the reason the workflow catalogue is.**
//! A repository that has never been configured still meets a Fleet that knows
//! `vitest`, and a description nobody can delete out from under a running Job
//! is one fewer thing a Job's behaviour depends on.
//!
//! Only `run_changed` is read today. The other five shapes the schema fixes are
//! in the concept page and not yet in any caller, so a description declaring
//! one is not refused — it is simply not asked for.

use serde_yaml_ng::Value;

/// One runner, as Armada ships it.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct RunnerDescription {
    name: String,
    run_changed: Option<String>,
    command: Option<String>,
}

impl RunnerDescription {
    /// The command that runs the tests reaching a set of changed files.
    /// **`None` where this runner has no such shape**, and then a Check driven
    /// by it runs whole however many files a Drone names — the fallback the
    /// schema fixes, which is always `run`.
    pub fn run_changed(&self) -> Option<&str> {
        self.run_changed.as_deref()
    }

    pub fn name(&self) -> &str {
        &self.name
    }

    /// The program a runnable's command names when this runner drives it.
    /// **`None` where the description claims none**, and then nothing detects
    /// it and a repository has to name it.
    pub fn command(&self) -> Option<&str> {
        self.command.as_deref()
    }
}

/// The runner a runnable's own command names, where a shipped description
/// claims that program.
///
/// **Every token, not the first.** A script reading `pnpm exec vitest run`
/// names vitest as surely as one reading `vitest run`, and which of the two a
/// repository writes is not something to be right about.
pub fn detected(run: &str) -> Option<String> {
    let named: Vec<&str> = run
        .split_whitespace()
        .filter(|token| !token.starts_with('-'))
        .collect();
    SHIPPED
        .iter()
        .filter_map(|text| read(text))
        .find(|runner| {
            runner
                .command()
                .is_some_and(|program| named.contains(&program))
        })
        .map(|runner| runner.name)
}

/// Every runner compiled in, in the order they are declared here.
const SHIPPED: &[&str] = &[include_str!("../runners/vitest.yml")];

/// The shipped description answering to `name`.
///
/// **`None` is a runner nobody has described**, which is the case the learn
/// path in the concept page exists for and which nothing here does anything
/// about: a Check naming an unknown runner runs whole, exactly as one naming
/// no runner does.
pub fn shipped(name: &str) -> Option<RunnerDescription> {
    SHIPPED
        .iter()
        .filter_map(|text| read(text))
        .find(|runner| runner.name == name)
}

/// One description, or `None` where it cannot be read.
///
/// **Unreadable is `None` rather than a refusal.** These are Armada's own
/// files, compiled in, and `every_shipped_runner_reads` is what catches a
/// broken one — at build time for whoever broke it, rather than at a gate for
/// somebody who did not.
fn read(text: &str) -> Option<RunnerDescription> {
    let document: Value = serde_yaml_ng::from_str(text).ok()?;
    let name = document.get("name")?.as_str()?.to_string();
    let run_changed = document
        .get("commands")
        .and_then(|commands| commands.get("run_changed"))
        .and_then(Value::as_str)
        .map(str::to_string);
    let command = document
        .get("detect")
        .and_then(|detect| detect.get("command"))
        .and_then(Value::as_str)
        .map(str::to_string);
    Some(RunnerDescription {
        name,
        run_changed,
        command,
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    /// Every compiled-in description parses and names itself. A description
    /// that does not read is one every Check naming it silently runs whole.
    #[test]
    fn every_shipped_runner_reads() {
        for text in SHIPPED {
            let runner = read(text).expect("a shipped runner reads");
            assert!(!runner.name().is_empty(), "it names itself");
        }
        assert_eq!(SHIPPED.len(), 1, "add the new one to this file's own list");
    }

    #[test]
    fn vitest_narrows_by_the_files_that_changed() {
        let vitest = shipped("vitest").expect("vitest ships");
        let run_changed = vitest.run_changed().expect("vitest narrows");
        assert!(run_changed.contains("{files}"), "{run_changed}");
        assert!(run_changed.contains("{dir}"), "{run_changed}");
        // The two properties #1205 and #1204 cost, written into the data
        // rather than into whoever reads it.
        assert!(
            run_changed.contains(" exec "),
            "through the package script, the arguments are swallowed: {run_changed}"
        );
        assert!(
            run_changed.contains("--passWithNoTests=false"),
            "without it a run matching nothing exits zero and reads as a pass: {run_changed}"
        );
    }

    /// **Every token, not the first.** Which of these two spellings a
    /// repository writes is not something to be right about.
    #[test]
    fn a_script_naming_a_shipped_runner_is_detected_however_it_is_spelled() {
        assert_eq!(detected("vitest run"), Some("vitest".to_string()));
        assert_eq!(detected("pnpm exec vitest run"), Some("vitest".to_string()));
        assert_eq!(
            detected("vitest --coverage run"),
            Some("vitest".to_string()),
            "a flag between the program and the rest changes nothing"
        );
    }

    #[test]
    fn a_script_naming_nothing_shipped_is_detected_as_nothing() {
        assert_eq!(detected("node --test"), None);
        assert_eq!(
            detected("jest --ci"),
            None,
            "not shipped yet, and not guessed at"
        );
        assert_eq!(detected(""), None);
    }

    /// A flag spelling a runner's name is a flag.
    #[test]
    fn a_flag_is_never_read_as_the_program() {
        assert_eq!(detected("node --test --vitest"), None);
    }

    #[test]
    fn a_runner_nobody_ships_is_none() {
        assert_eq!(shipped("nose"), None);
    }
}
