// Where a Job's work is, what the Job was told, and what it has spent — the
// region a person opens when a Job has gone wrong.
//
// # Derived is not served, and the region says which
//
// `branch` is served. The worktree, the log and the transcript directory are
// **derived**, from the Job's id and the repository its Manifest was read from
// — see `shared/artifacts.ts`, which owns that arithmetic for both sides. The
// architecture fixes the layout, says it is not configurable, and says any
// path Fleet needs is derived rather than stored, so a path derived on this
// side is the same path and not a guess.
//
// # A row opens by naming itself, never by handing over its path
//
// `open` sends the Job id and one of three words. Main derives the path again
// and hands that to the OS — a string composed here and passed to
// `shell.openPath` would be an arbitrary-file capability wearing a row's
// clothes. So this file draws the path and main opens it, from one derivation.
//
// # No count Bridge would have to invent
//
// The drawing shows "142 lines · 0 error". Nothing counts lines or errors, so a
// path row names its path and stops. The `meta` on one of them says a file is
// not written yet, which is read off `branch`: the worktree is created and the
// branch stamped on the Job before the log is opened, so a Job with no branch
// has nothing under any of these paths.
//
// **What the Job has spent is the exception, and it is the rule's own reason
// that admits it.** `JobDetail.spend` is served — Fleet is the only side that
// can add it up, because a Job's Drones do not know about each other — so
// drawing it here is reading a figure rather than inventing one. It is the
// second group, under a rule, and `spending` below carries why both halves of
// each pair are on the row.

import { File, Folder, GitBranch } from "lucide-react";
import type { JobBriefProps, JobDetailLog, JobLogReferenceRow, NotOpened } from "@armada/components";

import type { Watched } from "../../shared/bridge";
import { artifactPath, repoOf } from "../../shared/artifacts";
import type { Artifact, Opened } from "../../shared/artifacts";
import type {
  JobDetail as JobWhole,
  JobSpend,
  JobSummary,
} from "../../shared/protocol";
import { lasting } from "./duration";
import type { ManifestSummary } from "../../shared/setup";

export { repoOf };

/**
 * What the Job was told, what done means for it, and what is still waiting to
 * be told to it. All three are served.
 *
 * **The waiting note is passed through and never remembered.** Fleet clears it
 * off the record the instant a drone's opening brief is built from it, so
 * `redirect_waiting` absent is both "nobody wrote one" and "the one somebody
 * wrote has gone in" — and neither of those is a thing to draw. The move that
 * delivers it puts the job at `running`, which is a `job.state_changed` that
 * `connection.ts` re-reads the open job on, so the block leaves the screen on
 * the same transition that empties the field.
 */
export function briefOf(whole: JobWhole): JobBriefProps {
  return {
    criteria: whole.acceptance_criteria.map((criterion) => ({
      text: criterion.text,
      // The wire spelling. No registry carries a verb for `criterion_source`,
      // and one written here would be a second vocabulary.
      source: criterion.source,
    })),
    criteriaAbsent:
      "This job was proposed with no acceptance criteria, so nothing states what done " +
      "means for it. Bridge's composer does not offer them yet.",
    facts: whole.facts,
    factsAbsent: "This job was given no context beyond its title.",
    // No `waitingAbsent` beside it. Absent here is the ordinary state of every
    // job on the board, and a sentence saying so would be a permanent
    // paragraph reporting that nothing has happened.
    waiting: whole.redirect_waiting?.note,
  };
}

/**
 * Why an open did not happen, in the app's voice.
 *
 * **Four sentences, not one.** A reclaimed worktree, a Manifest Fleet no
 * longer holds and an OS with no handler for `.jsonl` need three different
 * next steps, and a shared sentence would send a person to look for the wrong
 * one. Beside the paths rather than in `copy.ts` because each names what these
 * rows are, the way `noteOf` below does.
 */
