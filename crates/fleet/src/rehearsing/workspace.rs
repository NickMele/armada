//! Which `armada.yml` a Verify runs: the root's, which Fleet already holds, or
//! one workspace's below it, read from disk when Verify is pressed.
//!
//! **The checkout is still the owner.** A workspace's file sits in the same
//! tree and build directory as the root's, so a workspace Verify holds the one
//! slot keyed by the repository root — only the file and the directory differ.

use std::path::{Component, Path, PathBuf};

use config::Manifest;
use ipc::{ManifestFault, ManifestRefused};

use super::Unrehearsable;

/// A workspace's own Manifest, and the directory its commands run in.
pub(crate) struct Workspace {
    /// Relative to the repository root, `/`-separated, never empty or `.`.
    pub(crate) dir: String,
    pub(crate) manifest: Manifest,
}

/// `asked` resolved against `root`. `None` is the root's Manifest, which
/// absent, empty and `.` all name.
pub(crate) fn resolved(
    root: &str,
    asked: Option<&str>,
) -> Result<Option<Workspace>, Unrehearsable> {
    let Some(asked) = asked else {
        return Ok(None);
    };
    let outside = || Unrehearsable::WorkspaceOutside {
        dir: asked.to_string(),
    };
    let mut parts = Vec::new();
    for part in Path::new(asked).components() {
        match part {
            Component::Normal(name) => parts.push(name.to_str().ok_or_else(outside)?),
            Component::CurDir => {}
            Component::ParentDir | Component::RootDir | Component::Prefix(_) => {
                return Err(outside())
            }
        }
    }
    if parts.is_empty() {
        return Ok(None);
    }
    let dir = parts.join("/");
    let at = PathBuf::from(root).join(&dir);
    // A symlink below the root can still lead out of the checkout.
    if let (Ok(real), Ok(real_root)) = (at.canonicalize(), Path::new(root).canonicalize()) {
        if !real.starts_with(&real_root) {
            return Err(outside());
        }
    }
    let manifest =
        Manifest::load(&at.join("armada.yml")).map_err(|why| Unrehearsable::WorkspaceManifest {
            file: format!("{dir}/armada.yml"),
            refused: ManifestRefused {
                summary: why.to_string(),
                faults: why
                    .refusals()
                    .iter()
                    .map(|one| ManifestFault {
                        key: one.key.clone(),
                        fault: one.fault.to_string(),
                    })
                    .collect(),
            },
        })?;
    Ok(Some(Workspace { dir, manifest }))
}
