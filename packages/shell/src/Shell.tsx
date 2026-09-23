// Bridge's frame: the rail, the panel the journeys mount in, and the status
// bar. The drawn shell, wired to what Fleet actually serves.
//
// # The roster is what exists
//
// The rail carries the Job Board and the held worktrees, which are the
// surfaces built. Alerts, Doctor, Manifest and Helm are named in the concept
// page and none of them is built — a disabled row would be a promise Armada
// does not keep, so each holds its place in the order and its digit without
// drawing anything. `surfaces.ts` is where that order lives.
//
// Active Jobs, Reviews and the Activity Feed used to be named here too. They
// are not surfaces any more: the Board holds every Job with state as a filter,
// and each of the three is that list under one filter. See
// `docs/concepts/bridge.md`.
//
// # One region of the drawing has no source, and is left out
//
// `today ~$4.80`: nothing measures spend, and it is not drawn as a labelled
// blank — a label with nothing under it reads as a value that failed to load.
//
// `1 drone` used to be here for the same reason: `assigned_drone` has no event
// that sets it, so no count could be taken from the board. `get_capacity`
// answers it now, from the roster admission itself counts against, so the
// contract's `pid, port, drone count` is finally all three.
//
// # The status bar is gone; Fleet's state lives in the left column
//
// Bridge/1088 replaced the rail and the status bar with three rounded
// panels — Navigation, Stats and Fleet. **Their rows arrive built**, from
// `apps/desktop`'s `left-column.ts`, which reads the same arithmetic
// Overview's own tiles do. Not built here: `@armada/screens` already depends
// on this package for `statementOf`, and the reverse import would be a cycle.
//
// **And under `--layout-breakpoint` all three collapse to the 48px rail.**
// Navigation keeps its glyphs, Stats and Fleet keep one status dot each, and
// nothing leaves the screen. #1435 took the column away outright and the owner
// corrected it on 18 Sep 2026. There was a second, wider band it collapsed in
// to pay for Helm's dock; #1583 put the dock on a layer and the band went with
// the bill.
//
// # The picker and Dispatch live in the title row, not here
//
// #1087 moved the repository picker out of the rail and put a second
// `Dispatch` beside it, in the row `TheShell` draws over the traffic lights.
// This file still renders the picker's markup — it is Bridge's own reading of
// what Fleet serves — but hands it to `TheShell` as `repositoryPicker` rather
// than `railHeader`.
//
// # No screen draws a page head, #1090
//
// Every screen used to spend a header row naming the surface the rail already
// says — `headOf` built it and this file carried it to `TheShell` as `title`,
// `summary` and `actions`. All three are gone: the rail is where a person
// reads where they are, and a screen's own controls — Board's menu, the way
// out of the composer, the reports and the held worktrees — sit at the top of
// each screen's own content instead. `App.tsx` is where that moved.

import { useEffect, useRef, useState, type ReactNode } from "react";
import {
  ACTION,
  JOB_LIFECYCLE,
  DockQuestions,
  DropdownMenu,
  TheShell,
  type DockQuestion,
  type DropdownMenuEntry,
  type FleetPanelProps,
  type StatsPanelProps,
} from "@armada/components";

import type { Connection } from "@armada/protocol";
import type { JobSummary } from "@armada/protocol";
import type { RepositorySummary } from "@armada/protocol";
import { useDockWidth } from "./dock-width";
// The rail collapses below `--layout-breakpoint`, and job detail's inspector
// folds to a sheet at the same bound. One reader, in `floor.ts`, so the two
// cannot answer a resize a pixel apart.
import { useNarrow } from "./floor";
import { useLeftWidth } from "./left-width";
import { ALL_REPOSITORIES } from "./RepositoryOptions";
import { repositoryLabel } from "./repository-label";
import { SURFACE, SURFACES } from "./surfaces";

