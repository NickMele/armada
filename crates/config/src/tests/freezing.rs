//! `freeze:` — read, refused, written by a form, and moved under a running Fleet.

use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicU64, Ordering};

use crate::amending::{amend, Edit};
use crate::error::{Fault, LoadError};
use crate::manifest::Manifest;
use crate::LiveKey;

static NEXT: AtomicU64 = AtomicU64::new(0);

fn at() -> &'static Path {
    Path::new("armada.yml")
}

fn parsed(text: &str) -> Manifest {
    Manifest::parse(at(), text).unwrap_or_else(|why| panic!("the file loads: {why}"))
}

fn amended(text: &str, edit: Edit) -> String {
    match amend(at(), text, &[edit]) {
        Ok(done) => done.text().to_string(),
        Err(why) => panic!("the edit applies: {why}"),
    }
}

/// Commented on purpose: the round trip is only worth anything if the comments survive it.
const COMMENTED: &str = "\
# Armada's own settings for this repository.
version: 1
id: armada

# The gate, in the order it runs.
checks:
  build:
    run: cargo build --workspace   # the slow one

auto_merge: never
";

#[test]
fn a_file_that_says_nothing_is_not_frozen() {
    assert!(!parsed("version: 1\nid: armada\n").frozen());
}

#[test]
fn true_freezes_and_false_does_not() {
    assert!(parsed("version: 1\nid: armada\nfreeze: true\n").frozen());
    assert!(!parsed("version: 1\nid: armada\nfreeze: false\n").frozen());
}

/// `freeze: yes` reading as frozen would be right by luck, and `freeze: no` would not be.
#[test]
fn a_freeze_that_is_not_true_or_false_is_refused_at_its_key() {
    let Err(LoadError::Refused { refusals, .. }) =
        Manifest::parse(at(), "version: 1\nid: armada\nfreeze: \"release\"\n")
    else {
        panic!("a word is not a flag");
    };
    assert!(
        refusals
            .iter()
            .any(|refusal| refusal.key == "freeze"
                && matches!(refusal.fault, Fault::WrongType { .. }))
    );
}

/// **The round trip, comments kept.** Setting it adds one line; clearing it takes that line
/// back out, and the file is what it was.
#[test]
fn a_form_freezes_and_unfreezes_and_every_other_byte_stays() {
    let frozen = amended(COMMENTED, Edit::Freeze(true));
    assert!(parsed(&frozen).frozen());
    assert!(frozen.lines().any(|line| line == "freeze: true"));
    let mut left = frozen.lines();
    assert!(
        COMMENTED
            .lines()
            .all(|line| left.any(|found| found == line)),
        "every line of the file is still there, comments included:\n{frozen}"
    );

    let lifted = amended(&frozen, Edit::Freeze(false));
    assert!(!parsed(&lifted).frozen());
    assert_eq!(
        lifted, COMMENTED,
        "and lifting it leaves the file it started from"
    );
}

/// Absent already means not frozen, so a written `false` is somebody's and stays.
#[test]
fn clearing_a_freeze_leaves_a_written_false_alone() {
    let text = "version: 1\nid: armada\nfreeze: false\n";
    assert_eq!(amended(text, Edit::Freeze(false)), text);
}

struct Held(PathBuf);

impl Held {
    fn file(&self) -> PathBuf {
        self.0.join("armada.yml")
    }
}

impl Drop for Held {
    fn drop(&mut self) {
        let _ = std::fs::remove_dir_all(&self.0);
    }
}

/// **Live, which is the whole claim.** A person lifting a freeze is answered by the
/// Manifest Fleet already holds, and the reading names the key that moved.
#[test]
fn a_freeze_lifted_under_a_running_fleet_moves_the_manifest_fleet_holds() {
    let dir = std::env::temp_dir().join(format!(
        "armada-freeze-{}-{}",
        std::process::id(),
        NEXT.fetch_add(1, Ordering::Relaxed)
    ));
    std::fs::create_dir_all(&dir).expect("a temporary directory");
    let held = Held(dir);
    std::fs::write(held.file(), "version: 1\nid: armada\nfreeze: true\n").expect("the file");
    let (manifest, reloads) = Manifest::reloadable(&held.file()).expect("it loads");
    assert!(manifest.frozen());

    std::fs::write(held.file(), "version: 1\nid: armada\n").expect("the save");
    let adopted = reloads.reread().expect("the save loads");

    assert!(
        !manifest.frozen(),
        "the Manifest Fleet holds sees the freeze lifted"
    );
    assert!(adopted
        .moved()
        .iter()
        .any(|moved| moved.key == LiveKey::Freeze && moved.after.as_deref() == Some("false")));
}
