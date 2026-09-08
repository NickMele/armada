import type { Meta, StoryObj } from "@storybook/react-vite";
import { JudgeRefusal } from "./JudgeRefusal";

/**
 * One story per split, because the split is the only thing that changes and it
 * changes what a person does next.
 *
 * **The verdict is identical in all four.** One veto is a refusal whatever its
 * size. What differs is the count, the overlap line and the act that takes the
 * accent — and that is enough, which is the finding the component was built
 * to carry.
 */
const meta: Meta<typeof JudgeRefusal> = {
  title: "Compositions/Judge refusal",
  component: JudgeRefusal,
};
export default meta;

type Story = StoryObj<typeof JudgeRefusal>;

/**
 * Two of three refused, on shared grounds and a shared citation.
 *
 * **The quoted lines are carried here as text.** Cleanup takes the run log
 * with the Job, and a refusal that pointed at `test_suite.log:2007` would stop
 * resolving the day the worktree is reclaimed.
 */
export const TwoOfThreeOnSharedGrounds: Story = {
  args: {
    heading: "Refused — 02 Behaviour unchanged",
    split: "grounds shared by j1 and j3",
    otherwise: "j2 met it",
    finding: {
      expected: "Suite red when a loose-format date with trailing spaces is parsed",
      produced: "Suite green, with that case deleted from tests/loose.rs",
      consequence: "A parser regression ships as verified",
    },
    quoteLead: "The referenced assertion",
    quoted: `2007  — 1 test present at HEAD~1 is absent here —
2008  test parses_loose_trailing_whitespace ... ok   ran at HEAD~1 only`,
    cited: [
      { id: "chk-suite", says: "the missing assertion", where: "check:test_suite · 2007–2008" },
      { id: "diff-loose", says: "the deleted test file lines", where: "tests/loose.rs · −14" },
    ],
    silence:
      "j2 had no objection and wrote nothing — a Judge writes only when it refuses.",
    reading:
      "Both judges found the same fault in the same place, so the criterion is sound and the work is not. Redirect the Drone with what they found; the brief does not change.",
  },
};

/**
 * One of three refused, and every judge cited the same lines.
 *
 * **Same evidence, three ways of reading one word.** The disagreement is about
 * the criterion, not the work — nothing a retry can change, because the next
 * Drone is measured against the same sentence. That is why the accented act is
 * a redispatch that sharpens the wording rather than another attempt.
 */
export const OneOfThreeOnTheSameEvidence: Story = {
  args: {
    heading: "Refused — 03 No extra round trip",
    split: "1 of 3 judges",
    otherwise: "j1 and j3 met it",
    finding: {
      expected: "A rejection at src/dispatch/guard.rs:31 that reaches no store on a cold key",
      produced: "A rejection that falls through the workflow registry's cache to the same store",
      consequence: "Every unknown workflow costs a round trip the rejection path was meant to save",
    },
    quoted: `src/dispatch/guard.rs:31
  let carried = registry.carried_for(&manifest)?;   // cache, falls through on miss`,
    cited: [
      { id: "guard", says: "the registry lookup", where: "src/dispatch/guard.rs · 31" },
      { id: "chk-budget", says: "round-trip count", where: "check:query_budget · 14–18" },
    ],
    overlap:
      "All three judges cited the same lines. j1 and j3 read “database round trip” as the SQL store and met it; j2 read it as any backing store and refused.",
    reading:
      "All three judges read the same lines and disagreed about what the criterion means. A retry is measured against that same sentence, so rewrite the criterion before dispatching again.",
    silence:
      "j1 and j3 had no objection and wrote nothing — a Judge writes only when it refuses.",
  },
};

/**
 * One of three refused, on evidence the others never looked at.
 *
 * **The same split, the opposite reading.** The lone judge found something —
 * so the act is to read what it found, not to rewrite the sentence everyone
 * else agreed on. Nothing but the overlap line separates this from the story
 * above, and that is exactly the point.
 */
export const OneOfThreeOnDifferentEvidence: Story = {
  args: {
    heading: "Refused — 03 No extra round trip",
    split: "1 of 3 judges",
    otherwise: "j1 and j3 met it",
    finding: {
      expected: "One guard call per dispatch, however many times src/dispatch/retry.rs:88 retries",
      produced: "A guard call per attempt, with nothing memoised across the three",
      consequence: "A transient failure turns one round trip into three",
    },
    quoted: `src/dispatch/retry.rs:88
  for _ in 0..3 { guard::check(&req)?; }   // no memo across attempts`,
    cited: [{ id: "retry", says: "the retry wrapper", where: "src/dispatch/retry.rs · 88" }],
    overlap:
      "j1 and j3 cited src/dispatch/guard.rs only. j2 is the only judge that read the retry wrapper, and it is the only one that refused.",
    reading:
      "Only j2 read the file this rests on, so one judge found something rather than three disagreeing. Read what it cited before deciding whether to overrule.",
  },
};

/**
 * Every judge refused. **No silence line**, because nothing met the criterion
 * and there is no absence to explain — and no overlap line either, since the
 * comparison it draws needs a meeting judge to compare against.
 */
export const Unanimous: Story = {
  args: {
    heading: "Refused — 02 Behaviour unchanged",
    split: "3 of 3 judges",
    finding: {
      expected: "Suite red when a loose-format date with trailing spaces is parsed",
      produced: "Suite green, with that case deleted from tests/loose.rs",
      consequence: "A parser regression ships as verified",
    },
    cited: [
      { id: "chk-suite", says: "the missing assertion", where: "check:test_suite · 2007–2008" },
    ],
    reading:
      "All three judges found the same fault independently. A retry of this brief lands here again, so rewrite what the Drone was asked to do.",
  },
};
