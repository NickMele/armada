import type { Meta, StoryObj } from "@storybook/react-vite";
import { expect, within } from "storybook/test";
import { FleetPanel, fleetSaid } from "./FleetPanel";

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

const MACOS_RUNTIME_FILE = "no runtime file at /Users/user/Library/Application Support/Armada/fleet.json";

/**
 * **The whole path, at the left column's narrowest.** The path is what a
 * person acts on and has no space to break at, so it wraps anywhere rather
 * than running past the panel's right edge.
 */
export const NotRunningAtColumnMinimum: Story = {
  args: { state: "not-running", label: "Not running", detail: MACOS_RUNTIME_FILE, open: true },
  decorators: [
    (Story) => (
      <div style={{ width: "var(--sidebar-min)" }}>
        <Story />
      </div>
    ),
  ],
  play: async ({ canvas }) => {
    const sentence = canvas.getByText(MACOS_RUNTIME_FILE);
    await expect(sentence).toBeVisible();
    await expect(sentence.scrollWidth).toBeLessThanOrEqual(sentence.clientWidth);
    const panel = sentence.closest("section");
    await expect(panel).not.toBeNull();
    const text = document.createRange();
    text.selectNodeContents(sentence);
    await expect(text.getBoundingClientRect().right).toBeLessThanOrEqual(panel!.getBoundingClientRect().right);
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

/**
 * The left column at its 48px rail — the whole of Fleet, as one dot. That is
 * every width below `--window-fold-left` with Helm's dock beside the content
 * since 18 Sep 2026, rather than the rare width it used to be.
 *
 * What a rendering cannot show: that the dot is not silent. It was — an
 * `aria-hidden` mark inside a region named "Fleet", so nothing said which
 * state — and the words are `fleetSaid`, the producer the title row's own dot
 * reads, rather than a second sentence composed here.
 */
export const Narrow: Story = {
  args: { state: "running", label: "Running", open: true, narrow: true },
  play: async ({ args, canvas }) => {
    const said = fleetSaid(args.label);
    const dot = canvas.getByRole("img", { name: said });
    await expect(dot).toHaveAttribute("title", said);
  },
};

/** Not running, at the rail: the one thing the dot says is the one thing that changed. */
export const NarrowNotRunning: Story = {
  args: { state: "not-running", label: "Not running", open: true, narrow: true },
  play: Narrow.play,
};

/**
 * Expanded, and unchanged: the state is in words in the panel's own body, so
 * nothing here is named `Fleet — Running`. The rail's name is for the width
 * that has no room for those words, not a second reading beside them.
 */
export const ExpandedSaysItInWords: Story = {
  args: { state: "running", label: "Running", rows: [{ label: "pid", value: "61372" }], open: true },
  play: async ({ canvas }) => {
    await expect(canvas.queryByRole("img", { name: /^Fleet — / })).not.toBeInTheDocument();
    await expect(canvas.getByText("Running")).toBeVisible();
  },
};
