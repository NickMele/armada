// The Board's half of the keyboard map, read off the one artifact that owns it.
//
// **`docs/contracts/design-system.md`, "Keyboard and command palette", is the
// map.** It stopped being a pattern on 2026-08-31, when drawing the Job Board
// and the command palette together forced the two halves into one artifact —
// the palette displays a binding beside every entry, so an unreconciled map is
// a palette that cannot be drawn. Nothing here decides a binding; this file
// reads the contextual tier and says which of its keys the Board can answer.
//
// **This file was read off prose, and the prose now has a source.**
// `crates/core-model/domain/actions.toml` is the artifact the contract promised
// and `xtask`'s action rule is the test, so the two blocks under "Two tiers"
// are checked against it. This is still a hand transcription of that table, and
// so is `packages/components/src/actions.ts`, which is what the palette draws
// from. The codegen that would emit both is #232.
//
// # The contextual tier, and what the Board does with each key
//
// | Key | Contract | On the Board |
// |---|---|---|
// | `j` `k` | move focus | here |
// | `↓` `↑` | move focus | `Active jobs list` owns them, and has since it became a listbox |
// | `Enter` | open the focused job | here |
// | `o` | open — the same act, named so the palette can display it | here, and unconditional like `Enter` |
// | `r` | review | here, where the row's own control is Review |
// | `t` | attest | here, where it is Attest |
// | `d` | redirect | here, where it is Redirect |
// | `s` | restart step | detail only, per the contract. Absent |
// | `p` | pilot | **not built.** Bridge has no pilot act at all, so this binds to nothing rather than to something invented |
// | `c` | copy debug info | bound here, and reaches nothing on a Job row. `copyDebugInfoFor` takes a `Failure` and a healthy Job has none. **The palette's row does better**: it copies the failure on screen where there is one, and says there is none where there is not |
// | `x` | kill, confirms | here, through the confirmation every destructive act already takes |
// | `n` | new job | here — the one key that acts on nothing on screen |
// | `/` | search the current list | here |
// | `1`–`5` | state filter, Job Board only, in tab order | here |
//
// **The arrows are not in `boardPressOf` and that is deliberate.** `Active jobs
// list` is a listbox and already roves on `↓ ↑ Home End`; a second handler on
// `window` would move the cursor twice per press. `j` and `k` are here because
// nothing else claims them, and both paths converge on the same cursor — they
// move DOM focus, and the list follows focus.
//
// # Every verb key is a signpost, and that is not the same as `Enter`
//
// None of Review, Attest or Redirect happens on the Board. Approval is a second
// act from detail by rule; a redirect is offered on `stuck.recourse`, a
// `GET /jobs/:job_id` field a list row has never read; and nothing serves an
// attestation at all. So the verb keys open the job, and what the verb carries
// is *why you are being sent there*.
//
// `Enter` and `o` open whatever is under the cursor. `r`, `t` and `d` open it
// **only where that is the verb the row offers**, which is what makes a
// rehearsed keystroke safe when the list reorders under you: `r` on a row that
// is not waiting on a review does nothing, where `Enter` would have opened
// something you were not looking at.
//
// # The safety rules are the contract's and are not negotiable here
//
// - **Single-key shortcuts are suppressed while a text input holds focus.**
//   `holdsText` below is the whole of it, and every read goes through it.
//   Typing "axe" into the filter box must not approve, kill and open something.
// - **A destructive key is never adjacent to `j`/`k`.** `x` is the only
//   destructive key in the map and it is four rows away from both.
// - **Every destructive act confirms, with Cancel holding initial focus.** `x`
//   returns a press and never a kill; what it reaches is the same `Dialog` the
//   detail's own kill reaches, which owns that rule.
// - **A modifier means this map is not the one being addressed.** `⌘K` is the
//   palette's and `⌘1`–`⌘5` are the rail's; a bare key is this tier's.

import type { JobSummary } from "@armada/protocol";
import { BOARD_TABS, type BoardTab } from "./board";

