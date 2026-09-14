import { useState } from "react";
import type { Meta, StoryObj } from "@storybook/react-vite";
import { expect } from "storybook/test";
import { Panel } from "./Panel";

/**
 * The rounded panel Navigation, Stats and Fleet stack inside — Bridge/1088.
 * Open state is the caller's; this story holds it in `useState` the way the
 * left column holds it against a preference.
 */
const meta: Meta<typeof Panel> = {
  title: "Compositions/Panel",
  component: Panel,
};
export default meta;

type Story = StoryObj<typeof Panel>;

function Held(props: { defaultOpen?: boolean; narrow?: boolean }) {
  const [open, setOpen] = useState(props.defaultOpen ?? true);
  return (
    <Panel label="Stats" trailing="6" open={open} onOpenChange={setOpen} narrow={props.narrow}>
      <p>Six things worth a glance.</p>
    </Panel>
  );
}

export const Open: Story = {
  render: () => <Held defaultOpen />,
  play: async ({ canvas, userEvent }) => {
    await expect(canvas.getByText("Six things worth a glance.")).toBeVisible();
    await userEvent.click(canvas.getByRole("button", { expanded: true }));
    await expect(canvas.queryByText("Six things worth a glance.")).toBeNull();
  },
};

export const Collapsed: Story = {
  render: () => <Held defaultOpen={false} />,
  play: async ({ canvas }) => {
    await expect(canvas.getByText("6")).toBeVisible();
    await expect(canvas.queryByText("Six things worth a glance.")).toBeNull();
  },
};

/** Below `--layout-breakpoint`: one dot, and nothing to press. */
export const Narrow: Story = {
  render: () => <Held narrow />,
};
