//! `armada doctor` — what this machine is missing or has drifted on.
//!
//! Read-only, and **safe to run in a shell prompt**: a warning alone does not
//! fail (`docs/commands/doctor.md`), so a guild three commits behind does not
//! make every prompt red.
//!
//! # The groups, in order
//!
//! | | Reports |
//! |---|---|
//! | **Tooling** | `git`, `claude`, a container runtime: present, and version |
//! | **Layout** | `~/.armada/` with its directories and `machine.yml` |
//! | **Guild drift** | behind, ahead of, or diverged from the remote, and by how many commits |
//! | **Fragments** | which of the three are still whatever import produced |
//!
//! **Guild drift is the check that earns the command.** Two machines silently
//! diverging is the guild's main failure mode (`PHASES.md` §11), and it is the
//! one thing nothing else on the machine would ever tell you.
//!
//! **Projection is not here yet, and its absence is deliberate rather than an
//! oversight.** `doctor.md` names a fourth group — whether the guild content
//! Claude Code is actually reading matches what the guild says it should be —
//! and there is nothing to compare against until the projector exists
//! (`PLAN.md` §13.2). Reporting `ok` for a check that ran nothing would be the
//! worse of the two failures: a `doctor` that says a machine is fine when
//! nobody looked.
//!
//! # Offline is not a failure
//!
//! The drift check needs the network and degrades to `offline` without it,
//! which is a warning. A `doctor` that failed on a train would be a `doctor`
//! nobody runs on a train, which is where you most want it.

use armada_core::ctx::{Run, RunRequest};
use armada_core::envelope::{DoctorData, Envelope, Finding, Problem, Settled};
use armada_core::error::ArmadaError;
use armada_guild::layout::{self, Guild, DIRECTORIES};
use armada_guild::{machine, memory, repo};
use std::path::Path;

use crate::verbs::guild::Where;
use crate::verbs::{preflight, Output};

/// Look at this machine and report.
///
/// `--fix` is accepted and refused by name: repairing a drifted guild is
/// `armada guild pull`, which the `→` line already names, and a flag that
/// silently did half of what it promised would be worse than one that says it
/// is not built.
pub fn run(runner: &impl Run, place: &Where) -> Result<Output, ArmadaError> {
    let guild = place.guild();
    // **Ordered so that a check's rows are contiguous**, because the render
    // draws one table per check (`render.rs`). `guild` reports drift and then
    // every fragment still as imported, and interleaving those with `manifest.db`
    // is what made a real reader ask which rows belonged together.
    let mut results = preflight::run(runner, &place.cwd, true).results;
    results.push(drone_argv(runner, &place.cwd));
    results.push(directories(&place.armada_home));
    results.extend(drift(runner, &guild));
    results.extend(fragments(&guild));
    results.push(store(&place.armada_home));

    let status = DoctorData::verdict(&results);
    Ok(Output::Doctor(Box::new(Envelope::ok(
        "doctor",
        None,
        status,
        DoctorData {
            tally: DoctorData::tally(&results),
            headline: DoctorData::headline(&results),
            results,
        },
    ))))
}

