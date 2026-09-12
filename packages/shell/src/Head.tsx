// What the panel head says, and what it offers.
//
// Split out of `App.tsx` when that file grew past the gate's 500-line
// warning, the same way `Acts.tsx` and `copy.ts` came out of `JobDetail.tsx`.
// One subject — which view is up, what it is called, the one control that
// leaves it — and the window's state machine beside it is another.
//
// **`Back to the list` and `Cancel` live here, not in the body** — a control
// that leaves a view belongs beside the view's name.
//
// **No count sentence** — the Board drew one beside its filter until 11 Sep
// 2026, cut as prose the tab counts already say.
//
// **The Board's head is one control**, `New job`; everything else is in its
// menu. `BoardActions` says why.

import type { ReactNode } from "react";
import { Button, Kbd } from "@armada/components";
import type { JobSummary } from "@armada/protocol";
import { BoardActions } from "./BoardActions";

/** The views one head serves, and everything each needs to draw it. */
export type HeadProps = {
  /** One Job, read whole. Leaves to the list. */
  reading: boolean;
  /** The composer. Leaves to the list. */
  composing: boolean;
  /**
   * What has been reported against the Judge, and the counts over it. Leaves
   * to the list.
   *
   * **Its own view rather than a panel on a Job.** A report is filed about one
   * Job and the rate is read across all of them, so a listing reached through a
   * Job would lose exactly the reports that most need reading together.
   */
  auditing: boolean;
  /**
   * What Fleet is holding disk for. Leaves to the list.
   *
   * **Assembled in `App.tsx` until 2026-09-03**, with a note there saying it
   * was the same shape as the other four and belonged in this file. It is the
   * same shape, and this is that file.
   */
  clearing: boolean;
  /** A live connection. What stops a new Job being proposed into nothing. */
  live: boolean;
  /** A re-read in flight, so a second press does not send a second one. */
  refreshing: boolean;
  onCloseComposer: () => void;
  onCompose: () => void;
  onCloseReports: () => void;
  onReadReports: () => void;
  onCloseWorktrees: () => void;
  onReadWorktrees: () => void;
  onRefresh: () => void;
  /** Every Job Bridge holds, for the counts on the Board's two bulk acts. */
  jobs: readonly JobSummary[];
  onClearTerminal: (jobIds: readonly string[]) => void;
  onForgetTerminal: (jobIds: readonly string[]) => void;
};

/** What the head is called, what it says beneath, and what sits at its edge. */
export type Head = { title: string; summary?: string; actions: ReactNode };

export function headOf({
  reading,
  composing,
  auditing,
  clearing,
  live,
  refreshing,
  onCloseComposer,
  onCompose,
  onCloseReports,
  onReadReports,
  onCloseWorktrees,
  onReadWorktrees,
  onRefresh,
  jobs,
  onClearTerminal,
  onForgetTerminal,
}: HeadProps): Head | null {
  if (clearing) {
    return {
      title: "Held worktrees",
      // No `Esc` hint. The key is bound while a Job is open and nowhere else,
      // and a hint for a key that does nothing is worse than no hint.
      actions: (
        <Button variant="ghost" size="sm" onClick={onCloseWorktrees}>
          Back to the list
        </Button>
      ),
    };
  }
  if (auditing) {
    return {
      title: "Reported in error",
      // No summary. The counts are the page's own first region and repeating
      // one of them here would be a second place to keep them right.
      actions: (
        <>
          <Button variant="ghost" size="sm" onClick={onCloseReports}>
            Back to the list
          </Button>
          <Kbd>Esc</Kbd>
        </>
      ),
    };
  }
  // **A Job read whole gets no head, and that is the one view where none is
  // right.** The others are named by it: `Held worktrees`, `Reported in error`,
  // `New job` are pages whose name is not written anywhere else on them. A Job
  // is not — its own header carries the badge, the title, the id and the run's
  // figures, and it is the thing the reader is looking at. A bar above that
  // saying `Job Board` names the surface behind this one, which is the last
  // thing a person reading one Job needs, and it spends the top of the window
  // to say it.
  //
  // **`Back to the list` goes with it and nothing is stranded.** `Esc` is bound
  // while a Job is open, the rail's Job Board row is lit and returns on a
  // click, and the Job's own header is where the acts on the Job already live.
  // What is left is the header a person wants pinned at the top, at the top.
  if (reading) return null;
  if (composing) {
    return {
      title: "New job",
      summary: "It lands at the approval gate. Nothing runs until you release it.",
      actions: (
        <Button variant="ghost" size="sm" onClick={onCloseComposer}>
          Cancel
        </Button>
      ),
    };
  }
  return {
    title: "Job Board",
    actions: (
      <BoardActions
        jobs={jobs}
        live={live}
        refreshing={refreshing}
        onCompose={onCompose}
        onRefresh={onRefresh}
        onReadReports={onReadReports}
        onReadWorktrees={onReadWorktrees}
        onClearTerminal={onClearTerminal}
        onForgetTerminal={onForgetTerminal}
      />
    ),
  };
}
