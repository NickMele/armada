// What the command palette can reach in this app, and what it cannot reach yet.
//
// **Its own file so `App.tsx` does not grow another hundred lines**, and
// because this is one subject: the palette is a superset of the UI, so every
// act in the registry has to be answered here — either by doing it, or by
// saying in a few words why this build cannot.
//
// **The places a person can go are not here.** The rail and the palette are two
// controls over one roster, and it was in this file while the palette was the
// only one of them that drew it. It is `packages/shell/src/surfaces.ts` now,
// beside both.
//
// # A row that cannot act says why, except an act on a Job with none focused
//
// The contract's rule about a registered binding nothing answers is that the
// palette may draw it disabled or leave it out, **on a fact rather than on a
// list of exceptions kept in the app**. The registry's own `unbuilt` column is
// that fact for `p` and `X`, and those rows stay drawn. This file is the second
// kind: an act Bridge has, which cannot reach anything at this moment or from
// this surface.
//
// **An act on one Job, with no Job focused, is left out** (`absentIn`). The
// owner settled it on 2026-09-17: six dimmed rows all saying "no job focused"
// were the first thing the palette showed and said nothing a person could act
// on. The shortcuts are still learned, just once a Job is focused, where every
// one of those rows can act. Every other dormant row is still drawn and still
// says why.
//
// # What is dormant here is a to-do list, not a design
//
// Most of the reasons below name a control on job detail that the palette
// cannot press because `JobDetail` owns the dialog and `App` does not hold a
// handle to it. That is a wiring gap and it is written as one. The three that
// are not: nothing serves an attestation, nothing pilots a job, and observing
// is automatic on an open Job rather than an act.

import type { PaletteChoice } from "@armada/shell";

/** What the palette says when the act needs a Job and none is under the cursor. */
const NO_JOB = "no job focused";

/** The acts that need a Job to act on. With none focused, the palette leaves them out. */
const ON_ONE_JOB = ["open", "review", "attest", "redirect", "kill", "redispatch", "restart_step"] as const;

/**
 * The acts the palette leaves out rather than dims: an act on one Job, where
 * no Job is open and the Board's cursor is on none.
 */
export function absentIn(where: { reading: boolean; cursor: string | null }): readonly string[] {
  return !where.reading && where.cursor === null ? ON_ONE_JOB : [];
}

/**
 * Why each act cannot be chosen, by action id. An id absent from the answer is
 * an act this build can do.
 *
 * `unbuilt` rows are not here: the registry already carries their reason and
 * the shell reads it, so repeating them would be the second list this file
 * exists to avoid.
 */
export function dormantIn(where: {
  /** Whether one Job is open, which is what makes the context `detail`. */
  reading: boolean;
  /** The Job the cursor is on, on the Board. */
  cursor: string | null;
  /** Whether a failure is on screen for Copy debug info to copy. */
  failing: boolean;
}): Readonly<Record<string, string | undefined>> {
  const noJob = !where.reading && where.cursor === null ? NO_JOB : undefined;
  return {
    // The four signposts. On the Board each opens the Job the cursor is on,
    // which is `keys.ts`'s reading of them: none of Review, Attest or Redirect
    // happens on a list, and what the verb carries is why you are being sent
    // there. On detail you are already there.
    open: where.reading ? "the job is already open" : noJob,
    review: where.reading ? "the job's own Review control" : noJob,
    attest: where.reading ? "nothing serves an attestation yet" : noJob,
    redirect: where.reading ? "the job's own Redirect control" : noJob,

    // Copy debug info takes a `Failure`, and a healthy Job has none — the
    // finding `keys.ts` recorded when it claimed `c` and reached nothing. The
    // palette can do better than the row could: where a failure *is* on
    // screen, this is the act that copies it.
    copy_debug_info: where.failing ? undefined : "no failure on screen to copy",

    // Acts on one Job, and the Board's cursor is what they act on.
    kill: noJob,
    redispatch: noJob,
    restart_step: where.reading ? undefined : "open a job first",

    // Job detail's own controls and its own keys. `App` raises neither, so the
    // palette cannot press them — a wiring gap, written as one.
    report_job: "the job's own Report control",
    observe: "a job that is open is already observed",
    // The issue that answers it is on the Pilot row above, from the
    // registry, so it is not repeated here — and a literal issue
    // reference in a renderer string reads to the design gate as a hex
    // colour, which is its own finding.
    submit_for_verification: "nothing pilots a job yet",
    disclose: "the focused row, with h and l",
    open_log: "the story's own chapter",
    open_diff: "the story's own Produced chapter",
    open_stage: "the phase strip",

    // The Studio's own control, for the reason above it: `Studios` holds which
    // kind is being written, and `App` has no handle to it. A wiring gap,
    // written as one — the keys `N`, `V` and `S` reach all three.
    add_note: "the Studio's own + Node control",
    add_link: "the Studio's own + Node control",
    add_sketch: "the Studio's own + Node control",

    // Global acts with no surface behind them. The rail carries the Job Board
    // and nothing else — four disabled rows would be a promise Armada does not
    // keep, which is `Shell.tsx`'s own reasoning about the rail.
    //
    // Helm's dock is built (#944) and `⌘J` opens it, but that state is
    // `Shell.tsx`'s own `useDock` and `App` holds no handle to it — a wiring
    // gap, same shape as the four signposts above.
    helm: "⌘J opens it; not reachable from the palette yet",
    toggle_sidebar: "the rail does not hide yet",
    history: "no back and forward yet",
  };
}

