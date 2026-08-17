//! The Job daemon **as a process** — the pidfile that says whether it is
//! running, and the two ways of getting it running (`034` §1, PLAN.md §1).
//!
//! **A bare pid, not a [`armada_core::fleet::job::Handle`].** A Drone's
//! liveness check has to survive pid recycling across *any* number of Drones
//! this machine has ever started, which is why [`crate::drone::alive`] proves
//! a handle against a recorded boot id and process start time
//! (`armada_core::reap::pgid_is_ours`). The daemon is one process this machine
//! starts and stops through its own two verbs; the window in which a recycled
//! pid could land on `~/.armada/daemon.pid` and be mistaken for a live daemon
//! is the moment between this process dying and `armada daemon` next reading
//! the file, which nothing but the OS's own pid-reuse policy bounds — an
//! acceptable, and cheap, deviation from the Drone mechanism precisely because
//! there is only ever one daemon, never a fleet of them.
//!
//! # Two ways this process starts, and only one of them owns the pidfile
//!
//! **`armada daemon enable` on macOS does not call [`start`].** It installs
//! and loads the launchd job ([`launchd`]), and `RunAtLoad`/`KeepAlive` are
//! what actually spawn and re-spawn `armada daemon run` from then on — calling
//! [`start`] as well would hand the same job to two supervisors, and the
//! second daemon a `launchctl load` produces has no pidfile of its own to
//! collide with the first's, so nothing would ever notice two were running.
//! Since launchd execs `armada daemon run` **directly** — never through this
//! module's [`start`] — the entrypoint records its own arrival, by calling
//! [`record_started`] with [`std::process::id`] the moment it is up. [`start`]
//! calls the same function with the pid it just spawned, so the pidfile and
//! `daemon.jsonl` read identically whichever path got the process running.
//!
//! [`start`]/[`stop`] remain real, tested, and reachable — spawning
//! `armada daemon run` **detached**, the same `setsid`-via-`ProcessGroup`
//! shape [`crate::drone::start`] already gives a Drone — for the machine that
//! has no launchd at all, and for anyone exercising the process directly
//! without going through `armada daemon enable`.

use armada_core::ctx::{Clock, RunRequest, SpawnErrorKind, StdioMode};
use armada_core::error::{ArmadaError, ErrClass};
use armada_manifest::clock::SystemClock;
use armada_manifest::posix;
use armada_manifest::process::{ProcessGroup, GRACE};
use std::path::{Path, PathBuf};

use crate::daemon_log::{self, Entry};

/// Whether the daemon is alive **right now**, and its pid if so.
///
/// **A stale pidfile reads as `None`, never as an error** — the fail-safe
/// direction [`armada_helm::machine::HelmSection::read`] documents for its own
/// switch, applied here to a process instead of a boolean: a file left behind
/// by a daemon that crashed must not make `armada daemon status` lie about a
/// process that is not there.
///
/// **`group_alive`, not `pgid_is_ours`.** [`armada_manifest::posix::group_alive`]
/// is the same primitive [`armada_manifest::posix::stop_group`] already uses
/// to decide whether a group needs signalling at all — reused rather than a
/// second liveness check invented for one more caller, per this module's own
/// header on why the daemon does not need the boot-id-and-start-time proof a
/// Drone's handle carries.
pub fn is_running(armada_home: &Path) -> Option<u32> {
    let text = std::fs::read_to_string(crate::home::daemon_pidfile(armada_home)).ok()?;
    let pid: u32 = text.trim().parse().ok()?;
    if pid == 0 {
        // Not a real pid; `stop_owned_processes` treats zero the same way —
        // nothing can be alive behind it, so there is nothing to probe.
        return None;
    }
    posix::group_alive(pid as i32).then_some(pid)
}

