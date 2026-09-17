// Armada, photographed showing a real Job.
//
// **Every frame here is a recording, drawn by the app itself.** The page is
// Bridge's own `App` on the mock Fleet (`pnpm -C apps/desktop mock`), on a
// scenario that replays a Job a Fleet actually ran through the fold Bridge's
// main process folds with — `packages/screens/src/fixtures/recorded.ts`. A
// frame of a scenario composed in code would show what somebody wrote down
// rather than what the app read, so none is taken here.
//
// **It captures and never compares**: no baseline, so a difference between two
// runs is the answer rather than a failure. `docs/concepts/manifest.md`.
import { fileURLToPath } from "node:url";

import { expect, test, type Page } from "@playwright/test";

// `evidence.frames`, resolved from this file: Playwright is invoked with
// `-C packages/components` and the path Armada reads is the repository's.
const FRAMES = fileURLToPath(new URL("../../../.armada/frames", import.meta.url));

/**
 * The port the mock is served on, from the claim Armada placed.
 *
 * **Read from the environment and never defaulted.** `armada.yml` names this
 * port once, under `ports:`; a fallback here would be a second place the
 * repository declares one, and the first worktree given a different number
 * would photograph whatever else was listening.
 */
function mock(): string {
  const port = process.env.ARMADA_PORT_MOCK;
  if (port === undefined || port.trim() === "") {
    throw new Error(
      "ARMADA_PORT_MOCK is not set, so there is no mock Bridge to photograph. " +
        "Armada sets it from the `mock` claim in armada.yml; by hand, serve " +
        "`pnpm -C apps/desktop exec vite --config vite.mock.config.ts --port <n>` and export that port.",
    );
  }
  return port.trim();
}

/** The app on one scenario, with nothing of the mock's own drawn over it. */
function at(scenario: string): string {
  return `http://localhost:${mock()}/?scenario=${encodeURIComponent(scenario)}&frame`;
}

// The titles the two recordings carry. Waiting on one of these is what makes a
// frame provably of the Job that was recorded, rather than of a scenario that
// fell back to another.
const ON_THE_BOARD = "Support attaching screenshots and file search in job context";
const THE_JOB = "Board's clear button should reclaim worktrees, not delete records";

/**
 * Write the page to `evidence.frames` under `name`, once `says` is drawn.
 *
 * **The wait is on something the recording renders, never a duration** — the
 * argument `evidence.ready` is a command rather than a number for. A fixed
 * sleep photographs a half-drawn page, which looks exactly like a broken one.
 */
async function photograph(page: Page, name: string, says: string): Promise<void> {
  await expect(page.getByText(says).first()).toBeVisible();
  // Text drawn in a fallback face and text drawn in the real one are different
  // pictures, and which one a frame catches would otherwise be a race.
  await page.evaluate(() => document.fonts.ready.then(() => true));
  await page.screenshot({ path: `${FRAMES}/${name}.png`, fullPage: true });
}

test("the board, as a day's work left it", async ({ page }) => {
  await page.goto(at("recorded-board"), { waitUntil: "domcontentloaded" });
  // `App` opens on Overview; the Board is its rail item.
  await page.getByRole("button", { name: "Job Board" }).first().click();
  await photograph(page, "board-recorded", ON_THE_BOARD);
});

test("a job that landed, read in full", async ({ page }) => {
  await page.goto(at("recorded/done-worktree-given-back"), { waitUntil: "domcontentloaded" });
  await photograph(page, "job-detail-recorded", THE_JOB);
});

test("the same job at the narrowest window bridge lays out for", async ({ page }) => {
  // `--window-floor`, the desktop window's minWidth, in `packages/tokens/src/spacing.css`.
  await page.setViewportSize({ width: 768, height: 900 });
  await page.goto(at("recorded/done-worktree-given-back"), { waitUntil: "domcontentloaded" });
  await photograph(page, "job-detail-recorded-narrow", THE_JOB);
});
