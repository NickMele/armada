//! The ambient world, read once at the entrypoint and handed to every request.
//!
//! **This is what makes the server stateless rather than merely long-lived.**
//! The base protocol is *"JSON-RPC message format, stateless, self-contained
//! requests, per-request capability negotiation"* as of spec revision
//! `2026-07-28` ([`docs/traps.md`](../../../../docs/traps.md)), so there is no
//! session to keep. What the process does keep is exactly what `main` already
//! read before any verb existed — the current directory, `$HOME`, the
//! environment, this executable and this boot — and a request rebuilds its
//! `Where` or its `App` from those every time.
//!
//! Nothing here reaches for `std::env` (`ARCHITECTURE.md` §1.4). The whole
//! struct is filled at the entrypoint and passed down, which is the same rule
//! every other verb runs under and the reason a test can point the server at a
//! `TempDir` instead of at somebody's real `~/.armada/`.

use std::collections::BTreeMap;
use std::path::PathBuf;

/// Everything the server may know about the machine it is running on.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct World {
    /// Where the server was started. Manifest's tools resolve a workspace from
    /// it when a call names no path.
    pub cwd: PathBuf,
    /// `$HOME`.
    pub home: PathBuf,
    /// The environment Armada was started with.
    pub inherited: BTreeMap<String, String>,
    /// This executable, for the Manifest verbs Fleet runs inside a worktree.
    pub exe: PathBuf,
    /// This boot, so a Drone handle from a previous one is stale by definition.
    pub boot_id: String,
}

impl World {
    /// `~/.armada/`.
    pub fn armada_home(&self) -> PathBuf {
        armada_manifest::machine::armada_home(&self.home)
    }

    /// The Fleet verbs' `Where`, rebuilt for this request.
    pub fn place(&self) -> crate::verbs::fleet::Where {
        crate::verbs::fleet::Where {
            home: self.home.clone(),
            armada_home: self.armada_home(),
            cwd: self.cwd.clone(),
            exe: self.exe.clone(),
            boot_id: self.boot_id.clone(),
        }
    }

    /// The Job this process belongs to, when it belongs to one.
    ///
    /// **`ARMADA_JOB_UUID` first, `ARMADA_JOB` only as the fallback.** Both are
    /// set by [`armada_fleet::drone::job_env`] and by nothing else, so either
    /// one's presence is the machine's own answer to *"is the caller a Drone"* —
    /// the same pair a `Stop` hook reads to say which Job stopped (PLAN.md
    /// §15.3). Read out of the environment map the entrypoint captured rather
    /// than from a second `std::env` call.
    ///
    /// # Why the uuid, and it is not a preference
    ///
    /// A name is a **handle**: a Job releases it when it ends and the next Job
    /// may take it, while the ended Job keeps its record because that record is
    /// how its transcript is found. Both are deliberate, and together they mean a
    /// name can match two records — which
    /// [`armada_fleet::jobs::Store::find`] now resolves to the live one.
    ///
    /// **A Drone must not depend on that resolution existing.** Measured
    /// 2026-08-17 on job `86d8cf8e`: `armada failures fix` respawned a killed
    /// Job's failure, the derived name collided with the `ABORTED` record, and
    /// every tool on the Drone's belt was refused *"`armada-failed` names 2
    /// Jobs"* — `fleet_report`, `fleet_check`, and `fleet_ask_human`, which is
    /// the one it would have used to say so. It worked 42 turns and $2.69, wrote
    /// a correct reproduction, and recorded nothing anywhere.
    ///
    /// A uuid is unambiguous by construction, so this closes the class rather
    /// than the instance: no resolution rule has to be right for a Drone to
    /// reach its own Job. The name stays as the fallback for a Drone started by
    /// an older Armada, whose environment carries only that.
    pub fn job(&self) -> Option<&str> {
        ["ARMADA_JOB_UUID", "ARMADA_JOB"]
            .into_iter()
            .filter_map(|key| self.inherited.get(key))
            .map(String::as_str)
            .find(|handle| !handle.is_empty())
    }
}
