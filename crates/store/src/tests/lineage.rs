//! Reading a redispatch from both ends, off the one column that records it.
//!
//! The claim worth breaking is that there is no second record: every answer
//! here comes from a replacement's own `redispatched_from`, so forgetting
//! either end has to take the answer with it.

use core_model::{Job, JobId, NewJob, TopLevelOrigin};

use crate::tests::{at, full_new_job, job_id, open, top_level, TempDir};
use crate::Store;

/// A Job replacing `replaced`, created at `when` so creation order is stated
/// rather than implied.
fn replacing(store: &mut Store, id: &str, replaced: &str, when: &str) -> JobId {
    let new = NewJob {
        redispatched_from: Some(job_id(replaced)),
        ..full_new_job(id)
    };
    let job = Job::create_top_level(new, TopLevelOrigin::Manual, at(when));
    store
        .insert_job(&job, &at(when))
        .expect("the replacement is stored");
    job.id().clone()
}

fn a_job(store: &mut Store, id: &str) {
    let job = top_level(id);
    store
        .insert_job(&job, &crate::tests::created_at())
        .expect("the job is stored");
}

#[test]
fn a_job_nothing_replaced_has_no_replacement() {
    let dir = TempDir::new();
    let mut store = open(&dir);
    a_job(&mut store, "01ALONE");

    let found = store
        .replaced_by(&job_id("01ALONE"))
        .expect("the read runs");

    assert_eq!(found, None, "nothing named this job as its predecessor");
}

#[test]
fn a_replaced_job_names_the_job_that_replaced_it() {
    let dir = TempDir::new();
    let mut store = open(&dir);
    a_job(&mut store, "01FAILED");
    let replacement = replacing(
        &mut store,
        "01AGAIN",
        "01FAILED",
        "2026-09-17T10:00:00.000Z",
    );

    let found = store
        .replaced_by(&job_id("01FAILED"))
        .expect("the read runs")
        .expect("a replacement was minted");

    assert_eq!(found.job_id, replacement);
}

/// A chain reads a hop at a time: the first Job names the second and never the
/// third, which is what makes a loop impossible — there is no walk here.
#[test]
fn a_chain_names_the_next_job_and_never_the_last() {
    let dir = TempDir::new();
    let mut store = open(&dir);
    a_job(&mut store, "01FIRST");
    let second = replacing(
        &mut store,
        "01SECOND",
        "01FIRST",
        "2026-09-17T10:00:00.000Z",
    );
    let third = replacing(
        &mut store,
        "01THIRD",
        "01SECOND",
        "2026-09-17T11:00:00.000Z",
    );

    let after_first = store
        .replaced_by(&job_id("01FIRST"))
        .expect("the read runs")
        .expect("the first was replaced");
    let after_second = store
        .replaced_by(&job_id("01SECOND"))
        .expect("the read runs")
        .expect("the second was replaced too");

    assert_eq!(after_first.job_id, second);
    assert_eq!(after_second.job_id, third);
    assert_eq!(
        store.replaced_by(&third).expect("the read runs"),
        None,
        "nothing has replaced the newest yet"
    );
}

/// Two Jobs may name one predecessor — a `killed` Job is redispatchable and a
/// redispatch leaves one `killed`, and nothing in Fleet refuses the second.
/// The later press is the answer.
#[test]
fn two_replacements_of_one_job_answer_with_the_newer() {
    let dir = TempDir::new();
    let mut store = open(&dir);
    a_job(&mut store, "01TWICE");
    replacing(
        &mut store,
        "01EARLIER",
        "01TWICE",
        "2026-09-17T10:00:00.000Z",
    );
    let later = replacing(&mut store, "01LATER", "01TWICE", "2026-09-17T12:00:00.000Z");

    let found = store
        .replaced_by(&job_id("01TWICE"))
        .expect("the read runs")
        .expect("a replacement was minted");

    assert_eq!(found.job_id, later);
}

/// **The link is the replacement's row, so forgetting it takes the link.** A
/// second column would have left the original pointing at a Job that is gone.
#[test]
fn forgetting_the_replacement_leaves_the_original_reading_as_merely_stopped() {
    let dir = TempDir::new();
    let mut store = open(&dir);
    a_job(&mut store, "01ORIGINAL");
    let replacement = replacing(
        &mut store,
        "01REPLACEMENT",
        "01ORIGINAL",
        "2026-09-17T10:00:00.000Z",
    );

    store.forget_job(&replacement).expect("it is forgotten");

    assert_eq!(
        store
            .replaced_by(&job_id("01ORIGINAL"))
            .expect("the read runs"),
        None,
    );
}

