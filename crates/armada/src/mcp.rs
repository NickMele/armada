//! `armada mcp` — the agent's door, from the side an agent stands on.
//!
//! **Both halves of publishing it are here**: [`publish`] writes the entry into
//! a repository's own `.mcp.json` when Fleet starts, and [`speak`] is the
//! program that entry names. `api::door` is the door itself; this is its
//! audience.
//!
//! Three things decide whether a session is answered — the runtime file, the
//! directory the caller stands in, and the Manifest Fleet says it serves. None
//! is a request field: a directory a caller supplies is a directory a caller
//! chose, which is what keeps a Job id off every Drone tool.
//!
//! **None of it is authentication.** The listener is loopback with nothing in
//! front of it, so this selects a Manifest rather than granting one.

use std::io::{BufRead, Write};
use std::path::Path;
use std::process::ExitCode;

use config::{LoadError, Manifest};
use fleet::runtime::{self, Presence, ReadError, Staleness};
use ipc::door::{Answered, Asked};
use ipc::{ManifestSummary, Skew, PROTOCOL_VERSION};

use crate::loopback::Loopback;
use crate::setup::MANIFEST;

/// Register the door in `root`'s own agent configuration.
///
/// Written when Fleet starts, because that is the moment Armada knows a
/// repository is one it serves. The entry carries no port — [`speak`] finds
/// that — so the file is written once and does not move again, which is what
/// makes it a file a repository can commit rather than a diff every boot.
pub fn publish(root: &Path) -> Result<adapters::Published, std::io::Error> {
    adapters::publish_the_agents_door(
        &root.join(adapters::REPOSITORY_CONFIG),
        PROGRAM,
        &[crate::cli::MCP],
    )
}

/// The program a repository's configuration names.
///
/// **Bare, not this process's own path.** An absolute path would pin the entry
/// to where this binary happens to sit and make the file useless in another
/// checkout; a bare name is resolved on the agent's own `PATH`, which is where
/// the person running that agent installed `armada`.
const PROGRAM: &str = "armada";

/// Relay this session, or serve it the reason it cannot be relayed.
///
/// **A refusal is served, never exited.** A server that fails to start shows
/// its person a red line and shows the model nothing at all, so every answer
/// below — including "Armada is not running" — is an MCP session that opened.
pub fn speak() -> ExitCode {
    match reached() {
        Ok(fleet) => carried(&fleet),
        Err(why) => {
            // Said on stderr as well, once: the person who started the session
            // reads their client's log, and the model reads the handshake.
            eprintln!("armada mcp: {why}");
            mute(&why)
        }
    }
}

/// The Fleet this session may talk to, or the sentence saying why there is not
/// one.
fn reached() -> Result<Loopback, String> {
    let cwd = std::env::current_dir()
        .map_err(|why| format!("the working directory could not be read: {why}"))?;
    let standing = standing_in(&cwd)?;
    let at = runtime::machine_path().map_err(|why| why.to_string())?;
    let fleet = Loopback::at(listening(runtime::read(&at), &at)?);
    serving(&standing, &fleet)?;
    Ok(fleet)
}

/// The Manifest the caller is standing in.
///
/// **The working directory, never a search upward** — `Setup::at`'s rule, and
/// it lines up with how the door is published: an agent's client reads the
/// `.mcp.json` beside the `armada.yml`, so a session that found this server at
/// all was started at a repository root.
pub fn standing_in(cwd: &Path) -> Result<String, String> {
    match Manifest::load(&cwd.join(MANIFEST)) {
        Ok(manifest) => Ok(manifest.id().as_str().to_string()),
        Err(LoadError::Unreadable { cause, .. }) if cause.kind() == std::io::ErrorKind::NotFound => {
            Err(format!(
                "there is no {MANIFEST} in {}, so nothing here says which repository's Jobs you \
                 would be asking about. Armada answers inside one Manifest, and the repository \
                 you are standing in is what selects it.{}",
                cwd.display(),
                nearest(cwd)
            ))
        }
        Err(why) => Err(format!(
            "{}'s own {MANIFEST} could not be read, so this session has no Manifest to be \
             answered inside: {why}",
            cwd.display()
        )),
    }
}

/// An ancestor that does have a Manifest, named rather than adopted.
///
/// Adopting one is what `Setup::at` refuses; saying where it is costs nothing
/// and answers the likeliest reason somebody is reading that sentence.
fn nearest(cwd: &Path) -> String {
    let mut at: Option<&Path> = cwd.parent();
    while let Some(parent) = at {
        if parent.join(MANIFEST).is_file() {
            return format!(
                " {} has one — a session started there would reach it.",
                parent.display()
            );
        }
        at = parent.parent();
    }
    String::new()
}

/// The port to knock on, out of the runtime file.
pub fn listening(read: Result<Presence, ReadError>, at: &Path) -> Result<u16, String> {
    let found = match read {
        Ok(Presence::Running(found)) => found,
        Ok(Presence::NotRunning) => {
            return Err(format!(
                "Armada is not running on this machine — there is no runtime file at {}. Ask the \
                 person you are working with to start a Fleet in this repository.",
                at.display()
            ))
        }
        Ok(Presence::Stale {
            found,
            why: Staleness::PidDead,
        }) => {
            return Err(format!(
                "Armada is not running. The runtime file at {} names pid {}, and nothing holds \
                 that pid — the Fleet that wrote it did not exit cleanly.",
                at.display(),
                found.pid
            ))
        }
        // **Not connected, on purpose.** The port in that file is now whatever
        // the machine handed out next, and sending a fleet-control request to
        // an unrelated program is the failure `Staleness` exists to name.
        Ok(Presence::Stale {
            found,
            why: Staleness::PidHeldByAnother { .. },
        }) => {
            return Err(format!(
                "Armada is not running, and this session will not connect to port {}: the \
                 runtime file at {} names pid {}, which something else holds now, so that port \
                 may belong to an unrelated program.",
                found.port,
                at.display(),
                found.pid
            ))
        }
        Err(why) => {
            return Err(format!(
                "whether Armada is running could not be established, so nothing here will \
                 connect: {why}"
            ))
        }
    };
    // Both numbers are in the file so a refusal is a sentence rather than a
    // malformed first message. Bridge reads it the same way.
    match PROTOCOL_VERSION.reading(found.protocol_version) {
        Skew::Same | Skew::FleetAhead => Ok(found.port),
        Skew::FleetBehind | Skew::Incompatible => Err(format!(
            "the Fleet running speaks protocol {} and this `armada` speaks {}, which is a gap \
             this session cannot bridge. One of the two is out of date.",
            found.protocol_version, PROTOCOL_VERSION
        )),
    }
}

