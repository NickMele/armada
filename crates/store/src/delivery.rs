//! What a finished Job's branch came to, and the four columns it waits in.
//!
//! **Three of them are written once and the fourth is written later.** The
//! commit, the push and the pull request are what the finishing turn did; what
//! became of that pull request is settled after that turn is over, whether a
//! person merged it on the forge or pressed for Fleet to. So `delivery_landed`
//! has its own writer, its own migration and its own reason to be null.
//!
//! **The record existed and was thrown away.** `fleet::delivery` has committed,
//! pushed and opened a pull request since before this file, and put the result
//! in a map that `take_delivered` *drains* into the Drone's closing turn. So the
//! one reader was a process about to exit, and a person opening the Job an hour
//! later was told "Fleet does not open one yet" — which had stopped being true.
//!
//! **Columns on `jobs`, not a table.** This is at most one row per Job, written
//! once, and never queried except beside the Job it belongs to. `jobs.branch`
//! is the same shape for the same reason.
//!
//! **The column is the authority for its field**, as [`crate::note`]'s is: no
//! event carries a commit or a URL, so there is nothing to fold and
//! [`crate::read`] reads these straight back.

use std::collections::BTreeMap;

use adapter_traits::Landing;
use core_model::{JobId, Timestamp, Ulid};

use crate::error::{fault, WriteError};
use crate::open::Store;

/// Version 21 — what a Job's branch came to when it was sent out.
///
/// Beside the change it makes, like [`V20`](crate::note::V20): `schema.rs` is
/// at the 900 the gate refuses at.
///
/// Three nullable columns and no backfill. A Job written before this has none,
/// and null already means "nothing was recorded" — which is exactly true of
/// every Job that finished while the result was being dropped.
pub(crate) const V21: &str = r#"
ALTER TABLE jobs ADD COLUMN delivery_commit TEXT;
ALTER TABLE jobs ADD COLUMN delivery_pushed TEXT;
ALTER TABLE jobs ADD COLUMN delivery_pull_request TEXT;
"#;

/// Version 26 — what became of the pull request after somebody read it.
///
/// **A second write to the same three-column story, and the reason it is two
/// migrations rather than one.** [`V21`]'s columns are written once, by the
/// turn that finishes a Job, and are complete the moment they land. This one
/// is written by a later turn that asked the forge a question the finishing
/// turn could not have answered — Armada opens a pull request and a person
/// merges it — so it is a different fact with a different writer.
///
/// One nullable column and no backfill: a Job whose pull request merged before
/// this shipped reads as unasked, which is exactly what it is.
pub(crate) const V26: &str = r#"
ALTER TABLE jobs ADD COLUMN delivery_landed TEXT;
"#;

/// Version 51 — whether the branch a delivery describes ever reached its
/// remote. `#691`.
///
/// **One nullable column, and null is the ordinary answer.** A clean push, a
/// repository with no base to catch up to, and a worktree with nothing new to
/// commit are the same fact here: nothing is owed to a remote that does not
/// already have it. Non-null is the one case this exists for — catching the
/// branch up to its base conflicted, or its own commits would not replay, so
/// `fleet::delivery::deliver` left the commit standing in the worktree and
/// this names why, in the same words the Job's log carries.
pub(crate) const V51: &str = r#"
ALTER TABLE jobs ADD COLUMN delivery_unpushed_reason TEXT;
"#;

/// Version 48 — the base a pull request's branch was last brought up to,
/// where main moved under it. `#663`, in place of the close-and-reopen it
/// replaced.
///
/// **Durable, unlike the in-memory guard it replaces.** `fleet::noticing`
/// used to remember what it had closed and reopened in a map that did not
/// survive a restart, and #660 was closed and reopened twice for the same
/// base because of it. This is read back before a rebase is attempted, so
/// the memory outlives the process that wrote it.
///
/// Three nullable columns and no backfill, [`V21`]'s shape: a Job whose
/// branch has never needed to move has nothing to say here, and a Job written
/// before this column existed reads exactly the same as one that has not.
pub(crate) const V48: &str = r#"
ALTER TABLE jobs ADD COLUMN delivery_rebased_onto TEXT;
ALTER TABLE jobs ADD COLUMN delivery_rebased_at TEXT;
ALTER TABLE jobs ADD COLUMN delivery_rebase_conflict_files TEXT;
"#;