/// **Would a Drone actually start?**
///
/// This check exists because of a specific failure, and it is worth stating in
/// full. Fleet's Drone argv was `claude --session-id <uuid> --print
/// --output-format stream-json <prompt>`, which Claude Code rejects at
/// argument-parse time: that combination requires `--verbose`. Every test passed
/// — the unit tests on the built vector, and an integration test running a stub
/// that recorded what `execve` received — while **no Drone had ever run**.
///
/// > **Asserting on argv proves you built the argv you intended. It does not
/// > prove the argv is accepted.** A suite that only makes the first claim is
/// > green on a program that never starts (`docs/traps.md`).
///
/// So this asks the binary, twice, and neither question costs a token:
///
/// 1. **Every flag the Drone uses is still offered**, read off `claude --help`.
///    That catches a flag renamed or removed by a new version — the general
///    shape of the failure this is a sample of.
/// 2. **The real argument validator accepts the real combination.** The Drone's
///    own argv is run with its prompt replaced by `--input-format stream-json`
///    and nothing on stdin: Claude Code validates every flag, starts the
///    session, gets EOF and exits **without making an API call**
///    ([`armada_core::fleet::drone::probe_argv`]). A usage error there is the
///    finding, verbatim.
///
/// **The residual, stated rather than hidden.** `claude --help` does not
/// document the `--verbose` requirement, and `--help` short-circuits before
/// validation — so nothing can enumerate combination rules in advance. Probe 2
/// covers the combination Armada actually uses, which is the one that matters,
/// and it would have caught the failure this check was written after.
fn drone_argv(runner: &impl Run, cwd: &Path) -> Finding {
    use armada_core::fleet::drone as argv;

    let help = runner.call(
        &RunRequest::new(
            vec![argv::CLAUDE.to_string(), "--help".to_string()],
            cwd.to_path_buf(),
        )
        .timeout(PROBE),
    );
    let Ok(help) = help.and_then(|output| match output.ok() {
        true => Ok(output),
        false => Err(armada_core::ctx::SpawnError {
            program: argv::CLAUDE.to_string(),
            kind: armada_core::ctx::SpawnErrorKind::Other,
            message: "`--help` exited non-zero".to_string(),
        }),
    }) else {
        // A `claude` that will not answer `--help` is already reported by the
        // tooling check above; saying it twice would be noise.
        return Finding::needs(
            "drone argv",
            Problem::Offline,
            "`claude --help` did not answer, so the argv was not checked",
            "check `claude --help` runs",
        );
    };

    let missing: Vec<&str> = argv::FLAGS
        .iter()
        .copied()
        .filter(|flag| !help.stdout.contains(flag))
        .collect();
    if !missing.is_empty() {
        return Finding::needs(
            "drone argv",
            Problem::Missing,
            format!(
                "`claude` no longer offers {}: a Drone would not start",
                missing.join(", ")
            ),
            "pin an older claude, or report this to Armada",
        );
    }

    // The real validator, on the real combination.
    let Ok(output) =
        runner.call(&RunRequest::new(argv::probe_argv(), cwd.to_path_buf()).timeout(PROBE))
    else {
        return Finding::needs(
            "drone argv",
            Problem::Offline,
            "the argv probe would not run",
            "check `claude` runs",
        );
    };

    let said = format!("{}{}", output.stderr, output.stdout);
    if let Some(complaint) = said
        .lines()
        .map(str::trim)
        .find(|line| line.starts_with("Error:"))
    {
        return Finding::needs(
            "drone argv",
            Problem::Missing,
            format!("`claude` refuses the Drone's argv: {complaint}"),
            "armada cannot start a Drone until this is fixed",
        );
    }

    // **A turn would mean the probe cost something, and it must never.** No
    // `result` event is emitted when there is no input to answer; one appearing
    // says a future version began answering an empty conversation, and that is
    // reported rather than quietly paid for.
    if said.contains("total_cost_usd") {
        return Finding::needs(
            "drone argv",
            Problem::Partial,
            "the argv probe started a turn; it is meant to cost nothing",
            "report this to Armada: the doctor probe needs narrowing",
        );
    }

    Finding::settled(
        "drone argv",
        Settled::Ok,
        format!("{} flags accepted", argv::FLAGS.len()),
    )
}

/// The deadline on either half of the argv probe. Both are local; the probe
/// starts a session and exits at EOF, so anything approaching this is a wedged
/// binary or a hook that hangs.
const PROBE: std::time::Duration = std::time::Duration::from_secs(20);

/// `~/.armada/` and its three directories.
///
/// **The check is named for the thing, not for the code that checks it.** It
/// used to be called `layout` and to report `no jobs, workspaces` — an
/// implementation word over a set difference, which told a real reader neither
/// what was missing nor why he should care. It is now named `~/.armada` and says
/// which paths are absent and what writes to them.
fn directories(armada_home: &Path) -> Finding {
    let home = crate::verbs::machine::shown(armada_home);
    if !armada_home.is_dir() {
        return Finding::needs(
            &home,
            Problem::Missing,
            format!("{home} is not there; nothing Armada keeps is on this machine"),
            "armada init",
        );
    }
    let missing: Vec<&str> = DIRECTORIES
        .iter()
        .copied()
        .filter(|directory| !armada_home.join(directory).is_dir())
        .collect();
    if missing.is_empty() {
        return Finding::settled(
            &home,
            Settled::Ok,
            format!("{}, all present", paths(&DIRECTORIES)),
        );
    }
    Finding::needs(
        &home,
        Problem::Missing,
        format!(
            "{} {} missing; {} {} there",
            paths(&missing),
            if missing.len() == 1 { "is" } else { "are" },
            written(&missing.iter().map(|d| layout::holds(d)).collect::<Vec<_>>()),
            if missing.len() == 1 { "goes" } else { "go" },
        ),
        // **`--force`, and it no longer means "replace the guild".** A re-run
        // recreates what is missing and leaves an existing guild alone
        // (`verbs/machine.rs`), which is what makes naming it here safe.
        "armada init --force",
    )
}

