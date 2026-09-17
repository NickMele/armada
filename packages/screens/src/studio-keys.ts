// The keys live on an open Studio — #1364. Read off `actions.toml` through the
// generated registry, never written down here.
//
// **A place rather than an object, which is what `open studio` scopes.** The
// Board's map acts on the row under the cursor and job detail's on the job
// open; these act on the board a person is looking at, so they are their own
// small map rather than a branch inside either.
//
// The safety rules are `keys.ts`'s and are imported: a single key is suppressed
// while a text input holds focus, so typing "seven notes" into a note does not
// open three more fields, and a modifier means another tier is being addressed.

import { useEffect } from "react";
import { ACTION, type StudioNodeByHandKind } from "@armada/components";

import { holdsText } from "./keys";

/** Each kind's binding, from the registry. The one place a key is written is there. */
const ADD: readonly (readonly [string, StudioNodeByHandKind])[] = [
  [ACTION.add_note?.shortcut ?? "", "note"],
  [ACTION.add_link?.shortcut ?? "", "link"],
  [ACTION.add_sketch?.shortcut ?? "", "sketch"],
];

/** What a press means on an open Studio, or `null` where it means nothing here. */
export function addPressOf(event: {
  key: string;
  metaKey: boolean;
  ctrlKey: boolean;
  altKey: boolean;
  target: EventTarget | null;
}): StudioNodeByHandKind | null {
  if (event.metaKey || event.ctrlKey || event.altKey) return null;
  if (holdsText(event.target)) return null;
  return ADD.find(([key]) => key === event.key)?.[1] ?? null;
}

/**
 * `N`, `V` and `S` on an open Studio, while it can be written to.
 *
 * **Bound only while `editable`**, so a Studio reopened read-only answers no
 * key — the same rule its controls draw, rather than a second one.
 */
export function useAddNodeKeys(editable: boolean, onAdding: (kind: StudioNodeByHandKind) => void): void {
  useEffect(() => {
    if (!editable) return;
    function pressed(event: KeyboardEvent): void {
      const kind = addPressOf(event);
      if (kind === null) return;
      event.preventDefault();
      onAdding(kind);
    }
    window.addEventListener("keydown", pressed);
    return () => window.removeEventListener("keydown", pressed);
  }, [editable, onAdding]);
}
