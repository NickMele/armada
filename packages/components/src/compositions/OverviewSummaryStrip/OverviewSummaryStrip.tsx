/**
 * Overview's summary strip — every panel's count above the fold. Overview 27
 * (#1091) replaced the tile band with this; Fleet, Doctor, Drones and
 * Manifest drift moved to the left column (#1088).
 *
 * **No dot.** The count in mono is the one figure a glance is for.
 *
 * **A tone is the caller's decision, applied only past zero** — an empty
 * count is always subtle, so a caller cannot make it read as urgent.
 */

/** The two Job-state hues a strip item can carry — never a colour of its own. */
export type OverviewSummaryStripTone = "awaiting-review" | "completed-failed";

export type OverviewSummaryStripItem = {
  id: string;
  label: string;
  count: number;
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
    <nav className="armada-overview-summary" aria-label="What is on Overview" data-count={items.length}>
      {items.map((item) => {
        const empty = item.count === 0;
        return (
          <button
            key={item.id}
            type="button"
            className="armada-overview-summary__item"
            data-tone={empty ? undefined : item.tone}
            data-empty={empty || undefined}
            aria-label={`${item.count} ${item.label}`}
            onClick={item.onPress}
          >
            <span className="armada-overview-summary__count" aria-hidden>
              {item.count}
            </span>
            <span className="armada-overview-summary__label" aria-hidden>
              {item.label}
            </span>
          </button>
        );
      })}
    </nav>
  );
}
