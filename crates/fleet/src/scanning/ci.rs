//! What a repository's CI jobs run, joined into Scan's findings.
//!
//! **Through [`CiConfiguration`](adapter_traits::CiConfiguration), so Fleet
//! names no provider.** A file the reader claims is the reader's to report; one
//! nobody claims stays Scan's own not read, never clean.

use std::collections::BTreeSet;

use adapter_traits::CiReading;
use ipc::{CiCommand, RepositoryScan};

use super::not_read;

pub(super) fn join(scan: &mut RepositoryScan, reading: CiReading) {
    let claimed: BTreeSet<&str> = reading.claimed.iter().map(String::as_str).collect();
    scan.not_read
        .retain(|one| !claimed.contains(one.file.as_str()));
    for workspace in &mut scan.workspaces {
        workspace
            .not_read
            .retain(|one| !claimed.contains(one.file.as_str()));
    }
    let unfollowed = reading.not_followed.into_iter();
    scan.not_read
        .extend(unfollowed.map(|one| not_read(one.file, one.why)));
    scan.ci_commands = reading
        .commands
        .into_iter()
        .map(|one| CiCommand {
            file: one.file,
            job: one.job,
            key: one.key,
            run: one.run,
            cell: one.cell,
        })
        .collect();
}
