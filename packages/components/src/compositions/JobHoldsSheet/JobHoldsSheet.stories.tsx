import type { Meta, StoryObj } from "@storybook/react-vite";
import { expect, fn } from "storybook/test";
import type { JobExamined, JobResources as Held } from "@armada/protocol";

import { JobHoldsSheet } from "./JobHoldsSheet";

/**
 * The full reading, moved from the top of the run column onto the layer the log
 * and the patch already use. `JobResources` is unchanged inside it.
 *
 * The sheet is laid out inside the nearest positioned ancestor, so every story
 * draws one: outside a screen there is nothing for it to be flush to.
 */
const meta: Meta<typeof JobHoldsSheet> = {
  title: "Compositions/Job holds sheet",
  component: JobHoldsSheet,
  args: { open: true, jobId: "job_2d90bb", onExamine: fn(), onClose: fn() },
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

type Story = StoryObj<typeof JobHoldsSheet>;

const READING: Held = {
  job_id: "job_2d90bb",
  read_at: "2026-09-04T09:16:52.402Z",
  held: "running",
  processes: [
    {
      pid: 41233,
      command: "node",
      cpu_percent: 8.2,
      memory_bytes: 402_653_184,
      running_for: "06:11",
      recorded: true,
    },
    {
      pid: 41287,
      command: "cargo",
      cpu_percent: 61.4,
      memory_bytes: 268_435_456,
      running_for: "00:12",
      recorded: false,
    },
  ],
  worktree: {
    path: "/Users/user/armada/.armada/worktrees/job_2d90bb",
    branch: "fix/settings-split-selectors",
    bytes: 1_288_490_188,
  },
  wrote_last_at: "2026-09-04T09:16:44.100Z",
};

const EXAMINED: JobExamined = {
  job_id: "job_2d90bb",
  looked_at: "2026-09-04T09:16:52.402Z",
  found: "working",
  looks: [
    {
      asked: "process",
      found: "working",
      said: "the process Fleet recorded is running",
      fields: [{ name: "pid", value: "41233" }],
    },
    { asked: "worktree", found: "working", said: "the worktree is on disk" },
    { asked: "span", found: "working", said: "waiting for the step to finish" },
  ],
  resources: READING,
};

/**
 * **The whole reading, and the act that goes and looks.** `Look now` is here
 * rather than on the summary because it acts on this reading — the summary's
 * one control is the one that opened this.
 */
export const TheFullReading: Story = {
  args: { reading: READING, age: "3s", examined: EXAMINED },
  /**
   * **The act came with the reading.** Nothing about a sheet drawn open says
   * whether the props reached the card inside it: a wrapper that stopped
   * spreading them would render this exact header over an empty body, and the
   * control is the one prop the summary deliberately does not carry.
   */
  play: async ({ canvas }) => {
    await expect(canvas.getByRole("button", { name: /Look now/ })).toBeVisible();
    await expect(canvas.getByText(/This job is doing what it should be/)).toBeVisible();
  },
};

/**
 * **Nothing to ask, and the act is withdrawn.** Every state `JobResources`
 * holds came with it to this layer — this is the one that proves the move was a
 * new home rather than an edit.
 */
export const NothingToAsk: Story = {
  args: { reading: null, examined: null, nothingToAsk: "no_answer" },
};

/** At `--window-floor`: flush to both edges, no Job id, icon close. */
export const AtTheFloor: Story = {
  args: { reading: READING, age: "3s", examined: EXAMINED, floor: true },
};
