import type { Meta, StoryObj } from "@storybook/react-vite";

import { Button } from "../../primitives/Button/Button";
import { StepTimeline, StepTimelineSkeleton } from "./StepTimeline";

/**
 * A step as one timeline, replacing the phase strip and the story that repeated
 * it. Every story here puts plain lines in the bodies so the shape is what is on
 * trial; in the panel those are the step's own chapters.
 */
const meta = {
  title: "Compositions/Step timeline",
  component: StepTimeline,
  parameters: { layout: "padded" },
} satisfies Meta<typeof StepTimeline>;

export default meta;
type Story = StoryObj<typeof meta>;

/**
 * The skeleton is its own component, so its stories are typed against it.
 * Typed as the timeline's, they would owe `attempts` that a skeleton has none of.
 */
type SkeletonStory = StoryObj<typeof StepTimelineSkeleton>;

/** A line standing in for a chapter's body. */
function body(text: string) {
  return <p style={{ margin: 0, color: "var(--fg-muted)" }}>{text}</p>;
}

/** Worked once, and the Drone is in it. The ordinary step. */
export const Working: Story = {
  name: "Working",
  args: {
    label: "Where this step is",
    attempts: [
      {
        id: "1",
        name: "Attempt 1",
        current: true,
        rows: [
          { id: "a", name: "Instructed", activity: "advanced", meta: "06:26:54", body: body("The brief.") },
          {
            id: "b",
            name: "Working",
            activity: "running",
            live: true,
            meta: "351 calls · 43m 37s · 36 files",
            body: body("The activity log."),
            act: <Button variant="ghost" size="sm">Open the log</Button>,
          },
          { id: "c", name: "Checks", activity: "not_started", meta: "not reached" },
          { id: "d", name: "Judge", activity: "not_started", meta: "2 criteria, not asked" },
        ],
      },
    ],
  },
};

/**
 * Handed back once, and working again.
 *
 * **The whole reason the timeline exists.** The strip could say "handed back
 * once" on one edge; here the attempt that was handed back is a section you can
 * open and read.
 */
export const HandedBack: Story = {
  name: "Handed back",
  args: {
    label: "Where this step is",
    attempts: [
      {
        id: "1",
        name: "Attempt 1",
        said: "handed back · 2m 22s",
        rows: [
          { id: "1a", name: "Instructed", activity: "advanced", body: body("The brief.") },
          { id: "1b", name: "Working", activity: "advanced", meta: "88 calls · 2m 22s", body: body("What it did.") },
          {
            id: "1c",
            name: "Checks",
            activity: "failed",
            meta: "cargo_nextest · 1 of 8 did not pass",
            body: body("exit 101"),
          },
          { id: "1d", name: "Judge", activity: "not_started", meta: "not reached" },
        ],
      },
      {
        id: "2",
        name: "Attempt 2",
        said: "running · 4m 09s",
        current: true,
        rows: [
          { id: "2a", name: "Instructed", activity: "advanced", body: body("What it was told this time.") },
          {
            id: "2b",
            name: "Working",
            activity: "running",
            live: true,
            meta: "31 calls · 4m 09s",
            body: body("The activity log."),
          },
          { id: "2c", name: "Checks", activity: "not_started", meta: "not reached" },
          { id: "2d", name: "Judge", activity: "not_started", meta: "2 criteria, not asked" },
        ],
      },
    ],
  },
};

/** Every phase cleared, and a person is looking at a finished step. */
export const Advanced: Story = {
  name: "Advanced",
  args: {
    attempts: [
      {
        id: "1",
        name: "Attempt 1",
        current: true,
        rows: [
          { id: "a", name: "Instructed", activity: "advanced", body: body("The brief.") },
          { id: "b", name: "Working", activity: "advanced", meta: "351 calls · 43m 37s · 36 files", body: body("What it did.") },
          { id: "c", name: "Checks", activity: "advanced", meta: "8 of 8 passed", body: body("Every Check passed.") },
          { id: "d", name: "Judge", activity: "advanced", meta: "2 of 2 met", body: body("The verdicts.") },
        ],
      },
    ],
  },
};

/**
 * Before the step has come back.
 *
 * **The names are known and the standing is not.** A phase's name comes off the
 * workflow, so the skeleton says which four are coming; where each one stands is
 * what the read answers, and that is the bar.
 */
export const Reading: SkeletonStory = {
  name: "Reading the step",
  render: () => <StepTimelineSkeleton phases={["Instructed", "Working", "Checks", "Judge"]} />,
};

/** The same, where even the names are not in hand. */
export const ReadingUnnamed: SkeletonStory = {
  name: "Reading, names unknown",
  render: () => <StepTimelineSkeleton />,
};
