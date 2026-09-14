import type { Meta, StoryObj } from "@storybook/react-vite";
import { expect, fn } from "storybook/test";

import { GAMING_PATTERN_MEANING } from "../../generated/vocabulary";
import { HeldFlag, type HeldFinding } from "./HeldFlag";

const meta: Meta<typeof HeldFlag> = {
  title: "Compositions/Held flag",
  component: HeldFlag,
};
export default meta;

type Story = StoryObj<typeof HeldFlag>;

/** The registry's own words, never retyped here. */
function meaning(pattern: string): Pick<HeldFinding, "headline" | "explanation"> {
  const said = GAMING_PATTERN_MEANING[pattern];
  return said === undefined ? {} : { headline: said.headline, explanation: said.explanation };
}

const TEST_FILE = "packages/settings/test/useColumnSelectors.test.ts";

const ASKED =
  "Does this change alter an existing assertion so that it asserts less than it did, and is " +
  "that assertion made nowhere else in this change?";

const BRIEF = ".armada/briefs/77-split-the-settings-reducer/implement.1.gaming.assertion_weakened.txt";

/** The presets the owner chose for *No, the work is fine*. */
const PRESETS = ["It's not a test", "Checked elsewhere in this change", "The test checks the same thing"];

const ANSWERS = {
  carryOn: {
    consequence: "Overrules the flag. It is not the last step, so the job carries on at the next one.",
    presets: PRESETS,
    onCarryOn: fn(),
  },
  sendBack: {
    consequence: "Restarts the step. The new drone's brief carries the flag, and your note if you write one.",
    onSendBack: fn(),
  },
  onOpenBrief: fn(),
};

/**
 * **A flag on an added line.** Fleet placed it at line 41 of the post-image,
 * and the hunk holding that line is what the card draws — the old assertion
 * and the one that replaced it, one above the other.
 *
 * **Nothing to type.** Carry on with no reason picked and no note sends a
 * blank reason, which Fleet takes on a gaming flag.
 */
export const OnAnAddedLine: Story = {
  args: {
    ...ANSWERS,
    findings: [
      {
        pattern: "assertion_weakened",
        ...meaning("assertion_weakened"),
        hunk: {
          path: TEST_FILE,
          lines: [
            { kind: "hunk", text: '@@ -38,7 +38,7 @@ describe("useColumnSelectors", () => {' },
            { kind: "context", text: '   it("keeps hidden columns out of the visible set", () => {' },
            { kind: "context", text: "     const visible = selectVisible(state);" },
            { kind: "removed", text: '-    expect(visible).toEqual(["name", "status", "owner"]);' },
            { kind: "added", text: "+    expect(visible.length).toBeGreaterThan(0);" },
            { kind: "context", text: "   });" },
          ],
        },
        asked: ASKED,
        brief: BRIEF,
      },
    ],
  },
  play: async ({ args, canvas, userEvent }) => {
    await userEvent.click(canvas.getByRole("button", { name: "Carry on" }));
    await expect(args.carryOn.onCarryOn).toHaveBeenCalledWith("");
  },
};

/**
 * **A flag on a removed line.** A line the change took out has no post-image
 * number, so Fleet sent the file and no line — and the hunk is found by the
 * words the check quoted, never by a line guessed near it.
 *
 * **The reason is what the person picked and typed**, joined, and nothing
 * else.
 */
export const OnARemovedLine: Story = {
  args: {
    ...ANSWERS,
    findings: [
      {
        pattern: "assertion_weakened",
        ...meaning("assertion_weakened"),
        hunk: {
          path: TEST_FILE,
          lines: [
            { kind: "hunk", text: "@@ -52,9 +52,7 @@" },
            { kind: "context", text: '   it("drops a column that was hidden", () => {' },
            { kind: "context", text: '     const next = reducer(state, hide("owner"));' },
            { kind: "removed", text: '-    expect(next.hidden).toContain("owner");' },
            { kind: "removed", text: '-    expect(selectVisible(next)).not.toContain("owner");' },
            { kind: "context", text: "     expect(next.version).toBe(state.version + 1);" },
            { kind: "context", text: "   });" },
          ],
        },
        asked: ASKED,
        brief: BRIEF,
      },
    ],
  },
  play: async ({ args, canvas, userEvent }) => {
    await userEvent.click(canvas.getByRole("radio", { name: "Checked elsewhere in this change" }));
    await userEvent.type(canvas.getByRole("textbox", { name: "Note (optional)" }), "See reducer.test.ts");
    await userEvent.click(canvas.getByRole("button", { name: "Carry on" }));
    await expect(args.carryOn.onCarryOn).toHaveBeenCalledWith(
      "Checked elsewhere in this change. See reducer.test.ts",
    );
  },
};

/**
 * **A pattern the diff decides, which was asked nothing.** No question and no
 * brief, so the card draws neither — and its explanation speaks flatly,
 * because the patch measured it rather than a model judging it.
 */
export const DecidedByTheDiff: Story = {
  args: {
    ...ANSWERS,
    findings: [
      {
        pattern: "check_config_edited",
        ...meaning("check_config_edited"),
        hunk: {
          path: "packages/settings/package.json",
          lines: [
            { kind: "hunk", text: "@@ -6,7 +6,7 @@" },
            { kind: "context", text: '   "scripts": {' },
            { kind: "removed", text: '-    "test": "vitest run",' },
            { kind: "added", text: '+    "test": "vitest run --passWithNoTests src/selectors",' },
            { kind: "context", text: "   }," },
          ],
        },
      },
    ],
  },
};

/**
 * **Where no hunk could be located**, the citation and the file are drawn
 * instead of a nearby hunk. A deleted test file is quoted whole, and a guessed
 * window would send a person to the wrong lines believing them.
 */
export const NoHunkLocated: Story = {
  args: {
    ...ANSWERS,
    findings: [
      {
        pattern: "test_deleted",
        ...meaning("test_deleted"),
        cited: "`packages/settings/test/useColumnSelectors.test.ts` was deleted in the step it gates.",
        at: { file: TEST_FILE },
      },
    ],
  },
};

/**
 * **Two flags on one step.** Two findings, each with what it means and its own
 * lines, and one answer — the step is held once, and a person decides once.
 *
 * Send it back with no note sends none, rather than an empty string.
 */
export const TwoFlagsOnOneStep: Story = {
  args: {
    ...ANSWERS,
    findings: [
      { ...OnAnAddedLine.args!.findings![0]! },
      {
        pattern: "test_skipped",
        ...meaning("test_skipped"),
        hunk: {
          path: TEST_FILE,
          lines: [
            { kind: "hunk", text: "@@ -70,6 +70,6 @@" },
            { kind: "removed", text: '-  it("reorders columns by drag", () => {' },
            { kind: "added", text: '+  it.skip("reorders columns by drag", () => {' },
            { kind: "context", text: '     const next = reducer(state, move("owner", 0));' },
          ],
        },
      },
    ],
  },
  play: async ({ args, canvas, userEvent }) => {
    await userEvent.click(canvas.getByRole("button", { name: "Send it back" }));
    await expect(args.sendBack.onSendBack).toHaveBeenCalledWith(undefined);
  },
};

/**
 * **Send it back where Fleet will not restart the step.** A Drone still
 * holding its session is one a restart would end, and Fleet refuses that —
 * so the answer is drawn, off, with Fleet's reason under it.
 */
export const SendItBackWithheld: Story = {
  args: {
    ...OnAnAddedLine.args,
    sendBack: {
      ...ANSWERS.sendBack,
      withheld:
        "Restart is off while the drone is still running, because it would throw away the drone's session.",
    },
  },
};
