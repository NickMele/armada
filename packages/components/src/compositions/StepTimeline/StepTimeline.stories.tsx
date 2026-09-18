import type { Meta, StoryObj } from "@storybook/react-vite";
import { useState } from "react";
import { expect, within } from "storybook/test";

import { Button } from "../../primitives/Button/Button";
import { StepTimeline, StepTimelineSkeleton, type StepTimelineAttempt } from "./StepTimeline";

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
            now: { verb: "Editing", detail: "crates/fleet/src/settling.rs", took: "3s", mono: true },
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
  /**
   * **The live phase has no mark, and it still says where it stands.** The
   * running mark left this header when the sweep arrived — two loops in one
   * card compete — and the edge, the wash and the bar are all colour and
   * motion, which a reader may have neither of. The word is what is left, and
   * it is the registry's own through `stepActivitySaid`.
   *
   * Checked against a component with the state span removed: the row read
   * `Working` alone and the phase's state was gone from the document.
   */
  play: async ({ canvas }) => {
    await expect(canvas.getByText("Working, running")).toBeInTheDocument();
  },
};

/**
 * What the header says between calls. **The Drone's own sentence, with no
 * verb** — nothing is in flight, so there is nothing to conjugate, and a verb
 * invented for the gap would claim a call that is not being made.
 */
export const BetweenCalls: Story = {
  name: "Between calls",
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
            now: { detail: "Now I will check what settling does with a held slot." },
            meta: "351 calls · 43m 37s",
            body: body("The activity log."),
          },
          { id: "c", name: "Checks", activity: "not_started", meta: "not reached" },
        ],
      },
    ],
  },
};

/**
 * At the review gate. The live row would open on its own everywhere else —
 * `folded` is what keeps every row closed until a person presses one.
 */
export const FoldedAtTheGate: Story = {
  name: "Folded at the gate",
  args: { ...Working.args, folded: true },
  play: async ({ canvas, userEvent }) => {
    const working = canvas.getByRole("button", { name: /Working/ });
    await expect(working).toHaveAttribute("aria-expanded", "false");
    await expect(canvas.getByText("The activity log.")).not.toBeVisible();

    await userEvent.click(working);
    await expect(working).toHaveAttribute("aria-expanded", "true");
    await expect(canvas.getByText("The activity log.")).toBeVisible();
  },
};

/* --- The header holds its height ------------------------------------------ */

/** The shortest summary the fixtures carry, and one the length of a sentence. */
const A_WORD = "thinking";
const A_SENTENCE =
  "I'll wait for the survey agent to finish before declaring the plan done, since the " +
  "reducer still imports the store and the test would go green for the wrong reason.";

/** The two widths the panel is measured at, both named where a person reads them. */
const NARROW = "The panel at its narrowest";
const WIDE = "The panel with room to spare";

/** The live step, with whatever the Drone last said under the phase's name. */
function summarised(said: string): StepTimelineAttempt {
  return {
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
        now: { detail: said },
        meta: "351 calls · 43m 37s · 36 files",
        body: body("The activity log."),
        act: <Button variant="ghost" size="sm">Open the log</Button>,
      },
    ],
  };
}

function HeaderHoldsItsHeightDrawn() {
  const [said, setSaid] = useState(A_WORD);
  return (
    <>
      <Button
        variant="ghost"
        size="sm"
        onClick={() => setSaid(said === A_WORD ? A_SENTENCE : A_WORD)}
      >
        {said === A_WORD ? "Say something longer" : "Say one word"}
      </Button>
      {/* `--w-work-min` is the floor the work area is held to beside the dock,
          so it is the narrowest this header is drawn at; twice it stands for
          the window the annotation was taken at, with the dock shut. */}
      <section aria-label={NARROW} style={{ width: "var(--w-work-min)" }}>
        <StepTimeline attempts={[summarised(said)]} />
      </section>
      <section aria-label={WIDE} style={{ width: "calc(var(--w-work-min) * 2)" }}>
        <StepTimeline attempts={[summarised(said)]} />
      </section>
    </>
  );
}

/**
 * The header holds one height whatever the Drone last said.
 *
 * **The owner's annotation.** The summary is as long as the sentence, and the
 * header read that length as a width too — which decided whether the act
 * wrapped to a second row, so the panel header grew and everything under it
 * moved every time the Drone said something new.
 *
 * **Measured, because a rendering cannot show it**: what a person sees is a
 * jump between two drawings, each of which looks right alone. The figure is
 * the top of the header to the first line of the body — the complaint in
 * numbers. Checked against the change reverted: the wide panel read 57px on
 * one word and 97px on the sentence, and the play failed on that 40px.
 */
