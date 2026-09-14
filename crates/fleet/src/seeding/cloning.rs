//! A directory cloned copy-on-write, or not at all. #1064.
//!
//! **Never a full copy.** A seed is a build directory of many gigabytes, and on
//! a volume that cannot clone, copying one costs more than the build it saves.

use std::path::Path;

/// Why a directory was not cloned.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum NotCloned {
    /// The volume has no copy-on-write clone, or the two paths are on two
    /// volumes. Nothing was written.
    Unsupported { why: String },
    /// The clone was tried and failed.
    Failed { why: String },
}

/// Cloning a directory. **A seam so a test clones nothing real**, as
/// [`Machine`](crate::headroom::Machine) is for the machine's readings.
pub trait CopyOnWrite: Send + Sync {
    /// Clone the directory `from` to `to`, which must not exist yet.
    fn clone_tree(&self, from: &Path, to: &Path) -> Result<(), NotCloned>;
}

/// The volume Fleet runs on.
#[derive(Clone, Copy, Debug, Default)]
pub struct TheVolume;

impl CopyOnWrite for TheVolume {
    fn clone_tree(&self, from: &Path, to: &Path) -> Result<(), NotCloned> {
        clone_tree(from, to)
    }
}

/// One `clonefile(2)` over the whole directory. It fails with `ENOTSUP` on a
/// volume that cannot clone rather than copying, which is the refusal a seed
/// needs; `std::fs::copy` falls back to a copy.
#[cfg(target_os = "macos")]
fn clone_tree(from: &Path, to: &Path) -> Result<(), NotCloned> {
    use std::ffi::CString;
    use std::os::unix::ffi::OsStrExt;

    let (Ok(source), Ok(target)) = (
        CString::new(from.as_os_str().as_bytes()),
        CString::new(to.as_os_str().as_bytes()),
    ) else {
        return Err(NotCloned::Failed {
            why: String::from("a path holds a NUL byte"),
        });
    };
    if clonefile(&source, &target) == 0 {
        return Ok(());
    }
    let error = std::io::Error::last_os_error();
    match error.raw_os_error() {
        Some(libc::ENOTSUP) | Some(libc::EXDEV) => Err(NotCloned::Unsupported {
            why: error.to_string(),
        }),
        _ => Err(NotCloned::Failed {
            why: error.to_string(),
        }),
    }
}

#[cfg(target_os = "macos")]
#[allow(unsafe_code)]
fn clonefile(source: &std::ffi::CStr, target: &std::ffi::CStr) -> libc::c_int {
    // SAFETY: both pointers come from `CStr`s that outlive the call and are
    // NUL-terminated by construction; `clonefile` reads them and writes
    // nothing through either. Flags are zero.
    unsafe { libc::clonefile(source.as_ptr(), target.as_ptr(), 0) }
}

#[cfg(not(target_os = "macos"))]
fn clone_tree(_from: &Path, _to: &Path) -> Result<(), NotCloned> {
    Err(NotCloned::Unsupported {
        why: String::from("no copy-on-write clone is built for this platform"),
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    /// A three-file tree, never a real `target/`. A volume that cannot clone
    /// answers `Unsupported` and writes nothing, which is also a pass.
    #[test]
    fn a_small_tree_is_cloned_whole_or_not_at_all() {
        let home = crate::tests::tmp::TempDir::new();
        let from = home.path().join("from");
        std::fs::create_dir_all(from.join("debug/deps")).unwrap();
        std::fs::write(from.join("debug/deps/one.rlib"), "one").unwrap();
        std::fs::write(from.join("CACHEDIR.TAG"), "tag").unwrap();
        let to = home.path().join("to");

        match TheVolume.clone_tree(&from, &to) {
            Ok(()) => {
                assert_eq!(
                    std::fs::read_to_string(to.join("debug/deps/one.rlib")).unwrap(),
                    "one"
                );
                assert_eq!(
                    std::fs::read_to_string(to.join("CACHEDIR.TAG")).unwrap(),
                    "tag"
                );
            }
            Err(NotCloned::Unsupported { .. }) => assert!(!to.exists()),
            Err(other) => panic!("a clone into a fresh path failed: {other:?}"),
        }
    }

    #[test]
    fn a_clone_onto_a_path_that_exists_fails_rather_than_merging() {
        let home = crate::tests::tmp::TempDir::new();
        let from = home.path().join("from");
        std::fs::create_dir_all(&from).unwrap();
        let to = home.path().join("to");
        std::fs::create_dir_all(&to).unwrap();
        assert!(TheVolume.clone_tree(&from, &to).is_err());
    }
}
