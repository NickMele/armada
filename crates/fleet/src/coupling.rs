//! What an upstream's terminal status does to the Job waiting behind it.
//!
//! # Three outcomes, not two
//!
//! `docs/concepts/fleet.md` on DAG scheduling is the rule and [`coupling`] is
//! the whole of it: `completed_success` releases, `superseded` releases and is
//! carried as unsatisfied, any other terminal escalates, anything not terminal
//! waits. **`superseded` releasing is not leniency** — the work landed outside
//! the Job, so the base is there and only the record has nothing to say, which
//! is `job-statuses.toml`'s argument for it being a status rather than a queued
//! reason. Until it released here it blocked a dependent for ever. There is no
//! edge type and no hard/soft flag: the variation is entirely in that status.
//!
//! # One predicate, and `blocks` is not read
//!
//! [`coupling`] is the only place an edge is weighed. `dispatch::clear_to_run`
//! is `Clear` and nothing else, and `serving`'s Board label is that same call —
//! so a Board saying `blocked_by_dependency` and a Fleet admitting the Job
//! cannot both be answering. `ipc::DependencyEdge` says `blocks` is expressible
//! and never written, because a plan is created in dependency order; reading it
//! would also break the acyclicity [`Fleet::peers_held`] establishes, since "B
//! blocks A" points forward in time at a Job already on the board.

use std::collections::{BTreeMap, BTreeSet};

use adapter_traits::{AgentHarness, Delivery, Vcs, WorkProduct};
use core_model::{Actor, DependencyDirection, EscalationTrigger, Job, JobId, JobStatus, Target};

use crate::adrift::Adrift;
use crate::daemon::Fleet;

/// Where a Job stands against every peer it waits on.
///
/// **`Failed` outranks `Waiting`**, whatever order the edges are in. A peer
/// that failed never un-fails, so a dependent also waiting on a live peer is
/// already unrunnable and waiting for the live one to finish would only delay
/// the same answer — which is `escalation-triggers.toml`'s argument for
/// escalating at all: CPU headroom self-clears and a failed upstream does not.
#[derive(Clone, Debug, PartialEq, Eq)]
pub(crate) enum Coupling {
    /// At least one peer has not finished, or is not on the board at all. An
    /// edge pointing at a Job that was retained out counts as unfinished — of
    /// the two answers only "run it" cannot be taken back.
    Waiting,
    /// A peer reached a terminal that is neither `completed_success` nor
    /// `superseded`. The first one found, which is the one worth naming.
    Failed { peer: JobId },
    /// Every peer finished. `unsatisfied` names the ones that were superseded
    /// rather than completed — released, and released is not satisfied.
    Clear { unsatisfied: Vec<JobId> },
}

/// Weigh one Job's edges against the board. **The only reader of an edge.**
pub(crate) fn coupling(job: &Job, standing: &BTreeMap<JobId, JobStatus>) -> Coupling {
    let mut unsatisfied = Vec::new();
    let mut waiting = false;
    let edges = job
        .dependencies()
        .iter()
        .filter(|edge| edge.direction == DependencyDirection::DependsOn);
    for edge in edges {
        match standing.get(&edge.peer) {
            Some(JobStatus::CompletedSuccess) => {}
            Some(JobStatus::Superseded) => unsatisfied.push(edge.peer.clone()),
            Some(status) if status.is_terminal() => {
                return Coupling::Failed {
                    peer: edge.peer.clone(),
                }
            }
            _ => waiting = true,
        }
    }
    match waiting {
        true => Coupling::Waiting,
        false => Coupling::Clear { unsatisfied },
    }
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
    /// Escalate every `queued` Job whose upstream ended badly.
    ///
    /// **The walk is the other direction from admission.** `clear_to_run` asks
    /// one Job about its peers on the way to starting it; this asks the same
    /// question of every waiting Job, because nothing else in the loop visits a
    /// *dependent* of the Job that just ended. Without it the dependent sits at
    /// `queued` behind `blocked_by_dependency` for ever — a label with no action
    /// attached, which is the outcome `escalation-triggers.toml` rejected.
    ///
    /// **Per turn and not per transition.** A Job reaches a terminal from the
    /// gate, from a kill, from a reap and from reconciliation; hooking each
    /// would be five places agreeing, and the next one added would be the sixth
    /// that forgot. One read of the board a turn is the same cost `admit_next`
    /// already pays and it cannot be got out of step with.
    ///
    /// **Nothing downstream is cancelled.** Escalating stops at the first
    /// dependent and asks a person, which is `fleet.md`'s *"so a person decides
    /// rather than one failure terminating a chain unattended"*. A dependent of
    /// *this* Job stays `queued`, because `escalated` is not terminal and the
    /// chain has not failed — it is waiting on a person.
    pub(crate) async fn strand_dependents(&self) -> Result<Vec<JobId>, Adrift> {
        let (loaded, _) = self.every_job().await?;
        let standing: BTreeMap<JobId, JobStatus> = loaded
            .jobs
            .iter()
            .map(|job| (job.id().clone(), job.status()))
            .collect();
        let mut stranded = Vec::new();
        for job in &loaded.jobs {
            if job.status() != JobStatus::Queued {
                continue;
            }
            if !matches!(coupling(job, &standing), Coupling::Failed { .. }) {
                continue;
            }
            self.move_job(
                job,
                Target::Escalated(EscalationTrigger::DependencyFailed),
                Actor::Fleet,
            )
            .await?;
            stranded.push(job.id().clone());
        }
        Ok(stranded)
    }

    /// Refuse a proposal that names a peer this Fleet does not hold.
    ///
    /// **This is the cycle check, and it is a refusal rather than a search.**
    /// `ProposeJob.dependencies` already says a peer *"must already exist: an
    /// edge is a pointer, and Fleet mints the ids"* — and nothing enforced it,
    /// so two Jobs each naming the other were representable and both were
    /// permanently unadmittable behind a label that reads as ordinary waiting.
    ///
    /// Enforced, the rule makes a cycle **unrepresentable** rather than
    /// detected: `dependencies` is written once, at insert, by the one statement
    /// in `store::write` that creates a row. So every edge points at a strictly
    /// older Job, "older" is a strict order, and a topological sort would have
    /// nothing to find. That is the cheaper answer and the durable one — there
    /// is no acyclicity assertion to keep in step with a graph that can change,
    /// because the graph cannot.
    ///
    /// The proposer's own path is unaffected and already safe:
    /// `proposing::read` refuses a plan whose edges do not point backwards, and
    /// `propose_from_request` mints in that order.
    pub(crate) async fn peers_held(&self, edges: &[ipc::DependencyEdge]) -> Result<(), Adrift> {
        if edges.is_empty() {
            return Ok(());
        }
        let (loaded, _) = self.every_job().await?;
        let held: BTreeSet<JobId> = loaded.jobs.iter().map(|job| job.id().clone()).collect();
        match edges
            .iter()
            .map(|edge| edge.peer.to_domain())
            .find(|peer| !held.contains(peer))
        {
            Some(missing) => Err(Adrift::NoSuchPeer {
                named: missing.as_str().to_string(),
            }),
            None => Ok(()),
        }
    }
}
