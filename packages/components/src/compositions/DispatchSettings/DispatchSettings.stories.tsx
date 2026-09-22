import type { Meta, StoryObj } from "@storybook/react-vite";
import type { WorkflowSummary } from "@armada/protocol";
import { expect, fn, userEvent, within } from "storybook/test";
import { DispatchSettings } from "./DispatchSettings";

/**
 * Closed with nothing set is what a person opens the form on. The other two
 * states are the block open on nothing, and open on four things somebody
 * chose — the difference the head has to carry while it is shut.
 */
const meta: Meta<typeof DispatchSettings> = {
  title: "Compositions/Dispatch settings",
  component: DispatchSettings,
};
export default meta;

type Story = StoryObj<typeof DispatchSettings>;

function workflow(id: string, steps: number): WorkflowSummary {
  return {
    id,
    name: id,
    version: 1,
    manifest_id: "01M1CNPKTV0018H2M1CXDNBK06",
    steps: Array.from({ length: steps }, (_, at) => ({
      step_id: `s${at + 1}`,
      label: `Step ${at + 1}`,
      checks: [],
      judge_checks: [],
      advance_gate: "auto_if_judge_passes",
      delivers: at === steps - 1,
    })),
  };
}

const HELD = {
  workflows: [workflow("feature", 4), workflow("bug", 3)],
  models: ["haiku", "sonnet", "opus"],
  machineCap: 4,
  onOpenChange: () => {},
  onSettings: () => {},
};

/** Shut, and saying that nothing inside it has been touched. */
export const ShutAndUnset: Story = {
  args: { ...HELD, open: false, settings: {} },
};

/** Open on four decisions nobody has taken. Every field names who takes it. */
export const OpenAndUnset: Story = {
  args: { ...HELD, open: true, settings: {} },
};

/** Open on four a person set, including a Job cap under the machine's own. */
export const EverythingSet: Story = {
  args: {
    ...HELD,
    open: true,
    settings: {
      workflowId: "feature",
      tiers: { difficult: "opus", medium: "sonnet", easy: null },
      droneCap: 2,
      lands: "you_at_review",
    },
  },
};

/**
 * The block is controlled, so pressing its head reports and opens nothing on
 * its own. A body that appeared here would be a second copy of the open state
 * that the caller never agreed to.
 */
export const PressingTheHeadOnlyReports: Story = {
  args: { ...HELD, open: false, settings: {}, onOpenChange: fn() },
  play: async ({ args, canvasElement }) => {
    const block = within(canvasElement);
    await userEvent.click(block.getByRole("button", { name: /Settings/ }));
    await expect(args.onOpenChange).toHaveBeenCalledWith(true);
    await expect(block.queryByLabelText("Workflow")).toBeNull();
  },
};
