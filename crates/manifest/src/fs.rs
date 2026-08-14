//! The filesystem, which is deliberately **not** a seam.
//!
//! `ARCHITECTURE.md` §1.1 gives three injected seams and the filesystem is not
//! among them: a fake gives you a green test over your own fake's semantics,
//! and the semantics that matter here — `ENOENT` versus `EACCES`, an unlinked
//! inode that still accepts writes — are exactly the ones a fake gets wrong.
//! Tests use a real `tempfile::TempDir`.

use armada_core::error::{CharError, ErrClass};
use armada_core::reap::PathStat;
use std::io;
use std::path::{Path, PathBuf};

/// Resolve symlinks and relative components.
///
/// Load-bearing for identity: two paths reaching one directory must not
/// produce two `workspace_id`s, and a symlinked checkout is the ordinary way
/// that happens (PLAN.md §2.2).
pub fn canonical(path: &Path) -> Result<PathBuf, CharError> {
    std::fs::canonicalize(path).map_err(|e| CharError {
        class: ErrClass::Environment,
        r#where: path.display().to_string(),
        message: format!("cannot resolve path: {e}"),
        next_action: None,
    })
}

/// Ask the filesystem whether a directory is there, keeping the three answers
/// apart.
///
/// **Only `ENOENT` means gone.** Measured: `stat` on a *live* directory under a
/// mode-000 parent returns `EACCES`, which is byte-identical in failure to a
/// missing path — and `$HOME` is 0700 on Linux, so that is the ordinary
/// multi-user and devcontainer case rather than an exotic one. Collapsing the
/// two is how `char clean` in one workspace destroys another's live containers
/// (PLAN.md §2.3.1).
pub fn stat(path: &Path) -> PathStat {
    match std::fs::symlink_metadata(path) {
        Ok(_) => PathStat::Present,
        Err(e) if e.kind() == io::ErrorKind::NotFound => PathStat::Missing,
        Err(e) => PathStat::Unreadable(e.to_string()),
    }
}

/// Create `.armada/` and the two directories inside it (PLAN.md §4.2).
///
/// ```text
/// .armada/
///   logs/<component>.log            services — `up` is not a run
///   run/<run-id>/…                  checks
/// ```
///
/// **One rule decides what may live here: if losing it would leak a resource,
/// it does not belong.** A workspace directory is deleted by `rm -rf` or
/// `git worktree remove`, neither of which consults char — so anything recorded
/// only here is gone precisely when it is most needed. Everything reclaimable
/// lives in `~/.armada/manifest.db` instead.
pub fn create_armada_dir(root: &Path) -> Result<PathBuf, CharError> {
    let dir = root.join(".armada");
    for path in [dir.clone(), dir.join("logs"), dir.join("run")] {
        std::fs::create_dir_all(&path).map_err(|e| CharError {
            class: ErrClass::Environment,
            r#where: path.display().to_string(),
            message: format!("cannot create {}: {e}", path.display()),
            next_action: None,
        })?;
    }
    Ok(dir)
}

/// Remove `.armada/` entirely. Absent is success — `clean` is idempotent.
pub fn remove_armada_dir(root: &Path) -> Result<bool, CharError> {
    let dir = root.join(".armada");
    match std::fs::remove_dir_all(&dir) {
        Ok(()) => Ok(true),
        Err(e) if e.kind() == io::ErrorKind::NotFound => Ok(false),
        Err(e) => Err(CharError {
            class: ErrClass::Environment,
            r#where: dir.display().to_string(),
            message: format!("cannot remove .armada/: {e}"),
            next_action: None,
        }),
    }
}

