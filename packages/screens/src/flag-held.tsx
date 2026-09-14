// The card a gaming flag holds a step with, and the refusals it moves out of
// the way. #1079.
//
// **What the flag means, the lines, what it asked, and both answers**, in the
// band that says the step stopped. The owner hit a step held here on 14 Sep
// with a quoted doc comment, no sentence saying what it meant, no action beside
// it, and three unrelated refused commands in the same box.
//
// **Both answers are acts that already exist.** Carry on is the override, which
// Fleet takes with a blank reason on a gaming flag. Send it back is the step
// restart, which already puts the flag into the next Drone's brief.

import { GAMING_PATTERN_MEANING, HeldFlag, Refusals, type HeldFinding } from "@armada/components";
import type { Diff, JobDetail as JobWhole, JobSummary, StepDetail } from "@armada/protocol";
import type { ReactNode } from "react";

import { answerNamed } from "./copy";
import { flagsOf, hunkFor } from "./gaming";
import { openKept, type Opens } from "./phases";
import { onwards, recourseOf, RESTART_WITHHELD } from "./recovery";
import { refusedIn } from "./refused";
import type { Answering } from "./step";

/** What the card needs from the screen to draw its lines and send its answers. */
export type Deciding = {
  /** This Job's patch, as the screen already holds it. The hunks come out of it. */
  diff: Diff;
  stale: boolean;
  acting: boolean;
  onOverrule: (jobId: string, reason: string) => void;
  onSendBack: (jobId: string, note?: string) => void;
};

/** Whether this step is the one Fleet holds on a gaming flag that nothing cleared. */
export function heldByAFlag(whole: JobWhole | null, step: StepDetail): boolean {
  const stuck = whole?.stuck;
  return (
    stuck?.stopped_by === "evidence_suspect" &&
    stuck.step_id === step.step_id &&
    !step.overridden &&
    flagsOf(step).held.length > 0
  );
}

/** The presets the owner chose for *No, the work is fine*. Sent as the reason, word for word. */
export const CARRY_ON_PRESETS = [
  "It's not a test",
  "Checked elsewhere in this change",
  "The test checks the same thing",
];

/** What *Send it back* does. `restart_step` briefs the next Drone with the flag. */
const SENDS_IT_BACK = "Restarts the step. The new drone's brief carries the flag, and your note if you write one.";

/** Where Fleet offers no restart and no Drone is holding either. */
const NO_RESTART = "Fleet offers no restart on this step.";

/** Where Fleet offers no override on this step. */
const NO_OVERRIDE = "Fleet offers no override on this step.";

const STALE = "This Job is not live, so nothing can be sent.";
const SENDING = "That answer is already on its way to Fleet.";

/**
 * The card, or `undefined` where the step is not held on a flag, or where the
 * screen gave no way to answer — the band then draws the flags as rows.
 */
export function heldFlagOf(
  job: JobSummary,
  whole: JobWhole | null,
  step: StepDetail,
  opens: Opens,
  deciding: Deciding | undefined,
): ReactNode {
  if (deciding === undefined || !heldByAFlag(whole, step)) return undefined;
  const recourse = recourseOf(job, whole);
  const overrule = recourse.overrule?.trigger === "evidence_suspect" ? recourse.overrule : undefined;
  const findings: HeldFinding[] = flagsOf(step).held.map((flag) => {
    const means = GAMING_PATTERN_MEANING[flag.pattern];
    const located = hunkFor(flag, deciding.diff, job.id);
    return {
      pattern: flag.pattern,
      ...(means === undefined ? {} : { headline: means.headline, explanation: means.explanation }),
      // The lines where the patch holds them, and what the check quoted where
      // it does not — never a hunk near the one it meant.
      ...(located === undefined
        ? { cited: flag.cited, ...(flag.at === undefined ? {} : { at: flag.at }) }
        : { hunk: { path: located.file, lines: located.lines } }),
      ...(flag.asked === undefined ? {} : { asked: flag.asked }),
      ...(flag.brief_path === undefined ? {} : { brief: flag.brief_path }),
    };
  });
  return (
    <HeldFlag
      findings={findings}
      onOpenBrief={(brief) => openKept(opens, { kept: brief, what: "brief" })}
      carryOn={{
        consequence:
          overrule === undefined ? "Overrules the flag." : `Overrules the flag. ${onwards(overrule)}`,
        ...(overrule === undefined ? { withheld: NO_OVERRIDE } : {}),
        presets: CARRY_ON_PRESETS,
        onCarryOn: (reason) => deciding.onOverrule(job.id, reason),
      }}
      sendBack={{
        consequence: SENDS_IT_BACK,
        // **Fleet's answer, never guessed.** A Drone still holding its session
        // is one a restart would end, and Fleet refuses that.
        ...(recourse.act === "restart_step"
          ? {}
          : { withheld: recourse.act === "redirect" ? RESTART_WITHHELD : NO_RESTART }),
        onSendBack: (note) => deciding.onSendBack(job.id, note),
      }}
      disabled={deciding.stale || deciding.acting}
      disabledNote={deciding.stale ? STALE : deciding.acting ? SENDING : undefined}
    />
  );
}

/**
 * The refused commands, in a folded card of their own, on a step a flag holds
 * — *3 commands were refused during Implement · not why it stopped*.
 * `undefined` everywhere else, where the band draws them as it always has.
 */
export function refusedAsideOf(
  whole: JobWhole | null,
  step: StepDetail,
  answering: Answering | undefined,
): ReactNode {
  if (!heldByAFlag(whole, step)) return undefined;
  const refused = refusedIn(whole);
  if (refused === undefined) return undefined;
  const count = Math.max(whole?.stuck?.refusals ?? 0, refused.refused.length);
  return (
    <Refusals
      {...refused}
      again={undefined}
      folded={{
        summary: `${count} ${count === 1 ? "command was" : "commands were"} refused during ${step.label}`,
        aside: "not why it stopped",
        showLabel: "Show them",
        hideLabel: "Hide them",
      }}
      onAnswer={(call, name, rule) => {
        const chose = answerNamed(name);
        if (chose !== undefined) answering?.send(call, chose, undefined, rule);
      }}
      disabled={answering === undefined || answering.stale || answering.acting}
      disabledNote={answering?.stale ? STALE : answering?.acting ? SENDING : undefined}
    />
  );
}
