import type { Meta, StoryObj } from "@storybook/react-vite";
import { expect, fn } from "storybook/test";

import type { Refusal, StepDetail } from "@armada/protocol";
import { escalatedEvidenceSuspect } from "../../../fixtures/build/index";
import { diffRead, JOB_ID, watchedRead } from "../../../fixtures/build/base";
import type { JobFixture } from "../../../fixtures/fixture";
import { JobDetailFrom } from "./JobDetail";

/** Job detail, split by group — #1044. Same `title` as the rest of this directory, so ids hold. */
const meta: Meta<typeof JobDetailFrom> = {
  title: "Screens/Job detail",
  component: JobDetailFrom,
  parameters: { layout: "fullscreen" },
};
export default meta;

type Story = StoryObj<typeof JobDetailFrom>;

const TEST_FILE = "packages/settings/test/useColumnSelectors.test.ts";

/** A removed assertion, so the flag has a file and no line. */
const PATCH = [
  `diff --git a/${TEST_FILE} b/${TEST_FILE}`,
  `--- a/${TEST_FILE}`,
  `+++ b/${TEST_FILE}`,
  "@@ -52,6 +52,5 @@",
  '   it("drops a column that was hidden", () => {',
  '     const next = reducer(state, hide("owner"));',
  '-    expect(selectVisible(next)).not.toContain("owner");',
  "     expect(next.version).toBe(state.version + 1);",
  "   });",
].join("\n");

function refusedCommand(call: string, detail: string): Refusal {
  return { tool: "Bash", call, detail, truncated: false, because: "", offers: [], rules: [] };
}

/**
 * The owner's Job on 14 Sep, in the fixture's words: every Check passed, the
 * gaming check flagged a removed assertion, and three commands the Drone was
 * refused sat in the same box. The Drone has left, so Fleet offers both the
 * override and the restart.
 */
function heldByTheGamingCheck(): JobFixture {
  const fixture = escalatedEvidenceSuspect();
  if (fixture.watched.state !== "read") return fixture;
  const whole = fixture.watched.detail;
  const steps = whole.steps.map(
    (step): StepDetail =>
      step.step_id !== "regression_verify"
        ? step
        : {
            ...step,
            judged: step.judged.map((one) => ({ ...one, verdict: "met" })),
            flagged: [
              {
                attempt: 1,
                pattern: "assertion_weakened",
                cited: '`expect(selectVisible(next)).not.toContain("owner")` was taken out, and nothing replaces it.',
                at: { file: TEST_FILE },
                asked:
                  "Does this change alter an existing assertion so that it asserts less than it did, " +
                  "and is that assertion made nowhere else in this change?",
                brief_path: ".armada/briefs/77-split-the-settings-reducer/regression_verify.1.gaming.txt",
              },
            ],
          },
  );
  return {
    ...fixture,
    watched: watchedRead({
      ...whole,
      steps,
      stuck: {
        ...whole.stuck!,
        recourse: ["override_verdict", "restart_step", "redispatch_job"],
        refused: [
          refusedCommand("call_1", "cargo nextest run --package fleet 2>&1 | tail -80"),
          refusedCommand("call_2", "git stash"),
          refusedCommand("call_3", "git log --oneline -20"),
        ],
        refusals: 3,
      },
    }),
    recorded: {
      ...fixture.recorded,
      diff: diffRead([{ path: TEST_FILE, change: "modified" }], PATCH),
    },
  };
}

/** What Carry on sends. A module's spy, read by the play. */
const overrule = fn();

/**
 * **A step the gaming check stopped, and what to do about it.** #1079. The rail
 * names the gaming check, the panel has a section for it, and the card says
 * what the flag means over the lines it is about — with both answers, and the
 * refused commands folded into a card of their own.
 *
 * **Carry on needs nothing typed**, which is the definition of done.
 */
export const HeldByTheGamingCheck: Story = {
  name: "Held by the gaming check",
  render: () => <JobDetailFrom fixture={heldByTheGamingCheck()} on={{ onOverrule: overrule }} />,
  play: async ({ canvas, userEvent }) => {
    overrule.mockClear();
    await expect(canvas.getByText("A test may have been weakened to make this step pass")).toBeVisible();
    await expect(canvas.getByText("3 commands were refused during Regression check")).toBeVisible();
    await userEvent.click(canvas.getByRole("button", { name: "Carry on" }));
    await expect(overrule).toHaveBeenCalledWith(JOB_ID, "");
  },
};
