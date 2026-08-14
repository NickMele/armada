//! What this machine has, and what it does not.
//!
//! One pass, read by two verbs: `armada init`'s checklist and `armada doctor`'s
//! first group. **Written once because they are asking one question** — the two
//! drifting apart is how a machine passes `init` and fails `doctor` on the same
//! afternoon.
//!
//! # Missing `claude` is fatal and a missing container runtime is not
//!
//! `docs/commands/init.md` settles it, and the reason is worth keeping in
//! sight: everything Fleet and Helm do runs through `claude`, so a machine
//! without it cannot do the thing Armada exists for. Not every repository needs
//! a container runtime, so a machine without one is a machine that will work
//! for most of them — reported, and not refused.
//!
//! # Every version comes from the tool itself
//!
//! Through `ctx.run` (`ARCHITECTURE.md` §1.1), so a test asserts the argv and
//! nothing here shells out to a `which`. A tool that is not on `PATH` fails to
//! spawn, and that failure *is* the answer.

use armada_core::ctx::{Run, RunRequest, StdioMode};
use armada_core::envelope::{Finding, Health};
use armada_core::error::{ArmadaError, ErrClass};
use std::path::Path;
use std::time::Duration;

/// Armada's own deadline on a `--version` call.
///
/// Short on purpose. A version banner that takes five seconds is a broken tool,
/// and `doctor` is the command someone runs *because* something is wrong — it
/// must not be the thing that hangs.
const DEADLINE: Duration = Duration::from_secs(10);

/// One tool Armada needs, and what it is for.
struct Tool {
    /// `argv[0]`.
    program: &'static str,
    /// What to do about it when it is not there. `None` means it is fatal, and
    /// no remedy line can help.
    when_missing: &'static str,
    /// Whether Armada refuses to continue without it.
    fatal: bool,
}

/// The three, in the order `docs/commands/init.md` lists them.
const TOOLS: [Tool; 3] = [
    Tool {
        program: "git",
        when_missing: "install git: the guild is a git repository",
        fatal: false,
    },
    Tool {
        program: "claude",
        when_missing: "install claude: everything Fleet and Helm do runs through it",
        fatal: true,
    },
    Tool {
        program: "docker",
        when_missing: "not required by every repo",
        fatal: false,
    },
];

/// What the preflight found, and whether Armada may go on.
pub struct Preflight {
    /// One row per tool.
    pub results: Vec<Finding>,
    /// The fatal absence, if there was one.
    pub fatal: Option<ArmadaError>,
}

/// Check every tool.
///
/// `detail` is the version when it is there. `remedy` is what to do when it is
/// not — which for a fatal absence is stated in the error rather than in a `→`
/// line, because there is nothing to read past it.
pub fn run(run: &impl Run, cwd: &Path, doctor: bool) -> Preflight {
    let mut results = Vec::new();
    let mut fatal = None;

    for tool in TOOLS {
        match version(run, cwd, tool.program) {
            Some(found) => results.push(Finding {
                check: tool.program.to_string(),
                // **`found` for `init`, `ok` for `doctor`.** A preflight is
                // *looking* for something and a check is *confirming* it, and
                // the two verbs say so in their own words.
                status: if doctor { Health::Ok } else { Health::Found },
                detail: found,
                remedy: None,
            }),
            None => {
                if tool.fatal {
                    fatal.get_or_insert(ArmadaError {
                        class: ErrClass::Environment,
                        r#where: tool.program.to_string(),
                        message: format!("`{}` is not on PATH", tool.program),
                        next_action: Some(tool.when_missing.to_string()),
                    });
                }
                results.push(Finding {
                    check: tool.program.to_string(),
                    status: Health::Missing,
                    detail: tool.when_missing.to_string(),
                    remedy: doctor.then(|| tool.when_missing.to_string()),
                });
            }
        }
    }

    Preflight { results, fatal }
}

