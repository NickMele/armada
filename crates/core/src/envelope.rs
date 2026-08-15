//! The `--json` envelope (PLAN.md §3.1) and the per-verb bodies phase 2 fills.
//!
//! ```json
//! { "schema_version": 2, "verb": "init", "workspace": "a3f91c02",
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

use crate::error::{ArmadaError, ErrClass, Status};
use crate::id::{ProjectId, WorkspaceId};
use crate::lease::WaitingOn;
use crate::ports::{PortBlock, PortState};
use crate::reap::ReapPlan;
use serde::Serialize;
use std::collections::BTreeMap;
use std::fmt;

/// One global version for the whole CLI contract.
///
/// **Adding a field does not bump; removing a field or changing its type
/// does.** That rule is checkable, and it lets a consumer say "I need ≥ 1" and
/// be right. It is global rather than per-verb because six verbs ship in one
/// binary and an agent uses all of them.
///
/// **2 because [`InitData::port_block`] and [`ServicesData::port_block`] went
/// from `PortBlock` to `Option<PortBlock>`.** That is the "changing its type"
/// clause, not the "adding a field" one: both keys are emitted unconditionally,
/// so code that read `port_block.from` used to be right on every payload and
/// now breaks on the one a workspace with no `ports:` produces. A reader cannot
/// tell that from the version staying still.
///
/// **The envelope's own `workspace` is not the precedent it looks like.** That
/// key was nullable in version 1 and documented as such, so every consumer that
/// ever existed was written against a `T | null`. `port_block` changed *under* a
/// reader who had no reason to expect it. Nullable-from-the-start and
/// nullable-since are the same shape and different promises, and the bump rule
/// is about the promise.
pub const SCHEMA_VERSION: u32 = 2;

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
    pub error: Option<ArmadaError>,
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
    /// `armada manifest status`, whose only success state is `OK` and which otherwise had
    /// no way to report that it failed.
    pub fn failed(verb: &str, workspace: Option<WorkspaceId>, error: ArmadaError, data: D) -> Self {
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
                 \"status\":\"FAILED\",\"error\":{{\"class\":\"armada_bug\",\"where\":\"renderer\",\
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
    /// **What Armada owns for this row, by id** — `container:armada-a3f91c02-api`,
    /// `volume:pgdata`, `pgid:4212`.
    ///
    /// **Ids, never counts.** A count answers neither of the two questions
    /// anyone asks of this field: what will `armada manifest clean` remove, and
    /// what do I go and look at by hand. "3 containers" sends the reader to
    /// `docker ps` to find out which three, which is the work the field exists to
    /// save.
    ///
    /// **`<kind>:<reference>`, with no exceptions**, so one grammar covers every
    /// kind and a caller splits on the first colon. It is the grammar [`leases`]
    /// already uses in this same struct, which is why it needs no second
    /// explanation.
    ///
    /// **A resolved `owns.release:` command is deliberately not here.** It is a
    /// command rather than a resource, Armada never executes it, and
    /// [`StatusData::unreclaimed`] already carries it with the one fact that
    /// matters — whether the workspace that declared it still exists. Listing it
    /// twice would make a reader wonder which one was authoritative.
    ///
    /// **Additive, so it did not bump `schema_version`** (PLAN.md §3.1 — adding a
    /// field does not bump; removing one or changing its type does). Omitted when
    /// empty, so no verb that owns nothing gains a key.
    ///
    /// [`leases`]: ResultRow::leases
    #[serde(skip_serializing_if = "Vec::is_empty")]
    pub owns: Vec<String>,
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
    /// The exact vector Armada executed for this row, post-substitution.
    ///
    /// **`armada manifest up`'s documented payload asks for it by name**
    /// ([`commands/manifest/up.md`]: *"one result per component with argv, the
    /// ready-check that was waited on, the wait duration, and the assigned
    /// ports"*), and it is the one fact about a service that is impossible to
    /// reconstruct afterwards — `${port.NAME}` has already been substituted and
    /// the config no longer says what ran.
    ///
    /// Additive, so it did not bump `schema_version`, and omitted when empty, so
    /// no verb that spawns nothing gains a key.
    ///
    /// [`commands/manifest/up.md`]: ../../../docs/commands/manifest/up.md
    #[serde(skip_serializing_if = "Vec::is_empty")]
    pub argv: Vec<String>,
    /// How long it took.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub duration_ms: Option<u64>,
    /// This row's own failure.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub error: Option<ArmadaError>,
}