/// A sweep gives back a worktree and keeps the record, so the link stands.
#[test]
fn clearing_the_replacements_worktree_keeps_the_link() {
    let dir = TempDir::new();
    let mut store = open(&dir);
    a_job(&mut store, "01KEPT");
    let replacement = replacing(&mut store, "01SWEPT", "01KEPT", "2026-09-17T10:00:00.000Z");

    store
        .retain_job(&replacement, &at("2026-09-18T00:00:00.000Z"))
        .expect("the worktree is given back");

    assert_eq!(
        store
            .replaced_by(&job_id("01KEPT"))
            .expect("the read runs")
            .map(|found| found.job_id),
        Some(replacement),
    );
}

/// The number and the title come back as the record holds them, which is what
/// lets Fleet compose the one handle a person reads.
#[test]
fn the_replacement_comes_back_with_the_number_and_title_a_handle_is_made_of() {
    let dir = TempDir::new();
    let mut store = open(&dir);
    a_job(&mut store, "01NAMED");
    replacing(
        &mut store,
        "01NAMING",
        "01NAMED",
        "2026-09-17T10:00:00.000Z",
    );

    let found = store
        .replaced_by(&job_id("01NAMED"))
        .expect("the read runs")
        .expect("a replacement was minted");

    assert_eq!(
        found.title.as_str(),
        full_new_job("01NAMING").title.as_str()
    );
    assert!(found.number.get() > 0, "every job is numbered");
}

/// The forward read: a Job minted by a redispatch names what it replaced.
#[test]
fn a_redispatched_job_names_the_job_it_replaced() {
    let dir = TempDir::new();
    let mut store = open(&dir);
    a_job(&mut store, "01ORIGIN");
    let replacement = replacing(&mut store, "01HEIR", "01ORIGIN", "2026-09-17T10:00:00.000Z");

    let found = store
        .replaces(&replacement)
        .expect("the read runs")
        .expect("this job was minted by a redispatch");

    assert_eq!(found.job_id, job_id("01ORIGIN"));
}

/// Most Jobs. **The absence is the record**, not a missing row: nothing
/// redispatched this one, so the column is null and the join finds nothing.
#[test]
fn a_job_no_redispatch_minted_replaces_nothing() {
    let dir = TempDir::new();
    let mut store = open(&dir);
    a_job(&mut store, "01FIRSTEVER");

    assert_eq!(
        store
            .replaces(&job_id("01FIRSTEVER"))
            .expect("the read runs"),
        None,
    );
}

/// One hop backwards as well as forwards: the third names the second and never
/// the first, so neither read can go round a chain.
#[test]
fn the_backward_read_is_one_hop_and_never_the_root() {
    let dir = TempDir::new();
    let mut store = open(&dir);
    a_job(&mut store, "01ROOT");
    let second = replacing(&mut store, "01MIDDLE", "01ROOT", "2026-09-17T10:00:00.000Z");
    let third = replacing(&mut store, "01LEAF", "01MIDDLE", "2026-09-17T11:00:00.000Z");

    assert_eq!(
        store
            .replaces(&third)
            .expect("the read runs")
            .map(|found| found.job_id),
        Some(second.clone()),
    );
    assert_eq!(
        store
            .replaces(&second)
            .expect("the read runs")
            .map(|found| found.job_id),
        Some(job_id("01ROOT")),
    );
}

/// **The predecessor's row is the name, so forgetting it takes the name.** The
/// id stays on the replacement and points at nothing, and this read says so
/// with `None` rather than handing back a Job that is gone.
#[test]
fn forgetting_the_predecessor_leaves_the_replacement_naming_nobody() {
    let dir = TempDir::new();
    let mut store = open(&dir);
    a_job(&mut store, "01GONE");
    let replacement = replacing(
        &mut store,
        "01STILLHERE",
        "01GONE",
        "2026-09-17T10:00:00.000Z",
    );

    store
        .forget_job(&job_id("01GONE"))
        .expect("it is forgotten");

    assert_eq!(store.replaces(&replacement).expect("the read runs"), None);
}

/// The number and the title come back as the record holds them, which is what
/// lets Fleet compose the one handle a person reads — `replaced_by`'s rule,
/// read the other way.
#[test]
fn the_predecessor_comes_back_with_the_number_and_title_a_handle_is_made_of() {
    let dir = TempDir::new();
    let mut store = open(&dir);
    let named = full_new_job("01CALLED");
    let earlier = Job::create_top_level(
        named,
        TopLevelOrigin::Manual,
        at("2026-09-17T09:00:00.000Z"),
    );
    store
        .insert_job(&earlier, &at("2026-09-17T09:00:00.000Z"))
        .expect("the earlier job is stored");
    let replacement = replacing(
        &mut store,
        "01CALLING",
        "01CALLED",
        "2026-09-17T10:00:00.000Z",
    );

    let found = store
        .replaces(&replacement)
        .expect("the read runs")
        .expect("this job was minted by a redispatch");

    assert_eq!(
        found.title.as_str(),
        full_new_job("01CALLED").title.as_str()
    );
    assert!(found.number.get() > 0, "every job is numbered");
}
