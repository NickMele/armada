// A raise raises, and a figure that does not is refused before it is sent.
//
// # Why the comparison is worth a gate
//
// `RaiseCap.test.tsx`'s reason on the other ceiling: Fleet answers 422 to a cap
// at or under the one in force, so a control that let the press through turns a
// stopped job into a stopped job plus a round trip. The dialog is the
// confirmation, so the disabled confirm is the whole guard.
//
// # And what it sends, because this is the act with no conversion
//
// The cost cap is typed in dollars and sent in millionths of one. This is typed
// in turns and sent in turns, and the case below pins that: a factor slipped in
// here would be as invisible as the one that file guards against, and it would
// go the other way — a raise to 600 sent as 600000000 is a ceiling nothing
// reaches.
//
// # And the copy, because the argument it carries was wrong once
//
// The turn cap had no act for as long as a job over it was read as going in
// circles. Job `01M22TYSAE0023MADDP5ZQEYGW` passed every Check and stopped at
// 393 turns against 300 with a cheap summarise never run, so the dialog has to
// name both remedies rather than the one.

import { afterEach, expect, test } from "vitest";
import { page, userEvent } from "vitest/browser";
import type { JobSpend } from "@armada/protocol";

import { mount, unmount } from "./mounted";
import { RAISE_TURN_CAP_LABEL } from "./copy";
import { RaiseTurnCapControl } from "./RaiseTurnCap";

afterEach(unmount);

/** The job that produced this act: 393 turns taken against a cap of 300. */
const TAKEN: JobSpend = {
  cost_micros: 2_140_000,
  cost_cap_micros: 5_000_000,
  turns: 393,
  turn_cap: 300,
  ran_ms: 900_000,
  drones: 4,
};

/** Mount the control open, and hand back what the raise was told. */
function opened(spend: JobSpend = TAKEN): { sent: [string, number][] } {
  const sent: [string, number][] = [];
  mount(
    <RaiseTurnCapControl
      jobId="job_2d90bb"
      spend={spend}
      disabled={false}
      open
      onOpen={() => {}}
      onRaise={(jobId, turns) => sent.push([jobId, turns])}
    />,
  );
  return { sent };
}

/**
 * The confirm, scoped to the dialog. The opener and the confirm carry the same
 * words on purpose — an action keeps its name through the flow — so a query by
 * name alone finds two buttons, and the one outside is never disabled.
 */
function confirm() {
  return page.getByRole("dialog").getByRole("button", { name: RAISE_TURN_CAP_LABEL });
}

function figure() {
  return page.getByRole("textbox", { name: "New cap, in turns" });
}

test("it opens on a figure that is already a raise, so nobody does the arithmetic", async () => {
  opened();
  // Twice the cap in force, which is what Helm may reach on its own.
  await expect.element(figure()).toHaveValue("600");
  await expect.element(confirm()).toBeEnabled();
});

test("a figure at the cap in force is refused, and the dialog says what it is", async () => {
  const { sent } = opened();
  await userEvent.fill(figure(), "300");

  await expect.element(confirm()).toBeDisabled();
  await userEvent.keyboard("{Enter}");
  expect(sent, "a cap that raises nothing was sent to Fleet to be refused").toEqual([]);
  // Still up. Closing on a refused press would read as having gone through,
  // and the job would still be held.
  await expect.element(page.getByRole("dialog")).toBeVisible();
  await expect.element(page.getByText(/whole number above 300/)).toBeVisible();
});

test("a figure below the cap in force is refused too — this act only raises", async () => {
  const { sent } = opened();
  await userEvent.fill(figure(), "100");

  await expect.element(confirm()).toBeDisabled();
  expect(sent).toEqual([]);
});

test("a fraction of a turn is refused, because a drone takes a turn or it does not", async () => {
  const { sent } = opened();
  await userEvent.fill(figure(), "400.5");

  await expect.element(confirm()).toBeDisabled();
  expect(sent).toEqual([]);
});

test("what is typed in turns is sent in turns, with nothing converted", async () => {
  const { sent } = opened();
  await userEvent.fill(figure(), "450");

  await expect.element(confirm()).toBeEnabled();
  await userEvent.click(confirm());
  expect(sent).toEqual([["job_2d90bb", 450]]);
});

test("it names both figures the new one is decided against, and hedges neither", async () => {
  opened();
  // No tilde on either. A turn is counted, and the spend on the same header is
  // what the run would have cost at list price.
  await expect.element(page.getByText(/taken 393 of 300 turns/)).toBeVisible();
  await expect.element(page.getByText(/across 4 drones/)).toBeVisible();
});

test("it offers the redispatch as well, because the number cannot tell the two apart", async () => {
  opened();
  await expect.element(page.getByText(/Raise the cap to let it finish/)).toBeVisible();
  await expect.element(page.getByText(/Redispatch it instead/)).toBeVisible();
});
