//! One Fleet serving several repositories: adding one by folder, every
//! Manifest listed, a Job created in each against its own Checks, and a restart
//! that reconciles both.

use std::collections::BTreeMap;
use std::path::{Path, PathBuf};
use std::sync::{Mutex, PoisonError};

use crate::repositories::{Located, Locating, NotLocated};

/// Folders a case planted, read the way the composition root would read them.
#[derive(Default)]
pub struct Planted {
    folders: Mutex<BTreeMap<PathBuf, Located>>,
    served: Mutex<Vec<String>>,
}

impl Planted {
    pub fn nothing() -> Planted {
        Planted::default()
    }

    pub fn with(self, folder: impl Into<PathBuf>, located: Located) -> Planted {
        self.folders
            .lock()
            .unwrap_or_else(PoisonError::into_inner)
            .insert(folder.into(), located);
        self
    }

    /// Every root Fleet said it now serves, in order.
    pub fn served(&self) -> Vec<String> {
        self.served
            .lock()
            .unwrap_or_else(PoisonError::into_inner)
            .clone()
    }
}

impl Locating for Planted {
    fn located(&self, folder: &Path) -> Result<Located, NotLocated> {
        let folders = self.folders.lock().unwrap_or_else(PoisonError::into_inner);
        folders
            .get(folder)
            .cloned()
            .ok_or_else(|| NotLocated::NotARepository {
                folder: folder.display().to_string(),
                why: String::from("nothing is planted there"),
            })
    }

    fn serving(&self, root: &str) {
        self.served
            .lock()
            .unwrap_or_else(PoisonError::into_inner)
            .push(root.to_string());
    }
}
