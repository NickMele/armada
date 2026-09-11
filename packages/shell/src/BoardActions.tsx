// The Board's one control: `New job`, with everything else the Board offers
// in its menu.
//
// **One control, where there were six.** The head carried Refresh, Reported,
// Held disk and New job, and the Board carried Clear and Delete in a row of
// their own beneath it. The owner ruled on 11 Sep 2026 that a head carries one
// action and a menu, the rule Job detail's header already keeps, so the likely
// act is on the face and the rest are one click away.
//
// **The two bulk acts still confirm on their own.** Delete is the one act on
// the Board that cannot be undone, and a person reaching for Clear must never
// end up there by the same press, so each opens its own dialog.

import { Dialog, JOB_LIFECYCLE, SplitButton, type SplitButtonItem } from "@armada/components";
import type { JobSummary } from "@armada/protocol";
import { useState } from "react";

/** Which finished Jobs each bulk act would reach. */
export function terminalOf(jobs: readonly JobSummary[]): {
  reclaimable: string[];
  forgettable: string[];
} {
  const terminal = jobs.filter((job) => JOB_LIFECYCLE[job.status]?.terminal === true);
  return {
    // `Clear` only reaches a Job that has not already given its disk back, so
    // a second press is never a press that does nothing.
    reclaimable: terminal.filter((job) => job.reclaimed_at === undefined).map((job) => job.id),
    // `Delete record` reaches every terminal Job, reclaimed or not: deleting
    // the record a reclaim kept is the whole of what the act is for.
    forgettable: terminal.map((job) => job.id),
  };
}

export function BoardActions({
  jobs,
  live,
  refreshing,
  onCompose,
  onRefresh,
  onReadReports,
  onReadWorktrees,
  onClearTerminal,
  onForgetTerminal,
}: {
  /** Every Job Bridge holds, not just the rows drawn: clearing is Fleet's board. */
  jobs: readonly JobSummary[];
  live: boolean;
  refreshing: boolean;
  onCompose: () => void;
  onRefresh: () => void;
  onReadReports: () => void;
  onReadWorktrees: () => void;
  onClearTerminal: (jobIds: readonly string[]) => void;
  onForgetTerminal: (jobIds: readonly string[]) => void;
}) {
  const [asking, setAsking] = useState<"clear" | "forget" | null>(null);
  const { reclaimable, forgettable } = terminalOf(jobs);
  const clearNoun = reclaimable.length === 1 ? "job" : "jobs";
  const forgetNoun = forgettable.length === 1 ? "job's" : "jobs'";

  const items: SplitButtonItem[] = [
    // Re-reads over the connection Bridge already holds. It does not
    // reconnect: the runtime-file path already retries on its own.
    { label: refreshing ? "Refreshing" : "Refresh", onSelect: onRefresh },
    { label: "Reported", onSelect: onReadReports },
    { label: "Held disk", onSelect: onReadWorktrees },
    ...(reclaimable.length === 0
      ? []
      : [{ label: `Clear ${reclaimable.length} finished ${clearNoun}`, onSelect: () => setAsking("clear") }]),
    ...(forgettable.length === 0
      ? []
      : [
          {
            label: `Delete ${forgettable.length} ${forgetNoun} records`,
            danger: true,
            onSelect: () => setAsking("forget"),
          },
        ]),
  ];

  return (
    <>
      <SplitButton
        variant="primary"
        items={items}
        onAction={onCompose}
        disabled={!live}
        menuLabel="Everything else on the Board"
      >
        New job
      </SplitButton>
      <Dialog
        open={asking === "clear"}
        tone="destructive"
        title={`Clear ${reclaimable.length} finished ${clearNoun}?`}
        confirmLabel="Clear"
        onCancel={() => setAsking(null)}
        onConfirm={() => {
          setAsking(null);
          onClearTerminal(reclaimable);
        }}
      >
        <p>
          {`Every job that is done, failed, killed, rejected or superseded, ${reclaimable.length} right now, has `}
          its worktree and branch given back. The job and everything it recorded, its log, its
          checks and its judgments, stay on the board under Cleared.
        </p>
        <p>
          A branch holding commits the base cannot reach is left standing rather than deleted,
          and you are told which ones.
        </p>
      </Dialog>
      <Dialog
        open={asking === "forget"}
        tone="destructive"
        title={`Delete ${forgettable.length} finished ${forgetNoun} records?`}
        confirmLabel="Delete"
        onCancel={() => setAsking(null)}
        onConfirm={() => {
          setAsking(null);
          onForgetTerminal(forgettable);
        }}
      >
        <p>
          {`Every job that is done, failed, killed, rejected or superseded, ${forgettable.length} right now, is `}
          removed from the board along with its whole record. There is no undo, and a deleted job
          cannot be opened again.
        </p>
        <p>Its worktree and branch are left as its drone left them, or as a reclaim already left them.</p>
      </Dialog>
    </>
  );
}