/// A tool's version, or `None` when it is not there or would not say.
fn version(run: &impl Run, cwd: &Path, program: &str) -> Option<String> {
    let request = RunRequest::new(
        vec![program.to_string(), "--version".to_string()],
        cwd.to_path_buf(),
    )
    .stdio(StdioMode::Capture)
    .timeout(DEADLINE);
    let output = run.call(&request).ok()?;
    if !output.ok() {
        return None;
    }
    parse_version(&output.stdout)
}

/// The version out of a `--version` banner.
///
/// **The first dotted number on the first line, and nothing cleverer.** Every
/// one of the three prints something different around it — `git version 2.51.0`,
/// `2.0.14 (Claude Code)`, `Docker version 27.4.0, build bde2b89` — and a parser
/// per tool is three parsers to keep working. A banner with no number at all
/// falls back to the whole first line, because *something* is more useful than a
/// row that reads as missing when the tool is there.
fn parse_version(banner: &str) -> Option<String> {
    let line = banner.lines().find(|l| !l.trim().is_empty())?.trim();
    let number = line
        .split(|c: char| c.is_whitespace() || c == ',' || c == '(' || c == ')')
        .find(|word| {
            let trimmed = word.trim_start_matches('v');
            trimmed.contains('.')
                && trimmed.starts_with(|c: char| c.is_ascii_digit())
                && trimmed
                    .chars()
                    .all(|c| c.is_ascii_digit() || c == '.' || c == '-' || c.is_ascii_alphabetic())
        })
        .map(|word| word.trim_start_matches('v').to_string());
    Some(number.unwrap_or_else(|| line.to_string()))
}

#[cfg(test)]
mod tests {
    use super::*;
    use armada_core::ctx::{RunOutput, SpawnError, SpawnErrorKind};
    use std::cell::RefCell;

    struct Fake {
        banners: Vec<(&'static str, &'static str)>,
        calls: RefCell<Vec<Vec<String>>>,
    }

    impl Run for Fake {
        fn call(&self, request: &RunRequest) -> Result<RunOutput, SpawnError> {
            self.calls.borrow_mut().push(request.argv.clone());
            let program = request.argv[0].as_str();
            match self.banners.iter().find(|(name, _)| *name == program) {
                Some((_, banner)) => Ok(RunOutput {
                    code: Some(0),
                    signal: None,
                    stdout: (*banner).to_string(),
                    stderr: String::new(),
                    timed_out: false,
                }),
                None => Err(SpawnError {
                    program: program.to_string(),
                    kind: SpawnErrorKind::NotFound,
                    message: "No such file or directory".to_string(),
                }),
            }
        }
    }

    fn fake(banners: &[(&'static str, &'static str)]) -> Fake {
        Fake {
            banners: banners.to_vec(),
            calls: RefCell::new(Vec::new()),
        }
    }

    /// The three real banners, which is why this is one parser and not three.
    #[test]
    fn every_tools_banner_yields_the_number_in_it() {
        assert_eq!(parse_version("git version 2.51.0\n").unwrap(), "2.51.0");
        assert_eq!(parse_version("2.0.14 (Claude Code)\n").unwrap(), "2.0.14");
        assert_eq!(
            parse_version("Docker version 27.4.0, build bde2b89\n").unwrap(),
            "27.4.0"
        );
        assert_eq!(parse_version("podman version 5.3.1\n").unwrap(), "5.3.1");
    }

    /// A banner with no number is reported whole. Something is more useful than
    /// a row that reads as missing when the tool is there.
    #[test]
    fn a_banner_with_no_number_is_reported_rather_than_dropped() {
        assert_eq!(parse_version("nightly build\n").unwrap(), "nightly build");
        assert_eq!(parse_version(""), None);
    }

