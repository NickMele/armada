//! `~/.armada/machine.yml` — machine capacity, never committed (PLAN.md §4.3.1)
//! — plus the two machine facts that are not configuration: the boot id and a
//! random namespace.
//!
//! **YAML, and the same parser `armada.yml` uses.** The file was TOML while it
//! was called `config.toml`; carrying a second document language for six
//! integers meant a second parser in the dependency graph and a second set of
//! quoting rules for anyone editing either file. One language, one parser, one
//! set of surprises (PLAN.md §4.1.1, decision 5).
//!
//! **`armada.yml` declares how expensive a check is; this file declares how much
//! the machine has.** They cannot be the same file: `armada.yml` is committed,
//! and a repo cannot know your core count.
//!
//! **Who reads it, and where — one of the five things phase 2 had to settle.**
//! Phase 1 shipped a `Defaults` struct that is *passed in and never read*,
//! precisely so nothing below the entrypoint reaches for ambient state
//! (`ARCHITECTURE.md` §1.4). That property is kept: this module is the only
//! reader, the entrypoint is the only caller, and everything below receives
//! values. `$HOME` is likewise captured once, at the top, and arrives here as
//! an argument — which is also what lets the whole test suite point Armada at a
//! `TempDir` without an environment variable.
//!
//! **Absence is not an error.** A machine with no `~/.armada/machine.yml` is the
//! ordinary case — Armada never writes one — so a missing file is the documented
//! defaults and nothing else. A file that exists and cannot be understood *is*
//! an error, and it is `environment`: the repo is fine and the machine's
//! configuration is not.

use armada_core::config::Defaults;
use armada_core::ctx::Run;
// The two callers below are both the non-Linux branch: Linux answers both
// questions from `/proc` and spawns nothing.
#[cfg(not(target_os = "linux"))]
use armada_core::ctx::RunRequest;
use armada_core::error::{ArmadaError, ErrClass};
use serde::Deserialize;
use std::path::{Path, PathBuf};
use std::time::Duration;

/// The seven documented keys, and no others.
///
/// Adding an eighth is a contract change: PLAN.md §4.3.1 is the owner of this
/// list, and this crate codes against it rather than extending it.
///
/// **`up_timeout` is the seventh, and it arrived as a reconciliation rather
/// than an addition.** PLAN.md §6 names it and gives it a default — 600s on
/// `compose up`, `build`, `pull`, `down` and `rm` — while §4.3.1 listed six
/// keys and did not carry it, so the setting was specified with a value and no
/// home. §4.3.1 is where machine capacity lives, so that is where it went.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct MachineConfig {
    /// The machine's CPU budget, in slots.
    pub cpu_slots: u32,
    /// How many ports one workspace's block holds.
    pub port_block_size: u16,
    /// How many run directories `.armada/run/` keeps.
    pub run_retention: u32,
    /// Seconds a check gets when it declares no `timeout:`.
    pub check_timeout: u32,
    /// Cumulative seconds a check may spend *waiting* for leases.
    pub acquire_timeout: u32,
    /// Armada's own deadline on every docker call.
    pub docker_timeout: u32,
    /// Armada's deadline on the docker calls that do **work** rather than ask a
    /// question: `compose up`, `build`, `pull`, `down`, `rm`.
    pub up_timeout: u32,
}

/// The file, as written. Every key optional: a `machine.yml` that sets one
/// thing takes the documented default for the other five.
#[derive(Debug, Default, Deserialize)]
#[serde(deny_unknown_fields)]
struct MachineConfigFile {
    cpu_slots: Option<u32>,
    port_block_size: Option<u16>,
    run_retention: Option<u32>,
    check_timeout: Option<u32>,
    acquire_timeout: Option<u32>,
    docker_timeout: Option<u32>,
    up_timeout: Option<u32>,
}