function whyNotOpened(opened: Opened, what: Artifact): string | null {
  if (opened.ok) return null;
  switch (opened.why) {
    case "unknown_job":
      return "Bridge no longer holds this job, so it could not work out where that is.";
    case "no_repository":
      return (
        "Fleet holds no manifest with this job's id, so Bridge cannot say which repository " +
        "the work is in. The path above follows from the job id once it can."
      );
    case "not_there":
      return what === "worktree"
        ? "Nothing is at that worktree. It was reclaimed, and the branch survives it."
        : "That file has not been written, or it has been removed. The path is still where it goes.";
    case "refused":
      return `This machine did not open it: ${opened.detail}`;
  }
}

/** Open one of a Job's artifacts, and say why it did not when it did not. */
function opener(jobId: string, what: Artifact, label: string) {
  return {
    label,
    go: async (): Promise<NotOpened> => {
      const because = whyNotOpened(await window.armada.openArtifact(jobId, what), what);
      return because === null ? null : { because };
    },
  };
}

/**
 * The region: the brief, then the paths.
 *
 * `undefined` while the Job has not been read whole — the caller says why,
 * because "still reading" and "Fleet did not answer" are different sentences.
 * `withBranch` is false on the finished render, where the handover above
 * already names the branch and drawing it twice is two places to keep in step.
 */
export function workOf(
  job: JobSummary,
  whole: JobWhole | null,
  manifest: ManifestSummary | undefined,
  withBranch: boolean,
): JobDetailLog | undefined {
  if (whole === null) return undefined;
  const repo = repoOf(manifest);
  const dispatched = whole.branch !== undefined;
  const rows: JobLogReferenceRow[] = [];

  if (repo !== null) {
    const worktree = artifactPath("worktree", repo, job.id, job.assigned_drone);
    rows.push({
      // `folder` means "workspace" in the registry and a worktree is not one;
      // there is no row for a worktree. Reported, and no glyph is invented.
      icon: Folder,
      iconLabel: "Worktree",
      value: worktree,
      copyValue: worktree,
      open: opener(job.id, "worktree", "Open the worktree"),
      meta: dispatched ? undefined : NOT_WRITTEN,
    });
  }

  if (withBranch && whole.branch !== undefined) {
    // No `open`, and none is coming. A branch is served rather than derived,
    // it is not a path, and copying it is the whole of what it is for.
    rows.push({
      icon: GitBranch,
      iconLabel: "Branch",
      value: whole.branch,
      copyValue: whole.branch,
    });
  }

  if (repo !== null) {
    const log = artifactPath("log", repo, job.id, job.assigned_drone);
    rows.push({
      icon: File,
      iconLabel: "Log",
      value: log,
      copyValue: log,
      open: opener(job.id, "log", "Open the log"),
      meta: dispatched ? undefined : NOT_WRITTEN,
      separated: true,
    });
    rows.push(transcript(repo, job));
  }

  rows.push(...spending(whole.spend));

  return { brief: briefOf(whole), rows, note: noteOf(repo, dispatched) };
}

/**
 * What the job has spent, against what it is allowed to spend.
 *
 * **The one exception to this region's "no count, ever" rule, and it is the
 * rule's own reason that admits it.** That rule was written because nothing
 * counted: a file's lines and a log's errors are figures Bridge would have had
 * to invent, so a row named its path and stopped. These four are served —
 * `JobDetail.spend`, since protocol 5.5 — and Fleet is the only side that can
 * add them up, because a job's drones do not know about each other.
 *
 * **Both halves on one row.** A figure with no ceiling says nothing about
 * whether the job is near one, and a ceiling with no figure says nothing about
 * this job. It is the pair that tells a person which of the two signals held
 * their job back, which is what the `over budget` badge on its own cannot.
 *
 * **The wall clock has no ceiling beside it** and says so, because there is
 * none to draw: it is bounded by a setting at a different scope that nothing
 * enforces yet, and a figure drawn as though it were capped would be the
 * unenforced-cap failure in another shape.
 *
 * Absent on a Fleet older than 5.5, which draws nothing rather than zeroes —
 * a job that cost nothing and a Fleet that does not count are different facts.
 * No glyph on either row: nothing in the icon registry depicts money or a turn,
 * and the values read as what they are.
 */
