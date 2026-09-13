//! What a given-back worktree must keep true on the wire.
//!
//! Bridge tells a deliberately kept branch from one that would not delete by
//! whether `unmerged_commits` is there, and frames a deleted branch's SHA only
//! where `tip` is. Present-and-null passes both checks and crashed the window
//! the first time a branch went with no commit under it.

use crate::{decode, encode, JobId, ReclaimedBranch, ReclaimedWorktree, WorktreeReclaimed};

fn already_gone() -> WorktreeReclaimed {
    WorktreeReclaimed {
        job_id: JobId::carried("01JOB"),
        worktree: ReclaimedWorktree {
            path: "/worktrees/01JOB".to_string(),
            removed: true,
            why: None,
        },
        branch: ReclaimedBranch {
            branch: "armada/01JOB".to_string(),
            deleted: true,
            tip: None,
            why: None,
            base: None,
            unmerged_commits: None,
        },
    }
}

#[test]
fn an_empty_half_carries_no_key_and_no_null() {
    let written = encode(&already_gone()).expect("a reclaim encodes");
    for key in ["tip", "why", "base", "unmerged_commits", "null"] {
        assert!(!written.contains(key), "{key} in {written}");
    }
}

#[test]
fn a_reclaim_round_trips_with_every_half_absent() {
    let written = encode(&already_gone()).expect("a reclaim encodes");
    let read: WorktreeReclaimed = decode("a reclaim", written.as_bytes()).expect("it reads");
    assert_eq!(read, already_gone());
}
