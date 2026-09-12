// Journey 9's run sheet — *Running one inside a Job* — built from what Fleet
// serves plus the run a window is following. Not `run.ts`, which is the run
// *tree*; this is the run *sheet*, a different noun the wire happens to share
// a stem with.
//
// **Ids are prefixed by group**, so a press can tell a Check from a Command
// from a server apart without a second lookup — `setup:`, `check:`,
// `command:`, `server:`. The Checks chapter's own rows carry a Check's bare
// name, so `checkEntryId` composes the same prefix on the way in and the two
// agree without either side knowing the other's scheme.

import { useEffect, useState } from "react";
import type {
  ConsoleOutputProps,
  RunSheetEntry,
  RunSheetGroup,
  RunSheetPastRun,
  RunSheetProps,
  RunSheetServerStatus,
} from "@armada/components";
import type {
  Followed,
  Outcome,
  RunEntry,
  RunFollowed,
  RunListRead,
  RunOutputRead,
  RunRecord,
  RunSheet,
  RunSheetRead,
  ServerEntry,
  ServerList,
  ServerState,
  StartRun,
} from "@armada/protocol";
import { absoluteOf, span } from "./duration";
import { openServerLink } from "./opening";

export const SETUP_PREFIX = "setup:";
export const CHECK_PREFIX = "check:";
export const COMMAND_PREFIX = "command:";
export const SERVER_PREFIX = "server:";

/** The id `checkRow` composes for **Run it here**, off the same Check name. */
export function checkEntryId(name: string): string {
  return `${CHECK_PREFIX}${name}`;
}

/** Whether an id names one of the entries `servers` composed. */
export function isServerEntry(id: string): boolean {
  return id.startsWith(SERVER_PREFIX);
}

/**
 * The instance Fleet holds for a server entry id, for `stop_server` and
 * `open_server_link` — **not the entry id itself**, which is this sheet's
 * own scheme and names nothing Fleet holds.
 */
export function serverInstanceIdOf(sheet: RunSheet | undefined, entryId: string | undefined): string | undefined {
  if (sheet === undefined || entryId === undefined || !isServerEntry(entryId)) return undefined;
  return (sheet.servers ?? []).find((one) => one.name === nameOf(entryId))?.instance?.id;
}

/** The bare Manifest name an entry id carries, whatever its group. */
export function nameOf(id: string): string {
  const at = id.indexOf(":");
  return at === -1 ? id : id.slice(at + 1);
}

/** The three groups the sheet lists, in the journey's order. */
export function runSheetGroupsOf(sheet: RunSheet, wide: ReadonlySet<string>): RunSheetGroup[] {
  return [
    { kind: "setup", label: "Setup", entries: sheet.setup.map((e) => entryOf(SETUP_PREFIX, e, wide)) },
    { kind: "checks", label: "Checks", entries: sheet.checks.map((e) => entryOf(CHECK_PREFIX, e, wide)) },
    {
      kind: "commands",
      label: "Commands",
      entries: [
        ...sheet.commands.map((e) => entryOf(COMMAND_PREFIX, e, wide)),
        ...(sheet.servers ?? []).map(serverEntryOf),
      ],
    },
  ];
}

function entryOf(prefix: string, entry: RunEntry, wide: ReadonlySet<string>): RunSheetEntry {
  const id = `${prefix}${entry.name}`;
  return {
    id,
    name: entry.name,
    run: entry.run,
    note: noteOf(entry.requires),
    ...(entry.narrow_run === undefined ? {} : { narrowRun: entry.narrow_run, narrowed: !wide.has(id) }),
  };
}

function serverEntryOf(entry: ServerEntry): RunSheetEntry {
  return {
    id: `${SERVER_PREFIX}${entry.name}`,
    name: entry.name,
    run: entry.serve,
    note: entry.run === undefined ? undefined : `Runs ${entry.run} first.`,
  };
}

function noteOf(requires: readonly string[]): string | undefined {
  return requires.length === 0 ? undefined : `Runs ${requires.join(", ")} first.`;
}

/** The selected entry's server state, or `undefined` where it names no server. */
export function serverStatusOf(
  sheet: RunSheet | undefined,
  selectedId: string | null,
  now: number,
): RunSheetServerStatus | undefined {
  if (sheet === undefined || selectedId === null || !isServerEntry(selectedId)) return undefined;
  const entry = (sheet.servers ?? []).find((one) => one.name === nameOf(selectedId));
  return statusOf(entry?.instance, now);
}

function statusOf(instance: ServerState | undefined, now: number): RunSheetServerStatus | undefined {
  if (instance === undefined) return undefined;
  if (instance.phase === "starting") return { phase: "starting" };
  if (instance.phase === "exited") return { phase: "exited", exitCode: instance.exit_code ?? 0 };
  return {
    phase: "serving",
    address: instance.ports[0] === undefined ? instance.serve : `localhost:${instance.ports[0].port}`,
    uptime: instance.serving_since === undefined ? "" : (span(instance.serving_since, now) ?? ""),
    links: instance.links.map((link) => ({ url: link.url, name: link.name })),
    startedByDrone: instance.started_by === "drone",
  };
}

