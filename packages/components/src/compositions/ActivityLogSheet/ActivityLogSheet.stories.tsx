import type { Meta, StoryObj } from "@storybook/react-vite";
import { expect, fn } from "storybook/test";
import { ActivityLogSheet } from "./ActivityLogSheet";
import { ActivityLog, type ActivityEntry } from "../ActivityLog/ActivityLog";

/**
 * The activity log on a trailing sheet — Journey 4, frames `4i`, `4l` and `4m`.
 *
 * The sheet is laid out inside the nearest positioned ancestor, so every story
 * here draws one: outside a screen there is nothing for it to be flush to.
 */
const meta: Meta<typeof ActivityLogSheet> = {
  title: "Compositions/Activity log sheet",
  component: ActivityLogSheet,
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

type Story = StoryObj<typeof ActivityLogSheet>;

const ENTRIES: ActivityEntry[] = [
  { id: "1", at: "14:22:07", actor: "armada", summary: "Go on to Implement." },
  {
    id: "2",
    at: "14:22:44",
    actor: "drone",
    summary: "Splitting the selector block into its own module so the tests can import it.",
  },
  { id: "3", at: "14:23:11", actor: "drone", summary: "Read", subject: "packages/settings/src/reducer.ts" },
  {
    id: "4",
    at: "14:29:40",
    actor: "drone",
    summary: "Bash",
    subject: "cargo build --workspace --locked",
    output:
      "$ cargo build --workspace --locked\n   Compiling armada-settings v0.1.0 (packages/settings)\n   Finished `dev` profile [unoptimized] in 47.61s",
    ran: "exit 0 · 47.61s · in .armada/worktrees/job_2d90bb",
  },
  {
    id: "5",
    at: "14:30:28",
    actor: "fleet",
    summary: "Heartbeat — the Drone has been quiet for 48 seconds",
  },
];

/** Live, held at a timestamp, with the count of what arrived while reading. */
export const Held: Story = {
  args: {
    open: true,
    step: "Fix",
    jobId: "job_2d90bb",
    children: <ActivityLog entries={ENTRIES} openId="4" />,
    total: 1676,
    live: true,
    heldAt: "14:31:58",
    arrived: 31,
    onFilter: fn(),
  },
  /**
   * **The count is stated twice, and both have to be there.** The strip is at
   * the top of the sheet and the reader is at the bottom of it, so one of the
   * two is always off screen — which means dropping either is a change nobody
   * looking at the sheet can see.
   *
   * The filters are the caller's. Pressing one reports and moves nothing,
   * because which rows the log holds is the caller's answer and a strip
   * holding its own copy would disagree with the stream it is filtering.
   */
  play: async ({ args, canvas, userEvent }) => {
    await expect(canvas.getByRole("button", { name: /Jump to now/ })).toHaveTextContent("+31");
    await expect(canvas.getByText(/31 entries arrived while you were reading/)).toBeVisible();

    await userEvent.click(canvas.getByRole("tab", { name: "Fleet" }));
    await expect(args.onFilter).toHaveBeenCalledWith("fleet");
    await expect(canvas.getByRole("tab", { selected: true })).toHaveAccessibleName("All");
  },
};

/** At `--window-floor`: flush to both edges, filters in the strip, icon close. */
export const AtTheFloor: Story = {
  args: { ...Held.args, floor: true, jobId: undefined },
};

/** The Job escalated while the sheet was open. The live mark stops with it. */
export const Escalated: Story = {
  args: {
    ...Held.args,
    live: false,
    endedAt: "14:47:11",
    heldAt: undefined,
    arrived: 0,
    escalation: {
      at: "14:47:11",
      because:
        "The suite passed and the Judge refused: the diff widens the catch block rather than " +
        "addressing the cause named in root_cause.md. Three attempts spent.",
    },
    onClose: fn(),
  },
  /**
   * **`Show me` is a labelled second face of `Esc`, not a second binding.**
   * The escalation here names no `onShowMe`, so the control falls through to
   * the sheet's own close — and a fallthrough is the one kind of wiring that
   * cannot be drawn. A control that had quietly stopped falling through would
   * render as this exact band and do nothing when pressed.
   */
  play: async ({ args, canvas, userEvent }) => {
    await userEvent.click(canvas.getByRole("button", { name: /Show me/ }));
    await expect(args.onClose).toHaveBeenCalled();
  },
};
