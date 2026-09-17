// Job detail, through `App`: one Job open, the way a pressed notification opens
// it, and what its answers send to Fleet. Moved here from the `Screens/Job
// detail` stories' status, overview, frozen, compose, follow and gaming groups,
// which drew the screen from `JobDetailFrom` rather than from `App` — #1224.

import { expect, test, vi } from "vitest";
import { page, userEvent } from "vitest/browser";
import { reading, reviewAtDelivery, queued, running } from "@armada/screens/src/fixtures/build/index";
import { JOB_ID } from "@armada/screens/src/fixtures/build/base";
import { recorded } from "@armada/screens/src/fixtures/recorded";
import type { JobFixture } from "@armada/screens/src/fixtures/fixture";

import {
  advancedOnce,
  BROKEN,
  heldByTheGamingCheck,
  reviewAtAQuestion,
  withBreakages,
  withRow,
} from "./job-detail-fixtures";
import { PLAN_PARTWAY, withPlan } from "@armada/screens/src/fixtures/plans";
import type { FleetHandle } from "./scenario";
import { onJob } from "./scenario";
import { mount, unmountAfterEach } from "./testing";

unmountAfterEach();

/** App, with this Job open, and the fake it talks to. */
async function opened(fixture: JobFixture, options: { whereOpen?: boolean } = {}) {
  const app = mount(onJob(fixture, options));
  await expect.element(page.getByText(fixture.job.handle, { exact: true }).first()).toBeVisible();
  return app.api;
}

test("a judge question at the gate: Disagree, just this step, sends that answer", async () => {
  const api = await opened(reviewAtAQuestion());
  const answerJudge = vi.spyOn(api, "answerJudge");
  await page.getByRole("button", { name: "Disagree, just this step" }).click();
  expect(answerJudge).toHaveBeenCalledWith(JOB_ID, "2026-09-10T14:29:40Z", "disagree_once", undefined);
});

test("Merge asks first, with Cancel holding focus", async () => {
  await opened(reviewAtDelivery());
  await page.getByRole("button", { name: /^Merge/ }).first().click();
  await expect.element(page.getByRole("dialog").getByRole("button", { name: "Cancel" })).toHaveFocus();
});

test("picking comments on the pull request makes Send live", async () => {
  await opened(reviewAtDelivery());
  const remarks = page.getByRole("region", { name: "Comments on the pull request" });
  await expect.element(remarks).toBeVisible();
  const picks = remarks.getByRole("checkbox").elements() as HTMLElement[];
  const send = remarks.getByRole("button", { name: "Send to a drone" });
  await expect.element(send).toBeDisabled();
  await userEvent.click(picks[0]!);
  await userEvent.click(picks[1]!);
  await expect.element(send).toBeEnabled();
});

test("still reading: the run reads, and Where things are draws from what the Board held", async () => {
  await opened(reading(), { whereOpen: true });
  const run = page.getByRole("status", { name: "Reading the run" });
  await expect.element(run.getByText("Reproduction")).toBeVisible();
  await expect.element(page.getByText("Branch").first()).toBeInTheDocument();
  expect(page.getByText("Reading this job.").query()).toBeNull();
});

test("the header's one control opens the rest of what this Job can do", async () => {
  await opened(recorded("done-worktree-given-back"));
  await page.getByRole("button", { name: "Everything else this job can do" }).click();
  await expect.element(page.getByRole("menuitem", { name: /record/i })).toBeVisible();
});

test("a Check failed on a test another Job is fixing: the row names the fix", async () => {
  await opened(
    withBreakages(() => [
      {
        check: "test",
        test: BROKEN,
        failure: "expected the same reference on repeat calls",
        fix: "01M1FIXJOB000000000000000000",
        fix_title: "Fix the selectors test broken on main",
        reported_by: "01M1REPORTER0000000000000000",
      },
    ]),
    { whereOpen: true },
  );
  // The rows sit at the foot of the left column, below the window's fold at the test runner's size.
  await expect.element(page.getByText(BROKEN).first()).toBeInTheDocument();
  await expect.element(page.getByText(/Fix the selectors test broken on main is fixing it/)).toBeInTheDocument();
});

test("this Job is the fix, and two Jobs wait on it: a count, never the list", async () => {
  await opened(
    withBreakages((jobId) => [
      {
        check: "test",
        test: BROKEN,
        failure: "expected the same reference on repeat calls",
        fix: jobId,
        fix_title: "Fix the selectors test broken on main",
        reported_by: "01M1REPORTER0000000000000000",
        waiting: [
          { job_id: "01M1WAITINGONE00000000000000", title: "Split the settings reducer" },
          { job_id: "01M1WAITINGTWO00000000000000", title: "Memoise the manifest list" },
        ],
      },
    ]),
    { whereOpen: true },
  );
  await expect.element(page.getByText(/2 Jobs wait on it/)).toBeVisible();
});

const text = () => document.body.textContent ?? "";

test("frozen, queued: the status says so, and names what it waits for", async () => {
  await opened(withRow(queued(), { queued_reason: "frozen", frozen_by: ["armada"] }));
  await expect.element(page.getByText("Frozen", { exact: true }).first()).toBeVisible();
  await expect.poll(text).toMatch(/Waits for\s*armada\s*to unfreeze/);
});

