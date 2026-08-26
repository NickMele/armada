import type { Meta, StoryObj } from "@storybook/react-vite";
import { Stethoscope, TriangleAlert } from "lucide-react";
import { Alert } from "./Alert";

const meta: Meta<typeof Alert> = {
  title: "Primitives/Alert",
  component: Alert,
};
export default meta;

type Story = StoryObj<typeof Alert>;

/**
 * The tone the component sheet draws: a standing condition in the escalation
 * hue, headline first and the facts needed to decide beneath. The glyph is
 * triangle-alert, which the icon registry reserves to Doctor and to generic
 * warnings in toasts; see the report.
 */
export const Escalated: Story = {
  args: {
    tone: "escalated",
    icon: <TriangleAlert size={16} strokeWidth={2} aria-hidden="true" />,
    title: "Fleet lost the api/auth worktree",
    children: "The branch was deleted outside Armada. Two jobs are blocked.",
  },
};

/**
 * The neutral tone, drawn as the Doctor condition strip. A health check is a
 * standing condition, not a queued decision, so it takes no Job hue: a module
 * is not a Job state. It names what is failing and what that costs.
 */
export const Neutral: Story = {
  args: {
    tone: "neutral",
    icon: <Stethoscope size={16} strokeWidth={2} aria-hidden="true" />,
    children: (
      <span>
        Doctor: 1 of 6 modules failing — <span className="mono">toolchain</span>, node 20 missing
        on this machine. Jobs still dispatch.
      </span>
    ),
    action: (
      <button type="button" className="armada-alert__button">
        Open Doctor
      </button>
    ),
  },
};
