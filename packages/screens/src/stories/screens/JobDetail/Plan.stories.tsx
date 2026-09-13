// The Plan region, and the two acts a person takes on it. Split out of
// `JobDetail.stories.tsx` when that file crossed the gate's 900-line rule —
// the Plan block was already one cohesive section of it, from `#896`'s own
// fixture through `#897`'s new controls.

import type { Meta, StoryObj } from "@storybook/react-vite";
import { expect, within } from "storybook/test";

import type { WorkPlan } from "@armada/protocol";
import type { JobFixture } from "../../../fixtures/fixture";
import { preparing, running } from "../../../fixtures/build/index";
import { watchedRead } from "../../../fixtures/build/base";
import { JobDetailFrom } from "./JobDetail";

const meta: Meta<typeof JobDetailFrom> = {
  title: "Screens/Job detail",
  component: JobDetailFrom,
  parameters: { layout: "fullscreen" },
};
export default meta;

type Story = StoryObj<typeof JobDetailFrom>;

const PLAN_APPROACH =
  "Split the selectors module out of the reducer so the memoised selector can " +
  "be tested without constructing the whole store. Extract selectColumnOrder " +
  "first, then re-point the reducer's own import at it.";

const PLAN_PARTWAY: WorkPlan = {
  approach: PLAN_APPROACH,
  recorded_by: { by: "step", step_id: "fix", attempt: 1 },
  recorded_at: "2026-09-10T14:16:07Z",
  tasks: [
    { id: "T1", title: "Extract selectColumnOrder into its own module", state: "done" },
    { id: "T2", title: "Re-point the reducer's own import at it", state: "working" },
    { id: "T3", title: "Add a unit test that does not construct the store", state: "open" },
  ],
};

const PLAN_WITH_A_DROPPED_TASK: WorkPlan = {
  ...PLAN_PARTWAY,
  tasks: [
    ...PLAN_PARTWAY.tasks.slice(0, 2),
    {
      id: "T3",
      title: "Add a unit test that does not construct the store",
      state: "dropped",
      reason: "The existing integration test already exercises this path.",
    },
    { id: "T4", title: "Update the settings package's README", state: "open" },
  ],
};

/** `running()`, with a `work_plan` merged onto its detail. `#896`. */
function withPlan(work_plan: WorkPlan): JobFixture {
  const fixture = running();
  if (fixture.watched.state !== "read") return fixture;
  return { ...fixture, watched: watchedRead({ ...fixture.watched.detail, work_plan }) };
}

/** The Plan region, partway done — one segment past, one working, one open. */
export const PlanPartwayDone: Story = {
  name: "Plan, partway done",
  // `whereOpen` defaults to `false` in `propsFor` — Fleet's own preference,
  // closed until it says otherwise, `#927`.
  render: () => <JobDetailFrom fixture={withPlan(PLAN_PARTWAY)} />,
  play: async ({ canvas }) => {
    await expect(canvas.getByText("Plan")).toBeVisible();
    await expect(canvas.getByText("1 of 3")).toBeVisible();
    await expect(canvas.getByText("T1")).toBeVisible();
    await expect(canvas.getByText("Re-point the reducer's own import at it")).toBeVisible();
    // Where things are opens collapsed, showing the branch on its own line.
    const where = canvas.getByRole("button", { expanded: false, name: /Where things are/i });
    await expect(within(where).getByText("fix/settings-split-selectors")).toBeVisible();
  },
};

/** The same moment, open — `whereOpen` set directly, the preference's own terms. */
export const PlanPartwayDoneWhereOpen: Story = {
  name: "Plan, partway done, Where things are open",
  render: () => <JobDetailFrom fixture={withPlan(PLAN_PARTWAY)} on={{ whereOpen: true }} />,
};

/** A dropped task stays on the list, struck through, with its reason. */
export const PlanWithADroppedTask: Story = {
  name: "Plan, with a dropped task",
  render: () => <JobDetailFrom fixture={withPlan(PLAN_WITH_A_DROPPED_TASK)} />,
  play: async ({ canvas }) => {
    await expect(canvas.getByText("The existing integration test already exercises this path.")).toBeVisible();
    // The dropped task is not counted: 1 done over 3 not dropped.
    await expect(canvas.getByText("1 of 3")).toBeVisible();
    // Already dropped, so dropping it again is not offered — #897.
    const droppedRow = (
      await canvas.findByText("Add a unit test that does not construct the store")
    ).closest("li");
    if (droppedRow === null) throw new Error("the dropped row was not found");
    await expect(within(droppedRow).queryByRole("button", { name: "Drop…" })).toBeNull();
  },
};

