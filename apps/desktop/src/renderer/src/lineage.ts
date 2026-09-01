// A redispatch chain is one piece of work, and the list draws it as one row.
//
// # The defect
//
// A redispatch mints a new Job and kills the old one, so five goes at the same
// ask are five rows on a board whose whole purpose is scanning — and four of
// them are over. `docs/journeys/3-triage-queue.md` is explicit that the list is
// scanned rather than read, and four dead rows for one ask is what makes that
// fail.
//
// `JobSummary.redispatched_from` already carries the predecessor, so the chain
// is a linked list Bridge can walk. Nothing is asked of Fleet.
//
// # The word is `dispatch`, not `attempt`
//
// `Attempt` is taken, and it counts something else: `crates/core-model`'s
// `Attempt` is *which run of a step* a record belongs to, derived from the
// step's entries into `running`, and a step runs twice inside one Job. A
// redispatch is a whole new Job from the top. Two counters both called
// "attempt", one counting steps and one counting Jobs, is the vocabulary split
// the `milestone-step` skill warns about, so this one takes the word the act
// already has — `Approve dispatch` is on the row's own button, and `Redispatch
// as a new job` on the detail's.
//
// The chain itself is a **lineage**, which is what `docs/contracts/
// voice-engineering.md` and `docs/contracts/workflow-design-system.md` already
// call it: *"a repeat is counted across redispatches, along the
// `redispatched_from` lineage"*.
//
// # A fold never hides live work
//
// The ordinary chain has one live member, because redispatch kills the original
// as it mints the replacement. **It is not assumed.** Every member that is not
// terminal is drawn, so a lineage with two running Jobs draws two rows; only
// where nothing is live does the newest stand alone for the group. A board that
// folded a running Job away would be the failure this list exists to prevent,
// one scope smaller.
//
// Nothing is dropped. What the fold takes out is returned as `folded` and the
// list offers it, because those Jobs hold the evidence — the Judge verdicts and
// the worktrees — that is the whole reason the chain exists.
//
// # The walk is once per board, not once per row
//
// Resolving a leaf's root by walking `redispatched_from` per row is quadratic
// in the number of Jobs. The walk here memoises every id it passes, so each
// chain is climbed once however many members it has.

import { JOB_LIFECYCLE, RESUMPTION } from "@armada/components";
import type { JobSummary } from "@armada/protocol";
import { instant } from "./duration";

/** Which dispatch of one piece of work a Job is. */
export type Dispatch = {
  /** Oldest first, one-based. Dispatch 1 is the Job with no `redispatched_from`. */
  nth: number;
  /** How many the lineage has. */
  of: number;
  /**
   * True where the first dispatch is not on the board — retention swept it, or
   * Fleet did not list it. **The ordinal is then a count of what is in hand
   * rather than of what happened**, so nothing renders a number for it.
   */
  partial: boolean;
};

/** What the list draws, what it folded away, and which dispatch each Job is. */
export type Board = {
  /** Every live member of every lineage, and the newest where none is live. */
  shown: JobSummary[];
  /** The members the fold took out. Reachable, never dropped. */
  folded: JobSummary[];
  /** Keyed by job id. A Job that was never redispatched is absent. */
  dispatch: ReadonlyMap<string, Dispatch>;
};

/**
 * Group the board by lineage and choose what each group draws.
 *
 * Order is not promised: the caller sorts. `shown` and `folded` are disjoint
 * and together are every Job handed in.
 */
export function foldLineages(jobs: readonly JobSummary[]): Board {
  const byId = new Map(jobs.map((job) => [job.id, job] as const));
  const roots = rootsOf(jobs, byId);

  const lineages = new Map<string, JobSummary[]>();
  for (const job of jobs) {
    const root = roots.get(job.id) ?? job.id;
    const held = lineages.get(root);
    if (held === undefined) lineages.set(root, [job]);
    else held.push(job);
  }

  const shown: JobSummary[] = [];
  const folded: JobSummary[] = [];
  const dispatch = new Map<string, Dispatch>();

  for (const [root, members] of lineages) {
    members.sort(oldestFirst);
    // A lineage of one whose only member still names a predecessor is a chain
    // whose earlier dispatches are gone. It is still a redispatch and still
    // says so; what it does not do is call itself the first one.
    const partial = byId.get(root)?.redispatched_from !== undefined;
    if (members.length > 1 || partial) {
      members.forEach((job, i) =>
        dispatch.set(job.id, { nth: i + 1, of: members.length, partial }),
      );
    }

    const live = members.filter(working);
    // The newest stands for the group only when nothing in it is still going.
    const drawn = new Set((live.length > 0 ? live : members.slice(-1)).map((job) => job.id));
    for (const job of members) (drawn.has(job.id) ? shown : folded).push(job);
  }

  return { shown, folded, dispatch };
}

