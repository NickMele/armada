//! Which `armada.yml` a Verify runs: the root's, which Fleet already holds, or
//! one workspace's below it, read from disk when Verify is pressed.
//!
//! **The checkout is still the owner.** A workspace's file sits in the same
//! tree and build directory as the root's, so a workspace Verify holds the one
//! slot keyed by the repository root — only the file and the directory differ.
//!
//! **Its ports are its own span, claimed for the Verify and given back when it
//! ends**, rather than the root's grown to cover them: regrowing a contiguous
//! span re-picks it, which would move a port a root server is already bound to.

use std::collections::{BTreeMap, BTreeSet};
use std::path::{Component, Path, PathBuf};

use adapter_traits::{AgentHarness, Delivery, Vcs, WorkProduct};
use config::Manifest;
use ipc::{ManifestFault, ManifestRefused};
use store::PortClaimant;

use super::owner::Checkout;
use super::Unrehearsable;
use crate::daemon::Fleet;
use crate::ports::{env_names, env_vars, port_map};

/// A workspace's own Manifest, and the directory its commands run in.
pub(crate) struct Workspace {
    /// Relative to the repository root, `/`-separated, never empty or `.`.
    pub(crate) dir: String,
    pub(crate) manifest: Manifest,
}

/// `asked` resolved against `root`. `None` is the root's Manifest, which
/// absent, empty and `.` all name.
pub(crate) fn resolved(
    root: &str,
    asked: Option<&str>,
) -> Result<Option<Workspace>, Unrehearsable> {
    let Some(asked) = asked else {
        return Ok(None);
    };
    let outside = || Unrehearsable::WorkspaceOutside {
        dir: asked.to_string(),
    };
    let mut parts = Vec::new();
    for part in Path::new(asked).components() {
        match part {
            Component::Normal(name) => parts.push(name.to_str().ok_or_else(outside)?),
            Component::CurDir => {}
            Component::ParentDir | Component::RootDir | Component::Prefix(_) => {
                return Err(outside())
            }
        }
    }
    if parts.is_empty() {
        return Ok(None);
    }
    let dir = parts.join("/");
    let at = PathBuf::from(root).join(&dir);
    // A symlink below the root can still lead out of the checkout.
    if let (Ok(real), Ok(real_root)) = (at.canonicalize(), Path::new(root).canonicalize()) {
        if !real.starts_with(&real_root) {
            return Err(outside());
        }
    }
    let manifest =
        Manifest::load(&at.join("armada.yml")).map_err(|why| Unrehearsable::WorkspaceManifest {
            file: format!("{dir}/armada.yml"),
            refused: ManifestRefused {
                summary: why.to_string(),
                faults: why
                    .refusals()
                    .iter()
                    .map(|one| ManifestFault {
                        key: one.key.clone(),
                        fault: one.fault.to_string(),
                    })
                    .collect(),
            },
        })?;
    Ok(Some(Workspace { dir, manifest }))
}

/// Each workspace below `root` whose own `armada.yml` loads, with its Commands.
/// A file that will not load lists nothing; Verify is where it says why.
pub(crate) fn listed(root: &str) -> Vec<ipc::WorkspaceCommands> {
    crate::scanning::manifested(&crate::scanning::Checkout::at(root))
        .into_iter()
        .filter_map(|dir| {
            let one = resolved(root, Some(&dir)).ok().flatten()?;
            let (_, _, commands) = super::entries::declared(&one.manifest).sheet(&[]);
            Some(ipc::WorkspaceCommands {
                dir: one.dir,
                commands,
            })
        })
        .collect()
}

/// A workspace a Verify runs in: its directory, and the ports its own file
/// declares, resolved from the span claimed for that Verify.
#[derive(Clone)]
pub(crate) struct Within {
    /// Relative to the repository root.
    pub(crate) dir: PathBuf,
    claimant: PortClaimant,
    declared: BTreeSet<String>,
    ports: BTreeMap<String, u16>,
    env: Vec<(String, String)>,
    /// The root's variables for a name this file declares too.
    shadowed: BTreeSet<String>,
}

impl Within {
    /// The root's ports and variables with this file's laid over them. **A name
    /// both declare is the workspace's**, and stays unresolved rather than
    /// falling back to the root's number where the workspace's claim failed.
    pub(crate) fn overlaid(
        &self,
        ports: &mut BTreeMap<String, u16>,
        env: &mut Vec<(String, String)>,
    ) {
        ports.retain(|name, _| !self.declared.contains(name));
        ports.extend(self.ports.iter().map(|(name, port)| (name.clone(), *port)));
        env.retain(|(key, _)| {
            !self.shadowed.contains(key) && !self.env.iter().any(|(own, _)| own == key)
        });
        env.extend(self.env.iter().cloned());
    }
}

/// The key a workspace's span is kept under, beside the main checkouts' so boot
/// reconciliation and shutdown find a row a crash left. **Not a path**: a served
/// root is canonical and absolute, so no repository's key can equal it.
pub(crate) fn claimant(root: &str, dir: &str) -> PortClaimant {
    let at = Path::new(root).join(dir);
    PortClaimant::MainCheckout(format!("{WORKSPACE_KEY}{}", at.to_string_lossy()))
}

const WORKSPACE_KEY: &str = "workspace:";

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
    /// Claim the ports `workspace`'s own file declares, for one Verify. A row a
    /// crashed Verify left under the same key is given back first, so the span
    /// is sized from the file as it reads now.
    pub(crate) async fn workspace_claimed(
        &self,
        checkout: &Checkout,
        workspace: Workspace,
    ) -> Within {
        let Workspace { dir, manifest } = workspace;
        let claimant = claimant(checkout.root(), &dir);
        let PortClaimant::MainCheckout(key) = &claimant else {
            unreachable!("built as a main checkout's just above");
        };
        let _ = self.store().lock().await.release_port_span(&claimant);
        // Nothing to escalate on a refusal: an unresolved `${port.NAME}` reads
        // off the run's own log, as the main checkout's does.
        let _ = self.try_claim(claimant.clone(), &manifest).await;
        let claim = self.store().lock().await.port_span_for_main_checkout(key);
        let ports = match claim.ok().flatten() {
            Some(claim) => port_map(&manifest, &claim),
            None => BTreeMap::new(),
        };
        let names = env_names(&manifest).unwrap_or_default();
        let declared: BTreeSet<String> = manifest.port_names().into_iter().collect();
        let shadowed = checkout
            .served()
            .and_then(|served| env_names(served.manifest()).ok())
            .unwrap_or_default()
            .into_iter()
            .filter(|(name, _)| declared.contains(name))
            .flat_map(|(_, keys)| keys)
            .collect();
        Within {
            dir: PathBuf::from(dir),
            claimant,
            declared,
            env: env_vars(&names, &ports),
            ports,
            shadowed,
        }
    }

    /// Give a workspace's span back once its Verify has ended.
    pub(crate) async fn released_workspace_ports(&self, within: &Within) {
        let _ = self
            .store()
            .lock()
            .await
            .release_port_span(&within.claimant);
    }
}
