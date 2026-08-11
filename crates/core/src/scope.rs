//! The scope lens (PLAN.md §3.3) — the same flag on `status` and `clean`.
//!
//! | Scope | Covers | Answers |
//! |---|---|---|
//! | *(no flag)* | this checkout | "are my services up? what ports do I hold?" |
//! | `--project` | every workspace sharing this `--git-common-dir` | the orchestrating agent's view |
//! | `--all` | every workspace on the machine | "what is char holding anywhere?" |
//!
//! **Workspaces in a project are siblings, not parent and children.** The root
//! checkout is just another workspace with no authority over the worktrees.
//! That is load-bearing: model it as a hierarchy and `char clean` in the root
//! implies cascading into the worktrees, killing services another agent is
//! actively using. Flat siblings plus an explicit flag makes the destructive
//! step something you have to ask for.

use crate::id::{ProjectId, WorkspaceId};
use crate::registry::WorkspaceRow;

/// How wide a read or a reclaim reaches.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum Lens {
    /// This checkout, and nothing else.
    #[default]
    Workspace,
    /// Every workspace sharing this `--git-common-dir`.
    Project,
    /// Every workspace on the machine.
    All,
}

impl Lens {
    /// Parse the flag pair. Both together is a `bad_invocation` the caller
    /// reports; this returns `None` so it can.
    pub fn from_flags(project: bool, all: bool) -> Option<Self> {
        match (project, all) {
            (false, false) => Some(Lens::Workspace),
            (true, false) => Some(Lens::Project),
            (false, true) => Some(Lens::All),
            (true, true) => None,
        }
    }
}

/// Which rows this lens selects.
///
/// `project` is the invoking workspace's project id, or `None` when it was
/// underivable — in which case `--project` selects only this workspace, which
/// is the honest answer: the grouping key does not exist, so nothing can be
/// grouped with it. It does not error, because `project_id` owns nothing and
/// making it fatal would take a worktree whose resources are perfectly
/// reclaimable and refuse to reclaim them (PLAN.md §2.2).
pub fn select<'a>(
    rows: &'a [WorkspaceRow],
    lens: Lens,
    me: &WorkspaceId,
    project: Option<&ProjectId>,
) -> Vec<&'a WorkspaceRow> {
    rows.iter()
        .filter(|row| match lens {
            Lens::Workspace => &row.id == me,
            Lens::Project => match project {
                Some(project) => row.project.as_ref() == Some(project),
                None => &row.id == me,
            },
            Lens::All => true,
        })
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::ports::PortBlock;
    use std::path::PathBuf;

    fn row(id: &str, project: Option<&str>) -> WorkspaceRow {
        WorkspaceRow {
            id: WorkspaceId::from_stored(id),
            path: PathBuf::from(format!("/srv/{id}")),
            project: project.map(ProjectId::from_stored),
            ports: PortBlock {
                from: 5460,
                to: 5469,
            },
            claimed_at: "2026-08-09T14:02:11Z".to_string(),
        }
    }

    fn rows() -> Vec<WorkspaceRow> {
        vec![
            row("aaaaaaaa", Some("p1")),
            row("bbbbbbbb", Some("p1")),
            row("cccccccc", Some("p2")),
            row("dddddddd", None),
        ]
    }

    #[test]
    fn no_flag_is_just_me() {
        let rows = rows();
        let me = WorkspaceId::from_stored("aaaaaaaa");
        let selected = select(
            &rows,
            Lens::Workspace,
            &me,
            Some(&ProjectId::from_stored("p1")),
        );
        assert_eq!(selected.len(), 1);
        assert_eq!(selected[0].id, me);
    }

    /// The five-worktrees case: `--project` from any one of them reports all
    /// of them, because they share one `--git-common-dir`.
    #[test]
    fn project_selects_every_sibling_including_me() {
        let rows = rows();
        let me = WorkspaceId::from_stored("aaaaaaaa");
        let selected = select(
            &rows,
            Lens::Project,
            &me,
            Some(&ProjectId::from_stored("p1")),
        );
        assert_eq!(selected.len(), 2);
    }

    #[test]
    fn all_selects_the_machine_including_workspaces_with_no_project() {
        let rows = rows();
        let me = WorkspaceId::from_stored("aaaaaaaa");
        assert_eq!(select(&rows, Lens::All, &me, None).len(), 4);
    }

    /// An orphaned worktree has no derivable project. `--project` then means
    /// "me", rather than erroring or — far worse — matching every other row
    /// whose project is also null.
    #[test]
    fn an_underivable_project_scopes_to_me_and_never_to_other_nulls() {
        let rows = rows();
        let me = WorkspaceId::from_stored("dddddddd");
        let selected = select(&rows, Lens::Project, &me, None);
        assert_eq!(selected.len(), 1);
        assert_eq!(selected[0].id, me);
    }

    #[test]
    fn project_and_all_together_is_not_a_lens() {
        assert_eq!(Lens::from_flags(false, false), Some(Lens::Workspace));
        assert_eq!(Lens::from_flags(true, false), Some(Lens::Project));
        assert_eq!(Lens::from_flags(false, true), Some(Lens::All));
        assert_eq!(Lens::from_flags(true, true), None);
    }
}
