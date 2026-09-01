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
// | `Esc` | `back`, scope `detail` | the list, and `App.tsx` owns it |
//
// # It moves focus, and focus is what acts
//
// **Nothing here holds a second copy of what is open.** Every one of these
// regions is built from controls that already open on `Enter`, on click and —
// for the phase strip — on focus, so the keyboard's job is to put the cursor on
// the right control and press it. A parallel record of which step is expanded
// would be a second answer to a question the DOM already answers, and the two
// would disagree the first time somebody used the pointer.
//
// The consequence, said out loud: these read the components' own class names to
// find their controls. That is a coupling, and the alternative is controlled
// open-state props on `RunTree`, `StepStory` and `PhaseStrip`, which is a
// components change rather than this app's. Reported.

import { holdsText } from "./keys";

/** Where the detail's regions are, by the class each component ships. */
const REGION = {
  root: ".armada-screen__detail",
  step: ".armada-srow__name",
  chevron: ".armada-srow__chevron",
  entry: "button.armada-entry",
  chapter: ".armada-story__chapter",
  stage: "button.armada-phases__control",
} as const;

/** What a press on job detail means. `null` is a key this surface does not carry. */
export type DetailPress =
  /** `j` `k` `↓` `↑` — move the cursor within whichever list holds it. */
  | { act: "move"; by: 1 | -1 }
  /** `h` `l` `←` `→` — open or close what the focused row holds. */
  | { act: "disclose"; open: boolean }
  /** `[` `]` — move between the three chapters. */
  | { act: "chapter"; by: 1 | -1 }
  /** `f` — open the Produced chapter to the diff. */
  | { act: "diff" }
  /** `g` — open a stage of the phase strip. */
  | { act: "stage" };

/**
 * What a keypress means on job detail, or `null` for nothing.
 *
 * **Every suppression is here rather than at the call site**, so there is one
 * place the safety rules can be checked against the contract instead of five
 * places they can each be forgotten in. `Escape` is not among them: it belongs
 * to the surface that opened the Job, and two handlers answering one key is the
 * defect `answersEnter` exists to prevent one key over.
 */
export function pressOf(event: KeyboardEvent): DetailPress | null {
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
    default:
      return null;
  }
}

/**
 * Carry out a press.
 *
 * **`false` where nothing was there to act on**, so the caller knows not to
 * swallow the key — `f` on a Job with no diff should leave the browser's own
 * behaviour alone rather than silently eating the press.
 */
export function act(press: DetailPress): boolean {
  const root = document.querySelector<HTMLElement>(REGION.root);
  if (root === null) return false;
  switch (press.act) {
    case "move":
      return move(root, press.by);
    case "disclose":
      return disclose(root, press.open);
    case "chapter":
      return chapter(root, press.by);
    case "diff":
      return open(root, DIFF_CHAPTER);
    case "stage":
      return stage(root);
  }
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
function move(root: HTMLElement, by: 1 | -1): boolean {
  const inLog = document.activeElement?.closest(REGION.entry) != null;
  const rows = [...root.querySelectorAll<HTMLElement>(inLog ? REGION.entry : REGION.step)];
  if (rows.length === 0) return false;
  const at = rows.findIndex((row) => row === document.activeElement || row.contains(document.activeElement));
  // Wrapping is deliberate: these are short lists a person roves rather than
  // scrolls, and a cursor that stops dead at the end of six rows is a cursor
  // somebody presses `j` at twice.
  const next = at < 0 ? 0 : (at + by + rows.length) % rows.length;
  rows[next]?.focus();
  return true;
}

/**
 * Open or close what the focused row holds. A step's facts are behind its own
 * chevron; a log entry is its own control, so the row is what is pressed.
 */
function disclose(root: HTMLElement, open: boolean): boolean {
  const focused = document.activeElement;
  if (!(focused instanceof HTMLElement)) return false;

  const entry = focused.closest<HTMLElement>(REGION.entry);
  if (entry !== null) {
    // A payload already open stays open on `l`, and closes on `h`. Pressing
    // regardless would make the two keys one toggle, which is what having two
    // of them is for avoiding.
    if (entry.getAttribute("aria-expanded") === String(open)) return true;
    entry.click();
    return true;
  }

  const row = focused.closest<HTMLElement>(".armada-srow");
  const chevron = row?.querySelector<HTMLElement>(REGION.chevron);
  if (chevron === null || chevron === undefined || chevron.tagName !== "BUTTON") return false;
  if (chevron.getAttribute("aria-expanded") === String(open)) return true;
  chevron.click();
  return true;
}

/**
 * Move between the three chapters. **A Motion**: it lands somewhere and acts on
 * nothing, which is what `j`/`k` cannot do here — those rove inside whichever
 * chapter has focus. Reading order, so `]` goes down the screen.
 */
function chapter(root: HTMLElement, by: 1 | -1): boolean {
  const chapters = [...root.querySelectorAll<HTMLElement>(REGION.chapter)];
  if (chapters.length === 0) return false;
  const focused = document.activeElement;
  const at = chapters.findIndex((held) => focused instanceof Node && held.contains(focused));
  const next = at < 0 ? 0 : Math.min(Math.max(at + by, 0), chapters.length - 1);
  const landing = chapters[next]?.querySelector<HTMLElement>("button");
  if (landing === null || landing === undefined) return false;
  landing.focus();
  landing.scrollIntoView({ block: "nearest" });
  return true;
}

/** Which chapter the diff is in. Third, always — the order never changes. */
const DIFF_CHAPTER = 2;

/**
 * Open one chapter to its own content, by position. The order of the story is
 * fixed, which is what makes a position a stable thing to bind a key to.
 */
function open(root: HTMLElement, at: number): boolean {
  const held = [...root.querySelectorAll<HTMLElement>(REGION.chapter)][at];
  const control = held?.querySelector<HTMLElement>("button");
  if (control === null || control === undefined) return false;
  control.focus();
  // Already open closes, which is what pressing the control does and what a
  // reader pressing the same key twice expects.
  control.click();
  return true;
}

/**
 * Open a stage of the phase strip. **Focus is the whole of it** — a stage opens
 * its card on hover and on focus, so landing the cursor on one is the act, and
 * clicking would pin it before anybody had read it.
 */
function stage(root: HTMLElement): boolean {
  const stages = [...root.querySelectorAll<HTMLElement>(REGION.stage)];
  if (stages.length === 0) return false;
  const focused = document.activeElement;
  const at = stages.findIndex((held) => held === focused);
  // Repeated presses walk the strip, so `g g g` reaches the Judge without a
  // second binding for "the next one".
  stages[(at + 1) % stages.length]?.focus();
  return true;
}
