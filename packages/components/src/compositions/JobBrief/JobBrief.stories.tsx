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
