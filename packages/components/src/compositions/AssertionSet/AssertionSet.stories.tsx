import type { Meta, StoryObj } from "@storybook/react-vite";
import { AssertionSet, type Assertion } from "./AssertionSet";

/**
 * One story per state the set has to be able to say: a suite that is green
 * because a case stopped existing, a suite that is green and means it, an
 * assertion that ran and came out wrong, and a row the harness gave no
 * sentence for.
 *
 * **The first story is the argument for the component.** Every row reads `ok`
 * except one, the Check in front of it exited 0, and the set is the only place
 * on the screen where that is visible.
 */
const meta: Meta<typeof AssertionSet> = {
  title: "Compositions/Assertion set",
  component: AssertionSet,
};
export default meta;

type Story = StoryObj<typeof AssertionSet>;

const SAME = "identical at HEAD and HEAD~1";

const held: Assertion[] = [
  {
    says: "An RFC 3339 timestamp with a numeric offset parses to the right instant",
    identifier: "parses_rfc3339_offset",
    named: "passed",
    against: SAME,
  },
  {
    says: "An RFC 2822 date using an obsolete zone name (EST, GMT) still parses",
    identifier: "parses_rfc2822_obsolete_zone",
    named: "passed",
    against: SAME,
  },
];

/**
 * The suite that passed by deleting an assertion.
 *
 * **`absent` is the row the screen was opened for.** It did not fail — it did
 * not run, and it ran and passed at the parent commit. A count cannot carry
 * that and neither can an exit code, which is why the set exists.
 */
export const GreenBecauseACaseStoppedExisting: Story = {
  args: {
    rows: [
      ...held,
      {
        says: "A loose-format date with spaces after it parses instead of erroring",
        identifier: "parses_loose_trailing_whitespace",
        named: "absent",
        against: "passed at HEAD~1 · absent at HEAD",
      },
      {
        identifier: "rejects_empty_string",
        named: "passed",
        against: SAME,
      },
    ],
    rest: { says: "311 further assertions", against: SAME },
  },
};

/**
 * A suite that is green and means it. Every row compares identical, and the
 * set reads as one block with nothing in it to stop on — which is what a
 * passing Check should look like when it is telling the truth.
 */
export const GreenAndMeansIt: Story = {
  args: {
    rows: held,
    rest: { says: "313 further assertions", against: SAME },
  },
};

/**
 * An assertion that ran and came out wrong. **Not the same finding as an
 * absent one**, and the hue is deliberately the same: both are the work not
 * being what was asked, and `against` is what says which happened. A second
 * hue would read as two degrees of one thing.
 */
export const AnAssertionThatFailed: Story = {
  args: {
    rows: [
      held[0]!,
      {
        says: "A loose-format date with spaces after it parses instead of erroring",
        identifier: "parses_loose_trailing_whitespace",
        named: "failed",
        against: "passed at HEAD~1 · failed at HEAD",
      },
    ],
  },
};

/**
 * Every row a fallback. **The identifier renders bare and the row says why** —
 * a harness that supplies no descriptions gives a set that is honest and hard
 * to scan, and inventing sentences out of the identifiers would give one that
 * scans and is not checkable against the tests.
 */
export const WhereTheHarnessNamedNothing: Story = {
  args: {
    rows: [
      { identifier: "parses_rfc3339_offset", named: "passed", against: SAME },
      { identifier: "rejects_empty_string", named: "passed", against: SAME },
      {
        identifier: "parses_loose_trailing_whitespace",
        named: "absent",
        against: "passed at HEAD~1 · absent at HEAD",
      },
    ],
  },
};
