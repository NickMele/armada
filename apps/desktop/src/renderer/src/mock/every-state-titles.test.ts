// Every mock row's title, against the registry that owns the words a title may
// not carry.
//
// **A fixture's `name` is not a Job's title.** `asRow` built each row's title
// out of the builder's `name` — the state it demonstrates — so the Board drew
// `running — the drone is waiting for a person to allow a command` where a Job
// reads `Cache the manifest read`, and every row said what the status badge
// beside it already said (owner note, 17 Sep 2026). The words are not this
// file's to list: `crates/core-model/domain/enum-verbs.toml` owns them, and
// `JOB_STATUS` is how TypeScript reads it, so a status renamed in the registry
// is a status this proof still catches.
//
// **It covers the new rosters too.** The arc, the landing orders, the wave and
// the one-per-kind Board all put rows on a Board a person reads, and a row
// there is as much a Job as an `every-state` row is.

import { expect, test } from "vitest";
import { JOB_STATUS } from "@armada/components/src/generated/vocabulary";
import { ARC_JOB_ID, ARC_TITLE } from "@armada/screens/src/fixtures/build/arc";
import type { JobSummary } from "@armada/protocol";

import { SCENARIOS, scenarioNamed } from "./scenario";

/** Every Job `every-state` holds, recordings included. */
const JOBS = scenarioNamed("every-state")!.state.jobs;

/**
 * Every Job the arc, the landing orders, the wave and the kinds Board hold,
 * one row per Job id.
 *
 * **The arc's own Job is the exception, and it is exempt by id.** Its title is
 * the issue's own words — armada/1162, "Show what's running in the Drones
 * stat" — and a mock that renamed the work to satisfy a rule about titles
 * would be drawing a different Job from the one the boards were drawn against.
 */
const NEW_ROSTERS: JobSummary[] = [
  ...new Map(
    SCENARIOS.filter(
      (one) =>
        one.name.startsWith("arc/") ||
        one.name.startsWith("members/") ||
        one.name.startsWith("epic/") ||
        one.name === "kinds",
    )
      .flatMap((one) => one.state.jobs)
      .filter((job) => job.id !== ARC_JOB_ID)
      .map((job) => [job.id, job] as const),
  ).values(),
];

/**
 * Every word a status reads as: the wire value as it is spelled on screen, and
 * the verb the registry renders it with. Both, because a title could carry
 * either — `awaiting_review` and "awaiting review" are the same claim.
 */
const STATUS_WORDS: string[] = [
  ...Object.keys(JOB_STATUS).map((status) => status.replace(/_/g, " ")),
  ...Object.values(JOB_STATUS).flatMap((rendering) => (rendering?.verb === undefined || rendering.verb === null ? [] : [rendering.verb])),
];

/** The gate and its Checks: a title says what the work is, never how it is verified. */
const GATE_WORDS = ["gate", "gates", "check", "checks"];

/** What a title may not say, whichever roster the row came from. */
function namesWorkRatherThanAState(job: JobSummary): void {
  const title = job.title.toLowerCase();
  expect(job.title, `${job.handle} joins a state to a description with an em dash`).not.toContain("—");
  for (const word of [...STATUS_WORDS, ...GATE_WORDS]) {
    expect(
      new RegExp(`\\b${word}\\b`).test(title),
      `${job.handle}'s title carries "${word}": ${job.title}`,
    ).toBe(false);
  }
}

test("every-state holds a row", () => {
  expect(JOBS.length).toBeGreaterThan(0);
});

test("the new rosters hold rows", () => {
  expect(NEW_ROSTERS.length).toBeGreaterThan(0);
});

test.for(JOBS)("$handle's title names work rather than a state", namesWorkRatherThanAState);

test.for(NEW_ROSTERS)("$handle's title names work rather than a state", namesWorkRatherThanAState);

test("the arc's Job keeps the issue's own words", () => {
  const arc = SCENARIOS.flatMap((one) => one.state.jobs).find((job) => job.id === ARC_JOB_ID);
  expect(arc?.title).toBe(ARC_TITLE);
});

test("no two rows carry the same title", () => {
  const titles = JOBS.map((job) => job.title);
  expect(new Set(titles).size, `two every-state rows share a title: ${titles.join(", ")}`).toBe(titles.length);
});
