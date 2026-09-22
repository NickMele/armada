import type { Meta, StoryObj } from "@storybook/react-vite";
import { expect, fn, userEvent, within } from "storybook/test";

import { PlanTaskSheet } from "./PlanTaskSheet";

/**
 * One plan task's whole reading, moved off the 380px rail onto the layer that
 * can hold it. The content below is a real plan Fleet recorded — seven tasks
 * for *Show what's running in the Drones stat* — because the case this exists
 * for is a long one: 613 characters of note, ten paths and 237 of evidence,
 * under a 75-character title.
 *
 * The sheet lays out inside the nearest positioned ancestor, so every story
 * draws one: outside a screen there is nothing for it to be flush to.
 */
const meta: Meta<typeof PlanTaskSheet> = {
  title: "Compositions/Plan task sheet",
  component: PlanTaskSheet,
  args: { open: true, onClose: fn() },
  decorators: [
    (Story) => (
      <div
        style={{
          position: "relative",
          height: "var(--palette-max-height)",
          background: "var(--bg-base)",
        }}
      >
        <Story />
      </div>
    ),
  ],
};
export default meta;

type Story = StoryObj<typeof PlanTaskSheet>;

const LONG = {
  id: "T1",
  title: "Fleet: one read of every Drone, running Check and Judge call on the machine",
  note: "New query (working name `get_activity`) answers `drones: Vec<DroneSummary>`, reused whole from `list_drones`, plus one row per Job whose gate is running Checks — from Underway's existing `gates` map, never a per-Job re-read — and one row per Job with a Judge call out, from Aloft's existing map.",
  scope: [
    "crates/ipc/operations.toml",
    "crates/ipc/src/drones.rs",
    "crates/ipc/src/lib.rs",
    "crates/fleet/src/underway.rs",
    "crates/fleet/src/judging.rs",
    "crates/fleet/src/judging/marking.rs",
    "crates/fleet/src/serving.rs",
    "crates/api/src/fleetwide.rs",
    "crates/api/src/daemon/queries.rs",
    "crates/api/src/routes/served.rs",
  ],
  expects:
    "A Fleet-side test driving a Job mid-Check and a Job mid-Judge-call, asserting the new query's answer carries both rows plus the Drone roster, in one call.",
};

/** The case the sheet exists for: everything a task can carry, at once. */
export const Working: Story = {
  args: { ...LONG, state: "working" },
};

/**
 * **The two ends of the evidence read side by side, and nothing reconciles
 * them.** The plan named one artifact and the work proved it with another;
 * that is the useful answer, not a wrong one.
 */
export const Done: Story = {
  args: {
    ...LONG,
    state: "done",
    shown: "left-column.test.ts asserts the reading, not the Fleet-side test the plan named.",
  },
  play: async ({ canvasElement }) => {
    const sheet = within(canvasElement);
    await expect(sheet.getByText("Expects")).toBeVisible();
    await expect(sheet.getByText("Shown")).toBeVisible();
  },
};

/** A dropped task keeps its reason, and the reason leads. */
export const Dropped: Story = {
  args: {
    id: "T4",
    title: "Carry the reword everywhere else it is spelled",
    state: "dropped",
    reason: "SettingsSurface.tsx and Board.tsx were deleted in #1236 and #1235.",
    scope: ["packages/components/src/compositions/StatsPanel/StatsPanel.stories.tsx"],
    expects: 'StatsPanel.stories.tsx reads "1 running · 2 max".',
  },
};

/**
 * A task that names no files and no evidence. **The three fields a Drone is
 * dispatched on are still drawn**, because absent is an answer there: what it
 * will be told, what it runs beside and what covers it are the reading a
 * person approves a plan by, and a blank where one should be is the finding.
 */
export const NothingBeyondTheTitle: Story = {
  args: { id: "T7", title: "Cover the four states the definition of done names", state: "open" },
  play: async ({ canvasElement }) => {
    const sheet = within(canvasElement);
    await expect(sheet.getByText("Its title and the files below, and nothing else.")).toBeVisible();
    await expect(sheet.getByText("Nothing. It runs on its own.")).toBeVisible();
    await expect(sheet.getByText("Nothing covers this task.")).toBeVisible();
    await expect(sheet.queryByText("What the planner holds it to")).toBeNull();
  },
};