/**
 * The headline the row carries. **A qualifier is a suffix on the title**, which
 * is where `Job row (stacked)`'s own `Escalated, second time` story puts a
 * recurrence count — not a sixth field, which would land in the track the
 * drawing reserves for spend.
 *
 * Two qualifiers can apply at once and both are carried, in the order they
 * happened: which dispatch of the work this is, then what a person just did to
 * it. "Fix the parser, 2nd dispatch, restarted" is three facts and all three
 * are the row's.
 */
export function headlineOf(job: JobSummary, dispatch: Dispatch | undefined): string {
  return [job.title, ...dispatched(dispatch), ...resumed(job)].join(", ");
}

/** Which dispatch of the work this is, where the lineage has more than one. */
function dispatched(dispatch: Dispatch | undefined): string[] {
  if (dispatch === undefined) return [];
  // No number where the chain is incomplete. "3rd dispatch" on a lineage
  // missing its first two is a lie the board would have no way to correct.
  if (dispatch.partial) return ["redispatched"];
  return [`${ordinal(dispatch.nth)} dispatch`];
}

/**
 * What a person just did to put this Job back in the queue.
 *
 * **The row this exists for.** A restart and an override go back through
 * admission now, so pressing either while the bound is spent leaves the Job
 * `queued` with the badge it would have had anyway — the press moves nothing
 * on screen and reads as dropped. The badge still says what the Job is waiting
 * for; this says who put it there, and the two do not compete for one slot.
 *
 * **The word is the registry's**, from `resumption` in `enum-verbs.toml`. An
 * unknown key is a newer Fleet naming an act this build has never heard of, and
 * it renders as its own wire spelling — the fallback every other surface takes
 * rather than inventing a second vocabulary.
 */
function resumed(job: JobSummary): string[] {
  if (job.resumption === undefined) return [];
  return [RESUMPTION[job.resumption]?.verb ?? job.resumption];
}

/** What the list says beneath itself about the rows the fold took out. */
export function foldedNote(folded: number): string {
  return folded === 1
    ? "1 earlier dispatch is folded into the job above it."
    : `${folded} earlier dispatches are folded into the jobs above.`;
}

/** `1st`, `2nd`, `3rd`, `4th`, and `11th` through `13th`, which are not. */
export function ordinal(n: number): string {
  const tens = n % 100;
  if (tens >= 11 && tens <= 13) return `${n}th`;
  return `${n}${["th", "st", "nd", "rd"][n % 10] ?? "th"}`;
}

/**
 * Still going. **Anything the registry does not classify counts as live**, so
 * a status Bridge has never seen is drawn rather than folded out of sight.
 */
function working(job: JobSummary): boolean {
  return JOB_LIFECYCLE[job.status]?.terminal !== true;
}

/**
 * Creation order, which is dispatch order. **Not the linked list**, which
 * cannot be ordered when two Jobs name one predecessor — Fleet refuses that
 * with `already_redispatching`, and a board is not the place to find out it
 * did not hold. A Job whose `created_at` will not parse sorts last, matching
 * the list's own rule.
 */
function oldestFirst(a: JobSummary, b: JobSummary): number {
  const left = instant(a.created_at);
  const right = instant(b.created_at);
  if (left === null) return right === null ? 0 : 1;
  if (right === null) return -1;
  return left - right;
}

/**
 * Every Job's root, in one pass over the board.
 *
 * The walk records its whole path, so climbing a chain of five resolves all
 * five and the next member starts on a hit. A chain that loops or that names a
 * Job the board does not hold stops where it is: the renderer must not hang on
 * a corrupt lineage, and a predecessor nothing served is a root as far as this
 * board can tell.
 */
function rootsOf(
  jobs: readonly JobSummary[],
  byId: ReadonlyMap<string, JobSummary>,
): Map<string, string> {
  const roots = new Map<string, string>();
  for (const job of jobs) {
    if (roots.has(job.id)) continue;
    const path: string[] = [];
    const seen = new Set<string>();
    let at: JobSummary = job;
    let root = job.id;
    for (;;) {
      const known = roots.get(at.id);
      if (known !== undefined) {
        root = known;
        break;
      }
      if (seen.has(at.id)) {
        root = at.id;
        break;
      }
      seen.add(at.id);
      path.push(at.id);
      const from = at.redispatched_from;
      const next = from === undefined ? undefined : byId.get(from);
      if (next === undefined) {
        root = at.id;
        break;
      }
      at = next;
    }
    for (const id of path) roots.set(id, root);
  }
  return roots;
}
