// A raise raises, and a figure that does not is refused before it is sent.
//
// # Why the comparison is worth a gate
//
// The act exists because work is stopped for money. Fleet answers 422 to a cap
// at or under the one in force, so a control that let the press through would
// turn a stopped job into a stopped job plus a round trip — and the person who
// pressed it would have every reason to think they had just unblocked it. The
// dialog is the confirmation here, so the disabled confirm is the whole guard.
//
// # And what it sends, because the unit is where this could go quietly wrong
//
// A person types dollars and Fleet reads millionths of a dollar. Nothing on
// either side would notice a factor of a million: `$20` sent as 20 is a cap
// below a hundredth of a cent, which Fleet would refuse as no raise at all, and
// 20000000000000 is a ceiling nothing could ever reach. The case below asserts
// the integer.

import { afterEach, expect, test } from "vitest";
import { page, userEvent } from "vitest/browser";
import type { JobSpend } from "@armada/protocol";

import { mount, unmount } from "./mounted";
import { RAISE_CAP_LABEL } from "./copy";
import { RaiseCapControl } from "./RaiseCap";

afterEach(unmount);

/** The job that produced this act: $5.28 spent against a $5 cap. */
const SPENT: JobSpend = {
  cost_micros: 5_280_000,
  cost_cap_micros: 5_000_000,
  turns: 41,
  turn_cap: 300,
  ran_ms: 900_000,
  drones: 6,
};

/** Mount the control open, and hand back what the raise was told. */
function opened(spend: JobSpend = SPENT): { sent: [string, number][] } {
  const sent: [string, number][] = [];
  mount(
    <RaiseCapControl
      jobId="job_2d90bb"
      spend={spend}
      disabled={false}
      open
      onOpen={() => {}}
      onRaise={(jobId, micros) => sent.push([jobId, micros])}
    />,
  );
  return { sent };
}

/**
 * The confirm, scoped to the dialog. **The opener and the confirm carry the
 * same words on purpose** — an action keeps its name through the flow — so a
 * query by name alone finds two buttons, and the one outside is never disabled.
 */
function confirm() {
  return page.getByRole("dialog").getByRole("button", { name: RAISE_CAP_LABEL });
}

function figure() {
  return page.getByRole("textbox", { name: "New cap, in dollars" });
}

test("it opens on a figure that is already a raise, so nobody does the arithmetic", async () => {
  opened();
  // Twice the cap in force, which is what Helm may reach on its own — a person
  // who takes it has asked for no more than an agent could have.
  await expect.element(figure()).toHaveValue("10.00");
  await expect.element(confirm()).toBeEnabled();
});

test("a figure at the cap in force is refused, and the dialog says what it is", async () => {
  const { sent } = opened();
  await userEvent.fill(figure(), "5");

  await expect.element(confirm()).toBeDisabled();
  await userEvent.keyboard("{Enter}");
  expect(sent, "a cap that raises nothing was sent to Fleet to be refused").toEqual([]);
  // Still up. Closing on a refused press would read as having gone through,
  // and the job would still be stopped for money.
  await expect.element(page.getByRole("dialog")).toBeVisible();
  await expect.element(page.getByText(/above \$5\.00/)).toBeVisible();
});

test("a figure below the cap in force is refused too — this act only raises", async () => {
  const { sent } = opened();
  await userEvent.fill(figure(), "1");

  await expect.element(confirm()).toBeDisabled();
  expect(sent).toEqual([]);
});

test("what is typed in dollars is sent in millionths of a dollar", async () => {
  const { sent } = opened();
  await userEvent.fill(figure(), "20");

  await expect.element(confirm()).toBeEnabled();
  await userEvent.click(confirm());
  expect(sent).toEqual([["job_2d90bb", 20_000_000]]);
});

test("cents survive the conversion, because a cap may be set in them", async () => {
  const { sent } = opened();
  await userEvent.fill(figure(), "7.25");

  await userEvent.click(confirm());
  expect(sent).toEqual([["job_2d90bb", 7_250_000]]);
});

test("it names both figures the new one is decided against", async () => {
  opened();
  // The spend hedged and the cap exact, which is the distinction those two draw
  // everywhere else: one is what the run would have cost at list price.
  await expect.element(page.getByText(/spent about \$5\.28 of \$5\.00/)).toBeVisible();
  await expect.element(page.getByText(/across 6 drones/)).toBeVisible();
});
