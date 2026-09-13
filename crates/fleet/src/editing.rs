//! The Manifest file, read and written — Journey 9, *Editing*.
//!
//! **Fleet owns the write, and no caller names the file.** Fleet resolved
//! `armada.yml`, watches it and re-reads it; a second writer working from a
//! path it composed itself would be writing to a file nobody agreed on.
//!
//! **Save stops at the bytes.** No staging, no commit, no formatting, no
//! reserialisation. What Fleet does next is what it already does on any edit:
//! `crates/armada/src/watching.rs` settles, re-reads, and reports every fault.

use std::fs;
use std::io;
use std::path::{Path, PathBuf};

use adapter_traits::{AgentHarness, Delivery, Vcs, WorkProduct};
use api::Refusal;
use ipc::{Instant, ManifestFile, ManifestSaved, SaveManifestFile};

use crate::adrift::Adrift;
use crate::daemon::Fleet;

/// Where the new bytes are assembled before they replace the file.
///
/// A leading dot and the file's own name, so it sorts out of sight and cannot
/// collide with anything the repository tracks.
fn beside(file: &Path) -> PathBuf {
    let name = file
        .file_name()
        .map(|name| name.to_string_lossy().to_string())
        .unwrap_or_else(|| "armada.yml".to_string());
    file.with_file_name(format!(".{name}.saving"))
}

/// Why a save did not happen.
///
/// **Three outcomes and not two.** A file that moved under the edit is not a
/// failure — nothing broke, and what a person does next is reconcile rather
/// than retry.
#[derive(Debug)]
pub(crate) enum NotSaved {
    /// The file would not open to be compared against.
    Unreadable(io::Error),
    /// It changed after the edit started, and this is what is there now —
    /// **absent where it is no longer there at all**, which is a different
    /// thing to do about and so a different thing to say.
    Moved(Option<String>),
    /// The bytes would not go down.
    Unwritable(io::Error),
}

/// Put `text` where `file` is — **only where `read` is still what is there**.
///
/// **There is no unguarded write in this crate**, which is why the comparison
/// is inside this function rather than in front of it. A save that could skip
/// the check is one somebody eventually calls.
///
/// **The disk is read, never the watcher's last reading.** That reading can be
/// older than the file by up to the settle window, so a guard consulting it
/// would wave through exactly the save this exists to stop.
///
/// **Not a truncate-and-write.** That leaves a half-written `armada.yml` behind
/// a crash, in the one file a person is least able to afford one. The watcher
/// re-resolves the path each round, so a rename over the target is one change
/// like any other — `crates/armada/src/watching.rs`, `SETTLE`.
pub(crate) fn save(file: &Path, read: &str, text: &str) -> Result<(), NotSaved> {
    // **The window between this read and the rename below is microseconds**,
    // against the minutes a person spends editing. Closing it would mean a lock
    // on a repository file held across a request, which is a worse thing to
    // leave behind than the race it removes.
    match fs::read_to_string(file) {
        Ok(on_disk) if on_disk == read => {}
        Ok(on_disk) => return Err(NotSaved::Moved(Some(on_disk))),
        Err(why) if why.kind() == io::ErrorKind::NotFound => return Err(NotSaved::Moved(None)),
        Err(why) => return Err(NotSaved::Unreadable(why)),
    }
    write(file, text).map_err(NotSaved::Unwritable)
}

/// Why a create did not happen.
#[derive(Debug)]
pub(crate) enum NotCreated {
    /// Something is at the path already, with what it holds where it reads.
    Appeared(Option<String>),
    Unwritable(io::Error),
}

/// Put `text` at `file` only where nothing is there. A hard link, not a rename, so a file
/// that appeared is refused in the same call that would make the name.
pub(crate) fn create(file: &Path, text: &str) -> Result<(), NotCreated> {
    let staged = beside(file);
    let linked = fs::write(&staged, text).and_then(|()| fs::hard_link(&staged, file));
    let _ = fs::remove_file(&staged);
    match linked {
        Ok(()) => Ok(()),
        Err(why) if why.kind() == io::ErrorKind::AlreadyExists => {
            Err(NotCreated::Appeared(fs::read_to_string(file).ok()))
        }
        Err(why) => Err(NotCreated::Unwritable(why)),
    }
}

/// The replace itself. The mode is carried across, so a save does not quietly
/// reset what somebody set on the file.
fn write(file: &Path, text: &str) -> Result<(), io::Error> {
    let staged = beside(file);
    let written = fs::write(&staged, text)
        .and_then(|()| match fs::metadata(file) {
            Ok(was) => fs::set_permissions(&staged, was.permissions()),
            Err(_) => Ok(()),
        })
        .and_then(|()| fs::rename(&staged, file));
    if written.is_err() {
        // **Left behind it would be one more file in the repository root.** The
        // name is dotted and untracked, so a stray one is small and silent —
        // which is exactly why it is swept here rather than noticed later.
        let _ = fs::remove_file(&staged);
    }
    written
}

impl<H, V, W> Fleet<H, V, W>
where
    H: AgentHarness + Send + Sync + 'static,
    H::Error: std::error::Error + Send + Sync + 'static,
    V: Vcs + Delivery + Send + Sync + 'static,
    V::Error: std::error::Error + Send + Sync + 'static,
    V::CommitError: std::error::Error + Send + Sync + 'static,
    W: WorkProduct + Send + Sync + 'static,
    W::Error: std::error::Error + Send + Sync + 'static,
{
    /// `get_manifest_file` — the file whole, unparsed.
    ///
    /// **The path is spelled as `ManifestReading` spells it** and never
    /// canonicalised, so the two answers a surface draws together agree about
    /// which file they are about.
    pub(crate) fn read_manifest_file(&self) -> Result<ManifestFile, Refusal> {
        let file = self.manifest().path();
        let text = fs::read_to_string(file).map_err(|cause| {
            self.refusal(Adrift::ManifestUnreadable {
                path: file.display().to_string(),
                cause,
            })
        })?;
        Ok(ManifestFile {
            path: file.display().to_string(),
            text,
        })
    }

    /// `save_manifest_file` — the bytes the caller sent, on disk.
    ///
    /// **Nothing about the *text* can refuse this.** A person correcting a file
    /// gets it wrong on the way, and a save that parsed first would leave them
    /// unable to put down work in progress.
    ///
    /// **What the file was when the edit started can.** `armada.yml` is
    /// tracked, so a save that took an incoming `git checkout` with it would
    /// destroy a committed edit with nothing said.
    pub(crate) fn write_manifest_file(
        &self,
        asked: SaveManifestFile,
    ) -> Result<ManifestSaved, Refusal> {
        let file = self.manifest().path();
        let path = file.display().to_string();
        save(file, &asked.read, &asked.text).map_err(|why| {
            self.refusal(match why {
                NotSaved::Unreadable(cause) => Adrift::ManifestUnreadable { path, cause },
                NotSaved::Moved(on_disk) => Adrift::ManifestMovedUnderTheEdit {
                    path: file.display().to_string(),
                    on_disk,
                },
                NotSaved::Unwritable(cause) => Adrift::ManifestUnwritable { path, cause },
            })
        })?;
        Ok(ManifestSaved {
            path: file.display().to_string(),
            at: Instant::from(&self.now()),
        })
    }
}
