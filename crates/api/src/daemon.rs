//! What `api` needs from the daemon, and the refusals it may answer with.
//!
//! # These traits are the reason `api` does not depend on `fleet`
//!
//! They are stated here, where the transport is, and implemented over there,
//! where the Jobs are. `cargo tree -p api` names no `fleet`, and the daemon
//! core stays drivable in a test with **no socket, no port and no process** —
//! a fake implements the three and the same router serves it.
//!
//! # It speaks DTOs, not Jobs
//!
//! Every signature in the three modules below is `ipc` vocabulary. That is
//! where the redaction sits: Fleet converts at this boundary, so a field added
//! to `core_model::Job` reaches the wire only when somebody writes the line
//! that puts it there. `api` never sees a domain type, and nothing in this
//! crate can leak one.
//!
//! # Three, and the line is not a line count
//!
//! A query decodes no body, answers no 201 and has no refusal meaning the
//! machine would not admit a move; a command is the opposite of all three; a
//! tool's caller is a Drone rather than Bridge and its refusal carries no
//! status code at all. `crate::queries`, `crate::commands` and `crate::mcp`
//! drew that line through the transport, and the trait was the one place in
//! this crate that did not follow it. Each module below states its own half.

mod commands;
mod queries;
mod tools;

pub use commands::Commands;
pub use queries::Queries;
pub use tools::Tools;

use ipc::WireError;

/// A daemon that answers all three surfaces.
///
/// **The name of the whole seam, and it has no method of its own.** It is what
/// [`router`](crate::router) and the composition root take, because a process
/// serving the listener holds something that must answer everything; a handler
/// takes the one surface it uses instead, so a read that reached for a command
/// would not compile.
///
/// **One spelling, not two.** The blanket implementation below means a type
/// implements the three and gets this free — `impl Daemon for` anything is not
/// something that compiles, so which of the two to write cannot be asked.
///
/// # Why events are not a fourth
///
/// They would have no method to declare. Fleet *publishes* into a
/// [`Broadcaster`](crate::Broadcaster) this crate hands it, so that call runs
/// from the daemon into the transport rather than the other way, which is the
/// only direction a trait here describes. What the socket asks the daemon for
/// is `list_jobs`, for the resync it opens with, and that is an ordinary read
/// behind a [`Queries`] bound; `get_job_events` is a history read and is a
/// query for the same reason.
pub trait Daemon: Queries + Commands + Tools {}

impl<D: Queries + Commands + Tools> Daemon for D {}

/// A request the daemon would not serve.
///
/// Four variants because a caller has four different things to do about them,
/// and "the request failed" is not an answer anybody can act on. The status is
/// derived here rather than carried on [`WireError`]: an HTTP code is the
/// transport's, and putting one on the wire type would give the lifeboat's job
/// to the protocol.
#[derive(Clone, Debug, PartialEq)]
pub enum Refusal {
    /// No Job by that id. The id was invented, or the Job was retained out.
    NoSuchJob(WireError),
    /// The Job exists and the machine does not admit the move — approving one
    /// already running, killing one already terminal.
    IllegalMove(WireError),
    /// **The request decoded and names something that cannot work.** A proposal
    /// naming a workflow or a Manifest Fleet does not hold, or carrying a blank
    /// model where nothing configured supplies one.
    ///
    /// The fourth variant, added because those three had nowhere honest to go.
    /// A 400 belongs to the transport and means the bytes did not become a
    /// request; a 500 says the daemon broke, which sends the caller to retry
    /// something that will fail identically forever. This says: the request is
    /// well-formed, the values in it are not, and the message names them.
    Unacceptable(WireError),
    /// Something under the daemon failed. Not the caller's doing, and retrying
    /// the same request is reasonable.
    Fault(WireError),
}

impl Refusal {
    /// The HTTP status this answers with.
    pub fn status(&self) -> u16 {
        match self {
            Refusal::NoSuchJob(_) => 404,
            Refusal::IllegalMove(_) => 409,
            Refusal::Unacceptable(_) => 422,
            Refusal::Fault(_) => 500,
        }
    }

    pub fn error(&self) -> &WireError {
        match self {
            Refusal::NoSuchJob(error)
            | Refusal::IllegalMove(error)
            | Refusal::Unacceptable(error)
            | Refusal::Fault(error) => error,
        }
    }
}