impl MachineConfig {
    /// The documented defaults, for a machine with no file.
    ///
    /// `cpu_slots` is `num_cpus - 2`, **not** `num_cpus`: a budget that permits
    /// full saturation makes the machine feel dead even while the work is
    /// correctly bounded, because the editor, the agent processes and Armada
    /// itself all need something.
    pub fn defaults() -> Self {
        MachineConfig {
            cpu_slots: default_cpu_slots(),
            port_block_size: 10,
            run_retention: 10,
            check_timeout: 900,
            // Sized against the longest legitimate exclusive hold in the
            // fixture set — `python-ml`'s 1800-second GPU check — and the rule
            // is that it must **exceed** it. The figure has been wrong twice,
            // in the same way, and the fixtures caught it both times.
            acquire_timeout: 2400,
            docker_timeout: 30,
            // **Ten times `docker_timeout`, and the gap is the point.**
            // Measured, a stock `docker compose up -d` with
            // `depends_on: {condition: service_healthy}` took 43 seconds and a
            // `compose exec` running a check took 45 — so a blanket 30 kills
            // both and reports `environment`, exit 6: "fix the machine" when
            // nothing is wrong with it. A question that cannot be answered in
            // thirty seconds means a wedged daemon; a slow build is a slow
            // build (PLAN.md §6).
            up_timeout: 600,
        }
    }

    /// Read `<armada_home>/machine.yml`, or take the defaults if it is not there.
    pub fn read(armada_home: &Path) -> Result<Self, ArmadaError> {
        let path = armada_home.join("machine.yml");
        let text = match std::fs::read_to_string(&path) {
            Ok(text) => text,
            Err(e) if e.kind() == std::io::ErrorKind::NotFound => return Ok(Self::defaults()),
            Err(e) => {
                return Err(ArmadaError {
                    class: ErrClass::Environment,
                    r#where: path.display().to_string(),
                    message: format!("cannot read {}: {e}", path.display()),
                    next_action: None,
                })
            }
        };

        // `deny_unknown_fields` is deliberate and it is a trade. Armada never
        // writes this file, so an unrecognised key is a typo far more often
        // than it is version skew — and a typo'd `cpu_slot` accepted in
        // silence is a machine budget that does not apply, which is the
        // "unstated default is a per-implementer decision" failure with the
        // user in the implementer's seat.
        let file: MachineConfigFile = serde_yaml_ng::from_str(&text).map_err(|e| ArmadaError {
            class: ErrClass::Environment,
            r#where: path.display().to_string(),
            message: format!("cannot parse {}: {e}", path.display()),
            next_action: Some(format!(
                "fix {} — the keys are cpu_slots, port_block_size, run_retention, \
                 check_timeout, acquire_timeout, docker_timeout and up_timeout",
                path.display()
            )),
        })?;

        let defaults = Self::defaults();
        Ok(MachineConfig {
            cpu_slots: file.cpu_slots.unwrap_or(defaults.cpu_slots),
            port_block_size: file.port_block_size.unwrap_or(defaults.port_block_size),
            run_retention: file.run_retention.unwrap_or(defaults.run_retention),
            check_timeout: file.check_timeout.unwrap_or(defaults.check_timeout),
            acquire_timeout: file.acquire_timeout.unwrap_or(defaults.acquire_timeout),
            docker_timeout: file.docker_timeout.unwrap_or(defaults.docker_timeout),
            up_timeout: file.up_timeout.unwrap_or(defaults.up_timeout),
        })
    }

    /// The config-resolution defaults this machine implies.
    ///
    /// This is the join between the two files: `check_timeout` is machine
    /// capacity, and resolution materialises it into every check that declares
    /// no `timeout:` of its own.
    pub fn config_defaults(&self) -> Defaults {
        Defaults {
            check_timeout: self.check_timeout,
            ..Defaults::built_in()
        }
    }

    /// Armada's deadline on a docker call, as a duration.
    ///
    /// Measured: the docker CLI has **no client-side timeout** and no flag for
    /// one — `docker ps` against a socket that accepts and never replies was
    /// still running at 30 seconds. The one that matters most is the
    /// `docker ps` in `init`'s reap pass: without a deadline a hung daemon
    /// wedges every new workspace on the machine, including the verb whose job
    /// is recovery.
    pub fn docker_deadline(&self) -> Duration {
        Duration::from_secs(self.docker_timeout as u64)
    }

    /// Armada's deadline on a docker call that does work rather than asks a
    /// question.
    ///
    /// **The distinction is not cosmetic** (PLAN.md §6). Only the question
    /// calls are `environment` on expiry — a question that cannot be answered
    /// in thirty seconds means the daemon is wedged. A `compose up` that runs
    /// long is a slow build, so it expires as `timeout`.
    pub fn up_deadline(&self) -> Duration {
        Duration::from_secs(self.up_timeout as u64)
    }

