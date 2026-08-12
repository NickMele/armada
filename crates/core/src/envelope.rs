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

use crate::error::{CharError, ErrClass, Status};
use crate::id::{ProjectId, WorkspaceId};
use crate::lease::WaitingOn;
use crate::ports::{PortBlock, PortState};
use crate::reap::ReapPlan;
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
    /// Prose for the states where the status alone does not say enough.
    ///
    /// PLAN.md §4.1 requires it on a skipped check —
    /// `{"status": "SKIPPED", "reason": "no matching files"}` — so an agent that
    /// expected a check to run can tell *no files matched* from *never
    /// selected*. `check` also uses it for the check a cascaded `ABORTED` names,
    /// which cannot go in `error` because a cascade may not set `error.class`.
    ///
    /// Additive, so `schema_version` does not bump, and omitted when absent, so
    /// no existing golden snapshot changes.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub reason: Option<String>,
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
            reason: None,
            log: None,
            duration_ms: None,
            error: None,
        }
    }
}

/// The one error a set of `results[]` aggregates to.
///
/// **The top-level `error` is the strict maximum over `results[]` by a fixed
/// precedence, so two implementations cannot disagree** (PLAN.md §3.1):
///
/// ```text
/// char_bug > environment > bad_config > bad_invocation > timeout > aborted > tool_failed
/// ```
///
/// The strictly-worse signal wins because acting on the milder one wastes the
/// time the stricter one was reporting: a gate reading exit 1 goes looking for
/// a broken test, while reading 4 it raises a deadline or asks why the suite
/// got slow. `environment` sits near the top for the same reason inverted —
/// when Docker is down or the disk is full, the failures underneath it are
/// consequences, and reporting one of them sends the caller to fix a repo that
/// is fine.
///
/// **This function is the single implementation of that rule**, and it exists
/// as one function precisely because the rule's stated purpose is that two
/// implementations cannot disagree. Every verb with a `results[]` aggregates
/// through it rather than counting rows for itself.
///
/// `subject` names what the rows are — `checks`, `workspaces`, `services` —
/// because the id grammar differs per verb and the message should read in the
/// caller's vocabulary.
pub fn aggregate(results: &[ResultRow], subject: &str) -> Option<CharError> {
    // A row that failed without attaching an error of its own still counts: a
    // verb reporting success over a `FAILED` row is the shape a consumer least
    // expects.
    let failures: Vec<(&ResultRow, ErrClass)> = results
        .iter()
        .filter_map(|row| {
            let class = match &row.error {
                Some(error) => Some(error.class),
                None => implied_class(row.status),
            };
            class.map(|class| (row, class))
        })
        .collect();

    let (worst, class) = *failures.iter().max_by_key(|(_, class)| class.severity())?;

    Some(CharError {
        class,
        // The id from `results[]`, which is the actionable thing for every
        // class but `bad_config` — and for that one the row's own `where` is
        // already a config path, so it is carried rather than replaced.
        r#where: match (class, worst.error.as_ref()) {
            (ErrClass::BadConfig, Some(error)) => error.r#where.clone(),
            _ => worst.id.clone(),
        },
        message: format!(
            "{} of {} {subject} did not succeed",
            failures.len(),
            results.len()
        ),
        // Required for `bad_config`, so it may not be dropped on the way up.
        next_action: worst
            .error
            .as_ref()
            .and_then(|error| error.next_action.clone()),
    })
}