/// Remove a declared `owns.files` path, relative to the workspace root.
///
/// Returns whether anything was there. Refuses to escape the workspace: a
/// declared path is a claim about this workspace's own tree, and `..` in one
/// would make `char clean --artifacts --all` a machine-wide `rm -rf` driven by
/// a committed file.
pub fn remove_owned_file(root: &Path, declared: &str) -> Result<bool, CharError> {
    let target = root.join(declared);
    let escapes = target
        .components()
        .any(|c| matches!(c, std::path::Component::ParentDir));
    if escapes || Path::new(declared).is_absolute() {
        return Err(CharError::bad_config(
            armada_core::error::ConfigWhere::Path {
                file: "armada.yml".to_string(),
                path: "owns.files".to_string(),
            },
            format!("`{declared}` reaches outside the workspace"),
            "declare a path inside the workspace, or remove it by hand",
        ));
    }

    let result = if stat(&target) == PathStat::Missing {
        return Ok(false);
    } else if target.is_dir() {
        std::fs::remove_dir_all(&target)
    } else {
        std::fs::remove_file(&target)
    };

    match result {
        Ok(()) => Ok(true),
        Err(e) if e.kind() == io::ErrorKind::NotFound => Ok(false),
        Err(e) => Err(CharError {
            class: ErrClass::Environment,
            r#where: target.display().to_string(),
            message: format!("cannot remove {}: {e}", target.display()),
            next_action: None,
        }),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_missing_path_is_missing_and_a_live_one_is_present() {
        let dir = tempfile::tempdir().unwrap();
        assert_eq!(stat(dir.path()), PathStat::Present);
        assert_eq!(stat(&dir.path().join("nope")), PathStat::Missing);
    }

    /// The measured case: a live directory under a mode-000 parent answers
    /// `EACCES`, and char must not read that as gone.
    #[cfg(unix)]
    #[test]
    fn an_unreadable_parent_yields_unreadable_and_never_missing() {
        use std::os::unix::fs::PermissionsExt;
        if nix_is_root() {
            return; // root ignores the mode, so the case cannot be built.
        }
        let dir = tempfile::tempdir().unwrap();
        let parent = dir.path().join("locked");
        let child = parent.join("live");
        std::fs::create_dir_all(&child).unwrap();
        std::fs::set_permissions(&parent, std::fs::Permissions::from_mode(0o000)).unwrap();

        let answer = stat(&child);
        std::fs::set_permissions(&parent, std::fs::Permissions::from_mode(0o755)).unwrap();

        assert!(
            matches!(answer, PathStat::Unreadable(_)),
            "EACCES must not be Missing, got {answer:?}"
        );
    }

    #[cfg(unix)]
    fn nix_is_root() -> bool {
        std::process::Command::new("id")
            .arg("-u")
            .output()
            .map(|o| String::from_utf8_lossy(&o.stdout).trim() == "0")
            .unwrap_or(false)
    }

    #[test]
    fn the_char_dir_is_created_with_its_two_subdirectories_and_removed_whole() {
        let dir = tempfile::tempdir().unwrap();
        let armada_dir = create_armada_dir(dir.path()).unwrap();
        assert!(armada_dir.join("logs").is_dir());
        assert!(armada_dir.join("run").is_dir());

        assert!(remove_armada_dir(dir.path()).unwrap());
        assert!(!armada_dir.exists());
        // Idempotent: `clean` twice is not an error.
        assert!(!remove_armada_dir(dir.path()).unwrap());
    }

    #[test]
    fn creating_the_char_dir_twice_is_not_an_error() {
        let dir = tempfile::tempdir().unwrap();
        create_armada_dir(dir.path()).unwrap();
        create_armada_dir(dir.path()).unwrap();
    }

    #[test]
    fn a_declared_owns_file_is_removed_and_a_missing_one_is_not_an_error() {
        let dir = tempfile::tempdir().unwrap();
        std::fs::create_dir_all(dir.path().join("node_modules/a")).unwrap();
        assert!(remove_owned_file(dir.path(), "node_modules").unwrap());
        assert!(!remove_owned_file(dir.path(), "node_modules").unwrap());
    }

    /// `clean --artifacts --all` deletes every declared `owns.files` on the
    /// machine. A `..` in one would make that a machine-wide `rm -rf` driven
    /// by a committed file.
    #[test]
    fn a_declared_path_may_not_escape_the_workspace() {
        let dir = tempfile::tempdir().unwrap();
        assert!(remove_owned_file(dir.path(), "../outside").is_err());
        assert!(remove_owned_file(dir.path(), "/etc").is_err());
    }
}
