// The wave, placed for the canvas and grouped for the list. `#1544`.
//
// **Placement is computed here, from the waiting order.** The canvas holds
// none — `workflow-canvas.ts` is the precedent, and arithmetic is unit-tested
// in this package.
//
// **The needs-you split is read off `job-statuses.toml` and nothing else.**
// Blocked is a Job still holding a Drone process, which that registry gives
// exactly one of; waiting is a Job at a human gate whose Drone is gone.
// Neither word is a status and neither is minted here.

import { LANDED } from "./Row";
import type { WaveJobView, WaveView } from "./draft/wave";
import type { WaveCanvasEdge, WaveCanvasNode } from "@armada/components";

/**
 * The layout, in the canvas's own coordinates. `WAVE_APART` is a card's width
 * plus room for an arrowhead; `WAVE_DOWN` is a card's height plus its gap.
 * Numbers rather than tokens because React Flow places by number.
 */
const WAVE_APART = 320;
const WAVE_DOWN = 132;

/** `job:`, so a node id is never mistaken for a Job id a press would open. */
export const waveNodeId = (jobId: string): string => `job:${jobId}`;

/**
 * The two ways a Job of the wave needs a person.
 *
 * `blocked` holds a Drone and the worktree it is in; `waiting` is resting at a
 * gate with neither. `job-statuses.toml`'s `drone_process` column is the whole
 * of the difference, which is why this is a mapping rather than a new word.
 */
export type WaveNeed = "blocked" | "waiting";

/**
 * A Job at a human gate whose Drone has ended and whose slot has gone back.
 * Every one of these reads `drone_process = "None"` and `who_is_acting =
 * "Person"` in `job-statuses.toml`.
 */
const AT_A_GATE = new Set([
  "awaiting_approval",
  "awaiting_attestation",
  "awaiting_repair",
  "awaiting_review",
]);

/**
 * Which way this Job needs a person, or `null` for one that does not.
 *
 * `escalated` is the one status whose Drone is *"alive and idle where the step
 * stopped"* with *"the worktree and port span held as-is"*. A `running` Job
 * with a question out is the same condition mid-call, which is what
 * `JobSummary.asking` says.
 */
export function waveNeedOf(status: string, asking = false): WaveNeed | null {
  if (status === "escalated") return "blocked";
  if (status === "running") return asking ? "blocked" : null;
  return AT_A_GATE.has(status) ? "waiting" : null;
}

/** What a settled pull request reads as on a card. The one spelling, `Row.tsx`'s. */
export const waveLandedSaid = (job: WaveJobView): string | undefined =>
  job.landed === undefined ? undefined : LANDED[job.landed];

/**
 * How far behind the wave each Job sits: one past the furthest of everything
 * it waits on.
 *
 * **A cycle cannot deepen a Job past the wave's own size**, so the walk stops
 * there and leaves the rest at the depth it reached. A recorded plan should
 * carry no cycle; one that does draws a wave rather than hanging.
 */
export function waveDepths(jobs: readonly WaveJobView[]): Map<string, number> {
  const byId = new Map(jobs.map((job) => [job.job, job]));
  const depth = new Map<string, number>();
  const reaching = new Set<string>();
  const of = (id: string): number => {
    const held = depth.get(id);
    if (held !== undefined) return held;
    if (reaching.has(id)) return 0;
    reaching.add(id);
    const job = byId.get(id);
    const waits = (job?.waits_on ?? []).filter((one) => byId.has(one));
    const mine = waits.length === 0 ? 0 : Math.max(...waits.map(of)) + 1;
    reaching.delete(id);
    depth.set(id, mine);
    return mine;
  };
  for (const job of jobs) of(job.job);
  return depth;
}

/** The wave, in both arrangements, off one reading. */
export type WaveRun = {
  nodes: WaveCanvasNode[];
  edges: WaveCanvasEdge[];
  /** What a frame too small for the whole wave opens on: the Jobs still out. */
  opensOn: string[];
};