/// The class a terminal state implies for a row that attached no error of its
/// own, and `None` for a state that is not a failure at all.
///
/// **The match is exhaustive rather than a catch-all, so a new terminal state
/// is a compile error here.** Defaulting every failure state to `tool_failed`
/// is not a neutral choice: a run of nothing but `TIMEOUT` rows would aggregate
/// to exit 1, which is the exact "a gate reading 1 goes looking for a broken
/// test" failure the precedence rule exists to prevent. PLAN.md §3.1's own
/// example payload carries `{"id": "api:test", "status": "ABORTED"}` with no
/// `error` object, so a row in this shape is specified rather than malformed.
///
/// `DEAD` maps to `aborted` with `ABORTED`: the run's holder dying and a
/// cancellation are the same answer to the caller — nothing about the work was
/// established, and the response is to retry rather than to read output.
const fn implied_class(status: Status) -> Option<ErrClass> {
    match status {
        Status::Failed => Some(ErrClass::ToolFailed),
        Status::Timeout => Some(ErrClass::Timeout),
        Status::Aborted | Status::Dead => Some(ErrClass::Aborted),
        // `PARTIAL` is an envelope-level state describing a mixed set; a row
        // carrying it has not said that it failed.
        Status::Ready
        | Status::Up
        | Status::Down
        | Status::Clean
        | Status::Pass
        | Status::Ok
        | Status::Skipped
        | Status::Partial
        | Status::Running
        | Status::Waiting => None,
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

/// `char check` (PLAN.md §3.1).
#[derive(Debug, Clone, PartialEq, Serialize)]
pub struct CheckData {
    /// The run these results belong to, and the directory its logs are in.
    pub run_id: String,
    /// One row per selected check, in id order.
    pub results: Vec<ResultRow>,
    /// Run directories reaped at the start of this run. **Reported, never
    /// silent** — a tool that removes things without saying so is worse than
    /// one that does not remove them.
    #[serde(skip_serializing_if = "Vec::is_empty")]
    pub reaped_runs: Vec<String>,
}

/// `--dry-run` on `char check`: what would run, and nothing changed.
#[derive(Debug, Clone, Default, PartialEq, Serialize)]
pub struct CheckDryRun {
    /// The exact argv each selected check would be given, post-substitution.
    /// Not a re-derivation: it is the same value the dispatch record would have
    /// carried, produced by the same code path.
    pub would_run: Vec<String>,
    /// Checks that would be skipped, and why — "no matching files" being the
    /// one that matters, since a preview that hid it would report work that is
    /// not going to happen.
    #[serde(skip_serializing_if = "Vec::is_empty")]
    pub would_skip: Vec<String>,
    /// Run directories that would be reaped.
    #[serde(skip_serializing_if = "Vec::is_empty")]
    pub would_reap: Vec<String>,
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
    ///
    /// **`status` asks no daemon.** It answers from `char.db` and a port probe,
    /// which is what makes it cheap enough to poll — and what §6.1's own
    /// `status --all` example needs, since a declared `release:` command is a
    /// recorded row rather than a labelled resource. Reaping is `init`'s and
    /// `clean`'s job; a read verb that took 300 ms of docker calls in a repo
    /// with no services would be a read verb nobody runs.
    #[serde(skip_serializing_if = "Vec::is_empty")]
    pub unreclaimed: Vec<Unreclaimed>,
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
    /// What the run would hand back: the port blocks on the ordinary path, and
    /// on `--force-rebuild` the `char.db` and its `-wal`/`-shm` sidecars that
    /// would be moved aside, plus the fresh database that would replace them.
    pub would_release: Vec<String>,
    /// Resources that would be removed.
    pub would_remove: Vec<String>,
    /// Declared `owns.files` that would be deleted — `--artifacts` only.
    pub would_delete: Vec<String>,
    /// What would be reported rather than reclaimed: the external resources of
    /// §6.1, and on the `--force-rebuild` path the labelled resources the run
    /// would deliberately leave alone.
    ///
    /// It also carries that path's diagnostics — the namespace note, and any
    /// enumeration that could not run. A preview has no `skipped` channel, and
    /// the alternative is a fifth field on the envelope's frozen contract for
    /// something one path emits, so they are reported here and named as such
    /// rather than dropped.
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

    fn failed(id: &str, class: ErrClass) -> ResultRow {
        let mut row = ResultRow::new(id, Status::Failed);
        row.error = Some(CharError {
            class,
            r#where: id.to_string(),
            message: format!("{id} did not"),
            next_action: None,
        });
        row
    }

    /// **The strictly-worse signal wins**, because acting on the milder one
    /// wastes the time the stricter one was reporting: a gate reading 1 goes
    /// looking for a broken test, while reading 4 it raises a deadline or asks
    /// why the suite got slow.
    #[test]
    fn the_aggregate_is_the_strict_maximum_over_the_rows() {
        let rows = [
            failed("api:lint", ErrClass::ToolFailed),
            failed("web:e2e", ErrClass::Timeout),
            failed("api:test", ErrClass::Aborted),
        ];
        let error = aggregate(&rows, "checks").expect("three failures aggregate to one");
        assert_eq!(error.class, ErrClass::Timeout);
        assert_eq!(error.class.exit_code(), 4, "not 1");
        assert_eq!(error.r#where, "web:e2e", "the worst row names itself");
    }

    /// PLAN.md §3.1's own example payload is internally inconsistent — it prints
    /// `tool_failed` for a set containing a `TIMEOUT`, and the prose beneath it
    /// corrects itself. The rule wins over the illustration, and this pins it.
    #[test]
    fn a_run_containing_a_timeout_exits_four_and_not_one() {
        let rows = [
            ResultRow::new("web:lint", Status::Pass),
            failed("api:lint", ErrClass::ToolFailed),
            failed("web:e2e", ErrClass::Timeout),
        ];
        assert_eq!(aggregate(&rows, "checks").unwrap().class.exit_code(), 4);
    }

    #[test]
    fn the_whole_precedence_order_holds_pairwise() {
        // char_bug > environment > bad_config > bad_invocation > timeout >
        // aborted > tool_failed
        let order = [
            ErrClass::ToolFailed,
            ErrClass::Aborted,
            ErrClass::Timeout,
            ErrClass::BadInvocation,
            ErrClass::BadConfig,
            ErrClass::Environment,
            ErrClass::CharBug,
        ];
        for (i, milder) in order.iter().enumerate() {
            for worse in order.iter().skip(i + 1) {
                let rows = [failed("a", *milder), failed("b", *worse)];
                assert_eq!(
                    aggregate(&rows, "checks").unwrap().class,
                    *worse,
                    "{worse:?} should beat {milder:?}"
                );
            }
        }
    }

    #[test]
    fn rows_that_all_succeeded_aggregate_to_no_error() {
        let rows = [
            ResultRow::new("a", Status::Clean),
            ResultRow::new("b", Status::Clean),
        ];
        assert!(aggregate(&rows, "workspaces").is_none());
        assert!(aggregate(&[], "workspaces").is_none());
    }

    /// A row that failed without attaching an error still has to reach the
    /// aggregate — otherwise a verb reports success while `results[]` shows a
    /// `FAILED` row, which is the shape a consumer least expects.
    #[test]
    fn a_failed_row_with_no_error_of_its_own_still_fails_the_run() {
        let rows = [ResultRow::new("a", Status::Failed)];
        let error = aggregate(&rows, "workspaces").expect("a FAILED row is a failure");
        assert_eq!(error.class, ErrClass::ToolFailed);
    }

    /// The state carries the class when the row did not attach one. A run of
    /// nothing but timed-out rows exits 4, so a gate raises a deadline rather
    /// than hunting a broken test — and `check`, the verb that produces these
    /// rows, is coded against this.
    #[test]
    fn a_terminal_state_with_no_error_of_its_own_scores_its_own_class() {
        for (status, class) in [
            (Status::Failed, ErrClass::ToolFailed),
            (Status::Timeout, ErrClass::Timeout),
            (Status::Aborted, ErrClass::Aborted),
            (Status::Dead, ErrClass::Aborted),
        ] {
            let rows = [ResultRow::new("api:test", status)];
            let error = aggregate(&rows, "checks").expect("a terminal failure is a failure");
            assert_eq!(error.class, class, "{status:?} scored the wrong class");
        }
    }

    /// PLAN.md §3.1's own example payload contains a row that is nothing but an
    /// id and `ABORTED`, so the shape is specified — and mixed with an ordinary
    /// failure the stricter one still wins.
    #[test]
    fn an_aborted_row_with_no_error_beats_a_tool_failure() {
        let rows = [
            failed("api:lint", ErrClass::ToolFailed),
            ResultRow::new("api:test", Status::Aborted),
        ];
        let error = aggregate(&rows, "checks").unwrap();
        assert_eq!(error.class, ErrClass::Aborted);
        assert_eq!(error.class.exit_code(), 5, "not 1");
        assert_eq!(error.r#where, "api:test");
    }

    #[test]
    fn a_progress_or_success_state_never_becomes_a_failure() {
        for status in [
            Status::Ready,
            Status::Up,
            Status::Down,
            Status::Clean,
            Status::Pass,
            Status::Ok,
            Status::Skipped,
            Status::Partial,
            Status::Running,
            Status::Waiting,
        ] {
            let rows = [ResultRow::new("a", status)];
            assert!(
                aggregate(&rows, "checks").is_none(),
                "{status:?} was admitted as a failure"
            );
        }
    }

    #[test]
    fn the_message_counts_the_failures_and_names_the_subject() {
        let rows = [
            ResultRow::new("a", Status::Clean),
            failed("b", ErrClass::ToolFailed),
            failed("c", ErrClass::ToolFailed),
        ];
        assert_eq!(
            aggregate(&rows, "workspaces").unwrap().message,
            "2 of 3 workspaces did not succeed"
        );
    }

    /// `next_action` is required for `bad_config`, so the aggregate may not drop
    /// the one the row carried.
    #[test]
    fn a_bad_config_aggregate_keeps_its_next_action() {
        let mut row = ResultRow::new("api", Status::Failed);
        row.error = Some(CharError::bad_config(
            crate::error::ConfigWhere::Path {
                file: "char.yml".into(),
                path: "components.api.setup".into(),
            },
            "no such step",
            "correct the setup step",
        ));
        let error = aggregate(&[row], "components").unwrap();
        assert_eq!(error.class, ErrClass::BadConfig);
        assert_eq!(error.next_action.as_deref(), Some("correct the setup step"));
    }

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
