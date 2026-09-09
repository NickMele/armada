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
//
// What the components were built for and the wire has not got — the assertion
// set, the console viewer, the input digest, the per-judge citation sets behind
// `JudgeRefusal.overlap`, and `JudgeVerdicts.measured` — is not drawn. Every
// one of them is a record of what a judge or a Check *read* rather than what it
// answered, which is a change to Fleet before it is a change to any screen.
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

import { checksChapter } from "./checks";
import { panelsOf } from "./gates";
import type { Opens } from "./phases";
import { verdictsChapter } from "./verdicts";

export { CHECKS_CHAPTER } from "./checks";
export { VERDICTS_CHAPTER } from "./verdicts";

/**
 * Where the evidence starts in the story. Produced is chapter three, so the
 * Checks are four and the Verdicts are five.
 *
 * **Numbered over what is drawn, not over what could be.** A step that declares
 * no Check and asks a Judge draws one evidence chapter, and numbering it `5`
 * would leave a gap in the ordinals a reader navigates by. The order is fixed —
 * Checks always before Verdicts — and the count is not.
 */
const AFTER_PRODUCED = 4;

/**
 * Chapters four and five, or fewer, or none.
 *
 * **A step with nothing gating it gets no chapter at all.** The phase strip
 * says what advances such a step, in words, and that sentence is the one place
 * it is said.
 */
export function evidenceChaptersOf({
  step,
  criteria,
  opens,
}: {
  step: StepDetail;
  /** The Job's frozen criteria, for the words and the position a citation names. */
  criteria: readonly Criterion[];
  /** How a record is opened, and where a refusal to open is said. */
  opens: Opens;
}): StepChapter[] {
  // Read once and drawn twice: the Checks chapter's Judge row and the Verdicts
  // grid are the same panel, and the two counts have to be one count.
  const panels = panelsOf(step, criteria);
  const drawn = [checksChapter(step, panels, opens), verdictsChapter(step, panels, opens)].filter(
    (chapter): chapter is Omit<StepChapter, "ordinal"> => chapter !== undefined,
  );
  return drawn.map((chapter, at) => ({ ...chapter, ordinal: AFTER_PRODUCED + at }));
}
