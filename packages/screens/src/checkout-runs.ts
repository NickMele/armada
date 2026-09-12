// The Manifest surface's reading — Journey 9's *Running one*, in the main
// checkout — built from what Fleet serves plus the run a window is following.
//
// **Not `rehearsal.ts` with the Job id dropped.** That module is the run
// *sheet*, inside one Job, against the Manifest that Job froze; this is the
// run *page*, against the file Fleet holds, before any Job exists. The wire
// keeps them apart for the same reason — `CheckoutRunSheet` carries no
// `job_id`, no `worktree_on_disk`, no `drone_working` and no narrowing — so
// the two readings are separate here rather than one with four fields that are
// always absent.
//
// **The entry-id scheme is shared.** `rehearsal.ts` prefixes by group so a
// press can tell a Check from a Command from a server without a second lookup,
// and this uses the same four prefixes off the same helpers: an id that means
// one thing on one surface and another on the next is the drift the one scheme
// exists to prevent.

import { useEffect, useState } from "react";
import type {
  ConsoleOutputProps,
  RunPageEntry,
  RunPageGroup,
  RunPagePastRun,
  RunPageProps,
  RunPageResult,
  RunPageServerStatus,
} from "@armada/components";
import type {
  CheckoutRunList,
  CheckoutRunRecord,
  CheckoutRunSheet,
  Followed,
  Outcome,
  RunEntry,
  RunOutputRead,
  ServerEntry,
  ServerState,
  StartCheckoutRun,
} from "@armada/protocol";
import { absoluteOf, span } from "./duration";
import { openServerLink } from "./opening";
import { CHECK_PREFIX, COMMAND_PREFIX, isServerEntry, nameOf, SERVER_PREFIX, SETUP_PREFIX } from "./rehearsal";

/**
 * `GET /manifest/run_sheet`, as main published it.
 *
 * **`Reports`' shape rather than a `JobRead`**, and for that type's own stated
 * reason: there is no id to check an answer against, which is the whole of
 * what `JobRead` exists to do. The four states are the same four, because
 * "nobody asked" and "the read failed" are still different things to draw.
 *
 * Bridge-only, like `RunSheetRead` — nothing on the wire has this shape.
 */
export type CheckoutRunSheetRead =
  | { state: "none" }
  | { state: "reading" }
  | { state: "read"; sheet: CheckoutRunSheet }
  | { state: "failed"; outcome: Outcome };

/**
 * The checkout run a window is watching on `observe_checkout_run`, as main
 * holds it. `RunFollowed`'s four states with the Job taken out of each.
 */
export type CheckoutRunFollowed =
  | { state: "none" }
  | { state: "opening"; runId: string }
  | {
      state: "following";
      runId: string;
      name: string;
      path: string;
      /** The file's own line number of `lines[0]`, counted from one. */
      fromLine: number;
      /** The newest lines, oldest first, bounded. */
      lines: string[];
      /** Why the stream ended, or absent while it is still arriving. */
      ended?: string;
    }
  | { state: "failed"; runId: string; detail: string };

/** What `list_checkout_runs` came back as, answered to the caller. */
export type CheckoutRunListRead =
  | { ok: true; runs: CheckoutRunList }
  | { ok: false; outcome: Outcome };

/** What the Manifest surface asks of the host. One prop, `rehearsal`'s precedent. */
export type ManifestSlice = {
  /** `GET /manifest/run_sheet`, held open while this surface is showing. */
  sheet: CheckoutRunSheetRead;
  /** The run being read, as it prints. */
  followed: CheckoutRunFollowed;
  /** One run's output, or `null` to stop. */
  onObserveRun: (runId: string | null) => void;
  onStartRun: (body: StartCheckoutRun) => Promise<Outcome>;
  onStopRun: (runId: string) => Promise<Outcome>;
  onUndoRun: (runId: string) => Promise<Outcome>;
  onListRuns: () => Promise<CheckoutRunListRead>;
  onGetRunOutput: (runId: string) => Promise<RunOutputRead>;
  /** A declared server, in the main checkout — no Job. */
  onStartServer: (name: string) => Promise<Outcome>;
  onStopServer: (serverId: string) => Promise<Outcome>;
  onOpenServerLink: (serverId: string, url: string) => Promise<Followed>;
};