impl ResultRow {
    /// A bare row: an id and a state, with every optional field empty.
    ///
    /// Written out rather than derived from `Default`, because `Status` has no
    /// defensible default — every candidate is either a success Armada has not
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
            owns: Vec::new(),
            released: None,
            waiting_on: None,
            reason: None,
            log: None,
            argv: Vec::new(),
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
/// armada_bug > environment > bad_config > bad_invocation > timeout > aborted > tool_failed
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
pub fn aggregate(results: &[ResultRow], subject: &str) -> Option<ArmadaError> {
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

    Some(ArmadaError {
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
/// own, and `None` for a state that does not establish a failure.
///
/// **The match is exhaustive rather than a catch-all, so a new terminal state
/// is a compile error here.** Defaulting every failure state to `tool_failed`
/// is not a neutral choice: a run of nothing but `TIMEOUT` rows would aggregate
/// to exit 1, which is the exact "a gate reading 1 goes looking for a broken
/// test" failure the precedence rule exists to prevent.
///
/// **This inference is Armada's, not the specification's, and it is narrower than
/// it first shipped.** PLAN.md §3.1's precedence chain runs over `error`
/// *classes*, and a row with no `error` object has none — so §3.1 is silent
/// about what such a row contributes, and everything below is Armada filling that
/// silence. The first version filled it for every failure state alike. That is
/// right for `FAILED`, where the alternative is a verb reporting success while
/// `results[]` shows a failure, and wrong for the two states that mean *no
/// verdict was reached*:
///
/// - **`ABORTED` implies nothing.** PLAN.md §4.1 is explicit that a check
///   abandoned because its prerequisite failed "never sets `error.class`", and
///   spells out why: `aborted` is the *retryable* class, so a run whose only
///   real failure is a deterministic test failure would tell a merge gate to
///   try again on a bug that will fail identically forever. `aborted` is
///   reserved for a run stopped from **outside** it — SIGINT, or the
///   acquisition ceiling — and both of those set the run's error directly
///   rather than inferring it from a row.
/// - **`DEAD` implies nothing**, for the same reason: the run's holder dying is
///   a fact about the run, and the verb reporting it knows that without asking
///   a row.
///
/// A row that genuinely carries `aborted` — a claim that hit the ceiling — still
/// aggregates like any other, because it attaches a real `error` object. What
/// changed is only what is inferred in the absence of one.
const fn implied_class(status: Status) -> Option<ErrClass> {
    match status {
        Status::Failed => Some(ErrClass::ToolFailed),
        Status::Timeout => Some(ErrClass::Timeout),
        Status::Aborted | Status::Dead => None,
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
///
/// `Deserialize` because `armada fleet kill` runs `armada manifest clean --json`
/// in the Job's worktree and reads its answer back (`crates/fleet`'s `manifest`
/// module). The envelope is a contract in both directions, and this is the one
/// place Armada is its own consumer.
#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, serde::Deserialize)]
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

/// `armada manifest init`.
#[derive(Debug, Clone, PartialEq, Serialize)]
pub struct InitData {
    /// The span reserved for this workspace, or `null` for one that needs none.
    ///
    /// **A block exists so parallel worktrees do not collide on a service's
    /// port** (PLAN.md §2.2). A workspace whose components declare no `ports:`
    /// has nothing to collide over, so none is claimed and none is printed —
    /// a range that reserved nothing was ten ports of a finite pool taken from
    /// a workspace that did need them.
    ///
    /// **Emitted as `null` rather than omitted**, which is the rule the
    /// envelope's own `workspace` already follows: a consumer reading this key
    /// unconditionally finds it, and finds out that there is no block, instead
    /// of finding nothing at all.
    ///
    /// **This field is why [`SCHEMA_VERSION`] is 2.** It was `PortBlock` and is
    /// now `Option<PortBlock>` — a type change under a reader, which the bump
    /// rule says bumps. Borrowing `workspace`'s emit-rather-than-skip *shape*
    /// does not borrow its version history: `workspace` was nullable in version
    /// 1, so no consumer was ever promised otherwise, and this one was.
    pub port_block: Option<PortBlock>,
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

/// `armada manifest up` and `armada manifest down`.
///
/// **One body for both verbs, because they answer the same question from two
/// sides.** `up` reports what it started and `down` what it stopped; both carry
/// one row per component and the workspace's port block, and PLAN.md §3.1 is
/// explicit that `init`, `up`, `down` and `status` all emit `results[]` — *"that
/// is what lets the two states with ports but no running services (`init`, and
/// `down` which keeps the block) report them without a second, duplicate
/// top-level map."*
///
/// **`down` carries the block precisely because it keeps it.** That is the whole
/// distinction from `clean`: `down` is pause and `clean` is release, and the
/// next `up` gets the same ports, which keeps URLs, bookmarks and `.env` files
/// valid across a restart.
#[derive(Debug, Clone, PartialEq, Serialize)]
pub struct ServicesData {
    /// The span reserved for this workspace, held across `down` — or `null` for
    /// a workspace that needs none. See [`InitData::port_block`].
    pub port_block: Option<PortBlock>,
    /// One row per selected component, in the order it was acted on —
    /// dependency order for `up`, the reverse for `down`.
    pub results: Vec<ResultRow>,
}

/// `--dry-run` on `armada manifest up`: what would run, and nothing changed.
#[derive(Debug, Clone, Default, PartialEq, Serialize)]
pub struct UpDryRun {
    /// The exact argv each selected service would be given, in start order.
    /// **Not a re-derivation**: it is produced by the same code path that would
    /// have executed it.
    pub would_run: Vec<String>,
    /// The ready-check each one would then be waited on, and for how long. A
    /// preview that showed the spawn and hid the wait would hide the half that
    /// takes the time.
    pub would_wait: Vec<String>,
}

/// `armada manifest check` (PLAN.md §3.1).
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

/// `--dry-run` on `armada manifest check`: what would run, and nothing changed.
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

/// `armada manifest clean`.
#[derive(Debug, Clone, PartialEq, Serialize)]
pub struct CleanData {
    /// What the reap passes did.
    pub reaped: ReapPlan,
    /// One row per workspace touched — `--all` and `--project` make this
    /// plural, which is why `clean` reports `PARTIAL`.
    pub results: Vec<ResultRow>,
    /// External resources Armada **recorded and did not reclaim** (PLAN.md
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

/// `armada manifest status`.
#[derive(Debug, Clone, PartialEq, Serialize)]
pub struct StatusData {
    /// Which lens produced this: `workspace`, `project` or `all`.
    pub scope: String,
    /// One row per workspace in scope.
    pub results: Vec<ResultRow>,
    /// External resources Armada will never reclaim, named so a human can.
    ///
    /// **`status` asks no daemon.** It answers from `manifest.db` and a port probe,
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
    /// child exiting 3 from Armada's own `bad_config`: Armada's error codes can
    /// only occur when the child never ran.
    pub dispatched: bool,
    /// The child's code, **verbatim and unremapped** — Armada did not decide the
    /// outcome, so it does not get to classify it.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub child_exit: Option<i32>,
    /// The argv Armada executed, post-substitution and post-passthrough. Cheap to
    /// record, impossible to reconstruct without reimplementing the split.
    pub argv: Vec<String>,
}

/// `--dry-run` on `clean`: the ordinary envelope with `would_*` in place of
/// `results[]`, and nothing changed.
#[derive(Debug, Clone, Default, PartialEq, Serialize)]
pub struct CleanDryRun {
    /// What the run would hand back: the port blocks on the ordinary path, and
    /// on `--force-rebuild` the `manifest.db` and its `-wal`/`-shm` sidecars that
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

/// `armada manifest config scan` — layer 1 of PLAN.md §5.
///
/// **Two views of one scan, and neither is derived from the other by a
/// reader.** `results[]` is the summary a caller iterates the same way it
/// iterates every other verb's — one row per finding, with the file it came
/// from in `path` and the one-line detail in `reason`. `evidence` is the
/// verbatim material that summary counts: every script, every service, every
/// CI step. The agent authoring the config wants the second; a script asking
/// "is there a compose file here" wants the first.
///
/// **There is no absent row.** A kind that turned nothing up contributes
/// nothing to `results[]`, because an absence is not a finding — the human
/// render says `absent` for it, which is a statement about the fixed set of
/// kinds it draws rather than about anything the scan produced.
#[derive(Debug, Clone, Default, PartialEq, Serialize)]
pub struct ScanData {
    /// One row per finding, in the order the human render draws them.
    pub results: Vec<ResultRow>,
    /// The facts themselves, uninterpreted and untruncated.
    pub evidence: crate::scan::Evidence,
    /// What happens once the evidence has been printed: ask, tell, or neither.
    ///
    /// **In the payload rather than derived by the renderer**, and **beside
    /// the evidence rather than inside it**. It is decided from facts only the
    /// entrypoint has — whether each stream is a terminal, whether there is a
    /// skill to hand over to — and none of those is a fact about the
    /// repository, which is what `evidence` holds and all it holds.
    ///
    /// A renderer that worked it out for itself would give the two human
    /// audiences different *content* rather than different styling, which is
    /// the one way they may not differ (PLAN.md §3.1.1).
    #[serde(default)]
    pub handover: crate::scan::Handover,
}

/// `armada manifest config verify` — layer 3 of PLAN.md §5.
///
/// **Two passes, and only the first is cheap.** `results[]` is pass 1: static,
/// seconds, nothing executed. `pass_2` is the real run of the check suite, and
/// it is `None` whenever pass 1 did not pass — *not attempted* rather than
/// *skipped*, which is why the field is absent instead of empty.
#[derive(Debug, Clone, Default, PartialEq, Serialize)]
pub struct VerifyData {
    /// Pass 1, one row per static check.
    pub results: Vec<ResultRow>,
    /// Entries under `shell: true`, which have **no `argv[0]` to resolve**
    /// (PLAN.md §5). Counted rather than guessed at or silently passed: the
    /// string is a program in a language Armada does not parse, and this number
    /// is the honest cost of that key.
    pub unchecked: usize,
    /// Pass 2, when pass 1 passed and the suite therefore ran for real.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub pass_2: Option<Box<CheckData>>,
}

/// `armada manifest skills` and `armada manifest skills show <name>`.
///
/// **There is deliberately no way to run one** (PLAN.md §4.8). "Add a
/// migration" has no deterministic expansion; the `commands:` entry it names
/// does, and that already has a verb. A runner would mean Armada choosing
/// arguments on the user's behalf, which is precisely what §5's layer 1
/// refuses to do.
#[derive(Debug, Clone, Default, PartialEq, Serialize)]
pub struct SkillsData {
    /// One row per skill listed, in name order.
    pub results: Vec<ResultRow>,
    /// The resolved skills themselves — the same structure the CLI table, a
    /// future MCP response and the generated frontmatter are three renderings
    /// of, so a skill cannot mean one thing to a shell caller and another to an
    /// agent.
    pub skills: Vec<ResolvedSkillView>,
}

/// `armada manifest components` — what this repository can be filtered by.
///
/// **The question `--component <name>` could not answer for itself.** A caller
/// told to narrow a run to a component had no way to learn what the components
/// were except by opening `armada.yml`, which is the first thing a newcomer —
/// human or agent — asks and the last thing they should have to parse. `skills:`
/// and `commands:` had the same gap; this is the same answer, in the same shape
/// (PLAN.md §4.5's reserved `armada manifest commands` is the third).
#[derive(Debug, Clone, PartialEq, Serialize)]
pub struct ComponentsData {
    /// One row per component, in name order.
    pub results: Vec<ResultRow>,
    /// The components themselves, so an agent reads structure rather than a
    /// rendered table.
    pub components: Vec<ComponentView>,
}

/// One component, as the listing describes it.
#[derive(Debug, Clone, PartialEq, Serialize)]
pub struct ComponentView {
    /// The declared name — what `--component <name>` takes.
    pub name: String,
    /// Where its source lives, when it says.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub root: Option<String>,
    /// Whether it declares a `run:`, and so takes part in `up` and `down`.
    ///
    /// **A boolean rather than the driver's name.** What a caller is deciding
    /// is whether `armada manifest up <name>` means anything for this component;
    /// which driver it uses is a different question and `status` answers it.
    pub runs: bool,
    /// Its checks, by the selector each is reached with — `<component>:<check>`
    /// truncated to the check's own name, since the component is the row.
    #[serde(skip_serializing_if = "Vec::is_empty")]
    pub checks: Vec<String>,
}

/// One skill, resolved: its grants expanded to the commands they name.
#[derive(Debug, Clone, PartialEq, Serialize)]
pub struct ResolvedSkillView {
    /// The declared name.
    pub name: String,
    /// One line, for listings and generated frontmatter.
    pub summary: String,
    /// Workspace-relative path to the prose. **Held, never parsed**
    /// (`ARCHITECTURE.md` §1.9).
    pub doc: String,
    /// The `commands:` entries this skill may invoke, and what each of them
    /// runs. **`uses:` grants nothing** — every name here was already declared
    /// under `commands:`, which is what makes a repo-authored skill safe to
    /// load.
    #[serde(skip_serializing_if = "Vec::is_empty")]
    pub uses: Vec<GrantedCommand>,
    /// The check scope that proves the work landed.
    #[serde(skip_serializing_if = "Vec::is_empty")]
    pub verify: Vec<String>,
    /// Advisory globs. Not enforced.
    #[serde(skip_serializing_if = "Vec::is_empty")]
    pub touches: Vec<String>,
}

/// A `commands:` entry a skill names, with the command it resolves to.
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct GrantedCommand {
    /// The `commands:` key.
    pub name: String,
    /// What that entry runs, as the config declares it.
    pub cmd: String,
}

/// A body with nothing in it, for a failure that never got as far as one.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, Serialize)]
pub struct NoData {}

// ------------------------------------------------------------------- M2:
// the machine, the guild, and the one word that is not a `Status`

/// **The one uppercase word in Armada's output that is not a [`Status`].**
///
/// `armada doctor` and `armada guild pull` end on `NEEDS ATTENTION`
/// (`tests/golden/render/doctor.plain`, `guild-pull.plain`), and no terminal
/// state spells that. The two verbs are reporting the condition of a *machine*
/// rather than the outcome of a *run*, and the distinction is real: `FAILED`
/// would say `doctor` failed, which it did not — it succeeded, at telling you
/// something is wrong.
///
/// `render.rs`'s rule is that an uppercase word is the envelope's own spelling
/// and a lowercase one is render-only. **This keeps that rule rather than
/// breaking it**: the word is in the payload, under `data.headline`, spelled
/// exactly as it is printed. A reader can still grep for anything they saw.
///
/// **One variant, deliberately.** The healthy case leads with an ordinary
/// [`Status`], so exactly one word in the whole CLI needed inventing and it is
/// declared here where it can be counted.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
pub enum Headline {
    /// Something on this machine needs a person. Not a failure of the verb.
    #[serde(rename = "NEEDS ATTENTION")]
    NeedsAttention,
}

impl fmt::Display for Headline {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(match self {
            Headline::NeedsAttention => "NEEDS ATTENTION",
        })
    }
}

/// `armada init` — set up **this machine**.
///
/// Not to be confused with [`InitData`], which claims a *workspace*.
///
/// **The body is a transcript, because the verb is a conversation.** Every
/// other envelope in this file describes work that happened; this one also
/// carries what was *asked* and what was answered, because `armada init` is the
/// one verb that interviews you and the agreed layout
/// (`tests/golden/render/init-machine.plain`) prints the questions. A reader of
/// `--json` gets the same account the terminal did.
#[derive(Debug, Clone, PartialEq, Serialize)]
pub struct MachineInitData {
    /// One row per preflight check, plus one for the directories created.
    ///
    /// The same [`Finding`] rows `armada doctor` reports, because the two verbs
    /// are asking one question about one machine.
    pub results: Vec<Finding>,
    /// The one question that matters, and which of the three was chosen.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub guild: Option<GuildChoice>,
    /// What the import adopted, as the facts of one line.
    #[serde(skip_serializing_if = "Vec::is_empty")]
    pub imported: Vec<String>,
    /// The interview prompts that were put to the person.
    #[serde(skip_serializing_if = "Vec::is_empty")]
    pub asked: Vec<Asked>,
    /// How many questions the interview has.
    pub questions: usize,
    /// How many you typed an answer to. **`--defaults` is none of them**, and it
    /// leaves a working guild that `armada doctor` reports as incomplete.
    pub answered: usize,
    /// Where the guild is, as a person writes it.
    pub guild_path: String,
}

/// *Do you already have a guild?* — the one question `armada init` asks before
/// anything else (`PLAN.md` §13.4).
#[derive(Debug, Clone, PartialEq, Serialize)]
pub struct GuildChoice {
    /// The question, in words.
    pub question: String,
    /// The three answers, in the order they are offered.
    pub options: Vec<String>,
    /// Which was taken, **one-based**, to match what was typed.
    pub chosen: usize,
}

/// One interview prompt, as it was put.
#[derive(Debug, Clone, PartialEq, Serialize)]
pub struct Asked {
    /// Its position, one-based.
    pub number: usize,
    /// Out of how many.
    pub of: usize,
    /// The question.
    pub prompt: String,
    /// What answer is wanted, and what the answer is for.
    pub purpose: String,
    /// The guild file this answer lands in.
    pub writes: String,
    /// What the default answer is, as the object of a sentence — *what import
    /// found*. The key that takes it is the render's, because it differs between
    /// a single-line prompt and a text area.
    pub keeps: String,
    /// Whether the answer is paragraphs, and therefore gets a text area rather
    /// than a line.
    pub prose: bool,
    /// **What pressing enter would keep, as it stands right now.**
    ///
    /// A default you cannot see is not a default you can accept with
    /// confidence: the first run of this interview offered *(enter to keep what
    /// import found)* over an empty line, and the person answering had no way to
    /// know whether import had found anything at all. Truncated to one line by
    /// the render — the whole of it is on disk, in the file named by `writes`.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub standing: Option<String>,
}

/// `armada doctor` — what this machine is missing.
#[derive(Debug, Clone, PartialEq, Serialize)]
pub struct DoctorData {
    /// One row per check, in the order they are run.
    pub results: Vec<Finding>,
    /// `NEEDS ATTENTION`, or absent when everything is `ok`.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub headline: Option<Headline>,
    /// The tally the summary line carries — `4 ok`, `1 missing`, `2 warnings`.
    pub tally: Vec<String>,
}

impl DoctorData {
    /// The summary line's counts, derived from the rows.
    ///
    /// **Derived rather than carried, so it cannot disagree with the table
    /// above it.** The reviewed drawing counted seven rows in a table of six
    /// (`crates/helm/tests/render_golden.rs` records the correction), which is
    /// exactly what a hand-written tally does eventually.
    ///
    /// Three buckets, and `missing` is its own rather than folded into the
    /// warnings: it is the one that fails the command, so it is the one a
    /// reader has to be able to count without re-reading the table.
    pub fn tally(results: &[Finding]) -> Vec<String> {
        let count =
            |predicate: fn(&Finding) -> bool| results.iter().filter(|r| predicate(r)).count();
        let mut out = vec![format!("{} ok", count(|r| r.status == Health::Ok))];
        let missing = count(|r| r.status.is_failure());
        if missing > 0 {
            out.push(format!("{missing} missing"));
        }
        let warnings = count(|r| r.status.is_warning());
        if warnings > 0 {
            out.push(format!(
                "{warnings} warning{}",
                if warnings == 1 { "" } else { "s" }
            ));
        }
        out
    }

    /// The verdict the counts imply.
    ///
    /// **A warning alone does not fail** (`docs/commands/doctor.md`), so
    /// `doctor` stays safe to run in a shell prompt — which it would not be if
    /// a guild three commits behind exited non-zero every time.
    pub fn verdict(results: &[Finding]) -> Status {
        if results.iter().any(|r| r.status.is_failure()) {
            Status::Failed
        } else if results.iter().any(|r| r.status.is_warning()) {
            Status::Partial
        } else {
            Status::Ok
        }
    }

    /// `NEEDS ATTENTION`, or nothing at all when every check is `ok`.
    pub fn headline(results: &[Finding]) -> Option<Headline> {
        results
            .iter()
            .any(|r| r.status != Health::Ok)
            .then_some(Headline::NeedsAttention)
    }
}

/// One `doctor` check.
///
/// **`remedy` is the point of the command.** `docs/commands/doctor.md`: a check
/// that reports a problem without the command that fixes it sends the reader to
/// the documentation, which is most of what `doctor` exists to save.
///
/// # A problem without a remedy is not representable
///
/// The rule used to be a sentence in this comment, and three checks did not
/// follow it — `missing layout` and both `partial guild` rows reached a real
/// reader with nothing to act on. So it is a type now. The struct is
/// `#[non_exhaustive]`, which means no crate but this one may write a `Finding {
/// .. }` literal; the only ways in are [`Finding::settled`], which cannot name a
/// problem, and [`Finding::needs`], whose remedy is a `String` and not an
/// `Option`. A check that reports a problem and offers no fix now fails to
/// compile rather than failing to help.
#[derive(Debug, Clone, PartialEq, Serialize)]
#[non_exhaustive]
pub struct Finding {
    /// What was checked: `git`, `docker`, `guild`, `~/.armada`.
    pub check: String,
    /// How it stands.
    pub status: Health,
    /// The specific delta rather than a verdict — `3 commits behind origin`,
    /// not `out of date`.
    pub detail: String,
    /// The exact command that would fix it, or the sentence that says how.
    /// Present on every [`Problem`] and absent on everything else.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub remedy: Option<String>,
}

impl Finding {
    /// A check with nothing to do about it.
    pub fn settled(
        check: impl Into<String>,
        status: Settled,
        detail: impl Into<String>,
    ) -> Finding {
        Finding {
            check: check.into(),
            status: status.health(),
            detail: detail.into(),
            remedy: None,
        }
    }

    /// A check that found a problem, **and the fix for it**.
    ///
    /// `remedy` is a `String` rather than an `Option<String>` on purpose: this
    /// is the constructor that makes "every non-`ok` row carries a fix line"
    /// hold by construction. It is a command where one exists and a sentence
    /// where none does — *write `~/.armada/guild/voice.md` in your own
    /// words* is a fix; *out of date* is not.
    pub fn needs(
        check: impl Into<String>,
        status: Problem,
        detail: impl Into<String>,
        remedy: impl Into<String>,
    ) -> Finding {
        Finding {
            check: check.into(),
            status: status.health(),
            detail: detail.into(),
            remedy: Some(remedy.into()),
        }
    }
}

/// The half of [`Health`] that means nothing needs doing.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Settled {
    /// Present and current — `doctor`'s word.
    Ok,
    /// Present, with the version that was found — `armada init`'s word.
    Found,
    /// Armada made it, just now.
    Created,
}

impl Settled {
    /// The wire spelling.
    pub const fn health(self) -> Health {
        match self {
            Settled::Ok => Health::Ok,
            Settled::Found => Health::Found,
            Settled::Created => Health::Created,
        }
    }
}

/// The half of [`Health`] that means something does.
///
/// **Split from [`Settled`] so that a remedy can be required for one and
/// forbidden for the other**, which is the whole mechanism behind the rule in
/// [`Finding`]'s own documentation. A single enum could only ever ask nicely.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Problem {
    /// Not there at all.
    Missing,
    /// There, and behind what it should be.
    Stale,
    /// There, and only half set up.
    Partial,
    /// Could not be checked, because the network was not reachable.
    Offline,
}

impl Problem {
    /// The wire spelling.
    pub const fn health(self) -> Health {
        match self {
            Problem::Missing => Health::Missing,
            Problem::Stale => Health::Stale,
            Problem::Partial => Health::Partial,
            Problem::Offline => Health::Offline,
        }
    }

    /// Every one of them, for the test that holds the two halves against
    /// [`Health`].
    pub const ALL: [Problem; 4] = [
        Problem::Missing,
        Problem::Stale,
        Problem::Partial,
        Problem::Offline,
    ];
}

/// How one thing on this machine stands — for `armada init`'s preflight and
/// for every `armada doctor` check.
///
/// **Lowercase in the payload and lowercase on the screen**, which is the same
/// rule [`Status`] follows in the other direction: one spelling, and it is the
/// JSON spelling. These are not terminal states of a run — nothing here ends
/// anything — so they are their own small enum rather than seven more members
/// of an enum whose exit-code mapping they would have no meaning in.
///
/// **One enum across the two verbs, because they are asking one question.**
/// `armada init` and `armada doctor` both report the condition of this machine;
/// `found` and `created` are the two answers only the setup pass can give, and
/// `stale` and `partial` the two only the drift pass can. Splitting them would
/// have meant deciding twice what `missing` looks like, and getting two
/// answers — which is what the mockups did.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
#[serde(rename_all = "SCREAMING_SNAKE_CASE")]
pub enum Health {
    /// Present and current — `doctor`'s word.
    Ok,
    /// Present, with the version that was found — `armada init`'s word for the
    /// same fact. Distinct from `ok` because a preflight is *looking* for
    /// something and a check is *confirming* it.
    Found,
    /// Armada made it, just now.
    Created,
    /// Not there at all.
    Missing,
    /// There, and behind what it should be.
    Stale,
    /// There, and only half set up — a fragment still as imported.
    Partial,
    /// Could not be checked, because the network was not reachable. **Not a
    /// failure**: `doctor` degrades to this rather than failing, so it stays
    /// safe to run in a shell prompt.
    Offline,
}

impl Health {
    /// The word, in both audiences.
    pub const fn word(self) -> &'static str {
        match self {
            Health::Ok => "OK",
            Health::Found => "FOUND",
            Health::Created => "CREATED",
            Health::Missing => "MISSING",
            Health::Stale => "STALE",
            Health::Partial => "PARTIAL",
            Health::Offline => "OFFLINE",
        }
    }

    /// Whether this state fails the command. **`ok` and `offline` do not, and
    /// nor does a warning** — `doctor` is safe to run in a shell prompt, which
    /// it would not be if every drifted guild exited non-zero.
    pub const fn is_failure(self) -> bool {
        matches!(self, Health::Missing)
    }

    /// Whether this state is a warning: real, and not a failure.
    pub const fn is_warning(self) -> bool {
        matches!(self, Health::Stale | Health::Partial | Health::Offline)
    }

    /// Whether this state asks the reader to do something.
    ///
    /// The two halves — [`Settled`] and [`Problem`] — partition this enum, and
    /// the test below holds them against it so a state added to one is not
    /// quietly absent from both.
    pub const fn needs_action(self) -> bool {
        self.is_failure() || self.is_warning()
    }
}

/// `armada guild pull` and `armada guild push`.
#[derive(Debug, Clone, PartialEq, Serialize)]
pub struct GuildSyncData {
    /// The remote, or absent when sync is off.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub remote: Option<String>,
    /// Commits this machine has that the remote does not.
    pub ahead: usize,
    /// Commits the remote has that this machine does not.
    pub behind: usize,
    /// One row per area of the guild the change set touches.
    pub results: Vec<SyncItem>,
    /// **Whether anything was written.** `false` on a divergence, which is the
    /// whole of `guild/pull.md`'s exit-code contract: *diverged, and nothing
    /// changed*. A reader must be able to tell "here is what is waiting" from
    /// "here is what landed", and the rows alone cannot say it.
    pub applied: bool,
    /// `NEEDS ATTENTION`, when a conflict or a divergence needs a person.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub headline: Option<Headline>,
    /// What re-projecting the pulled guild onto Claude Code's load path did.
    ///
    /// **Absent when nothing was projected**, which is the divergence case —
    /// nothing was applied, so there is nothing new to project. A pulled guild
    /// that has not been projected is a guild that has not taken effect, and the
    /// gap between the two is a confusing hour (`guild/pull.md`).
    #[serde(skip_serializing_if = "Option::is_none")]
    pub projected: Option<Projection>,
}

/// What a projection did — `armada guild project`, and the step `guild init`
/// and `guild pull` both end on.
///
/// The guild's mechanical half, written into the directories Claude Code reads,
/// tracked by a manifest of what was placed and a hash of each file
/// ([`PLAN.md`](../../../docs/PLAN.md) §13.2).
#[derive(Debug, Clone, PartialEq, Serialize)]
pub struct Projection {
    /// Where it was written, as a person writes it — `~/.claude/`.
    pub at: String,
    /// One row per area, in the order the `STATUS` column reads.
    pub results: Vec<SyncItem>,
    /// The summary line's facts: `2 placed`, `1 left as yours`.
    pub facts: Vec<String>,
    /// **How many files were left exactly as they were because you had edited
    /// them.** The count that earns the manifest: without it, a projection that
    /// declined to overwrite your work would look identical to one that had
    /// nothing to do.
    pub kept: usize,
    /// `NEEDS ATTENTION`, when something was left as yours.
    ///
    /// **Absent when this projection is carried by another verb.** `guild init`
    /// and `guild pull` draw their own summary line and speak for themselves;
    /// two headlines on one run is one too many.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub headline: Option<Headline>,
}

/// One area of the guild, and what the change set does to it.
#[derive(Debug, Clone, PartialEq, Serialize)]
pub struct SyncItem {
    /// What happened to it.
    pub status: Sync,
    /// The area — `skills`, `hooks`, `workflows` — or a single file's name.
    pub item: String,
    /// Which ones, or how many.
    pub detail: String,
}

/// What a sync does to one area.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
#[serde(rename_all = "SCREAMING_SNAKE_CASE")]
pub enum Sync {
    /// The remote has it and this machine does not.
    Added,
    /// Both have it, and they differ.
    Changed,
    /// This machine has it and the remote does not.
    Removed,
    /// **Edited here and on origin.** Reported, never resolved automatically —
    /// two machines' guilds merged without asking is how you end up with a hook
    /// you did not write (`guild/push.md`).
    Conflict,
    /// Identical on both sides.
    Unchanged,
}

impl Sync {
    /// The word, in both audiences.
    pub const fn word(self) -> &'static str {
        match self {
            Sync::Added => "ADDED",
            Sync::Changed => "CHANGED",
            Sync::Removed => "REMOVED",
            Sync::Conflict => "CONFLICT",
            Sync::Unchanged => "UNCHANGED",
        }
    }
}

/// `armada guild init`.
#[derive(Debug, Clone, PartialEq, Serialize)]
pub struct GuildInitData {
    /// Where the guild is.
    pub guild_path: String,
    /// What import adopted.
    pub imported: Vec<String>,
    /// **Every credential-shaped value the importer refused, by key.** Never a
    /// value: see `armada-guild`'s `secrets` module for why the type carrying
    /// these has nowhere to put one.
    #[serde(skip_serializing_if = "Vec::is_empty")]
    pub withheld: Vec<String>,
    /// The files written — the three fragments and the starters.
    pub wrote: Vec<String>,
    /// What a pre-namespace `machine.yml` had moved into its module's section,
    /// when this run migrated one (`PLAN.md` §4.3.1).
    ///
    /// **Absent is the ordinary case**, and it means the file was already in the
    /// current layout or there was none. Present, it names the keys that moved:
    /// this file is hand-edited, and a count does not tell anyone which keys to
    /// go and look at.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub migrated: Option<String>,
    /// The sync remote, if one was named.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub remote: Option<String>,
    /// How many questions the interview has.
    pub questions: usize,
    /// How many you typed an answer to. **The rest were `kept`, not skipped** —
    /// pressing enter is what the hint instructs and it accepts a value, so the
    /// count that used to be called `skipped` told someone who followed the
    /// instructions that he had done nothing.
    pub answered: usize,
    /// What projecting the new guild onto Claude Code's load path did.
    ///
    /// **A `guild init` that stopped before this would leave a guild nothing
    /// reads** — the failure `PHASES.md` §8.4 records, where a skill Armada
    /// ships and `guild init` installs answers `Unknown command` in the session
    /// Armada hands you to.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub projected: Option<Projection>,
}

/// `armada guild export` and `armada guild import`.
#[derive(Debug, Clone, PartialEq, Serialize)]
pub struct GuildBundleData {
    /// The bundle's path.
    pub path: String,
    /// Its size, when it was just written. Absent on import, where the reader
    /// is being told what came out rather than what went in.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub bytes: Option<u64>,
    /// What is in it.
    pub contents: Vec<String>,
    /// Whether `machine.yml` travelled. **Reported either way**, because "the
    /// file that never syncs did not sync" is the fact the flag exists to make
    /// checkable.
    pub secrets: bool,
    /// What was deliberately not taken from the bundle.
    #[serde(skip_serializing_if = "Vec::is_empty")]
    pub skipped: Vec<String>,
    /// Files `--merge` left alone because this machine's copy differs.
    #[serde(skip_serializing_if = "Vec::is_empty")]
    pub conflicts: Vec<String>,
}

// ------------------------------------------------------------------ M3: Fleet
//
// **Fleet's bodies live here for the same reason Manifest's and Guild's do**:
// `data` is the per-verb half of one envelope, and a second place to define one
// is a second place for a key to be renamed without a golden snapshot noticing
// (`ARCHITECTURE.md` §1.6).
//
// **Nothing here is a `Status`.** A Job's state and a step's verdict are Fleet's
// own enums (PLAN.md §14.3), spelled SCREAMING in the payload exactly as they
// are on the screen, and the envelope's top-level `status` stays Manifest's —
// which is what keeps `exit = f(error.class)` true for a verb that reports
// `BLOCKED` Jobs and exits 0.

/// `armada fleet spawn` — the Job that now exists.
///
/// **Every field is minted before the Drone starts.** The uuid, the worktree,
/// the branch and the budget are all facts about the Job, and a `spawn` that
/// reported them only on success would leave a Job on disk that its own output
/// never named (PLAN.md §14.1).
#[derive(Debug, Clone, PartialEq, Serialize)]
pub struct SpawnData {
    /// The Claude Code session id, minted before anything ran.
    pub uuid: String,
    /// The handle a person types.
    pub name: String,
    /// Which workflow was chosen.
    pub workflow: String,
    /// How sure classification was. **Absent for an override**, because "you
    /// said so" and "the model was certain" are different facts.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub confidence: Option<f64>,
    /// Where the worktree is, as a person writes it.
    pub worktree: String,
    /// The branch it is on.
    pub branch: String,
    /// The span `armada manifest init` claimed, when the repository has one.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub port_block: Option<PortBlock>,
    /// The ceilings this Job runs under.
    pub budget: crate::fleet::workflow::Budget,
    /// The step it starts on.
    pub step: String,
    /// What it is doing now.
    pub state: crate::fleet::JobState,
    /// How long classification took. `None` when a person named the workflow
    /// and no call was made.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub classify_ms: Option<u64>,
    /// How long the worktree and `armada manifest init` took together.
    pub prepare_ms: u64,
    /// The Drone's process group, or `None` under `--dry-run`.
    ///
    /// **`spawn` returns while the Drone is still working**, which is the whole
    /// point of Fleet: five Jobs at once with one thing to watch. So there is no
    /// spend to report here — the transcript is the ledger and `armada fleet ls`
    /// reads it. What this carries instead is the handle, so a caller can see
    /// that something really was started.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub pgid: Option<i32>,
}

/// `armada fleet ls` — what is running, how long, what it has spent, and who
/// needs you.
///
/// **Every column comes from data Claude Code already emits** (PHASES.md §9.1
/// F2). Fleet builds no accounting layer and estimates nothing.
#[derive(Debug, Clone, Default, PartialEq, Serialize)]
pub struct FleetLsData {
    /// One row per Job, oldest first.
    pub results: Vec<JobRow>,
    /// How many are waiting on you.
    pub needs_you: usize,
    /// What the listed Jobs have cost between them.
    pub spent_usd: f64,
}

/// One Job, as `ls` reports it.
#[derive(Debug, Clone, PartialEq, Serialize)]
pub struct JobRow {
    /// The session id.
    pub uuid: String,
    /// The handle.
    pub name: String,
    /// Which workflow.
    pub workflow: String,
    /// What it is doing.
    pub state: crate::fleet::JobState,
    /// The step, and what it is waiting on — the one thing a state word cannot
    /// say. Empty when there is nothing to add.
    #[serde(skip_serializing_if = "String::is_empty")]
    pub detail: String,
    /// How long it has been alive, in seconds.
    pub runtime_s: u64,
    /// Dollars, summed off the turns' `total_cost_usd`.
    pub cost_usd: f64,
    /// Every kind of token, summed off the turns' `usage`.
    pub tokens: u64,
    /// Turns, summed off the turns' `num_turns`.
    pub turns: u32,
    /// What is left of each ceiling.
    pub budget_remaining: crate::fleet::job::Remaining,
    /// Whether it is waiting on you.
    pub needs_attention: bool,
}

/// `armada fleet board` — the two facts needed to enter a Job.
///
/// **It does not attach, and it never will.** Boarding hands you the
/// conversation to drive yourself; streaming a live Drone's output is the pty
/// work withdrawn in PHASES.md §9.1 F1, and Armada owns no terminal.
#[derive(Debug, Clone, PartialEq, Serialize)]
pub struct BoardData {
    /// The Job's handle.
    pub job: String,
    /// Where the worktree is.
    pub worktree: String,
    /// The session id.
    pub uuid: String,
    /// The branch.
    pub branch: String,
    /// The command, assembled — `claude --resume <uuid>`.
    pub command: String,
}

/// `armada fleet kill` — what each Job released, and what became of its tree.
#[derive(Debug, Clone, Default, PartialEq, Serialize)]
pub struct KillData {
    /// One entry per Job killed.
    pub results: Vec<Killed>,
}

/// One Job, ended.
#[derive(Debug, Clone, PartialEq, Serialize)]
pub struct Killed {
    /// The handle.
    pub job: String,
    /// The session id.
    pub uuid: String,
    /// What `armada manifest clean` reclaimed.
    pub released: Released,
    /// The span that went back, when there was one.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub port_block: Option<PortBlock>,
    /// What became of the directory.
    pub worktree: Disposition,
    /// Where it was.
    pub worktree_path: String,
    /// What became of the branch.
    pub branch: Disposition,
    /// Which branch.
    pub branch_name: String,
    /// What would not release. **The Job is still marked ended** — a `kill` that
    /// bailed out here would leave the worktree as well, and ownership is
    /// recorded machine-globally so `armada manifest clean --all` reclaims the
    /// remainder (`commands/fleet/kill.md`).
    #[serde(skip_serializing_if = "Option::is_none")]
    pub error: Option<ArmadaError>,
}

/// What became of a Job's directory or branch.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
#[serde(rename_all = "SCREAMING_SNAKE_CASE")]
pub enum Disposition {
    /// Armada removed it.
    Removed,
    /// Armada left it alone, because you asked.
    Kept,
    /// It was already gone. **Not a failure**: a Job whose worktree somebody
    /// deleted by hand is exactly the Job the durable record exists for.
    Gone,
}

impl Disposition {
    /// The word, in both audiences.
    pub const fn word(self) -> &'static str {
        match self {
            Disposition::Removed => "REMOVED",
            Disposition::Kept => "KEPT",
            Disposition::Gone => "GONE",
        }
    }
}

/// `armada fleet inbox` — what the fleet needs from you.
#[derive(Debug, Clone, Default, PartialEq, Serialize)]
pub struct InboxData {
    /// One row per entry, oldest first.
    pub results: Vec<InboxRow>,
    /// How many are still open.
    pub open: usize,
}

/// One inbox entry.
///
/// **The id is the point** (PLAN.md §15.3.1): an item you cannot name is an item
/// you cannot acknowledge one row at a time, which is what turns a list of
/// things to do into a list nobody trusts after the first one.
#[derive(Debug, Clone, PartialEq, Serialize)]
pub struct InboxRow {
    /// The entry's own id.
    pub uuid: String,
    /// The Job that raised it.
    pub job: String,
    /// Why.
    pub kind: String,
    /// When, RFC 3339.
    pub raised_at: String,
    /// How long ago, in seconds.
    pub waiting_s: u64,
    /// What it wants to tell you.
    pub body: String,
    /// Your answer, once there is one.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub answered: Option<String>,
}

/// `armada fleet answer` — the entry you closed, and what the Job did next.
#[derive(Debug, Clone, PartialEq, Serialize)]
pub struct AnswerData {
    /// The handle.
    pub job: String,
    /// The session id.
    pub uuid: String,
    /// The entry that was answered.
    pub entry: String,
    /// What you said.
    pub answer: String,
    /// What it is doing now.
    pub state: crate::fleet::JobState,
    /// **Not reset by an answer** (`commands/fleet/answer.md`). An answer is a
    /// continuation rather than a new run, and resetting the ceiling here would
    /// make budgets unenforceable for any Job that asks a question.
    pub budget_remaining: crate::fleet::job::Remaining,
    /// The resumed Drone's process group.
    ///
    /// **An answer starts a turn; it does not wait for one.** A resumed Drone is
    /// detached exactly as a fresh one is, so what is reported is that it was
    /// started rather than what it produced.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub pgid: Option<i32>,
}

// ------------------------------------------------------------------- M3:
// the toolbelt, and the three things a Drone may say

/// `armada mcp serve` — what was served, and until when.
///
/// **The envelope of a clean shutdown**, which is the only terminal state a
/// server that lives as long as its session has. The transport failing is
/// `environment` and exits `6`; everything else here is `OK`
/// (`commands/helm/mcp.md`).
#[derive(Debug, Clone, PartialEq, Serialize)]
pub struct McpData {
    /// How it was served. `stdio` is the only transport, and the default.
    pub transport: String,
    /// Which belt was offered — `helm` or `drone`.
    ///
    /// **Reported rather than assumed**, because it is decided by the
    /// environment: a process with `ARMADA_JOB` set is a Drone's child and gets
    /// the smaller belt. A caller reading this is reading the answer to "was I
    /// allowed to spawn", which is otherwise only discoverable by trying.
    pub toolbelt: String,
    /// The tools that belt carried, in the order the documentation lists them.
    pub tools: Vec<String>,
}

/// `fleet.probe` — one Job's transcript, summarised.
///
/// **Read-only, and it never resumes the Drone** (PLAN.md §15.2). Messaging a
/// busy agent to ask how it is going costs you the thing you were measuring, so
/// the summary is produced by a second, cheap model reading the transcript the
/// Drone has already written.
#[derive(Debug, Clone, PartialEq, Serialize)]
pub struct ProbeData {
    /// The handle.
    pub job: String,
    /// The session id.
    pub uuid: String,
    /// What the Job is doing, as the record says — not as the summary guesses.
    pub state: crate::fleet::JobState,
    /// The step it is on.
    pub step: String,
    /// The summary itself, in prose.
    ///
    /// **This is what the orchestrator reads instead of the transcript**
    /// (PLAN.md §15.2). A Helm that reads raw transcripts fills its window in
    /// three days and starts forgetting the fleet.
    pub summary: String,
    /// How many transcript events the summary was drawn from. `0` means the
    /// Drone has written nothing yet, which is the ordinary state a moment
    /// after `spawn` returns.
    pub events: usize,
    /// Which model produced it, so a reader knows what the summary cost.
    pub model: String,
}

/// `fleet.report` — progress, appended to the Job's own record.
#[derive(Debug, Clone, PartialEq, Serialize)]
pub struct ReportData {
    /// The handle.
    pub job: String,
    /// The step it was reported against.
    pub step: String,
    /// How many notes the record now holds.
    pub notes: usize,
}

/// `fleet.ask_human` — the entry raised, and the answer if one came.
///
/// **The id is the whole point** (PLAN.md §15.3.1). An item a person cannot name
/// is an item they cannot acknowledge one row at a time.
#[derive(Debug, Clone, PartialEq, Serialize)]
pub struct AskData {
    /// The handle.
    pub job: String,
    /// The entry's own id, which is what `armada fleet answer` closes.
    pub entry: String,
    /// The question, as it was asked.
    pub question: String,
    /// The answer, once a person has given one. `None` means it is still open.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub answered: Option<String>,
}

/// `fleet.verdict` — how a step ended (PLAN.md §14.3).
///
/// **The one added field on the §3.1 envelope is `verdict`**, and it rides in
/// `data` rather than beside `status` for the reason the envelope is nested at
/// all: a Job's verdict and a workspace's `Status` are different enums, and a
/// future field called `status` in a body must not collide with the wrapper's.
#[derive(Debug, Clone, PartialEq, Serialize)]
pub struct VerdictData {
    /// The handle.
    pub job: String,
    /// The step this verdict is about.
    pub step: String,
    /// How it ended.
    pub verdict: crate::fleet::Verdict,
    /// What the verdict rests on.
    ///
    /// **A verdict is only `PASS` if it carries evidence an external command
    /// produced** (PLAN.md §14.3). An agent asserting that the tests pass is not
    /// evidence; an `armada manifest check` exit code is — which is why the
    /// verb refuses a `PASS` with an empty list rather than recording one.
    #[serde(skip_serializing_if = "Vec::is_empty")]
    pub evidence: Vec<Evidence>,
    /// How many times this step has now been attempted.
    pub attempts: u32,
    /// What the Job is doing after it.
    pub state: crate::fleet::JobState,
}

/// One thing a verdict rests on.
#[derive(Debug, Clone, PartialEq, Serialize, serde::Deserialize)]
pub struct Evidence {
    /// What kind of thing produced it — `check`, `command`.
    pub kind: String,
    /// Which one. A check id, a command name.
    pub scope: String,
    /// What it exited with. **The number, not a summary of it**: an exit code is
    /// the fact, and a sentence about the fact is the thing §14.3 refuses.
    pub exit: i32,
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::error::ErrClass;

    /// **Every state that asks the reader to do something carries the fix.**
    ///
    /// Asserted over [`Problem::ALL`] rather than over the checks that happen to
    /// exist, because the point is the type: there is no way to reach one of
    /// these four without a remedy, so this holds for checks nobody has written
    /// yet.
    #[test]
    fn a_finding_that_reports_a_problem_cannot_omit_the_fix() {
        for problem in Problem::ALL {
            let finding = Finding::needs("guild", problem, "something is wrong", "do this");
            assert!(finding.status.needs_action(), "{problem:?}");
            assert_eq!(finding.remedy.as_deref(), Some("do this"));
        }
    }

    /// The two halves partition [`Health`]. A state added to one and not the
    /// other would be either unreachable or reachable without a fix, and this is
    /// what makes that a failing test rather than a discovery.
    #[test]
    fn the_two_halves_of_health_partition_it() {
        for problem in Problem::ALL {
            assert!(problem.health().needs_action(), "{problem:?}");
        }
        for settled in [Settled::Ok, Settled::Found, Settled::Created] {
            assert!(!settled.health().needs_action(), "{settled:?}");
            assert!(Finding::settled("git", settled, "2.51.0").remedy.is_none());
        }
        // Seven states, and every one of them is in exactly one half.
        let covered = Problem::ALL.len() + 3;
        for health in [
            Health::Ok,
            Health::Found,
            Health::Created,
            Health::Missing,
            Health::Stale,
            Health::Partial,
            Health::Offline,
        ] {
            assert!(
                Problem::ALL.iter().any(|p| p.health() == health)
                    || [Settled::Ok, Settled::Found, Settled::Created]
                        .iter()
                        .any(|s| s.health() == health),
                "{health:?} is in neither half"
            );
        }
        assert_eq!(covered, 7);
    }

    fn failed(id: &str, class: ErrClass) -> ResultRow {
        let mut row = ResultRow::new(id, Status::Failed);
        row.error = Some(ArmadaError {
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
        // armada_bug > environment > bad_config > bad_invocation > timeout >
        // aborted > tool_failed
        let order = [
            ErrClass::ToolFailed,
            ErrClass::Aborted,
            ErrClass::Timeout,
            ErrClass::BadInvocation,
            ErrClass::BadConfig,
            ErrClass::Environment,
            ErrClass::ArmadaBug,
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
        ] {
            let rows = [ResultRow::new("api:test", status)];
            let error = aggregate(&rows, "checks").expect("a terminal failure is a failure");
            assert_eq!(error.class, class, "{status:?} scored the wrong class");
        }
    }

    /// **The two states that mean *no verdict was reached* imply nothing**, and
    /// `implied_class`'s own doc says why. The inference exists to stop a verb
    /// reporting success over a `FAILED` row; a row that was never attempted is
    /// the opposite case.
    #[test]
    fn a_row_that_reached_no_verdict_of_its_own_scores_no_class() {
        for status in [Status::Aborted, Status::Dead] {
            let rows = [ResultRow::new("api:test", status)];
            assert!(
                aggregate(&rows, "checks").is_none(),
                "{status:?} inferred a class it does not establish"
            );
        }
    }

    /// **The outcome PLAN.md §4.1 requires, and the assertion this replaces.**
    ///
    /// Phase 2 asserted the opposite here — exit 5, with `"not 1"` written into
    /// it — on the reasoning that §3.1's own example payload contains a bare
    /// `ABORTED` row, so the shape is specified. The shape is; the *inference*
    /// was Armada's own, and §4.1 forbids its result: a run whose only real
    /// failure is a deterministic test failure must not hand a merge gate the
    /// retryable class.
    #[test]
    fn a_bare_aborted_row_does_not_outrank_the_failure_that_caused_it() {
        let rows = [
            failed("core:build", ErrClass::ToolFailed),
            ResultRow::new("ui:types", Status::Aborted),
        ];
        let error = aggregate(&rows, "checks").unwrap();
        assert_eq!(error.class, ErrClass::ToolFailed);
        assert_eq!(error.class.exit_code(), 1, "not 5 — this is deterministic");
        assert_eq!(error.r#where, "core:build", "the check that actually broke");
    }

    /// **The precedence claim underneath it is untouched.** A row that genuinely
    /// carries `aborted` — a claim that hit the acquisition ceiling — still
    /// outranks a tool failure, because it attaches a real `error` object.
    /// What narrowed is only what is inferred in the absence of one.
    #[test]
    fn an_explicit_aborted_error_still_outranks_a_tool_failure() {
        let rows = [
            failed("api:lint", ErrClass::ToolFailed),
            failed("web:e2e", ErrClass::Aborted),
        ];
        let error = aggregate(&rows, "checks").unwrap();
        assert_eq!(error.class, ErrClass::Aborted);
        assert_eq!(error.class.exit_code(), 5);
        assert_eq!(error.r#where, "web:e2e");
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
        row.error = Some(ArmadaError::bad_config(
            crate::error::ConfigWhere::Path {
                file: "armada.yml".into(),
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
            ArmadaError {
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

    // --------------------------------------------------------------- M2

    /// **The human spelling is the JSON spelling**, which is the rule that lets
    /// `NEEDS ATTENTION` be an uppercase word without breaking `render.rs`'s
    /// "uppercase means the envelope's own". A reader can grep for anything
    /// they saw on the screen.
    #[test]
    fn the_headline_is_spelled_in_the_payload_exactly_as_it_is_printed() {
        assert_eq!(
            serde_json::to_string(&Headline::NeedsAttention).unwrap(),
            "\"NEEDS ATTENTION\""
        );
        assert_eq!(Headline::NeedsAttention.to_string(), "NEEDS ATTENTION");
    }

    /// The same rule, on `doctor`'s own words: **SCREAMING on the screen and
    /// SCREAMING in the payload.**
    ///
    /// These serialised lowercase until a reader pointed out that a status
    /// column holding `PASS` next to `ok` reads as two kinds of thing. §3.1 has
    /// always said one spelling in both audiences and SCREAMING in both; this
    /// enum was the exception, and the exception is what changed.
    #[test]
    fn a_doctor_finding_is_spelled_the_same_in_both_audiences() {
        for (health, word) in [
            (Health::Ok, "OK"),
            (Health::Found, "FOUND"),
            (Health::Created, "CREATED"),
            (Health::Missing, "MISSING"),
            (Health::Stale, "STALE"),
            (Health::Partial, "PARTIAL"),
            (Health::Offline, "OFFLINE"),
        ] {
            assert_eq!(health.word(), word);
            assert_eq!(
                serde_json::to_string(&health).unwrap(),
                format!("\"{word}\"")
            );
        }
    }

    /// **`warn` alone does not fail**, so `doctor` is safe to run in a shell
    /// prompt (`docs/commands/doctor.md`). A drifted guild that exited non-zero
    /// would make the command unusable exactly where it is most useful.
    #[test]
    fn a_warning_does_not_fail_doctor_and_a_missing_thing_does() {
        assert!(Health::Missing.is_failure());
        for health in [Health::Stale, Health::Partial, Health::Offline] {
            assert!(!health.is_failure(), "{health:?} failed the command");
            assert!(health.is_warning());
        }
        for health in [Health::Ok, Health::Found, Health::Created] {
            assert!(!health.is_failure(), "{health:?}");
            assert!(!health.is_warning(), "{health:?}");
        }
    }

    /// The `remedy` is what earns `doctor` its existence, so a `Finding` that
    /// has one carries it in the payload under that name.
    #[test]
    fn a_finding_carries_the_command_that_fixes_it() {
        let finding = Finding {
            check: "guild".to_string(),
            status: Health::Stale,
            detail: "3 commits behind origin".to_string(),
            remedy: Some("armada guild pull".to_string()),
        };
        let json = serde_json::to_string(&finding).unwrap();
        assert_eq!(
            json,
            r#"{"check":"guild","status":"STALE","detail":"3 commits behind origin","remedy":"armada guild pull"}"#
        );

        // A passing check has nothing to fix, and the key is absent rather
        // than null — a field that is usually empty teaches agents to ignore it.
        let ok = Finding {
            check: "git".to_string(),
            status: Health::Ok,
            detail: "2.51.0".to_string(),
            remedy: None,
        };
        assert!(!serde_json::to_string(&ok).unwrap().contains("remedy"));
    }

    /// **A reader must be able to tell "here is what is waiting" from "here is
    /// what landed"**, and the rows alone cannot say it. `applied` is the whole
    /// of `guild/pull.md`'s *diverged, and nothing changed*.
    #[test]
    fn a_diverged_pull_says_in_the_payload_that_nothing_was_applied() {
        let data = GuildSyncData {
            remote: Some("git@example.com:me/guild.git".to_string()),
            ahead: 2,
            behind: 3,
            results: vec![SyncItem {
                status: Sync::Conflict,
                item: "voice.md".to_string(),
                detail: "edited here and on origin".to_string(),
            }],
            applied: false,
            headline: Some(Headline::NeedsAttention),
            projected: None,
        };
        let json = serde_json::to_string(&data).unwrap();
        assert!(json.contains(r#""applied":false"#), "{json}");
        assert!(json.contains(r#""status":"CONFLICT""#), "{json}");
        assert!(json.contains(r#""headline":"NEEDS ATTENTION""#), "{json}");
    }

    #[test]
    fn every_sync_word_is_the_word_the_layout_prints() {
        for (sync, word) in [
            (Sync::Added, "ADDED"),
            (Sync::Changed, "CHANGED"),
            (Sync::Removed, "REMOVED"),
            (Sync::Conflict, "CONFLICT"),
            (Sync::Unchanged, "UNCHANGED"),
        ] {
            assert_eq!(sync.word(), word);
            assert_eq!(serde_json::to_string(&sync).unwrap(), format!("\"{word}\""));
        }
    }

    /// **`armada init`'s payload is the transcript the terminal got.** The one
    /// verb that interviews you is the one whose `--json` has to carry what was
    /// asked, or the two audiences are looking at different runs.
    #[test]
    fn machine_init_carries_the_questions_it_asked() {
        let data = MachineInitData {
            results: vec![Finding {
                check: "git".to_string(),
                status: Health::Found,
                detail: "2.51.0".to_string(),
                remedy: None,
            }],
            guild: Some(GuildChoice {
                question: "Do you already have a guild?".to_string(),
                options: vec![
                    "pull from a remote".to_string(),
                    "import a bundle".to_string(),
                    "build one now".to_string(),
                ],
                chosen: 3,
            }),
            imported: vec!["imported from ~/.claude/".to_string()],
            asked: vec![Asked {
                number: 1,
                of: 5,
                prompt: "How should agents write to you?".to_string(),
                purpose: "Tone, length, and what to lead with.".to_string(),
                writes: "voice.md".to_string(),
                keeps: "what import found".to_string(),
                prose: true,
                standing: Some("Lead with the answer.".to_string()),
            }],
            questions: 5,
            answered: 1,
            guild_path: "~/.armada/guild".to_string(),
        };
        let json = serde_json::to_string(&data).unwrap();
        assert!(json.contains(r#""chosen":3"#), "{json}");
        assert!(json.contains("How should agents write to you?"), "{json}");
        assert!(json.contains(r#""answered":1"#), "{json}");
        // **What pressing enter would have kept is in the payload too.** The
        // transcript on the terminal showed it; a `--json` account of the same
        // run that did not would be a different account.
        assert!(
            json.contains(r#""standing":"Lead with the answer.""#),
            "{json}"
        );
    }

    /// The withheld list is keys, and the type it is built from has nowhere to
    /// put a value — asserted here too, because this is the payload that leaves
    /// the process.
    #[test]
    fn guild_init_reports_withheld_keys_and_not_values() {
        let data = GuildInitData {
            guild_path: "~/.armada/guild".to_string(),
            imported: vec!["19 skills".to_string()],
            withheld: vec!["settings.json:env.GITHUB_TOKEN".to_string()],
            wrote: vec!["voice.md".to_string()],
            migrated: None,
            remote: None,
            questions: 5,
            answered: 0,
            projected: None,
        };
        let json = serde_json::to_string(&data).unwrap();
        assert!(json.contains("env.GITHUB_TOKEN"), "{json}");
        assert!(!json.contains("remote"), "sync off is absent, not null");
        assert!(
            !json.contains("migrated"),
            "the ordinary run migrated nothing, and says nothing: {json}"
        );
    }
}