    /// The acquisition ceiling in milliseconds.
    pub fn acquire_ceiling_ms(&self) -> u64 {
        self.acquire_timeout as u64 * 1_000
    }
}

fn default_cpu_slots() -> u32 {
    let cores = std::thread::available_parallelism()
        .map(|n| n.get() as u32)
        .unwrap_or(1);
    cores.saturating_sub(2).max(1)
}

/// `~/.armada/`, given the `$HOME` the entrypoint captured.
pub fn armada_home(home: &Path) -> PathBuf {
    home.join(".armada")
}

/// This boot's identity, so a recorded pgid or heartbeat from a previous boot
/// is stale **by definition** rather than by guesswork.
///
/// "Boot id" is not a portable concept, so this is two sources: `sysctl
/// kern.bootsessionuuid` on darwin — verified present and stable, and there is
/// no `/proc/sys/kernel/random/boot_id` there — and that file on Linux.
///
/// A machine that answers neither returns `None`, and every liveness check that
/// depends on it then declines to act: [`armada_core::reap::pgid_is_ours`]
/// returns false without a boot id, so Armada reports rather than kills.
pub fn boot_id(run: &impl Run, cwd: &Path) -> Option<String> {
    #[cfg(target_os = "linux")]
    {
        let _ = (run, cwd);
        std::fs::read_to_string("/proc/sys/kernel/random/boot_id")
            .ok()
            .map(|s| s.trim().to_string())
            .filter(|s| !s.is_empty())
    }

    #[cfg(not(target_os = "linux"))]
    {
        let output = run
            .call(
                &RunRequest::new(
                    vec![
                        "sysctl".to_string(),
                        "-n".to_string(),
                        "kern.bootsessionuuid".to_string(),
                    ],
                    cwd.to_path_buf(),
                )
                .timeout(Duration::from_secs(5)),
            )
            .ok()?;
        let id = output.stdout.trim().to_string();
        (output.ok() && !id.is_empty()).then_some(id)
    }
}

