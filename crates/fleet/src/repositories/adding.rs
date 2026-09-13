//! Fleet over its [`Repositories`](super::Repositories): which one a Job or a
//! request is in, and adding one by folder.

use std::path::PathBuf;
use std::sync::Arc;

use adapter_traits::{AgentHarness, Delivery, Vcs, WorkProduct};
use api::Refusal;
use core_model::{Job, JobId, JobReference};
use ipc::{AddRepository, RepositoryList, RepositorySummary, WireError};
use store::{NamedJob, ResolveJobError};

use super::{NotAdded, NotLocated, Repository, Served};
use crate::adrift::Adrift;
use crate::daemon::Fleet;
use crate::wire::manifest_summary;

/// A folder that is not a repository's root. A 422.
const NOT_A_REPOSITORY: &str = "fleet.not_a_repository";
/// A repository whose `armada.yml` or workflows will not load. A 422.
const REPOSITORY_REFUSED: &str = "fleet.repository_refused";
/// A folder or a Manifest id already served. A 409.
const ALREADY_SERVED: &str = "fleet.repository_served";
/// A `?repository=` naming nothing served. A 422.
const NO_SUCH_REPOSITORY: &str = "fleet.no_such_repository";

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
    /// The repository a Job was created in. **Refused, never guessed**: one
    /// store serves the machine, and a Job whose repository this Fleet does not
    /// serve would otherwise be worked in a repository it never ran in.
    pub(crate) fn served_by(&self, job: &Job) -> Result<Served, Adrift> {
        self.repositories()
            .serving(job.owner_manifest_id().as_str())
            .ok_or_else(|| Adrift::NotServed {
                job: job.id().clone(),
                manifest: job.owner_manifest_id().as_str().to_string(),
            })
    }

    /// The repository a Job is in, for the paths that hold only its id.
    pub(crate) fn served_by_id(&self, job: &JobId) -> Result<Served, Adrift> {
        let manifest = self.names().owner_of(job).unwrap_or_default();
        self.repositories()
            .serving(&manifest)
            .ok_or(Adrift::NotServed {
                job: job.clone(),
                manifest,
            })
    }

    /// The repository a Manifest-scoped request names. Absent is the one
    /// Fleet was started in, which is every request a Bridge sent before this.
    pub(crate) fn served_named(&self, named: Option<&ipc::ManifestId>) -> Result<Served, Refusal> {
        let Some(named) = named else {
            return Ok(self.repositories().first());
        };
        self.repositories().serving(named.as_str()).ok_or_else(|| {
            self.refusal(Adrift::NoSuchManifest {
                named: named.as_str().to_string(),
                held: self.held_manifests(),
            })
        })
    }

    /// Every Manifest id served, for a refusal to name.
    pub(crate) fn held_manifests(&self) -> String {
        let held: Vec<String> = self
            .repositories()
            .served()
            .iter()
            .map(|one| one.manifest().id().as_str().to_string())
            .collect();
        held.join("`, `")
    }

    /// The repository Scan and its proposals read, by root. Absent is the
    /// first, for [`served_named`](Self::served_named)'s reason.
    pub(crate) fn repository_named(&self, root: Option<&str>) -> Result<Arc<Repository>, Refusal> {
        let Some(root) = root else {
            return Ok(Arc::clone(self.repositories().first().repository()));
        };
        self.repositories().at(root).ok_or_else(|| {
            Refusal::Unacceptable(WireError::raised(
                NO_SUCH_REPOSITORY,
                format!("{root} is not a repository this Fleet serves; `list_repositories` says which are"),
                self.run_id(),
            ))
        })
    }

    /// A reference resolved across every repository served. An id names one Job
    /// wherever it is; a number or handle more than one repository holds is
    /// refused as naming no one Job.
    pub(crate) async fn resolve_job_across(
        &self,
        reference: &JobReference,
        served: &[Served],
    ) -> Result<NamedJob, ResolveJobError> {
        let store = self.store().lock().await;
        if let JobReference::Id(_) = reference {
            return store.resolve_job(reference, None);
        }
        let mut found = Vec::new();
        for one in served {
            match store.resolve_job(reference, Some(one.manifest().id())) {
                Ok(named) => found.push(named),
                Err(ResolveJobError::NoSuchJob { .. }) => {}
                Err(why) => return Err(why),
            }
        }
        let named = reference.to_string();
        match found.len() {
            1 => Ok(found.remove(0)),
            0 => Err(ResolveJobError::NoSuchJob { named }),
            _ => Err(ResolveJobError::NoManifest { named }),
        }
    }

    /// The repository Fleet was started in, for a test that has only the one.
    #[cfg(test)]
    pub(crate) fn first(&self) -> Served {
        self.repositories().first()
    }

    /// `list_repositories`.
    pub(crate) fn repository_list(&self) -> RepositoryList {
        RepositoryList {
            repositories: self
                .repositories()
                .every()
                .iter()
                .map(|one| summary_of(one))
                .collect(),
        }
    }

    /// `add_repository`: read the folder, serve it, and reconcile the Jobs the
    /// store already holds for it — a repository served again after a restart
    /// is cleaned up the way the one Fleet started in is.
    pub(crate) async fn added_repository(
        &self,
        asked: AddRepository,
    ) -> Result<RepositorySummary, Refusal> {
        let folder = PathBuf::from(&asked.path);
        // Before the read, so a second add of one folder costs nothing. `add`
        // below asks again under its lock.
        if let Some(served) = folder
            .canonicalize()
            .ok()
            .and_then(|root| self.repositories().at(&root.to_string_lossy()))
        {
            return Err(self.not_added(NotAdded::AlreadyServed {
                root: served.root().to_string(),
            }));
        }
        if !folder.is_absolute() {
            return Err(self.not_located(NotLocated::NotARepository {
                folder: asked.path,
                why: "a relative path names a folder under Fleet's own working directory".into(),
            }));
        }
        let locating = Arc::clone(self.locating());
        let located = tokio::task::spawn_blocking(move || locating.located(&folder))
            .await
            .expect("reading a folder panicked")
            .map_err(|why| self.not_located(why))?;
        let added = self
            .repositories()
            .add(located)
            .map_err(|why| self.not_added(why))?;
        if let Some(served) = Served::of(&added) {
            self.locating().serving(served.root());
            self.reconciled_in(&served)
                .await
                .map_err(|why| self.refusal(why))?;
        }
        Ok(summary_of(&added))
    }

    /// Give a repository served without a Manifest the one Write just put at
    /// its root. **Quiet where nothing loads**: Write has already answered, and
    /// what the file came to is the Manifest surface's to read.
    pub(crate) fn set_up_after_write(&self, repository: &Repository) {
        if repository.manifest().is_some() {
            return;
        }
        let Ok(located) = self
            .locating()
            .located(std::path::Path::new(repository.root()))
        else {
            return;
        };
        if let Some(set_up) = located.set_up {
            if let Ok(served) = self.repositories().set_up(repository.root(), set_up) {
                self.locating().serving(served.root());
            }
        }
    }

    fn not_located(&self, why: NotLocated) -> Refusal {
        let code = match why {
            NotLocated::NotARepository { .. } => NOT_A_REPOSITORY,
            NotLocated::Refused { .. } => REPOSITORY_REFUSED,
        };
        Refusal::Unacceptable(WireError::raised(code, why.to_string(), self.run_id()))
    }

    fn not_added(&self, why: NotAdded) -> Refusal {
        Refusal::IllegalMove(WireError::raised(
            ALREADY_SERVED,
            why.to_string(),
            self.run_id(),
        ))
    }
}

/// One repository as the wire carries it.
fn summary_of(repository: &Repository) -> RepositorySummary {
    RepositorySummary {
        root: repository.root().to_string(),
        records_root: repository.records_root().to_string(),
        manifest: repository
            .manifest()
            .map(|held| manifest_summary(held, repository.records_root())),
    }
}
