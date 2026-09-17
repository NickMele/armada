// What a press asks the trackpad to play, through `App` — the one place the
// tap can be read without a trackpad under it.
//
// **Answers, not controls.** The tap plays where Fleet's answer arrives, so
// what is pinned here is that an act still taps when the control that sent it
// is gone by the time the answer lands. #1326.

import { expect, test, vi } from "vitest";
import { page, userEvent } from "vitest/browser";
import type { Outcome } from "@armada/protocol";
import { reviewAtDelivery } from "@armada/screens/src/fixtures/build/index";
import { PLAN_PARTWAY, withPlan } from "@armada/screens/src/fixtures/plans";
import type { JobFixture } from "@armada/screens/src/fixtures/fixture";

import type { BridgeApi } from "../../../shared/api";
import type { Scenario } from "./scenario";
import { onJob } from "./scenario";
import { mount, unmountAfterEach } from "./testing";

unmountAfterEach();

const NOT_CONNECTED: Outcome = { ok: false, why: "not_connected" };

async function opened(fixture: JobFixture, behaves?: Scenario["behaves"]): Promise<BridgeApi> {
  const app = mount({ ...onJob(fixture), behaves });
  await expect.element(page.getByText(fixture.job.handle, { exact: true }).first()).toBeVisible();
  return app.api;
}

/** A plan row, found by its task's title. */
function rowOf(title: string) {
  return page.getByRole("listitem").filter({ hasText: title }).first();
}

test("an accepted act taps once, in alignment", async () => {
  const api = await opened(reviewAtDelivery());
  const tap = vi.spyOn(api, "tap");
  await page.getByRole("button", { name: /^Merge/ }).first().click();
  await page.getByRole("dialog").getByRole("button", { name: /^Merge/ }).click();
  await expect.poll(() => tap.mock.calls.length).toBe(1);
  expect(tap).toHaveBeenCalledWith("alignment");
});

test("a refused act plays the level change, from the same place", async () => {
  const api = await opened(reviewAtDelivery(), () => ({
    mergePullRequest: async () => NOT_CONNECTED,
  }));
  const tap = vi.spyOn(api, "tap");
  await page.getByRole("button", { name: /^Merge/ }).first().click();
  await page.getByRole("dialog").getByRole("button", { name: /^Merge/ }).click();
  await expect.poll(() => tap.mock.calls.length).toBe(1);
  expect(tap).toHaveBeenCalledWith("level_change");
});

test("a refused plan drop taps, from the screen that holds its own answer", async () => {
  const api = await opened(withPlan(PLAN_PARTWAY), () => ({
    dropTask: async () => ({ ok: false, outcome: NOT_CONNECTED }),
  }));
  const tap = vi.spyOn(api, "tap");
  const row = rowOf("Add a unit test that does not construct the store");
  await row.getByRole("button", { name: "Drop…" }).click();
  await userEvent.type(row.getByLabelText("Reason"), "Already covered elsewhere.");
  await row.getByRole("button", { name: "Drop", exact: true }).click();
  await expect.poll(() => tap.mock.calls.length).toBe(1);
  expect(tap).toHaveBeenCalledWith("level_change");
});
