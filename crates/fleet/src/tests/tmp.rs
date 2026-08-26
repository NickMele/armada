//! A directory that exists for one test and is gone after it.
//!
//! The runtime file's whole subject is a real path with a real file at it, so
//! nothing here is faked in memory: a test that never wrote a file could not
//! tell a removal-on-drop from a removal that never happened.

use std::path::PathBuf;
use std::sync::atomic::{AtomicU64, Ordering};

static NEXT: AtomicU64 = AtomicU64::new(0);

/// A directory under the system temp dir, removed on drop.
pub struct TempDir {
    path: PathBuf,
}

impl TempDir {
    pub fn new() -> TempDir {
        let path = std::env::temp_dir().join(format!(
            "armada-fleet-{}-{}",
            std::process::id(),
            NEXT.fetch_add(1, Ordering::Relaxed)
        ));
        std::fs::create_dir_all(&path).expect("a temporary directory");
        TempDir { path }
    }

    /// Where the runtime file under test lives.
    pub fn runtime_file(&self) -> PathBuf {
        self.path.join(crate::runtime::FILE_NAME)
    }
}

impl Drop for TempDir {
    fn drop(&mut self) {
        let _ = std::fs::remove_dir_all(&self.path);
    }
}