/// Start the daemon **directly**, without launchd — detached, the way
/// [`crate::drone::start`] starts a Drone.
///
/// **Idempotent.** A second `start` while one is already running returns the
/// pid already on file rather than spawning a second process this module
/// would then have no way to tell apart from the first — the same reason
/// `armada helm enable` twice in a row reports `changed: false` instead of
/// doing the write again.
///
/// `exe` is the real binary path — [`std::env::current_exe`], resolved by the
/// caller once at the entrypoint (`ARCHITECTURE.md` §1.4) — because a bare
/// `armada` on `PATH` is not guaranteed to be *this* build, and a daemon that
/// re-execs a different Armada than the one that enabled it is exactly the
/// kind of drift a background process must not have.
pub fn start(armada_home: &Path, exe: &Path) -> Result<u32, ArmadaError> {
    if let Some(pid) = is_running(armada_home) {
        return Ok(pid);
    }

    let argv = vec![
        exe.display().to_string(),
        "daemon".to_string(),
        "run".to_string(),
    ];
    let request = RunRequest::new(argv, armada_home.to_path_buf())
        // **A real file, not a pipe** — [`crate::drone::start`]'s own reason:
        // the daemon outlives this call, and a captured pipe nobody reads
        // becomes `EPIPE` on its first write, moments after `spawn` reported
        // it started.
        .stdio(StdioMode::Log(armada_home.join("daemon.out.log")));

    let group = ProcessGroup::spawn(&request).map_err(|e| match e.kind {
        SpawnErrorKind::NotFound => not_runnable(exe),
        _ => ArmadaError {
            class: ErrClass::Environment,
            r#where: "daemon".to_string(),
            message: format!("the daemon would not start: {}", e.message),
            next_action: Some(
                "check the armada binary is executable, then retry unchanged".to_string(),
            ),
        },
    })?;
    let pid = group.pgid();
    // Dropped without waiting, on [`crate::drone::start`]'s own reasoning: the
    // daemon is meant to outlive this process, and `setsid` moved it to a new
    // session, not to a new parent, so this process stays responsible for
    // reaping it if it dies before the next invocation reads the pidfile.
    drop(group);

    record_started(armada_home, pid as u32)?;
    Ok(pid as u32)
}

/// Stop the daemon: SIGTERM, a grace period, then SIGKILL — the same
/// escalation [`armada_manifest::posix::stop_group`] gives a Drone's group,
/// applied to the daemon's own.
///
/// **A missing pidfile is nothing to stop, not an error** — [`crate::drone::Stopped::NothingToStop`]'s
/// own case, restated for a daemon that was never running or was already
/// cleaned up. A pidfile that names a pid already dead (the ordinary shape
/// after `launchctl unload` has killed the launchd-managed process) is still
/// cleaned up and still logged: the file existing at all is the daemon
/// telling this call it once considered itself started, and that fact belongs
/// in `daemon.jsonl` whether or not there was anything left to signal.
pub fn stop(armada_home: &Path) -> Result<(), ArmadaError> {
    let path = crate::home::daemon_pidfile(armada_home);
    let Ok(text) = std::fs::read_to_string(&path) else {
        return Ok(());
    };
    if let Ok(pid) = text.trim().parse::<i32>() {
        if pid > 0 && posix::group_alive(pid) {
            posix::stop_group(pid, GRACE);
        }
    }
    let _ = std::fs::remove_file(&path);
    record_stopped(armada_home, "armada daemon disable")
}

/// Record that the daemon has started — **the pidfile, then the log line, in
/// that order**, so a reader of `daemon.jsonl` never finds a `Started` entry
/// for a pid [`is_running`] cannot also see.
///
/// **The one function both starting paths call.** [`start`] calls it with the
/// pid it just spawned; `armada daemon run` (reached directly when launchd
/// execs it, per this module's header) calls it with its own,
/// [`std::process::id`]. One place writes the pidfile and appends the log
/// line, so the two paths cannot drift into recording the fact two different
/// ways.
pub fn record_started(armada_home: &Path, pid: u32) -> Result<(), ArmadaError> {
    let path = crate::home::daemon_pidfile(armada_home);
    if let Some(parent) = path.parent() {
        std::fs::create_dir_all(parent).map_err(|e| unwritable(&path, &e))?;
    }
    std::fs::write(&path, pid.to_string()).map_err(|e| unwritable(&path, &e))?;
    daemon_log::append(
        armada_home,
        &Entry::Started {
            at: SystemClock.wall_rfc3339(),
            at_ms: SystemClock.wall_ms(),
            pid,
        },
    )
}

