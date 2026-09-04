// Job detail's half of the keyboard map, read off `actions.toml`.
//
// The Board's half is `keys.ts` and the two are deliberately separate: they
// answer different keys on different objects, and one handler branching on
// which surface is up is how a key ends up doing two things. What they share is
// the safety rules, and those are imported rather than restated —
// `holdsText` is the whole of "single-key shortcuts are suppressed while a text
// input holds focus", and typing "fig" into a redirect box must not open a
// diff, open a gate and open a stage.
//
// # The contextual tier, and what detail does with each key
//
// | Key | `actions.toml` | Here |
// |---|---|---|
// | `j` `k` `↓` `↑` | `move_focus`, scope `list and detail` | move between the steps of the run, or between the rows of the log, depending on which the cursor is in |
// | `h` `l` `←` `→` | `disclose`, scope `detail` | open and close what the focused row holds — a step's facts, a log entry's payload |
// | `[` `]` | `focus_chapter`, scope `detail` | move between the three chapters |
// | `f` | `open_diff`, scope `detail` | open the Produced chapter to the diff |
// | `g` | `open_stage`, scope `detail` | open a stage of the phase strip |
// | `b` | `report_job`, scope `detail` | open the dialog that says this job failed in error |
// | `Esc` | `back`, scope `detail` | the list, and `App.tsx` owns it |
//
// # It names what it opens, and holds what it opened
//
// This file used to find its controls by the class names the components ship —
// `.armada-srow__name`, `button.armada-entry`, `.armada-story__chapter`,
// `button.armada-phases__control` — and press them. It worked, and it was a
// component's internals leaking into the app: a class rename broke the keyboard
// with nothing to catch it, because typecheck cannot see a string selector.
// #271.
//
// **The state moved here instead.** `RunTree`, `StepStory` and `PhaseStrip` all
// take controlled open state, so this file holds which steps have their facts
// open, which chapter is open, which stage is pinned and which log row is
// showing its payload, and `JobDetail` passes all four down. Opening a thing is
// naming it now, and a name is a value the compiler reads.
//
// The one rule that comes back the other way: **`Escape` on a controlled strip
// reports rather than unpins.** The strip calls `onPin(null)` and this holds
// the answer — which is why `pinnedStage` is cleared here and nowhere else.
//
// # Focus is still the cursor, because nothing draws another one
//
// `j`/`k` move between rows and a row is a control; there is no prop for "the
// row the keyboard is on", and a cursor held in state that nothing renders is a
// cursor nobody can see. So the two lists the cursor roves are still found in
// the document — but by two names **this app writes itself**: `data-armada-step`
// on a step's label, from `namesStep` below, and the payload id `Log` already
// gives every row. A rename of either is a rename in this package, and it is
// one the compiler follows.
//
// Hover on the strip is not among them and never will be: it reports where the
// pointer is, not what a reader decided.

import { useCallback, useEffect, useRef, useState } from "react";
import type { Dispatch, SetStateAction } from "react";
import type { PhaseStage, RunTreeStep, StepChapter } from "@armada/components";

import { holdsText } from "./keys";
import { LOG_REGION, rowOfPayload } from "./Log";

/** Which chapter the diff is in. The id, so the story can be reordered. */
export const DIFF_CHAPTER = "produced";

/** Which chapter the activity log is in. */
export const LOG_CHAPTER = "log";

/**
 * The log of what Fleet did to the Job itself, which is not in a chapter.
 *
 * **A region name and not a chapter id.** The chapters are a step's story and
 * this one belongs to no step, so it is named here beside them for the same
 * reason they are named at all: a log's region is how the keyboard finds its
 * rows, and a literal at the call site is what broke `h`/`l` the last time.
 */
export const FLEET_LOG = "fleet";

/** The attribute that names the step a label belongs to. Written by `namesStep`. */
const STEP = "data-armada-step";

/**
 * The attribute that names the chapter a control belongs to. Written by
 * `namesChapter`, read when a sheet closes and when `[` `]` land.
 *
 * **This app's own name, like `STEP` above.** The chapter's control is a
 * `Button` in `packages/components`; reaching for the class it ships is what
 * #271 took out of this file.
 */
const CHAPTER = "data-armada-chapter";

/**
 * What the app writes on a step's name so the keyboard can find the control it
 * sits in. **Spread into the label, never onto a component's own element** —
 * the marker box is `display: contents`, so the row draws exactly as it did.
 */