/// `jobs/ and workspaces/` — directory names, written as paths so a reader can
/// see they are directories.
fn paths(directories: &[&str]) -> String {
    written(
        &directories
            .iter()
            .map(|d| format!("{d}/"))
            .collect::<Vec<_>>(),
    )
}

/// `a`, `a and b`, `a, b and c`.
fn written(items: &[impl AsRef<str>]) -> String {
    let items: Vec<&str> = items.iter().map(AsRef::as_ref).collect();
    match items.split_last() {
        None => String::new(),
        Some((last, [])) => (*last).to_string(),
        Some((last, rest)) => format!("{} and {last}", rest.join(", ")),
    }
}

/// How far this machine's guild is from its remote.
///
/// **The check that earns the command.** Two machines silently diverging is the
/// guild's main failure mode, and nothing else on the machine would tell you.
fn drift(runner: &impl Run, guild: &Guild) -> Vec<Finding> {
    if !guild.exists() {
        return vec![Finding::needs(
            GUILD,
            Problem::Missing,
            "there is no guild on this machine",
            "armada guild init",
        )];
    }
    let remote = match repo::remote(runner, guild.root()) {
        Ok(Some(remote)) => remote,
        // **Sync off is the documented default, not a problem.** `export` still
        // works, and a `doctor` that warned about it every day would train the
        // reader to skim the report.
        Ok(None) => {
            return vec![Finding::settled(
                GUILD,
                Settled::Ok,
                "no remote: sync off, export still works",
            )]
        }
        Err(error) => return vec![offline(&error)],
    };
    let _ = remote;

    let apart = match repo::fetch(runner, guild.root()) {
        Ok(apart) => apart,
        Err(error) => return vec![offline(&error)],
    };
    if apart.diverged() {
        return vec![Finding::needs(
            GUILD,
            Problem::Stale,
            apart.written(),
            "armada guild pull",
        )];
    }
    let mut found = Vec::new();
    if apart.behind > 0 {
        found.push(Finding::needs(
            GUILD,
            Problem::Stale,
            format!(
                "{} behind origin",
                crate::render::format::count(apart.behind, "commit")
            ),
            "armada guild pull",
        ));
    }
    if apart.ahead > 0 {
        found.push(Finding::needs(
            GUILD,
            Problem::Stale,
            format!(
                "{} not pushed",
                crate::render::format::count(apart.ahead, "commit")
            ),
            "armada guild push",
        ));
    }
    if found.is_empty() {
        found.push(Finding::settled(GUILD, Settled::Ok, "in step with origin"));
    }
    found
}

/// The check every guild row belongs to. One spelling, because the render groups
/// on it.
const GUILD: &str = "guild";

/// A drift check that could not run. **A warning, never a failure.**
fn offline(error: &ArmadaError) -> Finding {
    Finding::needs(
        GUILD,
        Problem::Offline,
        format!("could not reach the remote: {}", error.message),
        // Not nothing. "Could not be checked" without a next step reads as a
        // fault the reader has to diagnose, and it is usually a train.
        "reconnect, then armada doctor again",
    )
}

/// Which fragments are still whatever import produced.
///
/// **The half of `--defaults` that has to be reported.** A skipped interview
/// leaves a *working* guild, and the promise `PLAN.md` §13.4 makes is that
/// `doctor` names the fragments that are still a machine's reading of your
/// memory file rather than your own words. Without this, `--defaults` would
/// finish silently in a state that looks configured and is not — the exact
/// failure that section forbids.
fn fragments(guild: &Guild) -> Vec<Finding> {
    if !guild.exists() {
        return Vec::new();
    }
    memory::FRAGMENTS
        .iter()
        .filter_map(|name| {
            let body = std::fs::read_to_string(guild.path(name)).ok()?;
            // **Asked of the file rather than matched here.** `armada-guild`
            // writes these and owns what "still Armada's words" looks like; a
            // second reading of it in this file is how the two eventually
            // disagree about whether somebody has written his own voice.
            Some((name, memory::state(&body)?))
        })
        .map(|(name, state)| {
            Finding::needs(
                GUILD,
                Problem::Partial,
                format!("{name} {}", state.said()),
                // **A sentence rather than a command, and that is still a fix.**
                // There is no verb that writes this for you — `armada guild
                // edit` is not built — so the remedy is the path and what to do
                // with it. An earlier version left this `None` on the grounds
                // that prose is not a command, and the row reached a reader with
                // nothing to act on, which is the failure the rule is about.
                format!(
                    "write {} in your own words",
                    crate::verbs::machine::shown(&guild.path(name))
                ),
            )
        })
        .collect()
}

