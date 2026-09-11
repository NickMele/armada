// What the panel says about the step you are looking at, before its story
// starts: the step's own short facts, the band above the story, and the box a
// question is answered in.
//
// **The band is what happened, and why you are looking at this step.** Waiting,
// stopped and failed are three kinds of stopped and never share a tone: waiting
// on you is amber and carries no surface, because everything mechanical cleared
// and the workflow is working; a Job that is over is red.
//
// **One band, and the reason you are here wins it.** A drone's open question
// outranks the render's own notice — nothing else on this step is what a person
// is here for while one is open, and the two would otherwise both claim the
// band.
//
// Split out of `JobDetail.tsx` at the 900-line line, for `chapters.tsx`'s
// reason and on the seam beside it: that file assembles a Job's screen and
// holds the open state of one reading, and everything here is a sentence about
// one step, decided from what arrived. Nothing here holds state, and nothing
// here knows which step is selected — it is handed the one that is.
//
// The header's counterpart is `heading.tsx`: what is there changes when the Job
// does, and what is here changes when the selection does.

import {
  DroneQuestion,
  GAMING_PATTERN,
  GamingFlags,
  JOB_LIFECYCLE,
  Refusals,
} from "@armada/components";
import type { JobDetailField } from "@armada/components";
import type { StepNotice } from "./InsideAJob";
import type { ReactNode } from "react";

import type { JobDetail as JobWhole, JobSummary, StepDetail } from "@armada/protocol";
import { span } from "./duration";
import { sentenceOf } from "./gates";
import { Opening, openKept, type Opens } from "./phases";
import { recourseOf, type Recourse } from "./recovery";
import { refusedIn } from "./refused";
import { escalation } from "./render";
import { steeringOf } from "./steering";
import { stoppedAt } from "./stopped";

/**
 * The step's own short facts. **Figures, never a chart** — a filled bar reads
 * as progress and a step has no percentage.
 *
 * **The attempt is which run this is**, from `attempts`, and it is absent on a
 * step nothing has entered rather than drawn as a zero. A step run once still
 * says `Attempt 1`, because the drawing does and because it is the fact a
 * person checks before deciding a Drone is going in circles.
 */
export function fieldsOf(step: StepDetail, now: number): JobDetailField[] {
  const running = step.state === "running" || step.state === "retrying";
  const elapsed = running
    ? span(step.entered_at, now)
    : step.entered_at === step.updated_at
      ? undefined
      : span(step.entered_at, step.updated_at);
  return [
    ...(elapsed === undefined
      ? []
      : [{ label: running ? "Running for" : "Took", value: elapsed, mono: true }]),
    ...(step.attempts.length === 0
      ? []
      : [{ label: "Attempt", value: String(step.attempts.length), mono: true }]),
  ];
}

/**
 * The band, where this Job's drone is waiting on an answer.
 *
 * **`waiting`, the same tone `reviewing` takes**, and for the reason the screen
 * already gives it: everything mechanical has cleared and nothing advances until
 * a person answers. Amber, never red — a drone that asked rather than guessed
 * did the right thing.
 *
 * It says only that a question is open. What was asked, and what each answer
 * commits to, is the box beneath: this band is scanned and that is read.
 */
export function askingOf(whole: JobWhole | null): StepNotice | undefined {
  if (whole?.asking === undefined) return undefined;
  return {
    tone: "waiting",
    title: "The drone asked a question and is waiting for you.",
  };
}

/**
 * The question itself. `undefined` where nothing is outstanding, which is every
 * drone that knows what it is doing.
 *
 * **The elapsed is computed here and nowhere else.** `asked_at` crosses once and
 * nothing on the wire ticks, so the surface subtracts for itself — the same
 * arrangement `JudgeInFlight.since` has, on the `now` this screen re-renders
 * from.
 *
 * **Stale and in-flight both disable, and each says which.** A window showing a
 * reading it knows is not live must not send an answer against it.
 */
export function questionOf(
  whole: JobWhole | null,
  jobId: string,
  now: number,
  stale: boolean,
  acting: boolean,
  onAnswer: (jobId: string, questionId: string, chose: string) => void,
): ReactNode {
  const asking = whole?.asking;
  if (asking === undefined) return undefined;
  return (
    <DroneQuestion
      question={asking.question}
      options={asking.options}
      waiting={span(asking.asked_at, now) ?? undefined}
      disabled={stale || acting}
      disabledNote={
        stale
          ? "This Job is not live, so nothing can be sent. The drone is still waiting."
          : acting
            ? "That answer is already on its way to the drone."
            : undefined
      }
      onAnswer={(label) => onAnswer(jobId, asking.question_id, label)}
    />
  );
}

/**
 * The band above the story: what stopped this step, and what the machine that
 * stopped it actually found.
 *
 * **`flagged` renders here, and it is the whole point of the band on a step
 * where the evidence was disputed.** Everything mechanical can pass, every
 * criterion can be met, and the step still stop — and a person reading
 * `7 of 7 passed`, `2 of 2 met` and a stopped step, with nothing reconciling
 * them, can only conclude the app is broken. The gaming check's finding is
 * what reconciles them, and it was reachable only by pressing *Overrule the
 * flag* and reading it in the dialog that confirms the act it exists to
 * inform.
 *
 * **And the finding is drawn with the question it answers.** A pattern and a
 * quoted line say what was seen and not what was claimed about it, so the band
 * used to reconcile a stopped step with a clean gate by asking a person to
 * trust the check. #580: it read `assertion_weakened` over a rustdoc sentence,
 * and only the diff said so.
 *
 * **What each act does is not here.** That was ninety words describing four
 * acts, in the imperative, detached from every control it named. Each sentence
 * is on its act's tooltip now, with its binding, and the band says where the
 * step stands.
 *
 * **`refused` renders here for `flagged`'s reason.** The title names a trigger
 * and the band is where the evidence for it goes: `blocked_by_policy` named a
 * policy, nothing named what the policy stopped, and a person told to unblock a
 * Job had nothing saying what from. It rides on every trigger rather than that
 * one, because a Drone denied the command it needed escalates as `stalled` or
 * `silent` just as often.
 */
