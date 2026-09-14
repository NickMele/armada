//! The sweep beside a warm-up: which seeds it keeps, and which one a new
//! base's warm-up starts from.

use std::path::Path;
use std::sync::Arc;

use adapter_traits::BaseSpec;
use config::Manifest;
use testkit::{FakeHarness, FakeVcs, FakeWorkProduct};

use super::{a_seed, base, Copying, COMMIT, PREVIOUS};
use crate::daemon::Fleet;
use crate::tests::daemon::fittings;
use crate::tests::tmp::TempDir;

/// A Fleet whose seed is warmed by two commands, so a warm-up can be held
/// open while a sweep runs beside it.
fn a_fleet_warming_with(
    home: &TempDir,
    first: &str,
    then: &str,
) -> Fleet<FakeHarness, FakeVcs, FakeWorkProduct> {
    let mut fittings = fittings(home, FakeWorkProduct::changed(&["src/log.rs"]));
    fittings.vcs = FakeVcs::new().with_ref_at("main", COMMIT);
    fittings.copy_on_write = Arc::new(Copying::default());
    fittings.starting().manifest = Manifest::parse(
        Path::new("armada.yml"),
        &format!(
            "version: 1\nid: 01FIXTUREMANIFEST\nbase: main\ncommands:\n\
             \x20 first:\n    run: {first}\n\
             \x20 then:\n    run: {then}\n\
             setup:\n  seed:\n    paths: [target]\n    warm: [first, then]\n"
        ),
    )
    .expect("a manifest that declares a seed");
    Fleet::assembled(fittings)
}

/// A seed's marker, dated `seconds` before now.
fn marked_ago(spec: &BaseSpec, seconds: u64) {
    let at = std::time::SystemTime::now() - std::time::Duration::from_secs(seconds);
    std::fs::File::options()
        .write(true)
        .open(spec.seed_marker())
        .and_then(|marker| marker.set_modified(at))
        .expect("the marker's time is set");
}

/// **The sweep that starts a warm-up must not take the seed it starts from.**
/// The base has moved to `COMMIT` and the last warm seed is at `PREVIOUS`. One
/// full sweep starts the new warm-up and walks the base checkouts in the same
/// call; `PREVIOUS` stays until `COMMIT`'s seed is marked, and only then goes.
#[tokio::test]
async fn a_sweep_keeps_the_last_warm_seed_until_the_new_one_is_marked() {
    let home = TempDir::new();
    let previous = a_seed(&home, PREVIOUS, true);
    let spec = base(&home, COMMIT);
    std::fs::create_dir_all(spec.path()).expect("a base checkout");
    let fleet = a_fleet_warming_with(
        &home,
        "/bin/sleep 2",
        "/bin/test -f target/debug/libwarm.rlib",
    );

    let swept = fleet.reclaim_what_is_safe().await.expect("the sweep runs");
    assert!(
        !swept.bases.contains(&PREVIOUS.to_string())
            && !fleet.vcs().dropped_bases().contains(&PREVIOUS.to_string()),
        "the sweep that started the warm-up took the seed it starts from: {:?}",
        swept.bases
    );

    fleet
        .warm_up_the_sweep_started()
        .expect("the sweep started a warm-up")
        .await
        .expect("the warm-up ends");
    assert!(
        Path::new(&spec.seed_marker()).exists(),
        "the new warm-up found the previous seed's build in its checkout"
    );
    assert!(Path::new(&previous.path()).exists());

    assert_eq!(
        fleet.bases_gone_by(),
        vec![PREVIOUS.to_string()],
        "once the new seed is warm, the old one is given back"
    );
}

/// A warm-up that fails leaves the last warm seed standing for the next one.
#[tokio::test]
async fn a_failed_warm_up_leaves_the_last_warm_seed_standing() {
    let home = TempDir::new();
    a_seed(&home, PREVIOUS, true);
    std::fs::create_dir_all(base(&home, COMMIT).path()).expect("a base checkout");
    let fleet = a_fleet_warming_with(&home, "/usr/bin/true", "/usr/bin/false");

    fleet.reclaim_what_is_safe().await.expect("the sweep runs");
    fleet
        .warm_up_the_sweep_started()
        .expect("the sweep started a warm-up")
        .await
        .expect("the warm-up ends");

    assert!(fleet.bases_gone_by().is_empty());
}

/// Of two warm seeds, the warm-up starts from the one marked most recently,
/// whatever order the directory lists them in.
#[tokio::test]
async fn a_warm_up_starts_from_the_newest_warm_seed() {
    const OLDEST: &str = "c911ffc2000000000000000000000000000000ab";
    let home = TempDir::new();
    marked_ago(&a_seed(&home, OLDEST, true), 3_600);
    marked_ago(&a_seed(&home, PREVIOUS, true), 60);
    let spec = base(&home, COMMIT);
    std::fs::create_dir_all(spec.path()).expect("a base checkout");
    let fleet = a_fleet_warming_with(
        &home,
        "/usr/bin/true",
        &format!("/usr/bin/grep -q {PREVIOUS} target/debug/libwarm.rlib"),
    );

    fleet
        .warm_seeds()
        .expect("a warm-up starts")
        .await
        .expect("the warm-up ends");

    assert!(
        Path::new(&spec.seed_marker()).exists(),
        "the warm-up started from the newest seed"
    );
    assert_eq!(
        fleet.bases_gone_by().len(),
        2,
        "both older seeds go once it is warm"
    );
}