export type ShellProps = {
  connection: Connection;
  /** Every repository Fleet serves, set up or not. One entry is why the drawing shows the control. */
  repositories: readonly RepositorySummary[];
  /** Whether Fleet has answered the listing, so an empty one says nothing is set up rather than not read. */
  listed?: boolean;
  /** The picked repository's root, or `null` for All repositories, which is where Bridge opens. */
  scope: string | null;
  /** A root, or `null` for All repositories. */
  onScope: (root: string | null) => void;
  /** Opens Locate. Absent draws no control. */
  onAddRepository?: () => void;
  /** The Board's Jobs, for the rail's count. They follow the pick. */
  boardJobs: readonly JobSummary[];
  /**
   * The left column's Stats panel, built by the caller from the same
   * arithmetic Overview's own tiles read — `apps/desktop`'s `left-column.ts`.
   */
  stats: Omit<StatsPanelProps, "narrow">;
  /** The left column's Fleet panel — what the status bar used to draw. */
  fleet: Omit<FleetPanelProps, "narrow">;
  /** Opens the composer. The title row's own Dispatch control — #1087 — beside the Board's own. */
  onCompose: () => void;
  /** Opens the command palette from the title row's search field. */
  onSearch: () => void;
  /** Which surface is up, by its id in `surfaces.ts`. The rail marks it. */
  showing: string;
  /** Selecting a rail row goes to that surface, from wherever you are. The
   *  rail is the one thing present on every view, so it is where a person
   *  looks to get back — Escape and Cancel both work and neither is what they
   *  reach for. **It takes the id**: with more than one row, a handler that
   *  ignored which was pressed would land on the wrong screen silently. */
  onSurface?: (surfaceId: string) => void;
  /** Every question waiting on a person, from every repository, as the dock's cards. Oldest first. */
  questions?: readonly DockQuestion[];
  /**
   * How many Helm asks are waiting inside `helm` rather than in the block
   * above it — #1519. The folded strip's count is every question waiting on a
   * person, and an ask drawn down in the conversation is still one of them;
   * without this the strip would say nothing was waiting while a reply sat
   * held open.
   */
  asking?: number;
  /** Helm's own conversation, under the questions — #944. Bridge builds it; the dock only mounts it. */
  helm?: ReactNode;
  /**
   * Helm's own act in the dock's head, beside Close — *Start fresh*, moved
   * there by the owner on 18 Sep 2026. Bridge builds it, for the same reason
   * `helm` is a node: what the act costs and when Fleet refuses it are the
   * app's to know, not the shell's.
   */
  helmAction?: ReactNode;
  children: ReactNode;
};

export function Shell({
  connection,
  repositories,
  listed = false,
  scope,
  onScope,
  onAddRepository,
  boardJobs,
  stats,
  fleet,
  onCompose,
  onSearch,
  showing,
  onSurface,
  questions = [],
  asking = 0,
  helm,
  helmAction,
  children,
}: ShellProps) {
  const narrow = useNarrow();
  const dock = useDock(narrow);
  // One band, the window's own. It was two until #1583, the second being
  // `--window-fold-left` *and* the dock beside the content — pixels the column
  // gave up so the dock could hold its 380. The dock is a layer over the work
  // now and asks for none, so opening Helm cannot move the column under it.
  const collapsed = narrow;
  const [dockWidth, resizeDock] = useDockWidth();
  const [leftWidth, resizeLeft] = useLeftWidth();
  const live = connection.state === "connected";

  return (
    <TheShell
      dock={{
        ...dock,
        // The breakpoint alone. The dock folds to its sheet where the title
        // row has no room for its button, which is not the same question as
        // whether the left column is at its rail.
        folded: narrow,
        binding: HELM_KEY,
        questions: questions.length + asking,
        width: dockWidth,
        onResize: resizeDock,
        action: helmAction,
        children: (
          <>
            {/* A Drone's question and a Judge's belong to a Job and have no
                conversation to sit in, so the dock is the only surface they
                have. A Helm ask has one, and is drawn at the end of it
                instead — #1519. `asking` is how it still reaches the count. */}
            {questions.length === 0 ? null : <DockQuestions questions={questions} />}
            {helm}
          </>
        ),
      }}
      surfaces={SURFACES.map((surface) => ({
        id: surface.id,
        label: surface.label,
        icon: surface.icon,
        shortcut: surface.shortcut,
        // Only the Board carries one. What Fleet is holding disk for is read
        // while that screen is open and not before, so a number here would be
        // right for as long as somebody was looking at it and stale after —
        // and a count nobody can trust is worse than a row with none.
        //
        // **Active Jobs only.** Finished and cleared ones are the Board's
        // record rather than its work, and the owner ruled on 11 Sep 2026
        // that the rail counts the work. Zero draws no count, as a tab's does.
        ...(surface.id === SURFACE.board && activeOf(boardJobs) > 0
          ? { count: activeOf(boardJobs) }
          : {}),
      }))}
      activeId={showing}
      onSelect={onSurface}
      collapsed={collapsed}
      leftWidth={leftWidth}
      onResizeLeft={resizeLeft}
      repositoryPicker={
        <DropdownMenu
          triggerLabel={repositoryTriggerLabel(repositories, scope, listed)}
          entries={repositoryEntries(repositories, listed, onAddRepository !== undefined, scope)}
          // Disabled only where nothing behind it is actionable — Add a
          // repository stays reachable on an empty Fleet, which is when it
          // matters most, so its presence keeps the trigger live.
          disabled={repositories.length === 0 && onAddRepository === undefined}
          onSelect={(id) => (id === ADD_REPOSITORY ? onAddRepository?.() : onScope(id === ALL_REPOSITORIES ? null : id))}
        />
      }
      onSearch={onSearch}
      onDispatch={onCompose}
      dispatchDisabled={!live}
      stats={stats}
      fleet={fleet}
    >
      {children}
    </TheShell>
  );
}