export function namesStep(stepId: string): Record<string, string> {
  return { [STEP]: stepId };
}

/**
 * What the app writes on a chapter's own control so the keyboard can find it.
 *
 * **`[` `]` land focus on it and `Enter` presses it**, which is exactly the
 * reading `actions.toml` gives `open_log`: *`Enter` already belongs to whatever
 * holds focus, which here is the chapter `[` `]` landed on*. So the act has no
 * key handler of its own — a second one would be two answers to one press, and
 * `Enter` on every other focused control would have to be told apart from it.
 */
export function namesChapter(chapterId: string): Record<string, string> {
  return { [CHAPTER]: chapterId };
}

/** What a press on job detail means. `null` is a key this surface does not carry. */
export type DetailPress =
  /** `j` `k` `↓` `↑` — move the cursor within whichever list holds it. */
  | { act: "move"; by: 1 | -1 }
  /** `h` `l` `←` `→` — open or close what the focused row holds. */
  | { act: "disclose"; open: boolean }
  /** `[` `]` — move between the three chapters. */
  | { act: "chapter"; by: 1 | -1 }
  /** `f` — open the Job's patch, on the layer that can hold it. */
  | { act: "diff" }
  /** `g` — open a stage of the phase strip. */
  | { act: "stage" }
  /** `b` — say this job failed in error. The only act key this surface binds. */
  | { act: "report" };

/**
 * What a keypress means on job detail, or `null` for nothing.
 *
 * **Every suppression is here rather than at the call site**, so there is one
 * place the safety rules can be checked against the contract instead of five
 * places they can each be forgotten in. `Escape` is not among them: it belongs
 * to the surface that opened the Job, and two handlers answering one key is the
 * defect `answersEnter` exists to prevent one key over.
 */
export function detailPressOf(event: KeyboardEvent): DetailPress | null {
  // A modifier means a different tier is being addressed — the palette's `⌘K`,
  // the rail's `⌘1`–`⌘5`, and `⌘[` `⌘]` for back and forward, which is exactly
  // why the brackets here are unmodified.
  if (event.metaKey || event.ctrlKey || event.altKey) return null;
  if (holdsText(event.target)) return null;
  // A held key repeats. Only movement accepts one — a repeat that opened
  // fourteen diffs is a repeat nobody asked for.
  const moves =
    event.key === "j" || event.key === "k" || event.key === "ArrowDown" || event.key === "ArrowUp";
  if (event.repeat && !moves) return null;

  switch (event.key) {
    case "j":
    case "ArrowDown":
      return { act: "move", by: 1 };
    case "k":
    case "ArrowUp":
      return { act: "move", by: -1 };
    case "l":
    case "ArrowRight":
      return { act: "disclose", open: true };
    case "h":
    case "ArrowLeft":
      return { act: "disclose", open: false };
    case "[":
      return { act: "chapter", by: -1 };
    case "]":
      return { act: "chapter", by: 1 };
    case "f":
      return { act: "diff" };
    case "g":
      return { act: "stage" };
    case "b":
      return { act: "report" };
    default:
      return null;
  }
}

/**
 * What the keyboard can name on the screen, as the surface built it.
 *
 * **Data, not selectors.** Every act below reaches its target through one of
 * these lists, so a chapter that cannot open or a phase that draws no card is
 * something the compiler knows about rather than something a query happens to
 * miss.
 */
export type DetailShape = {
  /** The run, in order. `factsOpen` seeds the open set exactly once. */
  run: readonly RunTreeStep[];
  /**
   * The step's story, in order. A chapter with no `content` cannot open.
   *
   * **Read when a key asks, rather than taken.** The story is built around the
   * activity log, and which of the log's rows is open is one of the four things
   * this file holds — so the chapters are assembled after it and this is how
   * they get back in. Nothing calls it during a render.
   */
  chapters: () => readonly StepChapter[];
  /** The strip, in order, or nothing on a step that has none. */
  stages: readonly PhaseStage[] | undefined;
  /**
   * Open the trailing sheet `f` names — the Job's patch.
   *
   * **The one act here that opens a layer rather than a chapter.** The patch
   * stopped being something the panel draws, so the key that opened a chapter
   * in place opens the sheet instead, and the screen passes in how rather than
   * this file learning what a sheet is.
   *
   * Absent leaves the press unswallowed, as `onReport` does.
   */
  onOpenSheet?: () => void;
  /**
   * Open the report dialog. **The one entry here that moves nothing on the
   * screen** — every other act opens something this file already holds, and
   * this one raises a dialog the screen owns, so the screen passes in what to
   * call rather than this file learning what a report is.
   *
   * Absent where the state does not offer it, and the press is then left
   * unswallowed rather than answered with nothing.
   */
  onReport?: () => void;
};

