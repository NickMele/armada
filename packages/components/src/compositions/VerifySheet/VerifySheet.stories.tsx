import type { Meta, StoryObj } from "@storybook/react-vite";
import { expect, fn } from "storybook/test";

import { VerifySheet } from "./VerifySheet";
import type { VerifyPanelStep } from "../VerifyPanel/VerifyPanel";

/**
 * Verify's run, moved off the top of the Manifest surface onto a layer.
 * `VerifyPanel` is unchanged inside it, its own button included.
 *
 * The sheet is laid out inside the nearest positioned ancestor, so every story
 * draws one: outside a screen there is nothing for it to be flush to.
 */
const meta: Meta<typeof VerifySheet> = {
  title: "Compositions/Verify sheet",
  component: VerifySheet,
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

type Story = StoryObj<typeof VerifySheet>;

function step(group: string, name: string, run: string, state: VerifyPanelStep["state"]): VerifyPanelStep {
  return { id: `${group}:${name}`, group, name, run, state };
}

const ran = (result: string, duration: string): VerifyPanelStep["state"] => ({ kind: "ran", result, duration });

/**
 * **Opening the layer presses nothing.** Journey 9's rule is that Verify runs
 * behind its own button, and moving the panel onto a sheet must not turn
 * arriving at the reading into starting a run.
 */
export const OpeningRunsNothing: Story = {
  name: "Opening runs nothing",
  args: { onVerify: fn() },
  play: async ({ args, canvas, userEvent }) => {
    await expect(args.onVerify).not.toHaveBeenCalled();
    await userEvent.click(canvas.getByRole("button", { name: "Verify" }));
    await expect(args.onVerify).toHaveBeenCalledTimes(1);
    // The layer carries the one heading; the panel's own title is off.
    await expect(canvas.getAllByRole("heading", { name: "Verify" })).toHaveLength(1);
  },
};

/** The last run, still readable after it ended — which is why this is a sheet and not a toast. */
export const AnEndedRun: Story = {
  name: "An ended run",
  args: {
    steps: [
      step("Setup", "bootstrap", "pnpm install --frozen-lockfile", ran("exit 0 (expects 0)", "8.2s")),
      step("Checks", "build", "cargo build --workspace --locked", ran("exit 0 (expects 0)", "41.0s")),
      step("Checks", "typecheck", "pnpm typecheck", ran("exit 2 (expects 0)", "9.4s")),
      step("Checks", "format", "cargo fmt --all --check", ran("exit 0 (expects 0)", "1.8s")),
    ],
    ended: "Ran 4 of 4. 1 ended with a code other than the one it expects.",
    onVerify: fn(),
    onDismiss: fn(),
  },
};

/** Out in the checkout: setup has run, `build` is going, the rest wait. */
export const Underway: Story = {
  args: {
    steps: [
      step("Setup", "bootstrap", "pnpm install --frozen-lockfile", ran("exit 0 (expects 0)", "8.2s")),
      step("Checks", "build", "cargo build --workspace --locked", { kind: "running", elapsed: "0:42" }),
      step("Checks", "test", "cargo nextest run --workspace --exclude acceptance", { kind: "waiting" }),
    ],
    onStop: fn(),
  },
};

/** At `--window-floor`: flush to both edges, no file in the subtitle, icon close. */
export const AtTheFloor: Story = {
  args: { onVerify: fn(), floor: true },
};
