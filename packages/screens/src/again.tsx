// Asking a Job to show its work again, read off what Fleet said about it.
//
// **Fleet sends facts and this file reads them.** `ShowAgain` carries five
// things Fleet checked — a harness, a worktree, a spec, whether it is still on
// disk, a Drone at work — and no verdict, so the order a reason is chosen in
// and the sentence it is said in are here, once. The order is Fleet's own
// refusal order, so the control and a press that raced a change say the same.
//
// **The control lives on the step whose spec it reruns.** A press reruns the
// last spec a Drone named, and that spec came from one step's evidence; the
// Shown chapter of that step is where its frames already are. A Job no Drone
// named a spec for draws no control at all — that is every Job whose workflow
// never asked a step to show its work, and a chapter saying so on every step of
// every such Job would be the noise `chapters.tsx` refuses to draw for a step
// that captured nothing. Fleet still refuses the press in words, for a caller
// that reaches it some other way.
//
// **A press's sets are drawn beside the step's frames, never among them.** Each
// is the same `FramesShown` the step's own frames are, under the moment it ran.

import { useCallback, useEffect, useRef, useState } from "react";

import type { ShowAgainOffer, ShownAgainSet } from "@armada/components";
import type { KeptFrame, Outcome, ShowAgain } from "@armada/protocol";
import { said } from "./copy";
import { shownFrames, type Frames } from "./frames";

/** Asking Fleet for a press, as the screen's caller hands it in. */
export type ShowAgainCall = (jobId: string) => Promise<Outcome>;

/** What this window knows about a press it sent. */
export type Pressing = {
  /** A press from this window is out. */
  pressing: boolean;
  /** What the last one came to, where it was not a new set. */
  said?: string;
  press: () => void;
};

/**
 * Send a press, hold whether one is out, and ask for the frames the Job's
 * presses kept on the open step.
 *
 * **The asking is here rather than in the screen** so the screen carries one
 * line for all of this: a set's frames are fetched when the step is opened, for
 * the reason the step's own are — `frames.ts` says why.
 */
export function useShowAgain(
  call: ShowAgainCall | undefined,
  jobId: string,
  facts: ShowAgain | undefined,
  stepId: string | undefined,
  frames: Frames,
): Pressing {
  const [pressing, setPressing] = useState(false);
  const [said, setSaid] = useState<string | undefined>(undefined);
  // The Job an answer is about, so one landing after the person moved to
  // another Job is dropped rather than said on the wrong one.
  const current = useRef(jobId);
  current.current = jobId;
  useEffect(() => {
    setPressing(false);
    setSaid(undefined);
  }, [jobId]);

  const wanted = pressedFramesOf(facts, stepId);
  useEffect(() => {
    if (wanted.length > 0) frames.want(wanted);
  }, [wanted, frames]);

  const press = useCallback(() => {
    if (call === undefined) return;
    const asked = jobId;
    setPressing(true);
    setSaid(undefined);
    void call(asked).then(
      (outcome) => {
        if (current.current !== asked) return;
        setPressing(false);
        setSaid(saidOf(outcome));
      },
      () => {
        if (current.current !== asked) return;
        setPressing(false);
        setSaid(NOT_ANSWERED);
      },
    );
  }, [call, jobId]);

  return { pressing, said, press };
}

/** What the Shown chapter draws for a press, where anything. */
export type Again = {
  /** Absent where no control applies to this step. */
  offer?: ShowAgainOffer;
  sets: ShownAgainSet[];
  said?: string;
  onShow: () => void;
};

/**
 * The control and the sets for one step, or nothing.
 *
 * **Nothing from an older Fleet**, which sends no facts: a control built on a
 * guess about what Fleet can do would be a press that fails.
 */
export function againOf(
  facts: ShowAgain | undefined,
  stepId: string,
  frames: Frames,
  pressing: Pressing,
): Again | undefined {
  if (facts === undefined) return undefined;
  const sets = facts.shown
    .filter((set) => set.step_id === stepId)
    .map((set) => ({
      key: String(set.press),
      heading: `Shown again ${when(set.pressed_at)}`,
      frames: shownFrames(set.frames, frames),
    }));
  const offer = offerOf(facts, stepId, pressing.pressing);
  if (offer === undefined && sets.length === 0) return undefined;
  return {
    ...(offer === undefined ? {} : { offer }),
    sets,
    ...(pressing.said === undefined ? {} : { said: pressing.said }),
    onShow: pressing.press,
  };
}

