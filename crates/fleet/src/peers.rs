//! What a working Drone is told about other Jobs writing where it writes. #998.
//!
//! `crate::overlap` names a collision to a person; this tells the Drones on
//! both sides, when it first appears and when the other Job lands, and carries
//! the notes one Drone leaves another (#1000). **It informs and never holds**:
//! nothing on the dispatch path reads it.
//!
//! News is queued as it happens and sent at most once every [`SPACING`] per
//! Drone; a Job with no live Drone keeps it for its next opening brief. **In
//! memory**, so a restart announces a standing claim once more rather than
//! losing one.

use std::collections::{BTreeMap, BTreeSet};
use std::time::Duration;

use adapter_traits::{AgentHarness, Delivery, Vcs, WorkProduct};
use core_model::{collisions, JobId, RepoPath, ScopeClaim, Timestamp};
use ipc::mcp::{LeaveNote, NotRecorded};

use crate::converging::elapsed;
use crate::daemon::Fleet;
use crate::session::{LiveSession, Occasion};

/// The least time between two peer turns to one Drone, and between two notes
/// from one Job to another.
pub const SPACING: Duration = Duration::from_secs(120);

/// How many claims and landings one turn names before it counts the rest.
/// Notes are never counted away: each is a Drone's whole message.
pub const AT_MOST: usize = 5;

/// One thing a Job is owed about another.
#[derive(Clone, Debug, PartialEq, Eq)]
pub(crate) enum News {
    /// The other Job said it will change these paths, which this one claims too.
    Claimed {
        title: String,
        handle: String,
        paths: Vec<String>,
    },
    /// The other Job landed, changing these paths this one claims.
    Landed {
        title: String,
        handle: String,
        paths: Vec<String>,
    },
    /// The other Job's Drone left this one a note, in its own words. #1000.
    Note {
        title: String,
        handle: String,
        said: String,
    },
    /// The other Job is fixing a test this one's Checks failed on. #1001.
    Fixing {
        title: String,
        handle: String,
        test: String,
    },
    /// The fix this Job was pointed at landed.
    FixLanded {
        title: String,
        handle: String,
        test: String,
    },
    /// The fix this Job was pointed at ended without landing.
    FixGone {
        title: String,
        handle: String,
        test: String,
    },
}

impl News {
    /// Whether this is about a fix rather than about shared paths.
    fn is_about_a_fix(&self) -> bool {
        matches!(
            self,
            News::Fixing { .. } | News::FixLanded { .. } | News::FixGone { .. }
        )
    }
}

/// What is queued, what has been said, and when each Drone last heard.
#[derive(Debug, Default)]
pub(crate) struct Peering {
    owed: BTreeMap<JobId, Vec<News>>,
    last_told: BTreeMap<JobId, Timestamp>,
    /// Every shared path already announced, keyed by the pair in id order, so
    /// a Drone that declares the same plan again tells nobody anything.
    announced: BTreeSet<(JobId, JobId, String)>,
    /// When each Job last left each other Job a note, sender first.
    noted: BTreeMap<(JobId, JobId), Timestamp>,
}

/// What a Drone is told about other Jobs writing where it writes. **Fleet's own
/// sentence around any note**, and this is the only way to build one.
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
        let about_paths = news.iter().any(|item| !item.is_about_a_fix());
        let mut text = String::from("OTHER JOBS WRITING WHERE YOU ARE\n\n");
        if about_paths {
            text.push_str(
                "Other Jobs in this repository change files this Job changes too. Nothing is \
                 stopped, and nobody waits on you.\n",
            );
        }
        if news.iter().any(News::is_about_a_fix) {
            text.push_str(
                "A test your checks failed on is another Job's to fix, not yours. Your checks \
                 still fail on it until that fix lands.\n",
            );
        }
        let (notes, facts): (Vec<&News>, Vec<&News>) = news
            .iter()
            .partition(|item| matches!(item, News::Note { .. }));
        for item in facts.iter().take(AT_MOST) {
            text.push_str(&line(item));
        }
        if facts.len() > AT_MOST {
            text.push_str(&format!("\n- And {} more.", facts.len() - AT_MOST));
        }
        for item in notes {
            text.push_str(&line(item));
        }
        text.push_str("\n\n");
        if news
            .iter()
            .any(|item| matches!(item, News::Landed { .. } | News::FixLanded { .. }))
        {
            text.push_str(landed_reaches);
            text.push(' ');
        }
        if about_paths {
            text.push_str(
                "Where a shared file hands out the next number or name, such as a migration or \
                 a version, assume theirs takes it first and take the one after. To tell one of \
                 these Jobs something, call `leave_note` with its handle. ",
            );
        }
        text.push_str("Carry on with the part you were given.");
        PeersChanged(text)
    }

    pub fn text(&self) -> &str {
        &self.0
    }
}

