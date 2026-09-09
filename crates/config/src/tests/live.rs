//! Re-reading an `armada.yml` under a running Fleet: what moves, what does
//! not, and what a broken file does.
//!
//! **These write real files**, unlike the rest of this crate's tests, because
//! the subject is the second read rather than the parse. The temporary
//! directory is this module's own so the parser tests stay filesystem-free.

use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicU64, Ordering};

use core_model::{AutoMerge, ReviewGate};

use crate::live::Frozen;
use crate::manifest::Manifest;
use crate::LiveKey;

static NEXT: AtomicU64 = AtomicU64::new(0);

/// A directory holding one `armada.yml`, gone when the test ends.
struct Repository {
    path: PathBuf,
}

impl Repository {
    fn holding(text: &str) -> Repository {
        let path = std::env::temp_dir().join(format!(
            "armada-live-{}-{}",
            std::process::id(),
            NEXT.fetch_add(1, Ordering::Relaxed)
        ));
        std::fs::create_dir_all(&path).expect("a temporary directory");
        let repository = Repository { path };
        std::fs::write(repository.manifest(), text).expect("the file");
        repository
    }

    fn manifest(&self) -> PathBuf {
        self.path.join("armada.yml")
    }

    fn save(&self, text: &str) {
        std::fs::write(self.manifest(), text).expect("the file");
    }

    /// Write beside the target and rename over it — **the save a great many
    /// editors do**, and the one that leaves an inode subscription watching a
    /// file nothing will write to again. Here it is only proving that a
    /// re-read reads the new bytes and not a cached inode.
    fn save_by_replacing(&self, text: &str) {
        let beside = self.path.join("armada.yml.tmp");
        std::fs::write(&beside, text).expect("the file beside it");
        std::fs::rename(&beside, self.manifest()).expect("the rename over it");
    }
}

impl Drop for Repository {
    fn drop(&mut self) {
        let _ = std::fs::remove_dir_all(&self.path);
    }
}

fn reloadable(path: &Path) -> (Manifest, crate::Reloads) {
    Manifest::reloadable(path).expect("it loads")
}

const PATIENT: &str = r#"
version: 1
id: armada
base: main
checks:
  build:
    run: cargo build --workspace
drone:
  quiet_after_seconds: 300
  poke_limit: 2
"#;

#[test]
fn an_edit_reaches_the_manifest_fleet_is_already_holding() {
    let repository = Repository::holding(PATIENT);
    let (manifest, reloads) = reloadable(&repository.manifest());
    assert_eq!(manifest.quiet_after_seconds(), Some(300));

    repository.save(&PATIENT.replace("quiet_after_seconds: 300", "quiet_after_seconds: 90"));
    let adopted = reloads.reread().expect("it reads");

    // The value moved on the Manifest handed out before the edit, which is the
    // whole of `#430`: Fleet holds that one and nothing hands it a second.
    assert_eq!(manifest.quiet_after_seconds(), Some(90));
    assert_eq!(manifest.poke_limit(), Some(2));
    assert_eq!(
        adopted.moved().iter().map(|m| m.key).collect::<Vec<_>>(),
        [LiveKey::QuietAfterSeconds]
    );
    // The value as `armada.yml` writes it, which is text for every live key
    // now that two of the four hold a word rather than a number.
    assert_eq!(adopted.moved()[0].before.as_deref(), Some("300"));
    assert_eq!(adopted.moved()[0].after.as_deref(), Some("90"));
    assert!(adopted.at_restart().is_empty());
    // The line a person reads on the daemon's console. It names the key as
    // `armada.yml` spells it, so it can be searched for in the file it is about.
    assert_eq!(
        adopted.moved()[0].to_string(),
        "drone.quiet_after_seconds 300 -> 90"
    );
}

#[test]
fn a_clone_of_the_manifest_sees_the_edit_too() {
    let repository = Repository::holding(PATIENT);
    let (manifest, reloads) = reloadable(&repository.manifest());
    // `Setup::into_parts` moves and `Fittings` moves again; a clone that saw a
    // stale number would be a reload reaching everything except Fleet.
    let held = manifest.clone();

    repository.save(&PATIENT.replace("poke_limit: 2", "poke_limit: 5"));
    reloads.reread().expect("it reads");

    assert_eq!(held.poke_limit(), Some(5));
}