/** The verb a row's one control names. Four, and a row carries exactly one. */
export type RowVerb = "review" | "attest" | "redirect" | "open";

/**
 * The key for each verb, and its label.
 *
 * **The label is the button's word and the key is its initial**, which is the
 * whole mnemonic — so they live in one row and a rename cannot leave the key
 * standing for a word nobody sees. Redirect is the exception: `r` is review,
 * settled 2026-08-31, because review is on every needs-you row and is the
 * most-pressed contextual key in the app.
 */
export const ROW_VERBS: Readonly<Record<RowVerb, { key: string; label: string }>> = {
  review: { key: "r", label: "Review" },
  attest: { key: "t", label: "Attest" },
  redirect: { key: "d", label: "Redirect" },
  open: { key: "o", label: "Open" },
};

/**
 * Which verb a row carries, from the summary and nothing else.
 *
 * **Everything here is on `JobSummary`.** A list row holds the summary, so a
 * verb decided from a `JobDetail` field would be a request per row — the first
 * failure `docs/practices/bridge.md` names. That is why Redirect keys off
 * `assigned_drone`, which is presence on the row, rather than off
 * `stuck.recourse`, which is the answer to a second read and is what the detail
 * header uses.
 *
 * The consequence, said out loud: a row can offer Redirect where the detail
 * then does not, because Fleet's recourse is the narrower reading. The detail
 * is right and the row is a signpost — the same relationship Review has to the
 * approval it does not perform.
 */
export function verbOf(job: JobSummary, terminal: boolean): RowVerb {
  if (job.status === "awaiting_approval" || job.status === "awaiting_review") return "review";
  if (job.status === "awaiting_attestation") return "attest";
  if (!terminal && job.assigned_drone !== undefined) return "redirect";
  return "open";
}

/** What a press on the Board means. `null` is a key this surface does not carry. */
export type BoardPress =
  /** `/` — put the cursor in the search field. */
  | { act: "search" }
  /** `j` / `k` — move the cursor. The arrows are the listbox's. */
  | { act: "move"; by: 1 | -1 }
  /** `Enter` and `o` — open whatever is under the cursor. */
  | { act: "open" }
  /** `r` `t` `d` — open the row under the cursor if it carries this verb. */
  | { act: "verb"; verb: Exclude<RowVerb, "open"> }
  /** `x` — kill the job under the cursor. Confirms; this is only the press. */
  | { act: "kill" }
  /**
   * `c` — copy debug info.
   *
   * Returned so the key is claimed where the map says it is. It reaches nothing
   * on a Job row: `copyDebugInfoFor` takes a `Failure`, a healthy Job has none,
   * and the payload opens `armada error` with a required `message` — so a
   * job-identity payload under the same key would be a different artifact.
   */
  | { act: "copy" }
  /** `1`–`5` — set the state filter. */
  | { act: "tab"; tab: BoardTab }
  /** `n` — the one key that acts on nothing on screen. */
  | { act: "compose" };

/** Built once, from the tab list, so a tab's key is its position and stays so. */
const BY_TAB_KEY = new Map(BOARD_TABS.map((tab) => [tab.shortcut, tab.id]));

/**
 * The three conditional verb keys, built from the verb table for the reason the
 * tabs are built from theirs. `o` is not among them: the contract makes it the
 * same act as `Enter` rather than a fourth conditional one.
 */
const BY_VERB_KEY = new Map<string, Exclude<RowVerb, "open">>(
  (["review", "attest", "redirect"] as const).map((verb) => [ROW_VERBS[verb].key, verb]),
);

/**
 * Whether a text input holds focus, and so whether every single-key shortcut is
 * suppressed.
 *
 * `contentEditable` is in here with the three element types because a rich
 * field is a text input to the person typing in it, whatever the DOM calls it.
 * A `select` counts too: its own type-ahead is how a person picks an option by
 * name, and a `d` that redirected a job instead of jumping to "Done" would be
 * the same defect one control over.
 */
