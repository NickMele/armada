//! `armada daemon enable`/`disable`/`status`/`run` — `034`'s daemon, as a
//! machine switch around a process (PLAN.md §1).
//!
//! **`enable`/`disable` write `fleet.daemon.enter` in `~/.armada/machine.yml`,
//! on [`crate::verbs::helm::enable`]'s own read-compare-write shape** — but
//! unlike Helm's switch, flipping this one also has to make something true in
//! the world: Helm's `enter` gates a session that only ever starts because a
//! person typed `armada helm`, and this one gates a process nobody has to
//! type anything to keep running. So `enable` on macOS also installs and
//! loads the launchd job ([`armada_fleet::daemon::launchd`]), and `disable`
//! unloads and removes it.
//!
//! # `enable` does not also call `armada_fleet::daemon::start`
//!
//! **launchd's `RunAtLoad` is what starts the process, and calling
//! [`armada_fleet::daemon::start`] as well would hand the same job to two
//! supervisors.** [`armada_fleet::daemon::start`]/`stop` remain real and
//! tested — they are what a machine with no launchd, or a caller bypassing it
//! on purpose, uses to run `armada daemon run` detached — but this stage's
//! `enable` reaches the process only through launchd, on the reasoning
//! [`armada_fleet::daemon`]'s own module header gives in full.

use armada_core::envelope::{DaemonStatusData, DaemonSwitchData, Envelope};
use armada_core::error::{ArmadaError, ErrClass, Status};
use std::path::Path;

use crate::verbs::Output;

/// `armada daemon enable` — let `034`'s daemon run unattended on this
/// machine.
///
/// **macOS only, in this stage.** PLAN.md §1 asks for a launchd job on macOS
/// and nothing else; refused by name everywhere else, before anything is
/// written — the same "never a silent no-op" rule `034` states for exactly
/// this switch.
#[cfg(target_os = "macos")]
pub fn enable(armada_home: &Path, exe: &Path) -> Result<Output, ArmadaError> {
    let before = armada_fleet::machine::read(armada_home).daemon.enter;
    write_switch(armada_home, true)?;
    armada_fleet::daemon::launchd::install(armada_home, exe)?;
    Ok(switch_output("daemon enable", true, !before))
}

/// The refusal every other OS gets, in this stage.
#[cfg(not(target_os = "macos"))]
pub fn enable(armada_home: &Path, exe: &Path) -> Result<Output, ArmadaError> {
    let _ = (armada_home, exe);
    Err(armada_fleet::daemon::enable_unsupported())
}

/// `armada daemon disable` — put the switch back where a fresh install
/// leaves it.
///
/// **Always safe to run, on every OS, whatever `enable` did or did not do.**
/// A machine that never enabled has nothing to unload and nothing running to
/// stop; [`armada_fleet::daemon::launchd::uninstall`] is already tolerant of
/// "not loaded" and [`armada_fleet::daemon::stop`] of "no pidfile", so
/// neither turns the ordinary case into a refusal — the same idempotence
/// `armada helm disable` gives its own switch.
pub fn disable(armada_home: &Path) -> Result<Output, ArmadaError> {
    let before = armada_fleet::machine::read(armada_home).daemon.enter;
    write_switch(armada_home, false)?;
    uninstall_and_stop(armada_home)?;
    Ok(switch_output("daemon disable", false, before))
}

#[cfg(target_os = "macos")]
fn uninstall_and_stop(armada_home: &Path) -> Result<(), ArmadaError> {
    // **Unload before cleaning up the pidfile.** `launchctl unload` is what
    // stops `KeepAlive` from resurrecting the process the moment `stop`'s own
    // signal reaches it; reversing the order would have `stop` kill it and
    // launchd bring it straight back.
    armada_fleet::daemon::launchd::uninstall(armada_home)?;
    armada_fleet::daemon::stop(armada_home)
}

#[cfg(not(target_os = "macos"))]
fn uninstall_and_stop(armada_home: &Path) -> Result<(), ArmadaError> {
    armada_fleet::daemon::stop(armada_home)
}

