// Helm's dock resting width, remembered across a restart. `panel-open.ts`
// (Bridge/1088, `apps/desktop`) is the precedent — one `localStorage` key,
// this window's own layout rather than a Fleet preference — but that file
// cannot be imported here: `useDock` (below, in `Shell.tsx`) is what actually
// assembles the dock's props, and `@armada/shell` cannot depend on
// `apps/desktop`. Same mechanism, read and written locally instead.
//
// One key rather than `panel-open.ts`'s named bag: there is exactly one dock,
// never a roster of them to key by name.

import { useState } from "react";
import { clampDockWidth, defaultDockWidth } from "@armada/components";

const KEY = "armada.bridge.dock-width";

function read(): number | null {
  try {
    const raw = window.localStorage.getItem(KEY);
    if (raw === null) return null;
    const value = Number(raw);
    return Number.isFinite(value) ? value : null;
  } catch {
    return null;
  }
}

function write(width: number): void {
  try {
    window.localStorage.setItem(KEY, String(width));
  } catch {
    // A failed write leaves the width unremembered, the honest answer for a preference.
  }
}

/**
 * The dock's width in px, backed by `localStorage`. Starts from what was
 * remembered, clamped to today's `--w-dock-min`/`--w-dock-max` in case the
 * tokens moved since it was saved; falls back to `--w-dock` when nothing was
 * saved yet.
 */
export function useDockWidth(): [number, (width: number) => void] {
  const [width, setWidth] = useState(() => clampDockWidth(read() ?? defaultDockWidth()));

  function press(next: number): void {
    const clamped = clampDockWidth(next);
    setWidth(clamped);
    write(clamped);
  }

  return [width, press];
}
