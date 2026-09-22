// Record — one ledger of everything that happened to this Job, newest first.
//
// **It replaced a column of regions, which replaced a strip.** The regions were
// the run tree and the brief folded under eyebrows; before that `JobRecord`
// held five readings behind a strip of its own. Reading why a step was refused
// meant opening four views and lining the timestamps up by hand, which is the
// whole of `#1537`.
//
// The rows are composed from today's reads in `draft/ledger.ts`, the words are
// `record.ts`, and this file holds the open state of one reading.

import { useMemo, useState } from "react";
import {
  ADVANCE_GATE,
  Button,
  ConsoleOutput,
  JobLedger,
  STEP_STATE,
  type JobLedgerRow,
} from "@armada/components";
import type { JobDetail as JobWhole } from "@armada/protocol";

import { TAB_LABEL } from "./detail-tabs";
import { absoluteOf } from "./duration";
import { filtersOf, ledgerRowsFor, underFilter, type RecordFilter } from "./record";
import { noteFor, regionOf, rowsOf, useCheckOutputs, type ReadCheckOutput } from "./outputs";
import { FieldLabel } from "./regions";
import { taskGroupsOf } from "./draft/group";
import type { LedgerRow } from "./draft/ledger";

export type RecordTabProps = {
  jobId: string;
  /** The Job whole, where Fleet answered for it. */
  detail: JobWhole | null;
  /** Every row of the Record, newest first. */
  rows: readonly LedgerRow[];
  /** The window is under `--layout-breakpoint`. */
  narrow: boolean;
  /** The window is at `--window-floor`. */
  floor: boolean;
  onReadCheckOutput: ReadCheckOutput;
  /** Say a sentence to the person — what an act that is not built yet answers. */
  onSaid: (sentence: string) => void;
};

export function RecordTab({
  jobId,
  detail,
  rows,
  narrow,
  floor,
  onReadCheckOutput,
  onSaid,
}: RecordTabProps) {
  const [filter, setFilter] = useState<RecordFilter>("all");
  // Which row is open. **Held here and not in the ledger**, so a live redraw of
  // the Record does not close the row somebody is reading.
  const [openRow, setOpenRow] = useState<string | null>(null);

  const outputs = useCheckOutputs(onReadCheckOutput, jobId);

  const shown = useMemo(() => underFilter(rows, filter), [rows, filter]);
  const drawn = useMemo(() => ledgerRowsFor(shown, detail), [shown, detail]);
  const open = shown.find((row) => String(row.cursor) === openRow);
  const openDrawn = drawn.find((row) => row.id === openRow);

  return (
    <div className="armada-detail-tab" role="tabpanel" aria-label={TAB_LABEL.record}>
      {detail === null ? (
        <p className="armada-inside__absent" role="note">
          Fleet has not answered for this job, so there is nothing to read back.
        </p>
      ) : (
        <JobLedger
          rows={drawn}
          filters={filtersOf(rows)}
          filter={filter}
          onFilter={(id) => {
            setFilter(id as RecordFilter);
            // The open row may not answer the new filter, and an inspector
            // showing a row the table no longer holds is the stale panel this
            // screen exists to end.
            setOpenRow(null);
          }}
          openRow={openRow}
          onOpenRow={setOpenRow}
          narrow={narrow}
          floor={floor}
          emptyNote={EMPTY[filter]}
          {...(openDrawn === undefined ? {} : { inspectorTitle: plainly(openDrawn) })}
          {...(open === undefined || openDrawn === undefined
            ? {}
            : {
                inspector: (
                  <RowRead
                    row={open}
                    drawn={openDrawn}
                    detail={detail}
                    outputs={outputs}
                    onSaid={onSaid}
                  />
                ),
              })}
        />
      )}
    </div>
  );
}

/** What each filter says when it holds nothing. Never one sentence for eight. */
const EMPTY: Record<RecordFilter, string> = {
  all: "Nothing has happened on this Job yet.",
  evidence: "No Drone has submitted evidence on this Job.",
  files: "Nothing has written a file on this Job yet.",
  checks: "No Check has run on this Job yet.",
  judges: "No Judge has answered on this Job yet.",
  drones: "No Drone has opened a step on this Job yet.",
  tasks: "No task on this Job has moved yet.",
  tests: "No case has been run on this Job yet.",
};

