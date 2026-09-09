//! The three forms a person can say, and the one that needs a Manifest.
//!
//! The claim worth breaking is that they are one answer: a Job's ULID, its
//! whole handle and its bare number all resolve to the same `JobId`, and none
//! of the three resolves to a different Job. The second claim is the refusal —
//! a number outside a Manifest names no one Job and is answered as nothing
//! rather than as whichever row came first.

use core_model::{JobReference, ManifestId, Title};

use crate::tests::{created_at, full_new_job, open, title, top_level, ulid, TempDir};
use crate::{ResolveJobError, Store};

fn said(text: &str) -> JobReference {
    JobReference::read(text).unwrap_or_else(|| panic!("`{text}` reads as a reference"))
}

/// One Job in the store, given a title whose handle is worth pasting.
fn a_job(store: &mut Store, id: &str, called: &str) -> core_model::Job {
    let mut new = full_new_job(id);
    new.title = title(called);
    new.number = store
        .next_job_number(&new.owner_manifest_id)
        .expect("the next number");
    let job = core_model::Job::create_top_level(
        new,
        core_model::TopLevelOrigin::HelmDrafted,
        created_at(),
    );
    store.insert_job(&job, &created_at()).expect("it is stored");
    job
}

/// The Manifest every fixture Job belongs to — `full_new_job`'s own.
fn owner() -> ManifestId {
    ManifestId::carried(ulid("01OWNERMANIFEST"))
}

// The defect, in one test: the owner pasted a handle out of Bridge and nothing
// could look it up.
#[test]
fn the_three_forms_are_one_job() {
    let dir = TempDir::new();
    let mut store = open(&dir);
    let job = a_job(
        &mut store,
        "01M22TYSAE0023MADDP5ZQEYGW",
        "Board's clear button should reclaim worktrees",
    );
    let number = job.number().to_string();

    let by_id = store
        .resolve_job(&said(job.id().as_str()), Some(&owner()))
        .expect("the ULID resolves");
    let by_handle = store
        .resolve_job(&said(&job.handle()), Some(&owner()))
        .expect("the handle resolves");
    let by_number = store
        .resolve_job(&said(&number), Some(&owner()))
        .expect("the number resolves");

    assert_eq!(&by_id.job_id, job.id());
    assert_eq!(by_handle, by_id);
    assert_eq!(by_number, by_id);
    assert_eq!(by_id.handle, job.handle());
}

// **The refusal that stays a refusal.** A resolver that picks one of the Jobs
// numbered 1 is worse than one that will not.
#[test]
fn a_number_outside_a_manifest_is_refused_rather_than_guessed() {
    let dir = TempDir::new();
    let mut store = open(&dir);
    let job = a_job(&mut store, "01M22NUMBERONLY", "Say why a Job is waiting");

    let refused = store
        .resolve_job(&said(&job.number().to_string()), None)
        .expect_err("a number needs a Manifest");

    assert!(
        matches!(refused, ResolveJobError::NoManifest { .. }),
        "{refused:?}"
    );
    assert!(refused.to_string().contains("Manifest"), "{refused}");

    // And the id, which is unique across every Manifest, still resolves with
    // none — the refusal is about the number, not about the caller.
    assert!(store.resolve_job(&said(job.id().as_str()), None).is_ok());
}

#[test]
fn a_number_names_a_job_of_its_own_manifest_and_no_other() {
    let dir = TempDir::new();
    let mut store = open(&dir);
    let mine = a_job(&mut store, "01M22MINE", "Reclaim the worktree");
    let mut theirs = full_new_job("01M22THEIRS");
    theirs.owner_manifest_id = ManifestId::carried(ulid("01SOMEOTHERMANIFEST"));
    theirs.number = mine.number();
    theirs.title = Title::new("Some other repository's job one").expect("a title");
    let theirs = core_model::Job::create_top_level(
        theirs,
        core_model::TopLevelOrigin::HelmDrafted,
        created_at(),
    );
    store
        .insert_job(&theirs, &created_at())
        .expect("the other Manifest's Job is stored");

    let found = store
        .resolve_job(&said(&mine.number().to_string()), Some(&owner()))
        .expect("the number resolves within its own Manifest");

    assert_eq!(&found.job_id, mine.id());
}

// A handle's number finds the row and its words say whether the row is the one
// that was named. Pasted out of some other repository, the words disagree.
#[test]
fn a_handle_whose_words_are_not_this_jobs_names_nothing() {
    let dir = TempDir::new();
    let mut store = open(&dir);
    let job = a_job(&mut store, "01M22WORDS", "The drone count is wrong");

    let refused = store
        .resolve_job(
            &said(&format!("{}-something-else-entirely", job.number())),
            Some(&owner()),
        )
        .expect_err("the words are not this Job's");

    assert!(
        matches!(refused, ResolveJobError::NoSuchJob { .. }),
        "{refused:?}"
    );
}

#[test]
fn an_id_nothing_holds_is_refused_rather_than_carried() {
    let dir = TempDir::new();
    let mut store = open(&dir);
    a_job(&mut store, "01M22HELD", "A Job that is here");

    for named in ["01M22NOTHINGHOLDSTHIS", "4098"] {
        let refused = store
            .resolve_job(&said(named), Some(&owner()))
            .expect_err("nothing answers to it");
        assert!(
            matches!(refused, ResolveJobError::NoSuchJob { .. }),
            "{named}: {refused:?}"
        );
    }
}

// A title naming a credential is carried by its number alone, so its handle
// *is* a number — and the form a person reads is the form that resolves.
#[test]
fn a_job_whose_handle_is_only_a_number_resolves_by_it() {
    let dir = TempDir::new();
    let mut store = open(&dir);
    let job = a_job(
        &mut store,
        "01M22SECRETIVE",
        "set AWS_SECRET_ACCESS_KEY and go",
    );

    let found = store
        .resolve_job(&said(&job.handle()), Some(&owner()))
        .expect("its handle resolves");

    assert_eq!(&found.job_id, job.id());
    assert_eq!(found.handle, job.number().to_string());
}

// Nothing here is a second key: resolving reads the same row `load_job` does.
#[test]
fn what_resolves_is_the_row_the_ulid_loads() {
    let dir = TempDir::new();
    let mut store = open(&dir);
    let job = top_level("01M22SAMEROW");
    store.insert_job(&job, &created_at()).expect("it is stored");

    let found = store
        .resolve_job(&said(&job.handle()), Some(&owner()))
        .expect("the handle resolves");

    assert_eq!(
        store.load_job(&found.job_id).expect("it loads").id(),
        job.id()
    );
}
