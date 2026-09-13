// Where things are' own open choice, held locally so a press moves it at
// once — Fleet's round trip (`#927`, `SavePreference`) is not on the
// critical path of a toggle, and an unreachable Fleet must not make the
// control do nothing.
//
// **`fold` is the whole decision, and takes no Electron, no DOM.** A press
// always wins; a publish from Fleet lands only where it disagrees with what
// this window already believes it told Fleet — which is what lets the echo
// of this window's own save pass through as a no-op rather than a flicker.
// Testable on its own for exactly that reason.

import { useReducer } from "react";
import type { SavePreference } from "@armada/protocol";

type State = { open: boolean; lastFromFleet: boolean };

type Event = { kind: "press"; open: boolean } | { kind: "fleet"; open: boolean };

export function fold(state: State, event: Event): State {
  if (event.kind === "press") return { ...state, open: event.open };
  // The echo of this window's own save, most of the time — and otherwise
  // another window's press, arriving the same way. Either is adopted.
  if (event.open === state.lastFromFleet) return state;
  return { open: event.open, lastFromFleet: event.open };
}

/**
 * Where things are' open choice. `fromFleet` is `BridgeState.preferences.
 * where_things_are_open`, read every render; `press` moves the choice at
 * once and saves it in the background.
 *
 * **A failed or refused save is silent.** Nothing here publishes an
 * `Outcome` — the choice simply is not remembered, which is the honest
 * answer for a preference, never a control that looks like it did nothing.
 */
export function useWhereOpen(fromFleet: boolean): [boolean, (open: boolean) => void] {
  const [state, dispatch] = useReducer(fold, { open: fromFleet, lastFromFleet: fromFleet });
  if (fromFleet !== state.lastFromFleet) dispatch({ kind: "fleet", open: fromFleet });

  function press(open: boolean): void {
    dispatch({ kind: "press", open });
    const save: SavePreference = { name: "where_things_are_open", value: open };
    void window.armada.savePreference(save);
  }

  return [state.open, press];
}
