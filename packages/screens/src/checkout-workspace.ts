// A workspace's own Commands on the Manifest surface: a row's id carries the directory whose
// `armada.yml` declares it, so two of one name stay apart from the press to Earlier runs.

import type { RunPageEntry, RunPageGroup } from "@armada/components";
import type { CheckoutRunFollowed, CheckoutRunRecord, RunEntry, StartCheckoutRun } from "@armada/protocol";
import { isServerEntry, nameOf } from "./rehearsal";

export const WORKSPACE_PREFIX = "workspace:";

/** One row, saying where it runs. */
export function workspaceEntryOf(dir: string, entry: RunEntry): RunPageEntry {
  const first = entry.requires.length === 0 ? "" : ` Runs ${entry.requires.join(", ")} first.`;
  return {
    id: `${WORKSPACE_PREFIX}${dir}/${entry.name}`,
    name: entry.name,
    run: entry.run,
    note: `In ${dir}.${first}`,
    ...(entry.destructive ? { destructive: true } : {}),
  };
}

/** What a row id names: the entry, and the workspace whose own file declares it. */
export function checkoutEntryOf(id: string): { name: string; workspace?: string } {
  if (!id.startsWith(WORKSPACE_PREFIX)) return { name: nameOf(id) };
  const rest = id.slice(WORKSPACE_PREFIX.length);
  const at = rest.lastIndexOf("/");
  return { name: rest.slice(at + 1), workspace: rest.slice(0, at) };
}

/** `start_checkout_run`'s body for a row: a workspace's names its directory. */
export function checkoutStartOf(id: string): StartCheckoutRun {
  const { name, workspace } = checkoutEntryOf(id);
  return workspace === undefined ? { name } : { name, workspace };
}

/** A run as a person reads it: a workspace's says where. */
export function checkoutRunLabelOf(run: { name: string; workspace?: string }): string {
  return run.workspace === undefined ? run.name : `${run.name} in ${run.workspace}`;
}

/** Whether a row and a run are the same entry, workspace and all. */
export function sameCheckoutEntry(id: string, run: { name: string; workspace?: string }): boolean {
  const at = checkoutEntryOf(id);
  return at.name === run.name && at.workspace === run.workspace;
}

/** The workspace a followed run is in: as it was seen out on the sheet, else its record's. */
export function followedWorkspaceOf(
  followed: CheckoutRunFollowed,
  seen: ReadonlyMap<string, string | undefined>,
  runs: readonly CheckoutRunRecord[],
): string | undefined {
  if (followed.state === "none") return undefined;
  if (seen.has(followed.runId)) return seen.get(followed.runId);
  return runs.find((record) => record.id === followed.runId)?.workspace;
}

/** The entry id a run in flight is for, off its bare Manifest name. Servers
 * start with `start_server` rather than as a run, so they never match. */
export function runningEntryOf(
  groups: readonly RunPageGroup[],
  runningName: string | undefined,
  workspace?: string,
): string | undefined {
  if (runningName === undefined) return undefined;
  const run = workspace === undefined ? { name: runningName } : { name: runningName, workspace };
  return groups
    .flatMap((group) => group.entries)
    .find((entry) => !isServerEntry(entry.id) && sameCheckoutEntry(entry.id, run))?.id;
}

/**
 * Which finished run the result line is about: one opened from *Earlier runs*, else the selected
 * entry's newest. **A server has none** — it is not a run, and `gate`'s result under it read as the server's.
 */
export function checkoutResultRunOf(
  runs: readonly CheckoutRunRecord[],
  shown: string | null,
  viewing: string | undefined,
): CheckoutRunRecord | undefined {
  if (viewing !== undefined) return runs.find((record) => record.id === viewing);
  if (shown === null) return runs[0];
  if (isServerEntry(shown)) return undefined;
  return runs.find((record) => sameCheckoutEntry(shown, record));
}