/// `manifest.db`, and what Guild is allowed to know about it — which is only
/// that it is there.
///
/// **Guild may not name Manifest** (`ARCHITECTURE.md` §1.9), and `doctor` is
/// Helm's, so this is the one place the two are looked at together. It reports
/// the file's presence and nothing about its contents: reading it here would be
/// Helm reimplementing a store that has an owner.
fn store(armada_home: &Path) -> Finding {
    let path = armada_home.join("manifest.db");
    let withheld = machine::read(armada_home).withheld.len();
    if !path.is_file() {
        return Finding::settled(
            "manifest.db",
            Settled::Ok,
            "not created yet: `armada manifest init` makes it",
        );
    }
    Finding::settled(
        "manifest.db",
        Settled::Ok,
        match withheld {
            0 => "present".to_string(),
            n => format!(
                "present, {} withheld from the guild",
                crate::render::format::count(n, "value")
            ),
        },
    )
}

#[cfg(test)]
mod tests {
    use super::*;
    use armada_core::ctx::{RunOutput, RunRequest, SpawnError};
    use armada_core::envelope::Health;
    use std::path::PathBuf;

    struct Answers {
        divergence: &'static str,
        remote: bool,
    }

    impl Run for Answers {
        fn call(&self, request: &RunRequest) -> Result<RunOutput, SpawnError> {
            let argv: Vec<&str> = request.argv.iter().map(String::as_str).collect();
            let (ok, out) = match argv.as_slice() {
                ["git", "--version"] => (true, "git version 2.51.0\n"),
                ["claude", "--version"] => (true, "2.0.14\n"),
                ["docker", "--version"] => (false, ""),
                ["git", "remote", ..] if self.remote => (true, "git@example.com:me/guild.git\n"),
                ["git", "remote", ..] => (false, ""),
                ["git", "rev-list", ..] => (true, self.divergence),
                _ => (true, ""),
            };
            Ok(RunOutput {
                code: Some(if ok { 0 } else { 1 }),
                signal: None,
                stdout: out.to_string(),
                stderr: String::new(),
                timed_out: false,
            })
        }
    }

    /// A `claude` that answers `--help` with `flags` and the argv probe with
    /// `complaint`.
    struct Claude {
        flags: String,
        complaint: &'static str,
    }

    impl Claude {
        /// One that behaves the way the installed binary does.
        fn healthy() -> Claude {
            Claude {
                flags: armada_core::fleet::drone::FLAGS.join(" "),
                complaint: "",
            }
        }
    }

    impl Run for Claude {
        fn call(&self, request: &RunRequest) -> Result<RunOutput, SpawnError> {
            let argv: Vec<&str> = request.argv.iter().map(String::as_str).collect();
            match argv.as_slice() {
                ["claude", "--help"] => Ok(RunOutput {
                    code: Some(0),
                    signal: None,
                    stdout: self.flags.clone(),
                    stderr: String::new(),
                    timed_out: false,
                }),
                _ => Ok(RunOutput {
                    code: Some(1),
                    signal: None,
                    stdout: String::new(),
                    stderr: format!("{}\n", self.complaint),
                    timed_out: false,
                }),
            }
        }
    }

    /// A `claude` that records what it was asked and says nothing back.
    struct Watching(std::cell::RefCell<Vec<Vec<String>>>);

    impl Run for Watching {
        fn call(&self, request: &RunRequest) -> Result<RunOutput, SpawnError> {
            self.0.borrow_mut().push(request.argv.clone());
            Ok(RunOutput {
                code: Some(0),
                signal: None,
                stdout: armada_core::fleet::drone::FLAGS.join(" "),
                stderr: String::new(),
                timed_out: false,
            })
        }
    }

