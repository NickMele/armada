// What a finished Job cost, and what each group of it came to. #1542.
//
// Apart from `landed.ts` so that file stays a reading rather than a ledger of
// sums. Every figure here counts its retries: a retried group ran its tasks
// and its boundary's Checks again, and a count that ignores that contradicts
// the retry drawn beside it.

import type { Figure } from "@armada/components";
import type { JobDetail as JobWhole } from "@armada/protocol";

import { cap } from "./RaiseCap";
import { span } from "./duration";
import { GROUP_STATE_WORDS, type GroupView } from "./draft";

/** One group, as the Produced panel draws it. */
export type LandedGroup = {
  name: string;
  verb: string;
  status?: string;
  tasks: string;
  took?: string;
  files: string;
  checks?: string;
  commit?: string;
};

/** What a Job cost, as the board's own readings. */
export type LandedCost = { name: string; figures: Figure[]; note: string };

/** What it cost: the run, the spend and the turns, the agents and the Checks. */
export function costOf(whole: JobWhole, groups: readonly GroupView[]): LandedCost {
  const spend = whole.spend;
  const tasks = groups.flatMap((group) => group.tasks);
  const priced = tasks.filter((task) => task.cost_micros !== undefined);
  const micros = priced.reduce((sum, task) => sum + (task.cost_micros ?? 0), 0);
  const turns = tasks.reduce((sum, task) => sum + (task.turns ?? 0), 0);
  const ran = whole.job.started_at === undefined || whole.job.ended_at === undefined
    ? undefined
    : (span(whole.job.started_at, whole.job.ended_at) ?? undefined);
  const drones = dronesOf(groups);
  const checks = checksOf(groups);

  const figures: Figure[] = [];
  if (ran !== undefined) figures.push({ label: "Run time", value: ran });
  figures.push(
    spend === undefined
      ? { label: "Spend", value: cap(micros), detail: "no cap on this Job's record" }
      : { label: "Spend", value: `${cap(spend.cost_micros)} of ${cap(spend.cost_cap_micros)}` },
  );
  figures.push(
    spend === undefined
      ? { label: "Turns", value: String(turns), detail: "no cap on this Job's record" }
      : { label: "Turns", value: `${spend.turns} of ${spend.turn_cap}` },
  );
  figures.push({
    label: "Drones",
    value: String(spend?.drones ?? drones.count),
    ...(drones.detail === undefined ? {} : { detail: drones.detail }),
  });
  if (checks.count > 0) {
    figures.push({
      label: "Checks",
      value: String(checks.count),
      ...(checks.detail === undefined ? {} : { detail: checks.detail }),
    });
  }
  return {
    name: "What it cost",
    figures,
    note:
      spend === undefined
        ? "Spend and turns are added up from each task's own agent, which is where cost arrives."
        : "Spend and turns are the Job's own totals, which every Drone of it reported into.",
  };
}

/**
 * How many agents ran. **One per task, and a retried group ran its tasks
 * again** — the count the retry beside it implies.
 */
function dronesOf(groups: readonly GroupView[]): { count: number; detail?: string } {
  const count = groups.reduce((sum, group) => sum + group.tasks.length * (1 + group.retry_count), 0);
  const again = groups.filter((group) => group.retry_count > 0);
  if (again.length === 0) return { count };
  return { count, detail: `${retriedIn(again)} ran again` };
}

/** How many Check runs the boundaries made, a retried boundary counted twice. */
function checksOf(groups: readonly GroupView[]): { count: number; detail?: string } {
  const count = groups.reduce(
    (sum, group) => sum + group.checks_selected.length * (1 + group.retry_count),
    0,
  );
  const again = groups.filter((group) => group.retry_count > 0);
  if (again.length === 0) return { count };
  return { count, detail: `${retriedIn(again)} ran twice` };
}

/** `group three` — what a retried group is called in a sentence. */
function retriedIn(groups: readonly GroupView[]): string {
  return groups.map((group) => `group ${ordinalWord(group.ordinal)}`).join(", ");
}

const ORDINALS = ["one", "two", "three", "four", "five", "six", "seven", "eight"];

function ordinalWord(ordinal: number): string {
  return ORDINALS[ordinal - 1] ?? String(ordinal);
}

/** One group's row: what it came to, and what it left. */
export function groupOf(group: GroupView): LandedGroup {
  const word = GROUP_STATE_WORDS[group.state];
  const done = group.tasks.filter((task) => task.state === "done").length;
  const files = new Set(group.tasks.flatMap((task) => task.scope)).size;
  const checks = group.checks_selected.length;
  const took =
    group.started_at === undefined || group.ended_at === undefined
      ? null
      : span(group.started_at, group.ended_at);
  return {
    name: `Group ${ordinalWord(group.ordinal)}`,
    verb: word.verb ?? group.state,
    status: word.badgeStatus ?? undefined,
    tasks: `${done} of ${group.tasks.length} done`,
    ...(took === null ? {} : { took }),
    files: files === 1 ? "1 file" : `${files} files`,
    ...(checks === 0
      ? {}
      : { checks: group.retry_count > 0 ? `${checks} Checks, twice` : `${checks} Checks` }),
    ...(group.commit === undefined ? {} : { commit: group.commit }),
  };
}

export function summaryOf(groups: readonly GroupView[]): string {
  const tasks = groups.flatMap((group) => group.tasks);
  const files = new Set(tasks.flatMap((task) => task.scope)).size;
  return `${groups.length} groups · ${tasks.length} tasks · ${files} files`;
}

/** Why a group's row says `not timed`. Fleet times a step, and a group is not one. */
export const GROUPS_NOTE =
  "Nothing times a group: the record times a step, so no group here carries a span of its own.";