/** The open state this file holds, in the shape the screen takes it in. */
export type DetailKeys = {
  /** `InsideAJob.openSteps` — which steps have their facts open. */
  openSteps: readonly string[];
  onOpenStep: (stepId: string, open: boolean) => void;
  /** `StepPanel.openChapterId` — the one chapter that is open, or none. */
  openChapterId: string | null;
  onOpenChapter: (chapterId: string | null) => void;
  /** `PhaseStrip.pinnedStage` — the stage whose card is held open. */
  pinnedStage: string | null;
  onPinStage: (stageId: string | null) => void;
  /**
   * Put focus on a chapter's own control, by name. **What a closing sheet
   * calls**: the chapter line is the way back, so `[` `]` carry on from the
   * chapter the reader opened rather than from the top of the story.
   */
  onFocusChapter: (chapterId: string) => void;
  /**
   * What one log takes: its own name, the row of *its* rows that is open, and
   * how to say one was pressed. **Per log, not per story** — chapter one's
   * turns and chapter two's preview draw the same rows under the same ids, and
   * one held row would otherwise open in both at once.
   */
  inLog: (region: string) => {
    region: string;
    openId: string | null;
    onOpen: (rowId: string | null) => void;
  };
};

/** A row of a log, which is a row and the log it is in. */
type OpenEntry = { region: string; row: string };

/**
 * Bind the detail's contextual tier, and hold what it opened.
 *
 * The listener is bound once, for the life of the open Job, and reads the
 * screen through a ref: the run and the story are rebuilt on every tick of the
 * clock, and a listener re-bound sixty times a minute is a listener that is
 * sometimes not bound at all.
 */
export function useDetailKeys(shape: DetailShape): DetailKeys {
  const [openSteps, setOpenSteps] = useState<readonly string[] | null>(null);
  const [openChapter, setOpenChapter] = useState<string | null>(null);
  const [pinnedStage, setPinnedStage] = useState<string | null>(null);
  const [openEntry, setOpenEntry] = useState<OpenEntry | null>(null);

  // Seeded from the run the first time it has rows — which is the moment an
  // uncontrolled tree would have mounted and seeded itself from `factsOpen`,
  // so the step the panel opens on still arrives with its facts open. Set
  // during that render rather than from an effect: an effect draws one frame
  // with them shut and then opens them, which is a flicker on every Job.
  if (openSteps === null && shape.run.length > 0) {
    setOpenSteps(shape.run.filter((step) => step.factsOpen === true).map((step) => step.id));
  }

  const held = useRef(shape);
  useEffect(() => {
    held.current = shape;
  });

  const moves = useRef<Moves>({
    steps: setOpenSteps,
    chapter: setOpenChapter,
    pinned: setPinnedStage,
    entry: setOpenEntry,
  });

  useEffect(() => {
    function pressed(event: KeyboardEvent): void {
      const press = detailPressOf(event);
      if (press === null) return;
      if (act(press, held.current, moves.current)) event.preventDefault();
    }
    window.addEventListener("keydown", pressed);
    return () => window.removeEventListener("keydown", pressed);
  }, []);

  const onOpenStep = useCallback((stepId: string, open: boolean) => {
    setOpenSteps((was) => withStep(was, stepId, open));
  }, []);

  // The control is back in the document only after the panel re-renders, so the
  // focus move waits a frame rather than running inside the press that closed
  // the sheet — the same arrangement the Board's return-to-the-row uses.
  const onFocusChapter = useCallback((chapterId: string) => {
    requestAnimationFrame(() => chapterControl(chapterId)?.focus());
  }, []);

  return {
    openSteps: openSteps ?? NONE,
    onOpenStep,
    openChapterId: openChapter,
    onOpenChapter: setOpenChapter,
    pinnedStage,
    onPinStage: setPinnedStage,
    onFocusChapter,
    inLog: (region) => ({
      region,
      openId: openEntry?.region === region ? openEntry.row : null,
      onOpen: (row) => setOpenEntry(row === null ? null : { region, row }),
    }),
  };
}