/// Append a `Stopped` line — **the trail is written from the first action**
/// (`034` §6.5), and stopping is the second one the daemon ever takes.
fn record_stopped(armada_home: &Path, reason: &str) -> Result<(), ArmadaError> {
    daemon_log::append(
        armada_home,
        &Entry::Stopped {
            at: SystemClock.wall_rfc3339(),
            at_ms: SystemClock.wall_ms(),
            reason: reason.to_string(),
        },
    )
}

fn not_runnable(exe: &Path) -> ArmadaError {
    ArmadaError {
        class: ErrClass::Environment,
        r#where: "daemon".to_string(),
        message: format!("`{}` is not runnable: not found", exe.display()),
        next_action: Some("reinstall armada, then retry unchanged".to_string()),
    }
}

fn unwritable(path: &Path, error: &std::io::Error) -> ArmadaError {
    ArmadaError {
        class: ErrClass::Environment,
        r#where: path.display().to_string(),
        message: format!("cannot write {}: {error}", path.display()),
        next_action: Some("check the permissions on ~/.armada/".to_string()),
    }
}

/// Why `armada daemon enable` is refused on anything but macOS.
///
/// **Named and refused, never a silent no-op.** `034` and PLAN.md §1 ask for
/// launchd only in this stage; a switch that claimed success while wiring
/// nothing up would be the exact silent-stall shape `034` exists to end —
/// a Job waiting on a daemon that was never actually going to run, with
/// nothing on the screen saying so.
#[cfg(not(target_os = "macos"))]
pub fn enable_unsupported() -> ArmadaError {
    ArmadaError {
        class: ErrClass::Environment,
        r#where: "daemon".to_string(),
        message: "the daemon's launchd job is macOS-only, and this is not a macOS machine"
            .to_string(),
        next_action: Some(
            "run `armada daemon enable` on a macOS machine instead — every other verb this \
             stage ships still works here"
                .to_string(),
        ),
    }
}

/// The launchd job that keeps `armada daemon run` alive across logins and
/// crashes — macOS only, `034` §1.
///
/// **`launchctl` is shelled to directly rather than through [`armada_core::ctx::Run`].**
/// Every other impure call in this crate goes through that seam so a test can
/// assert the exact argv and feed back a recorded answer
/// ([`crate::lib`]'s own header) — but `load`/`unload` are one-shot
/// administrative requests to launchd itself, not a workload with output
/// Armada reads, and a fake `Run` standing in for a real login session's
/// launchd would assert nothing a person could not already read off this
/// module's two pure functions, [`plist_path`] and [`plist`], which the test
/// suite exercises directly instead.
#[cfg(target_os = "macos")]
pub mod launchd {
    use super::{ArmadaError, ErrClass, Path, PathBuf};

    /// The job's name, in both the plist and every `launchctl` argument that
    /// names it.
    pub const LABEL: &str = "com.armada.daemon";

    /// Where the job is installed: `~/Library/LaunchAgents/com.armada.daemon.plist`.
    ///
    /// **`LaunchAgents`, not `LaunchDaemons`.** An agent runs as the logged-in
    /// user and needs no `sudo` to install or remove; a daemon runs as root
    /// and would need this whole verb to escalate privileges to do the one
    /// thing `034` asks for — pushing branches and opening PRs as the person
    /// who is already authenticated to `gh`, not as root.
    pub fn plist_path(home: &Path) -> PathBuf {
        home.join("Library")
            .join("LaunchAgents")
            .join(format!("{LABEL}.plist"))
    }

