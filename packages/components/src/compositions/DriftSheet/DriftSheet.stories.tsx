import type { Meta, StoryObj } from "@storybook/react-vite";
import { expect, fn } from "storybook/test";

import { DriftSheet } from "./DriftSheet";
import type { DriftPanelRow } from "../DriftPanel/DriftPanel";

/**
 * Drift's reading, moved off the top of the Manifest surface onto the layer the
 * run's own patch already uses. `DriftPanel` is unchanged inside it.
 *
 * The sheet is laid out inside the nearest positioned ancestor, so every story
 * draws one: outside a screen there is nothing for it to be flush to.
 */
const meta: Meta<typeof DriftSheet> = {
  title: "Compositions/Drift sheet",
  component: DriftSheet,
  args: { open: true, file: "armada.yml", onClose: fn() },
  decorators: [
    (Story) => (
      <div
        style={{
          position: "relative",
          height: "var(--palette-max-height)",
          background: "var(--bg-base)",
        }}
      >
        <Story />
      </div>
    ),
  ],
};
export default meta;

type Story = StoryObj<typeof DriftSheet>;

const CURRENT: DriftPanelRow[] = [
  { id: "checks.build.run", where: "checks.build.run", run: "cargo build --workspace --locked", verdict: "current" },
  { id: "checks.typecheck.run", where: "checks.typecheck.run", run: "pnpm typecheck", verdict: "current" },
];

const GONE: DriftPanelRow = {
  id: "checks.bridge_test.run",
  where: "checks.bridge_test.run",
  run: "pnpm run bridge-test",
  verdict: "gone",
  missing: ["package.json: scripts.bridge-test"],
};

/**
 * **One reading, not two headings.** The layer says `Drift` and the panel
 * inside it drops its own title, so the first line under the header is the
 * summary rather than the word again.
 */
export const ALineThatIsGone: Story = {
  name: "A line that is gone",
  args: { rows: [CURRENT[0]!, GONE, CURRENT[1]!] },
  /**
   * **The rows came with the reading.** Nothing about a sheet drawn open says
   * whether the props reached the panel inside it — a wrapper that stopped
   * spreading them would render this exact header over an empty body. And the
   * heading is asserted as one, because `titled={false}` is the whole of what
   * this wrapper does to the panel.
   */
  play: async ({ canvas }) => {
    await expect(canvas.getByText("package.json: scripts.bridge-test")).toBeVisible();
    await expect(canvas.getByText(/1 of 3 lines names something/)).toBeVisible();
    await expect(canvas.getAllByText("Drift")).toHaveLength(1);
  },
};

/** Everything still current — the answer this reading usually gives. */
export const EveryLineCurrent: Story = {
  name: "Every line current",
  args: { rows: CURRENT },
};

/** Before the read has answered. Free, so it is already underway on opening. */
export const Reading: Story = {
  args: { note: "Reading whether this checkout still has what armada.yml names." },
};

/** At `--window-floor`: flush to both edges, no file in the subtitle, icon close. */
export const AtTheFloor: Story = {
  args: { rows: [GONE, ...CURRENT], floor: true },
};
