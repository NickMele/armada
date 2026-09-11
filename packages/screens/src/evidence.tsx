// Chapters four and five of a step's story — what the Checks found, and what
// the panel made of it.
//
// # They are chapters, not a region
//
// The story is the order things happened: the Drone was instructed, it worked,
// it produced, the Checks ran, the panel read them. A Checks region drawn
// anywhere else on the screen would be a sixth place a reader has to learn, and
// the whole claim of this screen is that the arrangement does not move.
//
// **Both existed as components and as story fixtures and neither was ever built
// into the screen that ships.** `chapters.tsx` returned three chapters while
// the Storybook story returned five, two of them invented — so a person looking
// at a real refused Job saw the phase strip's two-line summary and nothing
// else: no finding, no grounds, no way to the output. This file is the join,
// and `evidence.test.tsx` mounts it so the two cannot come apart again.
//
// # Only what the wire serves
//
// Checked against `packages/protocol` on 2026-09-09.
//
// | Drawn | Served as |
// |---|---|
// | Each declared Check and what it came to | `StepDetail.checks`, `StepDetail.check_runs` |
// | A Check's output, opened | `CheckRun.output_path`, through `main/open.ts` |
// | The panel's shape — how many judges | `DeclaredJudge.panel_size`, absent at one |
// | One mark per judge per criterion | `Judged.member`, since protocol 7.7 |
// | A refusal's finding | `Judged.expected`, `produced`, `consequence` |
// | The brief a refusal answers, opened | `Judged.brief_path` |
// | The criterion's own words and its frozen position | `JobDetail.acceptance_criteria` |
// | What each member quoted, and where in the brief | `Judged.cited`, since protocol 8.3 |
// | What each member was handed, compared across the panel | `Judged.given`, since protocol 8.3 |
// | What the suite asserted, one row per Check | `StepDetail.check_runs`, every attempt's |
// | A Check's output, read in the app | `CheckOutput`, since protocol 9.1 |
//
// The last two rows were the two components with no producer, and `asserted.ts`
// and `outputs.ts` are what they are drawn from now. **Both are served at the
// granularity Armada's record actually keeps**, which is not the one the
// components were drawn for: an assertion is a Check rather than a test, and
// there is no parent-commit run to compare either against. Each file says why
// in its own header.
//
// What is still built and undrawn — the per-judge citation *sets* behind
// `JudgeRefusal.overlap`, and `JudgeVerdicts.measured` — is a reading over
// `Judged.cited` that nobody has written, and a join from a criterion to the
// Check that settled it, which does not exist.
//
// # Nothing empty is labelled
//
// A step that declares no Check draws no Checks chapter and a step that asks no
// Judge draws no Verdicts chapter. An empty labelled region reads as a value
// that failed to load, which is what this screen refuses everywhere else — and
// the phase strip's note is the one place an ungated step is explained.
//
// # The reading is `gates.ts`'s, not this file's
//
// The phase strip has read `check_runs` and `judged` since `#246`, including
// the panel grouping and the unanimity rule. A second reading here would drift,
// so both surfaces call `gates.ts` and these files decide only what a row looks
// like.

import type { Criterion, StepDetail } from "@armada/protocol";
import type { StepChapter } from "@armada/components";

/**
 * A chapter before the story has counted it.
 *
 * **The ordinal is the story's and never a chapter's own.** A reader navigates
 * by the number, so it has to be the position in what was drawn — and which
 * chapters are drawn depends on the step: one that gates on nothing has no
 * evidence chapter, and one that shows its work has an extra chapter before
 * Produced. A chapter that numbered itself would be right until either
 * happened.
 */
export type Unnumbered = Omit<StepChapter, "ordinal">;

import { checksChapter } from "./checks";
import { panelsOf } from "./gates";
import type { Following, Outputs } from "./outputs";
import type { Opens } from "./phases";
import { verdictsChapter } from "./verdicts";

export { CHECKS_CHAPTER } from "./checks";
export { VERDICTS_CHAPTER } from "./verdicts";

/**
 * The last chapters of the story, or fewer, or none.
 *
 * **A step with nothing gating it gets no chapter at all.** The phase strip
 * says what advances such a step, in words, and that sentence is the one place
 * it is said.
 */
export function evidenceChaptersOf({
  step,
  criteria,
  opens,
  outputs,
  now,
  following,
  undecided,
}: {
  /** Now, injected, so a running Check's elapsed time moves with the clock. */
  now: number;
  /** The running Check's log this window is following, and how to follow one. */
  following: Following;
  step: StepDetail;
  /** The Job's frozen criteria, for the words and the position a citation names. */
  criteria: readonly Criterion[];
  /** How a record is opened, and where a refusal to open is said. */
  opens: Opens;
  /**
   * Fleet's own reason the gate could not decide on this step's current
   * attempt. `stuck.undecided`, handed down from the Job's own record rather
   * than read here — a chapter has no route to `stuck` of its own.
   */
  undecided?: string;
  /**
   * What each Check printed, as this window has it, and how to ask for one.
   *
   * **Handed in rather than fetched here**, which is `Calls`' rule one record
   * over: what a Check printed is a reading, and fetching it is a round trip to
   * the process holding the file. Required rather than optional, for `opens`'
   * reason — a chapter given no way to read would go quietly back to being the
   * fixture this file replaced.
   */
  outputs: Outputs;
}): Unnumbered[] {
  // Read once and drawn twice: the Checks chapter's Judge row and the Verdicts
  // grid are the same panel, and the two counts have to be one count.
  const panels = panelsOf(step, criteria);
  // **Unnumbered, because the story numbers over what is drawn.** These used
  // to carry `4` and `5` off a constant, which was right while Produced was
  // always chapter three. A step that shows its work has a chapter before it,
  // and a fixed number here would put two chapters on one ordinal — so the
  // count is the story's to make, over the list it actually built. The order
  // is still fixed: Checks always before Verdicts.
  return [
    checksChapter(step, panels, opens, outputs, now, following, undecided),
    verdictsChapter(step, panels, opens, undecided),
  ].filter(
    (chapter): chapter is Unnumbered => chapter !== undefined,
  );
}
