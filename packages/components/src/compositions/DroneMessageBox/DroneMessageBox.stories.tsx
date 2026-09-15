import { useState } from "react";
import type { ReactElement } from "react";
import type { Meta, StoryObj } from "@storybook/react-vite";
import { expect, fn, userEvent, within } from "storybook/test";

import { DroneMessageBox } from "./DroneMessageBox";

/** The box, drawn at the width a step's chapter or the log sheet gives it. */
const meta: Meta<typeof DroneMessageBox> = {
  title: "Compositions/Drone message box",
  component: DroneMessageBox,
  args: { value: "", onChange: fn(), onSend: fn() },
  render: (args) => (
    <div style={{ width: "480px", background: "var(--bg-sunken)", padding: "var(--space-4)" }}>
      <DroneMessageBox {...args} />
    </div>
  ),
};
export default meta;

type Story = StoryObj<typeof DroneMessageBox>;

function Typed(): ReactElement {
  const [value, setValue] = useState("");
  return (
    <DroneMessageBox value={value} onChange={setValue} onSend={fn()} />
  );
}

/** A drone is working the step: the field takes a message and Send lights up once there is one. */
export const EnabledWhileADroneWorks: Story = {
  render: () => <Typed />,
  play: async ({ canvasElement }) => {
    const canvas = within(canvasElement);
    await expect(canvas.getByRole("button", { name: "Send" })).toBeDisabled();
    await userEvent.type(canvas.getByRole("textbox"), "Check the second failing test too");
    await expect(canvas.getByRole("button", { name: "Send" })).toBeEnabled();
  },
};

/** Queued, between steps, or finished — no drone is on the step to send to. */
export const DisabledWithNoDrone: Story = {
  args: { disabled: true, disabledReason: "No drone is on this step, so there's nothing to send this to." },
  play: async ({ canvas }) => {
    await expect(canvas.getByRole("textbox")).toBeDisabled();
    await expect(canvas.getByRole("button", { name: "Send" })).toBeDisabled();
    await expect(
      canvas.getByText("No drone is on this step, so there's nothing to send this to."),
    ).toBeInTheDocument();
  },
};

/** A redirect is already out. The field stays open — sending again replaces it, as it does today. */
export const RedirectAlreadyWaiting: Story = {
  args: {
    waiting:
      "Sent, waiting for the drone. The instruction went into its session at 2:41pm, and nothing " +
      "here moves when it lands — this job was never held, and it goes on working either way. " +
      "Redirecting again replaces what is outstanding.",
  },
  play: async ({ canvas }) => {
    await expect(canvas.getByRole("textbox")).toBeEnabled();
    await expect(canvas.getByText(/Sent, waiting for the drone/)).toBeInTheDocument();
  },
};
