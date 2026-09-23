// Whether the person asked for the left column's rail, remembered across a
// restart — #1591. `localStorage` rather than a Fleet preference, for
// `left-width.ts`' own reason: one window's way of looking, not a fact about
// the fleet.
//
// **It holds the choice, never the drawn state.** Under `--layout-breakpoint`
// the column is at its rail whatever this says, and `Shell.tsx` puts the two
// together — storing what was drawn would leave a window that narrowed once
// coming back wide with the column shut and nothing to say why.

import { useState } from "react";

const KEY = "armada.bridge.left-collapsed";

function read(): boolean {
  try {
    return window.localStorage.getItem(KEY) === "true";
  } catch {
    return false;
  }
}

/** The remembered choice, and the press that moves it. Expanded where nothing is stored. */
export function useLeftCollapsed(): [boolean, (collapsed: boolean) => void] {
  const [collapsed, setCollapsed] = useState(read);

  function press(next: boolean): void {
    setCollapsed(next);
    try {
      window.localStorage.setItem(KEY, String(next));
    } catch {
      // A failed write leaves the choice unremembered, the honest answer for a preference.
    }
  }

  return [collapsed, press];
}
