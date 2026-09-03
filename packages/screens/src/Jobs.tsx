// The Job Board: every Job, what it is called, its state, the step it is on,
// the one decision a person can make from here — and the controls that narrow
// it.
//
// **It said it was not the Job Board.** It was written before the Board had a
// journey of its own, and the sentence held while the surface was a bare list.
// It is the Board now: the filter set is here, the sort is here, and the
// contract's contextual keys are answered here.
//
// # The controls, and the two axes there are
//
// **State, plus one text match.** The Board is already scoped to one Manifest,
// so scope is not a control; origin was drawn as a filter and rejected, because
// *what needs me*, *what is running* and *why has that not started* are all
// state. `board.ts` owns the arithmetic — which tab a Job is in, what a search
// matches, what order the two sorts produce, and the sentence over the top.
//
// **The count sentence moved here from the panel head.** It states both numbers
// — `4 jobs need you. 15 on the Board.` — and both change with the filter, so
// it belongs beside the control that changes them rather than in a head that
// also serves the composer and the reports view. The head kept it while there
// was no filter for it to disagree with.
//
// # The keyboard
//
// `keys.ts` owns the map, read off `docs/contracts/design-system.md`. Nothing
// here decides a binding; this file decides what each press reaches. Two things
// worth knowing at this level:
//
// **The listener is on `window` and the cursor is DOM focus.** `j` and `k` move
// focus; `Active jobs list` already roves on the arrows and follows focus, so
// both paths land on one cursor rather than two that drift.
//
// **`x` reaches the same confirmation the detail's kill reaches.** It returns a
// press and never a kill — `App` owns the dialog, which owns the rule that
// Cancel holds initial focus.
//
//
// # One state the row shape cannot draw, said out loud
//
// `JobRowStacked` requires a `statusIcon`, "required on every state, from the
// icon registry" — and the registry has states with no glyph. `escalated`
// carries none, and six of the escalation reasons carry none either, so a Job
// Fleet actually produces (`interrupted` aside, which does have one) can reach
// a status this row physically cannot render.
//
// **No glyph is invented for it.** `octagon-alert` is reserved to `stalled`,
// `triangle-alert` is reserved to Doctor, and there is no registry row meaning
// "unspecified". So those Jobs are named beneath the list rather than drawn in
// a second row shape: a missing thing that renders as a finding is a finding,
// and one that is silently absent is a gap nobody sees. Reported against the
// composition, not worked around with a prop only this app would use.
//
// A row the store refused is a different thing and is not here. It is a failure
// with a fault and a log, so `App` draws it as a failure notice — a Job that
// will not render and a Job that will not load are told apart.
//
// # A redispatch chain is one piece of work and gets one row
//
// Five goes at one ask were five rows, four of them over. `lineage.ts` groups
// the board by `redispatched_from` and this draws what it returns: the live
// member of each chain, its title carrying which dispatch it is. The word is
// `dispatch` rather than `attempt` because `attempt` counts a step's runs
// inside one Job and would be a second counter under one name — that file owns
// the reasoning.
//
// **The earlier dispatches are one press away and nothing is dropped.** They
// carry the Judge verdicts and the worktrees the chain exists to keep. The
// press is beneath the frame rather than on a row: a listbox holds options and
// nothing else, and the row shape carries one secondary control which a queued
// replacement already spends on `Approve dispatch`.

import { ActiveJobsList, BoardControls, BoardEmptyState, Button } from "@armada/components";
import { useEffect, useMemo, useRef, useState } from "react";

import { JOB_LIFECYCLE } from "@armada/components";
import type { JobSummary } from "@armada/protocol";
import type { WorkflowSummary } from "@armada/protocol";
import {
  BOARD_SORTS,
  BOARD_TABS,
  countSentence,
  DEFAULT_SORT,
  emptiedBy,
  FIRST_TAB,
  inTab,
  matches,
  needsYou,
  sorted,
  tabOf,
  tabSuspended,
  type BoardSort,
  type BoardTab,
} from "./board";
import { ClearTerminalControl } from "@armada/shell";
import { boardPressOf, SEARCH_KEY, verbOf } from "./keys";
import type { BoardReach } from "./keys";
import { foldedNote, foldLineages, headlineOf } from "./lineage";
import { readingOf } from "./reading";
import { isTerminal, Row } from "./Row";

