//! What the forge says about a pull request that is still open, read on the
//! sweep `crate::noticing` already runs.
//!
//! **The same rotation, never a second one.** The Job asked is the Job the
//! rotation had already reached, and it is asked only where the merge read came
//! back `Landing::Open`. A second loop over the same set would double the cost
//! of the same question and disagree with the first about a pull request that
//! settled between them. What it costs, and how stale a reading can be, is in
//! `docs/concepts/fleet.md`, *What Fleet knows after the merge*.
//!
//! **Nothing here decides anything.** No Job transitions and no gate is touched
//! by the reading. What it is *for*, beyond the line, is `auto_merge:
//! checks-pass`, and every part of whether that may merge is `crate::merging`'s.
//!
//! **A line in the Job's log when the reading changes, and not on every ask.**
//! The rotation returns to the same open pull request for as long as it stays
//! open, and a line per ask would bury the log under one repeated sentence.
//!
//! **No remark's text reaches the log; the count does.** A comment is written
//! by whoever can see the pull request, and the road it is read for ends at a
//! Drone's prompt — `#526`. What a failing check is *called* is in the line,
//! because that is the one thing that sends a person to the right tab.
//!
//! **Nothing is stored.** `store::record_landed` writes a settled answer only,
//! and a reading that changes hour to hour is the absence of news.

use std::collections::{BTreeMap, BTreeSet};
use std::hash::{Hash, Hasher};

use adapter_traits::{
    AgentHarness, Delivery, UnderReview, Vcs, WhatPeopleSaid, WhatTheForgeRan, WorkProduct,
};
use core_model::{Actor, Component, Envelope, FieldValue, JobId, Level};

use crate::daemon::Fleet;

/// What the last sweep read about one pull request, as much of it as deciding
/// *changed* needs.
///
/// **Three words and a count, not the reading itself.** What is compared is
/// what a line would say, so a remark edited in place changes nothing here and
/// writes no line — which is right: the line says how many there are, and there
/// are still that many.
#[derive(Clone, Debug, PartialEq, Eq)]
pub(crate) struct AsItStood {
    people: WhatPeopleSaid,
    checks: WhatTheForgeRan,
    remarks: usize,
}

impl AsItStood {
    fn of(read: &UnderReview) -> AsItStood {
        AsItStood {
            people: read.people,
            checks: read.checks.clone(),
            remarks: read.remarks.len(),
        }
    }
}

/// Every open pull request this process has read, and what it said last time.
///
/// **In memory, for the rotation cursor's reason**, and losing it costs one
/// repeated line per open pull request after a restart — against a set that is
/// small by construction and shrinks as Jobs settle.
pub(crate) type Standings = BTreeMap<String, AsItStood>;

