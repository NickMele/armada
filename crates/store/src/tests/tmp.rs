//! A directory that exists for one test and is gone after it.
//!
//! # Why not an in-memory database
//!
//! Because the thing being proved is that a Job survives the process that made
//! it. An in-memory store cannot be closed and reopened, cannot be in WAL mode,
//! and would let every test in this crate pass without touching the property
//! the milestone is about. `Store` offers no in-memory constructor for the same
//! reason; this is what stands in for one.

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
            "armada-store-{}-{}",
            std::process::id(),
            NEXT.fetch_add(1, Ordering::Relaxed)
        ));
        std::fs::create_dir_all(&path).expect("a temporary directory");
        TempDir { path }
    }

    /// Where the store under test lives.
    pub fn db(&self) -> PathBuf {
        self.path.join("armada.sqlite")
    }
}

impl Drop for TempDir {
    fn drop(&mut self) {
        let _ = std::fs::remove_dir_all(&self.path);
    }
}
