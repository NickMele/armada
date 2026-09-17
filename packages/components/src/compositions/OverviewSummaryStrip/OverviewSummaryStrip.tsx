/**
 * Overview's summary tiles — four glass cards, two by two, one per panel below.
 * `docs/contracts/design-system.md` → Overview summary tiles. #1091, #1261.
 *
 * **Each tile wears its hue at every count** — the dot, the label and a corner
 * wash — so the counts sort by colour before they are read. The hue is the
 * caller's status stem, read as `--status-{stem}` the way Badge reads its own.
 *
 * **A tone is the caller's too, applied only past zero** — an empty count is
 * always `--fg-muted`, so a "0" never reads as something waiting.
 */
import type { CSSProperties } from "react";

/** The two Job-state hues a count past zero can carry — never a colour of its own. */
export type OverviewSummaryStripTone = "awaiting-review" | "completed-failed";

export type OverviewSummaryStripItem = {
  id: string;
  label: string;
  count: number;
  /** The status token stem the tile wears at every count, e.g. `running`. */
  hue: string;
  /** Applied only where `count` is past zero — see the file header. */
  tone?: OverviewSummaryStripTone;
  onPress: () => void;
};

export type OverviewSummaryStripProps = {
  /** Four, since Overview 28 (#1092) added Recently ended. */
  items: OverviewSummaryStripItem[];
};

export function OverviewSummaryStrip({ items }: OverviewSummaryStripProps) {
  return (
    <nav className="armada-overview-summary" aria-label="What is on Overview">
      {items.map((item) => {
        const empty = item.count === 0;
        return (
          <button
            key={item.id}
            type="button"
            className="armada-overview-summary__item armada-glass"
            data-tone={empty ? undefined : item.tone}
            data-empty={empty || undefined}
            style={{ "--armada-tile-hue": `var(--status-${item.hue})` } as CSSProperties}
            aria-label={`${item.count} ${item.label}`}
            onClick={item.onPress}
          >
            <span className="armada-overview-summary__label" aria-hidden>
              <span className="armada-overview-summary__dot" />
              <span className="armada-overview-summary__label-text">{item.label}</span>
            </span>
            <span className="armada-overview-summary__count" aria-hidden>
              {item.count}
            </span>
          </button>
        );
      })}
    </nav>
  );
}