/// `armada daemon status` — the switch and the live process, **as two
/// separate facts** (`envelope::DaemonStatusData`'s own reasoning).
pub fn status(armada_home: &Path) -> Result<Output, ArmadaError> {
    let enter = armada_fleet::machine::read(armada_home).daemon.enter;
    let pid = armada_fleet::daemon::is_running(armada_home);
    Ok(Output::DaemonStatus(Box::new(Envelope::ok(
        "daemon status",
        None,
        Status::Ok,
        DaemonStatusData { enter, pid },
    ))))
}

/// `armada daemon run` — the hidden entrypoint launchd execs directly, and
/// what [`armada_fleet::daemon::start`]'s own detached spawn execs when
/// launchd is not the one starting it.
///
/// **No watch loop yet — `034` §5 item 6 builds it.** This stage's whole job
/// is to exist as something [`armada_fleet::daemon::is_running`] can observe:
/// it records its own arrival the instant it is up
/// ([`armada_fleet::daemon::record_started`], with its own pid — the
/// counterpart to what [`armada_fleet::daemon::start`] records for the pid
/// *it* spawns, so the pidfile and `daemon.jsonl` read the same whichever
/// path got the process running) and then sleeps, forever, until something
/// kills it.
///
/// **Never returns**, except to propagate a failure to record its own start —
/// a daemon that cannot write its own pidfile has nothing `armada daemon
/// status` could ever observe, so failing loudly here beats looping anyway
/// and reporting "running" for a process nothing can ever find again.
pub fn run(armada_home: &Path) -> Result<(), ArmadaError> {
    armada_fleet::daemon::record_started(armada_home, std::process::id())?;
    loop {
        std::thread::sleep(std::time::Duration::from_secs(3600));
    }
}

/// The one write path [`enable`] and [`disable`] share: read, compare, write.
fn write_switch(armada_home: &Path, enter: bool) -> Result<(), ArmadaError> {
    let mut section = armada_fleet::machine::read(armada_home);
    section.daemon.enter = enter;
    armada_fleet::machine::write(armada_home, &section).map_err(|error| ArmadaError {
        class: ErrClass::Environment,
        r#where: armada_home.join("machine.yml").display().to_string(),
        message: format!(
            "cannot write {}: {error}",
            armada_home.join("machine.yml").display()
        ),
        next_action: Some("check the permissions on ~/.armada/".to_string()),
    })
}

fn switch_output(verb: &str, enter: bool, changed: bool) -> Output {
    Output::DaemonSwitch(Box::new(Envelope::ok(
        verb,
        None,
        Status::Ok,
        DaemonSwitchData { enter, changed },
    )))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn disabling_a_machine_that_never_enabled_is_not_an_error() {
        let home = tempfile::tempdir().unwrap();
        let output = disable(home.path()).unwrap();
        let Output::DaemonSwitch(envelope) = output else {
            panic!("expected DaemonSwitch");
        };
        assert!(!envelope.data.enter);
        assert!(!envelope.data.changed);
    }

    #[test]
    fn status_on_a_fresh_machine_is_off_and_not_running() {
        let home = tempfile::tempdir().unwrap();
        let output = status(home.path()).unwrap();
        let Output::DaemonStatus(envelope) = output else {
            panic!("expected DaemonStatus");
        };
        assert!(!envelope.data.enter);
        assert_eq!(envelope.data.pid, None);
    }

    /// `run`'s own startup half — the loop is not exercised here, only the
    /// self-registration `enable` on a launchd-managed process depends on.
    #[test]
    fn run_records_its_own_pid_before_it_would_start_looping() {
        let home = tempfile::tempdir().unwrap();
        armada_fleet::daemon::record_started(home.path(), std::process::id()).unwrap();
        assert_eq!(
            armada_fleet::daemon::is_running(home.path()),
            Some(std::process::id())
        );
    }

    #[cfg(not(target_os = "macos"))]
    #[test]
    fn enabling_off_macos_is_refused_and_writes_nothing() {
        let home = tempfile::tempdir().unwrap();
        let error = enable(home.path(), Path::new("/usr/local/bin/armada")).unwrap_err();
        assert_eq!(error.class, ErrClass::Environment);
        assert!(!home.path().join("machine.yml").exists());
    }
}