#[test]
fn a_key_deleted_from_the_file_falls_back_rather_than_keeping_its_last_number() {
    let repository = Repository::holding(PATIENT);
    let (manifest, reloads) = reloadable(&repository.manifest());

    repository.save(&PATIENT.replace("  poke_limit: 2\n", ""));
    let adopted = reloads.reread().expect("it reads");

    // Absent is a repository deferring to what Fleet is running with. Keeping
    // the last number would make a deletion the one edit that cannot be made.
    assert_eq!(manifest.poke_limit(), None);
    assert_eq!(adopted.moved()[0].after, None);
    // Spelled, because a blank there reads as a value nobody wrote.
    assert_eq!(
        adopted.moved()[0].to_string(),
        "drone.poke_limit 2 -> unset"
    );
}

#[test]
fn a_rename_over_the_file_is_read_like_any_other_save() {
    let repository = Repository::holding(PATIENT);
    let (manifest, reloads) = reloadable(&repository.manifest());

    repository
        .save_by_replacing(&PATIENT.replace("quiet_after_seconds: 300", "quiet_after_seconds: 45"));
    reloads.reread().expect("it reads");

    assert_eq!(manifest.quiet_after_seconds(), Some(45));
}

#[test]
fn a_file_that_no_longer_parses_leaves_the_last_good_numbers_in_force() {
    let repository = Repository::holding(PATIENT);
    let (manifest, reloads) = reloadable(&repository.manifest());

    repository.save("version: 1\nid: armada\ndrone:\n  quiet_after_seconds: [not a number]\n");
    let why = reloads.reread().expect_err("a file that does not parse");

    assert_eq!(manifest.quiet_after_seconds(), Some(300));
    assert_eq!(manifest.poke_limit(), Some(2));
    // And it says which file, because the daemon's console is where this ends
    // up and a message naming no path is one nobody can act on.
    assert!(why.to_string().contains("armada.yml"), "{why}");
}

#[test]
fn a_value_out_of_range_leaves_the_last_good_numbers_in_force() {
    let repository = Repository::holding(PATIENT);
    let (manifest, reloads) = reloadable(&repository.manifest());

    // Zero is refused where it is written: a Drone quiet the instant it is
    // spawned is a sentence nobody means. It parses as YAML and is refused as a
    // value, which is the second of the two ways an edit can be wrong.
    repository.save(&PATIENT.replace("quiet_after_seconds: 300", "quiet_after_seconds: 0"));
    reloads
        .reread()
        .expect_err("a threshold of zero seconds is refused");

    assert_eq!(manifest.quiet_after_seconds(), Some(300));
}

#[test]
fn a_file_that_has_been_deleted_leaves_the_last_good_numbers_in_force() {
    let repository = Repository::holding(PATIENT);
    let (manifest, reloads) = reloadable(&repository.manifest());

    // What the middle of an atomic replace looks like if it is read too early,
    // and what a person deleting the file looks like. Neither is grounds for a
    // fleet to change its terms.
    std::fs::remove_file(repository.manifest()).expect("it is removed");
    reloads.reread().expect_err("a file that is not there");

    assert_eq!(manifest.quiet_after_seconds(), Some(300));
}

#[test]
fn a_changed_check_is_reported_and_not_adopted() {
    let repository = Repository::holding(PATIENT);
    let (manifest, reloads) = reloadable(&repository.manifest());

    repository.save(&PATIENT.replace(
        "checks:\n  build:",
        "checks:\n  lint:\n    run: cargo clippy\n  build:",
    ));
    let adopted = reloads.reread().expect("it reads");

    // Every workflow was resolved against the Checks the file declared at
    // start. Adopting a new registry here would leave a ResolvedWorkflow
    // pointing at a Check the Manifest no longer agrees with, and nothing would
    // fail until a step ran.
    assert_eq!(manifest.check_names(), ["build"]);
    assert_eq!(adopted.at_restart(), [Frozen::Checks]);
    assert!(adopted.moved().is_empty());
}

#[test]
fn a_save_that_changed_nothing_this_fleet_reads_says_nothing() {
    let repository = Repository::holding(PATIENT);
    let (_manifest, reloads) = reloadable(&repository.manifest());

    repository.save(&format!("# a comment somebody added\n{PATIENT}"));
    let adopted = reloads.reread().expect("it reads");

    assert!(adopted.is_quiet(), "{adopted:?}");
}

