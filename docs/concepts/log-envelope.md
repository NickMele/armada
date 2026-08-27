# Log Envelope

**What it is:** The field contract every log line carries across Fleet, Bridge and Drone, so that one Job's activity can be assembled from three emitters. Fleet is the sole ID authority.

---

**Kind:** Policy.

Armada emits log lines from three independent emitters into sinks that already exist — per-Job `.armada/logs/<job-id>.jsonl`, machine-level `audit.jsonl`, and the WebSocket event stream. Nothing currently guarantees a line from Bridge can be joined to a line from Fleet or a Drone. This document is that guarantee.

**This document owns field names, their types, and which emitter is allowed to mint them.** It does not own sink paths, retention or redaction — those live in `../contracts/system-architecture.md` sections 7 and 8.

**It is a Policy rather than an Entity: it has no ID, no state and no lifecycle.** It is a rule applied at every emit site, and code branches on it only at the sink.

## The envelope

Identical key names across all three emitters. A key is either present with a value or absent. Never present and null.

| Field | Type | Present | Notes |
| --- | --- | --- | --- |
| `ts` | RFC3339 UTC, ms precision | Always |  |
| `level` | `trace` `debug` `info` `warn` `error` | Always | Lowercase |
| `component` | `fleet` `bridge-main` `bridge-ui` `drone` | Always | Emitter identity. Closed set |
| `run_id` | ULID | Always | The **emitting process's** instance, minted by that process at start |
| `msg` | String | Always | Never carries an interpolated ID |
| `job_id` | ULID | Once a Job exists | The correlation spine |
| `drone_id` | ULID | While a Drone executes | A retry is a second `drone_id` under one `job_id` |
| `step_id` | String | Inside a Workflow step | From the WorkflowDef, not generated |
| `workspace` | String | When the line concerns exactly one workspace | Single-valued. See the Convoy rule below |
| `target`, `span` | String | `fleet` only | Supplied by `tracing` |
| `fields` | Object | Optional | All structured data. Nothing structured belongs at the top level |

## Rules

**Fleet is the sole authority for `job_id` and `drone_id`.** Bridge and Drones never mint either; they echo what they were handed, which makes the join reliable rather than best-effort. Why: an id invented by something that does not own the record joins to nothing.

### `run_id` names the emitter

**`run_id` is the exception, because it names the emitter rather than a record.** Each process mints its own at start: Fleet's changes on every restart, which is what makes a restart visible in the log, and Bridge's and a Drone's identify theirs.

A Drone outlives a Fleet restart under `setsid`, and Bridge runs before it has reached Fleet at all, so a single Fleet-owned `run_id` could not describe either. Reading it is per emitter: a Fleet restart is Fleet's `run_id` changing, not any `run_id` changing.

### Each emitter is handed the spine

**IDs reach a Drone as environment variables** — `ARMADA_JOB_ID`, `ARMADA_DRONE_ID`, `ARMADA_STEP_ID` — set in `DroneSpawnConfig`. Hooks and transcripts then carry the spine without anything parsing a prompt.

**Bridge attaches the spine at the protocol layer, not at the call site.** Every Fleet-to-Bridge message envelope carries the IDs; a Bridge log line inherits them from the message being handled. Why: threading them by hand through TypeScript call sites is how they go missing.

**Fleet uses spans, not arguments.** `job`, `step` and `drone` spans mean `tracing` propagates the fields automatically, so no function signature grows an `id` parameter for logging's sake.

### Lexicographic sort is chronological

**ULIDs, not UUIDv4.** Why: lexicographic sort is chronological, so merging three emitters' lines into one ordered view costs a string sort.

### Nothing greps `msg`

**IDs are fields only.** Any query or `jq` filter targets a field; nothing greps `msg` text. This is the same discipline as `store` being the only crate that deserializes: the structured path is the only path.

## Convoy and `workspace`

**`workspace` is single-valued and omitted when the line is not scoped to one workspace.** A Convoy-spanning line carries `job_id` and no `workspace`. Why: an array would tax every query on a hot-path field forever, and most lines genuinely concern one workspace or none.

