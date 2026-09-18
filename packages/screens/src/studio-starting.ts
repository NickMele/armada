// What a Studio can start, off the checkout's own run sheet — #1345, #1289.
// No React: this is a function of the sheet Fleet sent, so it is tested as one.
//
// **The Manifest Fleet holds, never the Studio's own.** A Studio belongs to one
// repository and a run started from it runs in that repository's checkout, so
// what is startable is what that checkout declares — the same sheet the
// Manifest surface is drawn from.

import type { DropdownMenuEntry } from "@armada/components";
import type { CheckoutRunSheetRead } from "@armada/protocol";

/**
 * One thing a Studio can start.
 *
 * **`server` is the whole of the difference on this side.** A server goes to
 * `start_studio_server` and everything else to `start_studio_run`, because a
 * server is held rather than run and is read back by a different reader.
 */
export type StudioStart = {
  /** The menu's own id: the kind and the name, so two of one name stay apart. */
  id: string;
  name: string;
  server: boolean;
};

/** The menu's id for one entry. Exported so a test names one the way the menu does. */
export function startId(name: string, server: boolean): string {
  return `${server ? "server" : "run"}:${name}`;
}

/**
 * Everything the checkout declares, servers last.
 *
 * **Setup is left out.** Its Commands are run to repair an install in a
 * worktree, which is a Job's own business; a Studio starts what a person is
 * looking at. Workspaces are left out for now: `start_studio_run` takes one, and
 * nothing on this surface picks a directory yet.
 */
export function studioStarts(read: CheckoutRunSheetRead): StudioStart[] {
  if (read.state !== "read") return [];
  const runs = [...read.sheet.checks, ...read.sheet.commands].map((entry) => ({
    id: startId(entry.name, false),
    name: entry.name,
    server: false,
  }));
  const servers = (read.sheet.servers ?? []).map((entry) => ({
    id: startId(entry.name, true),
    name: entry.name,
    server: true,
  }));
  return [...runs, ...servers];
}

/**
 * The menu, grouped and labelled — **Checks and Commands under one label and
 * servers under their own**, because a server is what a person came here to
 * start and it does not stop when it is done.
 *
 * `undefined` where there is nothing to start, which is what says to draw no
 * control at all rather than an empty menu.
 */
export function studioStartEntries(starts: readonly StudioStart[]): DropdownMenuEntry[] | undefined {
  if (starts.length === 0) return undefined;
  const runs = starts.filter((start) => !start.server);
  const servers = starts.filter((start) => start.server);
  const item = (start: StudioStart): DropdownMenuEntry => ({
    kind: "item",
    id: start.id,
    label: start.name,
  });
  return [
    ...(runs.length === 0 ? [] : [{ kind: "label" as const, id: "runs", label: "Checks and Commands" }]),
    ...runs.map(item),
    ...(servers.length === 0
      ? []
      : [
          ...(runs.length === 0 ? [] : [{ kind: "separator" as const, id: "before-servers" }]),
          { kind: "label" as const, id: "servers", label: "Servers" },
        ]),
    ...servers.map(item),
  ];
}
