// Where a person can go in Bridge, and the digit that reaches it.
//
// **One roster, read by both controls that navigate.** The rail draws it and
// the palette lists it. A second copy is how a place ends up in one and not the
// other, or answers a key the other spells differently — and a binding that
// disagrees with itself is worse than one that is missing.
//
// **The digit is computed from the rail order, never typed.** The contract
// binds `⌘1–⌘6` to Bridge surfaces *in rail order*, so a digit is a place in
// the rail and nothing else. A surface added at the end takes the next digit
// by arithmetic — Overview is the one exception, joining first instead (#921).
//
// **The order below is transcribed by hand, and it is the only transcription.**
// Nothing generated carries it: `actions.toml` holds the whole rail as one
// `bridge_surfaces` row spelling the range, deliberately, because its rule is
// the order rather than the numbers. So the order is written here once, from
// `docs/concepts/bridge.md`, and every digit falls out of it.

import { useEffect, useRef } from "react";
import { ClipboardList, FileCog, HardDrive, LayoutDashboard } from "lucide-react";

import type { PaletteSurface } from "./Palette";

/**
 * Bridge's surfaces in rail order. A surface's place in here is its digit.
 *
 * **Overview goes first, not last.** Every other surface here joined at the
 * end of the rail and took the next digit; Overview is where Bridge opens
 * (#921), so it took `⌘1` and every other digit moved down one instead —
 * the one deliberate exception to "a surface joins at the end", decided with
 * the owner on 13 Sep.
 *
 * Two of the rest draw no row yet and they keep their place anyway. A rail
 * that renumbered as surfaces were built would move a learned key every time
 * — which is the thing moving Helm to `⌘6`, and then off the rail entirely
 * for `⌘J`, was allowed to do, with the reason recorded in `actions.toml`
 * beside the binding.
 */
const RAIL = ["overview", "board", "alerts", "doctor", "manifest", "worktrees"] as const;

type SurfaceId = (typeof RAIL)[number];

/** The ids Bridge routes on. Here, so a typo cannot be a dead row. */
export const SURFACE = {
  overview: "overview",
  board: "board",
  manifest: "manifest",
  worktrees: "worktrees",
} as const satisfies Record<string, SurfaceId>;

/** What reaches a surface: its place in the rail, spelled as the contract does. */
function digitOf(id: SurfaceId): string {
  return `⌘${RAIL.indexOf(id) + 1}`;
}

/**
 * The places that exist, in rail order, with what finds each one.
 *
 * **What is not built is not in here.** A row a person presses and gets nothing
 * from is worse than one that is absent, which is the contract's own rule about
 * a registered binding nothing answers. So the digits skip: `⌘3` and `⌘4` are
 * owed to Alerts and Doctor and reach nothing today.
 *
 * `held disk` is an alias because that is the word on the control this screen
 * has been reached by since it shipped, and a person who learned it should not
 * have to learn a second.
 */
export const SURFACES: readonly PaletteSurface[] = [
  {
    id: SURFACE.overview,
    label: "Overview",
    shortcut: digitOf(SURFACE.overview),
    // No alias, for the same reason Manifest carries none: this is the first
    // surface built at this name, so there is no earlier word to keep.
    icon: LayoutDashboard,
  },
  {
    id: SURFACE.board,
    label: "Job Board",
    shortcut: digitOf(SURFACE.board),
    icon: ClipboardList,
  },
  {
    id: SURFACE.manifest,
    label: "Manifest",
    shortcut: digitOf(SURFACE.manifest),
    // No alias. The rule above is that one is for a place a person already
    // knows by another word, and nothing in Bridge has ever reached this
    // surface — there is no earlier word for it to keep.
    icon: FileCog,
  },
  {
    id: SURFACE.worktrees,
    label: "Cleanup",
    shortcut: digitOf(SURFACE.worktrees),
    aliases: ["held worktrees", "disk", "held disk"],
    icon: HardDrive,
  },
];

/**
 * `⌘1`…`⌘n`, bound to the rail in rail order.
 *
 * **The binding the contract already publishes, finally answered.** Every
 * digit above was drawn in the palette and on no key — a shortcut a person
 * reads and presses to no effect is the thing `dormant` exists to prevent, and
 * it had been true of `⌘1` since the rail shipped.
 *
 * **Only the surfaces that draw.** `SURFACES` is what is built, so `⌘3` and
 * `⌘4` — Alerts and Doctor — reach nothing and are not bound; the digit stays
 * theirs, because `digitOf` reads the rail and not this list.
 *
 * **A modified key, so a focused field does not suppress it.** `⌘K` is bound
 * the same way and for the same reason: the contextual tier is suppressed
 * while a text input holds focus precisely so that typing cannot act, and the
 * whole point of the modifier is that it still works from inside one.
 */
export function useSurfaceKeys(onSurface: (surfaceId: string) => void): void {
  // **Bound once, and the handler is read through a ref.** The app rebuilds
  // its `goTo` every render, and a listener re-registered on every render is
  // one that has to be right about removal every time — `useCommandPalette`
  // avoids the question by depending on nothing, and this does the same.
  const latest = useRef(onSurface);
  latest.current = onSurface;

  useEffect(() => {
    function pressed(event: KeyboardEvent): void {
      if (!(event.metaKey || event.ctrlKey) || event.altKey || event.shiftKey) return;
      const surface = SURFACES.find((one) => one.shortcut === `⌘${event.key}`);
      if (surface === undefined) return;
      event.preventDefault();
      latest.current(surface.id);
    }
    window.addEventListener("keydown", pressed);
    return () => window.removeEventListener("keydown", pressed);
  }, []);
}
