//! What a working Drone is told about other Jobs writing where it writes. #998.
//!
//! `crate::overlap` names a collision to a person; this tells the Drones on
//! both sides, when it first appears and when the other Job lands. **It informs
//! and never holds**: nothing on the dispatch path reads it.
//!
//! News is queued as it happens and sent at most once every [`SPACING`] per
//! Drone; a Job with no live Drone keeps it for its next opening brief. **In
//! memory**, so a restart announces a standing claim once more rather than
//! losing one.

use std::collections::{BTreeMap, BTreeSet};
use std::time::Duration;

use adapter_traits::{AgentHarness, Delivery, Vcs, WorkProduct};
use core_model::{collisions, JobId, RepoPath, ScopeClaim, Timestamp};

use crate::converging::elapsed;
use crate::daemon::Fleet;
use crate::session::{LiveSession, Occasion};

/// The least time between two peer turns to one Drone.
pub const SPACING: Duration = Duration::from_secs(120);

/// How many items one turn names before it counts the rest.
pub const AT_MOST: usize = 5;

/// One thing a Job is owed about another.
#[derive(Clone, Debug, PartialEq, Eq)]
pub(crate) enum News {
    /// The other Job said it will change these paths, which this one claims too.
    Claimed { title: String, paths: Vec<String> },
    /// The other Job landed, changing these paths this one claims.
    Landed { title: String, paths: Vec<String> },
}

/// What is queued, what has been said, and when each Drone last heard.
#[derive(Debug, Default)]
pub(crate) struct Peering {
    owed: BTreeMap<JobId, Vec<News>>,
    last_told: BTreeMap<JobId, Timestamp>,
    /// Every shared path already announced, keyed by the pair in id order, so
    /// a Drone that declares the same plan again tells nobody anything.
    announced: BTreeSet<(JobId, JobId, String)>,
}

/// What a Drone is told about other Jobs writing where it writes. **Fleet's own
/// sentence**, and this is the only way to build one.
/// `docs/contracts/agent-prompt.md`, The peer turn, has the drafted wording.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct PeersChanged(String);

impl PeersChanged {
    /// Injected into a live session, where the branch has not moved yet.
    pub(crate) fn injected(news: &[News]) -> PeersChanged {
        PeersChanged::rendered(
            news,
            "What landed reaches your branch when your next part starts, not now.",
        )
    }

    /// In an opening brief, where the rebase has just run.
    pub(crate) fn opening(news: &[News]) -> PeersChanged {
        PeersChanged::rendered(
            news,
            "What landed was brought into your branch as this part started.",
        )
    }

    fn rendered(news: &[News], landed_reaches: &str) -> PeersChanged {
        let mut text = String::from(
            "OTHER JOBS WRITING WHERE YOU ARE\n\nOther Jobs in this repository change files \
             this Job changes too. Nothing is stopped, and nobody waits on you.\n",
        );
        for item in news.iter().take(AT_MOST) {
            text.push_str(&match item {
                News::Claimed { title, paths } => {
                    format!("\n- \"{title}\" has said it will change {}.", listed(paths))
                }
                News::Landed { title, paths } => {
                    format!("\n- \"{title}\" landed, changing {}.", listed(paths))
                }
            });
        }
        if news.len() > AT_MOST {
            text.push_str(&format!("\n- And {} more.", news.len() - AT_MOST));
        }
        text.push_str("\n\n");
        if news.iter().any(|item| matches!(item, News::Landed { .. })) {
            text.push_str(landed_reaches);
            text.push(' ');
        }
        text.push_str(
            "Where a shared file hands out the next number or name, such as a migration or a \
             version, assume theirs takes it first and take the one after. Carry on with the \
             part you were given.",
        );
        PeersChanged(text)
    }

    pub fn text(&self) -> &str {
        &self.0
    }
}

/// The first three paths, and a count for the rest.
fn listed(paths: &[String]) -> String {
    const NAMED: usize = 3;
    let named = paths
        .iter()
        .take(NAMED)
        .map(|path| format!("`{path}`"))
        .collect::<Vec<_>>()
        .join(", ");
    match paths.len().saturating_sub(NAMED) {
        0 => named,
        rest => format!("{named} and {rest} more"),
    }
}

