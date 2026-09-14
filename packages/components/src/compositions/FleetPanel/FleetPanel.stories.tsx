import type { Meta, StoryObj } from "@storybook/react-vite";
import { expect } from "storybook/test";
import { FleetPanel } from "./FleetPanel";

/**
 * Fleet — the left column's third panel, replacing the status bar's own
 * reading of the one connection. `Bridge/1088`.
 */
const meta: Meta<typeof FleetPanel> = {
  title: "Compositions/FleetPanel",
  component: FleetPanel,
};
export default meta;

type Story = StoryObj<typeof FleetPanel>;

export const Running: Story = {
  args: {
    state: "running",
    label: "Running",
    detail: "pid 61372 · port 40000",
    meta: "protocol 13.49 · up 2h 14m",
    doctor: { outcome: "pass", checked: "Fleet, SQLite, Manifest, system stats" },
    open: true,
  },
  play: async ({ canvas }) => {
    await expect(canvas.getByText("Running")).toBeVisible();
    await expect(canvas.getByText("pass")).toBeVisible();
  },
};

export const NotRunning: Story = {
  args: { state: "not-running", label: "Not running", detail: "no runtime file at ~/.armada/fleet.json", open: true },
};

export const Unreachable: Story = {
  args: {
    state: "unreachable",
    label: "Unreachable",
    detail: "pid 4417 alive on port 7411 · no answer for 20s",
    open: true,
  },
};

export const DoctorReading: Story = {
  args: {
    state: "running",
    label: "Running",
    detail: "pid 61372 · port 40000",
    doctor: { outcome: "reading", checked: "Fleet, SQLite, Manifest, system stats" },
    open: true,
  },
};

export const Collapsed: Story = {
  args: { state: "running", label: "Running", detail: "pid 61372 · port 40000", open: false },
};

export const Narrow: Story = {
  args: { state: "running", label: "Running", open: true, narrow: true },
};
