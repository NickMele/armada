// The two trackpad patterns, as the wire between the renderer and main spells
// them. `HapticPattern` in `@armada/components` is the renderer's name for the
// same set; the renderer hands `tap` to its provider, so a pattern added there
// and not here fails typecheck.

export type Pattern = "alignment" | "level_change";

const PATTERNS: readonly string[] = ["alignment", "level_change"] satisfies Pattern[];

/** Whether what the renderer sent is one of the two. Main plays nothing else. */
export function isPattern(value: unknown): value is Pattern {
  return typeof value === "string" && PATTERNS.includes(value);
}