#[test]
fn a_manifest_loaded_the_ordinary_way_has_no_writer_anywhere() {
    // Not an assertion — a note about what the type system is doing. There is
    // no method on `Manifest` or on anything Fleet holds that produces a
    // `Reloads`, so this is the only way to get one and the only place it can
    // be kept. `Manifest::load` below compiles and yields nothing that can move
    // it.
    let repository = Repository::holding(PATIENT);
    let manifest = Manifest::load(&repository.manifest()).expect("it loads");
    assert_eq!(manifest.quiet_after_seconds(), Some(300));
}

/// **The third key in `drone:` is not live, and its neighbours are.** Every
/// `ResolvedWorkflow` took its copy of this list at boot, so adopting a new one
/// would leave a running Job fenced by something no record of it says. Reported
/// as a key rather than as the section, because saying `drone` would tell a
/// person that the `quiet_after_seconds` beside it also needs a restart.
#[test]
fn a_changed_fence_is_reported_as_a_key_and_the_patience_beside_it_still_moves() {
    let repository = Repository::holding(PATIENT);
    let (manifest, reloads) = reloadable(&repository.manifest());

    repository.save(
        &PATIENT
            .replace("quiet_after_seconds: 300", "quiet_after_seconds: 90")
            .replace(
                "  poke_limit: 2",
                "  poke_limit: 2\n  exclude_paths:\n    - vendor",
            ),
    );
    let adopted = reloads.reread().expect("it reads");

    assert!(manifest.exclude_paths().is_empty());
    assert_eq!(manifest.quiet_after_seconds(), Some(90));
    assert_eq!(adopted.at_restart(), [Frozen::DroneExcludePaths]);
    assert_eq!(
        adopted.at_restart()[0].as_str(),
        "drone.exclude_paths",
        "the key a person would search the file for, not the section it sits in"
    );
}

/// **The lever the whole tier exists for.** A Job refused for money is refused
/// again at every admission, so the number has to be movable while the Fleet
/// that is refusing it is running. A cap adopted only at restart would reach
/// every Job except the one that needs it.
#[test]
fn a_repositorys_cost_cap_moves_under_a_running_fleet() {
    let repository = Repository::holding(&format!("{PATIENT}  cost_cap_micros_per_job: 5000000\n"));
    let (manifest, reloads) = reloadable(&repository.manifest());
    assert_eq!(manifest.cost_cap_micros(), Some(5_000_000));

    repository.save(&format!("{PATIENT}  cost_cap_micros_per_job: 20000000\n"));
    let adopted = reloads.reread().expect("it reads");

    assert_eq!(manifest.cost_cap_micros(), Some(20_000_000));
    assert_eq!(
        adopted.moved().iter().map(|m| m.key).collect::<Vec<_>>(),
        [LiveKey::CostCapMicrosPerJob],
        "the two keys beside it did not move and are not reported as having"
    );
    assert_eq!(
        adopted.moved()[0].to_string(),
        "drone.cost_cap_micros_per_job 5000000 -> 20000000"
    );
}

/// A cap deleted is a repository handing its Jobs back to the machine's number,
/// which reads differently from any number — so the message spells it.
#[test]
fn a_cost_cap_deleted_from_the_file_is_reported_as_unset() {
    let repository = Repository::holding(&format!("{PATIENT}  cost_cap_micros_per_job: 5000000\n"));
    let (manifest, reloads) = reloadable(&repository.manifest());

    repository.save(PATIENT);
    let adopted = reloads.reread().expect("it reads");

    assert_eq!(manifest.cost_cap_micros(), None);
    assert_eq!(
        adopted.moved()[0].to_string(),
        "drone.cost_cap_micros_per_job 5000000 -> unset"
    );
}

