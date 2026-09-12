//! What an agent standing in a repository is told, and when it is told it
//! instead of being connected.
//!
//! **No socket and no Fleet.** Each of the three decisions is a value in and a
//! sentence out, so what is asserted here is the decision and the wording — and
//! the wording is the whole product for an agent that cannot be connected.

use std::path::Path;

use fleet::process::StartedAt;
use fleet::runtime::{Presence, RuntimeFile, Staleness};
use ipc::{ManifestId, ManifestSummary, ProtocolVersion, PROTOCOL_VERSION};

use crate::mcp::{listening, refused, stands_in, standing_in};
use crate::tests::TempDir;

fn file(version: ProtocolVersion) -> RuntimeFile {
    RuntimeFile {
        protocol_version: version,
        pid: 4242,
        port: 51888,
        started_at: StartedAt::carried("Tue Sep  1 09:00:00 2026"),
    }
}

fn machine() -> &'static Path {
    Path::new("/Machine/Armada/fleet.json")
}

fn manifest(id: &str) -> ManifestSummary {
    ManifestSummary {
        id: ManifestId::carried(id),
        repository: String::from("armada"),
        path: format!("/repos/{id}/armada.yml"),
        records_root: String::from("/Machine/Armada/records"),
        version: 1,
        checks: Vec::new(),
    }
}

/// The definition of done's first half: a repository Armada knows resolves to
/// the Manifest that names it.
#[test]
fn the_repository_you_are_standing_in_is_the_manifest_you_are_asking_about() {
    let dir = TempDir::new();
    dir.write("armada.yml", "version: 1\nid: 01FIXTUREMANIFEST\n");

    assert_eq!(
        standing_in(dir.path()),
        Ok(String::from("01FIXTUREMANIFEST"))
    );
}

/// The second half: a repository Armada does not know is told so. **Never
/// answered from another repository's work**, which is what a search upward
/// would quietly do.
#[test]
fn a_directory_with_no_manifest_is_told_so_and_names_the_nearest_one() {
    let dir = TempDir::new();
    dir.write("armada.yml", "version: 1\nid: 01FIXTUREMANIFEST\n");
    std::fs::create_dir_all(dir.path().join("packages/inner")).expect("a subdirectory");

    let said = standing_in(&dir.path().join("packages/inner")).expect_err("no Manifest there");

    assert!(said.contains("there is no armada.yml"), "{said}");
    assert!(
        said.contains(&dir.path().display().to_string()),
        "the repository above is named rather than adopted: {said}"
    );
}

#[test]
fn a_manifest_that_will_not_parse_is_a_refusal_and_not_a_guess() {
    let dir = TempDir::new();
    dir.write("armada.yml", "version: 1\nid: 01FIXTURE\nnonsense: true\n");

    let said = standing_in(dir.path()).expect_err("a Manifest with a key nothing reads");

    assert!(said.contains("could not be read"), "{said}");
}

#[test]
fn a_running_fleet_is_the_port_in_the_file() {
    assert_eq!(
        listening(Ok(Presence::Running(file(PROTOCOL_VERSION))), machine()),
        Ok(51888)
    );
}

/// **The one the issue names.** A pid something else holds means the port in
/// that file may be an unrelated program's, and the answer is a sentence rather
/// than a connection.
#[test]
fn a_pid_held_by_another_is_refused_rather_than_guessed_at() {
    let said = listening(
        Ok(Presence::Stale {
            found: file(PROTOCOL_VERSION),
            why: Staleness::PidHeldByAnother {
                holder: StartedAt::carried("Wed Sep  2 11:00:00 2026"),
            },
        }),
        machine(),
    )
    .expect_err("nothing is connected to");

    assert!(said.contains("will not connect to port 51888"), "{said}");
    assert!(said.contains("unrelated program"), "{said}");
}

#[test]
fn the_two_not_running_answers_stay_two_answers() {
    let absent = listening(Ok(Presence::NotRunning), machine()).expect_err("nothing running");
    let dead = listening(
        Ok(Presence::Stale {
            found: file(PROTOCOL_VERSION),
            why: Staleness::PidDead,
        }),
        machine(),
    )
    .expect_err("nothing running");

    assert!(absent.contains("there is no runtime file"), "{absent}");
    assert!(dead.contains("did not exit cleanly"), "{dead}");
    assert_ne!(absent, dead, "two events, two sentences");
}

/// A Fleet older than this binary is refused for Bridge's reason: additive-only
/// promises nothing about what a newer reader needs.
#[test]
fn a_fleet_behind_this_binary_is_not_connected_to() {
    let behind = ProtocolVersion::new(PROTOCOL_VERSION.major, PROTOCOL_VERSION.minor + 1);
    let ahead = listening(Ok(Presence::Running(file(behind))), machine());
    let older = ProtocolVersion::new(PROTOCOL_VERSION.major + 1, 0);
    let said = listening(Ok(Presence::Running(file(older))), machine())
        .expect_err("a major gap is not bridged");

    assert_eq!(ahead, Ok(51888), "a Fleet ahead is connected to");
    assert!(said.contains("out of date"), "{said}");
}

#[test]
fn a_fleet_serving_another_manifest_answers_nothing_about_this_one() {
    let said = stands_in("mine", &[manifest("theirs")]).expect_err("a different Manifest");

    assert!(said.contains("you are standing in Manifest `mine`"), "{said}");
    assert!(said.contains("theirs"), "what it is serving is named: {said}");
    assert!(
        said.contains("/repos/theirs/armada.yml"),
        "and where that one is: {said}"
    );
    assert_eq!(stands_in("mine", &[manifest("mine")]), Ok(()));
}

/// **A refusal is a session that opened.** The handshake succeeds and carries
/// the reason, because a server that fails to start tells the person running
/// the client and never tells the model in it.
#[test]
fn a_session_that_reaches_no_fleet_is_told_at_the_handshake() {
    let why = "Armada is not running on this machine";
    let said = answered(
        br#"{"jsonrpc":"2.0","id":1,"method":"initialize","params":{"protocolVersion":"2025-06-18"}}"#,
        why,
    );

    assert!(said.contains(why), "{said}");
    assert!(said.contains("\"serverInfo\""), "it is a handshake: {said}");
    assert!(!said.contains("\"error\""), "not a failed one: {said}");
}

#[test]
fn a_session_that_reaches_no_fleet_offers_no_tools() {
    let said = answered(br#"{"jsonrpc":"2.0","id":2,"method":"tools/list"}"#, "nothing here");

    assert!(said.contains(r#""tools":[]"#), "{said}");
}

/// A call invented anyway is refused as a tool error rather than as a status
/// code: an agent reads the one and can only retry the other.
#[test]
fn a_call_made_anyway_is_refused_in_the_answer() {
    let why = "you are standing in Manifest `mine`";
    let said = answered(
        br#"{"jsonrpc":"2.0","id":3,"method":"tools/call","params":{"name":"list_jobs"}}"#,
        why,
    );

    assert!(said.contains("\"isError\":true"), "{said}");
    assert!(said.contains(why), "{said}");
}

#[test]
fn a_notification_is_not_answered_at_all() {
    assert_eq!(
        refused(br#"{"jsonrpc":"2.0","method":"notifications/initialized"}"#, "why"),
        None,
        "JSON-RPC forbids answering one"
    );
}

fn answered(message: &[u8], why: &str) -> String {
    String::from_utf8(refused(message, why).expect("an answer")).expect("text")
}