/// The pair in one order, so either side announcing it finds the same key.
fn pair(one: &JobId, other: &JobId, path: &str) -> (JobId, JobId, String) {
    let (first, second) = if one <= other {
        (one, other)
    } else {
        (other, one)
    };
    (first.clone(), second.clone(), path.to_string())
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
    /// Whether two Jobs belong to one repository.
    pub(crate) fn same_repository(&self, one: &JobId, other: &JobId) -> bool {
        self.names().owner_of(one) == self.names().owner_of(other)
    }

    /// Queue what this Job's claims newly share with each other unfinished Job,
    /// for the Drones on both sides.
    ///
    /// **Nothing here can fail the call that reached it**, a declaration or a
    /// spawn: a read that will not answer queues nothing.
    pub(crate) async fn claims_announced(&self, job: &JobId) {
        let Ok(this) = self.load(job).await else {
            return;
        };
        let Ok(Some(overlaps)) = self.write_scope_overlaps(&this).await else {
            return;
        };
        let mut peering = self.peering().lock().await;
        for other in overlaps {
            let other_id = other.job_id.to_domain();
            let mut fresh = Vec::new();
            for shared in &other.paths {
                if peering.announced.insert(pair(job, &other_id, &shared.path)) {
                    fresh.push(shared.path.clone());
                }
            }
            if fresh.is_empty() {
                continue;
            }
            peering
                .owed
                .entry(job.clone())
                .or_default()
                .push(News::Claimed {
                    title: other.title.clone(),
                    paths: fresh.clone(),
                });
            peering
                .owed
                .entry(other_id)
                .or_default()
                .push(News::Claimed {
                    title: this.title().as_str().to_string(),
                    paths: fresh,
                });
        }
    }

    /// Queue, for each unfinished Job in the same repository, what a Job that
    /// just landed changed where that Job claims to write.
    ///
    /// **What it changed is the footprint kept at its terminal transition.**
    /// Where none was kept, its own claims stand in, which says where it meant
    /// to write rather than nothing.
    pub(crate) async fn landing_announced(&self, landed: &JobId) {
        let Ok(this) = self.load(landed).await else {
            return;
        };
        let footprint = self.store().lock().await.footprint(landed).ok().flatten();
        let written = match footprint {
            Some(kept) => vec![ScopeClaim::by_the_job(
                kept.files
                    .iter()
                    .map(|file| RepoPath::new(file.path()))
                    .collect(),
            )],
            None => match self.scope_claims(&this).await {
                Ok(claims) => claims,
                Err(_) => return,
            },
        };
        let Ok((loaded, _)) = self.every_job().await else {
            return;
        };
        let mut owed = Vec::new();
        for other in &loaded.jobs {
            if other.id() == landed
                || other.status().is_terminal()
                || !self.same_repository(landed, other.id())
            {
                continue;
            }
            let Ok(theirs) = self.scope_claims(other).await else {
                continue;
            };
            let shared = collisions(&written, &theirs);
            if !shared.is_empty() {
                let paths = shared
                    .into_iter()
                    .map(|at| at.path.as_str().to_string())
                    .collect();
                owed.push((other.id().clone(), paths));
            }
        }
        let mut peering = self.peering().lock().await;
        for (job, paths) in owed {
            peering.owed.entry(job).or_default().push(News::Landed {
                title: this.title().as_str().to_string(),
                paths,
            });
        }
    }

    /// Tell each Drone what it is owed, where it has a live session and has not
    /// been told within [`SPACING`].
    ///
    /// **The slot before the queue, never the other way round**, the order a
    /// spawn already takes them in when it folds what is owed into a brief.
    pub(crate) async fn tell_peers(&self) {
        let now = self.now();
        let due: Vec<JobId> = {
            let peering = self.peering().lock().await;
            peering
                .owed
                .keys()
                .filter(|job| {
                    peering
                        .last_told
                        .get(*job)
                        .is_none_or(|last| elapsed(last, &now) >= SPACING)
                })
                .cloned()
                .collect()
        };
        for job in due {
            let Some(slot) = self.slot_of(&job).await else {
                self.forgotten_if_finished(&job).await;
                continue;
            };
            let working = slot.lock().await;
            let Some(at_work) = working.as_ref().filter(|at_work| at_work.is(&job)) else {
                continue;
            };
            let news = {
                let mut peering = self.peering().lock().await;
                peering.last_told.insert(job.clone(), now.clone());
                peering.owed.remove(&job).unwrap_or_default()
            };
            if news.is_empty() {
                continue;
            }
            let told = PeersChanged::injected(&news);
            at_work.instructed(Occasion::Peers, told.text());
            let _ = at_work.session().peers(&told).await;
        }
    }

    /// What this Job is owed, for the opening brief of the Drone being put on
    /// it. Taken off the queue, so it is said once.
    pub(crate) async fn peer_news_for_the_brief(&self, job: &JobId) -> Option<PeersChanged> {
        let mut peering = self.peering().lock().await;
        let news = peering.owed.remove(job).filter(|news| !news.is_empty())?;
        peering.last_told.insert(job.clone(), self.now());
        Some(PeersChanged::opening(&news))
    }

    /// Drop what a Job with no Drone is owed, once it will never have one.
    async fn forgotten_if_finished(&self, job: &JobId) {
        let finished = match self.load(job).await {
            Ok(record) => record.status().is_terminal(),
            // A Job nobody can read any more is one nobody will spawn onto.
            Err(_) => true,
        };
        if finished {
            self.peering().lock().await.owed.remove(job);
        }
    }
}
