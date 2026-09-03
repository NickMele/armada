//! Every proposal in flight — what it has reached, who is told, and the one
//! thing anybody can do to one.
//!
//! **`crate::judging`'s `Aloft` for a call with no Job.** That writes down the
//! Judge call that is out, keyed by Job. A proposal is the interval *before*
//! any Job exists, so it is keyed by an id minted for the call itself and
//! reaches no Job read at all: what a client learns, it learns from the stream.
//!
//! **[`Making`] is a guard**, for `Marking`'s reason with one more end to
//! forget: a proposal ends with Jobs, with a refusal, with a budget or with
//! somebody stopping it, and dropping it writes the coming-back message once.
//!
//! Nothing here is stored. A Fleet that restarts has no calls out, so an empty
//! map is the correct state after one.

use std::collections::BTreeMap;
use std::sync::{Arc, Mutex};
use std::time::Duration;

use adapter_traits::CallProgress;
use core_model::Timestamp;

use crate::clock::Clock;
use crate::judging::JudgeBudget;
use crate::mint::Mint;

/// How often a token estimate may be republished.
///
/// **A throttle and not a sample.** Every reach change publishes immediately;
/// this only bounds the one reading that moves continuously. One a second is
/// what a person reading a number can actually take in, and it is what holds
/// the arithmetic `ipc::proposing` states — a two-minute call bounded at about
/// a hundred and twenty-five messages rather than at the several hundred frames
/// the vendor's stream actually carries.
const TOKEN_TICK: Duration = Duration::from_secs(1);

/// Every proposal in flight, by the id Fleet minted for it.
///
/// **A stop, and nothing else.** There is no read here and no list: what a
/// proposal has reached goes out on the stream as it changes, and a route that
/// answered the same question a second way would be a second authority on a
/// value that moves between the two answers.
#[derive(Clone, Default)]
pub struct Proposals(Arc<Mutex<BTreeMap<ipc::ProposalId, tokio::sync::oneshot::Sender<()>>>>);

impl Proposals {
    pub fn new() -> Proposals {
        Proposals::default()
    }

    /// Stop the call this id names, and say whether there was one.
    ///
    /// **`false` is not a failure.** A proposal that has already answered is
    /// gone from here, and somebody who pressed stop a beat too late is not
    /// owed an error — their Jobs are on the board. `ipc::ProposalStopped` is
    /// what carries the difference, and both arms are a success.
    pub fn stop(&self, proposal: &ipc::ProposalId) -> bool {
        let Ok(mut held) = self.0.lock() else {
            return false;
        };
        // Sending consumes the sender, so a proposal can be stopped once. The
        // receiver is dropped with the call, which is why a send into a closed
        // channel is ignored rather than reported: the call ended between the
        // lookup and here, which is the same "too late" the absence is.
        match held.remove(proposal) {
            Some(stop) => stop.send(()).is_ok(),
            None => false,
        }
    }

    fn forget(&self, proposal: &ipc::ProposalId) {
        if let Ok(mut held) = self.0.lock() {
            held.remove(proposal);
        }
    }
}

/// Everything one proposal needs in order to be watched.
///
/// Assembled by `Fleet::making`, the one place that holds all of it — the same
/// arrangement `Fleet::judging` has for a Judge call.
#[derive(Clone)]
pub struct Watching {
    pub proposals: Proposals,
    pub events: api::Broadcaster,
    pub clock: Arc<dyn Clock>,
    pub mint: Arc<dyn Mint>,
    pub budget: JudgeBudget,
    /// Which model is reading the request. Read for the message, never for the
    /// call — what to ask is the caller's, and this only says what was asked.
    pub model: String,
}