/** What the app has to be able to do for a chosen row to mean anything. */
export type PaletteHands = {
  openJob: (jobId: string) => void;
  closeJob: () => void;
  compose: () => void;
  /**
   * Go to a place in Navigation, by the id the surfaces list gave it.
   *
   * **It takes the id, and it did not have to before.** With one destination
   * in that section every row meant the same thing, so the argument would have
   * been ignored; with two, a row that ignored it would go to the wrong screen
   * — silently, and only for whoever chose the second one.
   */
  surface: (surfaceId: string) => void;
  /**
   * Go to the Manifest surface with one Check or Command picked.
   *
   * **It selects; it does not run.** Journey 9 has the palette opening the
   * surface with that entry chosen, and a row that started a destructive
   * Command straight off a list of forty would be an act nobody confirmed —
   * which is the one confirmation that surface exists to carry.
   */
  run: (entryId: string) => void;
  filter: (tabId: string) => void;
  search: () => void;
  copyDebugInfo: () => void;
  confirm: (act: "kill_job" | "redispatch" | "restart_step", jobId: string) => void;
  /**
   * Go to the settings row's own screen, by the id `settings` was given.
   *
   * **One row today, so one id.** `fleet_settings` goes to the Settings
   * screen; the switch lives with the app that owns navigation rather than
   * here, the way `surface` and `run` already read their id against the
   * screen that answers for it.
   */
  openSetting: (id: string) => void;
};

/**
 * Carry out a chosen row.
 *
 * **The Job an act acts on is the open one, or the one under the cursor** —
 * in that order, because a Job read whole is unambiguously what is in front of
 * you and the Board's cursor is only what is in front of you when nothing is.
 *
 * **Nothing here decides whether the palette closes.** Choosing a row closes
 * it, which is the component's rule and the right one — every act below either
 * moves the screen or puts the cursor somewhere, and a list still covering
 * what you just asked for is a list you have to dismiss twice.
 */
export function carryOut(choice: PaletteChoice, job: string | null, hands: PaletteHands): void {
  switch (choice.of) {
    case "job":
      hands.openJob(choice.id);
      return;
    case "surface":
      hands.surface(choice.id);
      return;
    case "run":
      hands.run(choice.id);
      return;
    case "filter":
      hands.filter(choice.id);
      return;
    case "setting":
      hands.openSetting(choice.id);
      return;
    case "act":
      act(choice.id, job, hands);
  }
}

function act(id: string, job: string | null, hands: PaletteHands): void {
  switch (id) {
    // The four signposts, which open the Job rather than performing the verb.
    case "open":
    case "review":
    case "attest":
    case "redirect":
      if (job !== null) hands.openJob(job);
      return;
    case "new_job":
      hands.compose();
      return;
    case "search":
      hands.search();
      return;
    case "close":
      hands.closeJob();
      return;
    case "copy_debug_info":
      hands.copyDebugInfo();
      return;
    case "restart_step":
    case "redispatch":
      // Neither is destructive by the registry's reading — a restart puts a
      // fresh Drone on the same worktree and a redispatch mints a new Job —
      // and both still confirm, because `App` has always confirmed them and
      // the confirmation is where what survives is stated.
      if (job !== null) {
        hands.confirm(id === "restart_step" ? "restart_step" : "redispatch", job);
      }
      return;
    default:
      // An act the registry carries and this build does not answer. `dormantIn`
      // drew it dimmed and the palette refuses a dormant row before it reaches
      // here, so this arm holds only while the two disagree — which is what an
      // act added to one and not the other looks like.
      return;
  }
}
