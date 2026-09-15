// #1152: the panel following a Job, or holding where a person put it.
import type { Meta, StoryObj } from "@storybook/react-vite";
import { useState } from "react";
import { expect } from "storybook/test";

import type { StepDetail } from "@armada/protocol";
import { running } from "../../../fixtures/build/index";
import { advancedStep, BUILD_CHECK, freshStep, watchedRead } from "../../../fixtures/build/base";
import type { JobFixture } from "../../../fixtures/fixture";
import { JobDetailFrom } from "./JobDetail";

/** Same `title` as the rest of this directory, so ids hold — #1044. */
const meta: Meta<typeof JobDetailFrom> = {
  title: "Screens/Job detail",
  component: JobDetailFrom,
  parameters: { layout: "fullscreen" },
};
export default meta;

type Story = StoryObj<typeof JobDetailFrom>;

/** `Fix`, running rather than finished — `running.ts`'s own shape, for the step taking over. */
function runningStep(id: string, label: string, ordinal: number): StepDetail {
  return {
    ...freshStep(id, label, ordinal),
    state: "running",
    attempts: [{ attempt: 1, outcome: "running", started_at: "2026-09-10T14:30:00Z" }],
    entered_at: "2026-09-10T14:30:00Z",
    updated_at: "2026-09-10T14:30:00Z",
  };
}

/**
 * `running()`, one step on: Fix is done and Regression check is running —
 * the `job.step_advanced` Fleet would send, built by hand because these
 * stories are what a socket lands rather than a Job in a new state.
 */
function advancedOnce(fixture: JobFixture): JobFixture {
  const whole = fixture.watched.state === "read" ? fixture.watched.detail : undefined;
  if (whole === undefined) return fixture;
  const steps = whole.steps.map((step) => {
    if (step.step_id === "fix") return advancedStep("fix", "Fix", 3, [BUILD_CHECK]);
    if (step.step_id === "regression_verify") return runningStep("regression_verify", "Regression check", 4);
    return step;
  });
  const job = { ...fixture.job, current_step_id: "regression_verify" };
  return { ...fixture, job, watched: watchedRead({ ...whole, job, steps }) };
}

/**
 * `running()`, and a control standing in for Fleet's own event. **A button
 * the play function presses, not a timer it races** — the advance happens
 * exactly when the story says it does, and nowhere else.
 */
function AdvancingStepDrawn() {
  const [fixture, setFixture] = useState<JobFixture>(running);
  return (
    <>
      <button type="button" onClick={() => setFixture(advancedOnce)}>
        Fleet moves the job on to Regression check
      </button>
      <JobDetailFrom fixture={fixture} />
    </>
  );
}

/** The panel's own reading of which step is open — `InsideAJob.tsx`'s `armada-inside__step-name`. */
function openStepName(canvasElement: HTMLElement) {
  return canvasElement.querySelector(".armada-inside__step-name");
}

/**
 * A Job advancing with the step it is running on selected. **Follows** —
 * clicking the running step is not the same as holding on an earlier one,
 * even though both are presses on `onSelectStep`.
 */
export const FollowsTheRunningStep: Story = {
  name: "Advancing, the running step selected",
  render: () => <AdvancingStepDrawn />,
  play: async ({ canvas, canvasElement, userEvent }) => {
    await userEvent.click(canvas.getByRole("button", { name: "Fix" }));
    await expect(openStepName(canvasElement)).toHaveTextContent("Fix");

    await userEvent.click(
      canvas.getByRole("button", { name: "Fleet moves the job on to Regression check" }),
    );

    await expect(openStepName(canvasElement)).toHaveTextContent("Regression check");
  },
};

/**
 * The same advance, an earlier step selected. **Holds** — the reader stays on
 * `Root cause` until they click the step that is running now.
 */
export const HoldsOnAnEarlierStep: Story = {
  name: "Advancing, an earlier step selected",
  render: () => <AdvancingStepDrawn />,
  play: async ({ canvas, canvasElement, userEvent }) => {
    await userEvent.click(canvas.getByRole("button", { name: "Root cause" }));
    await expect(openStepName(canvasElement)).toHaveTextContent("Root cause");

    await userEvent.click(
      canvas.getByRole("button", { name: "Fleet moves the job on to Regression check" }),
    );

    await expect(openStepName(canvasElement)).toHaveTextContent("Root cause");
  },
};
