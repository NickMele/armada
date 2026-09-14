// What the panel head says, and what it offers.
//
// Split out of `App.tsx` when that file grew past the gate's 500-line warning,
// the same way `Acts.tsx` and `copy.ts` came out of `JobDetail.tsx`. It is one
// subject — which of the views is up, what it is called, and the one control
// that leaves it — and the window's state machine beside it is another.
//
// **`Back to the list` and `Cancel` live here rather than in the body.** A
// control that leaves a view belongs beside the view's name, not scrolled into
// it.
//
// **There is no count sentence.** The Board drew one beside its filter until
// 11 Sep 2026, when the owner cut it as prose the tab counts already say.
//
// **The Board's head is one control.** `Dispatch` (renamed from `New job`
// by #1087), with everything else the Board offers in its menu. `BoardActions`
// says why.

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
  /**
   * The Manifest surface, and which of its views is showing — Journey 9's
   * *Running one*, or the file half of *Editing*. `false` is any other surface.
   *
   * **The view and not a flag beside it**, so a head cannot describe a view
   * the surface is not showing.
   *
   * **A name and no way out, and the missing control is the point.** Manifest
   * is a rail destination: a person who pressed `⌘5` did not come from the
   * Board, so *Back to the list* would name a place they never were — and the
   * rail they would actually leave by is already on screen beside it. The head
   * is here only because the page's own rows never say what surface they
   * belong to.
   */
  manifest: false | "run" | "form" | "file";
  /**
   * Overview, the same shape as `manifest`'s own reasoning: a rail destination
   * with no way out, because the rail beside it is the way out. Where Bridge
   * opens (#921), so it gets no *New job* either — Overview reads, and the
   * Board is where a person acts.
   *
   * **Optional, so a caller that never shows Overview does not have to say
   * so.** Every existing story head was built before Overview existed.
   */
  overviewing?: boolean;
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
  /** Opens Fleet settings. Board's own menu row; the sheet itself is the
   *  App's, since it opens over any screen, not just the Board's. */
  onOpenLimits: () => void;
  onRefresh: () => void;
  /** Every Job Bridge holds, for the counts on the Board's two bulk acts. */
  jobs: readonly JobSummary[];
  onClearTerminal: (jobIds: readonly string[]) => void;
  onForgetTerminal: (jobIds: readonly string[]) => void;
  /** Which bulk sweep is out, so its own control waits and the other is off. #1117. */
  sweeping: "clear" | "forget" | null;
};

/** What the head is called, what it says beneath, and what sits at its edge. */
export type Head = { title: string; summary?: string; actions: ReactNode };

export function headOf({
  reading,
  composing,
  auditing,
  clearing,
  manifest,
  overviewing,
  live,
  refreshing,
  onCloseComposer,
  onCompose,
  onCloseReports,
  onReadReports,
  onCloseWorktrees,
  onReadWorktrees,
  onOpenLimits,
  onRefresh,
  jobs,
  onClearTerminal,
  onForgetTerminal,
  sweeping,
}: HeadProps): Head | null {
  if (manifest !== false) {
    return {
      title: "Manifest",
      // What the view is *for*, and the one thing about it that surprises
      // people, said once here rather than repeated on every row.
      //
      // Running: a run goes into the tree they are working in, and leaves no
      // verdict behind for any Job. Editing: a save goes to disk and no
      // further — the file is tracked, and Armada committing on a person's
      // behalf is the surprise Journey 9 rules out.
      summary:
        manifest !== "run"
          ? "Edit this repository's Manifest. Save writes the file to disk and stops, without staging or committing it."
          : "Run one Check or Command against this checkout, as it is on disk. Nothing here is a verdict.",
      // No action. The rail is how a person leaves a rail destination.
      actions: null,
    };
  }
  if (clearing) {
    return {
      title: "Cleanup",
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
  // right.** The others are named by it: `Cleanup`, `Reported in error`,
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
      title: "Dispatch",
      summary: "It lands at the approval gate. Nothing runs until you release it.",
      actions: (
        <Button variant="ghost" size="sm" onClick={onCloseComposer}>
          Cancel
        </Button>
      ),
    };
  }
  // Checked after the composer: the palette's `Dispatch` does not leave
  // Overview, so both can be true at once and the composer is what is on
  // screen.
  if (overviewing) return { title: "Overview", actions: null };
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
        onOpenLimits={onOpenLimits}
        onClearTerminal={onClearTerminal}
        onForgetTerminal={onForgetTerminal}
        sweeping={sweeping}
      />
    ),
  };
}
