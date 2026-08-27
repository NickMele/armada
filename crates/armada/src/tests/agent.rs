//! Which binary a Drone is started as, answered before the bind.
//!
//! **No process is started here.** Whether a name resolves is a question about
//! a filesystem and a `PATH`, and the whole point of asking it in this crate is
//! that it is answered before the bind rather than at the first Drone.
//!
//! Nothing below writes the agent CLI's name. It is a vendor's and `adapters`
//! owns it; these read it back through `HeadlessAgent::program()`, so the
//! settings default can change without a test here having to be edited to agree
//! with it.

use std::os::unix::fs::PermissionsExt;

use adapters::HeadlessAgent;

use crate::agent::agent_binary;
use crate::serve::drone_path;
use crate::tests::TempDir;

/// A directory holding something runnable under `name`, for a `PATH` to find.
fn holding(dir: &TempDir, name: &str) -> String {
    dir.write(name, "#!/bin/sh\nexit 0\n");
    let at = dir.path().join(name);
    let mut how = std::fs::metadata(&at).expect("the file").permissions();
    how.set_mode(0o755);
    std::fs::set_permissions(&at, how).expect("an execute bit");
    dir.path().to_string_lossy().into_owned()
}

/// **The unset case is the ordinary case**, on a machine where the CLI is
/// installed somewhere a Drone will look.
#[test]
fn a_fleet_with_no_named_agent_binary_gets_the_settings_default() {
    let dir = TempDir::new();
    let installed = holding(&dir, HeadlessAgent::on_path().program());

    let harness = agent_binary(None, &installed).expect("it is there, so it is not a refusal");
    assert_eq!(harness, HeadlessAgent::on_path());
}

/// **The test that matters.** The default is probed like any other name, so a
/// machine without the CLI is refused at the terminal rather than at the first
/// Drone — after a port, a runtime file, an accepted Job and a row on the Board.
#[test]
fn a_default_that_is_not_installed_is_refused_before_the_bind() {
    let refused =
        agent_binary(None, "/nowhere/at/all").expect_err("an absent CLI is a refusal, not a start");

    let said = refused.to_string();
    assert!(
        said.contains(HeadlessAgent::on_path().program()),
        "the refusal names the binary: {said}"
    );
    assert!(
        said.contains("fail at the first Drone"),
        "and says what starting anyway would have cost: {said}"
    );
}

/// The `PATH` it looked on, in the message. `which` succeeding in the
/// operator's own shell says nothing about a Drone's `PATH`, and a refusal that
/// left it out cost half an hour of diagnosis.
#[test]
fn a_refusal_names_the_path_it_searched() {
    let refused = agent_binary(None, "/one/place:/another").expect_err("a refusal");
    assert_eq!(refused.path(), "/one/place:/another");
    assert!(refused.to_string().contains("/one/place:/another"));
}

/// A binary somebody named that is not there is refused, **and the refusal
/// carries the name** — the one thing the reader has to go and fix.
#[test]
fn a_named_binary_that_is_not_there_is_refused_by_name() {
    let refused = agent_binary(Some("no-such-agent-binary".to_string()), "/usr/bin:/bin")
        .expect_err("a name with nothing behind it is a refusal");
    let said = refused.to_string();
    assert!(
        said.contains("no-such-agent-binary"),
        "the refusal names what was named: {said}"
    );
    assert!(
        said.contains("ARMADA_AGENT_BINARY"),
        "and where the name came from: {said}"
    );
}

/// A path, rather than a bare name, is answered by that one place.
#[test]
fn a_named_path_with_nothing_at_it_is_refused_whatever_the_path_holds() {
    let refused = agent_binary(Some("/nowhere/at/all/agent".to_string()), "/usr/bin:/bin")
        .expect_err("a path to nothing is a refusal");
    assert!(refused.to_string().contains("/nowhere/at/all/agent"));
}

/// The override that resolves. Two spellings of one binary this machine
/// certainly has — a bare name found on the `PATH`, and the same file named
/// outright.
#[test]
fn a_named_binary_that_is_there_is_the_one_fleet_uses() {
    assert_eq!(
        agent_binary(Some("sh".to_string()), "/nowhere:/bin").expect("`sh` is on `/bin`"),
        HeadlessAgent::at("sh")
    );
    assert_eq!(
        agent_binary(Some("/bin/sh".to_string()), "").expect("`/bin/sh` is a file that runs"),
        HeadlessAgent::at("/bin/sh")
    );
}

/// A directory is not runnable, and neither is a file with no execute bit. Both
/// are names that would fail at spawn, which is the failure this probe exists to
/// move to the terminal the operator is standing at.
#[test]
fn something_that_is_not_runnable_is_not_an_agent() {
    assert!(agent_binary(Some("/bin".to_string()), "").is_err());
    assert!(agent_binary(Some("/etc/hosts".to_string()), "").is_err());
}

// -------------------------------------------------- the PATH a Drone is given

/// `~/.local/bin` is where the agent CLI's own native installer puts it, and
/// leaving it off is what killed a Drone spawn with *no such file or directory*
/// on a machine where the CLI was installed the ordinary way.
#[test]
fn a_drones_path_holds_the_directory_the_clis_installer_writes_to() {
    let path = drone_path("/home/user");
    let entries: Vec<&str> = path.split(':').collect();
    assert!(
        entries.contains(&"/home/user/.local/bin"),
        "a Drone would not find the CLI: {entries:?}"
    );
    assert!(entries.contains(&"/home/user/.cargo/bin"));
}

/// Per-user first, so a toolchain somebody installed for themselves wins over
/// whatever the system happens to carry.
#[test]
fn a_drones_path_puts_the_per_user_directories_before_the_system_ones() {
    let path = drone_path("/home/user");
    let entries: Vec<&str> = path.split(':').collect();
    assert_eq!(
        &entries[..2],
        ["/home/user/.cargo/bin", "/home/user/.local/bin"]
    );
}
