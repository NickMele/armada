// The list. **Not the Job Board** — that is the Surface milestone's, with its
// own journey and its own component inventory. This is every Job, its state,
// the step it is on, and the one decision a person can make from here.
//
// Column identity and order are a module constant, not something derived from
// the rows that just arrived. Columns flip-flopping between renders was its own
// line in v1's failure log, and it is what deriving layout from a data snapshot
// looks like from the outside.

import { Badge, Button, Table, TableBody, TableCell, TableHead, TableHeaderCell, TableRow } from "@armada/components";

import type { JobSummary, UnreadableJob } from "../../shared/protocol";
import { readingOf } from "./reading";

/**
 * How many rows are drawn. Nothing renders an unbounded list directly, and no
 * virtualization library has been chosen — so the list is bounded and says what
 * it left out, which is the honest half of the same rule.
 */
const DRAWN = 200;

/** Stable across every render. The schema, not the snapshot. */
const COLUMNS = ["State", "Job", "Step", "Drone", "Workflow", "Model", ""] as const;

export type JobsProps = {
  jobs: readonly JobSummary[];
  unreadable: readonly UnreadableJob[];
  /** Jobs with an approval in flight. A second click on one does not dispatch twice. */
  approving: readonly string[];
  /** True while what is shown is not live. Every row reads as de-emphasised. */
  stale: boolean;
  onApprove: (jobId: string) => void;
};

export function Jobs({ jobs, unreadable, approving, stale, onApprove }: JobsProps) {
  const drawn = jobs.slice(0, DRAWN);

  return (
    <div className="flex flex-col gap-2">
      <Table>
        <TableHead>
          <TableRow>
            {COLUMNS.map((column) => (
              <TableHeaderCell key={column}>{column}</TableHeaderCell>
            ))}
          </TableRow>
        </TableHead>
        <TableBody>
          {drawn.map((job) => (
            <Row
              key={job.id}
              job={job}
              approving={approving.includes(job.id)}
              stale={stale}
              onApprove={onApprove}
            />
          ))}
        </TableBody>
      </Table>

      {jobs.length === 0 ? <p className="text-fg-muted">No jobs.</p> : null}
      {jobs.length > drawn.length ? (
        <p className="text-fg-muted">
          {`${drawn.length} of ${jobs.length} jobs drawn. The rest are on Fleet and not on screen.`}
        </p>
      ) : null}

      {/* Never merged into the list as a placeholder: a board that shows nine
          of ten Jobs and says so is honest, one that shows nine is not. */}
      {unreadable.map((row) => (
        <p key={row.job_id ?? row.fault} className="text-status-completed-failed">
          {row.job_id === undefined ? "A job did not load: " : `Job ${row.job_id} did not load: `}
          <span className="mono">{row.fault}</span>
        </p>
      ))}
    </div>
  );
}

function Row({
  job,
  approving,
  stale,
  onApprove,
}: {
  job: JobSummary;
  approving: boolean;
  stale: boolean;
  onApprove: (jobId: string) => void;
}) {
  const reading = readingOf(job);
  // The gate is the status, not the button. A Job that has moved off
  // `awaiting_approval` has no approve control at all, so the second click has
  // nothing to hit even before the guard in the main process refuses it.
  const atTheGate = job.status === "awaiting_approval";

  return (
    <TableRow dimmed={stale}>
      <TableCell>
        {reading.as === "badge" ? (
          <Badge status={reading.status} icon={reading.icon}>
            {reading.verb}
          </Badge>
        ) : (
          <span className="text-fg-muted">
            {reading.verb === null ? null : `${reading.verb} `}
            <span className="mono">{reading.wire}</span>
          </span>
        )}
      </TableCell>
      <TableCell variant="mono">{job.id}</TableCell>
      <TableCell variant="mono">{job.current_step_id ?? ""}</TableCell>
      <TableCell variant="mono">{job.assigned_drone ?? ""}</TableCell>
      <TableCell variant="mono">{job.workflow_id}</TableCell>
      <TableCell variant="mono">{job.model}</TableCell>
      <TableCell variant="secondary">
        {atTheGate ? (
          <Button size="sm" onClick={() => onApprove(job.id)} disabled={approving || stale}>
            {approving ? "Approving" : "Approve dispatch"}
          </Button>
        ) : null}
      </TableCell>
    </TableRow>
  );
}
