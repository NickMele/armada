//! Reaping (PLAN.md §2.3.1) — the decisions, not the removals.
//!
//! **The plan's one piece of empirical evidence is a sweep function that
//! existed and was never called.** So reaping is automatic, at `char init`
//! *and* `char clean`, in three passes:
//!
//! 1. **Registry** — drop `workspaces` rows whose path no longer exists.
//! 2. **Resource** — find everything labelled `char.workspace=*`, read its
//!    `char.workspace_path` label, and **`stat` that path. Remove only on
//!    `ENOENT`.**
//! 3. **Lease** — delete leases whose heartbeat has gone cold.
//!
//! The rule that carries all the weight is in pass 2 and it is a negative one:
//! **any errno other than `ENOENT` means "adopt or report", never remove.**
//! Measured, `stat` on a *live* directory under a mode-000 parent returns
//! `EACCES`, which is byte-identical in failure to a missing path — and that is
//! exactly the multi-user and devcontainer case the path label was added for,
//! since `$HOME` is 0700 on Linux. A naive "stat failed → gone" reintroduces
//! the bug the label exists to prevent, which is `clean` in one workspace
//! destroying another's live containers.
//!
//! Pass 1 obeys the same rule for the same reason: a live workspace on a
//! network mount that is momentarily unavailable answers `EACCES` or `EIO`, and
//! dropping its row releases a port block it is still using.

use crate::id::WorkspaceId;
use crate::registry::{LeaseRow, OwnedKind, WorkspaceRow};
use serde::Serialize;
use std::path::PathBuf;

/// The three answers a `stat` can give, and the middle one is the whole point.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum PathStat {
    /// The directory is there.
    Present,
    /// `ENOENT`, and nothing else. The **only** answer that permits removal.
    Missing,
    /// Any other errno — `EACCES`, `EIO`, a hung mount. Indistinguishable from
    /// `Missing` at the syscall's return value, and the opposite thing.
    Unreadable(String),
}

/// A resource carrying char's labels, as `docker ps`/`network ls`/`volume ls`/
/// `image ls` reported it.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct LabelledResource {
    /// What kind of thing.
    pub kind: OwnedKind,
    /// The id or name docker knows it by.
    pub reference: String,
    /// The `char.workspace` label.
    pub workspace: WorkspaceId,
    /// The `char.workspace_path` label. Stamping the path is what makes this
    /// pass **self-sufficient**: it stats a real directory and consults no
    /// database at all.
    pub workspace_path: PathBuf,
    /// The `char.namespace` label, absent on a resource stamped by a char that
    /// predates it.
    pub namespace: Option<String>,
}

/// Why a resource was left alone.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum LeftAlone {
    /// Its workspace is alive.
    WorkspaceLive,
    /// Its path could not be read, and that is not the same as gone.
    PathUnreadable,
    /// It belongs to a different `char.namespace` — a different filesystem
    /// view of this daemon, which is the ordinary devcontainer setup.
    ForeignNamespace,
    /// It carries no namespace label at all, so char cannot tell whose it is.
    UnstampedNamespace,
}

/// One thing char decided not to remove, and why. **Reported, never silent.**
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct Reported {
    /// What kind of thing.
    pub kind: OwnedKind,
    /// Its handle.
    pub reference: String,
    /// The workspace that stamped it.
    pub workspace: WorkspaceId,
    /// Why it survived.
    pub reason: LeftAlone,
}

/// What a reap pass decided.
///
/// **Reaping is reported, never silent** — in human output and under
/// `data.reaped` in `--json`. A tool that removes containers without saying so
/// is worse than one that does not remove them.
#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize)]
pub struct ReapPlan {
    /// Workspace rows to drop, releasing their port blocks and `owned` rows.
    #[serde(skip_serializing_if = "Vec::is_empty")]
    pub workspaces: Vec<WorkspaceId>,
    /// Resources to remove.
    #[serde(skip_serializing_if = "Vec::is_empty")]
    pub resources: Vec<ReapTarget>,
    /// Lease rows to delete.
    #[serde(skip_serializing_if = "Vec::is_empty")]
    pub leases: Vec<String>,
    /// Everything deliberately left alone, with its reason.
    #[serde(skip_serializing_if = "Vec::is_empty")]
    pub reported: Vec<Reported>,
}

impl ReapPlan {
    /// Whether the pass found nothing at all to do.
    pub fn is_empty(&self) -> bool {
        self.workspaces.is_empty()
            && self.resources.is_empty()
            && self.leases.is_empty()
            && self.reported.is_empty()
    }
}

