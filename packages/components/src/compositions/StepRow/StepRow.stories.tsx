import type { Meta, StoryObj } from "@storybook/react-vite";
import { FactChip } from "../FactChip/FactChip";
import { PathChip } from "../PathChip/PathChip";
import { StepRow } from "./StepRow";

const meta: Meta<typeof StepRow> = {
  title: "Compositions/Step row",
  component: StepRow,
  decorators: [
    // The well the run sits in. A row drawn on the canvas would be judged
    // against the wrong ground: every surface value on it is picked against
    // --bg-sunken.
    (Story) => (
      <div
        style={{
          width: "calc(var(--space-12) * 8)",
          padding: "var(--space-2)",
          borderRadius: "var(--radius-md)",
          background: "var(--bg-sunken)",
        }}
      >
        <Story />
      </div>
    ),
  ],
};
export default meta;

type Story = StoryObj<typeof StepRow>;

/**
 * The step the panel is showing. The surface says what the row is and the
 * accent left edge says which row you are on — the two are separate channels
 * on purpose, so a selected failed row keeps its own surface.
 *
 * The name is the only thing on the tree at full weight, and the duration
 * comes up with it.
 */
export const Selected: Story = {
  args: {
    label: "Fix",
    activity: "running",
    status: "running",
    elapsed: "6m 11s",
    selected: true,
    open: true,
    factsId: "facts-selected",
    onToggle: () => {},
    onSelect: () => {},
    facts: [
      { label: "Produced", value: <FactChip>3 files · +94 −31</FactChip> },
      { label: "Checks", value: <FactChip>not run</FactChip> },
      { label: "Judge", value: <FactChip>2 criteria</FactChip> },
    ],
  },
};

/**
 * The cursor passing over a row it has not selected. Held open here because a
 * pseudo-class cannot be photographed, and a hover treatment nobody can look
 * at is one that drifts unseen.
 *
 * The surface is halfway between the well and the panel above it, not
 * `--bg-hover` — that value belongs to the selected row, and a hover reaching
 * it would say "you are here" to a cursor merely passing through.
 */
export const Hover: Story = {
  args: {
    label: "Root cause",
    activity: "advanced",
    status: "advanced",
    elapsed: "3m 40s",
    hovered: true,
    factsId: "facts-hover",
    onToggle: () => {},
    onSelect: () => {},
  },
};

/**
 * A step that advanced, opened. `Produced` is on every advanced step and is a
 * path chip, so the filename survives the 380px column whatever the directory
 * costs.
 *
 * The mark is `check` rather than the drawing's filled green disc: the icon
 * registry assigns `advanced` the bare check and reserves `circle-check` to a
 * Judge verdict, and where the registry and the drawing disagree the registry
 * wins. Reported.
 */
export const Advanced: Story = {
  args: {
    label: "Reproduction",
    activity: "advanced",
    status: "advanced",
    elapsed: "1m 12s",
    open: true,
    factsId: "facts-advanced",
    onToggle: () => {},
    onSelect: () => {},
    facts: [
      {
        label: "Produced",
        value: (
          <PathChip
            directory="packages/settings/test/"
            basename="useColumnSelectors.test.ts"
          />
        ),
      },
      { label: "Cleared", value: <FactChip named="passed">test</FactChip> },
    ],
  },
};

/**
 * The step a Drone is on. The mark pulses — opacity and scale on the inner
 * dot only, so the ring holds still and no row reflows.
 *
 * One pulse per screen, on the most specific mark present.
 */
export const Running: Story = {
  args: {
    label: "Fix",
    activity: "running",
    status: "running",
    elapsed: "6m 11s",
    pulsing: true,
    factsId: "facts-running",
    onToggle: () => {},
    onSelect: () => {},
  },
};

/**
 * A step the run has not reached. **A hollow ring and an em dash**, which is
 * what the drawing gives it — the build showed a list number and no duration,
 * and a column of numbers 2, 3, 4 tells a reader only that the tree has four
 * rows.
 *
 * The name recedes a further step. Rows below where the run is are the shape
 * of the workflow, not a record of anything.
 */
export const Unreached: Story = {
  args: {
    label: "Regression check",
    activity: "not_started",
    status: "not started",
    factsId: "facts-unreached",
    onSelect: () => {},
  },
};

/**
 * The step that ended the Job. **Hued in both channels** — the surface and the
 * mark — because failed is an outcome rather than a position, and the step
 * that ended the run has to say so twice.
 *
 * Nothing below it ever ran, and the tree shows that by having nothing below
 * it.
 */
export const Failed: Story = {
  args: {
    label: "Regression check",
    activity: "failed",
    status: "failed",
    elapsed: "2m 51s",
    selected: true,
    open: true,
    factsId: "facts-failed",
    onToggle: () => {},
    onSelect: () => {},
    facts: [
      { label: "Checks", value: <FactChip named="failed">test failed · exit 101</FactChip> },
      { label: "Judge", value: <FactChip>not reached</FactChip> },
      { label: "Job", value: <FactChip named="failed">completed_failed</FactChip> },
    ],
  },
};

/**
 * A Drone that tried and cannot get further. **A surface, not just a mark**,
 * because this row has to stay findable while the refusals beside it are read.
 *
 * The `flag` stays `--fg-default`: the surface already carries the warning and
 * a hued glyph would say it twice. It is not a failure — a person decides what
 * happens next.
 *
 * Every attempt is its own row. Attempts beside each other show whether a
 * Drone is trying different things or rephrasing one; a count shows neither.
 */
export const Held: Story = {
  args: {
    label: "Fix",
    activity: "stopped",
    status: "retries spent",
    elapsed: "14m 22s",
    selected: true,
    open: true,
    factsId: "facts-held",
    onToggle: () => {},
    onSelect: () => {},
    facts: [
      { label: "Attempt 1", value: <FactChip named="refused">refused · reducer changed</FactChip> },
      { label: "Attempt 2", value: <FactChip named="refused">refused · same criterion</FactChip> },
      { label: "Attempt 3", value: <FactChip named="refused">refused · same criterion</FactChip> },
      { label: "Held", value: <FactChip>retries spent · waiting on you</FactChip> },
    ],
  },
};

/**
 * A step waiting on a person. **Amber, never red, and no surface at all** —
 * everything mechanical cleared, so the Job is stopped and that is the
 * workflow working.
 *
 * Waiting, stopped and failed are three kinds of stopped and folding any two
 * of them together is what makes a person stop trusting the colour.
 */
export const WaitingOnYou: Story = {
  args: {
    label: "Regression check",
    activity: "awaiting_human",
    status: "waiting on you",
    elapsed: "2m 04s",
    selected: true,
    open: true,
    factsId: "facts-waiting",
    onToggle: () => {},
    onSelect: () => {},
    facts: [
      { label: "Checks", value: <FactChip named="passed">2 of 2 passed</FactChip> },
      { label: "Judge", value: <FactChip named="met">2 of 2 met</FactChip> },
      { label: "Waiting", value: <FactChip named="waiting">on you · 2m 04s</FactChip> },
    ],
  },
};

/**
 * A locked step. Label only, no action behind it — a hard prerequisite is a
 * property of the workflow definition rather than of this run, and the way
 * past it is Pilot.
 */
export const Locked: Story = {
  args: {
    label: "Land",
    activity: "not_started",
    status: "not started",
    locked: true,
    factsId: "facts-locked",
    onSelect: () => {},
  },
};
