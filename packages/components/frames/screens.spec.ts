// Armada, photographed showing a real Job.
//
// **Every story here is a recording.** Both screens also carry states composed
// in code for cases a real Job rarely sits in; a frame of one of those shows
// what somebody wrote down rather than what the app read. These three draw a
// Job a Fleet actually ran, replayed through the fold Bridge's main process
// folds with — `packages/screens/src/fixtures/recorded.ts`.
//
// **It captures and never compares**: no baseline, so a difference between two
// runs is the answer rather than a failure. `docs/concepts/manifest.md`.
import { fileURLToPath } from "node:url";

import { expect, test, type Page } from "@playwright/test";

// `evidence.frames`, resolved from this file: Playwright is invoked with
// `-C packages/components` and the path Armada reads is the repository's.
const FRAMES = fileURLToPath(new URL("../../../.armada/frames", import.meta.url));

/**
 * The port Storybook is on, from the claim Armada placed.
 *
 * **Read from the environment and never defaulted.** `armada.yml` names this
 * port once, under `ports:`; a fallback here would be a second place the
 * repository declares one, and the first worktree given a different number
 * would photograph whatever else was listening.
 */
function storybook(): string {
  const port = process.env.ARMADA_PORT_STORYBOOK;
  if (port === undefined || port.trim() === "") {
    throw new Error(
      "ARMADA_PORT_STORYBOOK is not set, so there is no Storybook to photograph. " +
        "Armada sets it from the `storybook` claim in armada.yml; by hand, start " +
        "the server with `armada run storybook_dev` and export the port it prints.",
    );
  }
  return port.trim();
}

/** One story, by the id Storybook derives from its title and its export. */
function at(id: string): string {
  return `http://localhost:${storybook()}/iframe.html?id=${id}&viewMode=story`;
}

// The titles the two recordings carry. Waiting on one of these is what makes a
// frame provably of the Job that was recorded, rather than of a story that fell
// back to something built by hand.
const ON_THE_BOARD = "Support attaching screenshots and file search in job context";
const THE_JOB = "Board's clear button should reclaim worktrees, not delete records";

/**
 * Draw one story and write it to `evidence.frames` under `name`.
 *
 * **The wait is on something the recording renders, never a duration** — the
 * argument `evidence.ready` is a command rather than a number for. A fixed
 * sleep photographs a half-drawn page, which looks exactly like a broken one.
 * Neither screen draws a heading, so the anchor is the Job's own title.
 */
async function photograph(page: Page, id: string, name: string, says: string): Promise<void> {
  await page.goto(at(id), { waitUntil: "domcontentloaded" });
  await expect(page.getByText(says).first()).toBeVisible();
  // Text drawn in a fallback face and text drawn in the real one are different
  // pictures, and which one a frame catches would otherwise be a race.
  await page.evaluate(() => document.fonts.ready.then(() => true));
  await page.screenshot({ path: `${FRAMES}/${name}.png`, fullPage: true });
}

test("the board, as a day's work left it", async ({ page }) => {
  await photograph(page, "screens-board--recorded", "board-recorded", ON_THE_BOARD);
});

test("a job that landed, read in full", async ({ page }) => {
  await photograph(page, "screens-job-detail--done-recorded", "job-detail-recorded", THE_JOB);
});

test("the same job at the narrowest window bridge lays out for", async ({ page }) => {
  await photograph(
    page,
    "screens-job-detail--done-recorded-narrow",
    "job-detail-recorded-narrow",
    THE_JOB,
  );
});