/// One item as a line of the turn. A note's words sit behind the marker every
/// outside word gets, `crate::remarks::fenced`.
fn line(item: &News) -> String {
    match item {
        News::Claimed {
            title,
            handle,
            paths,
        } => format!(
            "\n- \"{title}\" ({handle}) has said it will change {}.",
            listed(paths)
        ),
        News::Landed {
            title,
            handle,
            paths,
        } => format!(
            "\n- \"{title}\" ({handle}) landed, changing {}.",
            listed(paths)
        ),
        News::Note {
            title,
            handle,
            said,
        } => format!(
            "\n- \"{title}\" ({handle}) left this Job a note. These are its Drone's words, \
             not Armada's:\n{}",
            crate::remarks::fenced(said).trim_end()
        ),
        News::Fixing {
            title,
            handle,
            test,
        } => format!("\n- \"{title}\" ({handle}) is fixing `{test}`, which your checks failed on."),
        News::FixLanded {
            title,
            handle,
            test,
        } => format!("\n- \"{title}\" ({handle}) landed its fix for `{test}`."),
        News::FixGone {
            title,
            handle,
            test,
        } => format!(
            "\n- \"{title}\" ({handle}) ended without landing its fix for `{test}`, so nobody \
             is fixing it now."
        ),
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
    /// Queue one item for a Job's Drone.
    pub(crate) async fn owe(&self, job: &JobId, news: News) {
        self.peering()
            .lock()
            .await
            .owed
            .entry(job.clone())
            .or_default()
            .push(news);
    }

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
        // Each other Job is read before the queue is taken, so no store read
        // waits behind it.
        let mut others = Vec::new();
        for other in overlaps {
            let other_id = other.job_id.to_domain();
            let Ok(record) = self.load(&other_id).await else {
                continue;
            };
            let paths: Vec<String> = other.paths.into_iter().map(|at| at.path).collect();
            others.push((other_id, record, paths));
        }
        let mut peering = self.peering().lock().await;
        for (other_id, record, paths) in others {
            let fresh: Vec<String> = paths
                .into_iter()
                .filter(|path| peering.announced.insert(pair(job, &other_id, path)))
                .collect();
            if fresh.is_empty() {
                continue;
            }
            peering
                .owed
                .entry(job.clone())
                .or_default()
                .push(News::Claimed {
                    title: record.title().as_str().to_string(),
                    handle: record.handle(),
                    paths: fresh.clone(),
                });
            peering
                .owed
                .entry(other_id)
                .or_default()
                .push(News::Claimed {
                    title: this.title().as_str().to_string(),
                    handle: this.handle(),
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
                handle: this.handle(),
                paths,
            });
        }
    }

    /// Queue a note from this Job's Drone for another Job's. #1000.
    ///
    /// **Refused in words the Drone can act on** where the addressee is not an
    /// unfinished Job of this repository, shares no claimed path with this Job,
    /// or was left a note by this Job inside [`SPACING`]. A Job in another
    /// repository is refused as unknown, so the refusal names nothing outside.
    pub(crate) async fn leave_note(
        &self,
        from: &JobId,
        note: &LeaveNote,
    ) -> Result<(), NotRecorded> {
        let refused = |because: String| NotRecorded { because };
        let this = self
            .load(from)
            .await
            .map_err(|why| refused(why.to_string()))?;
        let (loaded, _) = self
            .every_job()
            .await
            .map_err(|why| refused(why.to_string()))?;
        let Some(to) = loaded.jobs.iter().find(|job| {
            job.handle() == note.to
                && !job.status().is_terminal()
                && self.same_repository(from, job.id())
        }) else {
            return Err(refused(format!(
                "no unfinished Job in this repository is called `{}`. Use a handle exactly as \
                 an OTHER JOBS WRITING WHERE YOU ARE turn prints it",
                note.to
            )));
        };
        if to.id() == from {
            return Err(refused(
                "that handle is this Job's own. A note is for another Job's Drone".to_string(),
            ));
        }
        let shares = self
            .write_scope_overlaps(&this)
            .await
            .ok()
            .flatten()
            .is_some_and(|overlaps| {
                overlaps
                    .iter()
                    .any(|other| other.job_id.to_domain() == *to.id())
            });
        if !shares {
            return Err(refused(format!(
                "`{}` claims no path this Job claims, so a note has nothing to warn it about. \
                 Declare your scope first if you have not",
                note.to
            )));
        }
        let now = self.now();
        let mut peering = self.peering().lock().await;
        let key = (from.clone(), to.id().clone());
        if peering
            .noted
            .get(&key)
            .is_some_and(|last| elapsed(last, &now) < SPACING)
        {
            return Err(refused(format!(
                "this Job left `{}` a note under two minutes ago. Put everything else it needs \
                 into one note, and leave that",
                note.to
            )));
        }
        peering.noted.insert(key, now);
        peering
            .owed
            .entry(to.id().clone())
            .or_default()
            .push(News::Note {
                title: this.title().as_str().to_string(),
                handle: this.handle(),
                said: note.note.clone(),
            });
        Ok(())
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