/// One proposal, being made.
///
/// **Alive exactly as long as the call is.** Dropping it publishes the
/// coming-back message and takes the proposal out of the registry, so a caller
/// cannot end a call in a way that leaves a surface waiting on it.
pub struct Making {
    watching: Watching,
    proposal: ipc::ProposalId,
    client_ref: Option<String>,
    since: Timestamp,
    /// What has been reported so far, and when the last message went out.
    /// Behind a lock because the reporting callback is handed out by shared
    /// reference and the runner may report from wherever it is reading.
    reached: Mutex<Reached>,
}

/// What the call has reached, as the last message said it.
struct Reached {
    reach: ipc::ProposalReach,
    thinking_tokens: Option<u64>,
    answered_characters: Option<u64>,
    /// When a message last went out. `None` before the first.
    told_at: Option<Timestamp>,
}

impl Watching {
    /// Begin one. **Publishes the going-out message**, so a surface learns the
    /// id and the budget before anything has happened.
    ///
    /// The stop channel is registered here rather than lazily: a person may
    /// press stop in the first second, and a proposal that could not be stopped
    /// until it had got somewhere would be unstoppable exactly in the case
    /// worth stopping — a call that never reaches the vendor at all.
    pub fn begin(self, client_ref: Option<String>) -> (Making, StopWhenAsked) {
        let proposal = ipc::ProposalId::from(&self.mint.ulid());
        let (stop, stopped) = tokio::sync::oneshot::channel();
        if let Ok(mut held) = self.proposals.0.lock() {
            held.insert(proposal.clone(), stop);
        }
        let since = self.clock.now();
        let making = Making {
            watching: self,
            proposal,
            client_ref,
            since,
            reached: Mutex::new(Reached {
                reach: ipc::ProposalReach::Starting,
                thinking_tokens: None,
                answered_characters: None,
                told_at: None,
            }),
        };
        making.publish(true);
        (making, StopWhenAsked(stopped))
    }
}

/// The future the runner selects on. Resolves when somebody stops the call, and
/// never otherwise — a sender dropped without sending is a proposal that ended
/// on its own, which must not read as a stop.
pub struct StopWhenAsked(tokio::sync::oneshot::Receiver<()>);

impl StopWhenAsked {
    pub async fn asked(self) {
        match self.0.await {
            Ok(()) => (),
            // The registry dropped the sender, which happens when the call is
            // forgotten — by which point the call is over and nothing is
            // selecting on this. Waiting forever is the correct answer: it can
            // never win a race it is no longer in.
            Err(_) => std::future::pending::<()>().await,
        }
    }
}