test("frozen, at review: the header says nothing lands until the repository unfreezes", async () => {
  await opened(withRow(reviewAtDelivery(), { frozen_by: ["armada"] }));
  await expect.poll(text).toMatch(/Nothing lands until\s*armada\s*unfreezes/);
});

test("Merge confirmed while frozen is taken, waiting, and never drawn as a refusal", async () => {
  await opened(withRow(reviewAtDelivery(), { frozen_by: ["armada"] }));
  await page.getByRole("button", { name: /^Merge/ }).first().click();
  await page.getByRole("dialog").getByRole("button", { name: "Merge and take the work" }).click();
  await expect.element(page.getByText("Merge taken")).toBeVisible();
  await expect.element(page.getByText(/merges when the freeze lifts/)).toBeVisible();
});

// Fails today: App draws an open Job ahead of the composer, so `n` sets it open
// and nothing shows until the Job closes — #1242. Remove `.fails` with the fix.
test.fails("n opens the composer from a Job's detail", async () => {
  await opened(running());
  await userEvent.keyboard("n");
  await expect.element(page.getByText("Pick the repository this Job is for")).toBeVisible();
});

test("typing n into a task's drop reason is typing, not the dispatch key", async () => {
  await opened(withPlan(PLAN_PARTWAY));
  const found = page.getByText("Add a unit test that does not construct the store");
  await expect.element(found).toBeVisible();
  const row = page.elementLocator(found.element().closest("li")!);
  await row.getByRole("button", { name: "Drop…" }).click();
  await userEvent.type(row.getByLabelText("Reason"), "n");
  expect(page.getByText("Pick the repository this Job is for").query()).toBeNull();
});

/** The open step's name, as the panel draws it. */
const openStepName = () => document.querySelector(".armada-inside__step-name");

/** Running, on a Fleet this test can move on to the next step. */
async function advancing() {
  let fleet: FleetHandle | undefined;
  const scenario = onJob(running());
  mount({ ...scenario, behaves: (handle) => ((fleet = handle), {}) });
  await expect.element(page.getByRole("button", { name: "Fix", exact: true })).toBeVisible();
  return () => {
    const next = advancedOnce(running());
    fleet!.publish({ jobs: [next.job], watched: next.watched });
  };
}

test("advancing with the running step selected: the panel follows to the next step", async () => {
  const advance = await advancing();
  await page.getByRole("button", { name: "Fix", exact: true }).click();
  await expect.poll(() => openStepName()?.textContent).toContain("Fix");
  advance();
  await expect.poll(() => openStepName()?.textContent).toContain("Regression check");
});

test("advancing with an earlier step selected: the panel holds where the person put it", async () => {
  const advance = await advancing();
  await page.getByRole("button", { name: "Root cause", exact: true }).click();
  await expect.poll(() => openStepName()?.textContent).toContain("Root cause");
  advance();
  await expect.poll(() => page.getByRole("button", { name: /Regression check/ }).elements().length).toBeGreaterThan(0);
  expect(openStepName()?.textContent).toContain("Root cause");
});

test("held by the gaming check with the Drone still there: Send it back redirects with the flag and the note", async () => {
  const api = await opened(heldByTheGamingCheck(["override_verdict", "redirect_drone", "redispatch_job"]));
  const overrule = vi.spyOn(api, "overrideVerdict");
  const redirect = vi.spyOn(api, "redirectDrone");
  const restart = vi.spyOn(api, "restartStep");
  await expect.element(page.getByText("3 commands were refused during Regression check")).toBeVisible();
  await expect.element(page.getByText(/Sends the flag back to the drone still on this step/)).toBeVisible();
  await userEvent.type(page.getByRole("textbox", { name: "Note for the drone (optional)" }), "Put it back");
  await page.getByRole("button", { name: "Send it back" }).click();
  expect(redirect).toHaveBeenCalledWith(JOB_ID, expect.stringContaining("A test may have been weakened to make this step pass."));
  expect(redirect).toHaveBeenCalledWith(JOB_ID, expect.stringContaining("The person's note: Put it back"));
  expect(restart).not.toHaveBeenCalled();
  await page.getByRole("button", { name: "Carry on" }).click();
  await expect.poll(() => overrule.mock.calls.length).toBe(1);
  expect(overrule).toHaveBeenCalledWith(JOB_ID, "");
});

test("held by the gaming check with the Drone gone: Send it back restarts the step", async () => {
  const api = await opened(heldByTheGamingCheck(["override_verdict", "restart_step", "redispatch_job"]));
  const redirect = vi.spyOn(api, "redirectDrone");
  const restart = vi.spyOn(api, "restartStep");
  await expect.element(page.getByText(/Restarts the step with a fresh drone/)).toBeVisible();
  await page.getByRole("button", { name: "Send it back" }).click();
  await expect.poll(() => restart.mock.calls.length).toBe(1);
  expect(restart).toHaveBeenCalledWith(JOB_ID, undefined);
  expect(redirect).not.toHaveBeenCalled();
});
