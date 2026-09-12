//! A person saying no to a command, and saying why.
//!
//! What these prove: a reject carrying a person's words puts them inside the
//! refusal the Drone reads, with Fleet's own sentence unchanged and the words
//! attributed to the person; a reject carrying none is the refusal it has
//! always been, character for character; a field somebody opened and typed
//! nothing into is no note; and nothing sent beside an allow can become one.
//!
//! **The first is an integration case and the rest are not.** The path is the
//! claim for the first — the words go down the tool call the Drone is still
//! holding open, which is where a promptly answered reject is answered — and
//! the others are about the text, which is composed without a Fleet.
//!
//! The fixture is `crate::tests::permitting`'s. A second Fleet built here would
//! be a second answer to what a Job waiting on a command is.

use adapter_traits::DroneEvent;
use api::PermissionAnswer;
use core_model::{Reach, WhenBlocked};
use ipc::CommandAnswer;

use crate::permitting::{Answered, Note, Permitted, Refusing};
use crate::tests::permitting::{
    a_drone_that_reached_for, a_fleet_with, asked, heard, started, until_waiting,
};
use crate::tests::tmp::TempDir;

/// The refusal every Fleet before 11.5 sent, written out rather than composed.
///
/// **A second copy of the sentence on purpose.** What it pins is that the old
/// text did not move when the note was added to it, and a copy derived from the
/// same function would move along with it and pin nothing.
const THE_BARE_REFUSAL: &str = "A person said no to `npm publish`. Do not run it, or anything \
                                that does the same thing. Carry on without it if the task \
                                allows, or ask a question if it cannot be done without it.";

/// **The reason a person had at the moment they pressed reject.** It reaches the
/// Drone inside the call it is still holding open, under Fleet's own refusal
/// rather than instead of it — and the refusal the fold records carries the same
/// words, so the Job's record says what was said as well as that somebody said
/// no.
#[tokio::test]
async fn a_persons_reason_reaches_the_drone_inside_the_refusal() {
    const BECAUSE: &str = "we publish from CI, never from a task";
    let home = TempDir::new();
    let fleet = a_fleet_with(&home, a_drone_that_reached_for("c1"));
    let job = started(&fleet, &home).await;
    fleet
        .set_when_blocked(&job, WhenBlocked::AskMe)
        .await
        .unwrap();
    let asking = asked("Bash", "npm publish", "c1");

    let (answer, answered) = tokio::join!(fleet.permission(&job, &asking), async {
        until_waiting(&fleet, &job).await;
        fleet
            .answer_command(
                &job,
                "c1",
                Answered::of(CommandAnswer::Reject, Some(BECAUSE)),
            )
            .await
    });

    answered.expect("the call is waiting, and reject is one of the answers it offers");
    let PermissionAnswer::Deny(words) = answer else {
        panic!("a rejection is a deny: {answer:?}");
    };
    assert!(
        words.contains(THE_BARE_REFUSAL),
        "Fleet's own sentence is unchanged by the note: {words}"
    );
    assert!(
        words.contains(BECAUSE),
        "and the person's own words are in it: {words}"
    );
    assert!(
        words.contains("in their own words"),
        "attributed to the person, so a Drone does not read one person's reason \
         as a standing rule: {words}"
    );
    let because = because_of(&heard(&fleet).await, "c1").expect("the fold sees Fleet's refusal");
    assert!(
        because.contains(BECAUSE),
        "and the record carries the same words: {because}"
    );
}

/// The reason on the `refused` row the fold sees, where there is one.
fn because_of(heard: &[DroneEvent], call: &str) -> Option<String> {
    heard.iter().find_map(|event| match event {
        DroneEvent::Refused {
            call: refused,
            because,
            ..
        } if refused == call => Some(because.clone()),
        _ => None,
    })
}

/// **A reject with nothing to say is the refusal it always was.** The note is
/// additive, or it is a change to every refusal Armada has ever sent — and both
/// paths a reject takes say the same thing, the held call's and the turn's.
#[test]
fn a_reject_with_no_note_is_the_refusal_it_always_was() {
    assert_eq!(
        Refusing::Rejected { note: None }.to_the_drone("npm publish"),
        THE_BARE_REFUSAL
    );
    assert_eq!(
        Permitted::rejected("npm publish", None).text(),
        THE_BARE_REFUSAL,
        "the turn a late answer becomes is the same sentence"
    );
}

/// The other path: the hold ran out first, so the answer arrives as a turn —
/// and it carries the words the held call would have carried.
#[test]
fn a_reject_answered_after_the_hold_ended_carries_the_note_too() {
    let note = Note::saying("we publish from CI").expect("a note with something in it");
    let told = Permitted::rejected("npm publish", Some(&note));
    assert!(told.text().starts_with(THE_BARE_REFUSAL));
    assert!(told.text().contains("we publish from CI"));
}

/// **A field somebody opened and typed nothing into is no note at all**, which
/// is `redirect_drone`'s rule about a blank note rather than a second one: a
/// heading with nothing under it is exactly the information that was not enough.
#[test]
fn a_blank_note_is_no_note_at_all() {
    assert_eq!(Note::saying(""), None);
    assert_eq!(Note::saying("  \n\t "), None);
    assert_eq!(
        Answered::of(CommandAnswer::Reject, Some("   ")),
        Answered::Rejected(None),
        "so a blank note is the bare refusal and never an empty heading"
    );
    assert_eq!(
        Note::saying("  it pushes  ").map(|note| note.text().to_string()),
        Some("it pushes".to_string()),
        "and the whitespace around a real one is keystrokes rather than reason"
    );
}

/// **Nothing sent beside an allow can become a note, and the type is what says
/// so**: `Answered::Allowed` has no field for one, so a note on an allow stops
/// at the constructor that reads the wire rather than being carried down and
/// ignored somewhere further in.
#[test]
fn a_note_sent_with_an_allow_stops_where_the_answer_is_read() {
    assert_eq!(
        Answered::of(CommandAnswer::AllowForJob, Some("because I said so")),
        Answered::Allowed(Reach::Job)
    );
    assert_eq!(
        Answered::of(CommandAnswer::AlwaysAllow, Some("because I said so")),
        Answered::Allowed(Reach::Repository)
    );
}
