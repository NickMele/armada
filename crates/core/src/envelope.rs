//! The `--json` envelope (PLAN.md §3.1) and the per-verb bodies phase 2 fills.
//!
//! ```json
//! { "schema_version": 1, "verb": "init", "workspace": "a3f91c02",
//!   "status": "READY", "error": null, "data": {} }
//! ```
//!
//! The body is **nested rather than flattened** so the envelope is generically
//! validatable — one schema checks the wrapper, a per-verb schema checks `data`
//! — and so a future verb can add a field called `status` or `error` without
//! colliding with the envelope.
//!
//! **Everything here is serialized from structs and never assembled as a
//! `serde_json::Value`.** Measured: `Value`'s map is a `BTreeMap`, so anything
//! routed through one comes out alphabetised, while struct fields emit in
//! declaration order. Every payload in this corpus is written in *reading*
//! order — `schema_version`, `verb`, `workspace`, `status`, `error`, `data` — so
//! a `Value` would make every hand-regenerated golden snapshot wrong, and the
//! obvious fix (reorder the snapshot) hides that the renderer stopped emitting
//! the documented order. See `docs/traps.md`.

use crate::error::{CharError, Status};
use crate::id::{ProjectId, WorkspaceId};
use crate::lease::WaitingOn;
use crate::ports::{PortBlock, PortState};
use crate::reap::{ReapPlan, Reported};
use serde::Serialize;
use std::collections::BTreeMap;

/// One global version for the whole CLI contract.
///
/// **Adding a field does not bump; removing a field or changing its type
/// does.** That rule is checkable, and it lets a consumer say "I need ≥ 1" and
/// be right. It is global rather than per-verb because six verbs ship in one
/// binary and an agent uses all of them.
pub const SCHEMA_VERSION: u32 = 1;

/// The fixed wrapper every verb answers in.
#[derive(Debug, Clone, PartialEq, Serialize)]
pub struct Envelope<D: Serialize> {
    /// See [`SCHEMA_VERSION`].
    pub schema_version: u32,
    /// Which verb produced this.
    pub verb: String,
    /// **Always the invoking workspace**, even under `--project` / `--all` —
    /// other workspaces appear inside `data`, so the envelope shape never
    /// varies. `null` when workspace resolution is what failed, or for a
    /// machine-scoped invocation run from outside a workspace. A consumer must
    /// tolerate it, so the field is emitted rather than skipped.
    pub workspace: Option<WorkspaceId>,
    /// The terminal state — or, for the three read verbs only, a progress one.
    pub status: Status,
    /// The typed error, or `null`.
    pub error: Option<CharError>,
    /// The per-verb body.
    pub data: D,
}

impl<D: Serialize> Envelope<D> {
    /// A successful answer.
    pub fn ok(verb: &str, workspace: Option<WorkspaceId>, status: Status, data: D) -> Self {
        Envelope {
            schema_version: SCHEMA_VERSION,
            verb: verb.to_string(),
            workspace,
            status,
            error: None,
            data,
        }
    }

    /// A failure. `status` is `FAILED` whenever `error` is non-null and no more
    /// specific terminal state applies (PLAN.md §3.2.2) — which includes
    /// `char status`, whose only success state is `OK` and which otherwise had
    /// no way to report that it failed.
    pub fn failed(verb: &str, workspace: Option<WorkspaceId>, error: CharError, data: D) -> Self {
        Envelope {
            schema_version: SCHEMA_VERSION,
            verb: verb.to_string(),
            workspace,
            status: Status::Failed,
            error: Some(error),
            data,
        }
    }

    /// The process exit code: `f(error.class)`, or 0.
    pub fn exit_code(&self) -> u8 {
        self.error.as_ref().map_or(0, |e| e.class.exit_code())
    }

    /// Render, in reading order, with a trailing newline.
    pub fn to_json(&self) -> String {
        let mut json = serde_json::to_string_pretty(self).unwrap_or_else(|e| {
            // Serializing a tree of owned strings and integers cannot fail for
            // any reason the caller could act on, and a panic here would lose
            // the payload entirely. Report it in the one shape a consumer can
            // still parse.
            format!(
                "{{\"schema_version\":{SCHEMA_VERSION},\"verb\":\"unknown\",\"workspace\":null,\
                 \"status\":\"FAILED\",\"error\":{{\"class\":\"char_bug\",\"where\":\"renderer\",\
                 \"message\":\"could not serialize the envelope: {e}\"}},\"data\":{{}}}}"
            )
        });
        json.push('\n');
        json
    }
}