/**
 * The three groups the page lists, in the journey's order.
 *
 * **All three are drawn even where one is empty.** A Manifest that declares no
 * Commands is a fact about the file; a group that vanished would read as a
 * surface that failed to list them.
 */
export function checkoutGroupsOf(sheet: CheckoutRunSheet): RunPageGroup[] {
  return [
    { kind: "setup", label: "Setup", entries: sheet.setup.map((e) => entryOf(SETUP_PREFIX, e)) },
    { kind: "checks", label: "Checks", entries: sheet.checks.map((e) => entryOf(CHECK_PREFIX, e)) },
    {
      kind: "commands",
      label: "Commands",
      entries: [
        ...sheet.commands.map((e) => entryOf(COMMAND_PREFIX, e)),
        ...(sheet.servers ?? []).map(serverEntryOf),
      ],
    },
  ];
}

/**
 * One row. **`narrow_run` is never read**, and that is not an oversight: Fleet
 * builds this sheet against no changed paths, so the field is absent by
 * construction — and drawing a scope control off a field that can only ever be
 * absent would be a control that means nothing here.
 */
function entryOf(prefix: string, entry: RunEntry): RunPageEntry {
  return {
    id: `${prefix}${entry.name}`,
    name: entry.name,
    run: entry.run,
    note: noteOf(entry.requires),
    ...(entry.destructive ? { destructive: true } : {}),
  };
}

function serverEntryOf(entry: ServerEntry): RunPageEntry {
  return {
    id: `${SERVER_PREFIX}${entry.name}`,
    name: entry.name,
    run: entry.serve,
    note: entry.run === undefined ? undefined : `Runs ${entry.run} first.`,
    ...(entry.destructive ? { destructive: true } : {}),
  };
}

function noteOf(requires: readonly string[]): string | undefined {
  return requires.length === 0 ? undefined : `Runs ${requires.join(", ")} first.`;
}

/** The instance Fleet holds for a server entry id — never the entry id itself. */
export function checkoutServerInstanceIdOf(
  sheet: CheckoutRunSheet | undefined,
  entryId: string | undefined,
): string | undefined {
  if (sheet === undefined || entryId === undefined || !isServerEntry(entryId)) return undefined;
  return (sheet.servers ?? []).find((one) => one.name === nameOf(entryId))?.instance?.id;
}

/** The selected entry's server state, or `undefined` where it names no server. */
export function checkoutServerStatusOf(
  sheet: CheckoutRunSheet | undefined,
  selectedId: string | null,
  now: number,
): RunPageServerStatus | undefined {
  if (sheet === undefined || selectedId === null || !isServerEntry(selectedId)) return undefined;
  const entry = (sheet.servers ?? []).find((one) => one.name === nameOf(selectedId));
  return statusOf(entry?.instance, now);
}