/** The sheet's own name, which is the open row's. */
function plainly(drawn: JobLedgerRow): string {
  return typeof drawn.what === "string" ? drawn.what : "This row";
}

type RowReadProps = {
  row: LedgerRow;
  drawn: JobLedgerRow;
  detail: JobWhole;
  outputs: ReturnType<typeof useCheckOutputs>;
  onSaid: (sentence: string) => void;
};

/**
 * One row, read whole.
 *
 * **A Check shows its output and what it gated**, which is the reading the four
 * views could not put side by side: the run, the lines it printed, and the step
 * the run decided.
 */
function RowRead({ row, drawn, detail, outputs, onSaid }: RowReadProps) {
  const step = detail.steps.find((one) => one.step_id === row.coord?.step);
  const run =
    row.kind !== "checked"
      ? undefined
      : step?.check_runs.find(
          (one) => one.name === row.what && one.attempt === row.coord?.step_attempt,
        );
  const kept = run?.output_path;
  const held = kept === undefined ? undefined : outputs.of(basename(kept));

  return (
    <>
      <div className="armada-ledger__facts">
        <Fact label="When">{absoluteOf(row.at) ?? row.at}</Fact>
        <Fact label="Where">{drawn.where}</Fact>
        <Fact label="Who ran it">{drawn.whoSays}</Fact>
        <Fact label="Kind">{row.kind}</Fact>
      </div>

      {row.outcome === "" ? null : <p className="armada-ledger__note">{row.outcome}</p>}

      {run === undefined ? null : (
        <>
          {run.expected === undefined ? null : (
            <p className="armada-ledger__note">Expected {run.expected}</p>
          )}
          {/* What the Check gated: the step it ruled on, where that step now
              stands, and what the workflow said would decide it. */}
          {step === undefined ? null : (
            <p className="armada-ledger__note">
              Gated {step.label}, now {STEP_STATE[step.state]?.verb ?? step.state}.
              {step.advance_gate === undefined
                ? ""
                : ` It advances when ${ADVANCE_GATE[step.advance_gate]?.verb ?? step.advance_gate}.`}
            </p>
          )}
          {/* The output is fetched by whoever opened the row and never with it
              — a test runner's whole output is what the split keeps off the
              published state. So the control comes first and the reading
              replaces it. */}
          {kept === undefined ? (
            <p className="armada-ledger__note">This Check kept no output.</p>
          ) : held === undefined ? (
            <Button variant="secondary" onClick={() => outputs.fetch(basename(kept))}>
              Read what it printed
            </Button>
          ) : (
            <ConsoleOutput
              rows={held.state === "got" ? rowsOf(held.output) : []}
              {...(held.state === "got" ? { region: regionOf(held.output) } : {})}
              emptyNote={noteFor(held)}
            />
          )}
        </>
      )}

      <RunAgain row={row} detail={detail} onSaid={onSaid} />
    </>
  );
}

/**
 * Run this group again.
 *
 * **Mocked, and it says so when pressed.** No Fleet operation runs a group —
 * groups are not on the wire at all — so the control exists to be felt on the
 * mock and answers with what it would do rather than pretending to do it.
 */
function RunAgain({
  row,
  detail,
  onSaid,
}: {
  row: LedgerRow;
  detail: JobWhole;
  onSaid: (sentence: string) => void;
}) {
  const id = row.coord?.group;
  if (id === undefined) return null;
  const ordinal = taskGroupsOf(detail).find((group) => group.id === id)?.ordinal;
  if (ordinal === undefined) return null;
  const says = `Run group ${ordinal} again`;
  return (
    <Button
      variant="secondary"
      onClick={() => onSaid(`${says} — not built yet. Fleet has no operation for it.`)}
    >
      {says}
    </Button>
  );
}

function Fact({ label, children }: { label: string; children: React.ReactNode }) {
  return (
    <span className="armada-ledger__fact">
      <FieldLabel>{label}</FieldLabel>
      <span className="armada-ledger__fact-value">{children}</span>
    </span>
  );
}

// `outputs.fetch` is keyed by the file's own name and never the whole path —
// `fleet` builds that name out of the run's key, so a fixture or a record
// keyed on the path would never be found.
function basename(path: string): string {
  return path.slice(path.lastIndexOf("/") + 1);
}
