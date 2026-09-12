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

use crate::mcp::{found_in, listening, noting, publish, refused, standing_in, stands_in};
use crate::tests::{repository, TempDir};

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
/// the Manifest that names it, and says nothing about a walk it did not make.
#[test]
fn the_repository_you_are_standing_in_is_the_manifest_you_are_asking_about() {
    let dir = TempDir::new();
    dir.write("armada.yml", "version: 1\nid: 01FIXTUREMANIFEST\n");

    let standing = standing_in(dir.path()).expect("the Manifest here");

    assert_eq!(standing.id(), "01FIXTUREMANIFEST");
    assert_eq!(standing.root(), dir.path());
    assert_eq!(standing.adopted(), None, "nothing was adopted");
}

/// **A session started below a root resolves upward to it**, because a person
/// working in a subdirectory is working in the repository above it.
#[test]
fn a_subdirectory_resolves_to_the_repository_above_it() {
    let dir = TempDir::new();
    dir.write("armada.yml", "version: 1\nid: 01FIXTUREMANIFEST\n");
    let inner = dir.path().join("packages/inner");
    std::fs::create_dir_all(&inner).expect("a subdirectory");

    let standing = standing_in(&inner).expect("the Manifest above");

    assert_eq!(standing.id(), "01FIXTUREMANIFEST");
    assert_eq!(standing.root(), dir.path());
}

/// **What it adopted is disclosed, not assumed.** Answering a session about a
/// repository its person did not think they were standing in is the failure the
/// walk introduces, and the handshake is where a model reads what it is in.
#[test]
fn a_session_that_resolved_upward_says_so_and_names_the_root() {
    let dir = TempDir::new();
    dir.write("armada.yml", "version: 1\nid: 01FIXTUREMANIFEST\n");
    let inner = dir.path().join("packages/inner");
    std::fs::create_dir_all(&inner).expect("a subdirectory");

    let said = standing_in(&inner)
        .expect("the Manifest above")
        .adopted()
        .expect("a session that walked says so");

    assert!(said.contains(&inner.display().to_string()), "{said}");
    assert!(said.contains(&dir.path().display().to_string()), "{said}");
}

/// **The walk stops at a repository root.** Carrying on would answer a session
/// about the repository *containing* the one somebody is working in — which is
/// a directory nobody in that session has open.
#[test]
fn a_repository_with_no_manifest_ends_the_walk_rather_than_escaping_it() {
    let outer = TempDir::new();
    outer.write("armada.yml", "version: 1\nid: 01OUTERMANIFEST\n");
    let inner = outer.path().join("vendor/theirs");
    std::fs::create_dir_all(inner.join(".git")).expect("a repository of their own");

    let said = standing_in(&inner).expect_err("their repository is not ours");

    assert!(said.contains("has not been set up for"), "{said}");
    assert!(said.contains(&inner.display().to_string()), "{said}");
    assert!(!said.contains("01OUTERMANIFEST"), "{said}");
}

/// The other end of the walk. **A Manifest in a home directory is not every
/// session's Manifest**, so the search stops there rather than at `/`.
#[test]
fn the_walk_stops_at_the_home_directory() {
    let home = TempDir::new();
    home.write("armada.yml", "version: 1\nid: 01HOMEMANIFEST\n");
    let under = home.path().join("scratch");
    std::fs::create_dir_all(&under).expect("a directory under it");

    assert_eq!(
        found_in(&under, Some(home.path()))
            .expect("the home directory is itself searched")
            .id(),
        "01HOMEMANIFEST"
    );
    let said = found_in(&under, Some(&under)).expect_err("nothing above may be read");
    assert!(said.contains("or in any directory above it"), "{said}");
}

/// **A Manifest that is there and will not parse ends the walk**, rather than
/// being stepped over on the way to a different repository's.
#[test]
fn a_manifest_that_will_not_parse_is_a_refusal_and_not_a_guess() {
    let outer = TempDir::new();
    outer.write("armada.yml", "version: 1\nid: 01OUTERMANIFEST\n");
    let inner = outer.path().join("inner");
    std::fs::create_dir_all(&inner).expect("a subdirectory");
    std::fs::write(
        inner.join("armada.yml"),
        "version: 1\nid: 01FIXTURE\nnonsense: true\n",
    )
    .expect("a Manifest with a key nothing reads");

    let said = standing_in(&inner).expect_err("this one is refused");

    assert!(said.contains("could not be read"), "{said}");
    assert!(!said.contains("01OUTERMANIFEST"), "{said}");
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

    assert!(
        said.contains("you are standing in Manifest `mine`"),
        "{said}"
    );
    assert!(
        said.contains("theirs"),
        "what it is serving is named: {said}"
    );
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
    let said = answered(
        br#"{"jsonrpc":"2.0","id":2,"method":"tools/list"}"#,
        "nothing here",
    );

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
        refused(
            br#"{"jsonrpc":"2.0","method":"notifications/initialized"}"#,
            "why"
        ),
        None,
        "JSON-RPC forbids answering one"
    );
}

fn answered(message: &[u8], why: &str) -> String {
    String::from_utf8(refused(message, why).expect("an answer")).expect("text")
}

/// **This repository's own `.mcp.json` already names the door**, byte for byte
/// what publishing would write — so a session opened here reaches Fleet without
/// waiting for the next `armada serve` to rewrite the file.
///
/// Asserted against a copy, so the case reads the repository and writes
/// nothing in it.
#[test]
fn this_repository_publishes_its_own_door() {
    let dir = TempDir::new();
    let theirs =
        std::fs::read_to_string(repository().join(adapters::REPOSITORY_CONFIG)).expect("the file");
    dir.write(adapters::REPOSITORY_CONFIG, &theirs);

    assert_eq!(
        publish(dir.path()).expect("the entry"),
        adapters::Published::AlreadyThere,
        "this repository's {} is not what publishing writes:\n{theirs}",
        adapters::REPOSITORY_CONFIG
    );
}

/// The disclosure rides on Fleet's own handshake rather than replacing it: the
/// scope sentence is still there, and every other answer is untouched.
#[test]
fn the_note_is_added_to_the_handshake_and_to_nothing_else() {
    let handshake = br#"{"jsonrpc":"2.0","id":1,"result":{"instructions":"Every answer is inside Manifest armada."}}"#;
    let listed = br#"{"jsonrpc":"2.0","id":2,"result":{"tools":[]}}"#;

    let noted = String::from_utf8(noting(handshake.to_vec(), Some("It walked up."))).expect("text");

    assert!(noted.contains("It walked up."), "{noted}");
    assert!(
        noted.contains("Every answer is inside Manifest armada."),
        "{noted}"
    );
    assert_eq!(
        noting(listed.to_vec(), Some("It walked up.")),
        listed.to_vec(),
        "an answer with no instructions is carried through byte for byte"
    );
    assert_eq!(
        noting(handshake.to_vec(), None),
        handshake.to_vec(),
        "and so is a handshake where nothing was adopted"
    );
}
