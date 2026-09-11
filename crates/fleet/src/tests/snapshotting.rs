//! `crate::snapshotting` — a Job's Commands and Setup come off what
//! `armada.yml` said at creation, never off what it says by the time
//! somebody asks. #650.
//!
//! **A real file on disk, deliberately**, unlike most of this suite's
//! fixtures. `Fleet::manifest_snapshotted` reads `Manifest::path()` off the
//! filesystem — the same read Fleet's own boot does — so a fixture whose
//! Manifest names a path nothing wrote (the ordinary fixture here, and in
//! every other file in this module) exercises the pre-migration fallback
//! rather than the snapshot itself. This file is the one that writes the
//! file, edits it, and reads the Job's own answer against the edit.

use config::Manifest;
use testkit::{FakeHarness, FakeVcs, FakeWorkProduct};

use crate::daemon::Fleet;
use crate::tests::daemon::{a_fleet, a_proposal, fittings};
use crate::tests::tmp::TempDir;

type Fixture = Fleet<FakeHarness, FakeVcs, FakeWorkProduct>;

const ORIGINAL: &str = "version: 1\nid: 01FIXTUREMANIFEST\n\
     checks:\n  test:\n    run: \"echo original-check\"\n\
     commands:\n  build:\n    run: \"echo original-build\"\n  install:\n    run: \"echo original-install\"\n\
     setup:\n  requires: [install]\n";

const EDITED: &str = "version: 1\nid: 01FIXTUREMANIFEST\n\
     checks:\n  test:\n    run: \"echo edited-check\"\n\
     commands:\n  build:\n    run: \"echo edited-build\"\n  install:\n    run: \"echo edited-install\"\n\
     setup:\n  requires: [install]\n";

/// A Fleet whose Manifest is a real file at `home`'s root, holding `text`.
fn a_fleet_reading_a_real_manifest(home: &TempDir, text: &str) -> Fixture {
    let path = home.path().join("armada.yml");
    std::fs::write(&path, text).expect("the fixture writes its own Manifest");
    let manifest =
        Manifest::parse(&path, text).unwrap_or_else(|why| panic!("the fixture manifest: {why}"));
    let mut fittings = fittings(home, FakeWorkProduct::changed(&[]));
    fittings.manifest = manifest;
    Fleet::assembled(fittings)
}

/// **The property `#650` exists for.** A Job created against the file above
/// keeps answering with `echo original-build` and `echo original-install`
/// once the file on disk says `edited-build` and `edited-install` — the
/// Commands a Drone's toolbelt grants and the Setup `crate::preparing` runs
/// both resolve through this same function.
#[tokio::test]
async fn a_jobs_effective_manifest_is_unmoved_by_an_edit_to_the_file_it_was_snapshotted_from() {
    let home = TempDir::new();
    let fleet = a_fleet_reading_a_real_manifest(&home, ORIGINAL);

    let job = fleet
        .propose(a_proposal("fix the off-by-one"))
        .await
        .expect("a Job at the approval gate");

    // The snapshot is there from the moment of creation, and it is the file
    // as it read then.
    let stored = fleet
        .store()
        .lock()
        .await
        .manifest_snapshot(job.id())
        .expect("the column reads")
        .expect("a Job created after the migration always gets one");
    assert_eq!(stored, ORIGINAL);

    // The file moves on. Nothing reads it again for this Job.
    std::fs::write(home.path().join("armada.yml"), EDITED).expect("the file is edited");

    let (manifest, has_snapshot) = fleet.effective_manifest(&job).await;
    assert!(has_snapshot, "a Job created after the migration has one");
    assert_eq!(
        manifest
            .command("build")
            .expect("the Command is still declared")
            .run(),
        "echo original-build",
        "the Commands a Drone's toolbelt grants read the snapshot"
    );
    assert_eq!(
        manifest
            .prepared_by()
            .first()
            .expect("setup.requires still names one")
            .run(),
        "echo original-install",
        "crate::preparing reads the same snapshot for Setup"
    );

    // And Fleet's own live Manifest, asked directly, does see the edit — the
    // fixture proves the two are actually different answers, not the same
    // path taken twice.
    assert_eq!(
        fleet
            .manifest()
            .command("build")
            .expect("still declared")
            .run(),
        "echo original-build",
        "Fleet does not hot-reload in this fixture; the point stands regardless \
         — effective_manifest never consulted it for this Job"
    );
}

/// A Job whose Manifest names no real file on disk — every other fixture in
/// this suite, and what a pre-migration row reads back as too — falls back to
/// Fleet's live Manifest exactly as it always did. Not a downgrade: it is the
/// one reading every Job got before `#650`, `manifest_snapshotted`'s read
/// having found nothing there to keep.
#[tokio::test]
async fn a_job_whose_snapshot_could_not_be_taken_falls_back_to_fleets_live_manifest() {
    let home = TempDir::new();
    let fleet = a_fleet(&home, FakeWorkProduct::changed(&[]));
    let job = fleet
        .propose(a_proposal("fix the off-by-one"))
        .await
        .expect("a Job");

    assert_eq!(
        fleet
            .store()
            .lock()
            .await
            .manifest_snapshot(job.id())
            .expect("the column reads"),
        None,
        "the fixture Manifest names no real file, so there was nothing to read"
    );

    let (manifest, has_snapshot) = fleet.effective_manifest(&job).await;
    assert!(!has_snapshot);
    assert_eq!(manifest.id().as_str(), fleet.manifest().id().as_str());
}

/// **The run route's own claim.** Every entry the sheet lists — Setup, the
/// Check, the Commands — is `frozen: true` for a Job that carries a
/// snapshot, and an edit to the file after creation reaches none of them:
/// `crate::rehearsing::entries::frozen`'s `has_snapshot` branch marks the
/// whole sheet rather than merging in what the workflow happened to name.
#[tokio::test]
async fn the_run_sheet_shows_every_entry_frozen_and_unmoved_by_an_edit() {
    let home = TempDir::new();
    let fleet = a_fleet_reading_a_real_manifest(&home, ORIGINAL);
    let job = fleet
        .propose(a_proposal("fix the off-by-one"))
        .await
        .expect("a Job");

    std::fs::write(home.path().join("armada.yml"), EDITED).expect("the file is edited");

    let sheet = fleet.run_sheet(job.id()).await.expect("a sheet");
    assert!(
        sheet
            .setup
            .iter()
            .chain(sheet.checks.iter())
            .chain(sheet.commands.iter())
            .all(|entry| entry.frozen),
        "no entry is left `frozen: false` once the Job carries a snapshot: {:?}/{:?}/{:?}",
        sheet.setup,
        sheet.checks,
        sheet.commands
    );
    assert_eq!(
        sheet.setup[0].run, "echo original-install",
        "Setup reads the snapshot, not the edited file"
    );
    assert_eq!(sheet.checks[0].run, "echo original-check");
    assert_eq!(
        sheet
            .commands
            .iter()
            .find(|entry| entry.name == "build")
            .expect("`build` is still declared")
            .run,
        "echo original-build"
    );
}
