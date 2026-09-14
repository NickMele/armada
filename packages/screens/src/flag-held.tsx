// The card a gaming flag holds a step with, and the refusals it moves out of
// the way. #1079.
//
// **What the flag means, the lines, what it asked, and both answers**, in the
// band that says the step stopped. The owner hit a step held here on 14 Sep
// with a quoted doc comment, no sentence saying what it meant, no action beside
// it, and three unrelated refused commands in the same box.
//
// **Both answers are acts that already exist.** Carry on is the override, which
// Fleet takes with a blank reason on a gaming flag. Send it back is a redirect
// where the Drone still holds its session, which is nearly every flag, and the
// step restart where it has gone, which briefs the next Drone with the flag.

import { GAMING_PATTERN_MEANING, HeldFlag, Refusals, type HeldFinding, type HeldFlagAnswer } from "@armada/components";
import type { Diff, JobDetail as JobWhole, JobSummary, StepDetail } from "@armada/protocol";
import type { ReactNode } from "react";

import { answerNamed } from "./copy";
import { flagsOf, hunkFor, sentBackWords } from "./gaming";
import { openKept, type Opens } from "./phases";
import type { ActingAct } from "./pending";
import { onwards, recourseOf } from "./recovery";
import { refusedIn } from "./refused";
import type { Answering } from "./step";

/** What the card needs from the screen to draw its lines and send its answers. */
export type Deciding = {
  /** This Job's patch, as the screen already holds it. The hunks come out of it. */
  diff: Diff;
  stale: boolean;
  acting: boolean;
  /** Which act, where `acting` is true — `override_verdict`, or the recourse act `Send back` sends. #1117. */
  actingAct?: ActingAct;
  onOverrule: (jobId: string, reason: string) => void;
  /** The restart, where the Drone has gone. */
  onSendBack: (jobId: string, note?: string) => void;
  /** The redirect, where the Drone still holds its session. */
  onRedirect: (jobId: string, instruction: string) => void;
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

/** What *Send it back* does where the Drone still holds its session. */
const REDIRECTS =
  "Sends the flag back to the drone still on this step, with your note if you write one, and it " +
  "works the step again in the same session.";

/** What it does where the Drone has gone. `restart_step` briefs the next Drone with the flag. */
const RESTARTS =
  "Restarts the step with a fresh drone. Its brief carries the flag, and your note if you write one.";

/** Where Fleet offers neither. */
const NEITHER = "Fleet offers neither a redirect nor a restart on this step.";

/** Where Fleet offers no override on this step. */
const NO_OVERRIDE = "Fleet offers no override on this step.";

const STALE = "This Job is not live, so nothing can be sent.";

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
  const flags = flagsOf(step).held;
  const findings: HeldFinding[] = flags.map((flag) => {
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
  // The registry's reasons for each pattern holding the step, once each.
  const presets = [...new Set(flags.flatMap((flag) => GAMING_PATTERN_MEANING[flag.pattern]?.presets ?? []))];
  // **Fleet's answer, never guessed**: a Drone still holding its session takes
  // a redirect, and one that has gone takes a restart.
  const sendsBy = recourse.act;
  // Which of the two answers is out, named directly — `override_verdict` and
  // `sendsBy` are two distinct acts, so nothing here has to remember which
  // control sent it. #1117.
  const pending: HeldFlagAnswer | undefined = !deciding.acting
    ? undefined
    : deciding.actingAct === "override_verdict"
      ? "carryOn"
      : sendsBy !== undefined && deciding.actingAct === sendsBy
        ? "sendBack"
        : undefined;
  return (
    <HeldFlag
      findings={findings}
      onOpenBrief={(brief) => openKept(opens, { kept: brief, what: "brief" })}
      carryOn={{
        consequence:
          overrule === undefined ? "Overrules the flag." : `Overrules the flag. ${onwards(overrule)}`,
        ...(overrule === undefined ? { withheld: NO_OVERRIDE } : {}),
        presets,
        onCarryOn: (reason) => deciding.onOverrule(job.id, reason),
      }}
      sendBack={{
        consequence: sendsBy === "redirect" ? REDIRECTS : RESTARTS,
        ...(sendsBy === undefined ? { withheld: NEITHER } : {}),
        onSendBack: (note) =>
          sendsBy === "redirect"
            ? deciding.onRedirect(job.id, sentBackWords(flags, note))
            : deciding.onSendBack(job.id, note),
      }}
      disabled={deciding.stale || deciding.acting}
      disabledNote={deciding.stale ? STALE : undefined}
      pending={pending}
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
      disabledNote={answering?.stale ? STALE : undefined}
      pending={answering !== undefined && answering.acting && answering.actingAct === "answer_command"}
    />
  );
}
