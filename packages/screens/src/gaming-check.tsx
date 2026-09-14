// The step panel's Gaming check section — every pattern the check looks for,
// each flagged, cleared or not seen on this attempt. #1079.
//
// **A section of its own, beside Checks and Judge and counted in neither.** A
// flag is not a verdict, and folding it into either tier would make a green
// tier read red; nothing else on the panel named the check that stopped the
// step, which is the defect this answers.

import { GAMING_PATTERN, GamingFlags, type GamingFlag, type StepChapter } from "@armada/components";
import type { StepDetail } from "@armada/protocol";

import { onlyCurrentAttempt } from "./facts";
import {
  declaredPatterns,
  declaresGaming,
  flagsOf,
  gamingReached,
  gamingSummary,
  NOT_SEEN,
  type FlaggedRead,
} from "./gaming";
import { openKept, type Opens } from "./phases";

/** Which chapter the gaming check is, so the timeline can place it. */
export const GAMING_CHAPTER = "gaming";

/**
 * The section, or none. **A step that declares no gaming check and was flagged
 * nothing draws nothing** — an empty labelled region reads as a reading that
 * failed.
 *
 * **One row per declared pattern where Fleet says which**, so a person sees
 * what the check looked for as well as what it found. From a Fleet that does
 * not say, the rows are the flags alone.
 */
export function gamingChapter(step: StepDetail, opens: Opens): Omit<StepChapter, "ordinal"> | undefined {
  const flags: FlaggedRead[] = onlyCurrentAttempt(step.flagged);
  if (!declaresGaming(step) && flags.length === 0) return undefined;
  const read = flagsOf(step, flags);
  const reached = gamingReached(step);
  const declared = declaredPatterns(step);
  const rows: GamingFlag[] =
    declared === undefined
      ? flags.map((flag) => rowOf(flag, false))
      : [
          ...declared.flatMap((pattern) => {
            const found = flags.filter((flag) => flag.pattern === pattern);
            return found.length === 0
              ? [{ pattern, verb: verbOf(pattern), stands: NOT_SEEN }]
              : found.map((flag) => rowOf(flag, true));
          }),
          // A flag on a pattern the declaration does not name is still a flag.
          ...flags.filter((flag) => !declared.includes(flag.pattern)).map((flag) => rowOf(flag, true)),
        ];
  return {
    id: GAMING_CHAPTER,
    title: "Gaming check",
    summary: gamingSummary(read, reached, read.held.length > 0 && !step.overridden, declared),
    preview:
      rows.length === 0 ? (
        reached ? RAN_AND_FOUND_NOTHING : NOT_RUN_YET
      ) : (
        <GamingFlags
          citation="whole"
          flags={rows}
          onOpenBrief={(brief) => openKept(opens, { kept: brief, what: "brief" })}
        />
      ),
  };
}

/** The registry's verb for a pattern, where it has one. */
function verbOf(pattern: string): string | undefined {
  return GAMING_PATTERN[pattern]?.verb ?? undefined;
}

/** One flag as a row, with whether it stands flagged or cleared where rows say so. */
function rowOf(flag: FlaggedRead, stands: boolean): GamingFlag {
  return {
    pattern: flag.pattern,
    verb: verbOf(flag.pattern),
    cited: flag.cited,
    ...(stands ? { stands: flag.cleared === undefined ? FLAGGED : CLEARED } : {}),
    ...(flag.at === undefined ? {} : { at: flag.at }),
    ...(flag.asked === undefined ? {} : { asked: flag.asked }),
    ...(flag.brief_path === undefined ? {} : { brief: flag.brief_path }),
    ...(flag.cleared === undefined
      ? {}
      : {
          cleared: {
            why: flag.cleared.why,
            ...(flag.cleared.brief_path === undefined ? {} : { brief: flag.cleared.brief_path }),
          },
        }),
  };
}

const FLAGGED = "flagged";
const CLEARED = "cleared";

/** A gaming check that ran on this attempt and flagged nothing. */
const RAN_AND_FOUND_NOTHING = "The gaming check read this attempt's change and flagged nothing.";

/** One the gate has not reached on this attempt. */
const NOT_RUN_YET = "The gate has not reached the gaming check on this attempt.";
