//! The seed, as the two run sheets read it. #1064.

use std::path::Path;

use adapter_traits::{AgentHarness, BaseSpec, Delivery, Vcs, WorkProduct};
use config::Manifest;
use ipc::{DeclaredSeed, SeedWarmth, WorktreeSeeding};

use super::{recorded, Cold, Recorded};
use crate::basing::NoBase;
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
    /// `RunSheet::seeding`: what this Job's worktree was seeded with, as it was
    /// written down. Absent where the Job's own Manifest declares no seed.
    pub(crate) fn worktree_seeding(
        &self,
        records_root: &str,
        handle: &str,
        manifest: &Manifest,
    ) -> Option<WorktreeSeeding> {
        manifest.seed()?;
        Some(match recorded(records_root, handle) {
            Some(Recorded::Seeded { commit, paths }) => WorktreeSeeding::Seeded { commit, paths },
            Some(Recorded::Cold { why }) => WorktreeSeeding::Cold { why },
            None => WorktreeSeeding::Unrecorded,
        })
    }

    /// `CheckoutRunSheet::seed`, read against the base commit as it is now.
    pub(crate) fn declared_seed(
        &self,
        served: &crate::repositories::Served,
    ) -> Option<DeclaredSeed> {
        let manifest = served.manifest();
        let seed = manifest.seed()?;
        Some(DeclaredSeed {
            paths: seed.paths().to_vec(),
            warmed_by: seed
                .warmed_by()
                .iter()
                .map(|one| one.name().to_string())
                .collect(),
            warmth: self.warmth(served.root(), manifest),
        })
    }

    fn warmth(&self, root: &str, manifest: &Manifest) -> SeedWarmth {
        let cold = |why: NoBase| SeedWarmth::Cold {
            why: Cold::NoBase(why).said(),
        };
        let at = match self.vcs().base_commit(root, manifest.base()) {
            Ok(Some(at)) => at,
            Ok(None) => return cold(NoBase::Unnamed),
            Err(cause) => {
                return cold(NoBase::NotCheckedOut {
                    why: cause.to_string(),
                })
            }
        };
        let spec = match BaseSpec::at(root, &at) {
            Ok(spec) => spec,
            Err(why) => return cold(NoBase::NotCheckedOut { why: why.said() }),
        };
        let commit = spec.commit().to_string();
        let seeds = self.seeds().lock().expect("the seeds lock is not poisoned");
        if seeds.warming_in(root) == Some(commit.as_str()) {
            return SeedWarmth::Warming { commit };
        }
        if Path::new(&spec.seed_marker()).exists() {
            return SeedWarmth::Warm { commit };
        }
        let failed = seeds.failed_at(root, &commit).map(str::to_string);
        SeedWarmth::Cold {
            why: Cold::NotWarm { commit, failed }.said(),
        }
    }
}
