//! What `preflight` stamped about a tree: which pull request it is, and
//! which Checks its combination with the base hits.

use serde::{Deserialize, Serialize};

use super::codec::{self, ReadStateError, WriteStateError};
use super::dir::StateDir;

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct PreflightStamp {
    pub branch: String,
    pub head: String,
    pub tree: String,
    pub base: String,
    pub pr: u64,
    pub checks: Vec<String>,
}

pub fn read_stamp(dir: &StateDir, branch: &str) -> Result<Option<PreflightStamp>, ReadStateError> {
    codec::read("preflight stamp", &dir.stamp_path(branch))
}

pub fn write_stamp(dir: &StateDir, stamp: &PreflightStamp) -> Result<(), WriteStateError> {
    codec::write(&dir.stamp_path(&stamp.branch), stamp)
}