/**
 * How many rows are drawn. Nothing renders an unbounded list directly, and no
 * virtualization library has been chosen — so the list is bounded and says what
 * it left out, which is the honest half of the same rule.
 */
const DRAWN = 200;

/** What the fold sentence says once the fold has been undone. */
const EARLIER_SHOWN = "Every dispatch is drawn, including the ones that are over.";

export type JobsProps = {
  jobs: readonly JobSummary[];
  /** True while what is shown is not live. Every row reads as de-emphasised. */
  stale: boolean;
  /** Now, injected. Elapsed on a working Job runs to it, so it has to move. */
  now: number;
  /** What Fleet holds, so a row can say `bug` where it carried a ULID. */
  workflows: readonly WorkflowSummary[];
  /** The whole reading of the connection, for the empty state that is a fault. */
  disconnected: string | null;
  /** The Job whose detail is open, where one is. */
  selected: string | null;
  /** Open a Job. Every row is a control, so every row calls this. */
  onOpen: (jobId: string) => void;
  /**
   * Ask to kill the Job under the cursor — `x`, and the row's own control on a
   * job that has one. **It asks; it never kills.** The confirmation is `App`'s,
   * which is the dialog the detail's kill already goes through, and which owns
   * the rule that Cancel holds initial focus. A board with its own dialog would
   * be a second place that rule could be got wrong.
   */
  onKill: (jobId: string) => void;
  /**
   * Open the composer — `n`, the one key in the contextual tier that acts on
   * nothing on screen. The Board holds the binding because the Board is where
   * the cursor is; what it opens is `App`'s.
   */
  onCompose: () => void;
  /**
   * Clear every terminal Job at once. Confirmed here, before it is called —
   * there is no undo on the other side of this, and unlike a kill there is no
   * record left afterward to check the confirmation against.
   */
  onClearTerminal: (jobIds: readonly string[]) => void;
  /** A clipboard write is silent, so the surface confirms every one with a toast. */
  onCopied: (value: string) => void;
  /**
   * Where the cursor is, reported up. **The Board still owns it** and this
   * only mirrors it, from the same focus handler that sets the Board's own —
   * so the palette can title its context block with the job its acts would act
   * on, and no second cursor exists to drift.
   */
  onCursor?: (jobId: string | null) => void;
  /** Filled with what the palette can reach here. `keys.ts` says why. */
  reach?: { current: BoardReach | null };
};

