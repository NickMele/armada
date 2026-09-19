//! Every Helm call held open while a person answers it, and the answer going
//! back inside the call. `#1389`.
//!
//! **Fleet decides nothing here.** The person's own settings already allowed
//! what they allow and refused what they refuse; what reaches this module is
//! the remainder, and the only thing it does with one is carry it to the dock
//! and carry an answer back. There is no policy to read and none to write.
//!
//! **Held on the table and written to no column**, for
//! `crate::permitting::Waiting`'s reason one subject over: what is waiting is
//! one process inside one tool call, and a Fleet that restarts has lost the
//! process whose call it was. A restart leaves nothing to resume.

use std::collections::BTreeMap;
use std::sync::atomic::{AtomicU64, Ordering};
use std::sync::Mutex;
use std::time::Duration;

use adapter_traits::CallDetail;
use core_model::Timestamp;
use ipc::{
    AnswerHelmCall, AskingToRun, HelmCallAnswer, HelmCallInFlight, Instant, ManifestId, RunOrNot,
};
use tokio::sync::oneshot;

/// How long one ask is held open for a person before Fleet answers `deny` for
/// them. The shipped value is the *Helm ask hold* setting, which the
/// composition root resolves and writes out.
///
/// **A fitting rather than a bare constant**, for `PermissionHold`'s reason: a
/// test can plant a bound it can outlive, where five real minutes is not a wait
/// any case can make.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct HelmAskHold(Duration);

impl HelmAskHold {
    pub const fn of(hold: Duration) -> HelmAskHold {
        HelmAskHold(hold)
    }

    pub fn duration(&self) -> Duration {
        self.0
    }
}

/// What a person said, as it travels from the route to the call.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Said {
    pub answer: HelmCallAnswer,
    /// Their words, on a refusal. Trimmed away where they typed nothing.
    pub note: Option<String>,
}

/// Why an answer did not land.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum NotAnswerable {
    /// No call by that id is waiting. An answer from a window left open across
    /// one, or a second press.
    NothingWaiting,
    /// An answer outside what that call offered.
    NotOffered,
}

/// One ask, and the half of the channel its answer goes down.
struct Held {
    in_flight: HelmCallInFlight,
    answer: oneshot::Sender<Said>,
}

/// Every Helm call waiting on a person right now, across every repository.
///
/// **One table for the machine, not one per conversation.** The dock is
/// Fleet-wide and a person answers what is waiting on them wherever it came
/// from — which is how `#935` already gathers a Job's questions.
pub struct Asks {
    waiting: Mutex<BTreeMap<String, Held>>,
    minted: AtomicU64,
    hold: HelmAskHold,
}

impl Asks {
    pub fn holding_for(hold: HelmAskHold) -> Asks {
        Asks {
            waiting: Mutex::new(BTreeMap::new()),
            minted: AtomicU64::new(0),
            hold,
        }
    }

    pub fn hold(&self) -> Duration {
        self.hold.duration()
    }

    pub fn held_for(&self) -> HelmAskHold {
        self.hold
    }

    /// Put one call on the table and hand back what a person will be shown and
    /// the channel their answer arrives on.
    ///
    /// **The id is Fleet's, not the harness's.** `tool_use_id` is the session's
    /// own and is absent on some calls; an answer naming an id Fleet minted is
    /// an answer to the ask a person actually read.
    pub fn minted(
        &self,
        manifest_id: &ManifestId,
        asking: &AskingToRun,
        at: &Timestamp,
    ) -> (HelmCallInFlight, oneshot::Receiver<Said>) {
        let detail = CallDetail::of(asking.detail().unwrap_or_default().trim());
        let call = format!(
            "helm-{}",
            self.minted.fetch_add(1, Ordering::SeqCst).wrapping_add(1)
        );
        let in_flight = HelmCallInFlight {
            call: call.clone(),
            manifest_id: manifest_id.clone(),
            asked_at: Instant::from(at),
            tool: asking.tool_name.clone(),
            detail: detail.shown().to_string(),
            truncated: detail.truncated(),
            length: Some(detail.length()),
            rule: rule_for(&asking.tool_name, detail.whole().unwrap_or(detail.shown())),
            offers: offers_for(&asking.tool_name),
            holding_for_seconds: self.hold().as_secs(),
        };
        let (sender, receiver) = oneshot::channel();
        if let Ok(mut waiting) = self.waiting.lock() {
            waiting.insert(
                call,
                Held {
                    in_flight: in_flight.clone(),
                    answer: sender,
                },
            );
        }
        (in_flight, receiver)
    }

