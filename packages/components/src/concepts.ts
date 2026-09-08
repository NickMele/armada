import type { ReactNode } from "react";
import { phaseSaid } from "./compositions/PhaseCard/PhaseCard";

/**
 * What a word naming an Armada concept means, in one sentence, written once.
 *
 * **Standing copy rather than values.** Every sentence here is true of every
 * Job on every workflow, which is exactly what makes it a thing a surface must
 * not retype: the moment two screens each explain what a Check is, they can
 * disagree, and the difference between a Check and a Judge is the distinction
 * the whole gate rests on.
 *
 * **Keyed by the word a reader sees, so no call site holds a string.** A fact
 * label, a chapter title, a region's eyebrow and a header field are all the
 * same question — *what is this thing* — asked of the same vocabulary. Looking
 * the answer up by the visible word is what keeps this one table instead of one
 * annotation per component, which is how a hover pass becomes two hundred
 * hand-written strings that drift.
 *
 * **The three gate tiers are not here.** `PhaseCard` already writes them, keyed
 * by kind *and* state, because a tier that can never hold a step cannot say
 * what a tier holding one says — see `phaseSaid`. Those entries below read it
 * rather than carrying a second sentence about the same thing.
 *
 * **What does not get an entry.** A criterion sentence, a Judge's grounds, a
 * log line and a step's own name already read as themselves; a hover that
 * restates the visible text is worse than none.
 */
const SAID: Readonly<Record<string, string>> = {
  // The gate tiers, read from the one place they are written. `current` is the
  // state a tier holding this step stands in, which is what a reader asking
  // "what is a Check" is looking at when they ask it.
  checks: phaseSaid("checks", "current") as string,
  judge: phaseSaid("judge", "current") as string,
  waiting: phaseSaid("human", "waiting") as string,

  produced:
    "The work product of this step — the files the Drone changed, and anything else it wrote. " +
    "It is what the Checks run against and what the Judge reads.",
  cleared:
    "The Checks this step passed. A step advances when everything its gate asks for has cleared.",
  attempt:
    "One run of this step by a Drone. A refused step is handed back and tried again up to the " +
    "workflow's limit, and every attempt keeps its own log.",
  stopped:
    "Every attempt this step allows, spent on the same failure. Nothing is lost and nothing " +
    "advances; the next move is yours.",

  "drone instructions":
    "What the Drone was given at the start of this step: the brief, this step's own task, and " +
    "the criteria it will be judged against. It never sees the Judge's answer.",
  "activity log":
    "Every turn Fleet recorded on this step — what the Drone read, wrote and ran, and what Fleet " +
    "did about it.",

  worktree:
    "A checkout of the repository cut for this Job alone. Nothing outside it is touched, and it " +
    "is reclaimed when the Job is cleaned up.",
  branch:
    "The branch this Job's worktree sits on. Every commit its Drone makes lands here and nowhere " +
    "else.",
  manifest:
    "The repository's own file, declaring its Checks and the commands behind them. Fleet reads " +
    "it; a Drone cannot change what it runs.",
  workflow:
    "The steps this Job runs, in order, and what it takes to advance past each one. It is frozen " +
    "at dispatch, so editing the definition does not change a Job already running.",
  "job log":
    "Everything Fleet recorded about this Job, one line per event. The screen you are reading is " +
    "this file read back.",
  transcript: "Every turn between Fleet and the Drone, as it was sent. The activity log summarises it.",
  drone:
    "The coding agent working this Job. Bridge never talks to one — every instruction and every " +
    "answer goes through Fleet.",

  brief:
    "What was asked for, in the requester's own words, with the acceptance criteria written " +
    "beside it. Every step is read against it.",
  elapsed:
    "Wall clock since this Job was dispatched, including whatever it spent queued or waiting on " +
    "a person.",
  "spend, estimated":
    "What the Job has cost so far, from the tokens its Drone reported. Estimated rather than " +
    "measured, which is why it is marked approximate.",

  "the run": "The workflow's steps, in the order this Job ran them. The order is the workflow's and never changes.",
  "where things are":
    "What this Job holds on disk and what identifies it. The screen above exists so nobody needs " +
    "these; they are here for when you want one anyway.",
  "what this job holds":
    "What is on this machine right now — the processes Fleet started, the disk the worktree is " +
    "using, and the last lines Fleet wrote about the Job itself. Everything else on this screen " +
    "is what happened.",
  "what it left behind":
    "What the Job recorded, kept after it ended — its moves, its Drone's turns, what it touched " +
    "and what it claimed.",

  criterion:
    "One thing the work has to be true of, written when the Job was dispatched and frozen from " +
    "then on. The Judge answers per criterion, never overall.",
  // Keys nothing on screen spells, so the call site names the concept rather
  // than the word. Written here all the same: a sentence about the vocabulary
  // has one home whether or not a label happens to carry its name.
  "#":
    "The criterion's frozen position in the brief. A citation names this number, never the row's " +
    "place on screen, so the order here is the brief's and is never sorted.",
  "the split":
    "How many of the panel refused. One veto is a refusal whatever its size — the count is what " +
    "tells a lone dissent from a unanimous one, which are different situations for you and the " +
    "same verdict for the Job.",
};

/**
 * What a word means, or nothing where it names no concept.
 *
 * **The word is normalised, not matched exactly**, so `Attempt 2` and
 * `Attempt 3` are the same question as `Attempt 1` — a run tree with three
 * attempts on one step would otherwise need three entries and get two of them
 * wrong the day a fourth appears.
 *
 * A `ReactNode` that is not a string answers nothing: a label built from
 * elements is a label whose text this cannot read, and guessing at it is how a
 * hover ends up describing the wrong thing.
 */
export function conceptSaid(word: ReactNode): string | undefined {
  if (typeof word !== "string") return undefined;
  const key = word.trim().toLowerCase().replace(/\s+/g, " ").replace(/ \d+$/, "");
  return SAID[key];
}