    /// The plist's content, as a pure function of the binary path.
    ///
    /// **`RunAtLoad` and `KeepAlive`, both true** — PLAN.md §1's own words.
    /// `RunAtLoad` is what makes `armada daemon enable` actually start the
    /// process without this module also calling [`super::start`] (this
    /// module's own header explains why calling both would be two
    /// supervisors for one job); `KeepAlive` is what makes a crash a restart
    /// rather than a silent stop nobody notices until a Job's `land` step
    /// never moves.
    pub fn plist(exe: &Path) -> String {
        format!(
            "<?xml version=\"1.0\" encoding=\"UTF-8\"?>\n\
<!DOCTYPE plist PUBLIC \"-//Apple//DTD PLIST 1.0//EN\" \"http://www.apple.com/DTDs/PropertyList-1.0.dtd\">\n\
<plist version=\"1.0\">\n\
<dict>\n\
\t<key>Label</key>\n\
\t<string>{LABEL}</string>\n\
\t<key>ProgramArguments</key>\n\
\t<array>\n\
\t\t<string>{exe}</string>\n\
\t\t<string>daemon</string>\n\
\t\t<string>run</string>\n\
\t</array>\n\
\t<key>RunAtLoad</key>\n\
\t<true/>\n\
\t<key>KeepAlive</key>\n\
\t<true/>\n\
</dict>\n\
</plist>\n",
            exe = exe.display()
        )
    }

    /// Write the plist and `launchctl load -w` it.
    ///
    /// **`-w`, so a machine where a previous `disable` left the job
    /// `Disabled` in launchd's own database still loads.** Without it a second
    /// `enable` after a `disable` would silently stay off — the same
    /// silent-no-op `034` refuses to ship anywhere in this stage.
    ///
    /// # The load is verified, because `launchctl load` does not report failure
    ///
    /// **`launchctl load -w` exits 0 whether it worked or not.** Measured on
    /// the author's own machine: it printed `Load failed: 5: Input/output
    /// error` and exited `0`, and [`run_launchctl`]'s `status.success()` read
    /// that as a success — so `armada daemon enable` answered `OK  the daemon
    /// is on on this machine` over a launchd that held nothing.
    ///
    /// **Its stderr cannot be the signal either.** The same `Load failed: 5` is
    /// what an *already loaded* job prints, which is the ordinary second
    /// `enable` and is the state the caller asked for — failing on it would
    /// break the idempotence `034` requires of every switch, and would report
    /// a daemon that is running as off.
    ///
    /// So the question is asked of launchd instead of inferred from a stream:
    /// `launchctl list <LABEL>` exits 0 when launchd holds the job and 113 when
    /// it does not. That answers *is the daemon on this machine* — which is
    /// what the envelope claims — rather than *did one command print
    /// anything*.
    pub fn install(home: &Path, exe: &Path) -> Result<(), ArmadaError> {
        let path = plist_path(home);
        if let Some(parent) = path.parent() {
            std::fs::create_dir_all(parent).map_err(|e| unwritable(&path, &e))?;
        }
        std::fs::write(&path, plist(exe)).map_err(|e| unwritable(&path, &e))?;
        run_launchctl(&["load", "-w", &path.display().to_string()], "load")?;
        loaded(&path)
    }

    /// Whether launchd is actually holding the job, which is what `enable`
    /// promises and what its own exit code could not tell.
    fn loaded(path: &Path) -> Result<(), ArmadaError> {
        match std::process::Command::new("launchctl")
            .args(["list", LABEL])
            .output()
        {
            Ok(output) if output.status.success() => Ok(()),
            // An unreadable `launchctl` is not evidence the job is absent, and
            // this module's fail-safe direction is never to invent a failure —
            // `armada daemon status` reads the pidfile and answers separately.
            Err(_) => Ok(()),
            Ok(_) => Err(ArmadaError {
                class: ErrClass::Environment,
                r#where: "daemon".to_string(),
                message: format!("launchd did not take `{LABEL}` from {}", path.display()),
                next_action: Some(
                    "run `launchctl load -w` on that path to read launchd's own words, then \
                     `armada daemon enable` again"
                        .to_string(),
                ),
            }),
        }
    }