/** A workflow with no plan step draws no Plan region. */
export const NoPlan: Story = {
  name: "No plan on the workflow",
  render: () => <JobDetailFrom fixture={running()} />,
  play: async ({ canvas }) => {
    await expect(canvas.queryByText("Plan")).toBeNull();
  },
};

/** Before the first Drone turn, nothing is recorded yet — same absence. */
export const BeforeThePlanStepHasRecordedOne: Story = {
  name: "Before the plan step has recorded one",
  render: () => <JobDetailFrom fixture={preparing()} />,
  play: async ({ canvas }) => {
    await expect(canvas.queryByText("Plan")).toBeNull();
  },
};

/** The eyebrow act, open — the small form for a title and an optional detail. `#897`. */
export const PlanAddTaskOpen: Story = {
  name: "Plan, Add task open",
  render: () => <JobDetailFrom fixture={withPlan(PLAN_PARTWAY)} />,
  play: async ({ canvas, userEvent }) => {
    await userEvent.click(await canvas.findByRole("button", { name: "Add task" }));
    const dialog = within(document.body).getByRole("dialog");
    await expect(within(dialog).getByLabelText("Title")).toBeVisible();
    await expect(within(dialog).getByLabelText("Detail — optional")).toBeVisible();
    await expect(within(dialog).getByRole("button", { name: "Add task" })).toBeDisabled();
  },
};

/**
 * `Drop…` is offered only on `open` or `working` — present on the row a
 * cursor or the keyboard is on, and on no other. The reveal itself is
 * `:hover`/`:focus`, on the row's own stylesheet; a headless run drives
 * neither, so what a `play` can assert is what the rendering does not
 * already show for itself: which rows carry the control at all. `#897`.
 */
export const PlanDropOnRow: Story = {
  name: "Plan, Drop… on a row",
  render: () => <JobDetailFrom fixture={withPlan(PLAN_PARTWAY)} />,
  play: async ({ canvas }) => {
    const open = await canvas.findByText("Add a unit test that does not construct the store");
    const row = open.closest("li");
    if (row === null) throw new Error("the task row was not found");
    await expect(within(row).getByRole("button", { name: "Drop…" })).toBeInTheDocument();
    // `done`, on the row above, offers none — a done task cannot be dropped.
    const doneRow = (
      await canvas.findByText("Extract selectColumnOrder into its own module")
    ).closest("li");
    if (doneRow === null) throw new Error("the done row was not found");
    await expect(within(doneRow).queryByRole("button", { name: "Drop…" })).toBeNull();
  },
};

/** An empty reason cannot be sent, and the field says why. `#897`. */
export const PlanDropReasonEmpty: Story = {
  name: "Plan, Drop reason empty",
  render: () => <JobDetailFrom fixture={withPlan(PLAN_PARTWAY)} />,
  play: async ({ canvas, userEvent }) => {
    const row = (
      await canvas.findByText("Add a unit test that does not construct the store")
    ).closest("li");
    if (row === null) throw new Error("the task row was not found");
    await userEvent.click(within(row).getByRole("button", { name: "Drop…" }));
    await expect(within(row).getByText("A reason is needed.")).toBeVisible();
    await expect(within(row).getByRole("button", { name: "Drop" })).toBeDisabled();
  },
};

/**
 * Nothing is connected behind this story, so a real send refuses — and the
 * dialog stays open with what was typed, Fleet's own sentence beside it.
 * `#897`.
 */
export const PlanAddTaskRefused: Story = {
  name: "Plan, Add task refused",
  render: () => <JobDetailFrom fixture={withPlan(PLAN_PARTWAY)} />,
  play: async ({ canvas, userEvent }) => {
    await userEvent.click(await canvas.findByRole("button", { name: "Add task" }));
    const dialog = within(document.body).getByRole("dialog");
    await userEvent.type(within(dialog).getByLabelText("Title"), "Add a regression test");
    await userEvent.click(within(dialog).getByRole("button", { name: "Add task" }));
    await expect(
      await within(dialog).findByText("Fleet is not connected. Nothing was sent."),
    ).toBeVisible();
    await expect(within(dialog).getByLabelText("Title")).toHaveValue("Add a regression test");
  },
};
