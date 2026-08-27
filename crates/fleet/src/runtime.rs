//! The runtime file: how Bridge finds Fleet, and how it knows the Fleet it
//! found is the one the file names.
//!
//! Written once the listener is bound, removed on a clean exit, left behind on
//! an unclean one. Four fields — protocol version, pid, port, and the start
//! time that makes the pid mean something.
//!
//! # Three answers, because Bridge renders three different things
//!
//! `../../docs/contracts/design-system.md` gives Fleet's status bar three
//! states, and two of them are told apart by this file rather than by a
//! connection attempt. So [`read`] never collapses them:
//!
//! | [`Presence`] | What is true | What Bridge says |
//! |---|---|---|
//! | `NotRunning` | No file at the path | Fleet is not running |
//! | `Stale(PidDead)` | A file, naming a pid nothing holds | Fleet is not running |
//! | `Stale(PidHeldByAnother)` | A file, naming a pid *something else* holds | Fleet is not running — and do not connect |
//! | `Running` | A file whose pid is held by the process that wrote it | Connect. Silence past here is *unreachable*, not *down* |
//!
//! The third row is the one v1 could not produce. Its check proved a pid was
//! held; being held by the wrong process reads identically to being held by
//! the right one, and the consequence is a client opening a socket against a
//! port some unrelated program now owns.
//!
//! # The host is not in the file
//!
//! Only the port is written. `127.0.0.1` is a constant here and never data, so
//! there is no field an edited file could put `0.0.0.0` into and no code path
//! that reads a host from disk. Loopback is structural rather than configured.
//!
//! # Publishing needs proof the path is free
//!
//! [`RuntimeFile::publish`] takes a [`Vacancy`], and the only thing that mints
//! one is a [`read`] that found no live Fleet. A second Fleet cannot overwrite
//! a first's runtime file, because the call that would do it cannot be spelled
//! without the token, and the token does not exist while the first is alive.
//!
//! # A file that will not parse is refused, not ignored
//!
//! v1 read a corrupt pidfile as "not running" — fail-safe, and correct there,
//! because a torn file was a plausible crash artifact. It is not one here: the
//! file is written to a sibling and renamed, so a reader sees the whole of one
//! version or the whole of the previous one, never half of either. A file that
//! does not parse was therefore written by something that is not Fleet, and
//! reading that as "nothing is running" is how a second Fleet gets started over
//! a live one.

use std::fs;
use std::io;
use std::net::{IpAddr, Ipv4Addr, SocketAddr};
use std::path::{Path, PathBuf};

use serde::{Deserialize, Serialize};

use ipc::ProtocolVersion;

use crate::process::{holder_of, Holder, ProbeFailed, StartedAt};

/// The address Fleet binds. **Provisional**: hardcoded for M1, and the number
/// is not owned by anything yet.
///
/// Loopback, never `0.0.0.0`. Fleet answers commands that spawn processes
/// against a real repository, so a routable bind is a remote code execution
/// surface rather than a convenience, and the constant is the only place the
/// host is expressible.
pub const PROVISIONAL_PORT: u16 = 47821;

/// The bind address. Assembled here from [`PROVISIONAL_PORT`] so no caller
/// writes a host of its own.
pub fn provisional_address() -> SocketAddr {
    SocketAddr::new(IpAddr::V4(Ipv4Addr::LOCALHOST), PROVISIONAL_PORT)
}

/// The file name under the machine directory.
///
/// Not `fleet.pid`: a `.pid` file is a pid and a newline, and something will
/// eventually `cat` this one expecting that. It carries four fields.
pub const FILE_NAME: &str = "fleet.json";

/// What Fleet published about itself.
///
/// Unknown fields are ignored on read, deliberately — this file is a contract
/// between two independently versioned binaries, and an older reader that
/// refuses a field it has not heard of turns every additive change into a
/// breaking one. The same discipline the wire DTOs are held to.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct RuntimeFile {
    /// What Fleet speaks, both numbers. Read before connecting, so a refusal
    /// is a screen naming two versions rather than a malformed first message —
    /// and so a minor gap Bridge can survive is a banner instead of one.
    pub protocol_version: ProtocolVersion,
    pub pid: u32,
    /// On `127.0.0.1`, always. See the module comment.
    pub port: u16,
    /// When the process at `pid` started, as the OS reported it at the moment
    /// this file was written. **This is the field that makes `pid` a claim
    /// about a particular process rather than about a number.**
    pub started_at: StartedAt,
}

/// What is at the runtime file's path.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum Presence {
    /// No file. Fleet has never started here, or it exited cleanly.
    NotRunning,
    /// A file, and the process it names is not the one that wrote it.
    Stale { found: RuntimeFile, why: Staleness },
    /// A file whose pid is held by the same process that wrote it. The port is
    /// worth connecting to; whether anything answers is the next question.
    Running(RuntimeFile),
}