/** An open set before the run has arrived. One value, so it is one identity. */
const NONE: readonly string[] = [];

/** The four things a press can move. Passed as one so `act` stays a function. */
type Moves = {
  steps: Dispatch<SetStateAction<readonly string[] | null>>;
  chapter: Dispatch<SetStateAction<string | null>>;
  pinned: Dispatch<SetStateAction<string | null>>;
  entry: Dispatch<SetStateAction<OpenEntry | null>>;
};

/**
 * Carry out a press.
 *
 * **`false` where nothing was there to act on**, so the caller knows not to
 * swallow the key — `f` on a Job with no diff should leave the browser's own
 * behaviour alone rather than silently eating the press.
 *
 * Every mover takes the updater form. A state updater runs twice under
 * StrictMode, so each one is a pure reading of what was open and nothing else
 * happens inside them.
 */
function act(press: DetailPress, shape: DetailShape, on: Moves): boolean {
  switch (press.act) {
    case "move":
      return move(press.by);
    case "disclose":
      return disclose(press.open, on);
    case "chapter":
      return chapter(press.by, shape, on);
    case "diff":
      return diff(shape);
    case "stage":
      return stage(shape, on);
    case "report":
      return report(shape);
  }
}

/**
 * `b`, from `actions.toml` — `report_job`, scope `detail`, which is this
 * surface and no other. It confirms, and the dialog it raises is the
 * confirmation: nothing is filed by the press.
 */
function report(shape: DetailShape): boolean {
  if (shape.onReport === undefined) return false;
  shape.onReport();
  return true;
}

/** A row the cursor can be on: what it is called, and the control it is. */
type Row = { id: string; control: HTMLElement };

/** A row of a log, which is one of those and the log it was drawn in. */
type Entry = Row & { region: string };

/**
 * The run's rows, in tree order. Found by the marker this app writes on each
 * step's name, and the control is whatever that marker sits inside — which is
 * the button the row selects with, whatever it is called.
 */
function stepRows(): Row[] {
  return [...document.querySelectorAll<HTMLElement>(`[${STEP}]`)].flatMap((marker) => {
    const control = marker.closest("button");
    const id = marker.getAttribute(STEP);
    return control === null || id === null ? [] : [{ id, control }];
  });
}

/**
 * The log's rows, in document order and across every log the story is drawing.
 * A row is named by the payload it points at, which `Log` writes and nothing
 * else reads.
 */
function entryRows(): Entry[] {
  return [...document.querySelectorAll<HTMLElement>(`[${LOG_REGION}] button`)].flatMap(
    (control) => {
      const id = rowOfPayload(control.getAttribute("aria-controls"));
      const region = control.closest(`[${LOG_REGION}]`)?.getAttribute(LOG_REGION);
      return id === null || region == null ? [] : [{ id, control, region }];
    },
  );
}

/** Whether a row is the one the cursor is on. */
function under(row: Row): boolean {
  const focused = document.activeElement;
  return row.control === focused || (focused !== null && row.control.contains(focused));
}

/**
 * Move the cursor inside whichever list holds it — the run, or the log.
 *
 * **One pair of keys, two lists, and which one is decided by where focus
 * already is.** `actions.toml` says so in as many words: moving between steps
 * and moving between rows are one act, and a second pair of keys for it would
 * be a binding per region. Focus outside both lands on the run, because the run
 * is the thing a person arrived at the screen to read.
 */
function move(by: 1 | -1): boolean {
  const inLog = document.activeElement?.closest(`[${LOG_REGION}]`) != null;
  const rows = inLog ? entryRows() : stepRows();
  if (rows.length === 0) return false;
  const at = rows.findIndex(under);
  // Wrapping is deliberate: these are short lists a person roves rather than
  // scrolls, and a cursor that stops dead at the end of six rows is a cursor
  // somebody presses `j` at twice.
  const next = at < 0 ? 0 : (at + by + rows.length) % rows.length;
  rows[next]?.control.focus();
  return true;
}

/**
 * Open or close what the row under the cursor holds — a log entry's payload, or
 * a step's facts.
 *
 * **Neither is pressed, both are named.** A payload already open stays open on
 * `l` and closes on `h`; pressing regardless would make the two keys one
 * toggle, which is what having two of them is for avoiding.
 */
