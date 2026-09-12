//! What is outstanding: the question the Drone asked, and the redirect a
//! person sent it.
//!
//! **Two directions of one fact.** A question outstanding is Fleet owing the
//! Drone an answer; a redirect that has gone down the pipe is the Drone owing
//! Fleet proof that it heard. Neither is on the record and both are held here,
//! for the reason the `asked` field gives: each is only ever true now, and a
//! Fleet that restarted lost the process the answer would have gone to.
//!
//! What a person's word lands as is here too — [`Working::resumed`] where a
//! step had stopped, [`Working::steered`] where it never did — because which
//! clocks start again is the whole difference between the two.
//! `crate::questioning` and `crate::resume` are the readers, and what to do
//! about an answer that never comes is theirs.

use std::time::Duration;

use core_model::Timestamp;

use tokio::sync::oneshot;

use crate::converging::Chain;
use crate::permitting::{Answered, Waiting};
use crate::questioning::Question;
use crate::working::Working;

/// A redirect that has gone into the session and has not been answered: what
/// [`Progress::turned`](crate::Progress::turned) read the moment before it went
/// down the pipe, and when it did.
///
/// **One value because they are one act.** Held apart, a redirect could be
/// outstanding with no instant to serve, which is the reading
/// `JobDetail.redirecting` must not have.
pub(super) struct Awaiting {
    turned: usize,
    sent_at: Timestamp,
}

impl Working {
    /// The **stopped** step this Drone is on has been handed back by a person.
    ///
    /// **What starts again is what a stopped step stopped spending.** The wall
    /// clock and the tool-call count are readings of work being done, and none
    /// was being done: the step was frozen at the escalation until somebody
    /// spoke. A Drone that never stopped takes
    /// [`steered`](Working::steered) instead, and moves neither.
    ///
    /// **The step's baseline does not move, and nor does the declaration.** A
    /// redirect is the same step being done again, so what it entered with is
    /// still what it entered with — remeasuring here would hand the step
    /// whatever it had written before the person spoke, and it would then pass
    /// `diff_nonempty` on nothing.
    pub(crate) fn resumed(&mut self, at: Timestamp) {
        self.step_began = at.clone();
        self.calls_before = self.transcript.progress().calls;
        // **The dry runs do not go back.** The pokes are patience and a person
        // has just spent some of theirs; a Check run is minutes of a machine,
        // and a redirect is not a refund. `checked_for` is cleared only because
        // `step_began` moved above, so the time it accounted for is already
        // outside the window.
        self.checked_for = Duration::ZERO;
        self.steered(at);
    }

    /// A person has spoken to a Drone that **never stopped**. The healthy half
    /// of [`resumed`](Working::resumed), and the whole difference is that no
    /// clock moves here.
    ///
    /// **The step's ceilings go on meaning what they meant.** Nothing bounds
    /// how often a person may redirect — `crate::resume` says so — so a wall
    /// clock or a call count refilled by being spoken to is no ceiling at all,
    /// and a Drone could be held past every one of them by somebody typing at
    /// it. The work they count was never interrupted; only its direction was.
    ///
    /// **The chain and the pokes do go back**, because neither is a budget the
    /// step spends: a Drone steered off one loop can thrash into the next and
    /// has to be caught there, and a person who has just spoken has spent their
    /// own patience rather than the step's.
    pub(crate) fn steered(&mut self, at: Timestamp) {
        self.chain = Chain::Working;
        self.listening(at);
    }

    /// Start the silence clock again, and give the step its pokes back.
    ///
    /// **A new step, or the same step handed back by a person.** Both are
    /// moments the Drone has just been given something to do, and neither owes
    /// an answer for the time before it.
    fn listening(&mut self, at: Timestamp) {
        self.waiting(at);
        self.pokes = 0;
    }

    /// The question this Drone is waiting on, where there is one.
    pub(crate) fn asked(&self) -> Option<&Question> {
        self.asked.as_ref()
    }

