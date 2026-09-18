//! Reading and writing one state file. Every typed file under the state
//! directory — a queue entry, an outcome, a preflight stamp — goes through
//! these two functions and nothing else, so there is exactly one place that
//! turns a state file's bytes into a value or back.

use std::fs;
use std::io;
use std::path::{Path, PathBuf};

use serde::de::DeserializeOwned;
use serde::Serialize;

/// A state file that could not be answered.
///
/// **Not found is not this.** A missing file is "nothing known yet", which
/// [`read`] answers with `Ok(None)` — the same distinction
/// `scripts/land`'s `read_json` draws by returning `None` on any `OSError`,
/// narrowed here so a permission fault is still visible to the caller.
#[derive(Debug)]
pub enum ReadStateError {
    Unreadable {
        path: PathBuf,
        cause: io::Error,
    },
    Undecodable {
        path: PathBuf,
        cause: ipc::Undecodable,
    },
}

impl std::fmt::Display for ReadStateError {
    fn fmt(&self, out: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            ReadStateError::Unreadable { path, .. } => {
                write!(out, "{} could not be read", path.display())
            }
            ReadStateError::Undecodable { path, .. } => {
                write!(
                    out,
                    "{} is not a state file armada land wrote",
                    path.display()
                )
            }
        }
    }
}

impl std::error::Error for ReadStateError {
    fn source(&self) -> Option<&(dyn std::error::Error + 'static)> {
        match self {
            ReadStateError::Unreadable { cause, .. } => Some(cause),
            ReadStateError::Undecodable { cause, .. } => Some(cause),
        }
    }
}

/// A state file that could not be written.
#[derive(Debug)]
pub enum WriteStateError {
    Unencodable(ipc::Unencodable),
    Unwritable { path: PathBuf, cause: io::Error },
}

impl std::fmt::Display for WriteStateError {
    fn fmt(&self, out: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            WriteStateError::Unencodable(why) => write!(out, "{why}"),
            WriteStateError::Unwritable { path, .. } => {
                write!(out, "{} could not be written", path.display())
            }
        }
    }
}

impl std::error::Error for WriteStateError {
    fn source(&self) -> Option<&(dyn std::error::Error + 'static)> {
        match self {
            WriteStateError::Unencodable(why) => Some(why),
            WriteStateError::Unwritable { cause, .. } => Some(cause),
        }
    }
}

/// Read one state file, or `None` where nothing is there yet.
pub fn read<T: DeserializeOwned>(
    expected: &'static str,
    path: &Path,
) -> Result<Option<T>, ReadStateError> {
    let bytes = match fs::read(path) {
        Ok(bytes) => bytes,
        Err(cause) if cause.kind() == io::ErrorKind::NotFound => return Ok(None),
        Err(cause) => {
            return Err(ReadStateError::Unreadable {
                path: path.to_path_buf(),
                cause,
            })
        }
    };
    ipc::decode(expected, &bytes)
        .map(Some)
        .map_err(|cause| ReadStateError::Undecodable {
            path: path.to_path_buf(),
            cause,
        })
}

/// Write one state file whole. A sibling holds the bytes until they are
/// complete, then a rename swaps it in — matching `scripts/land`'s
/// `write_json`, so a reader never sees half a file.
pub fn write<T: Serialize>(path: &Path, value: &T) -> Result<(), WriteStateError> {
    let body = ipc::encode(value).map_err(WriteStateError::Unencodable)?;
    let staging = staging_path(path);
    fs::write(&staging, body).map_err(|cause| WriteStateError::Unwritable {
        path: staging.clone(),
        cause,
    })?;
    fs::rename(&staging, path).map_err(|cause| WriteStateError::Unwritable {
        path: path.to_path_buf(),
        cause,
    })
}

/// `{path}.{pid}.tmp`, a sibling of `path` so the rename stays on one
/// filesystem — the same name `scripts/land`'s `write_json` gives it.
fn staging_path(path: &Path) -> PathBuf {
    let mut name = path.file_name().unwrap_or_default().to_os_string();
    name.push(format!(".{}.tmp", std::process::id()));
    path.with_file_name(name)
}
