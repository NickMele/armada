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
  JudgeRefusal,
  Refusals,
} from "@armada/components";
import type { Explaining, JobDetailField } from "@armada/components";
import type { StepNotice } from "./InsideAJob";
import { useState } from "react";
import type { ReactNode } from "react";

import type { CommandAnswer, CommandInFlight, Criterion, JobDetail as JobWhole, JobSummary, StepDetail } from "@armada/protocol";
import type { CommandExplainedRead } from "./calls";
import { answerNamed, offeredOf, said } from "./copy";
import { span } from "./duration";
import { panelsOf, sentenceOf } from "./gates";
import { Opening, openKept, type Opens } from "./phases";
import { recourseOf, type Recourse } from "./recovery";
import { refusedIn, shownOf } from "./refused";
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
  if (whole?.asking !== undefined) {
    return { tone: "waiting", title: "The drone asked a question and is waiting for you." };
  }
  // A command it was not given, on a job set to Ask me first. **The same tone for the
  // same reason**: the drone stopped to ask rather than work round a refusal,
  // and nothing moves until a person answers.
  if (whole?.command_waiting !== undefined) {
    return {
      tone: "waiting",
      title: "The drone wants to run a command it was not given, and is waiting for you.",
    };
  }
  return undefined;
}

/** Why the answers are off, where the reading is not live. */
const STALE_NOTE = "This Job is not live, so nothing can be sent. The drone is still waiting.";

/** Why the answers are off, where one is already out. */
const SENDING_NOTE = "That answer is already on its way to the drone.";

/**
 * What a command's answer still does when it comes late. **Fleet holds a
 * command a little under what the harness waits**, then tells the drone to
 * hold and carries the answer in as its next turn — so an answer is never too
 * late, and a person deciding slowly should not rush for fear it is.
 */
const LATE_ANSWER =
  "If the drone stops waiting before you answer, it is told to hold, and your answer reaches it as its next turn.";

/**
 * How a person's answer to a command the drone was not given is sent, and
 * whether it can be. **One value for both places a person meets one** — the
 * command a drone is waiting on and a refused row on a stopped job — because
 * the call id is what says which, and both are off for the same two reasons.
 */
export type Answering = {
  /**
   * The answer, and the words typed with it where there are any. **Only a
   * reject reads them**, which is Fleet's rule rather than this screen's.
   */
  send: (call: string, answer: CommandAnswer, note?: string) => void;
  /** What is shown is not live, so nothing may be sent against it. */
  stale: boolean;
  /** An act on this job is already out. */
  acting: boolean;
};

export function answeringOf(
  jobId: string,
  stale: boolean,
  acting: boolean,
  onAnswerCommand: (jobId: string, call: string, answer: CommandAnswer, note?: string) => void,
): Answering {
  return {
    send: (call, answer, note) => onAnswerCommand(jobId, call, answer, note),
    stale,
    acting,
  };
}

/** Why a refused row's answers are off, where the reading is not live. */
const REFUSED_STALE = "This Job is not live, so nothing can be sent.";

/** Why they are off, where an answer is already out. */
const REFUSED_SENDING = "That answer is already on its way to Fleet.";

/**
 * The command a drone is waiting on a person to allow. `undefined` where
 * nothing waits, which is every job at Stop and wait for me or Run it, and most
 * at Ask me first.
 *
 * **The question's own box**, because it is the same moment — a drone stopped
 * inside a call, a closed set of answers, a person who has to pick one — and a
 * second composition would be two boxes for one kind of wait. The command is
 * what is asked, in mono because it is what the drone sent; each answer is one
 * Fleet offered, in its order, with what it commits to under it.
 *
 * **Aged here and nowhere else**, on `questionOf`'s terms, and a command cut by
 * the wire says so on a line of its own, as a refused row does.
 */
export function commandOf(
  whole: JobWhole | null,
  now: number,
  answering: Answering,
  explain?: ExplainOne,
): ReactNode {
  const waiting = whole?.command_waiting;
  if (waiting === undefined) return undefined;
  // Keyed by the call, so a reading of one command never outlives it.
  return (
    <CommandWaiting
      key={waiting.call}
      waiting={waiting}
      now={now}
      answering={answering}
      explain={explain}
    />
  );
}

/** Asking what this one command does. The screen is handed the way to ask. */
type ExplainOne = (call: string) => Promise<CommandExplainedRead>;

/** The field a refusal carries, and what becomes of the words typed in it. */
const REFUSAL_NOTE = "Note (optional)";
const REFUSAL_NOTE_SAYS = "Your words go to the drone with the refusal.";

/** What is said where the reading did not arrive and the refusal named no reason. */
const NO_READING = "Fleet did not explain this command.";

/**
 * The box, and the one thing in this file that holds state: what came back from
 * asking what the command does.
 *
 * **Held here rather than on the published state**, on `useCallArguments`'s
 * terms — one person asks about one call, the answer does not move once it has
 * arrived, and the window does not re-render because somebody read a paragraph.
 */