    #[test]
    fn a_tool_that_answers_is_found_with_its_version() {
        let run = fake(&[
            ("git", "git version 2.51.0\n"),
            ("claude", "2.0.14 (Claude Code)\n"),
            ("docker", "Docker version 27.4.0, build bde2b89\n"),
        ]);
        let preflight = run_at(&run, false);
        assert!(preflight.fatal.is_none());
        assert_eq!(preflight.results[0].detail, "2.51.0");
        assert_eq!(preflight.results[0].status, Health::Found);
        assert_eq!(
            run.calls.borrow()[0],
            ["git", "--version"],
            "the version comes from the tool, never from a `which`"
        );
    }

    /// **A missing container runtime is reported and not refused**, because not
    /// every repository needs one — and the DETAIL is what says so.
    #[test]
    fn a_missing_container_runtime_is_not_fatal() {
        let run = fake(&[("git", "git version 2.51.0\n"), ("claude", "2.0.14\n")]);
        let preflight = run_at(&run, false);
        assert!(preflight.fatal.is_none(), "a missing docker stopped init");
        assert_eq!(preflight.results[2].status, Health::Missing);
        assert_eq!(preflight.results[2].detail, "not required by every repo");
    }

    /// **A missing `claude` is fatal**, and it is `environment` — the machine is
    /// broken and the repository is fine, so the documented response is "fix the
    /// machine, then retry unchanged".
    #[test]
    fn a_missing_claude_is_fatal_and_is_an_environment_failure() {
        let run = fake(&[("git", "git version 2.51.0\n")]);
        let preflight = run_at(&run, false);
        let fatal = preflight.fatal.expect("a machine with no claude passed");
        assert_eq!(fatal.class, ErrClass::Environment);
        assert_eq!(fatal.class.exit_code(), 6);
        assert!(fatal.next_action.is_some());
    }

    /// **`found` for `init` and `ok` for `doctor`, one pass either way.** The
    /// two verbs are asking one question and answering it in their own words.
    #[test]
    fn the_same_pass_speaks_each_verbs_own_word() {
        let run = fake(&[
            ("git", "git version 2.51.0\n"),
            ("claude", "2.0.14\n"),
            ("docker", "Docker version 27.4.0\n"),
        ]);
        assert_eq!(run_at(&run, false).results[0].status, Health::Found);
        assert_eq!(run_at(&run, true).results[0].status, Health::Ok);
    }

    /// A `→` line is `doctor`'s, and `init` has none: `init` is a transcript of
    /// a setup, not a report of what to go and fix.
    #[test]
    fn only_doctor_carries_the_line_that_names_the_fix() {
        let run = fake(&[("git", "git version 2.51.0\n"), ("claude", "2.0.14\n")]);
        assert!(run_at(&run, false).results[2].remedy.is_none());
        assert_eq!(
            run_at(&run, true).results[2].remedy.as_deref(),
            Some("not required by every repo")
        );
    }

    /// Every call carries Armada's own deadline. `doctor` is run *because*
    /// something is wrong; it must not be the thing that hangs.
    #[test]
    fn every_version_call_is_bounded() {
        struct Deadlines(RefCell<Vec<Option<Duration>>>);
        impl Run for Deadlines {
            fn call(&self, request: &RunRequest) -> Result<RunOutput, SpawnError> {
                self.0.borrow_mut().push(request.timeout);
                Ok(RunOutput {
                    code: Some(0),
                    signal: None,
                    stdout: "1.0.0\n".to_string(),
                    stderr: String::new(),
                    timed_out: false,
                })
            }
        }
        let run = Deadlines(RefCell::new(Vec::new()));
        run_at(&run, false);
        assert_eq!(run.0.borrow().len(), TOOLS.len());
        for deadline in run.0.borrow().iter() {
            assert_eq!(*deadline, Some(DEADLINE));
        }
    }

    fn run_at(run: &impl Run, doctor: bool) -> Preflight {
        super::run(run, Path::new("/scratch"), doctor)
    }
}