/// One entry in `data.results[]`.
///
/// **`id` is a different grammar per verb, and that is intended**: a component
/// name for `init`, a workspace id for `clean --all`, a check id for `check`.
/// One field, because the *shape* is what an agent learns once — iterate
/// `results[]`, read `status`, read `error` — and the id is opaque to that loop.
///
/// Optional fields are omitted when empty. A row is meant to be readable, and a
/// `"ports": {}` on every entry buries the two that have one.
#[derive(Debug, Clone, PartialEq, Serialize)]
pub struct ResultRow {
    /// The id, in this verb's grammar.
    pub id: String,
    /// This entry's own state.
    pub status: Status,
    /// Where the workspace is, for the rows that describe one.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub path: Option<String>,
    /// The grouping key, when it is derivable.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub project: Option<ProjectId>,
    /// The span reserved for the workspace this row describes.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub port_block: Option<PortBlock>,
    /// Port name → what a probe found, right now.
    #[serde(skip_serializing_if = "BTreeMap::is_empty")]
    pub ports: BTreeMap<String, PortReport>,
    /// What this row is holding.
    #[serde(skip_serializing_if = "Vec::is_empty")]
    pub leases: Vec<String>,
    /// What was reclaimed for this row.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub released: Option<Released>,
    /// Why this row is not executing.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub waiting_on: Option<WaitingOn>,
    /// Where this row's output went.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub log: Option<String>,
    /// How long it took.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub duration_ms: Option<u64>,
    /// This row's own failure.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub error: Option<CharError>,
}

impl ResultRow {
    /// A bare row: an id and a state, with every optional field empty.
    ///
    /// Written out rather than derived from `Default`, because `Status` has no
    /// defensible default — every candidate is either a success char has not
    /// earned or a failure it has not established.
    pub fn new(id: impl Into<String>, status: Status) -> Self {
        ResultRow {
            id: id.into(),
            status,
            path: None,
            project: None,
            port_block: None,
            ports: BTreeMap::new(),
            leases: Vec::new(),
            released: None,
            waiting_on: None,
            log: None,
            duration_ms: None,
            error: None,
        }
    }
}

/// A port and what a probe found at it.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
pub struct PortReport {
    /// The assigned port.
    pub port: u16,
    /// Probed at report time, never remembered.
    pub state: PortState,
}

/// What `clean` actually released for one workspace.
#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize)]
pub struct Released {
    /// Process groups killed.
    pub processes: usize,
    /// Containers removed.
    pub containers: usize,
    /// Networks removed.
    pub networks: usize,
    /// Volumes removed.
    pub volumes: usize,
    /// Built images removed.
    pub images: usize,
    /// Whether the port block went back.
    pub port_block: bool,
    /// Declared `owns.files` deleted — `--artifacts` only.
    pub files: usize,
}

/// `char init`.
#[derive(Debug, Clone, PartialEq, Serialize)]
pub struct InitData {
    /// The span reserved for this workspace, and when.
    pub port_block: PortBlock,
    /// When the block was claimed. Wall clock, and only ever displayed.
    pub claimed_at: String,
    /// Port name → assigned port, workspace-global.
    #[serde(skip_serializing_if = "BTreeMap::is_empty")]
    pub ports: BTreeMap<String, u16>,
    /// What the reap passes did. **Reported, never silent.**
    pub reaped: ReapPlan,
    /// One row per component.
    pub results: Vec<ResultRow>,
}

/// `char clean`.
#[derive(Debug, Clone, PartialEq, Serialize)]
pub struct CleanData {
    /// What the reap passes did.
    pub reaped: ReapPlan,
    /// One row per workspace touched — `--all` and `--project` make this
    /// plural, which is why `clean` reports `PARTIAL`.
    pub results: Vec<ResultRow>,
    /// External resources char **recorded and did not reclaim** (PLAN.md
    /// §6.1). A stale `DROP DATABASE` is strictly more dangerous than a stale
    /// `kill`, so the same answer applies with more force: report it.
    #[serde(skip_serializing_if = "Vec::is_empty")]
    pub unreclaimed: Vec<Unreclaimed>,
    /// Workspaces skipped because they hold a live lease. `--all` is every
    /// workspace on this machine, so the unguarded version stops four live
    /// stacks while their agents are mid-run.
    #[serde(skip_serializing_if = "Vec::is_empty")]
    pub skipped: Vec<String>,
}

/// `char status`.
#[derive(Debug, Clone, PartialEq, Serialize)]
pub struct StatusData {
    /// Which lens produced this: `workspace`, `project` or `all`.
    pub scope: String,
    /// One row per workspace in scope.
    pub results: Vec<ResultRow>,
    /// External resources char will never reclaim, named so a human can.
    #[serde(skip_serializing_if = "Vec::is_empty")]
    pub unreclaimed: Vec<Unreclaimed>,
    /// Labelled resources char found and deliberately left alone.
    #[serde(skip_serializing_if = "Vec::is_empty")]
    pub reported: Vec<Reported>,
}