/// Whether the Fleet at that port is serving the Manifest the caller stands in.
fn serving(standing: &str, fleet: &Loopback) -> Result<(), String> {
    let answer = fleet
        .get(MANIFESTS)
        .map_err(|why| format!("Armada did not answer: {why}"))?;
    if answer.status != 200 {
        return Err(format!(
            "Armada answered {} when asked which Manifest it serves, so this session cannot be \
             scoped to one.",
            answer.status
        ));
    }
    let served: Vec<ManifestSummary> = ipc::decode("manifest list", &answer.body)
        .map_err(|why| format!("what Armada serves could not be read: {why}"))?;
    stands_in(standing, &served)
}

/// Where Fleet says which Manifest it holds: the route `list_manifests` is
/// served at, and the one request here that is not a tool call.
const MANIFESTS: &str = "/manifests";

/// The comparison itself, which is the whole of the scoping rule.
pub fn stands_in(standing: &str, served: &[ManifestSummary]) -> Result<(), String> {
    match served.iter().any(|manifest| manifest.id.as_str() == standing) {
        true => Ok(()),
        false => Err(format!(
            "you are standing in Manifest `{standing}`, and the Fleet on this machine is serving \
             {}. Nothing outside the Manifest you are standing in is reachable here.",
            listed(served)
        )),
    }
}

fn listed(served: &[ManifestSummary]) -> String {
    match served.is_empty() {
        true => String::from("no Manifest at all"),
        false => served
            .iter()
            .map(|manifest| format!("`{}` ({})", manifest.id.as_str(), manifest.path))
            .collect::<Vec<_>>()
            .join(", "),
    }
}

/// Carry every message to the door and every answer back. One line in, at most
/// one line out, which is what the stdio transport is.
fn carried(fleet: &Loopback) -> ExitCode {
    each_message(|message| {
        match fleet.post(api::DOOR_PATH, message) {
            // A notification: acknowledged with no body, because answering one
            // is what JSON-RPC forbids.
            Ok(answer) if answer.status == 202 || answer.body.is_empty() => None,
            Ok(answer) if answer.status == 200 => Some(answer.body),
            Ok(answer) => refused(
                message,
                &format!(
                    "Armada answered {} to that call rather than a JSON-RPC message",
                    answer.status
                ),
            ),
            // **The Fleet went away mid-session.** Answered rather than
            // dropped: a client waiting on an id it will never hear about is a
            // session that hangs instead of a session that is told.
            Err(why) => refused(message, &format!("Armada stopped answering: {why}")),
        }
    })
}

/// Serve a session that reaches no Fleet: a handshake that says why, and no
/// tools.
fn mute(why: &str) -> ExitCode {
    each_message(|message| refused(message, why))
}

/// One message, answered by this process rather than by Fleet.
fn refused(message: &[u8], why: &str) -> Option<Vec<u8>> {
    // No shapes, because there is no Fleet serving any: a `tools/call` that
    // arrives is refused by name before anything else looks at it.
    let answered = match ipc::door::read(message, &[]) {
        Asked::Nothing => return None,
        Asked::Handshake { id, revision } => Answered::Unreachable {
            id,
            revision,
            why: why.to_string(),
        },
        Asked::Ping { id } => Answered::Ping { id },
        Asked::Tools { id } => Answered::Tools {
            id,
            shapes: Vec::new(),
        },
        Asked::Call { id, .. } | Asked::NotACall { id, .. } => Answered::Refused {
            id,
            why: why.to_string(),
        },
        Asked::NoSuchMethod { id, named } => Answered::NoSuchMethod { id, named },
        Asked::Unreadable { why } => Answered::Unreadable { why },
    };
    ipc::door::answer(answered).ok().map(String::into_bytes)
}

/// The transport: newline-delimited JSON, on this process's own stdin and
/// stdout.
fn each_message(mut answer: impl FnMut(&[u8]) -> Option<Vec<u8>>) -> ExitCode {
    let stdin = std::io::stdin();
    let mut out = std::io::stdout();
    for message in stdin.lock().split(b'\n') {
        let Ok(message) = message else {
            // stdin is not readable. The session is over, and there is nothing
            // left to answer and nothing left to say that to.
            return ExitCode::FAILURE;
        };
        if message.iter().all(u8::is_ascii_whitespace) {
            continue;
        }
        let Some(said) = answer(&message) else {
            continue;
        };
        // **Flushed per message.** The client is blocked on this answer, and a
        // buffered one is a session that looks wedged.
        if out
            .write_all(&said)
            .and_then(|()| out.write_all(b"\n"))
            .and_then(|()| out.flush())
            .is_err()
        {
            return ExitCode::FAILURE;
        }
    }
    ExitCode::SUCCESS
}