/** No root is ever this, so it cannot collide with one. */
const ADD_REPOSITORY = "add-repository";

/** The trigger's own label: the picked repository, All, or why there is nothing to pick yet. */
function repositoryTriggerLabel(
  repositories: readonly RepositorySummary[],
  scope: string | null,
  listed: boolean,
): string {
  if (repositories.length === 0) return listed ? "Nothing set up yet" : "No repository read yet";
  if (scope === null) return "All repositories";
  const repository = repositories.find((one) => one.root === scope);
  return repository === undefined ? scope : repositoryLabel(repository, repositories);
}

/**
 * The picker's menu: All first, set up repositories, then a `Not set up`
 * group — `RepositoryOptions`'s own order, read here instead of rendered
 * there since a `<select>` groups with `optgroup` and a menu with a `label`
 * entry. Add a repository sits last, below a separator: not destructive, but
 * a different kind of row from the list above it, which is the separator's
 * established use here. All is a reset, so it gets the same separator above
 * the specific choices, and whichever entry matches `scope` carries the
 * checkmark.
 */
function repositoryEntries(
  repositories: readonly RepositorySummary[],
  listed: boolean,
  hasAdd: boolean,
  scope: string | null,
): DropdownMenuEntry[] {
  const setUp = repositories.filter((one) => one.manifest !== undefined);
  const loose = repositories.filter((one) => one.manifest === undefined);
  const item = (repo: RepositorySummary) => ({
    kind: "item" as const,
    id: repo.root,
    label: repositoryLabel(repo, repositories),
    selected: repo.root === scope,
  });
  const entries: DropdownMenuEntry[] = [
    {
      kind: "item",
      id: ALL_REPOSITORIES,
      label: repositories.length === 0
        ? (listed ? "Nothing set up yet" : "No repository read yet")
        : "All repositories",
      selected: scope === null,
    },
  ];
  if (setUp.length > 0) {
    entries.push({ kind: "separator", id: "all-repositories-rule" });
    entries.push(...setUp.map(item));
  }
  if (loose.length > 0) {
    entries.push({ kind: "label", id: "not-set-up", label: "Not set up" });
    entries.push(...loose.map(item));
  }
  if (hasAdd) {
    entries.push({ kind: "separator", id: "add-repository-rule" });
    entries.push({ kind: "item", id: ADD_REPOSITORY, label: "Add a repository" });
  }
  return entries;
}

/** Helm's binding, read from the registry rather than retyped. */
const HELM_KEY = ACTION.helm?.shortcut;

/**
 * Helm's dock: a panel over the content, or a sheet when folded. `⌘J` toggles
 * whichever the width draws, from every surface and from inside a field.
 *
 * **Shut on open, at every width** — the owner, 22 Sep 2026 (#1583). It stood
 * open because it cost the layout nothing to leave it there once a person had
 * arranged around it; over the content it covers what is being read, so Bridge
 * opens with the screen and Helm is a press away.
 */
function useDock(folded: boolean): { open: boolean; onOpen: (open: boolean) => void } {
  const [beside, setBeside] = useState(false);
  const [sheet, setSheet] = useState(false);

  // A sheet left open would cover the work again the next time the window narrows.
  useEffect(() => {
    if (!folded) setSheet(false);
  }, [folded]);

  const open = folded ? sheet : beside;
  const onOpen = folded ? setSheet : setBeside;
  const toggle = useRef(() => onOpen(!open));
  toggle.current = () => onOpen(!open);

  useEffect(() => {
    if (HELM_KEY === undefined) return undefined;
    function pressed(event: KeyboardEvent): void {
      if (!(event.metaKey || event.ctrlKey) || event.altKey || event.shiftKey || event.repeat) return;
      if (`⌘${event.key.toUpperCase()}` !== HELM_KEY) return;
      event.preventDefault();
      toggle.current();
    }
    window.addEventListener("keydown", pressed);
    return () => window.removeEventListener("keydown", pressed);
  }, []);

  return { open, onOpen };
}

/**
 * How many Jobs have not ended. A status the registry does not know counts,
 * since nothing says that Job is over.
 */
function activeOf(jobs: readonly JobSummary[]): number {
  return jobs.filter((job) => JOB_LIFECYCLE[job.status]?.terminal !== true).length;
}