/**
 * Whether the press can run, read in Fleet's own refusal order.
 *
 * **Only on the step the spec came from**, and only where a Drone named one —
 * see this file's header.
 */
export function offerOf(
  facts: ShowAgain,
  stepId: string,
  pressing: boolean,
): ShowAgainOffer | undefined {
  const spec = facts.spec;
  if (spec === undefined || spec.step_id !== stepId) return undefined;
  if (pressing || facts.showing_since !== undefined) return { state: "showing", spec: spec.spec };
  if (!facts.harness) return { state: "cannot", why: NO_HARNESS };
  if (!facts.worktree_on_disk) return { state: "cannot", why: NO_WORKTREE };
  if (facts.drone_working) return { state: "cannot", why: DRONE_WORKING };
  if (!spec.on_disk) return { state: "cannot", why: specGone(spec.spec) };
  return { state: "ready", spec: spec.spec };
}

/** Every frame this Job's presses kept on one step, to be fetched. */
export function pressedFramesOf(
  facts: ShowAgain | undefined,
  stepId: string | undefined,
): KeptFrame[] {
  if (facts === undefined || stepId === undefined) return NONE;
  const found = facts.shown.filter((set) => set.step_id === stepId).flatMap((set) => set.frames);
  return found.length === 0 ? NONE : found;
}

/** The part of a Shown chapter's header a press adds. */
export function againSummary(again: Again | undefined): string | undefined {
  const count = again?.sets.length ?? 0;
  if (count === 0) return undefined;
  return count === 1 ? "shown again once" : `shown again ${count} times`;
}

/**
 * What a press answered, as the line beside the control.
 *
 * **A new set says nothing here**, because the set is the answer and it is on
 * screen. Every other answer is a sentence, since a press that changed nothing
 * on screen and said nothing is a dead click.
 */
export function saidOf(outcome: Outcome): string | undefined {
  if (outcome.ok) {
    return outcome.shown?.nothing === undefined ? undefined : sentence(outcome.shown.nothing);
  }
  // Fleet's own words for why it would not run, which name what is missing.
  // Every other refusal is Bridge's, and `copy.ts` is where those are worded.
  return outcome.why === "refused" ? sentence(outcome.error.message) : said(outcome);
}

/**
 * Fleet's words, in sentence case and ending as a sentence. Fleet writes its
 * refusals to be read inside a longer line, starting lower case.
 */
function sentence(said: string): string {
  const trimmed = said.trim();
  if (trimmed === "") return trimmed;
  const opened = trimmed[0]!.toUpperCase() + trimmed.slice(1);
  return /[.!?]$/.test(opened) ? opened : `${opened}.`;
}

/**
 * When a press ran, as a person reads it. **Local time and the day**, because
 * two presses are told apart by this line alone and the day is what separates
 * a press from this morning from one last week.
 */
function when(at: string): string {
  const parsed = new Date(at);
  if (Number.isNaN(parsed.getTime())) return at;
  return parsed.toLocaleString([], {
    month: "short",
    day: "numeric",
    hour: "2-digit",
    minute: "2-digit",
  });
}

/** The same empty list every time, so an effect keyed on it does not re-run. */
const NONE: KeptFrame[] = [];

const NO_HARNESS =
  "This repository's armada.yml declares no evidence harness, so there is nothing to run.";
const NO_WORKTREE =
  "This Job's worktree is gone, so there is nowhere to run the harness. A clean or a reclaim took it.";
const DRONE_WORKING =
  "A Drone is working in this Job's worktree. Show its work again once the Job stops.";
/** The call itself rejected, which is main gone rather than Fleet answering. */
const NOT_ANSWERED = "Fleet did not answer. The Job's log says whether the run finished.";

function specGone(spec: string): string {
  return `${spec} is no longer in this Job's worktree. A later run renamed or deleted it.`;
}
