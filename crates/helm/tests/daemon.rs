//! `armada daemon enable`/`disable` through the real binary — **where the
//! launchd job actually lands**.
//!
//! # Why this file exists
//!
//! `armada_fleet::daemon::launchd::plist_path` had a unit test, and it was
//! green for the whole time the feature was broken. It asserted
//! `plist_path("/Users/nick")` is `/Users/nick/Library/LaunchAgents/…`, which
//! is true and was never the question — the question is *what the call sites
//! hand it*, and all three of them handed it `~/.armada`. `$HOME` and
//! `~/.armada` are both `&Path`, so nothing refused the swap, and the plist was
//! written to `~/.armada/Library/LaunchAgents/com.armada.daemon.plist`: a path
//! launchd does not read at login, so the daemon whose promise is to survive
//! one did not.
//!
//! Measured on the author's own machine before the fix: that file present,
//! `~/Library/LaunchAgents/` holding nothing matching `armada`.
//!
//! So these tests run the real binary against a scratch `$HOME` and look at
//! **which directory the file is in afterwards**. A function tested in
//! isolation cannot be wrong about that; a call chain can.
//!
//! # `launchctl` is stubbed, and it has to be
//!
//! `enable` loads the job into the *logged-in user's* launchd session, under
//! the same `com.armada.daemon` label a developer's own machine uses. A suite
//! that called the real one would install a `KeepAlive` job pointing at a test
//! binary on whoever ran it. So a stub goes first on `PATH`, records the vector
//! it was given, and answers.
//!
//! That makes the launchctl assertions *stronger* than a fake's would be: what
//! is recorded is what `execve` received.
//!
//! **macOS only, because the launchd job is** (`034` §1, and
//! `armada_fleet::daemon::enable_unsupported` refuses it by name everywhere
//! else). There is no plist to look for on any other machine.
#![cfg(target_os = "macos")]

mod support;

use serde_json::Value;
use std::path::{Path, PathBuf};
use support::Machine;

/// The label the plist and every `launchctl` argument name.
const LABEL: &str = "com.armada.daemon";

/// A `launchctl` on `PATH` that records its argv and exits how the test says.
///
/// `list_exit` is what `launchctl list <label>` answers with: `0` when launchd
/// holds the job, and `113` — macOS's own code for it — when it does not.
/// **`load` always exits 0**, because the real one does even when it has just
/// printed `Load failed: 5: Input/output error`, and that is the whole reason
/// `install` asks a second question.
fn stub_launchctl(machine: &Machine, list_exit: u8) -> (PathBuf, PathBuf) {
    let dir = machine.root.path().join("launchctl-stub");
    std::fs::create_dir_all(&dir).unwrap();
    let log = dir.join("argv.log");
    let script = dir.join("launchctl");
    std::fs::write(
        &script,
        format!(
            "#!/bin/sh\n\
             printf '%s\\n' \"$*\" >> '{log}'\n\
             case \"$1\" in\n\
             list) exit {list_exit} ;;\n\
             esac\n\
             exit 0\n",
            log = log.display(),
        ),
    )
    .unwrap();
    use std::os::unix::fs::PermissionsExt;
    std::fs::set_permissions(&script, std::fs::Permissions::from_mode(0o755)).unwrap();
    (dir, log)
}

/// Run a verb with the stub first on `PATH`.
fn daemon(machine: &Machine, stub: &Path, args: &[&str]) -> Value {
    let path = format!("{}:/usr/bin:/bin", stub.display());
    let out = machine.run_with_env(machine.root.path(), args, &[("PATH", &path)]);
    serde_json::from_slice(&out.stdout)
        .unwrap_or_else(|_| panic!("an envelope: {}", support::why(&out)))
}

fn recorded(log: &Path) -> Vec<String> {
    std::fs::read_to_string(log)
        .unwrap_or_default()
        .lines()
        .map(str::to_string)
        .collect()
}

