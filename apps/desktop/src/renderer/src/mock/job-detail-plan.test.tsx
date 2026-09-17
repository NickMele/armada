// Job detail's Plan region, its settings panel, a command it waits on, and the
// dock answering that command — through `App`. Moved here from the `Screens/Job
// detail` stories' plan, settings and dock groups — #1224.

import { expect, test, vi } from "vitest";
import { page, userEvent } from "vitest/browser";
import type { Outcome } from "@armada/protocol";
import { preparing, running, runningWaitingOnACommand } from "@armada/screens/src/fixtures/build/index";
import { JOB_ID, watchedRead } from "@armada/screens/src/fixtures/build/base";
import { WAITING_CALL } from "@armada/screens/src/fixtures/build/running";
import type { JobFixture } from "@armada/screens/src/fixtures/fixture";
import {
  awaitingApprovalPlanPending,
  PLAN_MID_TASK,
  PLAN_PARTWAY,
  PLAN_WITH_A_DROPPED_TASK,
  withPlan,
} from "@armada/screens/src/fixtures/plans";

import type { BridgeApi } from "../../../shared/api";
import { commandOutstanding, runningWithSettings } from "./job-detail-fixtures";
import type { FleetHandle, Scenario } from "./scenario";
import { onJob } from "./scenario";
import { entered, mount, unmountAfterEach } from "./testing";

unmountAfterEach();

/** What Fleet answers when it cannot be reached. */
const NOT_CONNECTED: Outcome = { ok: false, why: "not_connected" };

/** App with this Job open, over a scenario whose Fleet answers these calls its own way. */
async function opened(fixture: JobFixture, behaves?: Scenario["behaves"], whereOpen = false): Promise<BridgeApi> {
  const app = mount({ ...onJob(fixture, { whereOpen }), behaves });
  await expect.element(page.getByText(fixture.job.handle, { exact: true }).first()).toBeVisible();
  return app.api;
}

/** Whether some element reading exactly `text` is laid out on screen — the bar's own tooltip carries a hidden copy. */
function shownWithText(text: string): boolean {
  return [...document.querySelectorAll<HTMLElement>("body *")].some(
    (one) => one.textContent?.trim() === text && one.children.length === 0 && one.getClientRects().length > 0 && getComputedStyle(one).visibility !== "hidden",
  );
}

/** A plan row, found by its task's title. A list item, because the Working area names the task too. */
async function rowOf(title: string) {
  const row = page.getByRole("listitem").filter({ hasText: title });
  await expect.element(row).toBeVisible();
  return row;
}

test("a plan partway done: its region, its count, its tasks, and the branch in Where things are", async () => {
  await opened(withPlan(PLAN_PARTWAY));
  await expect.element(page.getByText("Plan", { exact: true })).toBeVisible();
  await expect.poll(() => shownWithText("1 of 3")).toBe(true);
  await expect.element(page.getByRole("listitem").getByText("T1", { exact: true })).toBeVisible();
  await expect.poll(() => shownWithText("Re-point the reducer's own import at it")).toBe(true);
  const where = page.getByRole("button", { expanded: false, name: /Where things are/i });
  await expect.element(where.getByText("fix/settings-split-selectors")).toBeVisible();
});

test("a dropped task reads its reason, and offers no second drop", async () => {
  await opened(withPlan(PLAN_WITH_A_DROPPED_TASK));
  await expect.element(page.getByText("The existing integration test already exercises this path.")).toBeVisible();
  await expect.poll(() => shownWithText("1 of 3")).toBe(true);
  const dropped = await rowOf("Add a unit test that does not construct the store");
  expect(dropped.getByRole("button", { name: "Drop…" }).query()).toBeNull();
});