function disclose(open: boolean, on: Moves): boolean {
  if (!(document.activeElement instanceof HTMLElement)) return false;

  const entry = entryRows().find(under);
  if (entry !== undefined) {
    const now = { region: entry.region, row: entry.id };
    on.entry((was) =>
      open ? now : was?.region === now.region && was.row === now.row ? null : was,
    );
    return true;
  }

  const step = stepRows().find(under);
  if (step === undefined) return false;
  on.steps((was) => withStep(was, step.id, open));
  return true;
}

/** The open set with one step added or taken out. */
function withStep(
  was: readonly string[] | null,
  stepId: string,
  open: boolean,
): readonly string[] {
  const now = was ?? NONE;
  if (!open) return now.filter((held) => held !== stepId);
  return now.includes(stepId) ? now : [...now, stepId];
}

/**
 * Move between the chapters, opening the one it lands on.
 *
 * **A chapter with nothing behind it is not a stop.** The story collapses every
 * other chapter to its header when one is open, so landing is opening — there
 * is no third state where a chapter is neither open nor collapsed, and a
 * chapter with no `content` cannot be either. Reading order, so `]` goes down
 * the screen, and it clamps at both ends rather than wrapping: the story is
 * three chapters in a fixed order and wrapping past the last one would take a
 * reader back to the top of a thing they are reading down.
 */
function chapter(by: 1 | -1, shape: DetailShape, on: Moves): boolean {
  const opens = openable(shape.chapters());
  if (opens.length === 0) return false;
  on.chapter((was) => {
    const at = was === null ? -1 : opens.indexOf(was);
    const next = at < 0 ? 0 : Math.min(Math.max(at + by, 0), opens.length - 1);
    const landed = opens[next] ?? was;
    // Focus follows the landing, so `Enter` reaches the chapter's own act —
    // which is the whole of `open_log`'s binding. A frame later, because the
    // control the chapter draws may only exist once the story has re-rendered.
    if (landed !== null) requestAnimationFrame(() => chapterControl(landed)?.focus());
    return landed;
  });
  return true;
}

/**
 * Open the Job's patch on the layer that can hold it.
 *
 * **It opens a sheet rather than a chapter**, because a patch in a 602px column
 * is a decision taken on a line that wrapped. The screen holds which sheet is
 * open and closing is `Esc`, so this is one direction only — pressing `f` twice
 * does not toggle, where pressing it twice used to close the chapter.
 */
function diff(shape: DetailShape): boolean {
  if (shape.onOpenSheet === undefined) return false;
  shape.onOpenSheet();
  return true;
}

/**
 * The chapters `[` `]` can land on, in the story's own order.
 *
 * **A chapter with an act counts, and it has no body.** The log and the patch
 * open as a layer, so neither carries `content` any more — reading only
 * `content` would have left the brackets stopping at chapter one and nothing
 * would have said so.
 */
function openable(chapters: readonly StepChapter[]): string[] {
  return chapters
    .filter((held) => held.content !== undefined || held.act !== undefined)
    .map((held) => held.id);
}

/** A chapter's own control, by the name this app writes on it. */
function chapterControl(chapterId: string): HTMLElement | null {
  const marker = document.querySelector<HTMLElement>(`[${CHAPTER}="${CSS.escape(chapterId)}"]`);
  return marker === null ? null : (marker.closest("button") ?? marker);
}

/**
 * Open a stage of the phase strip.
 *
 * **Pinned, not hovered.** The strip opens a card on hover and on focus and
 * holds one open on a click, and pinning is the only one of the three a caller
 * can hold — hover is the pointer's position rather than a decision. So `g`
 * pins, and repeated presses walk the strip: `g g g` reaches the Judge without
 * a second binding for "the next one". `Escape` comes back through `onPin` and
 * clears it.
 */
function stage(shape: DetailShape, on: Moves): boolean {
  const stages = (shape.stages ?? [])
    .filter((held) => held.opens ?? (held.kind ?? "phase") !== "phase")
    .map((held) => held.id);
  if (stages.length === 0) return false;
  on.pinned((was) => {
    const at = was === null ? -1 : stages.indexOf(was);
    return stages[(at + 1) % stages.length] ?? null;
  });
  return true;
}
