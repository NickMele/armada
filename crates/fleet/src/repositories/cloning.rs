//! `clone_repository`: git clones a URL into a new folder under one a person
//! picked, and Fleet serves the result the way `add_repository` does.
//!
//! **Answers when git finishes, within [`CLONE_BUDGET`].** A clone is one act a
//! person waits on; a stated bound is honest where progress nothing reads is not.

use std::collections::BTreeSet;
use std::path::{Path, PathBuf};
use std::sync::{Arc, Mutex, PoisonError};
use std::time::Duration;

use adapter_traits::{AgentHarness, Delivery, NotCloned, Vcs, WorkProduct};
use api::Refusal;
use ipc::{AddRepository, CloneRepository, RepositorySummary, WireError};

use crate::daemon::Fleet;

/// How long git may take before the clone is stopped. Long enough for a large
/// repository over a slow link; a hang still ends.
pub const CLONE_BUDGET: Duration = Duration::from_secs(10 * 60);

/// A `parent` that is not an absolute folder on this machine. A 422.
const NO_SUCH_FOLDER: &str = "fleet.no_such_folder";
/// A destination with something in it, or a clone already writing there. A 409.
const DESTINATION_OCCUPIED: &str = "fleet.destination_occupied";
/// A URL git refused, or one naming no folder. A 422.
const CLONE_REFUSED: &str = "fleet.clone_refused";
/// Git would not start, or was stopped at the bound. A 500.
const CLONE_NOT_FINISHED: &str = "fleet.clone_not_finished";

/// The folder `git clone` makes for `url`: the last segment, less `.git`.
pub fn folder_named_by(url: &str) -> Option<String> {
    let mut end = url.trim().trim_end_matches('/');
    if let Some(before) = end.strip_suffix("/.git") {
        end = before.trim_end_matches('/');
    }
    let start = end.rfind(['/', ':']).map_or(0, |at| at + 1);
    let last = &end[start..];
    let name = last.strip_suffix(".git").unwrap_or(last);
    match name {
        "" | "." | ".." => None,
        _ => Some(name.to_string()),
    }
}

/// Destinations a clone is writing into, so a second press is refused rather
/// than racing git into one folder.
#[derive(Default)]
pub struct Cloning(Mutex<BTreeSet<PathBuf>>);

/// One destination held until the clone and the add are both done.
pub struct Claimed<'a> {
    held: &'a Cloning,
    at: PathBuf,
}

impl Cloning {
    pub fn claim(&self, at: &Path) -> Option<Claimed<'_>> {
        let mut held = self.0.lock().unwrap_or_else(PoisonError::into_inner);
        held.insert(at.to_path_buf()).then(|| Claimed {
            held: self,
            at: at.to_path_buf(),
        })
    }
}

impl Drop for Claimed<'_> {
    fn drop(&mut self) {
        let mut held = self.held.0.lock().unwrap_or_else(PoisonError::into_inner);
        held.remove(&self.at);
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
    /// Spawned, so a caller that stops waiting does not strand a clone that
    /// landed and was never served.
    pub(crate) async fn cloned_repository(
        self: Arc<Self>,
        asked: CloneRepository,
    ) -> Result<RepositorySummary, Refusal> {
        tokio::spawn(async move { self.cloning(asked).await })
            .await
            .unwrap_or_else(|panicked| std::panic::resume_unwind(panicked.into_panic()))
    }

    async fn cloning(&self, asked: CloneRepository) -> Result<RepositorySummary, Refusal> {
        let Some(name) = folder_named_by(&asked.url) else {
            let why = format!("`{}` names no folder to clone into", asked.url);
            return Err(self.unacceptable(CLONE_REFUSED, why));
        };
        let parent = Path::new(&asked.parent);
        let parent = match parent.is_absolute().then(|| parent.canonicalize()) {
            Some(Ok(found)) if found.is_dir() => found,
            Some(_) => {
                let why = format!("{} is not a folder on this machine", asked.parent);
                return Err(self.unacceptable(NO_SUCH_FOLDER, why));
            }
            None => {
                let why = format!(
                    "{} is relative, and a clone needs an absolute folder",
                    asked.parent
                );
                return Err(self.unacceptable(NO_SUCH_FOLDER, why));
            }
        };
        let destination = parent.join(&name);
        let at = destination.to_string_lossy().to_string();
        if occupied(&destination) {
            let why = format!("{at} already exists and is not empty");
            return Err(self.occupied(why));
        }
        let Some(_claimed) = self.repositories().cloning().claim(&destination) else {
            return Err(self.occupied(format!("a clone into {at} is already under way")));
        };
        let (vcs, url, into) = (Arc::clone(self.vcs()), asked.url, at.clone());
        tokio::task::spawn_blocking(move || vcs.clone_repository(&url, &into, CLONE_BUDGET))
            .await
            .expect("cloning a repository panicked")
            .map_err(|why| self.not_cloned(why))?;
        self.added_repository(AddRepository { path: at }).await
    }

    fn unacceptable(&self, code: &str, why: String) -> Refusal {
        Refusal::Unacceptable(WireError::raised(code, why, self.run_id()))
    }

    fn occupied(&self, why: String) -> Refusal {
        Refusal::IllegalMove(WireError::raised(DESTINATION_OCCUPIED, why, self.run_id()))
    }

    fn not_cloned(&self, why: NotCloned) -> Refusal {
        let said = why.said();
        match why {
            NotCloned::Refused { .. } => self.unacceptable(CLONE_REFUSED, said),
            NotCloned::TookTooLong { .. } | NotCloned::NotRun { .. } => {
                Refusal::Fault(WireError::raised(CLONE_NOT_FINISHED, said, self.run_id()))
            }
        }
    }
}

/// Anything there but an empty folder, including a folder that will not read.
fn occupied(destination: &Path) -> bool {
    match std::fs::read_dir(destination) {
        Ok(mut entries) => entries.next().is_some(),
        Err(why) => why.kind() != std::io::ErrorKind::NotFound,
    }
}