/// What a Job's branch came to, as the record holds it.
///
/// **Three independent absences, not one.** A commit with no push is a
/// repository with no remote; a push with no pull request is a forge nothing on
/// this machine can open one against. Both are ordinary, and folding them into
/// one optional would make a surface unable to tell either from a Job that
/// finished before any of this was written down.
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct Delivery {
    /// The commit Fleet wrote over the Job's work.
    pub commit: Option<String>,
    /// What the push came to, in the adapter's own words.
    pub pushed: Option<String>,
    /// The address a person clicks. Absent where none was opened.
    pub pull_request: Option<String>,
    /// What became of that pull request, once somebody asked the forge and got
    /// a settled answer. **Absent is unasked or still open** — the two are one
    /// absence here because neither is news, and a column that stored "still
    /// open" would be storing the absence of it.
    ///
    /// [`adapter_traits::Landing`] and not a word of this crate's own, for the
    /// reason the `change` column takes the adapter's vocabulary: a third
    /// spelling of the same three states is a third thing to keep in step. The
    /// URL it carries is [`pull_request`](Delivery::pull_request) — the column
    /// holds the state alone, so a row cannot say two addresses.
    pub landed: Option<Landing>,
    /// Why the commit named above never reached the branch's remote, where it
    /// did not. `#691`.
    ///
    /// **Absent is not "unknown"** — it is a push that went out, a repository
    /// with no remote to fail against, or a Job that has not reached a
    /// delivering step at all. Present is the one fact this column exists
    /// for: the commit stands in the worktree and the pull request, if one is
    /// already open, still shows what it did before this attempt.
    pub unpushed: Option<String>,
}

impl Delivery {
    /// Whether there is anything here worth serving.
    ///
    /// A Job that finished before version 21, and one whose delivery failed
    /// before the commit, are the same answer: nothing to say.
    pub fn is_empty(&self) -> bool {
        self.commit.is_none()
            && self.pushed.is_none()
            && self.pull_request.is_none()
            && self.landed.is_none()
            && self.unpushed.is_none()
    }
}

/// What the last attempt to keep a pull request's branch current against a
/// moved base came to.
///
/// **Written once per base, and read back before the next attempt.** That is
/// the whole of what makes a conflicted rebase retried once per move of main
/// rather than once per sweep — `fleet::currency` compares `onto` against the
/// base's tip *now* and does nothing where they already agree.
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct Currency {
    /// The base's tip this was last attempted against. `None` is a branch
    /// that has never needed to move — the ordinary case for most of a pull
    /// request's life.
    pub onto: Option<String>,
    pub at: Option<Timestamp>,
    /// Absent on a clean rebase or where nothing has been attempted yet.
    /// Present — and never empty — where the attempt against `onto`
    /// conflicted and the branch was left exactly as it was.
    pub conflict_files: Option<Vec<String>>,
}

impl Currency {
    /// Whether the branch is currently behind `tip` with conflicts nobody has
    /// resolved — what a review panel offers a person the Drone for.
    pub fn conflicted_against(&self, tip: &str) -> bool {
        self.onto.as_deref() == Some(tip) && self.conflict_files.is_some()
    }
}

/// A read of these columns that would not come back, as a row-level fault.
fn unreadable(cause: rusqlite::Error) -> crate::error::LoadJobError {
    crate::error::LoadJobError::Unreadable(crate::error::RowError::Database(fault(
        "reading what became of the pull requests",
    )(cause)))
}

/// A Job whose pull request exists and whose fate nobody has read yet.
///
/// **The address and not the branch**, because the address is what the forge is
/// asked about: a merged branch is usually deleted, and a Job's worktree is
/// reclaimed long before anybody merges its work — so the one handle that still
/// resolves when the answer matters is the URL the record kept.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Unsettled {
    pub job_id: JobId,
    pub url: String,
}

/// The word the `delivery_landed` column holds, for a state worth storing.
///
/// **`Open` and `Unknown` have no word**, which is the column's whole shape:
/// only a settled answer is written down, so a `None` here refuses to store the
/// absence of news. [`Landing::is_settled`] is the same rule said at the call
/// site, and the two are read together.
fn as_stored(landed: &Landing) -> Option<&'static str> {
    match landed {
        Landing::Merged { .. } => Some("merged"),
        Landing::ClosedUnmerged { .. } => Some("closed_unmerged"),
        Landing::Open { .. } | Landing::Unknown => None,
    }
}

