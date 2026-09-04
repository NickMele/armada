// Where a person can go in Bridge, and the digit that reaches it.
//
// **One roster, read by both controls that navigate.** The rail draws it and
// the palette lists it. A second copy is how a place ends up in one and not the
// other, or answers a key the other spells differently — and a binding that
// disagrees with itself is worse than one that is missing.
//
// **The digit is computed from the rail order, never typed.** The contract
// binds `⌘1–⌘5` to Bridge surfaces *in rail order*, so a digit is a place in
// the rail and nothing else. A surface added at the end takes the next digit by
// arithmetic rather than by somebody remembering to renumber.
//
// **The order below is transcribed by hand, and it is the only transcription.**
// Nothing generated carries it: `actions.toml` holds the whole rail as one
// `bridge_surfaces` row spelling the range, deliberately, because its rule is
// the order rather than the numbers. So the order is written here once, from
// `docs/concepts/bridge.md`, and every digit falls out of it.

import { ClipboardList, HardDrive } from "lucide-react";

import type { PaletteSurface } from "./Palette";

/**
 * Bridge's surfaces in rail order. A surface's place in here is its digit.
 *
 * Three of these draw no row yet and they keep their place anyway. A rail that
 * renumbered as surfaces were built would move a learned key every time — which
 * is the thing moving Helm to `⌘6` was allowed to do exactly once, with the
 * reason recorded in `actions.toml` beside the binding.
 */
const RAIL = ["board", "alerts", "doctor", "manifest", "worktrees"] as const;

type SurfaceId = (typeof RAIL)[number];

/** The ids Bridge routes on. Here, so a typo cannot be a dead row. */
export const SURFACE = {
  board: "board",
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
 * a registered binding nothing answers. So the digits skip: `⌘2`–`⌘4` are owed
 * to Alerts, Doctor and Manifest and reach nothing today.
 *
 * `held disk` is an alias because that is the word on the control this screen
 * has been reached by since it shipped, and a person who learned it should not
 * have to learn a second.
 */
export const SURFACES: readonly PaletteSurface[] = [
  {
    id: SURFACE.board,
    label: "Job Board",
    shortcut: digitOf(SURFACE.board),
    icon: ClipboardList,
  },
  {
    id: SURFACE.worktrees,
    label: "Held worktrees",
    shortcut: digitOf(SURFACE.worktrees),
    aliases: ["disk", "held disk"],
    icon: HardDrive,
  },
];