    /// **The probe is what `doctor` runs, and it must be the Drone's own
    /// argv.** A probe that validated an approximation is a probe that passes on
    /// a combination Armada never uses — which is the failure this check exists
    /// to catch, reintroduced one level up.
    #[test]
    fn the_probe_runs_the_drones_own_argv_and_not_an_approximation() {
        let run = Watching(std::cell::RefCell::new(Vec::new()));
        drone_argv(&run, Path::new("/tmp"));

        let probe = run.0.borrow().last().cloned().expect("a probe ran");
        assert_eq!(
            probe,
            armada_core::fleet::drone::probe_argv(),
            "the probe drifted from the Drone's own argv"
        );
    }

    /// **It carries no prompt, which is the whole reason it costs nothing.**
    /// Claude Code is told to read messages from stdin and given none, so it
    /// starts, gets EOF and exits without an API call.
    #[test]
    fn the_argv_probe_has_nothing_to_say_and_so_cannot_spend_a_token() {
        let run = Watching(std::cell::RefCell::new(Vec::new()));
        drone_argv(&run, Path::new("/tmp"));
        let probe = run.0.borrow().last().cloned().unwrap();

        assert_eq!(
            &probe[probe.len() - 2..],
            ["--input-format", "stream-json"],
            "without stream-json input the probe would answer a prompt"
        );
        let real = armada_core::fleet::drone::spawn_argv(
            armada_core::fleet::drone::PROBE_SESSION,
            "a prompt",
        );
        assert!(
            !probe.iter().any(|word| word == "a prompt"),
            "the probe carries a prompt, so it would run a turn"
        );
        assert_eq!(
            probe.len(),
            real.len() + 1,
            "the probe should be the real argv less its prompt, plus two words"
        );
    }

    #[test]
    fn a_claude_that_offers_every_flag_and_refuses_nothing_is_ok() {
        let finding = drone_argv(&Claude::healthy(), Path::new("/tmp"));
        assert_eq!(finding.status, Health::Ok);
        assert!(finding.remedy.is_none());
    }

    /// **The general shape of the failure**: a new version renames or drops a
    /// flag, and every Job spawns into a Drone that dies on a usage error.
    #[test]
    fn a_flag_the_drone_needs_going_missing_is_a_finding() {
        let finding = drone_argv(
            &Claude {
                flags: "--session-id --resume --print --output-format --model".to_string(),
                ..Claude::healthy()
            },
            Path::new("/tmp"),
        );
        assert_eq!(finding.status, Health::Missing);
        assert!(finding.detail.contains("--verbose"), "{}", finding.detail);
        assert!(finding.remedy.is_some(), "a finding without a remedy");
    }

    /// **The specific failure this check was added after.** The flags all exist
    /// and the combination is refused; only asking the validator finds it.
    #[test]
    fn a_combination_the_cli_refuses_is_a_finding_even_when_every_flag_exists() {
        let finding = drone_argv(
            &Claude {
                complaint:
                    "Error: When using --print, --output-format=stream-json requires --verbose",
                ..Claude::healthy()
            },
            Path::new("/tmp"),
        );
        assert_eq!(finding.status, Health::Missing);
        assert!(finding.detail.contains("--verbose"), "{}", finding.detail);
    }

    /// **A probe that ran a turn is a finding, because it must never cost
    /// anything.** No `result` event is emitted when there is no input to
    /// answer; one appearing means a future version began answering an empty
    /// conversation, and a diagnostic that quietly started spending would be
    /// worse than one that does not run.
    #[test]
    fn a_probe_that_started_a_turn_is_reported_rather_than_paid_for() {
        struct Spending;
        impl Run for Spending {
            fn call(&self, request: &RunRequest) -> Result<RunOutput, SpawnError> {
                let helping = request.argv.iter().any(|word| word == "--help");
                Ok(RunOutput {
                    code: Some(0),
                    signal: None,
                    stdout: match helping {
                        true => armada_core::fleet::drone::FLAGS.join(" "),
                        false => r#"{"type":"result","total_cost_usd":0.02}"#.to_string(),
                    },
                    stderr: String::new(),
                    timed_out: false,
                })
            }
        }
        let finding = drone_argv(&Spending, Path::new("/tmp"));
        assert_eq!(finding.status, Health::Partial, "a spend was reported ok");
        assert!(
            finding.detail.contains("cost nothing"),
            "{}",
            finding.detail
        );
        // A warning rather than a failure: the flags are fine, and `doctor` has
        // to stay safe to run in a shell prompt.
        assert!(!finding.status.is_failure());
    }

