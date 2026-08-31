import type { Meta, StoryObj } from "@storybook/react-vite";

import { Button } from "../../primitives/Button/Button";
import { ErrorNotice } from "./ErrorNotice";
import type { DebugPayload } from "./payload";

/**
 * The payload the issue drew, verbatim. A Fleet error, so every guaranteed
 * field is there and the chain is three deep.
 */
const REFUSED: DebugPayload = {
  code: "judge.undecided",
  message: "judge returned prose for criterion 2",
  run_id: "01JQ8ZC4M2WYVK7T3RQN8H",
  job_id: "job_31c7",
  drone_id: "drn_4c8",
  step_id: "verify",
  fields: [
    { key: "criterion", value: "2" },
    { key: "judge_model", value: "claude-sonnet-4-5" },
    { key: "response_bytes", value: "1184" },
  ],
  chain: [
    "judge: no verdict parsed from response",
    "gate verify: undecided",
    "job_31c7: escalated",
  ],
  bridgeProtocol: "5.2",
  fleetProtocol: "5.2",
  at: "2026-08-30T09:16:40Z",
};

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

/**
 * Inline, offered. A row has no room for an expanded view, so the control
 * copies directly and there is no Details beside it — the payload never
 * appears on this surface, it only leaves it.
 */
export const InlineWithPayload: Story = {
  args: { ...(Inline.args as object), payload: REFUSED } as Story["args"],
};

/**
 * A toast, and its one action. Copies and dismisses in the same press, because
 * a toast is often the only sighting an error gets and a person reaching for a
 * second control after the first is how it is missed.
 */
export const ToastWithPayload: Story = {
  render: (args) => (
    <div className="armada-error-toast-region">
      <ErrorNotice {...args} />
    </div>
  ),
  args: {
    kind: "fault",
    placement: "toast",
    code: "judge.undecided",
    message: "Job 31c7 escalated. The judge returned prose for criterion 2.",
    payload: REFUSED,
  },
};

/**
 * A banner, folded. The one placement that offers both, because a standing
 * condition gets read rather than only quoted — Details opens the payload in
 * place and the surface keeps working beneath it.
 */
export const BannerWithPayload: Story = {
  args: { ...(Banner.args as object), payload: REFUSED } as Story["args"],
};

/**
 * Full-surface, and the payload shown rather than offered: nothing else is on
 * the screen, so there is nothing for a disclosure to protect.
 *
 * **What is in the block is byte-for-byte what the control copies.** One
 * producer formats it, and the expanded view renders that string in a `pre`
 * rather than walking the fields a second time — which is what makes "the same
 * order in the clipboard" a property of the code instead of something to keep
 * in step.
 *
 * The sentence under it states the mechanism and stops there. Structured
 * fields are five primitive variants and a credential does not compile into
 * one; the message and the chain are prose an error wrote and nothing bounds
 * those, so the claim does not reach them and does not pretend to.
 */
export const FullSurfaceWithPayload: Story = {
  render: (args) => (
    <div className="armada-error-surface-region">
      <ErrorNotice {...args} />
    </div>
  ),
  args: {
    kind: "fault",
    placement: "surface",
    code: "judge.undecided",
    message: "Job 31c7 escalated. The judge returned prose for criterion 2.",
    act: "Read the judge's response, then overrule the verdict or redispatch the job.",
    payload: REFUSED,
  },
};

/**
 * The payload of a failure that carries no code, which is the case the wire
 * does not describe: a renderer exception never reached Fleet, so there is no
 * `code` and no `run_id` to quote and no Fleet protocol version in the tail.
 *
 * **The `code` row is present and reads `none`.** Every other absent field is
 * absent — that is the rule, and this is its one exception. The payload is
 * read away from the screen it came from, and the treatment guarantees a code
 * on every error, so a reader meeting one with no code row cannot tell whether
 * the failure carried none or whether the paste was cut short. `none` is a
 * fact; nothing here mints a code, because a code's declaration lives beside
 * the variant that raises it.
 *
 * What renders a codeless fault on screen is a separate question and is open.
 * This story is about the artifact, not the notice around it.
 */
export const CodelessPayload: Story = {
  render: (args) => (
    <div className="armada-error-surface-region">
      <ErrorNotice {...args} />
    </div>
  ),
  args: {
    kind: "fault",
    placement: "surface",
    code: "bridge.render.threw",
    message: "Bridge could not draw the job board.",
    act: "Reload Bridge. Fleet keeps running and jobs keep progressing.",
    payload: {
      message: "Cannot read properties of undefined (reading 'status')",
      fields: [
        { key: "region", value: "the job board" },
        { key: "component", value: "JobRowStacked" },
      ],
      chain: [
        "at JobRowStacked (JobRowStacked.tsx:88:19)",
        "at JobBoard (JobBoard.tsx:142:7)",
        "at Shell (Shell.tsx:61:5)",
      ],
      bridgeProtocol: "5.2",
      at: "2026-08-30T09:16:40Z",
    },
  },
};
