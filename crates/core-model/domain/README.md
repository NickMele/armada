# crates/core-model/domain

**Authored here.** These files are the authority on Armada's domain enums —
where a Job can be, where one step of it can be, which edges between those are
legal, why a Job stops and asks, and what a WorkflowDef declares. The design
workspace's databases are the record of the decision and the reasoning that
produced them, not the authority on the current set: a page there that
disagrees with a file here is stale, not right.

They live beside `core-model` rather than in `docs/` for the reason
`crates/config/settings.toml`, `packages/tokens` and `packages/icons` do. A set
that code reads is a data file beside the crate that reads it, with a check over
it. While these lived in a database nothing could compare them to an enum. As
files, a rule can — and the rule is the point.

| Path | What it is |
|---|---|
| `job-statuses.toml` | Where a Job is. The `status` column on `jobs`, and the outer half of the two-level machine |
| `step-states.toml` | Where one step of the frozen WorkflowDef is. The `state` column on `job_steps`, and the inner half |
| `job-transitions.toml` | Every legal edge of the status machine, what fires it, and the escalation trigger it belongs to |
| `job-fields.toml` | What a Job record holds — the `jobs` row, the `job_steps` row, and the three things stored elsewhere |
| `escalation-triggers.toml` | Why a Job stopped and a person is being asked |
| `enum-verbs.toml` | The word, the glyph and the colour each variant of the seven enums renders as |
| `workflowdef-fields.toml` | What a WorkflowDef declares: a step's gates, its evidence scope, the def-level defaults |
| `workflows.toml` | The workflows Armada runs, and the ones it has decided not to design yet |
| `workflow-samples/*.json` | The WorkflowDefs themselves, verbatim. One per workflow whose row says it has one |

## What a rule can check, and why the layout is shaped for it

A transition names a `from` and a `to`, each of which must be a key in
`job-statuses.toml` **verbatim** — no title-casing, no slugging, no lookup
table in between. That is why the enum files key their tables on the wire value
rather than on a slug with a `name` field beside it, the way `settings.toml`
does: a setting has a human name and a key, an enum variant has only the one
spelling, and a second spelling is the drift this file exists to catch. The same
holds for `escalation_trigger` on a transition, for `step_states` on a status,
and for `seen_under` on a step state.

Transitions are an array of tables rather than one table per edge, because an
edge has no name of its own that a Rust identifier could carry. `from` and `to`
are the whole identity.

`transitions_in` and `transitions_out` on a status are derivable from
`job-transitions.toml`, and are carried anyway — for the reason a WorkflowDef
carries `structure` when `verdict_routing` already implies it. Declared intent
checked against what was actually wired is worth the redundancy; a rule reads
both and fails where they disagree. As carried, they agree.

The WorkflowDef samples are JSON rather than TOML because that is the shape a
repo authors and Fleet interprets, and embedding them in a TOML string would
make them unparseable as what they are. A rule can read one and check every key
against `workflowdef-fields.toml`, every `advance_gate` against that row's value
table, and every `mechanical_check.type` against the sanctioned set.

## The source disagrees with itself, and that is preserved

Nothing here was repaired to make it consistent and no row was dropped. Samples
declare `structure: "linear"` while carrying a `verdict_routing` that
`structure`'s own row says is rejected at config load; escalation triggers carry
no `level` while `last_verdict` in `job-fields.toml` says only step-level
triggers may appear in it; `advanced` is a step state no status names. Those are
the findings, and repairing them here would have destroyed them. Each survives
on the row it belongs to — in `notes`, or as the empty field that says the
source left it undecided — for whoever decides it.

## What did not survive the move

Relations in the source are arrays of page links, and this repository is public,
so no address into that workspace may appear here. Every relation is carried as
the **name** of the thing it points at, which is also what makes it checkable —
a link resolves to nothing a rule can compare, a name resolves to a table key.
One row carried a raw workspace address in prose where it meant the Workflow
concept; it reads as the concept's name now.

`In code` is carried verbatim as the source recorded it, and it is provenance
rather than fact: no crate in this workspace implements any of it yet, including
the rows that say otherwise.

## Adding to these files

A value added here is a value some enum must gain, so add it in the change that
adds the variant, not before and not after. A transition added here needs both
its ends present first. Nothing under this directory is Rust and nothing is
under `src/`.

## Known inconsistencies, not open questions

Two disagreements are recorded here rather than filed, because they are most
likely authoring artefacts. This data was written in sessions with no code
alongside it, so a type-level contradiction is more probably a slip than a
design position. Whoever implements the enums verifies each against intent and
corrects the data.

- **Four samples declare `structure = "linear"` and carry `verdict_routing` on
  their review step.** The `structure` field's own rule rejects that
  combination at config load. `bug`, `feature`, `refactor`, `revert`.
- **`silent` is typed `Sub-kind` in the trigger table and its `in_code` is
  blank.** Whether it is a variant of its own or a payload on another trigger
  decides a Rust type.

## Open questions

- **[retries-exhausted-destination]** "Retries exhausted" both transitions
  `running -> completed_failed` and raises the `gate_failure` escalation. One
  condition, two destinations.
  What decides it: the two say different things about what happened — a Job
  that failed, versus a Job that needs a person. The owner's lean is that this
  deserves a state of its own rather than a choice between the existing two,
  which would make the condition unambiguous at the cost of a status.
- **[workflowdef-schema-gaps]** Five keys appear in the workflow samples with
  no row in the field catalogue: `workflow_id`, `version`, `order`, `required`,
  `manifest_rule_overrides`. The `structure` field's prose also cites an `id`
  row that does not exist.
  What decides it: the samples are the working shape and the catalogue is the
  schema, so either the catalogue is incomplete or the samples carry keys
  nothing reads. Only one can be true, and the answer decides what a parser
  accepts.
- **[review-findings-evidence-type]** `review_findings`, used by the Code
  Review workflow, is not among `evidence_type`'s legal values.
  What decides it: the source flags this against itself. Either the value set
  grows or Code Review submits evidence under an existing type.
- **[interrupted-transition-set]** `interrupted` is defined as a `running` Job
  whose process is gone, but the only edge naming it is
  `awaiting_review -> escalated`.
  What decides it: the definition and the edge disagree about which status a
  Job is in when it is interrupted. A definition nothing can reach is a state
  that never occurs.