/// **A person turning `auto_merge` off is answered on the next sweep, not on
/// the next restart.** That is the whole reason `crates/config/settings.toml`
/// files both policies `Live`: the moment somebody most wants to stop a machine
/// landing work is the moment a Job is about to land it, and asking them to
/// restart a daemon with Drones on it is not an answer.
///
/// The Manifest that moves is the one handed out before the edit, which is
/// `#430`'s own guarantee applied to two more keys.
#[test]
fn a_policy_moves_under_a_running_fleet_and_says_which_way() {
    let repository = Repository::holding(PATIENT);
    let (manifest, reloads) = reloadable(&repository.manifest());
    assert_eq!(manifest.auto_merge(), AutoMerge::Never);
    assert_eq!(manifest.review_gate(), ReviewGate::HumanAlways);

    repository.save(&format!(
        "{PATIENT}auto_merge: always\nreview_gate: auto_if_judge_passes\n"
    ));
    let adopted = reloads.reread().expect("it reads");

    assert_eq!(manifest.auto_merge(), AutoMerge::Always);
    assert_eq!(manifest.review_gate(), ReviewGate::AutoIfJudgePasses);
    assert_eq!(
        adopted.moved().iter().map(|m| m.key).collect::<Vec<_>>(),
        [LiveKey::AutoMerge, LiveKey::ReviewGate]
    );
    // **Neither end is `unset`.** A policy has no tier above it to defer to, so
    // an absent key is its own default and the sentence names the value that
    // was actually in force rather than a blank.
    assert_eq!(
        adopted.moved()[0].to_string(),
        "auto_merge never -> always",
        "the key undotted, because that is where `armada.yml` writes it and what \
         a `manifest_rule:` gate names"
    );
    assert_eq!(
        adopted.moved()[1].to_string(),
        "review_gate human_always -> auto_if_judge_passes"
    );
    assert!(adopted.at_restart().is_empty());
}

/// **Deleting the key puts the cautious value back**, and says so, rather than
/// leaving the last word in force. The other direction is the dangerous one: a
/// repository that removes `auto_merge: always` has stopped asking for it.
#[test]
fn a_deleted_policy_falls_back_to_the_cautious_value_rather_than_keeping_the_last_one() {
    let loose = format!("{PATIENT}auto_merge: always\n");
    let repository = Repository::holding(&loose);
    let (manifest, reloads) = reloadable(&repository.manifest());
    assert_eq!(manifest.auto_merge(), AutoMerge::Always);

    repository.save(PATIENT);
    let adopted = reloads.reread().expect("it reads");

    assert_eq!(manifest.auto_merge(), AutoMerge::Never);
    assert_eq!(adopted.moved()[0].to_string(), "auto_merge always -> never");
}

/// A file that no longer parses moves neither policy, which is
/// [`Reloads::reread`]'s standing rule seen from the one key where getting it
/// wrong lands code: a mistyped `poke_limit` must not take `auto_merge: never`
/// down with it.
#[test]
fn a_refused_file_leaves_the_policy_in_force() {
    let repository = Repository::holding(&format!("{PATIENT}auto_merge: checks-pass\n"));
    let (manifest, reloads) = reloadable(&repository.manifest());

    repository.save(&format!(
        "{PATIENT}auto_merge: checks-pass\npoke_limit: 4\n"
    ));
    reloads
        .reread()
        .expect_err("a key at the top level nothing reads");

    assert_eq!(manifest.auto_merge(), AutoMerge::ChecksPass);
}

/// **Five live keys move in one save, and each is reported on its own.** The
/// two the cell holds apart — `drone:`'s dials and the top-level policies —
/// are one adoption and one instant, which is what one cell rather than two
/// buys; the list a person reads is still key by key.
#[test]
fn every_live_key_moving_at_once_is_still_five_sentences() {
    let repository = Repository::holding(&format!(
        "{PATIENT}  cost_cap_micros_per_job: 5000000\nauto_merge: never\nreview_gate: human_always\n"
    ));
    let (manifest, reloads) = reloadable(&repository.manifest());

    repository.save(
        &format!("{PATIENT}  cost_cap_micros_per_job: 9000000\nauto_merge: always\nreview_gate: auto_if_judge_passes\n")
            .replace("quiet_after_seconds: 300", "quiet_after_seconds: 90")
            .replace("poke_limit: 2", "poke_limit: 4"),
    );
    let adopted = reloads.reread().expect("it reads");

    assert_eq!(
        adopted
            .moved()
            .iter()
            .map(|moved| moved.to_string())
            .collect::<Vec<_>>(),
        [
            "drone.quiet_after_seconds 300 -> 90",
            "drone.poke_limit 2 -> 4",
            "drone.cost_cap_micros_per_job 5000000 -> 9000000",
            "auto_merge never -> always",
            "review_gate human_always -> auto_if_judge_passes",
        ]
    );
    assert!(adopted.at_restart().is_empty());
    assert_eq!(manifest.cost_cap_micros(), Some(9_000_000));
    assert_eq!(manifest.auto_merge(), AutoMerge::Always);
}
