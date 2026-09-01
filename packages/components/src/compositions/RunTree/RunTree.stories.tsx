import type { Meta, StoryObj } from "@storybook/react-vite";
import { RunTree, type RunTreeStep } from "./RunTree";

/**
 * The run, drawn from the Bug workflow — the reference sample, seven steps,
 * linear. One story per state the tree has to carry, and the three at the
 * bottom are the three kinds of stopped that must never look alike.
 */
const meta: Meta<typeof RunTree> = {
  title: "Compositions/Run tree",
  component: RunTree,
};
export default meta;

type Story = StoryObj<typeof RunTree>;

const ARTIFACTS = ".armada/artifacts/job_2d90bb/";

/** The four steps the drawing draws in full, and the three ahead of them. */
const BUG: RunTreeStep[] = [
  {
    id: "repro",
    label: "Reproduction",
    activity: "advanced",
    elapsed: "1m 12s",
    status: "advanced",
    facts: [
      {
        label: "Produced",
        paths: [{ directory: "packages/settings/test/", basename: "useColumnSelectors.test.ts" }],
      },
      { label: "Cleared", value: "test", named: "passed" },
    ],
  },
  {
    id: "root_cause",
    label: "Root cause",
    activity: "advanced",
    elapsed: "3m 40s",
    status: "advanced",
    facts: [
      { label: "Attempt 1", value: "refused", named: "refused" },
      { label: "Attempt 2", value: "advanced", named: "advanced" },
      { label: "Produced", paths: [{ directory: ARTIFACTS, basename: "root_cause.md" }] },
    ],
  },
  {
    id: "fix",
    label: "Fix",
    activity: "running",
    elapsed: "6m 11s",
    status: "running",
    current: true,
    factsOpen: true,
    facts: [
      { label: "Produced", value: "3 files · +94 −31" },
      { label: "Checks", value: "not run" },
      { label: "Judge", value: "2 criteria" },
    ],
  },
  {
    id: "regression_verify",
    label: "Regression check",
    activity: "not_started",
    facts: [],
    factsAbsent: "This step has not run, so it has produced nothing.",
  },
  {
    id: "consumers",
    label: "Check the consumers still compile",
    activity: "not_started",
    facts: [],
    factsAbsent: "This step has not run, so it has produced nothing.",
  },
  {
    id: "land",
    label: "Land",
    activity: "not_started",
    locked: true,
    facts: [],
    factsAbsent: "This step has not run, so it has produced nothing.",
  },
];

/**
 * The healthy run. The current step's facts are open, every other step is
 * closed — a seven-step workflow with every step expanded fits no screen.
 */
export const Running: Story = {
  args: { steps: BUG, pulsing: true, onSelect: () => {} },
};

/**
 * Nothing is open. This is what a reader lands on for every step but the one
 * the caller opened: the tree holds short facts and they are behind a chevron,
 * because the panel beside it is where the long content lives.
 */
export const EverythingClosed: Story = {
  args: {
    steps: BUG.map((step) => ({ ...step, factsOpen: false })),
    pulsing: true,
    onSelect: () => {},
  },
};

/**
 * **Waiting on you — amber, and no surface.** Everything mechanical cleared;
 * the Job is stopped and that is the workflow working. A tint here would make
 * a designed human gate look like a failure, which is the one shape that must
 * not.
 */
export const WaitingOnYou: Story = {
  args: {
    pulsing: true,
    onSelect: () => {},
    steps: [
      { id: "fix", label: "Fix", activity: "advanced", elapsed: "6m 11s", status: "advanced", facts: [] },
      {
        id: "regression_verify",
        label: "Regression check",
        activity: "awaiting_human",
        elapsed: "2m 04s",
        status: "waiting on you",
        current: true,
        factsOpen: true,
        facts: [
          { label: "Checks", value: "2 of 2 passed", named: "passed" },
          { label: "Judge", value: "2 of 2 met", named: "passed" },
          { label: "Waiting", value: "on you · 2m 04s" },
        ],
      },
    ],
  },
};

/**
 * **Stopped — retries spent.** A surface rather than a glyph alone, because
 * this row has to stay findable while the refusals are read in the panel
 * beside it. The flag stays `--fg-default`: the surface already carries the
 * warning, and saying it twice would make it look like a failure. It is not —
 * a person decides what happens next.
 */
export const Stopped: Story = {
  args: {
    onSelect: () => {},
    steps: [
      { id: "root_cause", label: "Root cause", activity: "advanced", elapsed: "3m 40s", status: "advanced", facts: [] },
      {
        id: "fix",
        label: "Fix",
        activity: "stopped",
        elapsed: "14m 22s",
        status: "retries spent",
        current: true,
        factsOpen: true,
        facts: [
          { label: "Attempt 1", value: "refused · reducer changed", named: "refused" },
          { label: "Attempt 2", value: "refused · same criterion", named: "refused" },
          { label: "Attempt 3", value: "refused · same criterion", named: "refused" },
          { label: "Held", value: "retries spent · waiting on you" },
        ],
      },
    ],
  },
};

/**
 * **Failed — the Job is over.** Hued in both channels, the surface and the
 * glyph, because failed is an outcome rather than a position and the step that
 * ended the Job has to say so twice. Nothing below it ever ran, and the tree
 * shows that by having nothing below it.
 */
export const Failed: Story = {
  args: {
    onSelect: () => {},
    steps: [
      { id: "fix", label: "Fix", activity: "advanced", elapsed: "6m 11s", status: "advanced", facts: [] },
      {
        id: "regression_verify",
        label: "Regression check",
        activity: "failed",
        elapsed: "2m 51s",
        status: "failed",
        current: true,
        factsOpen: true,
        facts: [
          { label: "Checks", value: "test failed · exit 101", named: "failed" },
          { label: "Judge", value: "not reached" },
          { label: "Job", value: "completed_failed", named: "failed" },
        ],
      },
    ],
  },
};

/**
 * A hard prerequisite, at the trailing edge in `--fg-muted`, label only. It is
 * a property of the workflow definition rather than of this run, so it reads
 * the same on a Job that has not started as on one that stopped — and the way
 * past it is Pilot, not a row action.
 */
export const HardPrerequisite: Story = {
  args: {
    onSelect: () => {},
    steps: [
      { id: "land", label: "Land", activity: "not_started", locked: true, facts: [] },
      {
        id: "announce",
        label: "Announce",
        activity: "not_started",
        locked: true,
        lockedLabel: "Cannot be skipped, even on retry",
        facts: [],
      },
    ],
  },
};

/**
 * A step whose workflow declares no label. The `step_id` renders in mono
 * instead — honest, and useless to scan. See `[workflow-step-human-label]`.
 */
export const NoHumanName: Story = {
  args: {
    onSelect: () => {},
    steps: [
      {
        id: "regression_verify",
        label: "regression_verify",
        labelIsAnIdentifier: true,
        activity: "running",
        elapsed: "2m 04s",
        status: "running",
        current: true,
        facts: [{ label: "Checks", value: "1 of 2 · running" }],
      },
    ],
  },
};

/**
 * A tree with no handler: a record being read rather than a surface being
 * acted on, so the names are not controls. The chevrons still open — a fact is
 * read whether or not the step can be selected.
 */
export const ReadOnly: Story = {
  args: { steps: BUG },
};