The full set a Job spans is recorded once, as a domain event at Job creation, and is already persisted in `job_manifests`. The absence of `workspace` on a line is itself the signal that the line is Job-scoped rather than workspace-scoped.

This leans on `job_manifests` being the record. If that table is ever dropped, the Convoy workspace set loses its home and this decision reopens.

## Relationship to the existing sinks

The envelope is the line shape. The sinks and their retention are already decided elsewhere and are not restated here.

| Sink | Carries the envelope | Owned by |
| --- | --- | --- |
| `<repo>/.armada/logs/<job-id>.jsonl` | Yes, in full | `../contracts/system-architecture.md` section 7 |
| `<repo>/.armada/transcripts/<drone-id>.jsonl` | No. A separate artifact, joined by `drone_id` | [Observe](observe.md) |
| `<repo>/.armada/checks/<job-id>/<step-id>.<n>.log` | No. What one Check printed, referenced from its `job_step_checks` row | `../contracts/system-architecture.md` section 7 |
| `~/Library/Application Support/Armada/audit.jsonl` | Yes, plus `actor` | `../contracts/system-architecture.md` section 7 |
| WebSocket event stream | The ID fields only. Events are typed, not log lines | `../contracts/system-architecture.md` section 6 |
| `job_events` in SQLite | The ID fields as columns | `../contracts/system-architecture.md` section 5 |

**`Redactor` runs after the envelope is assembled, not before.** It already scrubs three sinks. Nothing in the envelope is exempt, and `fields` is where a leaked credential would most plausibly land.

## A Drone's transcript is a separate artifact

**A transcript is not wrapped into the envelope line by line.** It is written to `<repo>/.armada/transcripts/<drone-id>.jsonl`, one row per line, and the Job log keeps its envelope unchanged.

Why: the transcript is far larger than everything else a Job emits and carries its own retention question, so wrapping it would make the Job log neither readable nor greppable — and a `jq` filter on `job_id` would return mostly turns.

**A retry is a second `drone_id` under one `job_id`, so a retry gets its own file** rather than interleaving with the first. Nothing appends to a transcript whose Drone is gone.

**The join is both a column and a line, and they answer different questions.** `assigned_drone` on the Job record names which Drone is on the Job — folded from the `drone.spawned` and `drone.exited` rows in `job_events`, so it is null again once the Drone is gone. The Job log line names the *file*, which the column does not: it carries `job_id`, `drone_id`, `step_id` and the transcript's path in `fields`, and it is what survives after the column has been cleared.

The row shape and what is in it belong to [Observe](observe.md). This document owns only the fact that the two files are separate and how they join.

## Where it surfaces

**The first surface is in the M1 — Dogfood mockups.**

**Bridge names a job's log on every job from dispatch** — running, failed and finished. Why: the job worth reading a log for is usually the one still running badly, and a failed job has already stopped, so its log is a post-mortem.

The treatment is a path in mono that copies on click, the line count and the error count beside it, and one secondary that opens the file.

- **No viewer, no level filter, no tail.** The Check output pane answers what the suite said; the log answers what Fleet, the drone and Bridge each did, joined on `job_id`. A reader is a later milestone's problem; knowing where the file is, is not.
- **UI copy says "log".** Never "Log Envelope" — the name is outside the sanctioned lexicon (see Open questions below), and a surface is not the place to settle it.
- **The `file-*` glyph family is reserved to evidence,** so the log row takes the plain page outline rather than a new glyph.

Component row: **Job log reference**, in the Components registry.

## Open questions

- **[log-envelope-name]** Is "Log Envelope" admitted into the sanctioned lexicon as a proper noun, or is this document renamed to something the lexicon already allows? "Log Envelope" is descriptive and currently outside it.
- **[log-envelope-bridge-ui-emitter]** Does `bridge-ui` emit log lines at all, or does it forward to `bridge-main`? A renderer writing to disk directly would be a second writer for one process class.
- **[log-envelope-error-count-source]** Are the error and warn counts shown on a job's log row computed per view, or carried on the job record?
- **[log-envelope-pruned-log-row]** What does a job's log row say once the log has been pruned? Per-job logs are pruned on terminal status after the retention grace period, so a finished job older than that has a path pointing at nothing.