    /// A `claude` that will not answer at all degrades to a warning: the tooling
    /// check above already reports it, and saying it twice is noise.
    #[test]
    fn a_claude_that_will_not_answer_help_degrades_rather_than_failing() {
        struct Absent;
        impl Run for Absent {
            fn call(&self, _: &RunRequest) -> Result<RunOutput, SpawnError> {
                Err(SpawnError {
                    program: "claude".to_string(),
                    kind: armada_core::ctx::SpawnErrorKind::NotFound,
                    message: "No such file or directory".to_string(),
                })
            }
        }
        let finding = drone_argv(&Absent, Path::new("/tmp"));
        assert_eq!(finding.status, Health::Offline);
        assert!(
            !finding.status.is_failure(),
            "a missing claude failed twice"
        );
    }

    fn machine_with_a_guild() -> (tempfile::TempDir, Where) {
        let home = tempfile::tempdir().unwrap();
        let armada_home = home.path().join(".armada");
        for directory in DIRECTORIES {
            std::fs::create_dir_all(armada_home.join(directory)).unwrap();
        }
        std::fs::create_dir_all(armada_home.join("guild/.git")).unwrap();
        for fragment in memory::FRAGMENTS {
            std::fs::write(armada_home.join("guild").join(fragment), "mine\n").unwrap();
        }
        let place = Where {
            armada_home,
            cwd: home.path().to_path_buf(),
            claude_home: home.path().join(".claude"),
        };
        (home, place)
    }

    fn data(output: &Output) -> DoctorData {
        match output {
            Output::Doctor(envelope) => envelope.data.clone(),
            _ => panic!("not a doctor"),
        }
    }