/** The run a window is following, as `RunSheet.output`. `undefined` where it is reading no run. */
export function runOutputOf(followed: RunFollowed): ConsoleOutputProps | undefined {
  if (followed.state !== "following") return undefined;
  return {
    rows: followed.lines.map((text, at) => ({ row: "line" as const, at: followed.fromLine + at, text })),
    region: { says: followed.name, path: followed.path },
    following: followed.ended === undefined,
    emptyNote: followed.ended === undefined ? "Nothing printed yet." : "Printed nothing.",
  };
}

/** One row of *Earlier runs* — unhued, because a rehearsal carries no verdict. */
export function pastRunOf(record: RunRecord, onOpen: (id: string) => void): RunSheetPastRun {
  return {
    id: record.id,
    name: record.name,
    result:
      record.exit_code === undefined
        ? record.ended
        : `exit ${record.exit_code} (expects ${record.expect_exit_code})`,
    time: clockOf(record.started_at),
    duration: `${(record.duration_ms / 1000).toFixed(1)}s`,
    onOpen: () => onOpen(record.id),
  };
}

/** `HH:MM:SS`, off an ISO instant — the sheet's own run rows carry no date. */
function clockOf(at: string): string {
  return new Date(at).toLocaleTimeString([], { hour: "2-digit", minute: "2-digit", second: "2-digit" });
}

/**
 * The newest run that wrote something and has not been undone — the one a
 * changed-files panel offers **Open the diff** and **Undo this run** for.
 * `undefined` where nothing has, or the one that did has already been undone.
 */
export function changedRunOf(runs: readonly RunRecord[]): RunRecord | undefined {
  return runs.find((run) => run.changed.length > 0 && run.undone_at === undefined);
}

/** Whether the sheet should draw a running strip, and what it says. */
export function runningOf(
  sheet: RunSheet | undefined,
  now: number,
): { elapsed: string } | undefined {
  const running = sheet?.running;
  if (running === undefined) return undefined;
  return { elapsed: span(running.started_at, now) ?? "just started" };
}

/** The ten props `JobDetailProps.rehearsal` bundles — one prop, `recorded`'s precedent. */
export type RunSheetSlice = {
  runSheet: RunSheetRead;
  runFollowed: RunFollowed;
  /** Every server Fleet holds, for the *Serving* rows on *Where things are*. */
  servers: ServerList;
  onWatchRunSheet: (jobId: string | null) => void;
  onObserveRun: (jobId: string | null, runId: string | null) => void;
  onStartRun: (jobId: string, body: StartRun) => Promise<Outcome>;
  onStopRun: (jobId: string, runId: string) => Promise<Outcome>;
  onUndoRun: (jobId: string, runId: string) => Promise<Outcome>;
  onListRuns: (jobId: string) => Promise<RunListRead>;
  onGetRunOutput: (jobId: string, runId: string) => Promise<RunOutputRead>;
  onStartServer: (name: string, jobId: string) => Promise<Outcome>;
  onStopServer: (serverId: string) => Promise<Outcome>;
  onOpenServerLink: (serverId: string, url: string) => Promise<Followed>;
};

/**
 * The run sheet's whole state machine — its own selection, the two reads it
 * opens and drops with itself, and the props `RunSheet` takes. One hook
 * rather than the piece of state and the wiring behind each of them, so
 * `JobDetail.tsx` gains one call.
 */
