import type { Meta, StoryObj } from "@storybook/react-vite";
import { expect } from "storybook/test";

import { NarrationPlanBar, WorkNarration, type NarrationBeat } from "./WorkNarration";

/**
 * A step's Working area, by the three shapes #1185 names: a plan mid-task, no
 * plan at all, and a plan whose Drone never marked a task. The rows under each
 * sentence stand in for the activity log's own.
 */
const meta = {
  title: "Compositions/Work narration",
  component: WorkNarration,
  parameters: { layout: "padded" },
} satisfies Meta<typeof WorkNarration>;

export default meta;
type Story = StoryObj<typeof meta>;

/** A line standing in for a log row. */
function row(text: string) {
  return <p style={{ margin: 0, padding: "var(--space-1) var(--space-3)" }}>{text}</p>;
}

const reloaded: NarrationBeat = {
  id: "b1",
  at: "08:28:36",
  said: "Now let's add `EvidenceInbox::reloaded`:",
  meta: "2 calls · Edit",
  body: (
    <>
      {row("Edit  crates/fleet/src/evidence.rs · 31ms")}
      {row("Edit  crates/fleet/src/evidence.rs · 25ms")}
    </>
  ),
};

const wired: NarrationBeat = {
  id: "b2",
  at: "08:28:45",
  said: "Now wire this into `Fleet::assembled` in fittings.rs:",
  meta: "2 calls so far · Edit",
  open: true,
  body: (
    <>
      {row("Edit  crates/fleet/src/daemon/fittings.rs · 31ms")}
      {row("Edit  crates/fleet/src/daemon/fittings.rs · 25ms")}
    </>
  ),
};

/**
 * Two tasks done and folded, the third open with its newest sentence live, and
 * two waiting. **A sentence's calls open with one press**, which the play holds.
 */
export const APlanMidTask: Story = {
  name: "A plan, mid-task",
  args: {
    emptyNote: "Nothing yet",
    sections: [
      {
        id: "T1",
        heading: { task: "T1", mark: "done", title: "Give arriving evidence its own durable row" },
        meta: "31 calls · 5m 02s",
        beats: [{ id: "a1", at: "08:17:02", said: "Start with the table.", meta: "31 calls · Read, Edit", body: row("Read  crates/store/src/lib.rs") }],
      },
      {
        id: "T2",
        heading: { task: "T2", mark: "done", title: "Update every place that empties the inbox" },
        meta: "14 calls · 3m 10s",
        beats: [{ id: "a2", at: "08:22:40", said: "Now the call sites.", meta: "14 calls · Grep, Edit", body: row("Grep  empty_the_inbox") }],
      },
      {
        id: "T3",
        heading: { task: "T3", mark: "working", title: "Reload saved evidence when Fleet starts" },
        meta: "4 calls · 1m 02s",
        open: true,
        beats: [reloaded, wired],
      },
      {
        id: "T4",
        heading: { task: "T4", mark: "open", title: "Rule on reloaded evidence without a live Drone" },
        beats: [],
      },
      {
        id: "T5",
        heading: { task: "T5", mark: "open", title: "Test a restart between submit and settle" },
        beats: [],
      },
    ],
  },
  play: async ({ canvas, userEvent }) => {
    await expect(canvas.getByText("Now wire this into `Fleet::assembled` in fittings.rs:")).toBeVisible();
    const folded = canvas.getByRole("button", { name: "2 calls · Edit" });
    await expect(folded).toHaveAttribute("aria-expanded", "false");
    await userEvent.click(folded);
    await expect(folded).toHaveAttribute("aria-expanded", "true");
    await expect(canvas.getByText(/evidence\.rs · 31ms/)).toBeVisible();
    // A task nobody has worked yet is a line, and nothing to press.
    await expect(canvas.queryByRole("button", { name: /Test a restart/ })).toBeNull();
  },
};

/** No plan on the workflow: the sentences, with no headings and no bar. */
export const NoPlan: Story = {
  name: "A step with no plan",
  args: {
    emptyNote: "Nothing yet",
    sections: [{ id: "outside", beats: [reloaded, wired] }],
  },
  play: async ({ canvas }) => {
    await expect(canvas.queryByText("Outside any task")).toBeNull();
    await expect(canvas.getByText("Now let's add `EvidenceInbox::reloaded`:")).toBeVisible();
  },
};

/**
 * A plan the Drone never marked: its work sits under its own heading, open,
 * and the tasks wait below. Ordinary, never an error.
 */
export const NeverMarked: Story = {
  name: "A Drone that never marks a task",
  args: {
    emptyNote: "Nothing yet",
    sections: [
      { id: "outside", heading: { title: "Outside any task" }, meta: "4 calls", open: true, beats: [reloaded, wired] },
      { id: "T1", heading: { task: "T1", mark: "open", title: "Give arriving evidence its own durable row" }, beats: [] },
      { id: "T2", heading: { task: "T2", mark: "open", title: "Update every place that empties the inbox" }, beats: [] },
    ],
  },
};

/** The bar the Working header carries beside Open the log: done over not dropped. */
export const PlanBar: StoryObj<typeof NarrationPlanBar> = {
  name: "The plan bar",
  render: () => <NarrationPlanBar tasks={["done", "done", "working", "open", "open"]} done={2} />,
  play: async ({ canvas }) => {
    await expect(canvas.getByRole("img", { name: "2 of 5 tasks" })).toBeInTheDocument();
  },
};
