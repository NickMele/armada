import { useId, type CSSProperties } from "react";

/**
 * Footprint — a finished Job's files as columns, width by lines changed, split
 * added over deleted. `docs/contracts/design-system.md`, under Instruments.
 *
 * **A file with no line count is never a column.** It arrives as `uncounted`
 * and is said in the key, because a guessed width is a wrong measurement.
 */
export type FootprintColumn = {
  path: string;
  added: number;
  deleted: number;
  /** Outside every declared plan: `planned_by` present and empty. */
  outsidePlan: boolean;
};

export type FootprintInstrumentProps = {
  /** In the order they are drawn, left to right. */
  columns: readonly FootprintColumn[];
  /** Files the record could not count. */
  uncounted: number;
  /** What the instrument measures, `Lines changed per file`. Its accessible name. */
  label: string;
  /** The reading in words. */
  description: string;
};

export function FootprintInstrument({ columns, uncounted, label, description }: FootprintInstrumentProps) {
  const labelId = useId();
  const descriptionId = useId();
  const added = columns.reduce((sum, column) => sum + column.added, 0);
  const deleted = columns.reduce((sum, column) => sum + column.deleted, 0);
  const total = added + deleted;
  const drifted = columns.some((column) => column.outsidePlan);
  let start = 0;
  return (
    <div
      className="armada-instrument-footprint"
      role="img"
      aria-labelledby={labelId}
      aria-describedby={descriptionId}
    >
      <div className="armada-instrument-footprint__head">
        <span id={labelId}>{label}</span>
        {total === 0 ? null : (
          <span>
            +{added} −{deleted}
          </span>
        )}
      </div>
      {columns.length === 0 || total === 0 ? null : (
        <svg className="armada-instrument-footprint__plot" aria-hidden>
          {columns.map((column) => {
            const lines = column.added + column.deleted;
            const at = start;
            start += lines;
            const place = {
              "--armada-footprint-x": `${(at / total) * 100}%`,
              "--armada-footprint-w": `${(lines / total) * 100}%`,
            } as CSSProperties;
            const split = lines === 0 ? 0 : (column.added / lines) * 100;
            return (
              <g key={column.path}>
                <title>{`${column.path}  +${column.added} −${column.deleted}`}</title>
                <rect className="armada-instrument-footprint__added" y="0" height={`${split}%`} style={place} />
                <rect
                  className="armada-instrument-footprint__deleted"
                  y={`${split}%`}
                  height={`${100 - split}%`}
                  style={place}
                />
                {column.outsidePlan ? (
                  <rect className="armada-instrument-footprint__outside" y="0" height="100%" style={place} />
                ) : null}
              </g>
            );
          })}
        </svg>
      )}
      <div className="armada-instrument-footprint__key">
        {total === 0 ? null : (
          <>
            <span className="armada-instrument-footprint__entry">
              <Swatch className="armada-instrument-footprint__added" />
              added
            </span>
            <span className="armada-instrument-footprint__entry">
              <Swatch className="armada-instrument-footprint__deleted" />
              deleted
            </span>
          </>
        )}
        {drifted ? (
          <span className="armada-instrument-footprint__entry">
            <Swatch className="armada-instrument-footprint__outside" />
            outside every declared plan
          </span>
        ) : null}
        {uncounted === 0 ? null : (
          <span>
            {uncounted} {uncounted === 1 ? "file" : "files"} not counted
          </span>
        )}
        {total === 0 && uncounted === 0 ? <span>no lines counted</span> : null}
      </div>
      <span id={descriptionId} className="armada-instrument-footprint__description">
        {description}
      </span>
    </div>
  );
}

/** A key entry's mark, drawn with the class the column it names uses. */
function Swatch({ className }: { className: string }) {
  return (
    <svg className="armada-instrument-footprint__swatch" aria-hidden>
      <rect className={className} x="0" y="0" width="100%" height="100%" />
    </svg>
  );
}
