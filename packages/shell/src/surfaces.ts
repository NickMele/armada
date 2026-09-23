// Where a person can go in Bridge, and the digit that reaches it.
//
// **One roster, read by both controls that navigate.** The rail draws it and
// the palette lists it. A second copy is how a place ends up in one and not the
// other, or answers a key the other spells differently — and a binding that
// disagrees with itself is worse than one that is missing.
//
// **The digit is computed from the rail order, never typed.** The contract
// binds `⌘1–⌘9` to Bridge surfaces *in rail order*, so a digit is a place in
// the rail and nothing else. A surface added at the end takes the next digit
// by arithmetic — Overview is the one exception, joining first instead (#921).
//
// **The order below is transcribed by hand, and it is the only transcription.**
// Nothing generated carries it: `actions.toml` holds the whole rail as one
// `bridge_surfaces` row spelling the range, deliberately, because its rule is
// the order rather than the numbers. So the order is written here once, from
// `docs/concepts/bridge.md`, and every digit falls out of it.

import { useEffect, useRef } from "react";
import {
  BookOpen,
  Briefcase,
  ClipboardList,
  FileCog,
  HardDrive,
  LayoutDashboard,
  Presentation,
  Settings as SettingsIcon,
} from "lucide-react";

import type { PaletteSurface } from "./Palette";

/**
 * Bridge's surfaces in rail order. A surface's place in here is its digit.
 *
 * **A surface joins at the end, and three have not.** Overview joined first
 * (#921), Studios third (#1287), and Kit before Settings rather than after it
 * (#1275) — each the owner's decision, each moving digits other than its own.
 * Kit is what he brings and Settings is what this machine is, so the machine
 * reads last.
 *
 * **Guides joined last and moved nothing** (#1602). It is the tenth row, which
 * is one past the digits the contract publishes, so it carries none — and
 * every digit above is the one it was. Where it belongs in the rail is the
 * thing here most likely to be pinned; moving it costs one line.
 *
 * Two of the rest draw no row yet and they keep their place anyway. A rail
 * that renumbered as surfaces were built would move a learned key every time
 * — which is the thing moving Helm to `⌘6`, and then off the rail entirely
 * for `⌘J`, was allowed to do.
 *
 * The whole digit history is `actions.toml`'s `bridge_surfaces` row and
 * `docs/contracts/design-system.md`, Two tiers. It is not repeated here.
 */
const RAIL = [
  "overview",
  "board",
  "studios",
  "alerts",
  "doctor",
  "manifest",
  "worktrees",
  "kit",
  "settings",
  "guides",
] as const;

type SurfaceId = (typeof RAIL)[number];

/** The ids Bridge routes on. Here, so a typo cannot be a dead row. */
export const SURFACE = {
  overview: "overview",
  board: "board",
  manifest: "manifest",
  worktrees: "worktrees",
  kit: "kit",
  settings: "settings",
  studios: "studios",
  guides: "guides",
} as const satisfies Record<string, SurfaceId>;

/**
 * What reaches a surface: its place in the rail, spelled as the contract does.
 *
 * **Nothing past the ninth, and Guides is the tenth.** The contract binds
 * `⌘1–⌘9` to Bridge surfaces in rail order, so a tenth digit does not exist to
 * hand out — and taking one off a surface that already publishes it would
 * change a binding a person has learned. The tenth row is reached by name in
 * the palette and by no key, which is the arrangement `Palette.tsx` already
 * describes for a destination with no position.
 */
function digitOf(id: SurfaceId): string | undefined {
  const at = RAIL.indexOf(id) + 1;
  return at > 9 ? undefined : `⌘${at}`;
}

/**
 * The places that exist, in rail order, with what finds each one.
 *
 * **What is not built is not in here.** A row a person presses and gets nothing
 * from is worse than one that is absent, which is the contract's own rule about
 * a registered binding nothing answers. So the digits skip: `⌘4` and `⌘5` are
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
    id: SURFACE.studios,
    label: "Studios",
    shortcut: digitOf(SURFACE.studios),
    // No alias, Manifest's reason: the first surface built at this name.
    icon: Presentation,
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
  {
    id: SURFACE.kit,
    label: "Kit",
    shortcut: digitOf(SURFACE.kit),
    // "MCP" and "servers" are what a person looking for this will type: Kit
    // is Armada's word for the set, and the thing they came to connect has
    // its own name in every other tool they use.
    aliases: ["mcp", "servers", "mcp servers"],
    icon: Briefcase,
  },
  {
    id: SURFACE.settings,
    label: "Settings",
    shortcut: digitOf(SURFACE.settings),
    // No alias: `fleet_settings` is the palette's own row, in its own
    // section, and it names an id rather than a word somebody already knows.
    icon: SettingsIcon,
  },
  {
    id: SURFACE.guides,
    label: "Guides",
    // Nothing: it is the tenth row and `⌘1–⌘9` is what the contract publishes.
    shortcut: digitOf(SURFACE.guides),
    // What a person looking for this will type. "Help" is the word every other
    // application uses for the place explanations live, and "guide" singular is
    // what somebody types who met one card and wants the rest.
    aliases: ["help", "guide", "explain"],
    icon: BookOpen,
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
 * **Only the surfaces that draw.** `SURFACES` is what is built, so `⌘4` and
 * `⌘5` — Alerts and Doctor — reach nothing and are not bound; the digit stays
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