function spending(spend: JobSpend | undefined): JobLogReferenceRow[] {
  if (spend === undefined) return [];
  return [
    {
      iconLabel: "Spent",
      value: `${money(spend.cost_micros)} of ${money(spend.cost_cap_micros)}`,
      meta: droneCount(spend.drones),
      separated: true,
    },
    {
      iconLabel: "Turns",
      value: `${spend.turns} of ${spend.turn_cap} turns`,
      meta: `${lasting(spend.ran_ms)} of drone time, uncapped`,
    },
  ];
}

/**
 * A spend, from millionths of a dollar.
 *
 * **Hedged, and the same rendering `turns.ts` uses**, for the same two reasons.
 * The design contract spells an estimated value `~$2.40` and never `$2.40`; and
 * the figure is notional besides — it is what the run would have cost at list
 * price, which is not what a subscription account is billed.
 *
 * **Four places under a cent**, so a job that has cost a tenth of a penny does
 * not round to `~$0.00` and read as free.
 */
function money(micros: number): string {
  const dollars = micros / 1_000_000;
  return `~$${dollars.toFixed(dollars > 0 && dollars < CENT ? 4 : 2)}`;
}

/** A cent, in dollars. Below this two decimal places round to nothing. */
const CENT = 0.01;

/**
 * How many drones the figure is the sum of.
 *
 * **Zero is stated rather than left out**, because zero drones and a cost of
 * zero are different facts: one is a job nobody has run yet and the other is a
 * job that ran for free, and a row that said nothing would let a reader take
 * the first for the second.
 */
function droneCount(drones: number): string {
  if (drones === 0) return "no drone has run yet";
  return drones === 1 ? "across 1 drone" : `across ${drones} drones`;
}

/** A file that will exist, named before it does. Never a count. */
const NOT_WRITTEN = "not written yet";

/**
 * The Drone's transcript.
 *
 * **The file is named by a drone id that is stored nowhere.** `assigned_drone`
 * has no event that sets it, so the only record the id existed is the line
 * Fleet writes into the Job log named above. The row names the directory and
 * says that, rather than printing a path with a hole in it — and opening it
 * lands in the directory, which is where a person would look anyway. No glyph:
 * nothing in the registry means a transcript, and `file` is reserved to the
 * log row.
 */
function transcript(repo: string, job: JobSummary): JobLogReferenceRow {
  const path = artifactPath("transcript", repo, job.id, job.assigned_drone);
  const open = opener(job.id, "transcript", "Open the transcript");
  if (job.assigned_drone === undefined) {
    return {
      iconLabel: "Transcript",
      value: path,
      copyValue: path,
      open,
      meta: "named by a drone id nothing serves — the job log above names it",
    };
  }
  return { iconLabel: "Transcript", value: path, copyValue: path, open };
}

/** What the paths are, said once beneath them rather than on every row. */
function noteOf(repo: string | null, dispatched: boolean): string {
  if (repo === null) {
    return (
      "Fleet holds no manifest with this job's id, so Bridge cannot say which repository " +
      "the work is in. The paths follow from the job id once it can."
    );
  }
  const derived =
    "The worktree, the log and the transcripts directory follow from this job's id and " +
    "the repository its manifest was read from. The branch is served.";
  return dispatched
    ? `${derived} Armada leaves the worktree and the branch in place.`
    : `${derived} Nothing has been dispatched, so nothing under these paths exists yet.`;
}

/** Why there is no region to draw, which is never the same sentence twice. */
export function whyNoWork(watched: Watched, jobId: string): string {
  if (watched.state === "failed" && watched.jobId === jobId) {
    return "Fleet did not answer for this job, so its paths and its brief are unknown.";
  }
  return "Reading this job.";
}