export function useRunSheet(
  slice: RunSheetSlice & {
    jobId: string;
    jobTitle: string;
    /** Which sheet is open. `"run"` and `"diff"` both matter here. */
    sheet: string | null;
    now: number;
    /** Puts a sheet up — `JobDetail`'s own sheet state. `diff` is **Open the diff**'s. */
    setSheet: (which: "run" | "diff") => void;
    /** Says why a server's link did not open. `JobDetail`'s own toast. */
    onSaid: (sentence: string) => void;
  },
): {
  /** Open the sheet, with `undefined` for nothing selected. */
  open: (entryId?: string) => void;
  slot: Omit<RunSheetProps, "open" | "floor" | "onClose">;
  /** The sheet's own reading of whether the worktree is still there. `undefined`
   * before the sheet has ever been opened — the worktree row falls back to
   * whether the Job has dispatched one at all. */
  worktreeOnDisk: boolean | undefined;
} {
  const {
    jobId,
    jobTitle,
    sheet,
    now,
    setSheet,
    onSaid,
    runSheet,
    runFollowed,
    onWatchRunSheet,
    onObserveRun,
    onStartRun,
    onStopRun,
    onUndoRun,
    onListRuns,
    onGetRunOutput,
    onStartServer,
    onStopServer,
    onOpenServerLink,
  } = slice;
  const [selected, setSelected] = useState<string | null>(null);
  const [wide, setWide] = useState<ReadonlySet<string>>(new Set());
  // Earlier runs, newest first, read alongside the sheet — `list_runs`.
  const [runs, setRuns] = useState<readonly RunRecord[]>([]);
  // A past run's own log, opened from *Earlier runs*. Replaces the live pane
  // until a fresh run starts or the sheet closes.
  const [viewing, setViewing] = useState<{ runId: string; output: ConsoleOutputProps } | null>(null);
  useEffect(() => {
    setSelected(null);
    setWide(new Set());
    setRuns([]);
    setViewing(null);
  }, [jobId]);

  const refreshRuns = (): void => {
    void onListRuns(jobId).then((read) => {
      if (read.ok) setRuns(read.runs.runs);
    });
  };

  // Opened with the sheet and dropped when it closes, `diff`'s reason: most
  // Jobs are never rehearsed, so nothing pays for a Job nobody opened the
  // sheet on.
  useEffect(() => {
    if (sheet !== "run") return;
    onWatchRunSheet(jobId);
    refreshRuns();
    return () => onWatchRunSheet(null);
  }, [sheet, jobId]);

  const data = runSheet.state === "read" && runSheet.jobId === jobId ? runSheet.sheet : undefined;

  // A run already underway is followed the moment the sheet shows it — the
  // one `startRun` just began, just as much as one the sheet is reopening
  // onto. `observeRun` is idempotent on the same run, so asking again on
  // every tick sends nothing twice.
  const runningId = data?.running?.id;
  useEffect(() => {
    if (sheet !== "run" || runningId === undefined) return;
    onObserveRun(jobId, runningId);
  }, [sheet, jobId, runningId]);
  // The run sheet's own reading moves the instant a run finishes — see
  // `RehearsalConnection.onRunFinished` — which is the signal that *Earlier
  // runs* has one more row.
  useEffect(() => {
    if (sheet === "run" && runningId === undefined) refreshRuns();
  }, [sheet, runningId]);

  const runningNow = runningOf(data, now);
  const changedRun = changedRunOf(runs);

  return {
    worktreeOnDisk: data?.worktree_on_disk,
    open: (entryId) => {
      setSheet("run");
      setSelected(entryId ?? null);
      setViewing(null);
    },
    slot: {
      jobName: jobTitle,
      ...(data?.manifest_edited_at === undefined
        ? {}
        : { manifestEditedAt: `armada.yml last edited ${absoluteOf(data.manifest_edited_at) ?? "—"}` }),
      groups: data === undefined ? [] : runSheetGroupsOf(data, wide),
      selectedId: selected,
      onSelect: (id) => {
        setSelected(id);
        setViewing(null);
      },
      onToggleNarrow: (id) =>
        setWide((prior) => {
          const next = new Set(prior);
          if (next.has(id)) next.delete(id);
          else next.add(id);
          return next;
        }),
      onRun: ({ id, narrowed }) => {
        if (isServerEntry(id)) void onStartServer(nameOf(id), jobId);
        else void onStartRun(jobId, { name: nameOf(id), narrowed });
      },
      droneWorking: data?.drone_working ?? false,
      ...(data?.worktree_differs !== true
        ? {}
        : {
            manifestDiffers: {
              // The selected entry, run again against the worktree's own
              // `armada.yml` rather than what the Job froze — `start_run`'s
              // own `worktree_version` flag.
              ...(selected === null || isServerEntry(selected)
                ? {}
                : {
                    onUseWorktreeVersion: () =>
                      void onStartRun(jobId, {
                        name: nameOf(selected),
                        narrowed: !wide.has(selected),
                        worktree_version: true,
                      }),
                  }),
            },
          }),
      output: viewing?.output ?? runOutputOf(runFollowed),
      ...(runningNow === undefined
        ? {}
        : {
            running: {
              elapsed: runningNow.elapsed,
              ...(data?.running === undefined
                ? {}
                : { onStop: () => void onStopRun(jobId, data.running!.id) }),
            },
          }),
      ...(runs.length === 0
        ? {}
        : { runs: runs.map((record) => pastRunOf(record, (id) => openPastRun(id))) }),
      ...(changedRun === undefined
        ? {}
        : {
            changed: {
              files: changedRun.changed,
              onOpenDiff: () => setSheet("diff"),
              ...(data?.drone_working === true
                ? {}
                : { onUndo: () => void onUndoRun(jobId, changedRun.id).then(refreshRuns) }),
            },
          }),
      server: serverStatusOf(data, selected, now),
      onStopServer: (id) => {
        const instanceId = serverInstanceIdOf(data, id);
        if (instanceId !== undefined) void onStopServer(instanceId);
      },
      onOpenLink: (url) => {
        const instanceId = serverInstanceIdOf(data, selected ?? undefined);
        if (instanceId === undefined) return;
        void openServerLink(onOpenServerLink, instanceId, url).then((because) => {
          if (because !== null) onSaid(because);
        });
      },
    },
  };

  /** *Earlier runs*' own control — reads that run's log into the output pane. */
  function openPastRun(runId: string): void {
    void onGetRunOutput(jobId, runId).then((read) => {
      if (!read.ok) return;
      const output = read.output;
      setViewing({
        runId,
        output: {
          rows: output.lines.map((text, at) => ({ row: "line" as const, at: output.from_line + at, text })),
          region: { says: output.name, path: output.path },
          emptyNote: "Printed nothing.",
        },
      });
    });
  }
}