/** What a card says under its title: where it landed, and how many it waits on. */
function factsOf(job: WaveJobView, held: ReadonlySet<string>): string[] {
  const facts: string[] = [];
  const landed = waveLandedSaid(job);
  if (landed !== undefined) facts.push(landed);
  const waits = job.waits_on.filter((one) => held.has(one)).length;
  if (waits > 0) facts.push(waits === 1 ? "waits on 1" : `waits on ${String(waits)}`);
  return facts;
}

/**
 * The wave placed, with one edge per pair.
 *
 * **The edge runs from the Job waited on to the Job waiting.** `waits_on` is
 * written the other way round — it is what *this* Job waits for — so the pair
 * is flipped exactly once, here, and every surface reads the same direction.
 *
 * A Job waiting on one the wave does not hold is dropped from the edges rather
 * than drawn hanging: a plan may name work that was never dispatched.
 */
export function waveRunOf(
  view: WaveView,
  { onOpen }: { onOpen?: (jobId: string) => void } = {},
): WaveRun {
  const held = new Set(view.jobs.map((job) => job.job));
  const depth = waveDepths(view.jobs);
  const down = new Map<number, number>();
  const nodes = view.jobs.map((job): WaveCanvasNode => {
    const at = depth.get(job.job) ?? 0;
    const row = down.get(at) ?? 0;
    down.set(at, row + 1);
    return {
      id: waveNodeId(job.job),
      position: { x: at * WAVE_APART, y: row * WAVE_DOWN },
      card: {
        job: job.job,
        title: job.title,
        status: job.status,
        ...(job.handle === undefined ? {} : { handle: job.handle }),
        facts: factsOf(job, held),
        ...(onOpen === undefined ? {} : { onOpen: () => onOpen(job.job) }),
      },
    };
  });
  const edges = view.jobs.flatMap((job) =>
    job.waits_on
      .filter((one) => held.has(one))
      .map(
        (one): WaveCanvasEdge => ({
          id: `${one}>${job.job}`,
          source: waveNodeId(one),
          target: waveNodeId(job.job),
        }),
      ),
  );
  const out = view.jobs.filter((job) => job.landed === undefined);
  return {
    nodes,
    edges,
    opensOn: (out.length === 0 ? view.jobs : out).map((job) => waveNodeId(job.job)),
  };
}

/** The wave split into what has landed and what is still out. */
export type WaveStanding = {
  landed: WaveJobView[];
  out: WaveJobView[];
  blocked: WaveJobView[];
  waiting: WaveJobView[];
};

/**
 * Where each Job of the wave stands.
 *
 * **Landed is the pull request merging, not the Job completing** — a member
 * counts as landed when its pull request merged (#1530), so the word comes off
 * `Settled` and never off `completed_success`.
 *
 * `asking` is every Job with a question out, by id, which the Board row
 * carries and the wave's own view does not.
 */
export function waveStandingOf(
  view: WaveView,
  asking: ReadonlySet<string> = new Set(),
): WaveStanding {
  const landed = view.jobs.filter((job) => job.landed === "merged");
  const out = view.jobs.filter((job) => job.landed !== "merged");
  const need = (want: WaveNeed) =>
    out.filter((job) => waveNeedOf(job.status, asking.has(job.job)) === want);
  return { landed, out, blocked: need("blocked"), waiting: need("waiting") };
}

/**
 * What the wave has reached, as one sentence.
 *
 * **Counted, never estimated.** The two halves are the two a person is looking
 * for: what is done with, and what is still out.
 */
export function waveSaid(standing: WaveStanding): string {
  const jobs = standing.landed.length + standing.out.length;
  const each = jobs === 1 ? "Job" : "Jobs";
  return `${String(jobs)} ${each} under one plan — ${String(standing.landed.length)} merged, ${String(standing.out.length)} still out.`;
}
