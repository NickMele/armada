import type { Guide } from "../../guides/guide";
import type { GuideShape } from "../../guide-shape";
import { GuideFigure } from "../GuideFigure/GuideFigure";

/**
 * A guide read as steps, or read as a figure with captions. **A prototype**:
 * two shapes built beside the prose one so the owner can choose between them.
 *
 * **`prose` never arrives here.** The panel and the card each draw their own
 * paragraphs and that rendering is untouched, so the baseline he is comparing
 * against is the same bytes it was.
 *
 * **One component for both surfaces**, so a shape reads the same in the wide
 * panel and in the small `?` card. The card is where the `figure` shape costs
 * the most, and it costs it here rather than being special-cased away.
 */
export type GuideShapedProps = {
  guide: Guide;
  shape: Exclude<GuideShape, "prose">;
  /** The catalogue panel, or the card the `?` opens. */
  where: "panel" | "card";
};

export function GuideShaped({ guide, shape, where }: GuideShapedProps) {
  const shapes = guide.shapes;
  if (shapes === undefined) return null;

  if (shape === "figure") {
    return (
      <div className="armada-guide-shaped" data-shape="figure" data-where={where}>
        <GuideFigure figure={shapes.figure.id} scale="lead" />
        {shapes.figure.captions.map((caption) => (
          <p key={caption} className="armada-guide-shaped__caption">
            {caption}
          </p>
        ))}
      </div>
    );
  }

  const { lines, figure } = shapes.steps;
  return (
    <div className="armada-guide-shaped" data-shape="steps" data-where={where}>
      <ol className="armada-guide-shaped__steps">
        {lines.map((line, index) => (
          <li key={line} className="armada-guide-shaped__step">
            <span className="armada-guide-shaped__ordinal mono">{index + 1}</span>
            <span className="armada-guide-shaped__line">{line}</span>
            {/* Only where a relation is the thing being learned, and no frame is
                held where one would go: a guide with none is the lines alone. */}
            {figure !== undefined && figure.at === index + 1 ? (
              <div className="armada-guide-shaped__figure">
                <GuideFigure figure={figure.id} scale="inline" />
              </div>
            ) : null}
          </li>
        ))}
      </ol>
    </div>
  );
}