/// A declared external resource: recorded at `init`, reported here, **never
/// executed**.
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct Unreclaimed {
    /// The workspace that declared it.
    pub workspace: WorkspaceId,
    /// The resolved command, with `${workspace.id}` already substituted — so
    /// it runs from anywhere, including after the workspace is gone.
    pub command: String,
    /// Whether that workspace's directory still exists.
    pub workspace_exists: bool,
}

/// A dispatched `commands:` entry (PLAN.md §4.5).
#[derive(Debug, Clone, PartialEq, Serialize)]
pub struct DispatchData {
    /// The entry's name.
    pub command: String,
    /// **True only if the child was executed.** This is what disambiguates a
    /// child exiting 3 from char's own `bad_config`: char's error codes can
    /// only occur when the child never ran.
    pub dispatched: bool,
    /// The child's code, **verbatim and unremapped** — char did not decide the
    /// outcome, so it does not get to classify it.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub child_exit: Option<i32>,
    /// The argv char executed, post-substitution and post-passthrough. Cheap to
    /// record, impossible to reconstruct without reimplementing the split.
    pub argv: Vec<String>,
}

/// `--dry-run` on `clean`: the ordinary envelope with `would_*` in place of
/// `results[]`, and nothing changed.
#[derive(Debug, Clone, Default, PartialEq, Serialize)]
pub struct CleanDryRun {
    /// Port blocks that would go back.
    pub would_release: Vec<String>,
    /// Resources that would be removed.
    pub would_remove: Vec<String>,
    /// Declared `owns.files` that would be deleted — `--artifacts` only.
    pub would_delete: Vec<String>,
    /// External resources that would be reported rather than reclaimed.
    pub would_report: Vec<String>,
}

/// `--dry-run` on `init`.
#[derive(Debug, Clone, Default, PartialEq, Serialize)]
pub struct InitDryRun {
    /// The block that would be claimed.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub would_claim: Option<PortBlock>,
    /// Setup steps that would run, in order.
    pub would_run: Vec<String>,
    /// What the reap passes would do.
    pub would_reap: ReapPlan,
}

/// A body with nothing in it, for a failure that never got as far as one.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, Serialize)]
pub struct NoData {}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::error::ErrClass;

    #[test]
    fn the_envelope_emits_its_fields_in_reading_order() {
        // The verb is deliberately not `status`: `"status"` would then appear
        // as a *value* before the key of the same name, and the assertion
        // would pass on the wrong match.
        let envelope = Envelope::ok(
            "init",
            Some(WorkspaceId::from_stored("a3f91c02")),
            Status::Ready,
            NoData {},
        );
        let json = envelope.to_json();
        let mut last = None;
        for key in [
            "schema_version",
            "verb",
            "workspace",
            "status",
            "error",
            "data",
        ] {
            let at = json
                .find(&format!("\"{key}\""))
                .unwrap_or_else(|| panic!("{key} missing from {json}"));
            if let Some(previous) = last {
                assert!(at > previous, "{key} is out of reading order in {json}");
            }
            last = Some(at);
        }
    }

    #[test]
    fn a_null_workspace_is_emitted_rather_than_skipped() {
        let envelope = Envelope::ok("clean", None, Status::Clean, NoData {});
        assert!(envelope.to_json().contains("\"workspace\": null"));
    }

    #[test]
    fn a_successful_envelope_carries_a_null_error_and_exits_zero() {
        let envelope = Envelope::ok("init", None, Status::Ready, NoData {});
        assert!(envelope.to_json().contains("\"error\": null"));
        assert_eq!(envelope.exit_code(), 0);
    }

    #[test]
    fn the_exit_code_follows_the_class_and_nothing_else() {
        let envelope = Envelope::failed(
            "init",
            None,
            CharError {
                class: ErrClass::Environment,
                r#where: "docker".into(),
                message: "daemon unreachable".into(),
                next_action: None,
            },
            NoData {},
        );
        assert_eq!(envelope.status, Status::Failed);
        assert_eq!(envelope.exit_code(), 6);
    }

    #[test]
    fn an_empty_result_row_omits_every_optional_field() {
        let row = ResultRow::new("web", Status::Ready);
        let json = serde_json::to_string(&row).unwrap();
        assert_eq!(json, r#"{"id":"web","status":"READY"}"#);
    }

    #[test]
    fn the_payload_ends_with_exactly_one_newline() {
        let json = Envelope::ok("status", None, Status::Ok, NoData {}).to_json();
        assert!(json.ends_with("}\n"));
        assert!(!json.ends_with("\n\n"));
    }
}
