//! What a run is, once whose tree it is in has been settled.
//!
//! **Every function here takes a [`Place`] and never a `JobId`.** Which tree,
//! whose ports and where the records go were decided in [`owner`](super::owner);
//! what is left is one command in one directory, and it is the same act for a
//! Job and for the main checkout. The two entrances — [`super`] and
//! [`checkout`](super::checkout) — differ only in the shapes they project.

use std::path::{Path, PathBuf};
use std::sync::Arc;

use adapter_traits::{AgentHarness, Delivery, Vcs, WorkProduct};
use api::Refusal;
use checks_runner::Narrowed;
use core_model::Timestamp;
use ipc::WireError;
use tokio::sync::watch;

use super::entries::{self, Entry};
use super::owner::{Owner, Place, Tree};
use super::record::{Record, Underway};
use super::{records, running, Seen, Unrehearsable, STOPPING};
use crate::daemon::Fleet;

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
    /// The entry a run names and the tree it would run in, checked in the
    /// order that refuses before anything is written.
    pub(super) async fn entry_at(
        &self,
        place: &Place,
        name: &str,
        worktree_version: bool,
    ) -> Result<(Entry, Tree), Unrehearsable> {
        let whose = place.whose();
        // A server never exits, so a run of one would hold the owner's one run
        // slot for good — and Fleet would not know to hand it on or stop it.
        let (manifest, has_snapshot) = self.manifest_at(place).await;
        if manifest.server(name).is_some() {
            return Err(Unrehearsable::IsAServer {
                name: name.to_string(),
            });
        }
        let Some(tree) = self.tree_at(place) else {
            return Err(Unrehearsable::NoWorktree);
        };
        if let Some(out) = self.rehearsals().in_flight(&place.owner) {
            return Err(Unrehearsable::AlreadyRunning {
                name: out.name,
                whose,
            });
        }
        let listed = match (worktree_version, place.job.as_ref()) {
            (true, _) => {
                let theirs = self
                    .theirs(&tree)
                    .map_err(|why| Unrehearsable::WorktreeManifest { why })?;
                entries::declared(&theirs)
            }
            (false, Some(job)) => entries::frozen(job, &manifest, has_snapshot),
            // The main checkout froze nothing: the file Fleet holds is the one
            // that runs, and `worktree_version` has nothing to mean there.
            (false, None) => entries::declared(&manifest),
        };
        Ok((listed.named(name, whose)?, tree))
    }

    /// Take the owner's one run slot, make the run's directory, and spawn it.
    /// `verify` is the Verify this run is a step of, where it is one.
    #[allow(clippy::too_many_arguments)]
    pub(super) async fn started_at(
        self: Arc<Self>,
        place: Place,
        tree: Tree,
        entry: Entry,
        command: String,
        narrowed: bool,
        worktree_version: bool,
        verify: Option<tokio::sync::oneshot::Sender<super::verifying::Handed>>,
    ) -> Result<Underway, Unrehearsable> {
        let whose = place.whose();
        let underway = Underway {
            id: self.mint().ulid().as_str().to_string(),
            name: entry.name.clone(),
            command,
            narrowed,
            started_at: ipc::Instant::from(&self.now()),
        };
        let (stop, stopped) = watch::channel(false);
        let (done, ended) = watch::channel(None);
        let feed = api::RunFeed::new();
        let Some(held) = self
            .rehearsals()
            .take(&place.owner, &underway, stop, ended, feed.clone())
        else {
            return Err(Unrehearsable::AlreadyRunning {
                name: entry.name,
                whose,
            });
        };
        let (root, handle) = (&self.host().records_root, place.handle.clone());
        records::swept(
            root,
            &handle,
            &self.now(),
            self.run_log_retention(),
            |reference| {
                let _ = adapters::snapshot::forget(&tree.path, reference);
            },
        );
        let dir =
            records::made(root, &handle, &underway.id).map_err(|why| Unrehearsable::NotKept {
                why: why.to_string(),
            })?;
        let plan = running::Plan {
            place,
            tree,
            entry,
            underway: underway.clone(),
            dir,
            worktree_version,
            feed,
            verify,
        };
        let this = Arc::clone(&self);
        tokio::spawn(async move { this.rehearsed(plan, held, stopped, done).await });
        Ok(underway)
    }

    pub(super) async fn stopped_at(
        &self,
        place: &Place,
        id: String,
    ) -> Result<Record, Unrehearsable> {
        let (root, handle) = (&self.host().records_root, place.handle.as_str());
        let Some((stop, mut done)) = self.rehearsals().stopping(&place.owner, &id) else {
            return Err(match records::read(root, handle, &id) {
                Some(Ok(_)) => Unrehearsable::NotRunning { id },
                _ => Unrehearsable::NoSuchRun {
                    id,
                    whose: place.whose(),
                },
            });
        };
        let _ = stop.send(true);
        let ended = tokio::time::timeout(STOPPING, done.wait_for(Option::is_some)).await;
        match ended {
            Ok(Ok(seen)) => seen.clone(),
            _ => None,
        }
        .ok_or_else(|| Unrehearsable::NotKept {
            why: String::from("the run did not write its record after it was stopped"),
        })
    }

    pub(super) async fn undone_at(
        &self,
        place: &Place,
        id: String,
    ) -> Result<Record, Unrehearsable> {
        if self.rehearsals().in_flight(&place.owner).is_some() {
            return Err(Unrehearsable::RunInFlight);
        }
        let Some(tree) = self.tree_at(place) else {
            return Err(Unrehearsable::NoWorktree);
        };
        let (root, handle) = (&self.host().records_root, place.handle.as_str());
        let mut record = match records::read(root, handle, &id) {
            Some(Ok(record)) => record,
            Some(Err(why)) => return Err(Unrehearsable::NothingToUndo { id, why }),
            None => {
                return Err(Unrehearsable::NoSuchRun {
                    id,
                    whose: place.whose(),
                })
            }
        };
        let nothing = |why: &str| Unrehearsable::NothingToUndo {
            id: record.id.clone(),
            why: why.to_string(),
        };
        if record.undone_at.is_some() {
            return Err(Unrehearsable::AlreadyUndone { id });
        }
        if record.changed.is_empty() {
            return Err(nothing("the run changed nothing"));
        }
        // **Never offered where no snapshot was taken.** This can be the tree
        // holding a person's own uncommitted work, and nothing puts that back
        // without a copy of what was there before.
        let Some(reference) = record.snapshot.clone() else {
            return Err(nothing(
                record
                    .changed_unreadable
                    .as_deref()
                    .unwrap_or("no snapshot was kept"),
            ));
        };
        let path = tree.path.clone();
        let undone =
            tokio::task::spawn_blocking(move || adapters::snapshot::undo(&path, &reference)).await;
        match undone {
            Ok(Ok(_)) => {}
            Ok(Err(adapters::snapshot::NotUndone::Moved { paths })) => {
                return Err(Unrehearsable::Moved { paths })
            }
            Ok(Err(adapters::snapshot::NotUndone::NotRestored { path, cause })) => {
                return Err(Unrehearsable::NotKept {
                    why: format!("{path} could not be written back: {cause}"),
                })
            }
            Ok(Err(other)) => return Err(nothing(&other.to_string())),
            Err(_) => return Err(nothing("the undo did not finish")),
        }
        record.undone_at = Some(ipc::Instant::from(&self.now()));
        let kept = records::dir_of(root, handle, &record.id)
            .map(|dir| records::write(&dir, &record))
            .unwrap_or_else(|| Err(std::io::Error::other("not one path component")));
        kept.map_err(|why| Unrehearsable::NotKept {
            why: format!("the tree was put back and the record was not: {why}"),
        })?;
        self.noted_undo(place.job.as_ref(), &record);
        Ok(record)
    }

    pub(super) fn history_at(&self, place: &Place) -> (Vec<Record>, Vec<ipc::UnreadableRun>) {
        let running = self.rehearsals().in_flight(&place.owner).map(|out| out.id);
        records::every(&self.host().records_root, &place.handle, running.as_deref())
    }

    /// One run's log. **The owner's own runs are the allowlist**: an id that
    /// names none of them reaches no file.
    pub(super) fn output_at(&self, place: &Place, id: &str) -> Option<ipc::RunOutput> {
        let (root, handle) = (&self.host().records_root, place.handle.as_str());
        let name = match self
            .rehearsals()
            .in_flight(&place.owner)
            .filter(|out| out.id == id)
        {
            Some(out) => Some(out.name),
            None => match records::read(root, handle, id) {
                Some(Ok(record)) => Some(record.name),
                _ => None,
            },
        };
        name.and_then(|name| records::output(root, handle, id, name))
    }

    pub(super) fn observed_at(&self, place: &Place, id: &str) -> Option<Seen> {
        let (root, handle) = (&self.host().records_root, place.handle.as_str());
        let (name, live) = match self.rehearsals().watching(&place.owner, id) {
            Some((name, watch)) => (name, Some(watch)),
            None => match records::read(root, handle, id) {
                Some(Ok(record)) => (record.name, None),
                _ => return None,
            },
        };
        let log = records::dir_of(root, handle, id)?.join(records::LOG);
        let (history, skipped, read_to, unreadable) = match records::history(&log, live.is_none()) {
            Some((history, skipped, read_to)) => (history, skipped, read_to, false),
            None => (Vec::new(), 0, 0, true),
        };
        Some(Seen {
            id: id.to_string(),
            name,
            path: records::relative_log(handle, id),
            live,
            history,
            skipped,
            read_to,
            unreadable,
        })
    }

    /// A record answered to a Job's caller. The `None` arm is unreachable from
    /// a Job's entrance — a directory under the Job's handle carries its id —
    /// and is a fault rather than an invented record if it ever is reached.
    pub(super) fn only_a_jobs(
        &self,
        place: &Place,
        record: Record,
    ) -> Result<ipc::RunRecord, Refusal> {
        record.of_job().ok_or_else(|| {
            self.refused_run(
                &place.owner,
                Unrehearsable::NotKept {
                    why: String::from("this run's record names no Job"),
                },
            )
        })
    }

    pub(super) fn no_such_run(&self, place: &Place, id: String) -> Refusal {
        self.refused_run(
            &place.owner,
            Unrehearsable::NoSuchRun {
                id,
                whose: place.whose(),
            },
        )
    }

    pub(super) fn refused_run(&self, owner: &Owner, why: Unrehearsable) -> Refusal {
        let (code, refusal) = why.spelled();
        let raised = WireError::raised(code, why.to_string(), self.run_id());
        refusal(match owner {
            Owner::Job(job) => raised.about_job(ipc::JobId::from(job)),
            Owner::Checkout => raised,
        })
    }

    /// What the Job changed, for a narrowing. A worktree that will not read
    /// is no change here: the sheet then offers the whole tree and nothing else.
    pub(super) fn changed_in(&self, tree: &Tree) -> Vec<String> {
        tree.worktree
            .as_ref()
            .and_then(|worktree| self.work().changed_files(worktree).ok())
            .map(|changed| changed.paths())
            .unwrap_or_default()
    }

    /// The command a narrowed run of `entry` is, **resolved the way a Drone's
    /// dry run resolves it** — the worktree's own diff through the same call.
    pub(super) fn narrowed(
        &self,
        entry: &Entry,
        tree: &Tree,
    ) -> Result<(String, bool), Unrehearsable> {
        let does_not = || Unrehearsable::DoesNotNarrow {
            name: entry.name.clone(),
        };
        let (Some(narrow), Some(worktree)) = (entry.narrow.as_ref(), tree.worktree.as_ref()) else {
            return Err(does_not());
        };
        let changed = self
            .work()
            .changed_files(worktree)
            .map_err(|why| Unrehearsable::Unreadable {
                why: why.to_string(),
            })?
            .paths();
        let nothing = || Unrehearsable::NothingToNarrowTo {
            name: entry.name.clone(),
        };
        if changed.is_empty() {
            return Err(nothing());
        }
        match checks_runner::narrowed(Some(narrow), &changed) {
            Narrowed::To(command) => Ok((command, true)),
            Narrowed::Nothing => Err(nothing()),
            // A path the narrowing cannot spell as one argument. Whole is
            // never less than narrow, and `narrowed: false` says which ran.
            Narrowed::Whole => Ok((entry.run.clone(), false)),
        }
    }

    /// Where the Manifest Fleet holds sits, relative to the repository, so the
    /// same file can be read in a worktree and in git.
    pub(super) fn manifest_file(&self) -> PathBuf {
        let held = self.manifest().path();
        held.strip_prefix(&self.host().repo_root)
            .map(Path::to_path_buf)
            .unwrap_or_else(|_| {
                held.file_name()
                    .map(PathBuf::from)
                    .unwrap_or_else(|| PathBuf::from("armada.yml"))
            })
    }

    /// The worktree's own Manifest, or why it would not read.
    pub(super) fn theirs(&self, tree: &Tree) -> Result<config::Manifest, String> {
        config::Manifest::load(&tree.path.join(self.manifest_file())).map_err(|why| why.to_string())
    }

    /// When the Manifest was last changed by a commit made at or before
    /// `before` — for a Job, **before it froze it**, not the latest edit.
    pub(super) fn edited_before(
        &self,
        before: &Timestamp,
        tree: Option<&Tree>,
    ) -> Option<ipc::Instant> {
        let before = before.epoch_millis()?.div_euclid(1_000);
        let root = tree.map_or_else(|| PathBuf::from(&self.host().repo_root), |t| t.path.clone());
        let file = self.manifest_file();
        let seconds =
            adapters::snapshot::last_touched(&root, &file.to_string_lossy(), before).ok()??;
        let at = Timestamp::from_rfc3339(crate::clock::rfc3339_utc(seconds.saturating_mul(1_000)));
        Some(ipc::Instant::from(&at))
    }
}