/// Why a runtime file does not describe a live Fleet.
///
/// Two variants because they are two different events, even though Bridge
/// renders both as "not running": one is an ordinary crash, and the other means
/// an unrelated process is sitting on the pid Fleet used to hold.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum Staleness {
    /// Nothing holds the pid. Fleet died without cleaning up.
    PidDead,
    /// Something holds the pid and it started at a different time, so it is not
    /// the process that wrote this file. **Connecting to the port anyway is the
    /// failure this field exists to prevent.**
    PidHeldByAnother { holder: StartedAt },
}

impl Presence {
    /// Proof the path may be written, if this answer is one that permits it.
    ///
    /// `None` for [`Presence::Running`], which is the whole point: there is no
    /// way to obtain the token while a live Fleet holds the file.
    pub fn vacancy(self, path: &Path) -> Option<Vacancy> {
        let path = path.to_path_buf();
        match self {
            Presence::Running(_) => None,
            Presence::NotRunning => Some(Vacancy {
                path,
                replacing: None,
            }),
            Presence::Stale { why, .. } => Some(Vacancy {
                path,
                replacing: Some(why),
            }),
        }
    }
}

/// Proof that no live Fleet holds a particular runtime file path.
///
/// Minted only by [`Presence::vacancy`], and it carries the path it was minted
/// for — so a publish cannot check one path and write another. It also carries
/// what it is replacing, so the composition root can say which of the two stale
/// cases it found, which is a fact worth an audit line.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Vacancy {
    path: PathBuf,
    replacing: Option<Staleness>,
}

impl Vacancy {
    /// The stale file this publish will overwrite, if there was one.
    pub fn replacing(&self) -> Option<&Staleness> {
        self.replacing.as_ref()
    }

    pub fn path(&self) -> &Path {
        &self.path
    }
}

/// Read the runtime file and decide what it describes.
///
/// The pid probe is part of reading. A caller cannot get the file's contents
/// without the verdict on them, so there is no shape here through which a pid
/// reaches a connection attempt unchecked.
pub fn read(path: &Path) -> Result<Presence, ReadError> {
    let bytes = match fs::read(path) {
        Ok(bytes) => bytes,
        Err(cause) if cause.kind() == io::ErrorKind::NotFound => return Ok(Presence::NotRunning),
        Err(cause) => {
            return Err(ReadError::Unreadable {
                path: path.to_path_buf(),
                cause,
            })
        }
    };

    // `ipc::decode` rather than a parser here: gate rule five scopes untyped
    // JSON to the two crates where bytes enter the process, and this is one of
    // those doorways wearing a file instead of a socket.
    let found: RuntimeFile =
        ipc::decode("runtime file", &bytes).map_err(|cause| ReadError::Undecodable {
            path: path.to_path_buf(),
            cause,
        })?;

    let holder = holder_of(found.pid).map_err(ReadError::ProbeFailed)?;
    Ok(match holder {
        Holder::Vacant => Presence::Stale {
            found,
            why: Staleness::PidDead,
        },
        Holder::Held(started_at) if started_at == found.started_at => Presence::Running(found),
        Holder::Held(holder) => Presence::Stale {
            found,
            why: Staleness::PidHeldByAnother { holder },
        },
    })
}

impl RuntimeFile {
    /// Publish this process, at a port that is already bound.
    ///
    /// Takes the port rather than choosing it, because the listener is bound
    /// first and its own address is read back: publishing a port nothing is
    /// listening on is the one thing this file must never do.
    pub fn publish(
        vacancy: Vacancy,
        port: u16,
        protocol_version: ProtocolVersion,
    ) -> Result<Published, PublishError> {
        let path = vacancy.path;
        let pid = std::process::id();
        let started_at = match holder_of(pid).map_err(PublishError::ProbeFailed)? {
            Holder::Held(started_at) => started_at,
            // Unreachable: the process asking is the process asked about.
            Holder::Vacant => return Err(PublishError::OwnPidNotHeld { pid }),
        };

        let file = RuntimeFile {
            protocol_version,
            pid,
            port,
            started_at,
        };
        let body = ipc::encode(&file).map_err(|cause| PublishError::Unencodable {
            cause: cause.to_string(),
        })?;

        if let Some(parent) = path.parent().filter(|p| !p.as_os_str().is_empty()) {
            fs::create_dir_all(parent).map_err(|cause| PublishError::Unwritable {
                path: parent.to_path_buf(),
                cause,
            })?;
        }

        // Whole file or nothing. A reader that catches Fleet mid-write sees the
        // previous version, never half of this one — which is what makes an
        // unparseable file mean "not written by Fleet" rather than "written by
        // a Fleet that died at the wrong moment".
        let staging = staging_path(&path);
        fs::write(&staging, body).map_err(|cause| PublishError::Unwritable {
            path: staging.clone(),
            cause,
        })?;
        fs::rename(&staging, &path).map_err(|cause| PublishError::Unwritable {
            path: path.clone(),
            cause,
        })?;

        Ok(Published { path, file })
    }
}

