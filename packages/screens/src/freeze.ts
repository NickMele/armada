// What a repository's freeze says about one Job, and about a press taken while
// it holds. `frozen.ts` is a different fact: a step beneath a Job that is over.

import { JOB_LIFECYCLE } from "@armada/components";
import type { JobSummary } from "@armada/protocol";

/** The frozen Manifests holding this Job, by id — the picker's label for a set-up repository. */
export function frozenBy(job: JobSummary): readonly string[] {
  return job.frozen_by ?? [];
}

/** `armada`, `armada and web`, `armada, web and api`. */
export function named(ids: readonly string[]): string {
  if (ids.length <= 1) return ids[0] ?? "";
  return `${ids.slice(0, -1).join(", ")} and ${ids[ids.length - 1]}`;
}

/** A sentence around the frozen ids, so a surface can set the ids in mono. */
export type FreezeLine = { lead: string; names: string; tail: string };

/**
 * What a row and the detail say about a freeze holding this Job, or `null`.
 * Only `queued` reads `frozen`; a gate's row carries `frozen_by` under its own status.
 */
export function freezeLineOf(job: JobSummary): FreezeLine | null {
  const ids = frozenBy(job);
  if (ids.length === 0) return null;
  const names = named(ids);
  if (job.status === "awaiting_review") {
    return { lead: "Nothing lands until", names, tail: ids.length > 1 ? "unfreeze" : "unfreezes" };
  }
  if (job.status === "queued" && job.queued_reason === "frozen") {
    return { lead: "Waits for", names, tail: "to unfreeze" };
  }
  return null;
}

/**
 * The same fact, short enough for a row, whose width a list shares: the badge already says `frozen`
 * on a queued row, so the row only has to name what it waits for.
 */
export function rowFreezeOf(job: JobSummary): FreezeLine | null {
  const line = freezeLineOf(job);
  if (line === null) return null;
  return job.status === "awaiting_review"
    ? { lead: "lands after", names: line.names, tail: line.tail }
    : { lead: "waits for", names: line.names, tail: "" };
}

/** The three presses a freeze takes and holds rather than refuses. */
export type TakenAct = "approve" | "restart" | "merge";

/** A press Fleet answered, and the status the Job had when it was pressed. */
export type Taken = { jobId: string; act: TakenAct; from: string | undefined };

const TAKEN_TITLE: Record<TakenAct, string> = {
  approve: "Approval taken",
  restart: "Restart taken",
  merge: "Merge taken",
};

/**
 * What to say about a press taken at a frozen repository, or `null`.
 * Read against the row as it is now: an approval or a restart reaches the queue first and gains `frozen_by` there.
 */
export function takenNotice(
  taken: Taken | null,
  job: JobSummary | undefined,
): { title: string; body: string } | null {
  if (taken === null || job === undefined || job.id !== taken.jobId) return null;
  if (JOB_LIFECYCLE[job.status]?.terminal === true) return null;
  const ids = frozenBy(job);
  if (ids.length === 0) return null;
  const frozen = `${named(ids)} ${ids.length > 1 ? "are" : "is"} frozen`;
  const body =
    taken.act === "merge"
      ? `Nothing merges while ${frozen}, so “${job.title}” merges when the freeze lifts. If Fleet restarts before then, press Merge again.`
      : `“${job.title}” waits while ${frozen}, and carries on when the freeze lifts.`;
  return { title: TAKEN_TITLE[taken.act], body };
}

/**
 * Whether a press is still worth holding: the Job has not moved on from it unfrozen, and is not over.
 * `queued` holds it too, because a row can reach the queue a beat before its `frozen_by` does.
 */
export function takenStands(taken: Taken, job: JobSummary | undefined): boolean {
  if (job === undefined || JOB_LIFECYCLE[job.status]?.terminal === true) return false;
  return frozenBy(job).length > 0 || job.status === taken.from || job.status === "queued";
}
