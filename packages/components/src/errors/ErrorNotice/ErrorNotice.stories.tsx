import type { Meta, StoryObj } from "@storybook/react-vite";

import { Button } from "../../primitives/Button/Button";
import { ErrorNotice } from "./ErrorNotice";

const meta: Meta<typeof ErrorNotice> = {
  title: "Errors/Error notice",
  component: ErrorNotice,
};
export default meta;

type Story = StoryObj<typeof ErrorNotice>;

/**
 * Inline, and the story the placement rule exists for. Approve-refused is
 * red-serious and affects exactly one row, so it renders in that row and
 * nowhere else — the rows around it keep their height, their badges and their
 * pulse. Blast radius picks the placement; severity would have picked a
 * banner, and a banner here would say a Job Board of fourteen healthy rows is
 * in trouble.
 */
export const Inline: Story = {
  args: {
    kind: "fault",
    placement: "inline",
    code: "fleet.approve.refused",
    message: "Job 12 was not approved. The gate had already closed on step 3.",
    act: "Redispatch to start a fresh attempt from the same brief.",
    fields: [
      { label: "Job", value: "job_2d90bb" },
      { label: "Step", value: "3 of 5" },
      { label: "Fleet run", value: "01J9Z4K7QW" },
    ],
  },
};

/**
 * A toast, and the one placement that may carry no act, because it reports
 * something already over. The copy still names the failure and still carries
 * the code — there is nothing to do, not nothing to know.
 *
 * The region pins it bottom trailing, inset `--space-6`, clear of the status
 * bar. The bar states Fleet's liveness out loud, and a toast that covered it
 * would hide the one thing still guaranteed true.
 */
export const Toast: Story = {
  render: (args) => (
    <div className="armada-error-toast-region">
      <ErrorNotice {...args} />
    </div>
  ),
  args: {
    kind: "fault",
    placement: "toast",
    code: "bridge.clipboard.denied",
    message: "The branch name did not copy. The system refused the clipboard write.",
    fields: [{ label: "Value", value: "auth/session-expiry" }],
  },
};

/**
 * A banner, carrying the failure class that is not red. Fleet is alive and
 * Bridge cannot reach it, so what is on screen is real and is no longer
 * moving — degraded, not a fault. Restarting Fleet is the wrong move here, and
 * a red edge is what would send somebody to do it. The amber dot says the same
 * thing amber says everywhere: this resolves when the wait ends.
 *
 * Persistent, and the surface works normally beneath it.
 */
export const Banner: Story = {
  args: {
    kind: "degraded",
    placement: "banner",
    code: "bridge.fleet.unreachable",
    message:
      "The Job Board stopped updating 4 minutes ago. Fleet is alive on its port and has not answered.",
    act: "Nothing to do yet. Bridge reconnects on its own, and re-reads rather than patching what it has.",
    fields: [
      { label: "Pid", value: "48213" },
      { label: "Port", value: "7681" },
      { label: "Last read", value: "16:42:08" },
    ],
    actions: (
      <Button variant="secondary" size="sm">
        Retry now
      </Button>
    ),
  },
};

/**
 * The same placement carrying the other class, which is what makes the two
 * readable against each other. A fault is red on the wider edge with a red
 * headline and no dot; degraded is neutral on the narrower edge with an amber
 * one. Same shape, same size, same chip geometry — the difference is the edge
 * and the dot, and it has to survive a screenshot.
 */
export const BannerFault: Story = {
  args: {
    kind: "fault",
    placement: "banner",
    code: "fleet.protocol.mismatch",
    message: "No Job dispatched since 14:02. Fleet speaks protocol 3 and Bridge speaks 4.",
    act: "Update Fleet, or run an older Bridge. The v0 routes still list and kill Jobs meanwhile.",
    fields: [
      { label: "Fleet", value: "0.4.1" },
      { label: "Bridge", value: "0.5.0" },
    ],
    actions: (
      <Button variant="secondary" size="sm">
        Open Doctor
      </Button>
    ),
  },
};

/**
 * Full-surface, the one placement that takes the screen, and the only radius
 * wide enough to earn it: nothing on this surface can be read, so there is
 * nothing for the error to sit above. Capped at the wide dialog measure. The
 * copy is the same size as it is in a toast — severity gets the edge and
 * nothing else, and taking the screen is a blast radius rather than a louder
 * failure.
 */
export const FullSurface: Story = {
  render: (args) => (
    <div className="armada-error-surface-region">
      <ErrorNotice {...args} />
    </div>
  ),
  args: {
    kind: "fault",
    placement: "surface",
    code: "bridge.fleet.absent",
    message: "Fleet is not running, so there are no Jobs to show.",
    act: "Start Fleet from a terminal. Bridge cannot start it, and Jobs keep progressing once it is up.",
    fields: [
      { label: "Runtime file", value: "no file at its path" },
      { label: "Bridge run", value: "01J9Z4K7QW" },
    ],
    actions: (
      <Button variant="secondary" size="sm">
        Check again
      </Button>
    ),
  },
};