export const HeaderHoldsItsHeight: Story = {
  name: "The header holds its height",
  args: { attempts: [summarised(A_WORD)] },
  render: () => <HeaderHoldsItsHeightDrawn />,
  play: async ({ canvas, userEvent }) => {
    // Off roles and text: which element draws the header line is the markup's
    // business, and what is on trial is where the body below it starts.
    const held = (panel: string) => {
      const region = within(canvas.getByRole("region", { name: panel }));
      const head = region.getByRole("button", { name: /Working/ });
      const first = region.getByText("The activity log.");
      return Math.round(first.getBoundingClientRect().top - head.getBoundingClientRect().top);
    };

    await expect(canvas.getAllByText(A_WORD)).toHaveLength(2);
    const onAWord = { narrow: held(NARROW), wide: held(WIDE) };

    await userEvent.click(canvas.getByRole("button", { name: "Say something longer" }));
    await expect(canvas.getAllByText(A_SENTENCE)).toHaveLength(2);

    await expect({ narrow: held(NARROW), wide: held(WIDE) }).toEqual(onAWord);
  },
};

/** `PhaseGoesLive`'s own attempt, built for whichever phase is live. */
function liveAttempt(live: "working" | "checks"): StepTimelineAttempt {
  return {
    id: "1",
    name: "Attempt 1",
    current: true,
    rows: [
      { id: "a", name: "Instructed", activity: "advanced", body: body("The brief.") },
      {
        id: "b",
        name: "Working",
        activity: live === "working" ? "running" : "advanced",
        live: live === "working",
        meta: "351 calls · 43m 37s · 36 files",
        body: body("The activity log."),
      },
      {
        id: "c",
        name: "Checks",
        activity: live === "checks" ? "running" : "not_started",
        live: live === "checks",
        meta: live === "checks" ? "running" : "not reached",
        body: live === "checks" ? body("Running the gate.") : undefined,
      },
      { id: "d", name: "Judge", activity: "not_started", meta: "2 criteria, not asked" },
    ],
  };
}

/**
 * A step moving from Working to Checks. **The live phase is the one open**,
 * and it moves without a press: `whereItIs` re-runs as the live row changes,
 * not only at mount, which is #1152's phase-level half.
 */
function PhaseGoesLiveDrawn() {
  const [live, setLive] = useState<"working" | "checks">("working");
  // Both ways, because the play below presses it on load: a one-way press
  // leaves anyone opening the story looking at the move's far side, with a
  // button that does nothing.
  return (
    <>
      <Button
        variant="ghost"
        size="sm"
        onClick={() => setLive(live === "working" ? "checks" : "working")}
      >
        {live === "working" ? "Move the step on" : "Back to Working"}
      </Button>
      <StepTimeline label="Where this step is" attempts={[liveAttempt(live)]} />
    </>
  );
}

export const PhaseGoesLive: Story = {
  name: "A phase goes live",
  // `render` draws its own state; `args` only satisfies the type `Working`'s
  // shape already carries — nothing here reads it.
  args: { attempts: [liveAttempt("working")] },
  render: () => <PhaseGoesLiveDrawn />,
  play: async ({ canvas, userEvent }) => {
    const working = canvas.getByRole("button", { name: /Working/ });
    await expect(working).toHaveAttribute("aria-expanded", "true");
    await expect(canvas.getByText("The activity log.")).toBeVisible();

    await userEvent.click(canvas.getByRole("button", { name: "Move the step on" }));

    await expect(working).toHaveAttribute("aria-expanded", "false");
    const checks = canvas.getByRole("button", { name: /Checks/ });
    await expect(checks).toHaveAttribute("aria-expanded", "true");
    await expect(canvas.getByText("Running the gate.")).toBeVisible();
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
 * A criterion call is out. #1153: the Judge phase used to say only
 * `asking · 2 criteria` for as long as the call took, whichever one it was.
 */
export const JudgeAskingCriterion: Story = {
  name: "Judge asking a criterion",
  args: {
    label: "Where this step is",
    attempts: [
      {
        id: "1",
        name: "Attempt 1",
        current: true,
        rows: [
          { id: "a", name: "Instructed", activity: "advanced", body: body("The brief.") },
          { id: "b", name: "Working", activity: "advanced", meta: "351 calls · 43m 37s · 36 files", body: body("What it did.") },
          { id: "c", name: "Checks", activity: "advanced", meta: "8 of 8 passed", body: body("Every Check passed.") },
          {
            id: "d",
            name: "Judge",
            activity: "running",
            live: true,
            meta: "asking implements_the_scope · call 2 of 5 · sonnet · 40s",
          },
        ],
      },
    ],
  },
};

/** The same call in flight, over a gaming look — named by the pattern instead. */
export const JudgeAskingGaming: Story = {
  name: "Judge asking about gaming",
  args: {
    label: "Where this step is",
    attempts: [
      {
        id: "1",
        name: "Attempt 1",
        current: true,
        rows: [
          { id: "a", name: "Instructed", activity: "advanced", body: body("The brief.") },
          { id: "b", name: "Working", activity: "advanced", meta: "351 calls · 43m 37s · 36 files", body: body("What it did.") },
          { id: "c", name: "Checks", activity: "advanced", meta: "8 of 8 passed", body: body("Every Check passed.") },
          {
            id: "d",
            name: "Judge",
            activity: "running",
            live: true,
            meta: "asking assertion_weakened · call 1 of 1 · sonnet · 12s",
          },
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
