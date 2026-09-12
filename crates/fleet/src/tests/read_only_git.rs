//! The read-only git grant every toolbelt carries.
//!
//! `spawning::toolbelt` puts [`Grant::ReadTheRepository`] on every Drone
//! beside [`Grant::ReadTheWorktree`], unconditionally — not read off anyone's
//! own settings file, and not something a Job's setting can withhold. So the
//! whole of what needs asserting is that one ordinary spawn, through the
//! ordinary path, carries it: there is no setting to vary, because the grant
//! is not conditioned on one.
//!
//! `adapters::harness`'s own tests are where each verb is asserted on the
//! rendered `--allowedTools` value and where the fixed `would_push` is
//! asserted on the subcommand rather than the word — both of those are
//! properties of a rendering, not of a Fleet, and belong there.

use adapter_traits::Grant;
use testkit::FakeWorkProduct;

use crate::tests::admitted::dispatched;
use crate::tests::daemon::{a_fleet, a_proposal, worktree_directory};
use crate::tests::tmp::TempDir;

#[tokio::test]
async fn every_spawned_drone_carries_the_read_only_git_grant() {
    let home = TempDir::new();
    let fleet = a_fleet(&home, FakeWorkProduct::changed(&["src/log.rs"]));
    let job = fleet
        .propose(a_proposal("read the repository"))
        .await
        .expect("a proposal");
    worktree_directory(&home, &job);
    dispatched(&fleet, job.id()).await.expect("it is approved");

    let configured = fleet.harness().configured();
    assert_eq!(configured.len(), 1, "one step, one Drone");
    assert!(
        configured[0]
            .toolbelt()
            .granted()
            .contains(&Grant::ReadTheRepository),
        "a Drone was spawned without the read-only git grant: {:?}",
        configured[0].toolbelt().granted()
    );
}
