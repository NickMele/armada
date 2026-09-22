import type { ReactNode } from "react";
import { Sheet } from "../../primitives/Sheet/Sheet";
import {
  Table,
  TableBody,
  TableCell,
  TableHead,
  TableHeaderCell,
  TableRow,
} from "../../primitives/Table/Table";
import { TabsWithCounts } from "../../primitives/TabsWithCounts/TabsWithCounts";

/**
 * Job ledger — one table of everything that happened to a Job, newest first.
 *
 * **It replaces `JobRecord` rather than nesting inside it.** That one folded
 * five readings behind a strip of its own, so reading why a step was refused
 * meant opening four and lining timestamps up by hand.
 *
 * **No row is counted twice, and the filters need not sum to All** — the board
 * this came from said All 34 while its filters summed to 35. A row that
 * answers to no filter is under All alone, and `note` is where the surface
 * says how many and what they are.
 *
 * **Who ran it is a column**, and telling a Check, a Judge and Fleet apart is
 * most of what this table is for.
 *
 * **The open row is a sheet at every width, unlike the step inspector.** That
 * one pays for a column because the run tree beside it is drawn for 240px. Six
 * columns are not: at 1440 with Helm's dock open the table had 250px left and
 * broke a repository path one character to a line, which is the v1 defect the
 * fold exists to prevent. `floor.ts` also says the window measure answers for
 * the window and not for a component's box, and the dock is 320px of it.
 */

/** Who ran a row, as the column reads it. */
export type LedgerWho = "you" | "contributor" | "fleet" | "drone" | "judge" | "check";

/**
 * What a row came to, for the hue. **None of these is a Job status** — a failed
 * Check inside a running Job is not a failed Job, and `tokens/status.css` is
 * what they alias.
 */
export type LedgerTone = "passed" | "failed" | "waiting" | "running";

export type JobLedgerRow = {
  /** What a selection names. Stable across renders, never the row's position. */
  id: string;
  /** When, already written for a reader: this owns chrome, the surface owns words. */
  when: ReactNode;
  /** The full instant, for the pointer. */
  whenExact?: string;
  /**
   * Where in the Job it happened — the step, its group and its task where it
   * has one. **`Where` and not `Step`**: a fact about the Job's own machine
   * names no step, and a blank cell would read as a gap.
   */
  where: ReactNode;
  who: LedgerWho;
  /** Who, spelled. The caller's, so a Drone's row can name the Drone. */
  whoSays: ReactNode;
  /**
   * The kind, as the record spells it. **The one column that gives way below
   * `--layout-breakpoint`** — what it says, the `what` beside it says in words.
   */
  kind: ReactNode;
  /** What happened. The thing the row is about. */
  what: ReactNode;
  /** What it came to. Absent where the row states no outcome, never a placeholder. */
  outcome?: ReactNode;
  tone?: LedgerTone;
};

/** One filter, with how many rows answer to it. */
export type JobLedgerFilter = {
  id: string;
  /** Sentence case. */
  label: string;
  count: number;
};

export type JobLedgerProps = {
  /** The rows the chosen filter holds, newest first. */
  rows: readonly JobLedgerRow[];
  /** The strip above the table — All first, and the families after it. */
  filters: readonly JobLedgerFilter[];
  filter: string;
  onFilter: (id: string) => void;
  /**
   * One line under the strip, for what the counts cannot say — the rows All
   * holds that no filter does. **Absent draws nothing**, which is the ordinary
   * case: it appears only where the arithmetic would otherwise be a reader's
   * to do.
   */
  note?: ReactNode;
  /** Which row is open, held by the surface so a live redraw does not close it. */
  openRow?: string | null;
  onOpenRow?: (rowId: string | null) => void;
  /** The open row, read whole. The body of the sheet a press opens. */
  inspector?: ReactNode;
  /** The folded inspector's own name, which is the open row's. */
  inspectorTitle?: string;
  /** Why no row is open, where none is. */
  inspectorAbsent?: string;
  /** What the table says where the chosen filter holds nothing. */
  emptyNote?: ReactNode;
  /**
   * How many rows are drawn. A Record grows for as long as the Job lives, and
   * what is left out is counted under the table rather than truncated quietly.
   */
  bound?: number;
  /**
   * The window is under `--layout-breakpoint`: the Kind column gives way.
   *
   * **It does not decide where the inspector goes**, because the inspector is
   * a sheet at every width — see the component's own note.
   */
  narrow?: boolean;
  /** The window is at `--window-floor`, where the folded inspector goes flush. */
  floor?: boolean;
};

