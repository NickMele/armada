//! Why a server was not started, stopped or watched, and how each is spelled
//! on the wire. **Each is checked before anything is spawned.**

use std::fmt;

use api::Refusal;
use ipc::WireError;

/// A name that declares no server here.
const NOT_A_SERVER: &str = "fleet.not_a_server";
/// A server that cannot start where it was asked for, or stop.
const CANNOT_SERVE_HERE: &str = "fleet.cannot_serve_here";
/// An id Fleet holds no server by.
const NO_SUCH_SERVER: &str = "fleet.no_such_server";
/// Its directory would not write, or it did not end once stopped.
const SERVER_FAULT: &str = "fleet.server_fault";

/// What this Fleet's last read of `armada.yml` came to, carried on the one
/// refusal that would otherwise name a list from boot as though it were
/// current.
///
/// **The list is the untrustworthy part.** A Fleet holds the Manifest it
/// resolved at startup, so a server added to `armada.yml` since is declared by
/// the repository and unknown to Fleet — and `it declares a, b, c` reads as
/// the repository's answer rather than as this process's memory of it. `#1564`.
#[derive(Debug)]
pub struct LastRead {
    /// When Fleet read the file — not when it was saved.
    pub at: String,
    /// Sections that changed in that read and were **not** adopted, as
    /// `armada.yml` spells them. `commands` is where a server is declared.
    pub at_restart: Vec<String>,
}

/// Where a server is declared, so the one section this refusal cares about is
/// named once.
const COMMANDS: &str = "commands";

#[derive(Debug)]
pub enum Unservable {
    NotAServer {
        name: String,
        servers: Vec<String>,
        /// Absent where this Fleet has not re-read the file since it started,
        /// which is the ordinary case and says nothing either way.
        last_read: Option<LastRead>,
    },
    IsACommand {
        name: String,
    },
    NoWorktree,
    JobEnded,
    /// `destructive: true`, asked for by a Drone. A person's start needs no
    /// second approval; a Drone's has nobody to ask inside a tool call.
    NeedsAPerson {
        name: String,
    },
    NoSuchServer {
        id: String,
    },
    NotRunning {
        id: String,
    },
    NotKept {
        why: String,
    },
}

impl Unservable {
    /// The code and the refusal. **A conflict with where things stand is a
    /// 409; a name that names nothing is a 422**; a disk that would not write
    /// is Fleet's fault.
    pub(crate) fn spelled(&self) -> (&'static str, fn(WireError) -> Refusal) {
        use Unservable::*;
        match self {
            NotAServer { .. } | IsACommand { .. } => (NOT_A_SERVER, Refusal::Unacceptable),
            NoSuchServer { .. } => (NO_SUCH_SERVER, Refusal::Unacceptable),
            NoWorktree | JobEnded | NeedsAPerson { .. } | NotRunning { .. } => {
                (CANNOT_SERVE_HERE, Refusal::IllegalMove)
            }
            NotKept { .. } => (SERVER_FAULT, Refusal::Fault),
        }
    }
}

impl fmt::Display for Unservable {
    fn fmt(&self, out: &mut fmt::Formatter<'_>) -> fmt::Result {
        use Unservable::*;
        match self {
            NotAServer {
                name,
                servers,
                last_read,
            } if servers.is_empty() => write!(
                out,
                "`{name}` is not a server this project declares, and it declares none. A \
                 server is a Command with `serve`{}",
                since(last_read.as_ref())
            ),
            NotAServer {
                name,
                servers,
                last_read,
            } => write!(
                out,
                "`{name}` is not a server this project declares — it declares {}{}",
                servers
                    .iter()
                    .map(|one| format!("`{one}`"))
                    .collect::<Vec<_>>()
                    .join(", "),
                since(last_read.as_ref())
            ),
            IsACommand { name } => write!(
                out,
                "`{name}` declares no `serve`: it is a Command that runs and exits, so there \
                 is nothing to keep running"
            ),
            NoWorktree => out.write_str(
                "this Job's worktree is no longer on disk, so there is nowhere to serve from",
            ),
            JobEnded => out.write_str("this Job has ended, and its servers stopped with it"),
            NeedsAPerson { name } => write!(
                out,
                "`{name}` is declared `destructive: true`, so a person starts it. Ask them \
                 to start it from the run sheet"
            ),
            NoSuchServer { id } => write!(out, "no server Fleet holds is called `{id}`"),
            NotRunning { id } => write!(
                out,
                "server `{id}` has already ended, so there is nothing to stop"
            ),
            NotKept { why } => write!(out, "the server could not be kept: {why}"),
        }
    }
}

/// What the list above is worth, in a clause. **Naming the read rather than
/// only the list**, because a Fleet that has been up for days is answering
/// from what it resolved at startup and the list alone does not say so — the
/// twelve-day Fleet in `#1564` refused a server the repository declared and
/// named four that it did not.
fn since(last_read: Option<&LastRead>) -> String {
    let Some(last_read) = last_read else {
        return String::new();
    };
    let at = &last_read.at;
    match last_read
        .at_restart
        .iter()
        .any(|section| section == COMMANDS)
    {
        true => format!(
            ". This Fleet read `armada.yml` at {at} and `{COMMANDS}` had changed, which it \
             cannot take up while it runs — restart Fleet and ask again"
        ),
        false => format!(". This Fleet last read `armada.yml` at {at}"),
    }
}

impl std::error::Error for Unservable {}
