// The guide catalogue list's resting width, remembered across a restart — the
// mechanism `left-width.ts` and `dock-width.ts` already use for the shell's own
// two resizable regions, one `localStorage` key per region. One key here too:
// there is one list, never a roster of them to key by name.
//
// **`localStorage`, not a Fleet preference**, for `panel-open.ts`' reason:
// Fleet's preference set is closed and a new key is a wire change in five
// crates. A column's width is this window's layout, not a fact about anything
// Fleet holds.

import { useState } from "react";
import { clampGuideListWidth, defaultGuideListWidth } from "@armada/components";

const KEY = "armada.bridge.guide-list-width";

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
 * The list's width in px, backed by `localStorage`. Starts from what was
 * remembered, clamped to today's `--w-guide-list-min` and `--w-guide-list-max`
 * in case either moved since it was saved; falls back to `--w-guide-list` when
 * nothing was saved yet.
 */
export function useGuideListWidth(): [number, (width: number) => void] {
  const [width, setWidth] = useState(() => clampGuideListWidth(read() ?? defaultGuideListWidth()));

  function press(next: number): void {
    const clamped = clampGuideListWidth(next);
    setWidth(clamped);
    write(clamped);
  }

  return [width, press];
}
