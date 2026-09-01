// Whether the window is at `--window-floor`.
//
// **`--window-floor` is not a breakpoint token.** The theme carries a
// `--breakpoint-*` namespace with two entries in it, `wide` and `narrow`, and a
// component that wants either spells the variant and gets a real media query.
// The floor is not one of them: `packages/tokens/tokens.theme.css` lists it
// under "Read from CSS as var(--token). No namespace carries them", so there is
// no variant to spell — and a media feature value cannot be a custom property,
// which rules out writing the `@media` by hand.
//
// So the one place that can read the token is the one place that can run: this
// reads `--window-floor` off the document and hands it to `matchMedia`. The
// value still lives in `packages/tokens` and nothing here restates it.
//
// **It answers for the window, not for a component's box.** Every floor
// departure Journey 4 draws is about the window running out of width — the
// sheet going flush, the close going icon-only — and a component that took its
// own width would answer a different question. A component takes `floor` as a
// prop, so a story can draw the state at any size, which is what the drawing
// does with `data-floor`.

import { useEffect, useState } from "react";

/** The token that says how narrow the window is allowed to get. */
const FLOOR = "--window-floor";

/**
 * The floor as a media query, read off the document.
 *
 * `null` where the token is not there — a stylesheet that failed to load is a
 * different fault, and answering `false` would quietly claim the window is
 * wide. Nothing then reports a floor, which is the safe half: the sheet keeps
 * its radius and its labelled close.
 */
function query(): string | null {
  const value = getComputedStyle(document.documentElement).getPropertyValue(FLOOR).trim();
  return value === "" ? null : `(max-width: ${value})`;
}

export function useAtFloor(): boolean {
  const [at, setAt] = useState(false);

  useEffect(() => {
    const asked = query();
    if (asked === null) return;
    const media = window.matchMedia(asked);
    setAt(media.matches);
    const changed = (event: MediaQueryListEvent) => setAt(event.matches);
    media.addEventListener("change", changed);
    return () => media.removeEventListener("change", changed);
  }, []);

  return at;
}