    /// A person's answer. **Takes the ask off the table** — a second press
    /// finds nothing waiting, which is what makes it a refusal rather than a
    /// second answer to the same call.
    pub fn answer(&self, said: &AnswerHelmCall) -> Result<HelmCallInFlight, NotAnswerable> {
        let mut waiting = self
            .waiting
            .lock()
            .map_err(|_| NotAnswerable::NothingWaiting)?;
        let held = waiting
            .get(&said.call)
            .ok_or(NotAnswerable::NothingWaiting)?;
        if !held.in_flight.offers.contains(&said.answer) {
            return Err(NotAnswerable::NotOffered);
        }
        let held = waiting
            .remove(&said.call)
            .ok_or(NotAnswerable::NothingWaiting)?;
        let in_flight = held.in_flight.clone();
        // The receiver is gone where the hold already ran out. The ask is off
        // the table either way, and the call has answered itself.
        let _ = held.answer.send(Said {
            answer: said.answer,
            note: said
                .note
                .as_deref()
                .map(str::trim)
                .filter(|note| !note.is_empty())
                .map(str::to_string),
        });
        Ok(in_flight)
    }

    /// Take one ask off the table without answering it — the hold running out,
    /// or the session going. `None` where a person got there first.
    pub fn withdraw(&self, call: &str) -> Option<HelmCallInFlight> {
        let mut waiting = self.waiting.lock().ok()?;
        waiting.remove(call).map(|held| held.in_flight)
    }

    /// Every ask waiting, oldest first — what `list_helm_calls` answers.
    pub fn waiting(&self) -> Vec<HelmCallInFlight> {
        let Ok(waiting) = self.waiting.lock() else {
            return Vec::new();
        };
        let mut asks: Vec<HelmCallInFlight> = waiting
            .values()
            .map(|held| held.in_flight.clone())
            .collect();
        asks.sort_by(|one, two| one.asked_at.cmp(&two.asked_at));
        asks
    }
}

/// What a person may answer about one call, in the order a surface draws them.
///
/// **A rule is offered only where one can be written.** `allow_and_remember`
/// writes a settings rule, and a tool whose rule would be its bare name is
/// still a rule the CLI honours — so every tool offers all three, and the
/// narrowing that a Drone's `offers` does by policy has no equivalent here.
fn offers_for(_tool: &str) -> Vec<HelmCallAnswer> {
    vec![
        HelmCallAnswer::AllowOnce,
        HelmCallAnswer::AllowAndRemember,
        HelmCallAnswer::Refuse,
    ]
}

/// The rule that would have to allow this call, in the CLI's own settings
/// spelling.
///
/// **The leading cut of a shell line, never the whole of one.** A rule holding
/// a whole command with its arguments would match that command and nothing
/// else, which is a rule a person would have to add again tomorrow;
/// `crate::permitting::always_allow_rules` already decides where a command
/// stops being a subcommand, and this is that answer in the CLI's syntax rather
/// than a second one.
pub fn rule_for(tool: &str, detail: &str) -> String {
    if tool != "Bash" {
        return tool.to_string();
    }
    match crate::permitting::always_allow_rules(detail).1 {
        Some(prefix) => format!("Bash({prefix}:*)"),
        // A line that chains, or starts with an operator-bearing word. There is
        // no safe prefix of it, so the rule is the command as it stands.
        None => format!("Bash({})", detail.trim()),
    }
}

/// What the session is told, out of what a person said.
///
/// **A refusal reaches the model and is written for one.** Spike 19: the deny
/// message arrives as the tool result with `is_error`, in place of what the
/// call would have returned, and it is the only thing the session learns about
/// why.
pub fn answering(asking: &AskingToRun, said: &Said) -> RunOrNot {
    match said.answer {
        HelmCallAnswer::AllowOnce | HelmCallAnswer::AllowAndRemember => RunOrNot::Allow {
            updated_input: asking.input.clone(),
        },
        HelmCallAnswer::Refuse => RunOrNot::Deny {
            message: match &said.note {
                Some(note) => format!("The person refused this: {note}"),
                None => String::from(
                    "The person refused this. Say what you could not do, and carry on without it",
                ),
            },
        },
    }
}

/// What a session is told where nobody answered inside the hold.
pub fn unanswered(seconds: u64) -> RunOrNot {
    RunOrNot::Deny {
        message: format!(
            "Nobody answered this within {seconds} seconds, so it was not run. Say what you were \
             waiting on, and ask again if it still matters"
        ),
    }
}
