import type { Meta, StoryObj } from "@storybook/react-vite";
import { expect, fn } from "storybook/test";

import { VerifyPanel, type VerifyPanelStep } from "./VerifyPanel";

/**
 * Verify on the Manifest surface — Journey 9, *Verify* — on this repository's
 * own `armada.yml`: `bootstrap` and `browsers` are setup, then the seven Checks
 * in the order the file writes them.
 *
 * **Behind its own button, and unhued.** Nothing presses it on opening, and an
 * exit code that is not the one expected is a fact in a chip, never a colour.
 */
const meta: Meta<typeof VerifyPanel> = {
  title: "Compositions/Verify panel",
  component: VerifyPanel,
};
export default meta;

type Story = StoryObj<typeof VerifyPanel>;

function step(group: string, name: string, run: string, state: VerifyPanelStep["state"]): VerifyPanelStep {
  return { id: `${group}:${name}`, group, name, run, state };
}

const ran = (result: string, duration: string): VerifyPanelStep["state"] => ({ kind: "ran", result, duration });
const waiting: VerifyPanelStep["state"] = { kind: "waiting" };

/** **Never automatic**: drawn with nothing pressed, it has run nothing, and one press runs it once. */
export const AtRest: Story = {
  name: "At rest",
  args: { onVerify: fn() },
  play: async ({ args, canvas, userEvent }) => {
    await expect(args.onVerify).not.toHaveBeenCalled();
    await userEvent.click(canvas.getByRole("button", { name: "Verify" }));
    await expect(args.onVerify).toHaveBeenCalledTimes(1);
  },
};

/** Setup has run, `build` is out, and the rest wait their turn. Stop ends the Verify through the step that is out. */
export const Underway: Story = {
  args: {
    steps: [
      step("Setup", "bootstrap", "pnpm install --frozen-lockfile", ran("exit 0 (expects 0)", "8.2s")),
      step("Setup", "browsers", "pnpm -C packages/components exec playwright install chromium --only-shell", ran("exit 0 (expects 0)", "1.4s")),
      step("Checks", "build", "cargo build --workspace --locked", { kind: "running", elapsed: "0:42" }),
      step("Checks", "test", "cargo nextest run --workspace --exclude acceptance", waiting),
      step("Checks", "typecheck", "pnpm typecheck", waiting),
    ],
    onStop: fn(),
  },
  play: async ({ args, canvas, userEvent }) => {
    await expect(canvas.getByRole("button", { name: "Verify" })).toBeDisabled();
    await userEvent.click(canvas.getByRole("button", { name: "Stop" }));
    await expect(args.onStop).toHaveBeenCalledTimes(1);
  },
};

/** Ended, with one Check that exited 2 where 0 was expected — said as a fact, and the rest still ran. */
export const WithAFailure: Story = {
  name: "With a failure",
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

/** Setup exited 1, so no Check ran over that install, and each says why. */
export const SetupFailed: Story = {
  name: "Setup failed",
  args: {
    steps: [
      step("Setup", "bootstrap", "pnpm install --frozen-lockfile", ran("exit 1 (expects 0)", "3.0s")),
      step("Checks", "build", "cargo build --workspace --locked", { kind: "not_run", why: "setup `bootstrap` exited 1" }),
      step("Checks", "typecheck", "pnpm typecheck", { kind: "not_run", why: "setup `bootstrap` exited 1" }),
    ],
    ended: "Ran 1 of 3. 1 ended with a code other than the one it expects. 2 did not run.",
    onVerify: fn(),
    onDismiss: fn(),
  },
};

/** A run a person started is out in the checkout, so Verify waits for it rather than refusing later. */
export const ARunIsOut: Story = {
  name: "A run is out",
  args: { unavailable: "`fmt` is running in this checkout. Verify can start once it ends." },
};
