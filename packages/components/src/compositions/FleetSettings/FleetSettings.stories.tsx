import type { Meta, StoryObj } from "@storybook/react-vite";
import { expect, fn } from "storybook/test";

import { FleetSettings, type FleetSettingsRow } from "./FleetSettings";

/**
 * Fleet's four limits, on the layer Job settings already draws
 * from. Window-fixed rather than `contained`: it opens from the status bar and
 * the rail, not from one screen — every story draws a positioned ancestor
 * anyway, so the geometry is checkable without a real window behind it.
 */
const meta: Meta<typeof FleetSettings> = {
  title: "Compositions/Fleet settings",
  component: FleetSettings,
  args: { open: true, onClose: fn() },
  decorators: [
    (Story) => (
      <div style={{ position: "relative", height: "100vh", background: "var(--bg-base)" }}>
        <Story />
      </div>
    ),
  ],
};
export default meta;

type Story = StoryObj<typeof FleetSettings>;

const SHIPPED: FleetSettingsRow = {
  value: 2,
  shipped: 2,
  min: 1,
  max: 8,
  onSave: fn(),
};

/** Every limit exactly as Armada ships it. Nothing to go back to. */
export const AtRest: Story = {
  name: "At rest",
  args: {
    concurrency: { ...SHIPPED, onSave: fn() },
    memorySparePercent: { value: 15, shipped: 15, min: 0, max: 50, unit: "%", onSave: fn() },
    diskFloorGib: { value: 10, shipped: 10, min: 0, max: 100, unit: "GiB", onSave: fn() },
    checksAtOnce: { value: 4, shipped: 4, min: 1, max: 8, onSave: fn() },
  },
  /**
   * **What goes to Fleet is the row's own number, never a string it typed
   * over.** Broken once by sending the field's raw text, which carried a
   * trailing space Fleet read as not a number at all.
   */
  play: async ({ args, canvas, userEvent }) => {
    const field = canvas.getByLabelText("Drones at once");
    await userEvent.clear(field);
    await userEvent.type(field, "4");
    await userEvent.click(canvas.getByRole("button", { name: "Save Drones at once" }));
    await expect(args.concurrency.onSave).toHaveBeenCalledWith(4);
  },
};

/**
 * **One changed from what Armada ships, and the way back to it stands beside
 * the figure.** The other two stay quiet — nothing to go back to.
 */
export const ChangedFromShipped: Story = {
  name: "Changed from shipped",
  args: {
    concurrency: {
      value: 4,
      shipped: 2,
      min: 1,
      max: 8,
      onSave: fn(),
      said: "Changed. Applies the next time a job is ready to start.",
    },
    memorySparePercent: { value: 15, shipped: 15, min: 0, max: 50, unit: "%", onSave: fn() },
    diskFloorGib: { value: 10, shipped: 10, min: 0, max: 100, unit: "GiB", onSave: fn() },
    checksAtOnce: { value: 4, shipped: 4, min: 1, max: 8, onSave: fn() },
  },
  play: async ({ args, canvas, userEvent }) => {
    await userEvent.click(
      canvas.getByRole("button", { name: "Use the shipped value for Drones at once" }),
    );
    await expect(args.concurrency.onSave).toHaveBeenCalledWith(2);
  },
};

/** A figure outside the row's own bound is refused before anything is sent. */
export const OutOfRange: Story = {
  name: "Refused in the field",
  args: {
    concurrency: { ...SHIPPED, onSave: fn() },
    memorySparePercent: { value: 15, shipped: 15, min: 0, max: 50, unit: "%", onSave: fn() },
    diskFloorGib: { value: 10, shipped: 10, min: 0, max: 100, unit: "GiB", onSave: fn() },
    checksAtOnce: { value: 4, shipped: 4, min: 1, max: 8, onSave: fn() },
  },
  play: async ({ args, canvas, userEvent }) => {
    const field = canvas.getByLabelText("Memory to keep free");
    await userEvent.clear(field);
    await userEvent.type(field, "90");
    await expect(canvas.getByText("Between 0 and 50.")).toBeInTheDocument();
    await expect(canvas.getByRole("button", { name: "Save Memory to keep free" })).toBeDisabled();
    await expect(args.memorySparePercent.onSave).not.toHaveBeenCalled();
  },
};

/**
 * Fleet's own refusal, read exactly, where the field's own bound did not catch
 * it: a Fleet whose range is narrower than this Bridge's. The field keeps the
 * value still in force, and the wording is `ipc::Within`'s.
 */
export const RefusedByFleet: Story = {
  name: "Refused by Fleet",
  args: {
    concurrency: { ...SHIPPED, onSave: fn() },
    memorySparePercent: {
      value: 15,
      shipped: 15,
      min: 0,
      max: 50,
      unit: "%",
      onSave: fn(),
      refused: "memory_spare_percent: 45 is outside 0 to 40",
    },
    diskFloorGib: { value: 10, shipped: 10, min: 0, max: 100, unit: "GiB", onSave: fn() },
    checksAtOnce: { value: 4, shipped: 4, min: 1, max: 8, onSave: fn() },
  },
};

/** Fleet is not connected, so every row is off and the lead says why. */
export const ControlsOff: Story = {
  name: "Controls off",
  args: {
    concurrency: { ...SHIPPED, onSave: fn() },
    memorySparePercent: { value: 15, shipped: 15, min: 0, max: 50, unit: "%", onSave: fn() },
    diskFloorGib: { value: 10, shipped: 10, min: 0, max: 100, unit: "GiB", onSave: fn() },
    checksAtOnce: { value: 4, shipped: 4, min: 1, max: 8, onSave: fn() },
    disabled: true,
    disabledNote: "Fleet is not connected, so nothing can be changed.",
  },
};