/// Where the file is assembled before it is renamed into place. A sibling, so
/// the rename stays on one filesystem and stays atomic.
fn staging_path(path: &Path) -> PathBuf {
    let mut name = path
        .file_name()
        .unwrap_or(FILE_NAME.as_ref())
        .to_os_string();
    name.push(".writing");
    path.with_file_name(name)
}

/// A published runtime file, held for the life of the process.
///
/// **Removal is [`Drop`], not a call.** A clean exit is any path that unwinds
/// or returns through here, including one nobody thought about, and an exit
/// that skips `Drop` — `SIGKILL`, a power cut — is exactly the unclean exit
/// that is supposed to leave the file behind. The two cases the step names are
/// therefore the two cases Rust already distinguishes, and there is no third
/// path where somebody forgot to call the cleanup.
#[derive(Debug)]
pub struct Published {
    path: PathBuf,
    file: RuntimeFile,
}

impl Published {
    pub fn path(&self) -> &Path {
        &self.path
    }

    pub fn file(&self) -> &RuntimeFile {
        &self.file
    }
}

impl Drop for Published {
    fn drop(&mut self) {
        // A removal that fails leaves a file whose pid is about to be dead,
        // which the next read calls stale and the next start replaces. There is
        // nothing better to do from a destructor, and nothing worse happens.
        let _ = fs::remove_file(&self.path);
    }
}

/// Where machine-level state lives, and the runtime file with it.
///
/// Application Support because Armada is a desktop application rather than a
/// command-line tool — `../../docs/contracts/system-architecture.md` settles
/// that, and `~/.armada/` was rejected on it.
pub fn machine_path() -> Result<PathBuf, NoHome> {
    let home = std::env::var_os("HOME").ok_or(NoHome)?;
    Ok(PathBuf::from(home)
        .join("Library")
        .join("Application Support")
        .join("Armada")
        .join(FILE_NAME))
}

/// `HOME` is unset, so there is no machine directory to resolve.
#[derive(Debug, PartialEq, Eq)]
pub struct NoHome;

impl std::fmt::Display for NoHome {
    fn fmt(&self, out: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        out.write_str("HOME is not set, so the machine directory cannot be resolved")
    }
}

impl std::error::Error for NoHome {}

/// Why the runtime file could not be read.
///
/// **None of these is "not running".** That answer is a [`Presence`], because
/// it is a fact about the world rather than a failure to establish one, and
/// folding a failed read into it tells a caller Fleet is down on no evidence.
#[derive(Debug)]
pub enum ReadError {
    Unreadable {
        path: PathBuf,
        cause: io::Error,
    },
    Undecodable {
        path: PathBuf,
        cause: ipc::Undecodable,
    },
    ProbeFailed(ProbeFailed),
}

impl std::fmt::Display for ReadError {
    fn fmt(&self, out: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            ReadError::Unreadable { path, .. } => {
                write!(
                    out,
                    "the runtime file at {} could not be read",
                    path.display()
                )
            }
            ReadError::Undecodable { path, .. } => {
                write!(out, "{} is not a runtime file Armada wrote", path.display())
            }
            ReadError::ProbeFailed(_) => {
                out.write_str("the process named by the runtime file could not be checked")
            }
        }
    }
}

impl std::error::Error for ReadError {
    fn source(&self) -> Option<&(dyn std::error::Error + 'static)> {
        match self {
            ReadError::Unreadable { cause, .. } => Some(cause),
            ReadError::Undecodable { cause, .. } => Some(cause),
            ReadError::ProbeFailed(cause) => Some(cause),
        }
    }
}

/// Why the runtime file could not be published.
#[derive(Debug)]
pub enum PublishError {
    Unwritable {
        path: PathBuf,
        cause: io::Error,
    },
    ProbeFailed(ProbeFailed),
    /// The running process cannot see itself. Not reachable in practice, and
    /// named rather than unwrapped because a panic here is a daemon that dies
    /// at startup with no line saying why.
    OwnPidNotHeld {
        pid: u32,
    },
    /// Four scalars would not serialise. Carried as text because there is no
    /// typed fault under it worth reconstructing.
    Unencodable {
        cause: String,
    },
}

impl std::fmt::Display for PublishError {
    fn fmt(&self, out: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            PublishError::Unwritable { path, .. } => {
                write!(out, "{} could not be written", path.display())
            }
            PublishError::ProbeFailed(_) => {
                out.write_str("this process's own start time could not be read")
            }
            PublishError::OwnPidNotHeld { pid } => {
                write!(out, "this process reports pid {pid}, which nothing holds")
            }
            PublishError::Unencodable { cause } => {
                write!(out, "the runtime file would not serialise: {cause}")
            }
        }
    }
}

impl std::error::Error for PublishError {
    fn source(&self) -> Option<&(dyn std::error::Error + 'static)> {
        match self {
            PublishError::Unwritable { cause, .. } => Some(cause),
            PublishError::ProbeFailed(cause) => Some(cause),
            PublishError::OwnPidNotHeld { .. } | PublishError::Unencodable { .. } => None,
        }
    }
}
