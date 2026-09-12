//! `armada mcp` — the agent's door, from the side an agent stands on.
//!
//! **Both halves of publishing it are here**: [`publish`] writes the entry into
//! a repository's own `.mcp.json` when Fleet starts, and [`speak`] is the
//! program that entry names. `api::door` is the door itself; this is its
//! audience.
//!
//! Three things decide whether a session is answered — the runtime file, the
//! repository the caller stands in, and the Manifest Fleet says it serves. The
//! second is a walk up from this process's own working directory, ending at a
//! repository root or the home directory; none of the three is a request field,
//! which is what keeps a Job id off every Drone tool.
//!
//! **None of it is authentication.** The listener is loopback with nothing in
//! front of it, so this selects a Manifest rather than granting one.

use std::io::{BufRead, Write};
use std::path::{Path, PathBuf};
use std::process::ExitCode;

use config::Manifest;
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
        Ok((fleet, adopted)) => {
            if let Some(said) = &adopted {
                eprintln!("armada mcp: {said}");
            }
            carried(&fleet, adopted.as_deref())
        }
        Err(why) => {
            // Said on stderr as well, once: the person who started the session
            // reads their client's log, and the model reads the handshake.
            eprintln!("armada mcp: {why}");
            mute(&why)
        }
    }
}

/// The Fleet this session may talk to and what the walk adopted, or the
/// sentence saying why there is not one.
fn reached() -> Result<(Loopback, Option<String>), String> {
    let cwd = std::env::current_dir()
        .map_err(|why| format!("the working directory could not be read: {why}"))?;
    let standing = standing_in(&cwd)?;
    let adopted = standing.adopted();
    let at = runtime::machine_path().map_err(|why| why.to_string())?;
    let fleet = Loopback::at(listening(runtime::read(&at), &at)?);
    // **The one refusal the walk changes the meaning of.** Being told the
    // Manifest is the wrong one is confusing to a session that does not know it
    // resolved upward, so where it did, that is said first.
    serving(standing.id(), &fleet).map_err(|why| match &adopted {
        Some(said) => format!("{said} {why}"),
        None => why,
    })?;
    answering(&fleet)?;
    Ok((fleet, adopted))
}

/// The Manifest a session resolved to, and where it was found.
///
/// **Carries the walk, not just its answer.** A session that resolved through an
/// ancestor is a session about a repository the person may not have thought they
/// were standing in, so the root it settled on is a value rather than something
/// only the search knew.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Standing {
    id: String,
    root: PathBuf,
    /// Where the session actually started, when that is not [`Standing::root`].
    /// `None` is the ordinary case: a client reads the `.mcp.json` beside the
    /// `armada.yml`, so most sessions open at a root already.
    started: Option<PathBuf>,
}

impl Standing {
    pub fn id(&self) -> &str {
        &self.id
    }

    pub fn root(&self) -> &Path {
        &self.root
    }

    /// What to add to the handshake where an ancestor was adopted. `None` where
    /// the session opened in the repository it is being answered about, because
    /// there is nothing to disclose.
    pub fn adopted(&self) -> Option<String> {
        let started = self.started.as_ref()?;
        Some(format!(
            "This session was started in {}, which has no {MANIFEST} of its own; it was resolved \
             upward to the repository at {}, and every answer here is about that one.",
            started.display(),
            self.root.display()
        ))
    }
}

/// The Manifest the caller is standing in, or the nearest one above it.
///
/// **The walk starts at the process's own working directory and nothing may
/// hand it another.** A directory a caller supplies is a directory a caller
/// chose, and a longer search does not relax that — this takes `cwd` so a test
/// can plant one, and [`speak`] is the only caller that can obtain it.
pub fn standing_in(cwd: &Path) -> Result<Standing, String> {
    found_in(cwd, std::env::var_os("HOME").map(PathBuf::from).as_deref())
}

/// The marker that ends a walk at a repository it did not start in.
///
/// A file in a worktree and a directory in a checkout, so `exists` rather than
/// `is_dir`: both are a repository root, and a Job's worktree is the first.
const REPOSITORY: &str = ".git";

