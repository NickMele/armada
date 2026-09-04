import type { Meta, StoryObj } from "@storybook/react-vite";
import { JobBrief } from "./JobBrief";

/**
 * What the Job was told, and what done means for it — the two halves of the
 * brief, drawn beside where the work is rather than in a region of their own.
 *
 * `acceptance_criteria` and `facts` are both served on `GET /jobs/:job_id` and
 * were drawn nowhere. The source is the wire's own spelling: no registry
 * carries a verb for `criterion_source`, and one written here would be the
 * second vocabulary that rule exists to prevent.
 */
const meta: Meta<typeof JobBrief> = {
  title: "Compositions/Job brief",
  component: JobBrief,
};
export default meta;

type Story = StoryObj<typeof JobBrief>;

const criteria = [
  { text: "A burst of 401s produces one refresh call, not one per request.", source: "check" },
  { text: "The retry ceiling is unchanged.", source: "check" },
  { text: "No token is written to a log line at any sink.", source: "judge" },
];

const facts =
  "The refresh path is in `auth/session.ts`. Two callers hit it concurrently on " +
  "a cold start, and the second one wins. Keep the public signature.";

export const Brief: Story = {
  args: { criteria, facts },
};

/**
 * A Job proposed with no criteria. Bridge's composer does not offer them, so
 * this is every Job composed in the app today — said plainly rather than left
 * as a label with nothing under it.
 */
export const NoCriteria: Story = {
  args: {
    criteria: [],
    criteriaAbsent:
      "This job was proposed with no acceptance criteria, so nothing states what done means for it.",
    facts,
  },
};

/** A Job given no context. `facts` is absent, never present and empty. */
export const NoFacts: Story = {
  args: {
    criteria,
    factsAbsent: "This job was given no context beyond its title.",
  },
};

/**
 * One half, where the two are placed in two regions. A finished Job leads with
 * what it was asked to achieve and folds the context it was given into the
 * record, so each half is drawn on its own and neither is drawn twice.
 */
export const CriteriaOnly: Story = {
  args: { criteria, only: "criteria" },
};

/** The other half, as the folded record draws it. */
export const FactsOnly: Story = {
  args: { criteria: [], facts, only: "facts" },
};

/**
 * The shape job detail draws — the sentence alone, with no label of its own.
 *
 * **The region is called `Brief` and the sentence follows it**, so a second
 * heading over one line is the sub-heading that screen removed. `null` draws no
 * element rather than an empty one: a blank span still occupies its line box,
 * which put dead space between the region's label and the one line under it.
 *
 * There is no surface here. The panel is the surface, and a card inside a
 * region already labelled BRIEF is the nested well the drawing does not have.
 */
export const AsALine: Story = {
  args: { criteria: [], facts, only: "facts", factsLabel: null },
};

/**
 * A note somebody has written that no drone has opened with yet.
 *
 * **The state is short and it is the only thing that says the note landed.**
 * Two acts leave one — sending work back at a gate, and restarting a step with
 * something to say — and both put the job in the queue with the words on its
 * record. The next drone to start opens with them and fleet clears the field in
 * the same breath, so this block is on screen for as long as the job waits and
 * gone the instant one starts.
 *
 * It leads, above both standing halves, because it is the newest thing about
 * the job and the only one that will not be here later.
 */
export const NoteWaiting: Story = {
  args: {
    criteria: [],
    facts,
    only: "facts",
    factsLabel: null,
    waiting: "Delete the assertion about the old header — it is testing behaviour we replaced.",
  },
};
