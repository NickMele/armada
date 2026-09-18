// One row of the Manifest surface's list: what a Check or Command declares,
// and what the surface knows about it.
//
// **Split off `checkout-runs.ts` when the rows gained facts** — #1383. That
// module is the surface's whole reading and was already over the 500-line
// warning; what a single row carries is its own question and this is where it
// is answered. A workspace's own row stays in `checkout-workspace.ts`, beside
// the id scheme that tells two Commands of one name apart.

import type { RunPageEntry } from "@armada/components";
import type { CheckoutRunRecord, RunEntry, ServerEntry } from "@armada/protocol";
import { sameCheckoutEntry } from "./checkout-workspace";
import { clockOf } from "./duration";
import { runOutcomeOf, SERVER_PREFIX } from "./rehearsal";

/**
 * What a row carries beyond its own declaration: whether its line drifted, and
 * how it last ran. **Both already in the window** — drift is the read the
 * surface holds open and the runs are what *Earlier runs* is drawn from.
 */
export type CheckoutSeen = {
  /** Every entry name whose line names something the checkout no longer has. */
  gone: ReadonlySet<string>;
  /** Finished runs in this checkout, newest first. */
  runs: readonly CheckoutRunRecord[];
};

/**
 * One row. **`narrow_run` is never read**, and that is not an oversight: Fleet
 * builds this sheet against no changed paths, so the field is absent by
 * construction — and drawing a scope control off a field that can only ever be
 * absent would be a control that means nothing here.
 */
export function entryOf(prefix: string, entry: RunEntry, seen?: CheckoutSeen): RunPageEntry {
  const id = `${prefix}${entry.name}`;
  const last = seen?.runs.find((record) => sameCheckoutEntry(id, record));
  return {
    id,
    name: entry.name,
    run: entry.run,
    note: noteOf(entry.requires),
    ...(entry.destructive ? { destructive: true } : {}),
    ...(seen?.gone.has(entry.name) === true ? { drifted: true } : {}),
    ...(last === undefined ? {} : { last: lastOf(last) }),
  };
}

/**
 * The last run on a row: the code alone, with what it expected in the title.
 * **Short by construction** — the rail is 240px, and `exit 2 (expects 0)`
 * beside a clock leaves no room for the command above it.
 */
function lastOf(record: CheckoutRunRecord): NonNullable<RunPageEntry["last"]> {
  return {
    result: record.exit_code === undefined ? record.ended : `exit ${record.exit_code}`,
    ...(record.exit_code === undefined
      ? {}
      : { whole: `exit ${record.exit_code} (expects ${record.expect_exit_code})` }),
    outcome: runOutcomeOf(record),
    at: clockOf(record.started_at),
  };
}

export function serverEntryOf(entry: ServerEntry): RunPageEntry {
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