/// The walk, with its ceiling handed in.
///
/// **Three things end it, and the first two are the point.** A repository root
/// with no Manifest is a repository Armada has not been set up for, and walking
/// past it would answer a session about the *parent* repository — a directory
/// nobody in that session is working in. The ceiling is the home directory,
/// because an `armada.yml` sitting there would otherwise be adopted by every
/// session anywhere under it. The filesystem root is the third and is only
/// reached from outside a home directory at all.
pub(crate) fn found_in(cwd: &Path, ceiling: Option<&Path>) -> Result<Standing, String> {
    let mut at = cwd;
    loop {
        if at.join(MANIFEST).is_file() {
            return read_at(at, cwd);
        }
        if at.join(REPOSITORY).exists() {
            return Err(format!(
                "{} is a repository Armada has not been set up for — it has no {MANIFEST}, and \
                 the repository above it is not what you are working in. Nothing here says which \
                 Jobs you would be asking about.",
                at.display()
            ));
        }
        if Some(at) == ceiling {
            break;
        }
        match at.parent() {
            Some(parent) => at = parent,
            None => break,
        }
    }
    Err(format!(
        "there is no {MANIFEST} in {}, or in any directory above it — so nothing here says which \
         repository's Jobs you would be asking about. Armada answers inside one Manifest, and the \
         repository you are standing in is what selects it.",
        cwd.display()
    ))
}

/// The Manifest at a root the walk settled on.
///
/// **A file that is there and will not parse ends the walk too.** Carrying on
/// upward would answer the session from a different repository because this
/// one's `armada.yml` has a typo in it, which is the quietest possible way to
/// be wrong.
fn read_at(root: &Path, cwd: &Path) -> Result<Standing, String> {
    match Manifest::load(&root.join(MANIFEST)) {
        Ok(manifest) => Ok(Standing {
            id: manifest.id().as_str().to_string(),
            root: root.to_path_buf(),
            started: (root != cwd).then(|| cwd.to_path_buf()),
        }),
        Err(why) => Err(format!(
            "{}'s own {MANIFEST} could not be read, so this session has no Manifest to be \
             answered inside: {why}",
            root.display()
        )),
    }
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
    match served
        .iter()
        .any(|manifest| manifest.id.as_str() == standing)
    {
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

/// Whether the door is there at all.
///
/// **One ping before the session opens.** A Fleet older than this binary is
/// serving the HTTP surface and not the agent's door, and every skew rule above
/// passes it: the protocol version says nothing about which routes exist. Found
/// here, it is a sentence; found later, it is a 404 for every tool call.
fn answering(fleet: &Loopback) -> Result<(), String> {
    let answer = fleet
        .post(api::DOOR_PATH, PING.as_bytes())
        .map_err(|why| format!("Armada did not answer: {why}"))?;
    match answer.status {
        200 => Ok(()),
        404 => Err(format!(
            "the Fleet running on this machine does not serve an agent door at {} — it is \
             older than this `armada`, and what it serves cannot be reached this way.",
            api::DOOR_PATH
        )),
        status => Err(format!(
            "Armada answered {status} when asked whether its agent door is open."
        )),
    }
}

/// The cheapest message on the door, and one with no session behind it.
const PING: &str = r#"{"jsonrpc":"2.0","id":0,"method":"ping"}"#;

/// Carry every message to the door and every answer back. One line in, at most
/// one line out, which is what the stdio transport is.
///
/// `adopted` is the one thing this adds to what Fleet said: a session resolved
/// upward from a subdirectory says so at its handshake, and nowhere else.
fn carried(fleet: &Loopback, adopted: Option<&str>) -> ExitCode {
    each_message(|message| {
        match fleet.post(api::DOOR_PATH, message) {
            // A notification: acknowledged with 202 and no body, because
            // answering one is what JSON-RPC forbids.
            Ok(answer) if answer.status == 202 => None,
            // **The status decides, and an empty body is not silence.** A 404
            // carries no body either, and reading that as a notification is a
            // client left waiting for an answer to an id nothing will mention
            // again.
            Ok(answer) if answer.status == 200 && !answer.body.is_empty() => {
                Some(noting(answer.body, adopted))
            }
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

/// The disclosure, added to the one answer that carries instructions.
///
/// **Every other answer goes through untouched**, and so does this one where
/// nothing was adopted or where the message is not a handshake: `also_saying`
/// hands back `None` and the bytes Fleet sent are the bytes written.
pub(crate) fn noting(answer: Vec<u8>, adopted: Option<&str>) -> Vec<u8> {
    let Some(adopted) = adopted else {
        return answer;
    };
    match ipc::door::also_saying(&answer, adopted) {
        Some(said) => said.into_bytes(),
        None => answer,
    }
}

/// Serve a session that reaches no Fleet: a handshake that says why, and no
/// tools.
fn mute(why: &str) -> ExitCode {
    each_message(|message| refused(message, why))
}

/// One message, answered by this process rather than by Fleet.
pub(crate) fn refused(message: &[u8], why: &str) -> Option<Vec<u8>> {
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
