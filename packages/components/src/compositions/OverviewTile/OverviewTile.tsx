import { ChevronRight } from "lucide-react";
import type { HTMLAttributes, ReactNode } from "react";
import { Skeleton } from "../../primitives/Skeleton/Skeleton";

/**
 * One reading Fleet already serves, at a glance — Overview's band.
 *
 * **The hue is a token's, named by the token.** Fleet and Doctor borrow the Job
 * values the status bar and Doctor's grid already borrow, and drift takes
 * `--notice-caution`. A tile never picks a colour of its own.
 *
 * **A tile that opens something is one control and trails `chevron-right`** in
 * its *goes to* use. One that opens nothing trails no glyph and is not a button,
 * so nothing on it reads as pressable.
 */
export type OverviewTileTone =
  | "completed-success"
  | "awaiting-review"
  | "completed-failed"
  | "notice-caution";

export type OverviewTileProps = {
  /** What the reading is of — `Fleet`, `Drones`. */
  label: string;
  /** The reading. Absent is not read yet, and draws a placeholder rather than a zero. */
  value?: ReactNode;
  /** Mono for a figure or a word the system reported; sans for a sentence. */
  valueFace?: "sans" | "mono";
  tone?: OverviewTileTone;
  /** One line beneath the reading, in `--fg-muted`. */
  detail?: ReactNode;
  detailFace?: "sans" | "mono";
  /** Where pressing it goes, said to a screen reader — `Fleet settings`. Needs `onOpen`. */
  opens?: string;
  onOpen?: () => void;
};

const GLYPH = 12;
const STROKE = 2;

export function OverviewTile({
  label,
  value,
  valueFace = "sans",
  tone,
  detail,
  detailFace = "sans",
  opens,
  onOpen,
}: OverviewTileProps) {
  const body = (
    <span className="armada-overview-tile__body">
      <span className="armada-overview-tile__label">{label}</span>
      {value === undefined ? (
        <Skeleton className="armada-overview-tile__placeholder" width="50%" />
      ) : (
        <span className="armada-overview-tile__value" data-face={valueFace} data-tone={tone}>
          {value}
        </span>
      )}
      {detail === undefined ? null : (
        <span className="armada-overview-tile__detail" data-face={detailFace}>
          {detail}
        </span>
      )}
    </span>
  );

  if (onOpen === undefined) {
    return (
      <div className="armada-overview-tile" role="group" aria-label={label}>
        {body}
      </div>
    );
  }
  return (
    <button type="button" className="armada-overview-tile" onClick={onOpen}>
      {body}
      <ChevronRight size={GLYPH} strokeWidth={STROKE} className="armada-overview-tile__mark" aria-hidden />
      {opens === undefined ? null : <span className="armada-overview-tile__sr">Open {opens}</span>}
    </button>
  );
}

/** The tiles, in a band that wraps to the width it is given. */
export function OverviewTileBand({ className, ...rest }: HTMLAttributes<HTMLDivElement>) {
  return <div className={className ? `armada-overview-band ${className}` : "armada-overview-band"} {...rest} />;
}