    fn find<'a>(data: &'a DoctorData, check: &str, detail: &str) -> Option<&'a Finding> {
        data.results
            .iter()
            .find(|row| row.check == check && row.detail.contains(detail))
    }

    /// **The check that earns the command**, with the remedy that fixes it.
    #[test]
    fn a_guild_behind_its_remote_is_stale_and_names_the_command_that_fixes_it() {
        let (_home, place) = machine_with_a_guild();
        let output = run(
            &Answers {
                divergence: "3\t0\n",
                remote: true,
            },
            &place,
        )
        .unwrap();
        let data = data(&output);
        let stale = find(&data, "guild", "3 commits behind origin").expect("no drift row");
        assert_eq!(stale.status, Health::Stale);
        assert_eq!(stale.remedy.as_deref(), Some("armada guild pull"));
    }

    /// **A warning alone does not fail**, so `doctor` stays safe in a shell
    /// prompt — and `missing` is what does fail.
    #[test]
    fn drift_warns_and_a_missing_tool_fails() {
        let (_home, place) = machine_with_a_guild();
        let output = run(
            &Answers {
                divergence: "3\t0\n",
                remote: true,
            },
            &place,
        )
        .unwrap();
        // `docker` is missing in this fixture, so the run does fail — and it is
        // the missing tool that did it, not the stale guild.
        assert_eq!(output.exit_code(), 0, "doctor answers; it does not gate");
        let data = data(&output);
        assert!(
            data.tally.iter().any(|f| f.contains("missing")),
            "{:?}",
            data.tally
        );
        assert!(
            data.tally.iter().any(|f| f.contains("warning")),
            "{:?}",
            data.tally
        );
        assert_eq!(
            DoctorData::verdict(&data.results),
            armada_core::error::Status::Failed
        );
    }

    /// **Sync off is not a problem.** It is the documented default and `export`
    /// still works; warning about it daily would train the reader to skim.
    #[test]
    fn a_guild_with_no_remote_is_ok_rather_than_a_warning() {
        let (_home, place) = machine_with_a_guild();
        let data = data(
            &run(
                &Answers {
                    divergence: "0\t0\n",
                    remote: false,
                },
                &place,
            )
            .unwrap(),
        );
        let row = find(&data, "guild", "no remote").expect("no guild row");
        assert_eq!(row.status, Health::Ok);
    }

    /// **The half of `--defaults` that has to be reported.** Without this, a
    /// skipped interview finishes silently in a state that looks configured and
    /// is not.
    #[test]
    fn a_fragment_still_as_imported_is_reported_by_name() {
        let (_home, place) = machine_with_a_guild();
        std::fs::write(
            place.guild().path("voice.md"),
            format!("<!-- {} imported -->\n", memory::MARKER),
        )
        .unwrap();
        let data = data(
            &run(
                &Answers {
                    divergence: "0\t0\n",
                    remote: false,
                },
                &place,
            )
            .unwrap(),
        );
        let row = find(&data, "guild", "voice.md still as imported").expect("no fragment row");
        assert_eq!(row.status, Health::Partial);
        assert!(
            find(&data, "guild", "expectations.md still as imported").is_none(),
            "a fragment the person wrote was reported as still imported"
        );
    }

    /// A machine that has never run `armada init` is told the one thing that
    /// helps.
    #[test]
    fn a_machine_with_no_armada_home_says_to_run_init() {
        let home = tempfile::tempdir().unwrap();
        let place = Where {
            armada_home: home.path().join(".armada"),
            cwd: home.path().to_path_buf(),
            claude_home: home.path().join(".claude"),
        };
        let data = data(
            &run(
                &Answers {
                    divergence: "0\t0\n",
                    remote: false,
                },
                &place,
            )
            .unwrap(),
        );
        let row = find(&data, "~/.armada", "is not there").expect("no directories row");
        assert_eq!(row.status, Health::Missing);
        assert_eq!(row.remedy.as_deref(), Some("armada init"));
    }

    /// **`layout` and `no jobs, workspaces` said nothing a reader could use.**
    /// The check is named for the thing it checks, and the detail names the
    /// paths and what writes to them.
    #[test]
    fn a_missing_directory_is_named_as_a_path_and_says_what_writes_there() {
        let (_home, place) = machine_with_a_guild();
        std::fs::remove_dir_all(place.armada_home.join("jobs")).unwrap();
        std::fs::remove_dir_all(place.armada_home.join("workspaces")).unwrap();
        let data = data(
            &run(
                &Answers {
                    divergence: "0\t0\n",
                    remote: false,
                },
                &place,
            )
            .unwrap(),
        );
        let row = find(&data, "~/.armada", "missing").expect("no directories row");
        assert_eq!(
            row.detail,
            "jobs/ and workspaces/ are missing; Jobs and worktrees go there"
        );
        assert_eq!(row.remedy.as_deref(), Some("armada init --force"));
    }

    /// **A `→` line for every row that asks the reader to do something**, not
    /// only for the missing ones. `missing ~/.armada` and both `partial guild`
    /// rows reached a real reader with nothing to act on, which is the failure
    /// `Finding::needs` now makes unrepresentable — this holds it against a real
    /// pass as well as against the type.
    #[test]
    fn every_row_that_needs_action_carries_the_fix_for_it() {
        let (_home, place) = machine_with_a_guild();
        std::fs::remove_dir_all(place.armada_home.join("jobs")).unwrap();
        for fragment in memory::FRAGMENTS {
            std::fs::write(
                place.guild().path(fragment),
                format!("<!-- {} imported -->\n", memory::MARKER),
            )
            .unwrap();
        }
        let data = data(
            &run(
                &Answers {
                    divergence: "3\t0\n",
                    remote: true,
                },
                &place,
            )
            .unwrap(),
        );
        let needy = data
            .results
            .iter()
            .filter(|row| row.status.needs_action())
            .count();
        assert!(needy >= 5, "the case did not produce problems: {data:?}");
        for row in &data.results {
            assert_eq!(
                row.status.needs_action(),
                row.remedy.is_some(),
                "`{}` reports {:?} with remedy {:?}",
                row.check,
                row.status,
                row.remedy
            );
        }
    }

    /// The directories row names `~/.armada` and never a real home directory.
    #[test]
    fn no_row_carries_a_real_home_directory() {
        let (_home, place) = machine_with_a_guild();
        let data = data(
            &run(
                &Answers {
                    divergence: "0\t0\n",
                    remote: false,
                },
                &place,
            )
            .unwrap(),
        );
        let row = find(&data, "~/.armada", "all present").expect("no directories row");
        assert_eq!(row.detail, "guild/, jobs/ and workspaces/, all present");
        let _ = PathBuf::new();
    }

    /// `a`, `a and b`, `a, b and c` — the shape every detail here is built from.
    #[test]
    fn a_list_is_written_the_way_a_sentence_writes_one() {
        assert_eq!(written(&["a"]), "a");
        assert_eq!(written(&["a", "b"]), "a and b");
        assert_eq!(written(&["a", "b", "c"]), "a, b and c");
        let none: [&str; 0] = [];
        assert_eq!(written(&none), "");
    }
}
