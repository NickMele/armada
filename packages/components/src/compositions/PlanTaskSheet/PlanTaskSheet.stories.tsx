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
 * A task that names no files and no evidence. **It says so rather than drawing
 * four empty labels** — the fields are optional by design, and a planner who
 * does not know a task's paths yet is not making a mistake.
 */
export const NothingBeyondTheTitle: Story = {
  args: { id: "T7", title: "Cover the four states the definition of done names", state: "open" },
  play: async ({ canvasElement }) => {
    const sheet = within(canvasElement);
    await expect(sheet.getByRole("note")).toHaveTextContent("says nothing beyond its title");
    await expect(sheet.queryByText("Evidence")).toBeNull();
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
