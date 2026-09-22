import type { Meta, StoryObj } from "@storybook/react-vite";
import { expect } from "storybook/test";

import { GroupBoundary, type GroupBoundaryCheck } from "./GroupBoundary";

const meta: Meta<typeof GroupBoundary> = {
  title: "Compositions/Group boundary",
  component: GroupBoundary,
  decorators: [
    (Story) => (
      <div style={{ padding: "var(--space-6)", background: "var(--surface-canvas)", maxWidth: "var(--w-step-panel-min)" }}>
        <Story />
      </div>
    ),
  ],
};
export default meta;

type Story = StoryObj<typeof GroupBoundary>;

const NAMES = ["typecheck", "format", "screens_test", "components_test", "bridge_build", "storybook", "acceptance"];

const all = (reads: GroupBoundaryCheck["reads"]): GroupBoundaryCheck[] =>
  NAMES.map((name) => ({ name, reads }));

const NO_CASES = "Fleet does not serve the cases a boundary owes yet, so none is drawn here.";

/** Nothing has reached it: seven segments, no verdict and no commit. */
export const NotRun: Story = {
  args: {
    says: "7 checks will run at this boundary",
    checks: all("not run"),
    testsAbsent: NO_CASES,
  },
};

/** One in flight. The bar is the only thing on this surface that says so. */
export const OneInFlight: Story = {
  args: {
    says: "7 checks are running at this boundary",
    checks: NAMES.map((name, at) => ({ name, reads: at < 3 ? "passed" : at === 3 ? "running" : "not run" })),
    verdictSays: "running now",
    testsAbsent: NO_CASES,
  },
};

/** All passed, with the commit the group left and the cases that ran beside them. */
export const AllPassed: Story = {
  args: {
    says: "7 checks ran at this boundary",
    checks: all("passed"),
    verdictSays: "all 7 passed",
    verdictNamed: "passed",
    commit: "7a2f0c5",
    testsSay: "2 tests ran at this boundary",
    tests: [
      { id: "c-panel", spec: "packages/screens/src/Running.test.tsx", reads: "owed" },
      { id: "c-board", spec: "packages/screens/src/Board.test.tsx", reads: "not covered" },
    ],
    testsAbsent: NO_CASES,
  },
};

/**
 * One failed, and the group stops behind it.
 *
 * **A `play`, because the claim is about what is drawn beside the red.** A
 * boundary that read failed with nothing said is the state this component was
 * built to end — the six that passed are on screen, the retry count is on
 * screen, and what the next Drone is told is the Check's own output rather than
 * a summary of it.
 */
export const OneFailed: Story = {
  args: {
    says: "7 checks ran at this boundary",
    checks: NAMES.map((name) => ({ name, reads: name === "screens_test" ? "failed" : "passed" })),
    verdictSays: "screens_test failed",
    verdictNamed: "failed",
    retrySays: "second run",
    stopsSays: "No task of group 4 starts until this boundary passes.",
    toldNext: "every test in the screens package passes\n1 of 1384 failed: the Drones row opened the Board",
    testsAbsent: NO_CASES,
  },
  play: async ({ canvas }) => {
    const checks = canvas.getByRole("region", { name: "Checks at this boundary" });
    await expect(checks).toHaveTextContent("screens_test");
    await expect(checks).toHaveTextContent("second run");
    await expect(checks).toHaveTextContent("1 of 1384 failed");
    await expect(checks.querySelectorAll('[data-reads="passed"]')).toHaveLength(6);
    // The tests stay their own region, so a case is never read as a Check.
    await expect(canvas.getByRole("region", { name: "Tests at this boundary" })).toHaveTextContent(
      "does not serve the cases",
    );
  },
};
