//! What the binary proves about itself without starting anything.
//!
//! **No Fleet is started here, and no port is taken.** Every verb but `serve`
//! is a read, a resolve and a filesystem act, and all three are testable
//! without a daemon — which is why they are separate modules rather than
//! branches inside `serve`.
//!
//! `setup` and `agent` read this repository's own files and this machine's own
//! `PATH`. `cli`, `declared` and `clean` build the world they act on.

mod agent;
mod clean;
mod cli;
mod declared;
mod setup;
mod watching;

use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicU64, Ordering};

static NEXT: AtomicU64 = AtomicU64::new(0);

/// The workspace root, from the crate that is being compiled.
///
/// Derived rather than searched for: `CARGO_MANIFEST_DIR` is set by cargo and
/// is exact, where a walk upward for an `armada.yml` would find whichever one
/// came first and pass on a machine that had one somewhere else.
pub fn repository() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR"))
        .parent()
        .and_then(Path::parent)
        .expect("crates/armada sits two directories below the workspace root")
        .to_path_buf()
}

/// A directory that exists for one test and is gone after it.
pub struct TempDir {
    path: PathBuf,
}

impl TempDir {
    pub fn new() -> TempDir {
        let path = std::env::temp_dir().join(format!(
            "armada-cli-{}-{}",
            std::process::id(),
            NEXT.fetch_add(1, Ordering::Relaxed)
        ));
        std::fs::create_dir_all(&path).expect("a temporary directory");
        TempDir {
            path: path.canonicalize().expect("a real path"),
        }
    }

    pub fn path(&self) -> &Path {
        &self.path
    }

    pub fn write(&self, relative: &str, contents: &str) {
        let at = self.path.join(relative);
        if let Some(parent) = at.parent() {
            std::fs::create_dir_all(parent).expect("the parent directory");
        }
        std::fs::write(at, contents).expect("the file");
    }
}

impl Drop for TempDir {
    fn drop(&mut self) {
        let _ = std::fs::remove_dir_all(&self.path);
    }
}