    /// `launchctl unload -w` the job and remove the plist.
    ///
    /// **Tolerant of "not loaded"**, the same idempotence
    /// `armada helm disable` gives its own switch: a machine that never ran
    /// `enable`, or whose job launchd already dropped, is already in the
    /// state `disable` asks for, and reporting that as a failure would refuse
    /// the one command that guarantees the daemon is off.
    pub fn uninstall(home: &Path) -> Result<(), ArmadaError> {
        let path = plist_path(home);
        let _ = std::process::Command::new("launchctl")
            .args(["unload", "-w"])
            .arg(&path)
            .status();
        let _ = std::fs::remove_file(&path);
        Ok(())
    }

    fn run_launchctl(args: &[&str], what: &str) -> Result<(), ArmadaError> {
        match std::process::Command::new("launchctl").args(args).status() {
            Ok(status) if status.success() => Ok(()),
            Ok(status) => Err(ArmadaError {
                class: ErrClass::Environment,
                r#where: "daemon".to_string(),
                message: format!(
                    "`launchctl {what}` exited {}",
                    status
                        .code()
                        .map_or("without a code".to_string(), |c| c.to_string())
                ),
                next_action: Some("check `launchctl list | grep armada`, then retry".to_string()),
            }),
            Err(error) => Err(ArmadaError {
                class: ErrClass::Environment,
                r#where: "daemon".to_string(),
                message: format!("`launchctl` would not run: {error}"),
                next_action: Some("launchctl ships with macOS; check PATH".to_string()),
            }),
        }
    }

    fn unwritable(path: &Path, error: &std::io::Error) -> ArmadaError {
        ArmadaError {
            class: ErrClass::Environment,
            r#where: path.display().to_string(),
            message: format!("cannot write {}: {error}", path.display()),
            next_action: Some("check the permissions on ~/Library/LaunchAgents/".to_string()),
        }
    }

    #[cfg(test)]
    mod tests {
        use super::*;

        #[test]
        fn the_plist_path_is_under_launchagents_not_launchdaemons() {
            let home = Path::new("/Users/nick");
            assert_eq!(
                plist_path(home),
                PathBuf::from("/Users/nick/Library/LaunchAgents/com.armada.daemon.plist")
            );
        }