/** A whole Job's worth of reading, without the list becoming the cost. */
const BOUND = 200;

export function JobLedger({
  rows,
  filters,
  filter,
  onFilter,
  note,
  openRow = null,
  onOpenRow,
  inspector,
  inspectorTitle,
  inspectorAbsent = "Press a row to read it whole",
  emptyNote = "Nothing under this filter yet",
  bound = BOUND,
  narrow = false,
  floor = false,
}: JobLedgerProps) {
  const drawn = rows.slice(0, bound);
  const leftOut = rows.length - drawn.length;

  return (
    <>
      <div className="armada-ledger" data-narrow={narrow || undefined}>
        <div className="armada-ledger__list">
          <TabsWithCounts
            label="What the Record holds"
            value={filter}
            onChange={onFilter}
            items={filters.map((one) => ({ id: one.id, label: one.label, count: one.count }))}
          />

          {note === undefined ? null : (
            <p className="armada-ledger__note" role="note">
              {note}
            </p>
          )}

          {drawn.length === 0 ? (
            <p className="armada-ledger__note" role="note">
              {emptyNote}
            </p>
          ) : (
            <Table className="armada-ledger__table">
              <TableHead>
                <TableRow>
                  <TableHeaderCell className="armada-ledger__when">When</TableHeaderCell>
                  <TableHeaderCell className="armada-ledger__where">Where</TableHeaderCell>
                  <TableHeaderCell className="armada-ledger__who">Who ran it</TableHeaderCell>
                  {narrow ? null : (
                    <TableHeaderCell className="armada-ledger__kind">Kind</TableHeaderCell>
                  )}
                  <TableHeaderCell className="armada-ledger__what">What</TableHeaderCell>
                  <TableHeaderCell className="armada-ledger__outcome">Outcome</TableHeaderCell>
                </TableRow>
              </TableHead>
              <TableBody>
                {drawn.map((row) => (
                  <TableRow
                    key={row.id}
                    selected={row.id === openRow}
                    data-tone={row.tone}
                    data-row-id={row.id}
                    // The whole row opens it, the way a verdict's row opens its
                    // refusal: a sentence is a larger thing to hit than a glyph.
                    onClick={onOpenRow === undefined ? undefined : () => onOpenRow(row.id)}
                  >
                    <TableCell
                      variant="metadata"
                      className="armada-ledger__when"
                      title={row.whenExact}
                    >
                      {row.when}
                    </TableCell>
                    <TableCell variant="secondary" className="armada-ledger__where">
                      {row.where}
                    </TableCell>
                    <TableCell variant="secondary" className="armada-ledger__who">
                      <span className="armada-ledger__actor" data-who={row.who}>
                        {row.whoSays}
                      </span>
                    </TableCell>
                    {narrow ? null : (
                      <TableCell variant="mono" className="armada-ledger__kind">
                        {row.kind}
                      </TableCell>
                    )}
                    <TableCell className="armada-ledger__what">
                      {onOpenRow === undefined ? (
                        row.what
                      ) : (
                        // The keyboard's path to the same act, and the row's
                        // accessible name. The press stops here, or the row's
                        // own handler answers it a second time — harmless on
                        // screen, and a second report to whoever is counting.
                        <button
                          type="button"
                          className="armada-ledger__open"
                          aria-expanded={row.id === openRow}
                          onClick={(event) => {
                            event.stopPropagation();
                            onOpenRow(row.id);
                          }}
                        >
                          {row.what}
                        </button>
                      )}
                    </TableCell>
                    <TableCell variant="secondary" className="armada-ledger__outcome">
                      {row.outcome}
                    </TableCell>
                  </TableRow>
                ))}
              </TableBody>
            </Table>
          )}

          {leftOut > 0 ? (
            <p className="armada-ledger__note" role="note">
              {leftOut} older {leftOut === 1 ? "row is" : "rows are"} not drawn
            </p>
          ) : null}
        </div>

      </div>

      {/* Contained, so the shell's rail stays out from under the layer. */}
      <Sheet
        open={openRow !== null}
        contained
        floor={floor}
        title={inspectorTitle ?? "This row"}
        closeLabel="Close"
        closeBinding="Esc"
        bleed
        onClose={() => onOpenRow?.(null)}
      >
        <div className="armada-ledger__panel">
          {inspector ?? (
            <p className="armada-ledger__note" role="note">
              {inspectorAbsent}
            </p>
          )}
        </div>
      </Sheet>
    </>
  );
}
