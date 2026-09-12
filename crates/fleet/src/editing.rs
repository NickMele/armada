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

/// Put `text` where `file` is, by writing beside it and renaming over.
///
/// **Not a truncate-and-write.** That leaves a half-written `armada.yml` behind
/// a crash, in the one file a person is least able to afford one. The watcher's
/// own measurement is what makes the replace safe to prefer: it polls and
/// re-resolves the path each round, so a rename over the target is one change
/// like any other — `crates/armada/src/watching.rs`, `SETTLE`.
///
/// The mode is carried across where the file is already there, so a save does
/// not quietly reset what somebody set on it.
pub(crate) fn save(file: &Path, text: &str) -> Result<(), io::Error> {
    let staged = beside(file);
    let write = fs::write(&staged, text)
        .and_then(|()| match fs::metadata(file) {
            Ok(was) => fs::set_permissions(&staged, was.permissions()),
            // Nothing to carry across: the file is not there, which is a save
            // over a Manifest somebody deleted and still a save.
            Err(_) => Ok(()),
        })
        .and_then(|()| fs::rename(&staged, file));
    if write.is_err() {
        // **Left behind it would be read as a Manifest.** The name is dotted
        // and untracked, so the cost of a stray one is small and silent, which
        // is exactly why it is swept here rather than noticed later.
        let _ = fs::remove_file(&staged);
    }
    write
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
    /// **Nothing about the text can refuse this.** A person correcting a file
    /// gets it wrong on the way, and a save that parsed first would leave them
    /// unable to put down work in progress.
    pub(crate) fn write_manifest_file(
        &self,
        asked: SaveManifestFile,
    ) -> Result<ManifestSaved, Refusal> {
        let file = self.manifest().path();
        save(file, &asked.text).map_err(|cause| {
            self.refusal(Adrift::ManifestUnwritable {
                path: file.display().to_string(),
                cause,
            })
        })?;
        Ok(ManifestSaved {
            path: file.display().to_string(),
            at: Instant::from(&self.now()),
        })
    }
}
