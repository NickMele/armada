import type { Meta, StoryObj } from "@storybook/react-vite";
import { expect } from "storybook/test";

import { DriftPanel, type DriftPanelRow } from "./DriftPanel";

/**
 * Drift on the Manifest surface — Journey 9, *Verify* — drawn from lines in
 * this repository's own `armada.yml`, in the shape `GET /manifest/drift` reads
 * into rows.
 *
 * **The two verdicts are words in amber, and no row offers anything to
 * press.** A drifted file is behind rather than broken, and fixing it is the
 * file's edit, where the consequence is stated.
 */
const meta: Meta<typeof DriftPanel> = {
  title: "Compositions/Drift panel",
  component: DriftPanel,
};
export default meta;

type Story = StoryObj<typeof DriftPanel>;

const CURRENT: DriftPanelRow[] = [
  {
    id: "checks.build.run",
    where: "checks.build.run",
    run: "cargo build --workspace --locked",
    verdict: "current",
  },
  {
    id: "checks.typecheck.run",
    where: "checks.typecheck.run",
    run: "pnpm typecheck",
    verdict: "current",
  },
  {
    id: "commands.storybook_dev.ready",
    where: "commands.storybook_dev.ready",
    run: "curl -sf http://localhost:41207",
    verdict: "current",
    unfollowed: [{ word: "curl", why: "a program on this machine, not a path in the repository" }],
  },
];

const GONE: DriftPanelRow = {
  id: "checks.bridge_test.run",
  where: "checks.bridge_test.run",
  run: "pnpm bridge-test",
  verdict: "gone",
  missing: ["package.json: scripts.bridge-test"],
};

/**
 * **A Check whose script the repository no longer has.** `gone`, in amber,
 * naming the file and the key it lacks — and the only thing to press is the
 * fold that shows the lines still current.
 */
export const ALineThatIsGone: Story = {
  name: "A line that is gone",
  args: { rows: [CURRENT[0]!, GONE, CURRENT[1]!, CURRENT[2]!] },
  play: async ({ canvas }) => {
    await expect(canvas.getByText("gone")).toBeVisible();
    await expect(canvas.getByText("package.json: scripts.bridge-test")).toBeVisible();
    await expect(canvas.getByText(/1 of 4 lines names something/)).toBeVisible();
    // Reports, never fixes: the fold is the one button in the panel.
    const pressable = canvas.getAllByRole("button").map((button) => button.textContent);
    await expect(pressable).toEqual(["Show the 3 lines still current"]);
    await expect(canvas.queryByText("pnpm typecheck")).toBeNull();
  },
};

/** Every line still names something the checkout has — and the panel says what that does not cover. */
export const EveryLineCurrent: Story = {
  name: "Every line current",
  args: { rows: CURRENT },
  play: async ({ canvas, userEvent }) => {
    await expect(canvas.getByText(/None of the 3 lines names/)).toBeVisible();
    await expect(canvas.getByText(/says nothing about policy, permissions or budgets/)).toBeVisible();
    await userEvent.click(canvas.getByRole("button", { name: "Show the 3 lines still current" }));
    await expect(canvas.getAllByText("current")).toHaveLength(3);
    await expect(canvas.getByText("curl")).toBeVisible();
  },
};

/** Before the read has answered. Free, so it is already underway on opening. */
export const Reading: Story = {
  args: { note: "Reading whether this checkout still has what armada.yml names." },
};