export function noticeOf(
  job: JobSummary,
  whole: JobWhole | null,
  render: string,
  step: StepDetail,
  opens: Opens,
): StepNotice | undefined {
  if (render === "reviewing") {
    return {
      tone: "waiting",
      // **It opens on what is true, not on a denial.** "Nothing is wrong"
      // answers a worry the reader had not had yet, and reads as reassurance
      // that something is.
      title: "This Job needs your review before it can go on.",
    };
  }
  // **The one thing a redirect into a healthy drone leaves behind.** That job
  // is `running` before the send and `running` after the answer, so nothing
  // else on the screen says a person spoke to it — and a press that changed
  // nothing reads as a press that failed. `note` and never a hue: nothing is
  // wrong, and the band is not announcing a stop.
  if (render === "working") {
    const sent = steeringOf(job, whole).sent;
    return sent === undefined ? undefined : { tone: "note", children: sent };
  }
  if (render !== "stopped") return undefined;
  const reason = escalation(job);
  const at = whole === null ? undefined : stoppedAt(whole);
  const said = [
    reason?.verb,
    at === undefined ? undefined : `stopped at ${at.label}`,
    at?.check,
  ].filter((part) => part != null);
  // **The line that named the file a person could not open.** This band is the
  // first thing read on a Job that stopped and it has always ended with the log
  // path — as text, which is where `#246` was reported from. The strip opens it
  // too, on the Check's own row; this one is where somebody is already looking.
  const log = at?.outputPath;
  const recourse = recourseOf(job, whole);
  // What the Job reached for and was refused. **Beside the flags and for their
  // reason**: the title names a trigger and this is the evidence for it, so a
  // person told a policy stopped the Job is told what the policy stopped. It is
  // `undefined` on a Job refused nothing, which is most of them.
  const refused = refusedIn(whole);
  const flagged = step.flagged;
  // **Present only where `stopped_by` is `gate_undecided`** — `Stuck.undecided`
  // says so, so nothing here re-checks the trigger before drawing it. Fleet
  // writes it lower-case and unpunctuated, like a log line; `sentenceOf` is
  // what every other reader of this same field takes to say it as a sentence.
  const undecided = whole?.stuck?.undecided;
  const shown =
    undecided !== undefined ||
    flagged.length > 0 ||
    refused !== undefined ||
    recourse.sent !== undefined;
  return {
    // A Job that is over is red; one holding with a live Drone is not, because
    // a person deciding what happens next is not a failure.
    //
    // **Terminality is read, never listed**, which is `frozen.ts`'s rule and is
    // what this sentence has always described. Naming `escalated` alone was the
    // same claim with one status hard-coded into it, and `awaiting_repair` is
    // the status that made the difference visible: a spent retry budget holds
    // the Job for a person, nothing has failed, and it would have drawn red.
    // #208.
    tone: JOB_LIFECYCLE[job.status]?.terminal === false ? "stopped" : "failed",
    title: (
      <>
        {said.length === 0 ? "This Job stopped." : said.join(" · ")}
        {log === undefined ? null : (
          <>
            {" · "}
            <Opening path={log} what="check" opens={opens} />
          </>
        )}
      </>
    ),
    says: saysOf(recourse, refused?.again),
    children: !shown ? undefined : (
      <>
        {/* Beside the headline, ahead of everything else the band says — the
            one fact #633 found missing was why the gate could not decide, and
            the headline above already says only that it could not. */}
        {undecided === undefined ? null : (
          <span className="block">{sentenceOf(undecided)}</span>
        )}
        {flagged.length === 0 ? null : (
          <GamingFlags
            flags={flagged.map((flag) => ({
              ...flag,
              // The registry carries a verb per pattern since #279; the wire
              // spelling is the key, never the copy. A pattern with no row
              // falls back to it rather than rendering nothing.
              verb: GAMING_PATTERN[flag.pattern]?.verb ?? undefined,
              // Named rather than spread: the wire says `brief_path`, as it
              // does on a criterion's verdict, and the component takes the
              // path of anything it can open under one word.
              brief: flag.brief_path,
            }))}
            citation="whole"
            // The brief opens where the flag is read. It is the third of the
            // records `phases.tsx` already opens, kept in the same directory
            // and by the same rule — so it goes through `openKept` rather
            // than growing a second answer to a failed open.
            onOpenBrief={(brief) => openKept(opens, { kept: brief, what: "brief" })}
          />
        )}
        {refused === undefined ? null : <Refusals {...refused} again={undefined} />}
        {recourse.sent === undefined ? null : <span className="block">{recourse.sent}</span>}
      </>
    ),
  };
}

/**
 * What a stopped band's title means, on hover over it. **The evidence stays on
 * the band**: what was flagged, what was refused and the answer to a redirect
 * are facts about this Job, and these sentences are the reading of them.
 *
 * **A refusal says only how to lift it.** That the drone is paused and why
 * Restart is missing are both on the screen already, in the badge and in the
 * controls, and four sentences in one hover read as none.
 */
function saysOf(recourse: Recourse, again: string | undefined): string {
  if (again !== undefined) return again;
  const stands =
    recourse.sent === undefined ? recourse.stands : recourse.stands.replace(`${recourse.sent} `, "");
  return [stands, recourse.withheld, again]
    .filter((line) => line !== undefined && line !== "")
    .join(" ");
}