impl Making {
    /// Report one reading. **Handed to the runner as a callback**, so the
    /// runner knows nothing about events, throttles or ids.
    pub fn telling(&self) -> impl Fn(CallProgress) + Send + Sync + '_ {
        move |progress| self.moved(progress)
    }

    /// The id, for a caller that has to name this proposal in a refusal.
    pub fn proposal(&self) -> &ipc::ProposalId {
        &self.proposal
    }

    fn moved(&self, progress: CallProgress) {
        let Ok(mut reached) = self.reached.lock() else {
            return;
        };
        // A reach change is always worth a message; a count that moved is worth
        // one at most once a tick. **The two are decided here and not by the
        // caller**, so a new arm of `CallProgress` cannot quietly become an
        // unthrottled one.
        let worth_telling = match progress {
            CallProgress::Started => step(&mut reached.reach, ipc::ProposalReach::Started),
            CallProgress::Requesting => step(&mut reached.reach, ipc::ProposalReach::Requesting),
            CallProgress::Thinking { tokens } => {
                reached.thinking_tokens = Some(tokens);
                step(&mut reached.reach, ipc::ProposalReach::Thinking) || self.ticked(&reached)
            }
            CallProgress::Answering { characters } => {
                // Cumulative, because the stream carries one frame's worth at a
                // time and what a surface draws is how much has arrived.
                reached.answered_characters =
                    Some(reached.answered_characters.unwrap_or(0) + characters);
                step(&mut reached.reach, ipc::ProposalReach::Answering) || self.ticked(&reached)
            }
            // The call is over. The coming-back message is the guard's, on
            // drop, so that every way of ending sends exactly one — and this
            // arm deliberately sends nothing rather than a second.
            CallProgress::Ended { .. } => false,
        };
        if worth_telling {
            reached.told_at = Some(self.watching.clock.now());
            drop(reached);
            self.publish(true);
        }
    }

    /// Whether a tick has passed since the last message.
    ///
    /// **Through `Timestamp::epoch_millis`, not a second clock.** That method's
    /// own doc names itself the one reader of an instant in the workspace, and
    /// reaching for `std::time::Instant` here would be this module quietly
    /// acquiring a clock of its own — which is the whole thing `crate::clock`
    /// exists to prevent, and it would make the throttle untestable against a
    /// planted clock.
    ///
    /// A reading that will not parse tells rather than swallows: a message too
    /// many is a cheaper mistake than a surface that stops moving.
    fn ticked(&self, reached: &Reached) -> bool {
        let Some(told_at) = reached.told_at.as_ref().and_then(Timestamp::epoch_millis) else {
            return true;
        };
        let Some(now) = self.watching.clock.now().epoch_millis() else {
            return true;
        };
        now.saturating_sub(told_at) >= TOKEN_TICK.as_millis() as i64
    }

    /// Send one message. `out` false is the coming-back one, which carries
    /// nothing — see `ipc::ProposalMoved::proposing`.
    fn publish(&self, out: bool) {
        // **Nobody listening, nothing produced.** The same decline
        // `crate::footprint` makes, and the first of the three bounds
        // `ipc::proposing` states: an unwatched proposal costs this channel
        // nothing at all.
        if self.watching.events.watching() == 0 {
            return;
        }
        let at = self.watching.clock.now();
        let proposing = out.then(|| {
            let reached = self.reached.lock().ok();
            ipc::ProposalInFlight {
                proposal_id: self.proposal.clone(),
                client_ref: self.client_ref.clone(),
                model: self.watching.model.clone(),
                since: (&self.since).into(),
                budget_ms: self.watching.budget.duration().as_millis() as u64,
                reached: reached
                    .as_ref()
                    .map(|reached| reached.reach)
                    .unwrap_or(ipc::ProposalReach::Starting),
                thinking_tokens: reached.as_ref().and_then(|reached| reached.thinking_tokens),
                answered_characters: reached
                    .as_ref()
                    .and_then(|reached| reached.answered_characters),
            }
        });
        self.watching
            .events
            .publish(ipc::Event::ProposalMoved(ipc::ProposalMoved {
                proposal_id: self.proposal.clone(),
                client_ref: self.client_ref.clone(),
                proposing,
                // The caller's, for `JobCreated`'s reason: a proposal is a
                // human or Helm act and nothing here is Fleet deciding on its
                // own. Fleet makes the call; it did not choose to.
                actor: core_model::Actor::Human.into(),
                at: (&at).into(),
            }));
    }
}

impl Drop for Making {
    /// However the call ended. See the type's own note.
    fn drop(&mut self) {
        self.watching.proposals.forget(&self.proposal);
        self.publish(false);
    }
}

/// Move `reach` forward and say whether it moved. **Forward only**: the frames
/// arrive interleaved — a thinking token estimate can land after the first
/// answer delta — and a reach that could go backwards would make a surface
/// flicker between two states the call is not moving between.
fn step(reach: &mut ipc::ProposalReach, to: ipc::ProposalReach) -> bool {
    if rank(to) <= rank(*reach) {
        return false;
    }
    *reach = to;
    true
}

/// How far along one value is. The order the doc on `ProposalReach` states, and
/// the only place it is written as a number.
fn rank(reach: ipc::ProposalReach) -> u8 {
    match reach {
        ipc::ProposalReach::Starting => 0,
        ipc::ProposalReach::Started => 1,
        ipc::ProposalReach::Requesting => 2,
        ipc::ProposalReach::Thinking => 3,
        ipc::ProposalReach::Answering => 4,
    }
}
