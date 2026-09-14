import type { Meta, StoryObj } from "@storybook/react-vite";
import { expect, fn } from "storybook/test";

import type { DeclaredJudge, Refusal, StepDetail } from "@armada/protocol";
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

/** The step's gaming check, with every pattern it looks for. Since protocol 13.50. */
const GAMING: DeclaredJudge = {
  criteria: 2,
  panel_size: 3,
  gaming_check: true,
  gaming_patterns: ["assertion_weakened", "test_scope_narrowed", "tautological_test", "test_skipped", "test_deleted", "check_config_edited"],
};

/**
 * The owner's Job on 14 Sep, in the fixture's words: every Check passed, the
 * gaming check flagged a removed assertion, and three refused commands sat in
 * the same box. `recourse` is Fleet's reading of whether the Drone is still
 * there, which decides what Send it back is.
 */
function heldByTheGamingCheck(recourse: string[]): JobFixture {
  const fixture = escalatedEvidenceSuspect();
  if (fixture.watched.state !== "read") return fixture;
  const whole = fixture.watched.detail;
  const steps = whole.steps.map(
    (step): StepDetail =>
      step.step_id !== "regression_verify"
        ? step
        : {
            ...step,
            judge_checks: [GAMING],
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
        recourse,
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

/** What each answer sends. Module spies, cleared by the plays that read them. */
const overrule = fn();
const redirect = fn();
const restart = fn();

/**
 * **A step the gaming check stopped, with the Drone still holding its
 * session**, which is what a flag leaves behind nearly every time. #1079. The
 * rail names the gaming check, the section reads `1 of 6 flagged`, and the card
 * says what the flag means over the lines it is about.
 *
 * **Send it back is a redirect here**, carrying the flag and the note, and
 * Carry on needs nothing typed.
 */
export const HeldByTheGamingCheck: Story = {
  name: "Held by the gaming check",
  render: () => (
    <JobDetailFrom
      fixture={heldByTheGamingCheck(["override_verdict", "redirect_drone", "redispatch_job"])}
      on={{ onOverrule: overrule, onRedirect: redirect, onSendBack: restart }}
    />
  ),
  play: async ({ canvas, userEvent }) => {
    overrule.mockClear();
    redirect.mockClear();
    restart.mockClear();
    await expect(canvas.getByText("3 commands were refused during Regression check")).toBeVisible();
    await expect(canvas.getByText(/Sends the flag back to the drone still on this step/)).toBeVisible();
    await userEvent.type(canvas.getByRole("textbox", { name: "Note for the drone (optional)" }), "Put it back");
    await userEvent.click(canvas.getByRole("button", { name: "Send it back" }));
    await expect(redirect).toHaveBeenCalledWith(
      JOB_ID,
      expect.stringContaining("A test may have been weakened to make this step pass."),
    );
    await expect(redirect).toHaveBeenCalledWith(JOB_ID, expect.stringContaining("The person's note: Put it back"));
    await expect(restart).not.toHaveBeenCalled();
    await userEvent.click(canvas.getByRole("button", { name: "Carry on" }));
    await expect(overrule).toHaveBeenCalledWith(JOB_ID, "");
  },
};

/**
 * **The same step once its Drone has gone.** Fleet offers the restart instead,
 * so Send it back restarts the step and the new Drone's brief carries the flag
 * — and the sentence under the answer says it is a restart.
 */
export const HeldByTheGamingCheckDroneGone: Story = {
  name: "Held by the gaming check, drone gone",
  render: () => (
    <JobDetailFrom
      fixture={heldByTheGamingCheck(["override_verdict", "restart_step", "redispatch_job"])}
      on={{ onRedirect: redirect, onSendBack: restart }}
    />
  ),
  play: async ({ canvas, userEvent }) => {
    redirect.mockClear();
    restart.mockClear();
    await expect(canvas.getByText(/Restarts the step with a fresh drone/)).toBeVisible();
    await userEvent.click(canvas.getByRole("button", { name: "Send it back" }));
    await expect(restart).toHaveBeenCalledWith(JOB_ID, undefined);
    await expect(redirect).not.toHaveBeenCalled();
  },
};
