//! A person's note with nowhere to go yet, and the one boundary it survives.
//!
//! # Why the record holds it at all
//!
//! A redirect is a turn injected into a live session and needs no record: the
//! process reads it and it is gone. A Drone belongs to a workflow step now, so
//! a Job at a human advance gate has no process, and the note a person writes
//! there is addressed to a Drone that does not exist yet. It is written down or
//! it is refused, and refusing it is what `awaiting_review` did until this.
//!
//! # It waits for the next Drone and for no Drone after that
//!
//! The owner's ruling of 31 Aug 2026: rendered into the very next opening brief
//! and cleared there, whether or not that Drone acts on it. What that avoids is
//! a note about part two surfacing during part four as advice about finished
//! work — which reads as Armada being confused rather than as a person having
//! changed their mind, and is worse than losing the note.
//!
//! **That lifetime is why the field is on the Job and not on a step.** "The
//! next Drone" is a fact about the Job; "the next Drone on step three" is
//! longer-lived, and a note keyed to a step the Job then overrode past would
//! sit there until that step ran again.

use alloc::string::{String, ToString};
use core::fmt;

/// A person's note, held on the record until a Drone opens with it.
///
/// **Never empty**, for `fleet::resume::Redirection`'s reason: a note that says
/// nothing is a poke, the poke is a different turn with its own wording, and a
/// Drone opened with a blank block has been given a heading with nothing under
/// it.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct RedirectWaiting(String);

impl RedirectWaiting {
    /// `None` where there is nothing in it a Drone could act on.
    pub fn saying(note: &str) -> Option<RedirectWaiting> {
        let said = note.trim();
        (!said.is_empty()).then(|| RedirectWaiting(said.to_string()))
    }

    pub fn text(&self) -> &str {
        &self.0
    }
}

/// A second note arrived while the first was still waiting.
///
/// **Refused rather than overwritten, and refused rather than queued.** Both
/// of the other two answers lose something a person typed: last-one-wins drops
/// the first silently, and a queue is the expiring backlog the waiting rule was
/// chosen to prevent. So the second act fails and says a note is already
/// waiting, which is the one answer that leaves the person holding their own
/// words.
///
/// **Nothing in Fleet can reach it today**, and it is here anyway. The only
/// writer is `request_changes`, which runs under the slot lock and takes the
/// Job out of `awaiting_review` in the same call — so a second request refuses
/// as `NotUnderReview` before it gets here. That is a property of one caller's
/// ordering, not of the record, and a record that leaned on it would lose a
/// note the first time a second writer appeared.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct RedirectAlreadyWaiting {
    /// What is already on the record. Carried so the refusal can be read
    /// without a second load.
    pub held: RedirectWaiting,
}

impl fmt::Display for RedirectAlreadyWaiting {
    fn fmt(&self, out: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(
            out,
            "a note is already waiting for the next Drone and has not been \
             delivered yet: \"{}\"",
            self.held.text()
        )
    }
}
