import type { Meta, StoryObj } from "@storybook/react-vite";
import { expect, waitFor, within } from "storybook/test";

import { preparing, running } from "../../../fixtures/build/index";
import { JobDetailFrom } from "./JobDetail";
import { awaitingApprovalPlanPending, PLAN_PARTWAY, PLAN_WITH_A_DROPPED_TASK, withPlan } from "../../../../../../../apps/desktop/src/renderer/src/mock/job-plan-fixtures";

/** Job detail, split by group — #1044. Same `title` as the rest of this directory, so ids hold. */
const meta: Meta<typeof JobDetailFrom> = {
  title: "Screens/Job detail",
  component: JobDetailFrom,
  parameters: { layout: "fullscreen" },
};
export default meta;

type Story = StoryObj<typeof JobDetailFrom>;

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
    const where = canvas.getByRole("button", { expanded: false, name: /Where things are/i });
    await expect(within(where).getByText("fix/settings-split-selectors")).toBeVisible();
  },
};

/** The same moment, `whereOpen` set directly — the preference's own terms. */
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
    await expect(canvas.getByText("1 of 3")).toBeVisible();
    // Already dropped, so dropping it again is not offered — #897.
    const found = await canvas.findByText("Add a unit test that does not construct the store");
    const droppedRow = found.closest("li");
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

/**
 * A step declares `plan_recorded` and has not run yet — a Bug Job at the
 * approval gate, its plan step still ahead. The quiet placeholder, and no
 * task bar, figure, approach or `Add task` until a plan exists. `#1007`.
 */
export const PlanPending: Story = {
  name: "Plan, before it's recorded",
  render: () => <JobDetailFrom fixture={awaitingApprovalPlanPending()} />,
  play: async ({ canvas }) => {
    await expect(canvas.getByText("Plan")).toBeVisible();
    await expect(canvas.getByText("No plan yet — Plan the change records it.")).toBeVisible();
    await expect(canvas.queryByRole("button", { name: "Add task" })).toBeNull();
  },
};

/** The eyebrow act, open — a one-line title and an optional detail. `#897`. */
export const PlanAddTaskOpen: Story = {
  name: "Plan, Add task open",
  render: () => <JobDetailFrom fixture={withPlan(PLAN_PARTWAY)} />,
  play: async ({ canvas, userEvent }) => {
    await userEvent.click(await canvas.findByRole("button", { name: "Add task" }));
    const dialog = within(document.body).getByRole("dialog");
    await waitFor(() => expect(within(dialog).getByLabelText("Title")).toBeVisible());
    await expect(within(dialog).getByLabelText("Detail — optional")).toBeVisible();
    await expect(within(dialog).getByRole("button", { name: "Add task" })).toBeDisabled();
  },
};

/** `Drop…` is offered only on `open` or `working`, never `done`. `#897`. */
export const PlanDropOnRow: Story = {
  name: "Plan, Drop… on a row",
  render: () => <JobDetailFrom fixture={withPlan(PLAN_PARTWAY)} />,
  play: async ({ canvas }) => {
    const found = await canvas.findByText("Add a unit test that does not construct the store");
    const row = found.closest("li");
    if (row === null) throw new Error("the task row was not found");
    await expect(within(row).getByRole("button", { name: "Drop…" })).toBeInTheDocument();
    const done = await canvas.findByText("Extract selectColumnOrder into its own module");
    const doneRow = done.closest("li");
    if (doneRow === null) throw new Error("the done row was not found");
    await expect(within(doneRow).queryByRole("button", { name: "Drop…" })).toBeNull();
  },
};

/** An empty reason cannot be sent, and the field says why. `#897`. */
export const PlanDropReasonEmpty: Story = {
  name: "Plan, Drop reason empty",
  render: () => <JobDetailFrom fixture={withPlan(PLAN_PARTWAY)} />,
  play: async ({ canvas, userEvent }) => {
    const found = await canvas.findByText("Add a unit test that does not construct the store");
    const row = found.closest("li");
    if (row === null) throw new Error("the task row was not found");
    await userEvent.click(within(row).getByRole("button", { name: "Drop…" }));
    // Pristine: a hint, not an error — nothing has been tried yet.
    await expect(within(row).getByText("A reason is needed.")).toHaveAttribute("data-tone", "muted");
    await expect(within(row).getByRole("button", { name: "Drop" })).toBeDisabled();
  },
};

/**
 * The same field, once a submit has been tried empty — the only way there,
 * since Drop itself stays disabled and unclickable while blank. `#897`.
 */
export const PlanDropReasonAttemptedEmpty: Story = {
  name: "Plan, Drop reason tried empty",
  render: () => <JobDetailFrom fixture={withPlan(PLAN_PARTWAY)} />,
  play: async ({ canvas, userEvent }) => {
    const found = await canvas.findByText("Add a unit test that does not construct the store");
    const row = found.closest("li");
    if (row === null) throw new Error("the task row was not found");
    await userEvent.click(within(row).getByRole("button", { name: "Drop…" }));
    await userEvent.click(within(row).getByLabelText("Reason"));
    await userEvent.keyboard("{Enter}");
    await expect(within(row).getByText("A reason is needed.")).toHaveAttribute("data-tone", "error");
  },
};

/** Nothing is connected, so a send refuses and the dialog stays open with what was typed. `#897`. */
export const PlanDropRefused: Story = {
  name: "Plan, Drop refused",
  render: () => <JobDetailFrom fixture={withPlan(PLAN_PARTWAY)} />,
  play: async ({ canvas, userEvent }) => {
    const found = await canvas.findByText("Add a unit test that does not construct the store");
    const row = found.closest("li");
    if (row === null) throw new Error("the task row was not found");
    await userEvent.click(within(row).getByRole("button", { name: "Drop…" }));
    await userEvent.type(within(row).getByLabelText("Reason"), "Already covered elsewhere.");
    await userEvent.click(within(row).getByRole("button", { name: "Drop" }));
    await expect(
      await within(row).findByText("Fleet is not connected. Nothing was sent."),
    ).toBeVisible();
    await expect(within(row).getByLabelText("Reason")).toHaveValue("Already covered elsewhere.");
  },
};

/** Nothing is connected, so a real send refuses and the dialog stays open with what was typed. `#897`. */
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