/// The state back out of the column, given the address the row already holds.
///
/// A word this version does not know, and a settled state with no address to
/// go with it, are both `None`: neither is something a surface could draw.
fn from_stored(word: Option<String>, url: Option<&String>) -> Option<Landing> {
    let url = url?.clone();
    match word?.as_str() {
        "merged" => Some(Landing::Merged { url }),
        "closed_unmerged" => Some(Landing::ClosedUnmerged { url }),
        _ => None,
    }
}

impl Store {
    /// Write what the Job's branch came to.
    ///
    /// **Once, at the end**, from `fleet::landing` after the delivery it
    /// describes. Every field is written including `None`, for
    /// [`record_redirect_waiting`](Store::record_redirect_waiting)'s reason: a
    /// method that could only set would leave clearing to a second spelling of
    /// the same `UPDATE`, and a redispatched Job delivering again must not
    /// inherit the last run's URL.
    pub fn record_delivery(
        &mut self,
        job_id: &core_model::JobId,
        delivery: &Delivery,
    ) -> Result<(), WriteError> {
        let updated = self
            .conn
            .execute(
                "UPDATE jobs SET delivery_commit = ?2, delivery_pushed = ?3, \
                 delivery_pull_request = ?4, delivery_landed = ?5, \
                 delivery_unpushed_reason = ?6 WHERE job_id = ?1",
                (
                    job_id.as_str(),
                    delivery.commit.as_deref(),
                    delivery.pushed.as_deref(),
                    delivery.pull_request.as_deref(),
                    delivery.landed.as_ref().and_then(as_stored),
                    delivery.unpushed.as_deref(),
                ),
            )
            .map_err(fault("recording what the branch came to"))
            .map_err(WriteError::Database)?;
        if updated == 0 {
            return Err(WriteError::NoSuchJob {
                job_id: job_id.clone(),
            });
        }
        Ok(())
    }

    /// Read what a Job's branch came to. Every field absent is every Job that
    /// has not finished, and every Job that finished before version 21.
    /// Write what became of the pull request, and nothing else on the row.
    ///
    /// **Its own `UPDATE` and not [`record_delivery`](Store::record_delivery)**,
    /// because the two have different writers at different times: that one is
    /// the turn that finished the Job, writing every field it owns including
    /// the nulls, and this is a later turn that asked the forge a question the
    /// finishing turn could not have answered. A merge written through that
    /// method would have to restate a commit and a push to keep them, and a
    /// caller that got one of them wrong would erase it.
    ///
    /// **Only a settled answer is written.** `Open` and `Unknown` return
    /// without touching the row: neither is news, and the next sweep asks
    /// again. That is what makes a merge recorded once and never re-asked.
    pub fn record_landed(&mut self, job_id: &JobId, landed: &Landing) -> Result<(), WriteError> {
        let Some(word) = as_stored(landed) else {
            return Ok(());
        };
        let updated = self
            .conn
            .execute(
                "UPDATE jobs SET delivery_landed = ?2 WHERE job_id = ?1",
                (job_id.as_str(), word),
            )
            .map_err(fault("recording what became of the pull request"))
            .map_err(WriteError::Database)?;
        if updated == 0 {
            return Err(WriteError::NoSuchJob {
                job_id: job_id.clone(),
            });
        }
        Ok(())
    }

    /// Write what the last attempt to keep this pull request's branch current
    /// came to. **Its own `UPDATE`**, for [`record_landed`](Store::record_landed)'s
    /// reason: a later turn asking a question the finishing turn could not
    /// have answered, writing columns nothing else touches.
    pub fn record_kept_current(
        &mut self,
        job_id: &JobId,
        onto: &str,
        at: &Timestamp,
        conflict_files: Option<&[String]>,
    ) -> Result<(), WriteError> {
        // Newline-joined rather than JSON: every path here is a line
        // `git diff --name-only` printed, so none of them holds one, and a
        // column this narrow does not earn a parser.
        let files = conflict_files.map(|files| files.join("\n"));
        let updated = self
            .conn
            .execute(
                "UPDATE jobs SET delivery_rebased_onto = ?2, delivery_rebased_at = ?3, \
                 delivery_rebase_conflict_files = ?4 WHERE job_id = ?1",
                (job_id.as_str(), onto, at.as_str(), files),
            )
            .map_err(fault("recording an attempt to keep a pull request current"))
            .map_err(WriteError::Database)?;
        if updated == 0 {
            return Err(WriteError::NoSuchJob {
                job_id: job_id.clone(),
            });
        }
        Ok(())
    }

