// A job killed and redispatched, on the Studio that dispatched it — #1440.
//
// **The defect was that the whiteboard said nothing**: the node kept reading
// the job that stopped, the work carried on somewhere else, and a person who
// came back to the Studio had no way to know. What is pinned here is what the
// board draws once Fleet has written the replacement onto it — both jobs, each
// reading its own status live, and the replacement naming where it came from.

import { expect, test } from "vitest";
import { page } from "vitest/browser";
import { killed, running } from "@armada/screens/src/fixtures/build/index";
import { repository } from "@armada/screens/src/fixtures/build/base";
import type { Studio, StudioEdge, StudioNode } from "@armada/protocol";

import { studying } from "./studio-fleet";
import { mount, unmountAfterEach } from "./testing";
import type { Scenario } from "./moment";

unmountAfterEach();

const AT = "2026-09-18T09:00:00Z";

const STOPPED = killed().job;

/**
 * `running`, on an id and a handle of its own and pointing back: a replacement is a different job
 * with a different branch and a different worktree.
 *
 * **The same title**, because `mint_replacement` carries the failed job's over — so the two cards
 * read alike, which is the whole reason the replacement says where it came from.
 */
const CARRIED_ON = {
  ...running().job,
  id: "01M2C1TJ8G0099REDISPATCHED",
  handle: "124-split-the-settings-reducer",
  title: STOPPED.title,
  redispatched_from: STOPPED.id,
};

/**
 * The Studio as Fleet leaves it after a redispatch: the node that was already there, untouched,
 * and the replacement one column to its right under the `produced` edge Fleet drew.
 */
function bothJobs(): Studio {
  const nodes: StudioNode[] = [
    { id: "n-draft", kind: "issue_draft", title: "Split the settings reducer", body: "…", state: "draft", position: { x: 0, y: 0 }, created_at: AT },
    { id: "n-stopped", kind: "job", job_id: STOPPED.id, position: { x: 340, y: 0 }, created_at: AT },
    { id: "n-carried-on", kind: "job", job_id: CARRIED_ON.id, position: { x: 680, y: 0 }, created_at: AT },
  ];
  const edges: StudioEdge[] = [
    { id: "e-dispatched", from: "n-draft", to: "n-stopped", kind: "produced", standing: "accepted", created_at: AT },
    { id: "e-carried-on", from: "n-stopped", to: "n-carried-on", kind: "produced", standing: "accepted", created_at: AT },
  ];
  return { id: "s-redispatch", manifest_id: repository().manifest!.id, name: "Settings reducer", created_at: AT, touched_at: AT, nodes, edges };
}

/** The `studios` scenario, with the two jobs on the Board the Studio's nodes read off. */
function aRedispatch(): Scenario {
  const { scenario } = studying([bothJobs()]);
  return { ...scenario, state: { ...scenario.state, jobs: [STOPPED, CARRIED_ON] } };
}

async function openTheStudio(): Promise<void> {
  await page.getByRole("button", { name: "Studios", exact: true }).first().click();
  await page.getByRole("button", { name: "Settings reducer", exact: true }).click();
}

test("both jobs are on the Studio, the one that stopped and the one the work carried on as", async () => {
  mount(aRedispatch());
  await openTheStudio();
  // Nothing is deleted: the job that was killed keeps reading what it did.
  await expect.element(page.getByRole("group", { name: `Job: ${STOPPED.title}, killed` })).toBeVisible();
  // And the replacement reads its own status, live, like any other Job node.
  await expect.element(page.getByRole("group", { name: `Job: ${CARRIED_ON.title}, running` })).toBeVisible();
  await expect.element(page.getByText(`carried on from ${STOPPED.handle}`, { exact: true })).toBeVisible();
});

test("the edge from the one that stopped to the one that took over is produced, not blocks", async () => {
  mount(aRedispatch());
  await openTheStudio();
  // One job made the next and neither waits on the other, so the relation is the Studio's own —
  // which is why it carries no label, and why no act on it is offered.
  await expect.element(page.getByRole("group", { name: /produced/ }).first()).toBeVisible();
  expect(page.getByRole("button", { name: /^Accept: / }).query()).toBeNull();
  expect(page.getByText("blocks", { exact: true }).query()).toBeNull();
});