    /// Hold the question the Drone just asked.
    ///
    /// **Nothing here checks that none is held.** `Fleet::ask_question` does,
    /// under this slot's lock and before it mints an id, because the refusal it
    /// answers with names the question already outstanding — which is a value
    /// this method has no way to return.
    pub(crate) fn asks(&mut self, asked: Question) {
        self.asked = Some(asked);
    }

    /// The question has been answered and the answer is down the pipe.
    ///
    /// **After the write, never before.** A session that would not take the
    /// answer leaves the question standing, so a person can answer again rather
    /// than being told there is nothing to answer on a Drone that never heard.
    pub(crate) fn answered_question(&mut self) {
        self.asked = None;
    }

    /// The permission question a person has been asked on this Drone's
    /// behalf, where there is one.
    pub(crate) fn permission(&self) -> Option<&Waiting> {
        self.permission.as_ref()
    }

    /// Hold a permission question for a person. **One at a time**, checked by
    /// the caller under this slot's lock, for `asks`' reason.
    pub(crate) fn waits_for_permission(&mut self, waiting: Waiting) {
        self.permission = Some(waiting);
    }

    /// The held tool call, taken to answer it. `None` where the hold has
    /// already ended and the answer has to go as a turn.
    pub(crate) fn permission_reply(&mut self) -> Option<oneshot::Sender<Answered>> {
        self.permission
            .as_mut()
            .and_then(|waiting| waiting.reply.take())
    }

    /// The permission question is answered, or no longer anybody's to answer.
    pub(crate) fn permission_settled(&mut self) -> Option<Waiting> {
        self.permission.take()
    }

    /// What [`Progress::turned`](crate::Progress::turned) reads now. The
    /// baseline a redirect is held against, and it is asked for **before** the
    /// instruction goes down the pipe.
    pub(crate) fn turned(&self) -> usize {
        self.transcript.progress().turned
    }

    /// A person's redirect has gone into the session and the Job is held at
    /// `escalated` until the Drone answers it.
    ///
    /// Both are handed in rather than read here, because the write has already
    /// happened by the time anything can call this — see
    /// [`answering`](Working::answering). The instant is when the instruction
    /// went into the session and not when the Drone read it: nothing on this
    /// side of the pipe can say the second.
    ///
    /// **A second redirect replaces the first.** Nothing bounds how often a Job
    /// may be redirected, and a baseline kept from the earlier one would let a
    /// turn taken before the new instruction answer it.
    pub(crate) fn awaiting_answer(&mut self, turned: usize, at: Timestamp) {
        self.answering = Some(Awaiting {
            turned,
            sent_at: at,
        });
    }

    /// When the outstanding redirect went into the session, where one is.
    /// **`None` is a Job with nothing outstanding** — the same reading
    /// [`turned_since_redirect`](Working::turned_since_redirect) takes, asked by
    /// a reader rather than by the vigil.
    pub(crate) fn awaiting_since(&self) -> Option<&Timestamp> {
        self.answering.as_ref().map(|awaiting| &awaiting.sent_at)
    }

    /// Whether the Drone has taken a turn since a redirect was put to it.
    ///
    /// **`false` where no redirect is outstanding**, which is every Job but the
    /// one a person has just spoken to — so the question costs a lock and a
    /// comparison and reaches no store.
    ///
    /// **`turned` and not `heard`.** A `tool_progress` heartbeat is a Drone
    /// that never stopped working rather than one that read what a person said,
    /// and counting it would move the Job back to `running` on a Drone that is
    /// wedged inside the same call it was wedged in when the vigil caught it.
    pub(crate) fn turned_since_redirect(&self) -> bool {
        self.answering
            .as_ref()
            .is_some_and(|awaiting| self.transcript.progress().turned > awaiting.turned)
    }

    /// The outstanding redirect has been answered, and nothing is waiting on
    /// this Drone.
    pub(crate) fn answered(&mut self) {
        self.answering = None;
    }
}
