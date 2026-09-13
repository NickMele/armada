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
// **The chooser offers what the Job's Drones named, and nothing else.** Fleet
// sends the list and refuses anything outside it, so a choice is always a file
// that was in the worktree when a Drone submitted it. One spec draws no menu.
//
// **A press's sets are drawn beside the step's frames, never among them.** Each
// is the same `FramesShown` the step's own frames are, headed by the spec that
// produced it and the moment it ran.

import { useCallback, useEffect, useRef, useState } from "react";

import type { ShowAgainChoices, ShowAgainOffer, ShownAgainSet } from "@armada/components";
import type { KeptFrame, NamedSpec, Outcome, ShowAgain } from "@armada/protocol";
import { said } from "./copy";
import { shownFrames, type Frames } from "./frames";

/** Asking Fleet for a press, as the screen's caller hands it in. */
export type ShowAgainCall = (jobId: string, spec?: string) => Promise<Outcome>;

/** What this window knows about a press it sent. */
export type Pressing = {
  /** A press from this window is out. */
  pressing: boolean;
  /** What the last one came to, where it was not a new set. */
  said?: string;
  /**
   * The spec the next press will run, where the Job named any. **Held in this
   * window and never sent until the press is**, so a person who opens the menu
   * and closes it again has asked Fleet for nothing.
   */
  chosen?: string;
  choose: (spec: string) => void;
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
  // Absent until a person picks, which is what makes a press with no choice
  // the press Fleet has always answered: no spec is sent.
  const [chosen, setChosen] = useState<string | undefined>(undefined);
  // The Job an answer is about, so one landing after the person moved to
  // another Job is dropped rather than said on the wrong one.
  const current = useRef(jobId);
  current.current = jobId;
  useEffect(() => {
    setPressing(false);
    setSaid(undefined);
    setChosen(undefined);
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
    void call(asked, chosen).then(
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
  }, [call, jobId, chosen]);

  return {
    pressing,
    said,
    ...(chosen === undefined ? {} : { chosen }),
    choose: setChosen,
    press,
  };
}

/** What the Shown chapter draws for a press, where anything. */
export type Again = {
  /** Absent where no control applies to this step. */
  offer?: ShowAgainOffer;
  /** Absent where there is nothing to choose between. */
  choices?: ShowAgainChoices;
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
      heading: headingOf(set.pressed_at, set.spec),
      frames: shownFrames(set.frames, frames),
    }));
  const offer = offerOf(facts, stepId, pressing.pressing, pressing.chosen);
  if (offer === undefined && sets.length === 0) return undefined;
  const choices = offer === undefined ? undefined : choicesOf(facts, pressing);
  return {
    ...(offer === undefined ? {} : { offer }),
    ...(choices === undefined ? {} : { choices }),
    sets,
    ...(pressing.said === undefined ? {} : { said: pressing.said }),
    onShow: pressing.press,
  };
}

/**
 * What a set is headed by.
 *
 * **The spec leads where the record holds one**, because two presses on one
 * step may now have run different specs and the moment alone would not say
 * which. A set kept before Fleet recorded it reads as it always did.
 */
function headingOf(at: string, spec: string | undefined): string {
  const moment = when(at);
  return spec === undefined || spec === "" ? `Shown again ${moment}` : `${spec}, shown again ${moment}`;
}

/**
 * Every spec this Job's Drones named, and which the next press will run.
 *
 * **A Fleet before 13.1 sends no list**, so the one spec it does send is the
 * whole of it — which draws no chooser, exactly as it drew none before.
 */
export function specsOf(facts: ShowAgain): NamedSpec[] {
  if (facts.specs !== undefined && facts.specs.length > 0) return facts.specs;
  return facts.spec === undefined ? [] : [facts.spec];
}

/** The chooser, where a Job's Drones named more than one spec. */
export function choicesOf(facts: ShowAgain, pressing: Pressing): ShowAgainChoices | undefined {
  const named = specsOf(facts);
  if (named.length < 2) return undefined;
  const chosen = named.find((one) => one.spec === pressing.chosen) ?? named[0]!;
  return { specs: named.map((one) => one.spec), chosen: chosen.spec, onChoose: pressing.choose };
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
  chosen?: string,
): ShowAgainOffer | undefined {
  const last = facts.spec;
  if (last === undefined || last.step_id !== stepId) return undefined;
  // What a person picked, or the last a Drone named — which is what a press
  // with no pick runs, and what Fleet answers with.
  const spec = specsOf(facts).find((one) => one.spec === chosen) ?? last;
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
const NOT_ANSWERED = "Fleet did not answer";

function specGone(spec: string): string {
  return `${spec} is no longer in this Job's worktree. A later run renamed or deleted it.`;
}