test("the Working area opens the task being worked, folds the one done, and carries the plan bar", async () => {
  await opened(withPlan(PLAN_MID_TASK));
  const working = page.getByRole("button", { name: /T2\s*Re-point the reducer's own import at it/ });
  await expect.element(working).toHaveAttribute("aria-expanded", "true");
  const done = page.getByRole("button", { name: /T1\s*Extract selectColumnOrder into its own module/ });
  await expect.element(done).toHaveAttribute("aria-expanded", "false");
  await expect.element(page.getByRole("img", { name: "1 of 3 tasks" }).first()).toBeInTheDocument();
  // The Drone's sentence, as written, and its call one press away.
  await done.click();
  const calls = page.getByRole("button", { name: "1 call · Edit" }).first();
  await expect.element(calls).toHaveAttribute("aria-expanded", "false");
  await calls.click();
  await expect.element(calls).toHaveAttribute("aria-expanded", "true");
});

test("no plan on the workflow draws no Plan region", async () => {
  await opened(running());
  expect(page.getByText("Plan", { exact: true }).query()).toBeNull();
});

test("before the plan step has recorded one, there is no Plan region", async () => {
  await opened(preparing());
  expect(page.getByText("Plan", { exact: true }).query()).toBeNull();
});

test("a plan not recorded yet says which step records it, and offers no Add task", async () => {
  await opened(awaitingApprovalPlanPending());
  await expect.element(page.getByText("Plan", { exact: true })).toBeVisible();
  await expect.element(page.getByText("No plan yet — Plan the change records it.")).toBeVisible();
  expect(page.getByRole("button", { name: "Add task" }).query()).toBeNull();
});

test("Add task opens with a title, an optional detail, and nothing to add until titled", async () => {
  await opened(withPlan(PLAN_PARTWAY));
  await page.getByRole("button", { name: "Add task" }).click();
  const dialog = page.getByRole("dialog");
  await expect.element(dialog.getByLabelText("Title")).toBeVisible();
  await expect.element(dialog.getByLabelText("Detail — optional")).toBeVisible();
  await expect.element(dialog.getByRole("button", { name: "Add task" })).toBeDisabled();
});

test("Drop… is on an open task's row, and not on a done one's", async () => {
  await opened(withPlan(PLAN_PARTWAY));
  const open = await rowOf("Add a unit test that does not construct the store");
  await expect.element(open.getByRole("button", { name: "Drop…" })).toBeInTheDocument();
  const done = await rowOf("Extract selectColumnOrder into its own module");
  expect(done.getByRole("button", { name: "Drop…" }).query()).toBeNull();
});

test("a drop reason not typed yet is a hint, and Drop waits for one", async () => {
  await opened(withPlan(PLAN_PARTWAY));
  const row = await rowOf("Add a unit test that does not construct the store");
  await row.getByRole("button", { name: "Drop…" }).click();
  await expect.element(row.getByText("A reason is needed.")).toHaveAttribute("data-tone", "muted");
  await expect.element(row.getByRole("button", { name: "Drop", exact: true })).toBeDisabled();
});

test("a drop tried with no reason turns the hint into an error", async () => {
  await opened(withPlan(PLAN_PARTWAY));
  const row = await rowOf("Add a unit test that does not construct the store");
  await row.getByRole("button", { name: "Drop…" }).click();
  await row.getByLabelText("Reason").click();
  await userEvent.keyboard("{Enter}");
  await expect.element(row.getByText("A reason is needed.")).toHaveAttribute("data-tone", "error");
});

/** A Fleet that cannot be reached for a plan edit. */
const planUnreachable: Scenario["behaves"] = () => ({
  addTask: async () => ({ ok: false, outcome: NOT_CONNECTED }),
  dropTask: async () => ({ ok: false, outcome: NOT_CONNECTED }),
});

test("a refused drop says nothing was sent, and keeps the reason typed", async () => {
  await opened(withPlan(PLAN_PARTWAY), planUnreachable);
  const row = await rowOf("Add a unit test that does not construct the store");
  await row.getByRole("button", { name: "Drop…" }).click();
  await userEvent.type(row.getByLabelText("Reason"), "Already covered elsewhere.");
  await row.getByRole("button", { name: "Drop", exact: true }).click();
  await expect.element(row.getByText("Fleet is not connected. Nothing was sent.")).toBeVisible();
  await expect.element(row.getByLabelText("Reason")).toHaveValue("Already covered elsewhere.");
});

test("a refused Add task says nothing was sent, and keeps the title typed", async () => {
  await opened(withPlan(PLAN_PARTWAY), planUnreachable);
  await page.getByRole("button", { name: "Add task" }).click();
  const dialog = page.getByRole("dialog");
  await entered(dialog);
  await userEvent.type(dialog.getByLabelText("Title"), "Add a regression test");
  await dialog.getByRole("button", { name: "Add task" }).click();
  await expect.element(dialog.getByText("Fleet is not connected. Nothing was sent.")).toBeVisible();
  await expect.element(dialog.getByLabelText("Title")).toHaveValue("Add a regression test");
});

test("Job settings: a choice sends this Job's id and the wire's word, and the repository-wide allow is read-only", async () => {
  const api = await opened(runningWithSettings());
  const setWhenBlocked = vi.spyOn(api, "setWhenBlocked");
  await page.getByRole("button", { name: /^Job settings/ }).click();
  const panel = page.getByRole("dialog", { name: "Job settings" });
  await entered(panel);
  (panel.getByRole("radio", { name: "Ask me first" }).element() as HTMLElement).click();
  await expect.poll(() => setWhenBlocked.mock.calls.length).toBe(1);
  expect(setWhenBlocked).toHaveBeenCalledWith(JOB_ID, "ask_me");
  await expect.element(panel.getByText("gh issue view")).toBeVisible();
  expect(panel.getByRole("button", { name: "Remove gh issue view" }).query()).toBeNull();
});

/** Choose an answer to the waiting command. The radios are inputs under the controls they style. */
function choose(name: string): void {
  const radio = page.getByRole("radio", { name }).element() as HTMLElement;
  radio.focus();
  radio.click();
}

test("waiting on a command: Allow for this job sends the call and the answer's wire name", async () => {
  const api = await opened(runningWaitingOnACommand());
  const answerCommand = vi.spyOn(api, "answerCommand");
  await expect.element(page.getByRole("radio", { name: "Allow for this job" })).toBeInTheDocument();
  choose("Allow for this job");
  await page.getByRole("button", { name: "Send this answer" }).click();
  await expect.poll(() => answerCommand.mock.calls.length).toBe(1);
  expect(answerCommand).toHaveBeenCalledWith(JOB_ID, WAITING_CALL, "allow_for_job", undefined, undefined);
});

test("always allowing picks the narrowest rule, and sends it", async () => {
  const api = await opened(runningWaitingOnACommand());
  const answerCommand = vi.spyOn(api, "answerCommand");
  await expect.element(page.getByRole("radio", { name: "Always allow in this repository" })).toBeInTheDocument();
  choose("Always allow in this repository");
  await expect.element(page.getByRole("radio", { name: "pnpm add" })).toBeChecked();
  await page.getByRole("button", { name: "Send this answer" }).click();
  await expect.poll(() => answerCommand.mock.calls.length).toBe(1);
  expect(answerCommand).toHaveBeenCalledWith(JOB_ID, WAITING_CALL, "always_allow", undefined, "pnpm add");
});

test("rejecting a command sends the reason typed", async () => {
  const api = await opened(runningWaitingOnACommand());
  const answerCommand = vi.spyOn(api, "answerCommand");
  await expect.element(page.getByRole("radio", { name: "Reject" })).toBeInTheDocument();
  choose("Reject");
  await userEvent.type(page.getByLabelText("Note (optional)"), "we are not taking that dependency");
  await page.getByRole("button", { name: "Send this answer" }).click();
  await expect.poll(() => answerCommand.mock.calls.length).toBe(1);
  expect(answerCommand).toHaveBeenCalledWith(JOB_ID, WAITING_CALL, "reject", "we are not taking that dependency", undefined);
});

test("reading a command before answering it names the model, and decides nothing", async () => {
  const explainCommand = vi.fn(async () => ({
    ok: true as const,
    explained: {
      explanation:
        "It adds reselect 5.1.1 to this repository as a development dependency and writes the lockfile. It reaches the network.",
      model: "haiku",
    },
  }));
  await opened(runningWaitingOnACommand(), () => ({ explainCommand }));
  await page.getByRole("button", { name: "Help me understand this command" }).click();
  await expect.poll(() => explainCommand.mock.calls.length).toBe(1);
  expect(explainCommand).toHaveBeenCalledWith(JOB_ID, WAITING_CALL);
  const reading = page.getByRole("status", { name: "What this command does" });
  await expect.element(reading.getByText(/adds reselect 5\.1\.1/)).toBeVisible();
  await expect.element(reading.getByText("haiku")).toBeVisible();
  await expect.element(page.getByRole("radio", { name: "Reject" })).toBeEnabled();
});

/**
 * A Job waiting on a command, listed in Helm's dock too. `answered` is what
 * Fleet does with an answer: clear the question from both, or refuse it.
 */
async function waitingInTheDock(answered: "clears" | "refused"): Promise<void> {
  const base = runningWaitingOnACommand();
  const scenario = onJob(base);
  const behaves = (fleet: FleetHandle): Partial<BridgeApi> => ({
    answerCommand: async () => {
      if (answered === "refused") return NOT_CONNECTED;
      const whole = base.watched.state === "read" ? base.watched.detail : undefined;
      fleet.publish({
        questions: [],
        ...(whole === undefined ? {} : { watched: watchedRead({ ...whole, command_waiting: undefined }) }),
      });
      return { ok: true };
    },
  });
  mount({ ...scenario, state: { ...scenario.state, questions: [commandOutstanding(base)] }, behaves });
  await expect.element(page.getByRole("region", { name: "A question from the drone" })).toBeVisible();
}

test("answered in the dock, Job detail agrees: the band and the card both go", async () => {
  await waitingInTheDock("clears");
  await page.getByRole("article").getByRole("button", { name: "Reject" }).click();
  await expect.poll(() => page.getByRole("article").query()).toBeNull();
  await expect.poll(() => page.getByRole("region", { name: "A question from the drone" }).query()).toBeNull();
});

test("answered on Job detail, the dock agrees", async () => {
  await waitingInTheDock("clears");
  const band = page.getByRole("region", { name: "A question from the drone" });
  const reject = band.getByRole("radio", { name: "Reject" }).element() as HTMLElement;
  reject.focus();
  reject.click();
  await band.getByRole("button", { name: "Send this answer" }).click();
  await expect.poll(() => page.getByRole("region", { name: "A question from the drone" }).query()).toBeNull();
  await expect.poll(() => page.getByRole("article").query()).toBeNull();
});

test("a refused answer stands on both, with the refusal said", async () => {
  await waitingInTheDock("refused");
  await page.getByRole("article").getByRole("button", { name: "Reject" }).click();
  await expect.element(page.getByRole("alert").filter({ hasText: "Fleet is not connected. Nothing was sent." })).toBeVisible();
  await expect.element(page.getByRole("region", { name: "A question from the drone" })).toBeVisible();
  await expect.element(page.getByRole("article")).toBeVisible();
});
