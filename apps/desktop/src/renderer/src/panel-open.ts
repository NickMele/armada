// Whether a panel is open, remembered across a restart — Bridge/1088's own DoD line. Overview 27
// (#1091) widened it past the left column's Stats and Fleet to Overview's own Needs you, Running,
// Queued and Other, so the bag is keyed by name rather than a fixed pair.
//
// **`localStorage`, not a Fleet preference.** `where_things_are_open` (`where-open.ts`) is
// Fleet-wide because more than one Bridge window can watch the same daemon; a panel fold is this
// window's own layout and Fleet's closed preference set (`fleet.unknown_preference`) has no room
// for a name without a wire change outside either issue's scope. Reported.

import { useState } from "react";

const KEY = "armada.bridge.panels-open";

type Stored = Record<string, boolean>;

function read(): Stored {
  try {
    const raw = window.localStorage.getItem(KEY);
    if (raw === null) return {};
    const parsed: unknown = JSON.parse(raw);
    return typeof parsed === "object" && parsed !== null ? (parsed as Stored) : {};
  } catch {
    return {};
  }
}

function write(next: Stored): void {
  try {
    window.localStorage.setItem(KEY, JSON.stringify(next));
  } catch {
    // A failed write leaves the choice unremembered, the honest answer for a preference.
  }
}

/** One panel's open state, backed by the same stored bag. Absent for a name reads open. */
export function usePanelOpen(panel: string): [boolean, (open: boolean) => void] {
  const [stored, setStored] = useState(read);

  function press(open: boolean): void {
    const next = { ...stored, [panel]: open };
    setStored(next);
    write(next);
  }

  return [stored[panel] ?? true, press];
}