export function holdsText(target: EventTarget | null): boolean {
  if (!(target instanceof HTMLElement)) return false;
  if (target.isContentEditable) return true;
  const tag = target.tagName;
  return tag === "INPUT" || tag === "TEXTAREA" || tag === "SELECT";
}

/**
 * Whether the focused element answers `Enter` itself.
 *
 * **`Enter` belongs to whatever has focus, and never to two things at once.** A
 * row is a control that opens on `Enter`, a button fires on it, and this map
 * also carries it — so without this, pressing `Enter` on a row's Kill button
 * fires the kill and opens the job, and pressing it on the row opens the job
 * twice. The map's `Enter` is for when focus is on none of them, which is where
 * a person lands after clicking empty space or reloading the window. `o` is the
 * same act under a key nothing else claims, so nothing is lost.
 */
function answersEnter(target: EventTarget | null): boolean {
  if (!(target instanceof HTMLElement)) return false;
  const tag = target.tagName;
  if (tag === "BUTTON" || tag === "A" || tag === "SUMMARY") return true;
  const role = target.getAttribute("role");
  return role === "option" || role === "button" || role === "tab";
}

/**
 * What a keypress means on the Board, or `null` for nothing.
 *
 * **Every suppression is here rather than at the call sites**, so there is one
 * place the safety rules can be checked against the contract instead of five
 * places they can each be forgotten in.
 */
/**
 * **Named apart from job detail's.** The two key maps answer different keys on
 * different objects and are deliberately separate files; one package exporting
 * two `pressOf` is the ambiguity that separation exists to avoid.
 */
export function boardPressOf(event: KeyboardEvent): BoardPress | null {
  // A modifier means a different tier is being addressed — the palette's `⌘K`,
  // the rail's `⌘1`–`⌘5`. This map is bare keys only.
  if (event.metaKey || event.ctrlKey || event.altKey) return null;
  if (holdsText(event.target)) return null;
  // A held key repeats. Only the two movement keys accept one: a repeat that
  // opened a detail forty times is a repeat nobody asked for, and `x` repeating
  // is the reason the contract worries about held keys at all.
  const movement = event.key === "j" || event.key === "k";
  if (event.repeat && !movement) return null;

  if (event.key === "/") return { act: "search" };
  if (event.key === "j") return { act: "move", by: 1 };
  if (event.key === "k") return { act: "move", by: -1 };
  if (event.key === "Enter") return answersEnter(event.target) ? null : { act: "open" };
  if (event.key === "o") return { act: "open" };
  if (event.key === "x") return { act: "kill" };
  if (event.key === "c") return { act: "copy" };
  if (event.key === "n") return { act: "compose" };

  const tab = BY_TAB_KEY.get(event.key);
  if (tab !== undefined) return { act: "tab", tab };

  const verb = BY_VERB_KEY.get(event.key);
  if (verb !== undefined) return { act: "verb", verb };

  return null;
}

/** The key that focuses the search, drawn beside the field. */
export const SEARCH_KEY = "/";

/**
 * What the command palette can reach on the Board.
 *
 * **Here because it is the same map from the other side.** `boardPressOf`
 * above says what a key means; this says what a palette row means, and the two
 * answer the same acts — `1`–`5` and `/`. Splitting them across two files
 * would be one act with two owners.
 *
 * **An imperative handle, and deliberately one.** The state filter and the
 * search field belong to the Board. Lifting either into `App` so a palette row
 * could set it would move a control out of the surface it is drawn on, and the
 * filter's own rule — that choosing a tab clears the search — would then live
 * in two places. The palette is a superset of the UI, not a second owner of
 * its state.
 */
export type BoardReach = {
  /** `1`–`5`, and the search clears with them, exactly as a tab press does. */
  tab: (tab: BoardTab) => void;
  /** `/` — put the cursor in the search field, selecting what is there. */
  search: () => void;
};
