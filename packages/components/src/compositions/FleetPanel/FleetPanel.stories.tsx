import type { Meta, StoryObj } from "@storybook/react-vite";
import { expect, within } from "storybook/test";
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

/**
 * **Each label reads beside its own value, in order, and nothing else is a
 * row.** Read through the list's roles, so a value drawn under the wrong label,
 * or a row left with a label and a blank, fails here rather than only on
 * screen.
 */
async function readsAsRows(canvasElement: HTMLElement, rows: [string, string][]) {
  const canvas = within(canvasElement);
  const labels = canvas.queryAllByRole("term");
  const values = canvas.queryAllByRole("definition");
  await expect(labels.map((label) => label.textContent)).toEqual(rows.map(([label]) => label));
  await expect(values.map((value) => value.textContent)).toEqual(rows.map(([, value]) => value));
  for (const [index, label] of labels.entries()) {
    await expect(label).toBeVisible();
    await expect(values[index]).toBeVisible();
  }
}

export const Running: Story = {
  args: {
    state: "running",
    label: "Running",
    rows: [
      { label: "pid", value: "61372" },
      { label: "port", value: "40000" },
      { label: "protocol", value: "13.49" },
      { label: "up", value: "2h 14m" },
    ],
    doctor: { outcome: "pass", checked: "Fleet, SQLite, Manifest, system stats" },
    open: true,
  },
  play: async ({ canvas, canvasElement }) => {
    await expect(canvas.getByText("Running")).toBeVisible();
    await readsAsRows(canvasElement, [
      ["pid", "61372"],
      ["port", "40000"],
      ["protocol", "13.49"],
      ["up", "2h 14m"],
    ]);
    await expect(canvas.getByText("pass")).toBeVisible();
  },
};

/**
 * **Fleet is newer than this Bridge.** Additive only, so the rows are as a
 * healthy connection draws them and the sentence under them names both
 * versions.
 */
export const FleetAhead: Story = {
  args: {
    state: "running",
    label: "Running",
    rows: [
      { label: "pid", value: "61372" },
      { label: "port", value: "40000" },
      { label: "protocol", value: "13.50" },
      { label: "up", value: "2h 14m" },
    ],
    detail: "Fleet 13.50, Bridge 13.49",
    open: true,
  },
};

/** **No runtime file, so no pid and no port.** A sentence, and not one labelled blank. */
export const NotRunning: Story = {
  args: { state: "not-running", label: "Not running", detail: "no runtime file at ~/.armada/fleet.json", open: true },
  play: async ({ canvas, canvasElement }) => {
    await expect(canvas.getByText("no runtime file at ~/.armada/fleet.json")).toBeVisible();
    await readsAsRows(canvasElement, []);
  },
};

/** **A live pid on a port that does not answer.** Two rows, because a socket that never opened has no protocol or uptime. */
export const Unreachable: Story = {
  args: {
    state: "unreachable",
    label: "Unreachable",
    rows: [
      { label: "pid", value: "4417" },
      { label: "port", value: "7411" },
    ],
    detail: "alive, no answer for 20s · last read 4s ago",
    open: true,
  },
  play: async ({ canvas, canvasElement }) => {
    await readsAsRows(canvasElement, [
      ["pid", "4417"],
      ["port", "7411"],
    ]);
    await expect(canvas.getByText("alive, no answer for 20s · last read 4s ago")).toBeVisible();
  },
};

/** Neither of the three: a neutral dot, and the pid and port the runtime file named. */
export const Connecting: Story = {
  args: {
    state: "unknown",
    label: "Connecting",
    rows: [
      { label: "pid", value: "61372" },
      { label: "port", value: "40000" },
    ],
    open: true,
  },
};

export const DoctorReading: Story = {
  args: {
    state: "running",
    label: "Running",
    rows: [
      { label: "pid", value: "61372" },
      { label: "port", value: "40000" },
    ],
    doctor: { outcome: "reading", checked: "Fleet, SQLite, Manifest, system stats" },
    open: true,
  },
};

export const Collapsed: Story = {
  args: { state: "running", label: "Running", rows: [{ label: "pid", value: "61372" }], open: false },
};

export const Narrow: Story = {
  args: { state: "running", label: "Running", open: true, narrow: true },
};