/// What would tell Bridge to re-ask `get_remarks`: every remark's handle, a
/// hash of its words, and every reviewer's verdict. `#661`.
///
/// **A stronger comparison than [`AsItStood`], and a separate one rather than
/// a replacement of it.** That one carries a count because the log line it
/// feeds says how many there are and nothing about which — a comment edited
/// in place, or one added the same turn another was deleted, changes nothing
/// a count can see and is exactly what a person waiting on a review needs to
/// know about. This does not change what the log line says; it decides
/// whether Bridge is told at all.
///
/// **A hash of the words, never the words**, for [`AsItStood`]'s own reason:
/// nothing here needs the text to notice it moved, and carrying it in memory
/// for the sweep's whole life would put a comment's body somewhere the rule
/// that keeps it out of the log does not reach.
///
/// **Verdicts are compared on their own**, apart from [`AsItStood::people`].
/// That field is the forge's roll-up, and a second approval on a pull request
/// already `Approved` moves nothing on it — so a comparison built only from it
/// would miss the one thing a reviewer most wants noticed: that somebody else
/// looked.
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub(crate) struct RemarkSignature {
    /// Every remark's handle and a hash of its body, so an add, a remove or an
    /// edit all move this even where the count does not.
    remarks: BTreeMap<String, u64>,
    /// Every reviewer's verdict, by who gave it.
    verdicts: BTreeSet<(String, &'static str)>,
}

impl RemarkSignature {
    fn of(read: &UnderReview) -> RemarkSignature {
        RemarkSignature {
            remarks: read
                .remarks
                .iter()
                .map(|remark| {
                    (
                        remark.id.as_written().to_string(),
                        hashed(remark.said.as_written()),
                    )
                })
                .collect(),
            verdicts: read
                .verdicts
                .iter()
                .map(|reviewed_by| {
                    (
                        reviewed_by.by.as_written().to_string(),
                        reviewed_by.verdict.kind(),
                    )
                })
                .collect(),
        }
    }
}

/// A comment's words, reduced to a number that changes when they do. Not a
/// security boundary — nothing here is checked against an adversary — only a
/// cheap way to notice an edit without keeping the edit around.
fn hashed(said: &str) -> u64 {
    let mut hasher = std::collections::hash_map::DefaultHasher::new();
    said.hash(&mut hasher);
    hasher.finish()
}

impl<H, V, W> Fleet<H, V, W>
where
    H: AgentHarness + Send + Sync + 'static,
    H::Error: std::error::Error + Send + Sync + 'static,
    V: Vcs + Delivery + Send + Sync + 'static,
    V::Error: std::error::Error + Send + Sync + 'static,
    V::CommitError: std::error::Error + Send + Sync + 'static,
    W: WorkProduct + Send + Sync + 'static,
    W::Error: std::error::Error + Send + Sync + 'static,
{
    /// Ask the forge what is happening on one open pull request, say so in the
    /// Job's log where it has changed since the last time this asked, and hand
    /// the reading back so the caller can cache it for `get_job`.
    ///
    /// **Nothing raises.** This is the shape `noticing::nudged` already has and
    /// for a stronger version of its reason: the Job finished, it is holding
    /// at a human gate, and no answer here moves it. A forge that would not
    /// answer is `None`, one silence and another sweep.
    pub(crate) async fn read_what_is_under_review(
        &self,
        job: &JobId,
        url: &str,
    ) -> Option<UnderReview> {
        let read = self.vcs().under_review(&self.host().repo_root, url);
        // **A forge that would not answer changes nothing that was already
        // known.** Recording the silence would erase the last real reading and
        // then write the same line again when it came back — `Landing::Unknown`
        // is not `Landing::Open`, said one type over.
        if !read.was_answered() {
            return None;
        }
        // **Before the line, and outside the comparison below.** What the
        // policy may do about this reading is not "is it news" — a Job whose
        // repository turned `auto_merge` on since the last sweep has the same
        // reading and a different answer, and a merge that waited for the forge
        // to change its mind would never happen. `crate::merging` is what keeps
        // it from being attempted twice.
        self.merged_if_the_policy_says_so(job, url, &read.checks)
            .await;
        let stood = AsItStood::of(&read);
        let changed = {
            let mut sweeping = self.sweeping().lock().await;
            let changed = sweeping.reviewed.get(url) != Some(&stood);
            sweeping.reviewed.insert(url.to_string(), stood);
            changed
        };
        // **The line is still only on change**, exactly as before — only the
        // return to the caller is unconditional now, so a rotation that finds
        // nothing new to log still refreshes `Sweep::pr_detail`.
        if changed {
            self.noted_the_review(job, url, &read);
        }
        // **A second, stronger comparison for Bridge, independent of the
        // line above.** `RemarkSignature`'s own doc says why a count is not
        // enough here. `None` on the baseline is the first sweep to read this
        // pull request — nothing to compare against, so it is recorded and
        // not reported, which is what keeps a Fleet that just started from
        // publishing one of these for every open pull request it holds.
        let signature = RemarkSignature::of(&read);
        let commented = {
            let mut sweeping = self.sweeping().lock().await;
            let baseline = sweeping
                .commented
                .insert(url.to_string(), signature.clone());
            matches!(baseline, Some(previous) if previous != signature)
        };
        if commented {
            self.published_remarks_changed(job).await;
        }
        Some(read)
    }

    /// Tell Bridge this Job's pull request comments changed, so a person
    /// looking at them gets a fresh `get_remarks` rather than the one taken
    /// when the Job's detail opened. `#661`.
    ///
    /// **Names the Job and nothing else.** `get_remarks` already answers what
    /// changed, and carrying a copy of its answer on this event would be a
    /// second shape for the one fact — see [`ipc::JobRemarksChanged`]'s own
    /// doc.
    async fn published_remarks_changed(&self, job: &JobId) {
        self.publish(ipc::Event::JobRemarksChanged(ipc::JobRemarksChanged {
            job_id: job.into(),
            actor: Actor::Fleet.into(),
            at: (&self.now()).into(),
        }));
    }

    /// The line in the Job's own log.
    ///
    /// **[`Level::Info`], whatever it says.** A request for changes and a
    /// failing forge check are things a person acts on, and a `Warn` would be
    /// Fleet colouring a fact it takes no action on — the Job is at a gate and
    /// the person is looking at the pull request itself. Nothing here is a
    /// Check result and nothing here failed.
    fn noted_the_review(&self, job: &JobId, url: &str, read: &UnderReview) {
        let mut said = String::from("the forge says ");
        said.push_str(read.people_said());
        said.push_str(", and ");
        said.push_str(&read.checks_said());
        let mut envelope = Envelope::new(
            self.now(),
            Level::Info,
            Component::Fleet,
            self.run().clone(),
            said,
        )
        .in_job(job.as_ulid().clone())
        .with_field("pull_request", FieldValue::Str(url.to_string()))
        .with_field("review", FieldValue::Str(read.people.kind().to_string()))
        .with_field("checks", FieldValue::Str(read.checks.kind().to_string()))
        .with_field("remarks", FieldValue::Int(read.remarks.len() as i64));
        // **The names of the checks that did not pass, and nothing else from
        // outside this machine.** A check's name is what sends a person to the
        // right tab; a remark's text is on the road that ends at a Drone's
        // prompt and does not travel through a log to get there.
        if let WhatTheForgeRan::SomeFailed { failed, .. } = &read.checks {
            let named: Vec<&str> = failed.iter().map(|name| name.as_written()).collect();
            envelope = envelope.with_field("failed", FieldValue::Str(named.join(", ")));
        }
        self.logged(job, envelope);
    }
}
