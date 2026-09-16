// The left column's resting width, remembered across a restart — the same
// mechanism `dock-width.ts` uses for Helm's dock, one `localStorage` key per
// resizable region. One key here too: Navigation, Stats and Fleet resize as
// one column, never three.

import { useState } from "react";
import { clampLeftWidth, defaultLeftWidth } from "@armada/components";

const KEY = "armada.bridge.left-width";

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
 * The left column's width in px, backed by `localStorage`. Starts from what
 * was remembered, clamped to today's `--sidebar-min`/`--sidebar-max` in case
 * either moved since it was saved; falls back to `--sidebar-default` when
 * nothing was saved yet.
 */
export function useLeftWidth(): [number, (width: number) => void] {
  const [width, setWidth] = useState(() => clampLeftWidth(read() ?? defaultLeftWidth()));

  function press(next: number): void {
    const clamped = clampLeftWidth(next);
    setWidth(clamped);
    write(clamped);
  }

  return [width, press];
}