function CommandWaiting({
  waiting,
  now,
  answering,
  explain,
}: {
  waiting: CommandInFlight;
  now: number;
  answering: Answering;
  explain?: ExplainOne;
}): ReactNode {
  const [reading, setReading] = useState<Explaining>({ state: "ready" });
  const offered = offeredOf(waiting.offers);
  const cut = shownOf(waiting);

  function ask(): void {
    if (explain === undefined) return;
    setReading({ state: "asking" });
    void explain(waiting.call).then(
      (answer) =>
        setReading(
          answer.ok
            ? {
                state: "read",
                explanation: answer.explained.explanation,
                model: answer.explained.model,
              }
            : { state: "failed", why: said(answer.outcome) || NO_READING },
        ),
      // A rejected call is main gone, which is the window closing. Recorded as
      // an absence so the control is not left reading for the rest of its life.
      () => setReading({ state: "failed", why: NO_READING }),
    );
  }

  return (
    <DroneQuestion
      question={
        <>
          {waiting.detail === "" ? (
            `The drone wants to use ${waiting.tool}.`
          ) : (
            <>
              The drone wants to run <span className="mono">{waiting.detail}</span>
            </>
          )}
          {cut === undefined ? null : <span className="block">{cut.size}</span>}
        </>
      }
      options={offered.map(({ offer, label, means }) => ({
        label,
        consequence: means,
        // Which answer reads words is Fleet's rule, so it is read off the
        // wire's own spelling and never off the words on the control.
        ...(offer === "reject" ? { noteLabel: REFUSAL_NOTE, noteSays: REFUSAL_NOTE_SAYS } : {}),
      }))}
      waiting={span(waiting.asked_at, now) ?? undefined}
      disabled={answering.stale || answering.acting}
      disabledNote={answering.stale ? STALE_NOTE : answering.acting ? SENDING_NOTE : undefined}
      redirectNote={LATE_ANSWER}
      answersLabel="Your answers"
      explain={explain === undefined ? undefined : reading}
      onExplain={ask}
      onAnswer={(label, note) => {
        const chose = offered.find((one) => one.label === label);
        if (chose !== undefined) answering.send(waiting.call, chose.offer, note);
      }}
    />
  );
}

/**
 * What the drone is waiting on a person for, in the slot a question takes. Its
 * own question, a command it was not given, or both: a drone held inside a
 * permission call is rarely asking as well, and when it is, neither box may
 * hide the other.
 */
export function waitingOf(question: ReactNode, command: ReactNode): ReactNode {
  if (command === undefined) return question;
  if (question === undefined) return command;
  return (
    <>
      {question}
      {command}
    </>
  );
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
      disabledNote={stale ? STALE_NOTE : acting ? SENDING_NOTE : undefined}
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
 *
 * **A refused row Fleet offers answers on is answered there**, through
 * `answering` — the same value the command a Drone is waiting on is answered
 * through. Without it the rows draw their controls off, which only a test
 * that is not about them does.
 */
export function noticeOf(
  job: JobSummary,
  whole: JobWhole | null,
  render: string,
  step: StepDetail,
  opens: Opens,
  answering?: Answering,
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
  // The Judge's own refused criteria — `judgeRefusalsOf`'s own doc says why.
  const judgeRefusals = judgeRefusalsOf(step, whole?.acceptance_criteria ?? []);
  // **Present only where `stopped_by` is `gate_undecided`** — `Stuck.undecided`
  // says so, so nothing here re-checks the trigger before drawing it. Fleet
  // writes it lower-case and unpunctuated, like a log line; `sentenceOf` is
  // what every other reader of this same field takes to say it as a sentence.
  const undecided = whole?.stuck?.undecided;
  const shown =
    undecided !== undefined ||
    judgeRefusals !== null ||
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
        {judgeRefusals}
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
        {refused === undefined ? null : (
          <Refusals
            {...refused}
            again={undefined}
            onAnswer={(call, name) => {
              const chose = answerNamed(name);
              if (chose !== undefined) answering?.send(call, chose);
            }}
            disabled={answering === undefined || answering.stale || answering.acting}
            disabledNote={
              answering?.stale ? REFUSED_STALE : answering?.acting ? REFUSED_SENDING : undefined
            }
          />
        )}
        {recourse.sent === undefined ? null : <span className="block">{recourse.sent}</span>}
      </>
    ),
  };
}

/**
 * The Judge's own refused criteria, for the band on a step it stopped over —
 * `flagged`'s counterpart, and `#689`'s gap: the finding sat under the Checks,
 * three chapters down. `null` where nothing was refused this attempt.
 */
function judgeRefusalsOf(step: StepDetail, criteria: readonly Criterion[]): ReactNode {
  const refused = panelsOf(step, criteria).filter((panel) => panel.verdict === "not_met");
  if (refused.length === 0) return null;
  return (
    <>
      {refused.map((panel) => {
        const finding = panel.refused[0];
        if (finding === undefined) return null;
        return (
          <JudgeRefusal
            key={panel.criterionId}
            heading={panel.criterion?.text ?? panel.criterionId}
            finding={{
              ...(finding.expected === undefined ? {} : { expected: finding.expected }),
              ...(finding.produced === undefined ? {} : { produced: finding.produced }),
              ...(finding.consequence === undefined ? {} : { consequence: finding.consequence }),
            }}
          />
        );
      })}
    </>
  );
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
