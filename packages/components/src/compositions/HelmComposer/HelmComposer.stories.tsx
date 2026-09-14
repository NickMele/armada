import { useState } from "react";
import type { ReactElement } from "react";
import type { Meta, StoryObj } from "@storybook/react-vite";
import { expect, fn, userEvent, within } from "storybook/test";

import { HelmComposer, type HelmRepositoryOption } from "./HelmComposer";

const repositories: HelmRepositoryOption[] = [
  { id: "01M2ARMADA", label: "armada" },
  { id: "01M2SHOP", label: "shop-01" },
];

/** The composer under Helm's thread, drawn at the dock's own width. */
const meta: Meta<typeof HelmComposer> = {
  title: "Compositions/Helm composer",
  component: HelmComposer,
  args: { value: "", onChange: fn(), onSend: fn() },
  render: (args) => (
    <div style={{ width: "var(--w-dock)", background: "var(--bg-sunken)", padding: "var(--space-4)" }}>
      <HelmComposer {...args} />
    </div>
  ),
};
export default meta;

type Story = StoryObj<typeof HelmComposer>;

/** A single repository: no switch to draw, nothing to switch to. */
export const AtRest: Story = {
  args: { current: repositories[0]!.id, repositories: [repositories[0]!], onStartFresh: fn() },
};

/** On All repositories, the dock's own switch — the rail's pick never moves for it. */
export const SwitchOnAll: Story = {
  args: { current: repositories[0]!.id, repositories, onSwitch: fn(), onStartFresh: fn() },
  play: async ({ args, canvasElement }) => {
    const canvas = within(canvasElement);
    await userEvent.selectOptions(canvas.getByRole("combobox"), repositories[1]!.id);
    await expect(args.onSwitch).toHaveBeenCalledWith(repositories[1]!.id);
  },
};

/** A reply is being written — Fleet refuses Start fresh until it finishes. */
export const StartFreshRefusedWhileReplying: Story = {
  args: {
    current: repositories[0]!.id,
    repositories: [repositories[0]!],
    onStartFresh: fn(),
    startFreshDisabled: true,
  },
  play: async ({ canvas }) => {
    await expect(canvas.getByRole("button", { name: "Start fresh" })).toBeDisabled();
  },
};

/** Nothing is servable yet — no repository has a Manifest for Helm to answer for. */
export const NothingToAskYet: Story = { args: { disabled: true } };

function Typed(): ReactElement {
  const [value, setValue] = useState("");
  return (
    <HelmComposer
      current={repositories[0]!.id}
      repositories={[repositories[0]!]}
      value={value}
      onChange={setValue}
      onSend={fn()}
    />
  );
}

/** Blank never sends — the button stays off until there is something to say. */
export const BlankDoesNotSend: Story = {
  render: () => <Typed />,
  play: async ({ canvasElement }) => {
    const canvas = within(canvasElement);
    await expect(canvas.getByRole("button", { name: "Send" })).toBeDisabled();
    await userEvent.type(canvas.getByRole("textbox"), "Why did job 12 stall?");
    await expect(canvas.getByRole("button", { name: "Send" })).toBeEnabled();
  },
};
