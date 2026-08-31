//! A note written where there was no Drone to take it, and the one boundary it
//! survives.
//!
//! **The claim is that the record can hold a person's words and lose them
//! exactly once.** `note.rs` says why the field is on the Job rather than on a
//! step; these are the three properties that follow — it waits, it is cleared
//! by delivering it, and a second one over an undelivered first is refused
//! rather than silently dropping either.

use super::*;

/// A Job holds nothing until somebody says something to it with no Drone
/// there, and the note it then holds is exactly the words that were written.
#[test]
fn a_note_waits_on_the_record_and_is_cleared_by_delivering_it() {
    let job = created();
    assert!(
        job.redirect_waiting().is_none(),
        "nothing has been said to a Job nobody has looked at"
    );

    let note = RedirectWaiting::saying("name the cause, not the symptom").expect("a note");
    let waiting = job.redirect_waits(note).expect("nothing was waiting");
    assert_eq!(
        waiting.redirect_waiting().map(RedirectWaiting::text),
        Some("name the cause, not the symptom"),
        "the person's own words, not a summary of them"
    );
    assert!(
        job.redirect_waiting().is_none(),
        "and the Job it was taken from is unchanged: no method here takes &mut self"
    );

    let delivered = waiting.redirect_delivered();
    assert!(
        delivered.redirect_waiting().is_none(),
        "it waits for the next Drone and for no Drone after that"
    );
    assert!(
        delivered.redirect_delivered().redirect_waiting().is_none(),
        "and clearing what was already clear is the ordinary case, not a fault"
    );
}

/// **A second note over an undelivered first is refused**, which is the
/// deliberate answer rather than the fallen-into one: overwriting drops the
/// first silently, and a queue is the expiring backlog the waiting rule was
/// chosen to prevent. The refusal carries what is already held, so the person
/// gets both sets of words back.
#[test]
fn a_second_note_is_refused_rather_than_losing_either_of_the_two() {
    let waiting = created()
        .redirect_waits(RedirectWaiting::saying("name the cause").expect("a note"))
        .expect("nothing was waiting");

    let refused = waiting
        .redirect_waits(RedirectWaiting::saying("and check the writer too").expect("a note"))
        .expect_err("one is already waiting");
    assert_eq!(refused.held.text(), "name the cause");
    assert!(
        refused.to_string().contains("name the cause"),
        "the refusal says what is already there: {refused}"
    );
    assert_eq!(
        waiting.redirect_waiting().map(RedirectWaiting::text),
        Some("name the cause"),
        "and the first note is still the one waiting"
    );

    // Delivered, the same Job takes the second note.
    let next = waiting
        .redirect_delivered()
        .redirect_waits(RedirectWaiting::saying("and check the writer too").expect("a note"))
        .expect("nothing is waiting any more");
    assert_eq!(
        next.redirect_waiting().map(RedirectWaiting::text),
        Some("and check the writer too")
    );
}

/// **A note that says nothing is unrepresentable**, for the reason
/// `fleet::resume::Redirection` is: a Drone told nothing at all would open with
/// a heading and a blank under it.
#[test]
fn a_blank_note_is_not_a_note() {
    assert!(RedirectWaiting::saying("").is_none());
    assert!(RedirectWaiting::saying("   \n\t ").is_none());
    assert_eq!(
        RedirectWaiting::saying("  do the writer too  ").map(|note| note.text().to_string()),
        Some(String::from("do the writer too")),
        "surrounding space is trimmed rather than carried into a brief"
    );
}
