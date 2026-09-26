import type { Guide } from "../../guides/guide";
import { GuideFigure } from "../GuideFigure/GuideFigure";

/**
 * A guide, read: a numbered sequence with the drawing under the step it
 * belongs to. **The one shape**, chosen by the owner on 26 September 2026 out
 * of three that were built — the figure over the steps, in one guide.
 *
 * **One component for both surfaces**, so a guide reads the same in the wide
 * panel and in the small `?` card. Which of the two it is reaches the figure,
 * because the card is a narrow layer and the panel is not.
 */
export type GuideStepsProps = {
  guide: Guide;
  /** The catalogue panel, or the card the `?` opens. */
  where: "panel" | "card";
};

export function GuideSteps({ guide, where }: GuideStepsProps) {
  const { steps, figure } = guide;
  return (
    <ol className="armada-guide-steps" data-where={where}>
      {steps.map((line, index) => (
        <li key={line} className="armada-guide-steps__step">
          <span className="armada-guide-steps__ordinal mono">{index + 1}</span>
          <span className="armada-guide-steps__line">{line}</span>
          {/* Only where a relation is the thing being learned, and no frame is
              held where one would go: a guide with none is the lines alone. */}
          {figure !== undefined && figure.at === index + 1 ? (
            <div className="armada-guide-steps__figure">
              <GuideFigure figure={figure.id} scale={where} />
            </div>
          ) : null}
        </li>
      ))}
    </ol>
  );
}
