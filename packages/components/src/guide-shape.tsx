// Which shape a guide is read in. **A prototype, and the only thing that sets
// it is the mock.**
//
// The owner, 25 September 2026, on the Guides catalogue: *"I would prefer
// guides be animations or steps, not just a wall of text."* Three shapes of one
// guide are built so he can look at all three and choose; the two he does not
// choose get deleted, and this file with them.
//
// **The default is `prose`, and Bridge provides nothing.** `App.tsx` mounts no
// provider, so the window under `pnpm dev` reads the default and draws exactly
// what it drew before this file existed. The one caller of the provider is
// `apps/desktop/src/renderer/src/mock/mount.tsx`, which takes the shape from
// `?guides=` beside the `?scenario=` it already reads — the same place the
// mock's other dev-only affordances live. Nothing crosses the preload, nothing
// is stored, and there is no setting for it.

import { createContext, useContext, type ReactNode } from "react";

/**
 * The three readings being compared.
 *
 * - `prose` — short paragraphs. Today, and the baseline.
 * - `steps` — a numbered sequence, one line each, with a figure only where a
 *   relation is the thing being learned.
 * - `figure` — the drawing leads, at size, and the words are its captions.
 */
export type GuideShape = "prose" | "steps" | "figure";

/** Every shape, so the mock can validate what the URL asked for without a second list. */
export const GUIDE_SHAPES: readonly GuideShape[] = ["prose", "steps", "figure"];

/** Whether a string names a shape. The mock's `?guides=` is the only caller. */
export function isGuideShape(named: string): named is GuideShape {
  return (GUIDE_SHAPES as readonly string[]).includes(named);
}

// Prose, because that is what the app draws where nobody has asked for anything
// else — which is everywhere but the mock.
const GuideShapeContext = createContext<GuideShape>("prose");

/** The shape every guide on this screen is read in. `prose` where nothing provides one. */
export function useGuideShape(): GuideShape {
  return useContext(GuideShapeContext);
}

export type GuideShapeProviderProps = {
  shape: GuideShape;
  children: ReactNode;
};

/**
 * Sets the shape for everything under it.
 *
 * **Mounted by the mock and by nothing else.** A provider in `App` would make
 * this a feature of Bridge rather than a comparison, and there is nothing for a
 * person to switch: the decision is the owner's and it is made by looking.
 */
export function GuideShapeProvider({ shape, children }: GuideShapeProviderProps) {
  return <GuideShapeContext.Provider value={shape}>{children}</GuideShapeContext.Provider>;
}
