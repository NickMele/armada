import { useEffect } from "react";

import { useGuidance } from "../../guidance";
import type { Guide } from "../../guides/guide";

/**
 * The `?` beside the thing it explains. Pressing it opens that guide as a card.
 *
 * **It goes only where the vocabulary is Armada's own** — a group boundary, a
 * landing rule, how a graph is read. A `?` on every noun is the same noise in
 * a smaller font.
 *
 * **Mounting is what counts as meeting the piece.** The rule is that a piece
 * is met when it is on a screen somebody is looking at, and a mark is drawn
 * beside the piece, so the mark being on screen is that fact with no second
 * registry to keep in step.
 */
export type GuideMarkProps = {
  guide: Guide;
  /**
   * Off where the piece is drawn but nobody can see it — a folded region, a
   * tab that is not the open one. Mounting would otherwise spend a person's
   * one first contact on something behind a chevron.
   */
  onScreen?: boolean;
};

/** The character, not a glyph. `circle-help` is refused: `circle-*` is the Judge's. */
const MARK = "?";

export function GuideMark({ guide, onScreen = true }: GuideMarkProps) {
  const { onOpen, onMet } = useGuidance();

  useEffect(() => {
    if (onScreen) onMet(guide);
  }, [onScreen, onMet, guide]);

  return (
    <button
      type="button"
      className="armada-guide-mark"
      aria-label={`Open guide ${guide.number}, ${guide.title}`}
      onClick={() => onOpen(guide)}
    >
      <span aria-hidden="true">{MARK}</span>
    </button>
  );
}