function statusOf(instance: ServerState | undefined, now: number): RunPageServerStatus | undefined {
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

/**
 * The run a window is following, as `RunPage.output`.
 *
 * **Keyed to the selection, so it cannot outlive it** — `runOutputOf`'s rule
 * and the case that produced it: selecting a server and starting it brought
 * the serving bar up live while the pane beneath still read the Command run
 * before it. `selectedName` absent is the one case this does not gate, which
 * reads as "nothing to compare against" rather than "compare against nothing".
 */
export function checkoutOutputOf(
  followed: CheckoutRunFollowed,
  selectedName?: string,
): ConsoleOutputProps | undefined {
  if (followed.state !== "following") return undefined;
  if (selectedName !== undefined && followed.name !== selectedName) return undefined;
  return {
    rows: followed.lines.map((text, at) => ({ row: "line" as const, at: followed.fromLine + at, text })),
    region: { says: followed.name, path: followed.path },
    following: followed.ended === undefined,
    emptyNote: followed.ended === undefined ? "Nothing printed yet." : "Printed nothing.",
  };
}

/** One row of *Earlier runs* — unhued, because a rehearsal carries no verdict. */
export function checkoutPastRunOf(
  record: CheckoutRunRecord,
  onOpen: (id: string) => void,
): RunPagePastRun {
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

/** `HH:MM:SS`, off an ISO instant — a run row here carries no date. */
function clockOf(at: string): string {
  return new Date(at).toLocaleTimeString([], { hour: "2-digit", minute: "2-digit", second: "2-digit" });
}

/**
 * The newest run that wrote something and can still be put back.
 *
 * **`undoable` is Fleet's answer and not a guess.** A run with no snapshot
 * behind it has nothing to restore from, and this tree holds a person's own
 * uncommitted work — so an Undo offered where Fleet cannot honour it would be
 * the one control on this page whose failure costs somebody their work.
 */
export function checkoutChangedRunOf(
  runs: readonly CheckoutRunRecord[],
): CheckoutRunRecord | undefined {
  return runs.find((run) => run.changed.length > 0 && run.undone_at === undefined);
}

/** Whether the page should draw a running strip, and what it says. */
export function checkoutRunningOf(
  sheet: CheckoutRunSheet | undefined,
  now: number,
): { elapsed: string } | undefined {
  const running = sheet?.running;
  if (running === undefined) return undefined;
  return { elapsed: span(running.started_at, now) ?? "just started" };
}

/**
 * Every Check and Command the palette lists, off the same reading the page
 * draws from.
 *
 * **One row per entry, and they carry this page's own entry ids.** Choosing
 * one opens the Manifest surface with that entry picked, which is exactly what
 * Journey 9's table says the palette does for the Job's sheet — the press
 * selects, and Run is still the person's.
 */
export function checkoutRunnablesOf(
  read: CheckoutRunSheetRead,
): { id: string; label: string; value: string }[] {
  if (read.state !== "read") return [];
  return checkoutGroupsOf(read.sheet).flatMap((group) =>
    group.entries.map((entry) => ({
      id: entry.id,
      label: String(entry.name),
      value: entry.run,
    })),
  );
}

/**
 * The Manifest surface's whole state machine — its own selection, the runs it
 * reads beside the sheet, and the props `RunPage` takes.
 *
 * One hook rather than the piece of state and the wiring behind each of them,
 * `useRunSheet`'s reason: the screen gains one call.
 */
export function useManifestRuns(
  slice: ManifestSlice & {
    /** The entry the palette picked, or `null`. */
    picked: string | null;
    now: number;
    /** Says why a server's link did not open. The app's own toast. */
    onSaid: (sentence: string) => void;
  },
): RunPageProps {
  const {
    sheet,
    followed,
    picked,
    now,
    onSaid,
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

  const [selected, setSelected] = useState<string | null>(picked);
  // Earlier runs, newest first — `list_checkout_runs`.
  const [runs, setRuns] = useState<readonly CheckoutRunRecord[]>([]);
  // A past run's own log, opened from *Earlier runs*. Replaces the live pane
  // until a fresh run starts or the panel is dismissed.
  const [viewing, setViewing] = useState<{ runId: string; output: ConsoleOutputProps } | null>(null);
  // Whether the panel has been put away. **Dismiss closes the panel, not the
  // log** — every run writes its output under `./.armada`, so this throws away
  // a view and not a record, and the next run opens it again.
  const [dismissed, setDismissed] = useState(false);

  // The palette picked one. **It selects rather than runs**: a row that
  // started a destructive Command straight off a list of forty would be an act
  // nobody confirmed, and Journey 9's own table has the palette opening the
  // surface with that entry chosen.
  useEffect(() => {
    if (picked === null) return;
    setSelected(picked);
    setViewing(null);
  }, [picked]);

  const refreshRuns = (): void => {
    void onListRuns().then((read) => {
      if (read.ok) setRuns(read.runs.runs);
    });
  };

  const data = sheet.state === "read" ? sheet.sheet : undefined;

  // A run already underway is followed the moment the page shows it — the one
  // `startRun` just began, as much as one the surface is reopening onto.
  // `observeRun` is idempotent on the same run, so asking again sends nothing
  // twice.
  const runningId = data?.running?.id;
  useEffect(() => {
    if (runningId === undefined) return;
    onObserveRun(runningId);
  }, [runningId]);
  // Read on opening, and again the instant a run finishes — the sheet's own
  // reading moves on `checkout_run.finished`, which is the signal that
  // *Earlier runs* has one more row.
  useEffect(() => {
    if (runningId === undefined) refreshRuns();
  }, [runningId]);

  const runningNow = checkoutRunningOf(data, now);
  const changedRun = checkoutChangedRunOf(runs);
  const server = checkoutServerStatusOf(data, selected, now);
  const live = checkoutOutputOf(followed, selected === null ? undefined : nameOf(selected));
  const output = dismissed ? undefined : (viewing?.output ?? live);
  // Which run the result line is about: the one opened from *Earlier runs*, or
  // the newest finished one while the live pane is what is showing. **Never
  // while a run is in flight** — a code from the run before it, sitting under
  // the output of the one still printing, is the panel claiming a result that
  // has not happened.
  const about =
    dismissed || runningNow !== undefined
      ? undefined
      : viewing === null
        ? runs[0]
        : runs.find((record) => record.id === viewing.runId);

  return {
    ...(data?.manifest_edited_at === undefined
      ? {}
      : { manifestEditedAt: `armada.yml last edited ${absoluteOf(data.manifest_edited_at) ?? "—"}` }),
    groups: data === undefined ? [] : checkoutGroupsOf(data),
    selectedId: selected,
    onSelect: (id) => {
      setSelected(id);
      setViewing(null);
    },
    onRun: (id) => {
      setDismissed(false);
      setViewing(null);
      if (isServerEntry(id)) void onStartServer(nameOf(id));
      else void onStartRun({ name: nameOf(id) });
    },
    ...(output === undefined ? {} : { output }),
    onDismiss: () => {
      setDismissed(true);
      setViewing(null);
    },
    ...(runningNow === undefined
      ? {}
      : {
          running: {
            elapsed: runningNow.elapsed,
            ...(data?.running === undefined
              ? {}
              : { onStop: () => void onStopRun(data.running!.id) }),
          },
        }),
    ...(about === undefined ? {} : { result: resultOf(about) }),
    ...(runs.length === 0
      ? {}
      : { runs: runs.map((record) => checkoutPastRunOf(record, openPastRun)) }),
    ...(changedRun === undefined || dismissed
      ? {}
      : {
          changed: {
            files: changedRun.changed,
            // Offered only where Fleet says it can be honoured — `undoable` is
            // its answer, not a guess. There is no *Open the diff*: the main
            // checkout has no base to be read against, and nothing on the wire
            // answers for one.
            ...(changedRun.undoable
              ? { onUndo: () => void onUndoRun(changedRun.id).then(refreshRuns) }
              : {}),
          },
        }),
    ...(server === undefined ? {} : { server }),
    onStopServer: (id) => {
      const instanceId = checkoutServerInstanceIdOf(data, id);
      if (instanceId !== undefined) void onStopServer(instanceId);
    },
    onOpenLink: (url) => {
      const instanceId = checkoutServerInstanceIdOf(data, selected ?? undefined);
      if (instanceId === undefined) return;
      void openServerLink(onOpenServerLink, instanceId, url).then((because) => {
        if (because !== null) onSaid(because);
      });
    },
  };

  /** *Earlier runs*' own control — reads that run's log into the panel. */
  function openPastRun(runId: string): void {
    void onGetRunOutput(runId).then((read) => {
      if (!read.ok) return;
      const out = read.output;
      setDismissed(false);
      setViewing({
        runId,
        output: {
          rows: out.lines.map((text, at) => ({ row: "line" as const, at: out.from_line + at, text })),
          region: { says: out.name, path: out.path },
          emptyNote: "Printed nothing.",
        },
      });
    });
  }
}

/** The result line for a finished run. Unhued: a rehearsal is not a verdict. */
function resultOf(record: CheckoutRunRecord): RunPageResult {
  return {
    name: record.name,
    ...(record.exit_code === undefined ? {} : { exitCode: record.exit_code }),
    expected: record.expect_exit_code,
    ended: record.ended,
    duration: `${(record.duration_ms / 1000).toFixed(1)}s`,
  };
}