/// **The deliverable of this file.** The plist goes in the user's
/// `Library/LaunchAgents`, and nothing at all is written under `~/.armada`'s
/// own tree — which is where every call site used to put it.
#[test]
fn the_launchd_job_lands_in_the_users_library_and_not_under_armadas_home() {
    let machine = Machine::new();
    let (stub, log) = stub_launchctl(&machine, 0);
    let envelope = daemon(&machine, &stub, &["daemon", "enable", "--json"]);
    assert_eq!(envelope["status"], "OK", "{envelope}");

    let home = machine.home.path();
    let plist = home
        .join("Library/LaunchAgents")
        .join(format!("{LABEL}.plist"));
    assert!(
        plist.is_file(),
        "the launchd job is not in ~/Library/LaunchAgents: {}",
        plist.display()
    );
    // The bug, stated as the thing that must not be true. `~/.armada` is
    // Armada's own directory and launchd has never heard of it.
    assert!(
        !home.join(".armada/Library").exists(),
        "the plist was written under ~/.armada again"
    );

    // And launchd was asked to load the file that was actually written, rather
    // than a path assembled a second time from different pieces.
    let argv = recorded(&log);
    assert!(
        argv.iter()
            .any(|line| line == &format!("load -w {}", plist.display())),
        "launchctl was not asked to load the plist that was written: {argv:?}"
    );

    // The plist names this build, not a bare `armada` off some future `PATH`.
    let text = std::fs::read_to_string(&plist).unwrap();
    assert!(
        text.contains(&format!(
            "<string>{}</string>",
            support::armada_binary().display()
        )),
        "{text}"
    );
}

/// **`disable` removes the file `enable` wrote**, which is the same fact from
/// the other side: an `uninstall` handed the wrong directory unloads a path
/// launchd was never told about and leaves the installed plist in place.
#[test]
fn disable_removes_the_job_from_the_directory_enable_put_it_in() {
    let machine = Machine::new();
    let (stub, log) = stub_launchctl(&machine, 0);
    daemon(&machine, &stub, &["daemon", "enable", "--json"]);

    let plist = machine
        .home
        .path()
        .join("Library/LaunchAgents")
        .join(format!("{LABEL}.plist"));
    assert!(plist.is_file(), "nothing was installed to remove");

    let envelope = daemon(&machine, &stub, &["daemon", "disable", "--json"]);
    assert_eq!(envelope["status"], "OK", "{envelope}");
    assert!(
        !plist.exists(),
        "the plist survived `disable`: {}",
        plist.display()
    );

    let argv = recorded(&log);
    assert!(
        argv.iter()
            .any(|line| line == &format!("unload -w {}", plist.display())),
        "launchctl was not asked to unload the installed plist: {argv:?}"
    );
}

/// **A load launchd refused reaches the envelope.**
///
/// `launchctl load -w` exits 0 whether it worked or not — measured: it printed
/// `Load failed: 5: Input/output error` and exited `0` — so the owner ran
/// `armada daemon enable`, read that line, and was then told `OK  the daemon is
/// on on this machine` by the same command. A switch that reports a state it
/// did not reach is the silent stall `034` exists to end.
#[test]
fn a_load_launchd_refused_is_reported_rather_than_called_ok() {
    let machine = Machine::new();
    // `load` still exits 0, exactly as the real one does. What says the job is
    // not there is launchd's own answer to `list`.
    let (stub, log) = stub_launchctl(&machine, 113);
    let envelope = daemon(&machine, &stub, &["daemon", "enable", "--json"]);

    assert_eq!(envelope["status"], "FAILED", "{envelope}");
    assert_eq!(envelope["error"]["class"], "environment", "{envelope}");
    assert!(
        envelope["error"]["message"]
            .as_str()
            .unwrap()
            .contains(LABEL),
        "the refusal does not name the job: {envelope}"
    );
    assert!(
        recorded(&log)
            .iter()
            .any(|line| line == "list com.armada.daemon"),
        "launchd was never asked whether it took the job"
    );
}