    /// Read what the last attempt to keep this pull request's branch current
    /// came to. [`Currency::default`] is every Job that has never needed one.
    pub fn kept_current_for(&self, job_id: &JobId) -> Result<Currency, crate::error::LoadJobError> {
        self.conn
            .query_row(
                "SELECT delivery_rebased_onto, delivery_rebased_at, \
                 delivery_rebase_conflict_files FROM jobs WHERE job_id = ?1",
                (job_id.as_str(),),
                |row| {
                    let files: Option<String> = row.get(2)?;
                    let at: Option<String> = row.get(1)?;
                    Ok(Currency {
                        onto: row.get(0)?,
                        at: at.map(Timestamp::from_rfc3339),
                        conflict_files: files
                            .map(|held| held.split('\n').map(str::to_string).collect()),
                    })
                },
            )
            .or_else(|why| match why {
                rusqlite::Error::QueryReturnedNoRows => Ok(Currency::default()),
                other => Err(crate::error::LoadJobError::Unreadable(
                    crate::error::RowError::Database(fault(
                        "reading what a rebase against a moved base came to",
                    )(other)),
                )),
            })
    }

    /// Every Job that has a pull request nobody has settled, with its branch
    /// and its address. **One query for the whole board**, which is what makes
    /// asking about merges affordable at all: the alternative is a read per Job
    /// on a turn that runs four times a second.
    ///
    /// A Job with no pull request is not here, so a repository with no remote
    /// is never asked about — there is nothing to ask.
    pub fn pull_requests_unsettled(&self) -> Result<Vec<Unsettled>, crate::error::LoadJobError> {
        let mut asking = self
            .conn
            .prepare(
                "SELECT job_id, delivery_pull_request FROM jobs \
                 WHERE delivery_pull_request IS NOT NULL AND delivery_landed IS NULL \
                 ORDER BY job_id",
            )
            .map_err(unreadable)?;
        let rows = asking
            .query_map((), |row| {
                Ok(Unsettled {
                    job_id: JobId::carried(Ulid::carried(row.get::<_, String>(0)?)),
                    url: row.get(1)?,
                })
            })
            .map_err(unreadable)?;
        rows.collect::<Result<Vec<Unsettled>, rusqlite::Error>>()
            .map_err(unreadable)
    }

    /// What became of every pull request that has settled, keyed by Job.
    ///
    /// **One query, for the same reason as the one above.** The Board draws
    /// every row it holds, and a merge read per row would be a query per row on
    /// a list that redraws on every event.
    pub fn landed_by_job(&self) -> Result<BTreeMap<JobId, Landing>, crate::error::LoadJobError> {
        let mut asking = self
            .conn
            .prepare(
                "SELECT job_id, delivery_landed, delivery_pull_request FROM jobs \
                 WHERE delivery_landed IS NOT NULL",
            )
            .map_err(unreadable)?;
        let rows = asking
            .query_map((), |row| {
                let job_id = JobId::carried(Ulid::carried(row.get::<_, String>(0)?));
                let url: Option<String> = row.get(2)?;
                Ok((job_id, from_stored(row.get(1)?, url.as_ref())))
            })
            .map_err(unreadable)?;
        let mut held = BTreeMap::new();
        for row in rows {
            let (job_id, landed) = row.map_err(unreadable)?;
            if let Some(landed) = landed {
                held.insert(job_id, landed);
            }
        }
        Ok(held)
    }

    pub fn delivery_for(
        &self,
        job_id: &core_model::JobId,
    ) -> Result<Delivery, crate::error::LoadJobError> {
        self.conn
            .query_row(
                "SELECT delivery_commit, delivery_pushed, delivery_pull_request, \
                 delivery_landed, delivery_unpushed_reason FROM jobs WHERE job_id = ?1",
                (job_id.as_str(),),
                |row| {
                    let pull_request: Option<String> = row.get(2)?;
                    Ok(Delivery {
                        commit: row.get(0)?,
                        pushed: row.get(1)?,
                        landed: from_stored(row.get(3)?, pull_request.as_ref()),
                        pull_request,
                        unpushed: row.get(4)?,
                    })
                },
            )
            .or_else(|why| match why {
                rusqlite::Error::QueryReturnedNoRows => Ok(Delivery::default()),
                other => Err(crate::error::LoadJobError::Unreadable(
                    crate::error::RowError::Database(fault("reading what the branch came to")(
                        other,
                    )),
                )),
            })
    }
}