/// A process start time, for the liveness cross-check that makes a recorded
/// pgid safe to kill.
///
/// One-second resolution on darwin, so pid reuse inside the same second is
/// undetectable and this is **a strong filter rather than a proof** — which is
/// why it is one half of a pair with the boot id rather than a test on its own.
/// `None` means the process is gone, or Armada could not sample it; both are
/// answered by declining to kill.
pub fn process_start_at(run: &impl Run, cwd: &Path, pid: i32) -> Option<String> {
    #[cfg(target_os = "linux")]
    {
        let _ = (run, cwd);
        // Field 22 of /proc/<pid>/stat is `starttime`, in clock ticks since
        // boot. The comm field can contain spaces and parentheses, so the
        // split has to start after the last `)`.
        let stat = std::fs::read_to_string(format!("/proc/{pid}/stat")).ok()?;
        let after_comm = stat.rsplit_once(')')?.1;
        after_comm.split_whitespace().nth(19).map(str::to_string)
    }

    #[cfg(not(target_os = "linux"))]
    {
        let output = run
            .call(
                &RunRequest::new(
                    vec![
                        "ps".to_string(),
                        "-o".to_string(),
                        "lstart=".to_string(),
                        "-p".to_string(),
                        pid.to_string(),
                    ],
                    cwd.to_path_buf(),
                )
                .timeout(Duration::from_secs(5)),
            )
            .ok()?;
        let started = output.stdout.trim().to_string();
        (output.ok() && !started.is_empty()).then_some(started)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_machine_with_no_file_gets_the_documented_defaults() {
        let home = tempfile::tempdir().unwrap();
        let config = MachineConfig::read(home.path()).unwrap();
        assert_eq!(config, MachineConfig::defaults());
        assert_eq!(config.port_block_size, 10);
        assert_eq!(config.check_timeout, 900);
        assert_eq!(config.docker_timeout, 30);
    }

    /// The ceiling must **exceed** the longest `timeout:` of any check
    /// declaring an `exclusive:`. `python-ml`'s GPU check holds one for 1800s,
    /// and a ceiling of 1200 fires on a healthy training run with the
    /// *retryable* class — telling a merge gate to try again on a machine
    /// behaving exactly as its own fixture specifies.
    #[test]
    fn the_acquisition_ceiling_exceeds_the_longest_exclusive_hold_in_the_fixtures() {
        assert!(MachineConfig::defaults().acquire_timeout > 1800);
    }

    #[test]
    fn cpu_slots_leave_the_machine_something_to_run_the_editor_with() {
        let slots = default_cpu_slots();
        assert!(slots >= 1);
        let cores = std::thread::available_parallelism().unwrap().get() as u32;
        assert!(slots <= cores.saturating_sub(2).max(1));
    }

    #[test]
    fn a_partial_file_takes_the_defaults_for_everything_it_does_not_set() {
        let home = tempfile::tempdir().unwrap();
        std::fs::write(home.path().join("machine.yml"), "cpu_slots: 3\n").unwrap();
        let config = MachineConfig::read(home.path()).unwrap();
        assert_eq!(config.cpu_slots, 3);
        assert_eq!(config.port_block_size, 10);
    }

    #[test]
    fn every_documented_key_is_readable() {
        let home = tempfile::tempdir().unwrap();
        std::fs::write(
            home.path().join("machine.yml"),
            "cpu_slots: 6\nport_block_size: 20\nrun_retention: 4\n\
             check_timeout: 60\nacquire_timeout: 3000\ndocker_timeout: 15\nup_timeout: 900\n",
        )
        .unwrap();
        let config = MachineConfig::read(home.path()).unwrap();
        assert_eq!(
            config,
            MachineConfig {
                cpu_slots: 6,
                port_block_size: 20,
                run_retention: 4,
                check_timeout: 60,
                acquire_timeout: 3000,
                docker_timeout: 15,
                up_timeout: 900,
            }
        );
        assert_eq!(config.config_defaults().check_timeout, 60);
    }

    /// **The two docker deadlines are separate, and the gap is the point.**
    /// Measured, a stock `docker compose up -d` with
    /// `depends_on: {condition: service_healthy}` took 43 seconds — so a
    /// blanket 30 kills it and reports `environment`, exit 6: "fix the machine"
    /// when nothing is wrong with it (PLAN.md §6).
    #[test]
    fn work_gets_a_longer_deadline_than_a_question() {
        let defaults = MachineConfig::defaults();
        assert_eq!(defaults.docker_deadline(), Duration::from_secs(30));
        assert_eq!(defaults.up_deadline(), Duration::from_secs(600));
        assert!(defaults.up_deadline() > defaults.docker_deadline());
    }

    #[test]
    fn an_unreadable_file_is_an_environment_failure_naming_the_keys() {
        let home = tempfile::tempdir().unwrap();
        std::fs::write(home.path().join("machine.yml"), "cpu_slots: [3\n").unwrap();
        let err = MachineConfig::read(home.path()).unwrap_err();
        assert_eq!(err.class, ErrClass::Environment);
        assert!(err.next_action.unwrap().contains("cpu_slots"));
    }

    #[test]
    fn a_mistyped_key_is_reported_rather_than_ignored() {
        let home = tempfile::tempdir().unwrap();
        std::fs::write(home.path().join("machine.yml"), "cpu_slot: 3\n").unwrap();
        let err = MachineConfig::read(home.path()).unwrap_err();
        assert_eq!(err.class, ErrClass::Environment);
    }

    #[test]
    fn armada_home_hangs_off_the_home_the_entrypoint_captured() {
        assert_eq!(
            armada_home(Path::new("/home/agent")),
            PathBuf::from("/home/agent/.armada")
        );
    }

    /// Measured on the machine this runs on rather than asserted from
    /// documentation: whichever source applies, it must answer, and it must
    /// answer the same thing twice.
    #[test]
    fn the_boot_id_is_available_and_stable() {
        let run = crate::process::RealRun;
        let cwd = std::path::Path::new("/");
        let first = boot_id(&run, cwd).expect("this platform must have a boot id");
        assert!(!first.is_empty());
        assert_eq!(boot_id(&run, cwd).as_deref(), Some(first.as_str()));
    }

    #[test]
    fn a_live_process_has_a_start_time_and_a_dead_one_does_not() {
        let run = crate::process::RealRun;
        let cwd = std::path::Path::new("/");
        assert!(process_start_at(&run, cwd, std::process::id() as i32).is_some());
        // pid 0 is never a process this call can sample.
        assert!(process_start_at(&run, cwd, 0).is_none());
    }
}
