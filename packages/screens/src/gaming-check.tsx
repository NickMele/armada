// The step panel's Gaming check section — every flag the check raised on this
// attempt, held or cleared. #1079.
//
// **A section of its own, beside Checks and Judge and counted in neither.** A
// flag is not a verdict, and folding it into either tier would make a green
// tier read red; nothing else on the panel named the check that stopped the
// step, which is the defect this answers.

import { GAMING_PATTERN, GamingFlags, type StepChapter } from "@armada/components";
import type { StepDetail } from "@armada/protocol";

import { onlyCurrentAttempt } from "./facts";
import { declaresGaming, flagsOf, gamingReached, gamingSummary, type FlaggedRead } from "./gaming";
import { openKept, type Opens } from "./phases";

/** Which chapter the gaming check is, so the timeline can place it. */
export const GAMING_CHAPTER = "gaming";

/**
 * The section, or none. **A step that declares no gaming check and was flagged
 * nothing draws nothing** — an empty labelled region reads as a reading that
 * failed.
 */
export function gamingChapter(step: StepDetail, opens: Opens): Omit<StepChapter, "ordinal"> | undefined {
  const flags: FlaggedRead[] = onlyCurrentAttempt(step.flagged);
  if (!declaresGaming(step) && flags.length === 0) return undefined;
  const read = flagsOf(step, flags);
  const reached = gamingReached(step);
  return {
    id: GAMING_CHAPTER,
    title: "Gaming check",
    summary: gamingSummary(read, reached, read.held.length > 0 && !step.overridden),
    preview:
      flags.length === 0 ? (
        reached ? RAN_AND_FOUND_NOTHING : NOT_RUN_YET
      ) : (
        <GamingFlags
          citation="whole"
          flags={flags.map((flag) => ({
            pattern: flag.pattern,
            // The registry's verb, keyed by the wire spelling, which is the
            // fallback where the registry has no row.
            verb: GAMING_PATTERN[flag.pattern]?.verb ?? undefined,
            cited: flag.cited,
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
          }))}
          onOpenBrief={(brief) => openKept(opens, { kept: brief, what: "brief" })}
        />
      ),
  };
}

/** A gaming check that ran on this attempt and flagged nothing. */
const RAN_AND_FOUND_NOTHING = "The gaming check read this attempt's change and flagged nothing.";

/** One the gate has not reached on this attempt. */
const NOT_RUN_YET = "The gate has not reached the gaming check on this attempt.";
