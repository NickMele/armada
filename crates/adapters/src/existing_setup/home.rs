//! A home directory on this machine, read and never written.
//!
//! **Apart from [`super`] because it knows a different thing.** That module
//! knows how one harness spells a home; this one knows what reading somebody's
//! own directory on macOS means — a link to follow, and a path that must stay
//! under the root it was given.

use std::fs;
use std::path::{Path, PathBuf};

use adapter_traits::{FileEntry, FileRead, SetupFiles};

/// A home directory on this machine.
///
/// **Follows a symbolic link**, unlike the reader of a checkout: a home is
/// where a dotfiles repository puts its links, and refusing them would report
/// an empty setup to exactly the people most likely to have a full one.
pub struct Home {
    root: PathBuf,
}

impl Home {
    pub fn at(root: impl Into<PathBuf>) -> Home {
        Home { root: root.into() }
    }

    /// Under the root, and never above it: a `..` in a path this crate builds
    /// would be a bug, and one that reached here would read another directory.
    fn under(&self, path: &str) -> Option<PathBuf> {
        let clean = path.trim_matches('/');
        (!clean.split('/').any(|part| part == "..")).then(|| match clean.is_empty() {
            true => self.root.clone(),
            false => self.root.join(Path::new(clean)),
        })
    }
}

impl SetupFiles for Home {
    fn read(&self, path: &str) -> FileRead {
        let Some(full) = self.under(path) else {
            return FileRead::Unreadable("above the home it is read from".to_string());
        };
        match fs::read(&full) {
            Ok(bytes) => FileRead::Bytes(bytes),
            Err(why) if why.kind() == std::io::ErrorKind::NotFound => FileRead::Absent,
            Err(why) => FileRead::Unreadable(why.to_string()),
        }
    }

    fn entries(&self, dir: &str) -> Result<Vec<FileEntry>, String> {
        let full = self
            .under(dir)
            .ok_or_else(|| "above the home it is read from".to_string())?;
        let mut entries = Vec::new();
        for entry in fs::read_dir(&full).map_err(|why| why.to_string())? {
            let entry = entry.map_err(|why| why.to_string())?;
            let name = entry.file_name().to_string_lossy().to_string();
            // `metadata` follows a link; `file_type` would report the link.
            let is_dir = fs::metadata(entry.path())
                .map(|it| it.is_dir())
                .unwrap_or(false);
            entries.push(FileEntry { name, is_dir });
        }
        entries.sort_by(|left, right| left.name.cmp(&right.name));
        Ok(entries)
    }
}
