import type { Meta, StoryObj } from "@storybook/react-vite";
import { CircleX, ShieldCheck, ShieldMinus, ShieldX } from "lucide-react";
import { expect } from "storybook/test";
import { CheckRuns, type CheckRun } from "./CheckRuns";

/**
 * One story per state the list has to say: a finished Job's Checks, one of
 * them selected, a Job still running with Checks queued behind the one in
 * flight, and a Check that failed.
 *
 * The glyphs are the `shield-*` family that means gates and Checks, except on
 * the Judge's row, where `circle-*` is the family — a panel's verdict is not a
 * Check result and the silhouette says so before the label is read.
 */
const meta: Meta<typeof CheckRuns> = {
  title: "Compositions/Check runs",
  component: CheckRuns,
};
export default meta;

type Story = StoryObj<typeof CheckRuns>;

const finished: CheckRun[] = [
  {
    id: "chk-suite",
    says: "All 315 tests passed",
    identifier: "check:test_suite",
    output: "output · 2,180 lines",
    result: "exit 0",
    named: "passed",
    icon: ShieldCheck,
  },
  {
    id: "chk-api",
    says: "No exported symbol added or removed",
    identifier: "check:public_api",
    output: "output · 44 lines",
    result: "exit 0",
    named: "passed",
    icon: ShieldCheck,
  },
  {
    id: "chk-bench",
    says: "1.19µs per parse — under the 1.5µs cap",
    identifier: "check:bench",
    output: "output · 96 lines",
    result: "exit 0",
    named: "passed",
    icon: ShieldCheck,
  },
  {
    id: "judge",
    says: "“Behaviour unchanged” refused by 2 of 3 judges",
    identifier: "The Judge — a panel of 3",
    identifierIsAName: true,
    output: "verdicts",
    result: "refused",
    named: "refused",
    icon: CircleX,
  },
];

/**
 * A finished Job's Checks. **Every row carries how much output there is**, so
 * the reading behind `exit 0` is visibly there before anyone presses anything
 * — which is the whole argument for the component.
 */
export const WhatEachCheckCameTo: Story = {
  args: {
    rows: finished,
    label: "Checks",
    note: "tailed from the run log",
  },
};

/**
 * One Check's output open in the viewer. **The selection is the accent, never
 * a verdict colour** — which output is being read is not a result, and a
 * selected row that turned green would say the Check passed twice.
 */
export const OneOutputOpen: Story = {
  args: {
    rows: finished,
    openId: "chk-suite",
    label: "Checks",
  },
};

/**
 * A Job still running. **The queued rows are drawn rather than hidden** — the
 * shape of what is coming is part of reading a running Job, and a list that
 * grew as Checks started would make a Job look like it had fewer gates than it
 * has.
 *
 * A queued row has no output control at all. A disabled one is a target that
 * refuses, which is worse than no target.
 */
export const StillRunning: Story = {
  args: {
    label: "Checks",
    rows: [
      {
        id: "chk-suite",
        says: "1,124 of 1,204 tests run",
        identifier: "check:test_suite",
        output: "output · 1,125 lines",
        result: "running",
        named: "running",
        icon: ShieldMinus,
      },
      {
        id: "chk-budget",
        says: "Waiting for the suite to finish",
        identifier: "check:query_budget",
        result: "queued",
        named: "queued",
      },
      {
        id: "judge",
        says: "Waiting for every check to finish",
        identifier: "The Judge — a panel of 3",
    identifierIsAName: true,
        result: "queued",
        named: "queued",
      },
    ],
    openId: "chk-suite",
  },
};

/**
 * A Check that failed. **The row says what broke rather than the command that
 * broke** — the command is on the rail, and this list exists to be read.
 */
export const ACheckThatFailed: Story = {
  args: {
    label: "Checks",
    rows: [
      {
        id: "chk-suite",
        says: "4 of 315 tests failed in the loose-format parser",
        identifier: "check:test_suite",
        output: "output · 2,204 lines",
        result: "exit 101",
        named: "failed",
        icon: ShieldX,
      },
      {
        id: "chk-api",
        says: "No exported symbol added or removed",
        identifier: "check:public_api",
        output: "output · 44 lines",
        result: "exit 0",
        named: "passed",
        icon: ShieldCheck,
      },
    ],
  },
};

/**
 * Which output the viewer is showing is on the control, not only in the
 * stylesheet.
 *
 * **What earns the assertion is that the selected row is a colour.** Nobody
 * reading the page any other way was told which of four Checks was open, and
 * a queued row's missing control is the other half of the same contract: it
 * is absent, not disabled.
 */
export const TheOpenOutputSaysSo: Story = {
  args: { rows: finished, openId: "chk-bench", label: "Checks" },
  play: async ({ canvas }) => {
    const open = canvas.getByRole("button", { name: "output · 96 lines", pressed: true });
    await expect(open).toBeVisible();

    // Every other row's control is a control, and says it is not the open one.
    const rest = canvas.getAllByRole("button", { pressed: false });
    await expect(rest).toHaveLength(3);
  },
};

/**
 * **A skipped Check's reason, which is a sentence and not a measurement.**
 *
 * `produced` is documented as *the exit code, the signal, the budget it
 * outran* — short, mono, measured. A Check that is skipped because nothing it
 * covers changed carries a whole clause there instead, naming every path
 * pattern it watches, and this row is where that turned up in the app.
 *
 * It broke the row in two ways at once. The result column was `max-content`
 * with `nowrap`, so the sentence sized the grid wider than the pane and drew
 * itself over the finding beside it; and the finding's column, squeezed to
 * nothing, took an identifier set to break `anywhere` down to one character a
 * line — `t`, `y`, `p`, `e`, `c`, `h`, `e`, `c`, `k` straight down the panel.
 *
 * Both halves give way here: the reason wraps against a right edge the short
 * results still line up on, and the name keeps its own width as the column's
 * floor.
 */
export const ASkippedCheckSaysWhy: Story = {
  args: {
    label: "Checks",
    rows: [
      { id: "build", says: "Passed", identifier: "build", named: "passed", output: "implement.1.0.log" },
      { id: "test", says: "Passed", identifier: "test", named: "passed", output: "implement.1.1.log" },
      {
        id: "typecheck",
        says: "Not run",
        identifier: "typecheck",
        named: "queued",
        result:
          "no changed file is under apps/**, packages/**, crates/core-model/domain/**, protocol-version.toml, package.json, pnpm-lock.yaml or pnpm-workspace.yaml",
      },
      {
        id: "bridge_build",
        says: "Not run",
        identifier: "bridge_build",
        named: "queued",
        result:
          "no changed file is under apps/**, packages/**, crates/core-model/domain/**, protocol-version.toml, package.json, pnpm-lock.yaml or pnpm-workspace.yaml",
      },
      { id: "diff_nonempty", says: "Passed", identifier: "diff_nonempty", named: "passed" },
    ],
  },
};