        /// **The real binary path, not a bare `armada`.** PLAN.md §1: "resolve
        /// the real binary path — do not hardcode `armada` unqualified".
        #[test]
        fn the_plist_names_the_resolved_binary_and_the_hidden_run_verb() {
            let text = plist(Path::new("/opt/armada/bin/armada"));
            assert!(text.contains("<string>/opt/armada/bin/armada</string>"));
            assert!(text.contains("<string>daemon</string>"));
            assert!(text.contains("<string>run</string>"));
            assert!(text.contains("<key>RunAtLoad</key>\n\t<true/>"));
            assert!(text.contains("<key>KeepAlive</key>\n\t<true/>"));
            assert!(text.contains(LABEL));
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn no_pidfile_is_not_running() {
        let home = tempfile::tempdir().unwrap();
        assert_eq!(is_running(home.path()), None);
    }

    /// **A stale pidfile reads as not running, not as an error.** Nothing on
    /// this machine has that pid, so `group_alive` says no and `is_running`
    /// agrees — the fail-safe direction this module's header promises.
    #[test]
    fn a_stale_pidfile_is_not_running() {
        let home = tempfile::tempdir().unwrap();
        // A pid this high is vanishingly unlikely to be a live process on any
        // machine this suite runs on, and if it is, `group_alive`'s own
        // `EPERM`-reads-as-false rule still keeps this from flaking into
        // "alive" for a process this test does not own.
        std::fs::write(home.path().join("daemon.pid"), "999999").unwrap();
        assert_eq!(is_running(home.path()), None);
    }

    #[test]
    fn a_pidfile_that_does_not_parse_is_not_running() {
        let home = tempfile::tempdir().unwrap();
        std::fs::write(home.path().join("daemon.pid"), "not-a-pid\n").unwrap();
        assert_eq!(is_running(home.path()), None);
    }

    #[test]
    fn a_zero_pid_is_never_alive() {
        let home = tempfile::tempdir().unwrap();
        std::fs::write(home.path().join("daemon.pid"), "0").unwrap();
        assert_eq!(is_running(home.path()), None);
    }

    /// A binary that is not on disk at all is reported as the machine being
    /// incomplete, the same class [`crate::drone::classify`] reports a
    /// missing `claude` as.
    #[test]
    fn an_exe_that_does_not_exist_is_reported_as_environment() {
        let home = tempfile::tempdir().unwrap();
        let error = start(home.path(), Path::new("/no/such/armada-binary")).unwrap_err();
        assert_eq!(error.class, ErrClass::Environment);
        assert_eq!(is_running(home.path()), None);
    }

    #[test]
    fn starting_writes_the_pidfile_and_the_log_and_is_idempotent() {
        let home = tempfile::tempdir().unwrap();
        // A fixed-purpose script, not the real `armada`, standing in for the
        // hidden entrypoint — `start`'s own reasoning permits any exe, and
        // this crate's suite is not the one that owns `armada daemon run`'s
        // own behaviour.
        let script = home.path().join("fake-armada.sh");
        std::fs::write(&script, "#!/bin/sh\nexec sleep 60\n").unwrap();
        #[cfg(unix)]
        {
            use std::os::unix::fs::PermissionsExt;
            std::fs::set_permissions(&script, std::fs::Permissions::from_mode(0o755)).unwrap();
        }

        let pid = start(home.path(), &script).expect("the script runs");
        assert!(pid > 0);
        assert_eq!(is_running(home.path()), Some(pid));

        let log_before = daemon_log::read(home.path()).unwrap();
        assert_eq!(
            log_before.len(),
            1,
            "start appends exactly one Started line"
        );

        // **Idempotent**: a second `start` while the first is still up
        // returns the same pid and writes nothing new.
        let second = start(home.path(), &script).expect("still running");
        assert_eq!(second, pid);
        assert_eq!(daemon_log::read(home.path()).unwrap().len(), 1);

        stop(home.path()).expect("stop reaps it");
        assert_eq!(is_running(home.path()), None);
        assert!(!crate::home::daemon_pidfile(home.path()).exists());
        let log_after = daemon_log::read(home.path()).unwrap();
        assert_eq!(log_after.len(), 2, "stop appends exactly one Stopped line");
    }

    /// **Stopping a daemon that was never started is nothing to stop, not an
    /// error** — [`crate::drone::Stopped::NothingToStop`]'s own case.
    #[test]
    fn stopping_with_no_pidfile_is_not_an_error_and_logs_nothing() {
        let home = tempfile::tempdir().unwrap();
        stop(home.path()).unwrap();
        assert!(daemon_log::read(home.path()).unwrap().is_empty());
    }

    /// **A pidfile naming a pid that is already dead is still cleaned up and
    /// still logged** — the ordinary shape after `launchctl unload` has
    /// already killed the launchd-managed process before `stop` runs.
    #[test]
    fn stopping_a_stale_pidfile_still_cleans_up_and_logs() {
        let home = tempfile::tempdir().unwrap();
        std::fs::write(home.path().join("daemon.pid"), "999999").unwrap();
        stop(home.path()).unwrap();
        assert!(!crate::home::daemon_pidfile(home.path()).exists());
        let entries = daemon_log::read(home.path()).unwrap();
        assert_eq!(entries.len(), 1);
        assert!(matches!(entries[0], Entry::Stopped { .. }));
    }

    #[test]
    fn record_started_is_what_the_hidden_run_entrypoint_calls_for_itself() {
        let home = tempfile::tempdir().unwrap();
        record_started(home.path(), std::process::id()).unwrap();
        assert_eq!(is_running(home.path()), Some(std::process::id()));
    }
}
