// Whether the window is at `--window-floor`, and whether it is under
// `--layout-breakpoint`. Two tokens, one reader.
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

/** The token the layout collapses at, one band above the floor. */
const BREAKPOINT = "--layout-breakpoint";

/**
 * A token read off the document as a `max-width` query.
 *
 * `null` where the token is not there — a stylesheet that failed to load is a
 * different fault, and answering `false` would quietly claim the window is
 * wide. Nothing then reports a floor, which is the safe half: the sheet keeps
 * its radius and its labelled close.
 */
function query(token: string): string | null {
  const value = getComputedStyle(document.documentElement).getPropertyValue(token).trim();
  return value === "" ? null : `(max-width: ${value})`;
}

/** Whether the window is at or under `token`, kept live as it resizes. */
function useUnder(token: string): boolean {
  const [under, setUnder] = useState(false);

  useEffect(() => {
    const asked = query(token);
    if (asked === null) return;
    const media = window.matchMedia(asked);
    setUnder(media.matches);
    const changed = (event: MediaQueryListEvent) => setUnder(event.matches);
    media.addEventListener("change", changed);
    return () => media.removeEventListener("change", changed);
  }, [token]);

  return under;
}

export function useAtFloor(): boolean {
  return useUnder(FLOOR);
}

/**
 * Whether the window is at or below `--layout-breakpoint`, where a screen with
 * two columns stops having room for both.
 *
 * **Read here rather than spelled as a media query**, for `useAtFloor`'s own
 * reason: a media feature value cannot be a custom property, and the breakpoint
 * lives in `packages/tokens`. The shell already answered this question for its
 * own rail; job detail asks it about its inspector, and one reader is what
 * keeps the two from drifting apart by a pixel.
 */
export function useNarrow(): boolean {
  return useUnder(BREAKPOINT);
}
