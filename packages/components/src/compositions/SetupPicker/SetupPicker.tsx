import { Check } from "lucide-react";

import { Button } from "../../primitives/Button/Button";
import { Checkbox } from "../../primitives/Checkbox/Checkbox";

/**
 * Setup's picker — Journey 3's *The picker* and *Check names across the batch*: the surface a
 * person stands on while each proposal opens over it, in any order.
 *
 * **Two controls a row, doing different jobs.** The tick decides whether a workspace is in the
 * batch; Open brings up its proposal. **No hue on a state**: none of them is a Job state. The
 * open row takes the accent edge and tint, the pair that means *the row you are on*.
 *
 * The grid is over the ticked batch. A cell is a tick, a dash, or `missing` on
 * `--step-stopped-bg` — a name every other ticked workspace declares and this one does not.
 */

export type SetupPickerRow = {
  /** Relative to the checkout, `.` for the root. Mono. */
  dir: string;
  ticked: boolean;
  open: boolean;
  /** The files Scan read here, mono. */
  files: string[];
  /** A sentence where the files do not say enough — nothing here could be read. Sans. */
  note?: string;
  /** Reports, never instructs: `ready to write`, `no checks proposed`. */
  state: string;
};

export type SetupGridCell = "declared" | "absent" | "missing";

export type SetupPickerProps = {
  rows: SetupPickerRow[];
  onTick: (dir: string, ticked: boolean) => void;
  onOpen: (dir: string) => void;
  /** Check names across the ticked batch, in first-declared order. */
  names: string[];
  /** One row per ticked workspace, a cell per name. */
  grid: { dir: string; cells: SetupGridCell[] }[];
};

export function SetupPicker({ rows, onTick, onOpen, names, grid }: SetupPickerProps) {
  return (
    <div className="armada-setup-picker">
      <section className="armada-setup-picker__panel" aria-label="Workspaces">
        <h2 className="armada-setup-picker__title">Workspaces</h2>
        <p className="armada-setup-picker__says">
          Ticked where a file names something runnable. Open one to read and correct its proposal.
        </p>
        <ul className="armada-setup-picker__rows">
          {rows.map((row) => (
            <li
              key={row.dir}
              className="armada-setup-picker__row"
              aria-label={row.dir}
              aria-current={row.open || undefined}
              data-open={row.open || undefined}
            >
              <Checkbox checked={row.ticked} onChange={(event) => onTick(row.dir, event.target.checked)}>
                <span className="armada-setup-picker__dir">{row.dir}</span>
              </Checkbox>
              <span className="armada-setup-picker__detail">
                {row.files.length === 0 ? null : (
                  <span className="armada-setup-picker__files">{row.files.join(", ")}</span>
                )}
                {row.note === undefined ? null : <span>{row.note}</span>}
              </span>
              <span className="armada-setup-picker__state">{row.state}</span>
              <Button variant="ghost" size="sm" aria-label={`Open ${row.dir}`} onClick={() => onOpen(row.dir)}>
                Open
              </Button>
            </li>
          ))}
        </ul>
      </section>

      <section className="armada-setup-picker__panel" aria-label="Check names across the batch">
        <h2 className="armada-setup-picker__title">Check names across the batch</h2>
        <p className="armada-setup-picker__says">
          A step naming a Check a workspace does not declare records it as not run, so a gap here is
          a pass that never ran. Marked only where every other ticked workspace declares the name.
        </p>
        {names.length === 0 || grid.length === 0 ? (
          <p className="armada-setup-picker__says">No ticked workspace proposes a Check.</p>
        ) : (
          <div className="armada-setup-picker__scroll">
            <table className="armada-setup-picker__grid">
              <thead>
                <tr>
                  <th scope="col">Workspace</th>
                  {names.map((name) => (
                    <th key={name} scope="col" className="armada-setup-picker__name">
                      {name}
                    </th>
                  ))}
                </tr>
              </thead>
              <tbody>
                {grid.map((row) => (
                  <tr key={row.dir}>
                    <th scope="row">
                      <Button variant="ghost" size="sm" onClick={() => onOpen(row.dir)}>
                        <span className="armada-setup-picker__dir">{row.dir}</span>
                      </Button>
                    </th>
                    {row.cells.map((cell, at) => (
                      <td key={names[at]} data-cell={cell} className="armada-setup-picker__cell">
                        {cell === "declared" ? (
                          <Check size={12} strokeWidth={2} aria-label="declared" />
                        ) : cell === "missing" ? (
                          "missing"
                        ) : (
                          <span aria-label="not declared">—</span>
                        )}
                      </td>
                    ))}
                  </tr>
                ))}
              </tbody>
            </table>
          </div>
        )}
      </section>
    </div>
  );
}
