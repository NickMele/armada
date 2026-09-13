//! A repository's CI configuration, read for the commands its jobs run.
//!
//! **Handed [`RepositoryFiles`], which only reads**, so an implementation has
//! no write and no process to reach for: what a job runs is evidence, and
//! running it is not a reader's to decide.

use alloc::string::String;
use alloc::vec::Vec;

/// What reading one path came to.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum FileRead {
    /// Nothing is there.
    Absent,
    Bytes(Vec<u8>),
    /// Something is there and would not read, with why.
    Unreadable(String),
}

/// One entry of a directory. A symbolic link is not listed.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct FileEntry {
    pub name: String,
    pub is_dir: bool,
}

/// A repository's files, read and never written. Paths are relative and
/// `/`-separated, and `""` is the root.
pub trait RepositoryFiles {
    fn read(&self, path: &str) -> FileRead;
    /// The entries of `dir`, or why it would not list.
    fn entries(&self, dir: &str) -> Result<Vec<FileEntry>, String>;
}

/// One command a CI job runs, and where that is written.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct CiCommand {
    /// Relative to the checkout.
    pub file: String,
    /// The job, by its id in that file.
    pub job: String,
    /// Where in the file the command is written.
    pub key: String,
    /// Verbatim as the file writes it.
    pub run: String,
    /// The one matrix cell this was read as, where the job has a matrix — one
    /// finding per command, never one per cell. Rendered, never matched on.
    pub cell: Option<String>,
}

/// CI configuration that was not followed, and why. **Never clean.**
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct CiNotFollowed {
    pub file: String,
    pub why: String,
}

/// Everything one reader made of a repository's CI configuration.
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct CiReading {
    pub commands: Vec<CiCommand>,
    pub not_followed: Vec<CiNotFollowed>,
    /// Every file recognised as CI configuration, followed or not, so a reader
    /// that knows none of them does not report it a second time.
    pub claimed: Vec<String>,
}

/// Reads what a repository's CI jobs run, for whichever providers an
/// implementation knows. One it does not know is in `not_followed`, or is left
/// unclaimed for the caller to report.
pub trait CiConfiguration {
    fn read_jobs(&self, files: &dyn RepositoryFiles) -> CiReading;
}
