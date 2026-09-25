import { useEffect, useRef } from "react";

import { ScrollArea } from "../../primitives/ScrollArea/ScrollArea";
import { Switch } from "../../primitives/Switch/Switch";
import type { Guide } from "../../guides/guide";
import { useGuideShape } from "../../guide-shape";
import { GuideShaped } from "../GuideShaped/GuideShaped";

/**
 * One guide, read. A number, a title, room for a picture and a few short
 * paragraphs — the shape a game's guide entry has.
 *
 * **A card is read, never glimpsed**, which is why it is a framed layer and
 * not a tooltip. It is centred in the window, it takes a scrim, and it goes
 * away on Escape, on the scrim, or on Close.
 *
 * **Not a second tone on `Dialog`.** That primitive is a confirmation: it has
 * a tone, a confirm and a cancel, and an `Enter` contract about which of the
 * two fires. Nothing here is confirmed. It follows the same framed-layer rules
 * and shares none of that machinery.
 */
export type GuideCardProps = {
  guide: Guide;
  /**
   * Somebody pressed for it. **An uninvited card does not animate in** — the
   * contract lets chrome a person summoned enter, and this is the one layer in
   * Bridge that can arrive without being asked for.
   */
  invited?: boolean;
  onClose: () => void;
  /**
   * The switch that turns first contact off, drawn on the first card anybody
   * ever sees and on no other. Absent means the card carries none.
   */
  off?: boolean;
  onOff?: (off: boolean) => void;
  /** Opens the catalogue. Absent where there is nowhere to go. */
  onReadAll?: () => void;
};

export function GuideCard({ guide, invited = true, onClose, off, onOff, onReadAll }: GuideCardProps) {
  const closeRef = useRef<HTMLButtonElement>(null);
  // The mock's `?guides=`. Bridge provides no shape, so a card in the app is
  // prose and the paragraphs below are the rendering it always had.
  const shape = useGuideShape();
  const shaped = shape !== "prose" && guide.shapes !== undefined ? shape : undefined;

  // Close holds initial focus, `Dialog`'s own rule: the safe control is the
  // one under the cursor, and on a card every control is safe but one is the
  // way out.
  useEffect(() => closeRef.current?.focus(), []);

  useEffect(() => {
    function onKey(event: KeyboardEvent): void {
      if (event.key !== "Escape") return;
      event.preventDefault();
      onClose();
    }
    window.addEventListener("keydown", onKey);
    return () => window.removeEventListener("keydown", onKey);
  }, [onClose]);

  const offered = off !== undefined && onOff !== undefined;

  return (
    <div className="armada-guide-card-scrim" onClick={onClose}>
      {/* The press that opened this card is still travelling up the tree on the
          scrim's own handler, so the layer stops it rather than closing under
          the reader. */}
      <div
        className="armada-guide-card"
        data-invited={invited || undefined}
        role="dialog"
        aria-modal="true"
        aria-label={`Guide ${guide.number}, ${guide.title}`}
        onClick={(event) => event.stopPropagation()}
      >
        <div className="armada-guide-card__head">
          <span className="armada-guide-card__number mono">Guide {guide.number}</span>
          <h2 className="armada-guide-card__title">{guide.title}</h2>
        </div>

        <ScrollArea className="armada-guide-card__body">
          {guide.picture === undefined ? null : (
            <img className="armada-guide-card__picture" src={guide.picture.src} alt={guide.picture.alt} />
          )}
          {shaped !== undefined ? <GuideShaped guide={guide} shape={shaped} where="card" /> : null}
          {shaped !== undefined
            ? null
            : guide.body.map((paragraph) => (
                <p key={paragraph} className="armada-guide-card__paragraph">
                  {paragraph}
                </p>
              ))}
        </ScrollArea>

        {offered ? (
          <div className="armada-guide-card__offer">
            <Switch
              checked={off !== true}
              onChange={(event) => onOff(!event.currentTarget.checked)}
              description={
                off === true
                  ? "Off. Nothing opens on its own. The ? beside a piece still opens its guide, and every guide is in the catalogue."
                  : "A card opens once for a piece you have not met, and never again for that one. Turn this off and nothing opens on its own."
              }
            >
              Open a guide the first time I meet a piece
            </Switch>
          </div>
        ) : null}

        <div className="armada-guide-card__actions">
          {onReadAll === undefined ? null : (
            <button type="button" className="armada-guide-card__secondary" onClick={onReadAll}>
              Read all guides
            </button>
          )}
          <button ref={closeRef} type="button" className="armada-guide-card__close" onClick={onClose}>
            Close
          </button>
        </div>
      </div>
    </div>
  );
}