export function Jobs({
  jobs,
  stale,
  now,
  workflows,
  disconnected,
  selected,
  onOpen,
  onKill,
  onCompose,
  onClearTerminal,
  onCopied,
  onCursor,
  reach,
}: JobsProps) {
  // Folded once for the whole board rather than per row. The dependency is the
  // array Bridge published, which is replaced on every event and never mutated,
  // so identity is the right key.
  const board = useMemo(() => foldLineages(jobs), [jobs]);
  // Folded by default, and the surface returns to folded when it is left: the
  // list is a scanning surface, and an expansion is a question asked of one
  // moment rather than a preference.
  const [unfolded, setUnfolded] = useState(false);
  // The filter set. Not lifted into `App`: it is what this surface is for, and
  // nothing above the list reads it now that the count sentence is here.
  const [tab, setTab] = useState<BoardTab>(FIRST_TAB);
  const [query, setQuery] = useState("");
  const [sort, setSort] = useState<BoardSort>(DEFAULT_SORT);
  // Where the cursor is, as a job id. It is set from DOM focus rather than kept
  // beside it, so `j`, the arrows, Tab and the mouse all move one cursor.
  const [cursor, setCursor] = useState<string | null>(null);
  const search = useRef<HTMLInputElement>(null);

  const showing = unfolded ? [...board.shown, ...board.folded] : board.shown;
  // The search first, and the tab second only where the search is not running.
  // **A text match suspends the state tab rather than changing it**: while the
  // field holds text every match is drawn, the tab keeps the value the person
  // chose, and clearing the field gives it straight back. The tab counts below
  // stay counts of `matched`, so a suspended strip is still a breakdown of what
  // is on screen.
  const suspended = tabSuspended(query);
  const matched = showing.filter((job) => matches(job, query, workflows));
  const narrowed = suspended ? matched : matched.filter((job) => inTab(job, tab));
  const bounded = sorted(narrowed, sort).slice(0, DRAWN);
  const drawn = bounded.filter((job) => readingOf(job).as === "badge");
  const undrawable = bounded.filter((job) => readingOf(job).as !== "badge");
  // Every Job Bridge holds, not just the bounded window drawn below — a Job
  // scrolled past `DRAWN` is still cleared, since the point is clearing the
  // board Fleet holds rather than the rows currently on screen.
  const terminalIds = jobs
    .filter((job) => JOB_LIFECYCLE[job.status]?.terminal === true)
    .map((job) => job.id);

  const tabs = BOARD_TABS.map((row) => ({
    id: row.id,
    label: row.label,
    shortcut: row.shortcut,
    // Zero renders as no count, never as `0`: an empty queue is the resting
    // state of a healthy fleet, and a row of zeros trains the eye to skip the
    // number. `TabsWithCounts` applies that; this only has to not lie.
    count: row.id === "all" ? matched.length : matched.filter((job) => tabOf(job) === row.id).length,
  }));

  /**
   * Choose a state tab, and the search is cleared.
   *
   * **The tab is suspended while a search runs, so pressing one has to end the
   * search or do nothing at all** — and a control that does nothing when pressed
   * is the worse of the two. Pressing a tab is asking for a state rather than
   * for a match, so the match is what gives way, and it gives way in the
   * direction that has an undo: retyping a query is a keystroke, while
   * recovering a tab you did not know you had lost is not.
   *
   * The same function serves `1`–`5` and a click, because they are one act.
   */
  function chooseTab(next: BoardTab): void {
    setTab(next);
    setQuery("");
  }

  /** `/`, and the palette's Search row. Named because two callers press it. */
  function focusSearch(): void {
    search.current?.focus();
    search.current?.select();
  }

  /** The rows, as the DOM has them — the only place their drawn order is. */
  function rowsOnScreen(): HTMLElement[] {
    return Array.from(document.querySelectorAll<HTMLElement>("[data-job-id]"));
  }

  /**
   * Move the cursor. Focus is what moves; `cursor` follows it through the
   * wrapper's own focus handler, so this never has to keep two things in step.
   *
   * Clamped rather than wrapped, which is `Active jobs list`'s rule for the
   * arrows and is the same rule here: a list that jumps from the last row to
   * the first loses the reader's place, and a Board is scanned rather than
   * cycled.
   */
  function move(by: 1 | -1): void {
    const rows = rowsOnScreen();
    if (rows.length === 0) return;
    const at = rows.findIndex((row) => row.dataset.jobId === cursor);
    const to = at < 0 ? (by === 1 ? 0 : rows.length - 1) : Math.min(Math.max(at + by, 0), rows.length - 1);
    rows[to]?.focus();
  }

  /**
   * Put the cursor back on the list without moving it — what `Esc` in the
   * search field hands back to. The row it was on where that row is still
   * drawn, and the first row where the search took it off screen.
   */
  function restoreCursor(): void {
    const rows = rowsOnScreen();
    const at = rows.findIndex((row) => row.dataset.jobId === cursor);
    (at >= 0 ? rows[at] : rows[0])?.focus();
  }

  /** The job the cursor is on, where the cursor is on one that is drawn. */
  function under(): JobSummary | undefined {
    return drawn.find((job) => job.id === cursor);
  }

  function press(event: KeyboardEvent): void {
    // **A modal is up, so every key belongs to it.** The confirmation this
    // surface opens is `App`'s and the bulk clear's is its own, and neither is
    // inside this component — so focus is not what tells them apart, and a
    // window listener would otherwise open a detail behind the dialog a person
    // is answering. Read off the document rather than passed in, because the
    // dialogs are on both sides of this file and a flag would have to be
    // threaded through both.
    if (document.querySelector('[role="dialog"], [role="alertdialog"]') !== null) return;
    const read = boardPressOf(event);
    if (read === null) return;
    const job = under();
    switch (read.act) {
      case "search":
        focusSearch();
        break;
      case "move":
        move(read.by);
        break;
      case "open":
        if (job === undefined) return;
        onOpen(job.id);
        break;
      case "verb":
        // The row carries one control, so at most one verb key applies. The
        // rest no-op rather than acting on the wrong verb — which is what makes
        // a rehearsed keystroke safe when the list reorders under you.
        if (job === undefined) return;
        if (verbOf(job, isTerminal(job)) !== read.verb) return;
        onOpen(job.id);
        break;
      case "kill":
        // A terminal job has nothing to kill, so the key reaches nothing there
        // rather than opening a dialog about a job that is already over.
        if (job === undefined || isTerminal(job)) return;
        onKill(job.id);
        break;
      case "copy":
        // **`c` reaches nothing on a Job row, and that is the answer rather
        // than a gap.** `copyDebugInfoFor` in `FailureSurface.tsx` is the act,
        // and it takes a `Failure` — a healthy Job has none. The payload opens
        // `armada error` with a required `message`, so a job-identity payload
        // would be a different artifact under one key. It follows the same rule
        // the verb keys follow: a key that does not apply no-ops rather than
        // acting on the wrong verb. The binding is claimed here so the key is
        // where the map says it is; the press does nothing and is not
        // swallowed.
        return;
      case "tab":
        chooseTab(read.tab);
        break;
      case "compose":
        onCompose();
        break;
    }
    // Only a press this surface answered is swallowed. A key it returned `null`
    // for reaches whatever else is listening, which is how the global tier
    // keeps working while the cursor is on a row.
    event.preventDefault();
  }

  // The listener is registered once and reads the current handler out of a ref.
  // Re-registering per render is what the deps array would ask for, and `now`
  // moves once a second — so the whole board re-renders on a clock and the
  // subscription would churn with it for no reason.
  const latest = useRef(press);
  useEffect(() => {
    latest.current = press;
    if (reach !== undefined) reach.current = { tab: chooseTab, search: focusSearch };
  });
  useEffect(() => {
    const listen = (event: KeyboardEvent): void => latest.current(event);
    window.addEventListener("keydown", listen);
    return () => window.removeEventListener("keydown", listen);
  }, []);

  const why = emptiedBy(tab, query);

  return (
    <div
      className="flex flex-col gap-2"
      // The cursor is DOM focus. Capturing here rather than on the list means a
      // row reached by mouse, by Tab, by the listbox's own arrows or by `j` all
      // set the same value — two cursors that drift is the alternative.
      onFocusCapture={(event) => {
        const row = (event.target as HTMLElement).closest<HTMLElement>("[data-job-id]");
        if (row?.dataset.jobId === undefined) return;
        setCursor(row.dataset.jobId);
        onCursor?.(row.dataset.jobId);
      }}
    >
      {terminalIds.length === 0 ? null : (
        <div className="flex justify-end">
          <ClearTerminalControl count={terminalIds.length} stale={stale} onConfirm={() => onClearTerminal(terminalIds)} />
        </div>
      )}
      <ActiveJobsList
        // The count sentence is here rather than in the panel head, because both
        // its numbers move with the filter directly below it and the head serves
        // three other views. No heading: the head still names the surface, and
        // the same name in two places is two chances to disagree.
        //
        // Every drawn row opens a Job, so the frame is a listbox and its rows
        // are options — which is what lets "this one is open" be a state a
        // screen reader can read rather than a fill only a sighted eye catches.
        summary={countSentence({
          total: jobs.length,
          matched: matched.length,
          needsYou: narrowed.filter(needsYou).length,
          query,
        })}
        selectable
        label="Job Board"
        controls={
          <BoardControls
            query={query}
            onQuery={setQuery}
            searchRef={search}
            searchKey={SEARCH_KEY}
            onLeaveSearch={restoreCursor}
            sorts={BOARD_SORTS}
            sort={sort}
            onSort={(next) => setSort(next as BoardSort)}
            tabs={tabs}
            tab={tab}
            onTab={(next) => chooseTab(next as BoardTab)}
            suspended={suspended}
          />
        }
        empty={
          disconnected !== null ? (
            // The three empty states differ because the three situations do: a
            // Fleet that is up with no work is a null result, one that is not
            // running is a fault Bridge cannot fix, and a filter that emptied
            // the list is neither — it is a control saying so.
            <BoardEmptyState command="armada fleet start" note={disconnected}>
              Fleet is not connected, so there is nothing to show.
            </BoardEmptyState>
          ) : why !== null ? (
            // **The control offered is the one that emptied the list.** Only
            // one of the two can have, because a search suspends the tab — so
            // clearing the search restores whatever tab was set rather than
            // discarding it, which is the whole reason suspending won over
            // resetting. A single "Show every job" here would have spent the
            // filter at the last moment it could have been kept.
            <BoardEmptyState
              quiet
              action={
                <Button
                  variant="ghost"
                  size="sm"
                  onClick={() => (suspended ? setQuery("") : setTab("all"))}
                >
                  {suspended ? "Clear the search" : "Show every job"}
                </Button>
              }
            >
              {why}
            </BoardEmptyState>
          ) : (
            <BoardEmptyState quiet>No jobs. Propose one above.</BoardEmptyState>
          )
        }
      >
        {drawn.map((job) => (
          <Row
            key={job.id}
            job={job}
            headline={headlineOf(job, board.dispatch.get(job.id))}
            stale={stale}
            now={now}
            workflows={workflows}
            selected={job.id === selected}
            focused={job.id === cursor}
            onOpen={onOpen}
            onKill={onKill}
            onCopied={onCopied}
          />
        ))}
      </ActiveJobsList>

      {/* Named whether or not it is pressed. A fold nobody is told about is
          rows that vanished. */}
      {board.folded.length === 0 ? null : (
        <p className="text-fg-muted">
          {unfolded ? EARLIER_SHOWN : foldedNote(board.folded.length)}{" "}
          <Button variant="ghost" size="sm" onClick={() => setUnfolded(!unfolded)}>
            {unfolded ? "Fold them again" : "Show them"}
          </Button>
        </p>
      )}

      {/* Counted against what the filters left, not against the board: the
          window bounds what is drawn, and what is drawn is what the controls
          admitted. Saying "200 of 900" under a filter holding 210 would name a
          number no control on screen produced. */}
      {narrowed.length > bounded.length ? (
        <p className="text-fg-muted">
          {`${bounded.length} of ${narrowed.length} rows drawn. The rest are on Fleet and not on screen.`}
        </p>
      ) : null}

      {/* The registry has no glyph for this state, so the row shape cannot draw
          it and nothing here invents one. Named rather than dropped. */}
      {undrawable.map((job) => {
        const reading = readingOf(job);
        if (reading.as === "badge") return null;
        return (
          <p key={job.id} className="text-fg-muted">
            {`${headlineOf(job, board.dispatch.get(job.id))} — `}
            <span className="mono">{reading.wire}</span>
            {`. The registry carries no ${reading.missing.join(" and no ")} for it, so the row shape cannot draw it.`}
          </p>
        );
      })}
    </div>
  );
}