/**
 * Everything the split added: the tier the planner gave it, the agent it gets,
 * what it runs beside, and a case with no spec that reads **not covered**
 * rather than green. `#1535`.
 */
export const ItsOwnAgent: Story = {
  args: {
    id: "T6",
    title: "Open a Drone's Job from its row",
    state: "failed",
    scope: ["packages/screens/src/running-rows.tsx"],
    tier: "medium",
    model: "sonnet",
    runBy: "its own agent",
    beside: ["T5"],
    failedReason: "The row's press opened the Board rather than the Job",
    tests: [
      { id: "c-panel", spec: "packages/screens/src/Running.test.tsx", reads: "owed" },
      { id: "c-board", spec: "packages/screens/src/Board.test.tsx", reads: "not covered" },
    ],
  },
  play: async ({ canvasElement }) => {
    const sheet = within(canvasElement);
    await expect(sheet.getByText("medium · sonnet · its own agent")).toBeVisible();
    await expect(sheet.getByText("T5")).toBeVisible();
    await expect(sheet.getByText("not covered")).toBeVisible();
  },
};

/**
 * A task done with nothing recorded against it reads differently from one
 * still working: **"Nothing was recorded" is a gap and "Not yet" is a wait**,
 * and a reader acts on those differently.
 */
export const DoneAndNothingShown: Story = {
  args: { ...LONG, state: "done" },
  play: async ({ canvasElement }) => {
    const sheet = within(canvasElement);
    await expect(sheet.getByText("Nothing was recorded.")).toBeVisible();
  },
};

/** The labelled control is one of the sheet's two exits. */
export const Closes: Story = {
  args: { ...LONG, state: "working" },
  play: async ({ args, canvasElement }) => {
    const sheet = within(canvasElement);
    await userEvent.click(sheet.getByRole("button", { name: /close/i }));
    await expect(args.onClose).toHaveBeenCalled();
  },
};

/**
 * **The reading the fields exist for.** The plan named ten files; the work
 * reached eight of them, never opened two, and changed one nobody planned.
 * Neither half is an error — `crates/fleet/src/scope.rs` refuses to fail a
 * step for either — so both read as a quiet note rather than a warning.
 */
export const AgainstWhatItTouched: Story = {
  args: {
    ...LONG,
    state: "done",
    shown: "The Fleet-side test drives a Job mid-Check and one mid-Judge-call.",
    touched: {
      declared: LONG.scope.map((path, at) => ({ path, touched: at > 1 })),
      unplanned: ["crates/ipc/src/activity.rs"],
    },
  },
  play: async ({ canvasElement }) => {
    const sheet = within(canvasElement);
    await expect(sheet.getByText("Files · declared 10 · touched 8 · unplanned 1")).toBeVisible();
    await expect(sheet.getAllByText("not touched")).toHaveLength(2);
    await expect(sheet.getByText("crates/ipc/src/activity.rs")).toBeVisible();
  },
};

/**
 * A task whose work reached everything it named. **The count says so and no
 * row is marked** — a badge on every row would say nothing at all.
 */
export const EverythingItNamed: Story = {
  args: {
    id: "T3",
    title: "Reword the Drones stat itself",
    state: "done",
    scope: ["packages/screens/src/overview.ts", "packages/screens/src/overview.test.ts"],
    touched: {
      declared: [
        { path: "packages/screens/src/overview.ts", touched: true },
        { path: "packages/screens/src/overview.test.ts", touched: true },
      ],
      unplanned: [],
    },
  },
  play: async ({ canvasElement }) => {
    const sheet = within(canvasElement);
    await expect(sheet.getByText("Files · declared 2 · touched 2")).toBeVisible();
    await expect(sheet.queryByText("not touched")).toBeNull();
  },
};

/**
 * The Job's turns were never read, so there is nothing to compare against.
 * **It draws the declared list plainly** rather than a comparison claiming
 * every file went untouched, which is what an absent reading would become if
 * it were treated as an empty one.
 */
export const NotReadAgainstAnything: Story = {
  args: { ...LONG, state: "working" },
  play: async ({ canvasElement }) => {
    const sheet = within(canvasElement);
    await expect(sheet.getByText("Files · 10")).toBeVisible();
    await expect(sheet.queryByText("not touched")).toBeNull();
  },
};
