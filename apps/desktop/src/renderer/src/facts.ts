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
  ManifestSummary,
  StepDetail,
  WorkflowSummary,
} from "../../shared/protocol";
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
    ...elapsedFact(whole, now),
    ...branchFact(whole),
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

/** How long the Job has been alive, from `created_at`. Measured, so it is flat. */
function elapsedFact(whole: JobWhole | null, now: number): JobDetailField[] {
  if (whole === null) return [];
  const alive = span(whole.created_at, now);
  return alive === null ? [] : [{ label: "Elapsed", value: alive, mono: true }];
}

/**
 * The branch, or the reason there is none. **Absent is a fact here**, not a
 * blank: a Job at the approval gate has no worktree, and saying so is different
 * from leaving the row out and letting somebody wonder.
 */
function branchFact(whole: JobWhole | null): JobDetailField[] {
  if (whole === null) return [];
  if (whole.branch === undefined) return [{ label: "No worktree yet" }];
  return [{ label: "Branch", value: whole.branch, mono: true, copyValue: whole.branch }];
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

/** The steps in the order they were frozen. `ordinal` is the order, not the array. */
export function ordered(whole: JobWhole | null): StepDetail[] {
  return whole === null ? [] : [...whole.steps].sort((a, b) => a.ordinal - b.ordinal);
}
