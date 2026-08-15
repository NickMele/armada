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

use armada_core::ctx::Run;
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
        .filter(|name| {
            std::fs::read_to_string(guild.path(name))
                .map(|body| body.contains("Imported from CLAUDE.md"))
                .unwrap_or(false)
        })
        .map(|name| {
            Finding::needs(
                GUILD,
                Problem::Partial,
                format!("{name} still as imported"),
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
            "<!-- Imported from CLAUDE.md by `armada guild init`. -->\n",
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
                "<!-- Imported from CLAUDE.md by `armada guild init`. -->\n",
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
