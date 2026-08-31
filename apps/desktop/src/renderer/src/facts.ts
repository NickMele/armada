// The header's field run on job detail: every fact the two operations carry,
// and nothing else.
//
// It is here rather than in `JobDetail.tsx` because it is one job — turning a
// served Job into a list of labelled facts — and the screen beside it is
// another. Nothing in this file decides how a fact looks; `JobDetailField` is
// the design system's shape and this only fills it in.
//
// **Spend is on all three drawings and on neither operation**, so it is not in
// the run: a labelled blank is worse than an absent field.

import type { JobDetailField } from "@armada/components";

import type {
  JobDetail as JobWhole,
  JobSummary,
  StepDetail,
} from "../../shared/protocol";
import type { ManifestSummary, WorkflowSummary } from "../../shared/setup";
import { span } from "./duration";

/**
 * The run, in the order the drawing runs it. Elapsed is here because
 * `created_at` is served, measured from that rather than from anything Bridge
 * remembers.
 */
export function factsOf(
  job: JobSummary,
  whole: JobWhole | null,
  workflow: WorkflowSummary | undefined,
  manifest: ManifestSummary | undefined,
  now: number,
): JobDetailField[] {
  return [
    ...stepFacts(job, whole),
    ...elapsedFact(job, now),
    ...branchFact(job),
    {
      // The workflow's name where Fleet holds it, the id where it does not —
      // which after the refusal at creation means a Job older than the check.
      label: "Workflow",
      value: workflow === undefined ? job.workflow_id : workflow.name,
      mono: workflow === undefined,
      copyValue: job.workflow_id,
    },
    {
      // The repository, because `armada.yml` declares no name for a Manifest.
      label: "Manifest",
      value: manifest?.repository ?? job.owner_manifest_id,
      mono: true,
      copyValue: job.owner_manifest_id,
    },
    { label: "Model", value: job.model, mono: true },
    // Origin and urgency are closed sets with no row in the enum→verb map, so
    // the wire spelling is what there is. Mono says it is the system's word.
    { label: "Origin", value: job.origin, mono: true },
    { label: "Urgency", value: job.urgency, mono: true },
    ...(job.atomic ? [{ label: "Atomic" }] : []),
    ...(job.assigned_drone === undefined
      ? []
      : [{ label: "Drone", value: job.assigned_drone, mono: true, copyValue: job.assigned_drone }]),
    ...scopeFacts(whole),
    ...overlapFacts(whole),
    ...(whole?.subject === undefined
      ? []
      : [
          {
            label: whole.subject.kind,
            value: whole.subject.reference,
            mono: true,
            copyValue: whole.subject.reference,
          },
        ]),
    ...(whole?.dependencies ?? []).map((edge) => ({
      label: edge.direction === "blocks" ? "Blocks" : "Depends on",
      value: edge.peer,
      mono: true,
      copyValue: edge.peer,
    })),
    // Lineage. A redispatch mints a new Job, so without this the second attempt
    // reads as a first one.
    ...(job.redispatched_from === undefined
      ? []
      : [
          {
            label: "Replaces",
            value: job.redispatched_from,
            mono: true,
            copyValue: job.redispatched_from,
          },
        ]),
  ];
}

/**
 * Where the Job is in its workflow. **The count comes from the served steps**,
 * not from the workflow roster — the steps were frozen at creation, so a
 * workflow edited since would put the Job at a step of a different length.
 */
function stepFacts(job: JobSummary, whole: JobWhole | null): JobDetailField[] {
  if (job.current_step_id === undefined) return [{ label: "Not started" }];
  const steps = ordered(whole);
  const at = steps.findIndex((step) => step.step_id === job.current_step_id);
  if (at < 0) {
    // The detail has not arrived, or the Job is on a step its frozen workflow
    // does not list. The step itself is still a fact and is still drawn.
    return [{ label: "At", value: job.current_step_id, mono: true }];
  }
  return [
    { label: "Step", value: `${at + 1} of ${steps.length}`, mono: true },
    { label: "at", value: job.current_step_id, mono: true, continues: true },
  ];
}

/**
 * How long the Job has been alive, from `created_at`.
 *
 * **Read off the row rather than off the detail.** Both carry it and cannot
 * disagree — the detail's is built from the same record — and the row is
 * already in hand, so the fact is there on the first frame instead of appearing
 * when `GET /jobs/:job_id` lands.
 */
function elapsedFact(job: JobSummary, now: number): JobDetailField[] {
  const alive = span(job.created_at, now);
  return alive === null ? [] : [{ label: "Elapsed", value: alive, mono: true }];
}

/**
 * The branch, or the reason there is none. **Absent is a fact here**, not a
 * blank: a Job at the approval gate has no worktree, and saying so is different
 * from leaving the row out and letting somebody wonder.
 */
function branchFact(job: JobSummary): JobDetailField[] {
  if (job.branch === undefined) return [{ label: "No worktree yet" }];
  return [{ label: "Branch", value: job.branch, mono: true, copyValue: job.branch }];
}

/**
 * What the Job may write. **Three different sentences, because there are three
 * different facts** — nobody has decided the scope, the scope is decided and it
 * is nothing, or here is the scope.
 */
function scopeFacts(whole: JobWhole | null): JobDetailField[] {
  if (whole === null) return [];
  if (whole.write_targets === undefined) return [{ label: "Scope undetermined" }];
  if (whole.write_targets.length === 0) return [{ label: "Writes nothing" }];
  return [
    {
      label: "Writes",
      value: whole.write_targets.join(", "),
      mono: true,
      copyValue: whole.write_targets.join(" "),
    },
  ];
}

/**
 * Who else claims these paths. **A fact in the run, not a warning banner** —
 * `docs/concepts/fleet.md` says the overlap is surfaced and never serialised,
 * and a fact beside `Writes` is what that looks like on screen. Nothing here
 * is red, nothing here disables anything, and approving anyway is the ordinary
 * case.
 *
 * Absent and empty are both silent, and for the same reason: neither is
 * something to tell a person. Their difference is on the wire and is what stops
 * a future surface saying "no overlap" about a Job nothing compared.
 *
 * **It names the collision and offers nothing to do about it.** The
 * `depends_on` edge `docs/concepts/job-board.md` promises beside this is
 * `#231`: it needs an operation that adds an edge to an existing Job, and
 * `crates/fleet/src/coupling.rs` is why that is not free. Until then the two
 * answers are the ones the gate already had.
 */
function overlapFacts(whole: JobWhole | null): JobDetailField[] {
  return (whole?.write_scope_overlaps ?? []).flatMap((other) => {
    const paths = other.paths.map((shared) => shared.path);
    return [
      { label: "Overlaps", value: other.title, copyValue: other.job_id },
      {
        label: "on",
        value: paths.join(", "),
        mono: true,
        copyValue: paths.join(" "),
        continues: true,
      },
    ];
  });
}

/** The steps in the order they were frozen. `ordinal` is the order, not the array. */
export function ordered(whole: JobWhole | null): StepDetail[] {
  return whole === null ? [] : [...whole.steps].sort((a, b) => a.ordinal - b.ordinal);
}