/// One resource to remove.
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct ReapTarget {
    /// What kind of thing.
    pub kind: OwnedKind,
    /// Its handle.
    pub reference: String,
    /// The workspace that stamped it — reported so the removal is attributable.
    pub workspace: WorkspaceId,
}

/// Pass 1: which `workspaces` rows are dead.
///
/// Takes the stat result rather than performing it, so the rule that decides
/// everything — `Missing` and nothing else — is testable without a filesystem.
pub fn registry_pass(rows: &[(WorkspaceRow, PathStat)]) -> Vec<WorkspaceId> {
    rows.iter()
        .filter(|(_, stat)| *stat == PathStat::Missing)
        .map(|(row, _)| row.id.clone())
        .collect()
}

/// Pass 2: which labelled resources are orphans, and which are merely not ours.
///
/// `namespace` is this installation's, read from `char.db` at creation. **A
/// third label, `char.namespace`, scopes the whole mechanism to one filesystem
/// view** — without it, path-based reaping is actively dangerous the moment two
/// char installations share a Docker daemon, which is the ordinary
/// devcontainer setup: a workspace at `/workspaces/repo` inside the container
/// is `ENOENT` when a host-side `char init` stats it, so the host reaps a live
/// workspace's containers.
pub fn resource_pass(
    resources: &[(LabelledResource, PathStat)],
    namespace: &str,
) -> (Vec<ReapTarget>, Vec<Reported>) {
    let mut remove = Vec::new();
    let mut reported = Vec::new();

    for (resource, stat) in resources {
        let reason = match resource.namespace.as_deref() {
            None => Some(LeftAlone::UnstampedNamespace),
            Some(theirs) if theirs != namespace => Some(LeftAlone::ForeignNamespace),
            Some(_) => match stat {
                PathStat::Missing => None,
                PathStat::Present => Some(LeftAlone::WorkspaceLive),
                PathStat::Unreadable(_) => Some(LeftAlone::PathUnreadable),
            },
        };

        match reason {
            None => remove.push(ReapTarget {
                kind: resource.kind,
                reference: resource.reference.clone(),
                workspace: resource.workspace.clone(),
            }),
            Some(reason) => reported.push(Reported {
                kind: resource.kind,
                reference: resource.reference.clone(),
                workspace: resource.workspace.clone(),
                reason,
            }),
        }
    }

    (remove, reported)
}

/// Pass 3: which leases have gone cold.
pub fn lease_pass(rows: &[LeaseRow], now_mono_ms: u64, boot_id: &str) -> Vec<String> {
    rows.iter()
        .filter(|row| {
            crate::lease::is_cold(row.heartbeat_mono, now_mono_ms, row.boot_id == boot_id)
        })
        .map(|row| format!("{}:{}", row.kind, row.key))
        .collect()
}

