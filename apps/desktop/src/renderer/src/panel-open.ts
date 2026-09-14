// Whether the left column's Stats and Fleet panels are open, remembered
// across a restart — Bridge/1088's own DoD line.
//
// **`localStorage`, not a Fleet preference.** `where_things_are_open`
// (`where-open.ts`) is Fleet-wide because more than one Bridge window can
// watch the same daemon; a panel fold is this window's own layout and Fleet's
// closed preference set (`fleet.unknown_preference`) has no room for a third
// name without a wire change outside this issue's scope. Reported.

import { useState } from "react";

const KEY = "armada.bridge.left-column-open";

type Stored = { stats: boolean; fleet: boolean };

const DEFAULT: Stored = { stats: true, fleet: true };

function read(): Stored {
  try {
    const raw = window.localStorage.getItem(KEY);
    if (raw === null) return DEFAULT;
    const parsed = JSON.parse(raw) as Partial<Stored>;
    return { stats: parsed.stats ?? true, fleet: parsed.fleet ?? true };
  } catch {
    return DEFAULT;
  }
}

function write(next: Stored): void {
  try {
    window.localStorage.setItem(KEY, JSON.stringify(next));
  } catch {
    // A failed write leaves the choice unremembered, the honest answer for a preference.
  }
}

/** One panel's open state, backed by the same stored pair. */
export function usePanelOpen(panel: keyof Stored): [boolean, (open: boolean) => void] {
  const [stored, setStored] = useState(read);

  function press(open: boolean): void {
    const next = { ...stored, [panel]: open };
    setStored(next);
    write(next);
  }

  return [stored[panel], press];
}