/// Whether a recorded process group is still ours to kill.
///
/// The naive version cannot tell "pgid 4212 is my orphaned service" from "pgid
/// 4212 was recycled by the OS", so it can only ever report. Recording the boot
/// id and the process start time makes recycling detectable: same boot, same
/// start time, it is ours. Anything else is stale and the row is dropped —
/// which turns "report forever" into "reap safely".
pub fn pgid_is_ours(
    recorded_boot: Option<&str>,
    recorded_start: Option<&str>,
    current_boot: &str,
    observed_start: Option<&str>,
) -> bool {
    match (recorded_boot, recorded_start, observed_start) {
        (Some(boot), Some(started), Some(observed)) => boot == current_boot && started == observed,
        // No observed start time means the process is gone; nothing to kill,
        // and the row is safe to drop either way.
        (Some(_), Some(_), None) => false,
        // A row written before these columns existed, or a process char could
        // not sample. Not provably ours, so never killed.
        (None, _, _) | (_, None, _) => false,
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::ports::PortBlock;

    fn row(id: &str, path: &str) -> WorkspaceRow {
        WorkspaceRow {
            id: WorkspaceId::from_stored(id),
            path: PathBuf::from(path),
            project: None,
            ports: PortBlock {
                from: 5460,
                to: 5469,
            },
            claimed_at: "2026-08-09T14:02:11Z".to_string(),
        }
    }

    fn resource(ns: Option<&str>, path: &str) -> LabelledResource {
        LabelledResource {
            kind: OwnedKind::Container,
            reference: "c0ffee".to_string(),
            workspace: WorkspaceId::from_stored("a3f91c02"),
            workspace_path: PathBuf::from(path),
            namespace: ns.map(str::to_string),
        }
    }

    #[test]
    fn a_deleted_workspace_loses_its_row() {
        let rows = [(row("a3f91c02", "/gone"), PathStat::Missing)];
        assert_eq!(
            registry_pass(&rows),
            vec![WorkspaceId::from_stored("a3f91c02")]
        );
    }

    #[test]
    fn a_live_workspace_keeps_its_row() {
        let rows = [(row("a3f91c02", "/live"), PathStat::Present)];
        assert!(registry_pass(&rows).is_empty());
    }

    /// A live workspace on a momentarily unavailable network mount answers
    /// `EACCES` or `EIO`. Dropping its row would release a port block it is
    /// still using.
    #[test]
    fn an_unreadable_path_keeps_its_row_in_pass_one_too() {
        let rows = [(
            row("a3f91c02", "/mnt/slow"),
            PathStat::Unreadable("EACCES".to_string()),
        )];
        assert!(registry_pass(&rows).is_empty());
    }

    #[test]
    fn an_orphaned_resource_in_our_namespace_is_removed() {
        let input = [(resource(Some("ns-1"), "/gone"), PathStat::Missing)];
        let (remove, reported) = resource_pass(&input, "ns-1");
        assert_eq!(remove.len(), 1);
        assert!(reported.is_empty());
    }

    #[test]
    fn a_live_workspaces_resource_is_reported_not_removed() {
        let input = [(resource(Some("ns-1"), "/live"), PathStat::Present)];
        let (remove, reported) = resource_pass(&input, "ns-1");
        assert!(remove.is_empty());
        assert_eq!(reported[0].reason, LeftAlone::WorkspaceLive);
    }

    /// The measured case the whole rule exists for: `stat` under a mode-000
    /// parent is `EACCES`, and treating that as gone destroys a running
    /// workspace's containers.
    #[test]
    fn eacces_is_never_treated_as_gone() {
        let input = [(
            resource(Some("ns-1"), "/home/other/repo"),
            PathStat::Unreadable("EACCES".to_string()),
        )];
        let (remove, reported) = resource_pass(&input, "ns-1");
        assert!(remove.is_empty());
        assert_eq!(reported[0].reason, LeftAlone::PathUnreadable);
    }

    /// Two char installations sharing one Docker daemon is the ordinary
    /// devcontainer setup. `/workspaces/repo` is `ENOENT` from the host, so
    /// without the namespace label the host reaps a live workspace.
    #[test]
    fn a_foreign_namespace_is_never_reaped_even_when_its_path_is_missing_here() {
        let input = [(
            resource(Some("their-ns"), "/workspaces/repo"),
            PathStat::Missing,
        )];
        let (remove, reported) = resource_pass(&input, "ns-1");
        assert!(remove.is_empty());
        assert_eq!(reported[0].reason, LeftAlone::ForeignNamespace);
    }

    #[test]
    fn an_unstamped_resource_is_reported_never_removed() {
        let input = [(resource(None, "/gone"), PathStat::Missing)];
        let (remove, reported) = resource_pass(&input, "ns-1");
        assert!(remove.is_empty());
        assert_eq!(reported[0].reason, LeftAlone::UnstampedNamespace);
    }

    #[test]
    fn a_cold_lease_is_collected_and_a_warm_one_is_not() {
        let warm = LeaseRow {
            workspace: Some(WorkspaceId::from_stored("a3f91c02")),
            kind: crate::lease::LeaseKind::Run,
            key: "run".to_string(),
            heartbeat_mono: 100_000,
            boot_id: "boot-1".to_string(),
            pid: 42,
            pid_started_at: None,
        };
        let cold = LeaseRow {
            heartbeat_mono: 0,
            key: "browser".to_string(),
            kind: crate::lease::LeaseKind::Exclusive,
            ..warm.clone()
        };
        assert_eq!(
            lease_pass(&[warm, cold], 100_000, "boot-1"),
            vec!["exclusive:browser".to_string()]
        );
    }

    #[test]
    fn a_pgid_from_another_boot_is_never_killed() {
        assert!(!pgid_is_ours(
            Some("boot-0"),
            Some("Sat Aug  9 10:00:00 2026"),
            "boot-1",
            Some("Sat Aug  9 10:00:00 2026")
        ));
    }

    #[test]
    fn a_recycled_pid_is_never_killed() {
        assert!(!pgid_is_ours(
            Some("boot-1"),
            Some("Sat Aug  9 10:00:00 2026"),
            "boot-1",
            Some("Sun Aug 10 12:00:00 2026")
        ));
    }

    #[test]
    fn our_own_live_process_group_is_ours() {
        assert!(pgid_is_ours(
            Some("boot-1"),
            Some("Sat Aug  9 10:00:00 2026"),
            "boot-1",
            Some("Sat Aug  9 10:00:00 2026")
        ));
    }

    #[test]
    fn a_row_written_without_the_liveness_columns_is_never_killed() {
        assert!(!pgid_is_ours(None, None, "boot-1", Some("whenever")));
    }
}
