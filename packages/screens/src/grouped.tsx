// The activity log, with runs of one tool folded to a line.
//
// **The grouping is derived from the turns and drawn through the log.** What a
// row is, how it opens and which keyboard reaches it stay `Log.tsx`'s — this
// only decides which rows belong together and what the folded line says. So a
// group's body is the same component the log draws unfolded, and the region and
// payload names `detail-keys.ts` reads are untouched.
//
// **The ids line up because both sides use the socket's sequence.** A
// `WorkingAct` is named `String(turn.seq)` and so is a `LogRow`, which is what
// lets a run select its own rows without a second pass over the wire.
import { WorkGroups, type WorkGroup } from "@armada/components";
import type { Turn } from "@armada/protocol";

import type { Calls } from "./calls";
import type { DetailKeys } from "./detail-keys";
import { briefly } from "./duration";
import { Log } from "./Log";
import type { LogRow } from "./story";
import { runsOf, workingOf } from "./working";

export function WorkGrouped({
  rows,
  turns,
  stepId,
  unread,
  emptyNote,
  calls,
  log,
  most,
}: {
  /** The step's readable rows, in order — what the log would draw flat. */
  rows: LogRow[];
  /** The same turns those rows came from, which carry the tool and the timing. */
  turns: readonly Turn[];
  stepId: string;
  /** How many rows this Bridge has no drawing for, by the wire's own kind. */
  unread?: { kind: string; count: number }[];
  emptyNote: string;
  calls: Calls;
  /** The log's own keyboard binding, by region. Threaded to every group. */
  log: ReturnType<DetailKeys["inLog"]>;
  /**
   * How many groups to draw, newest last. Absent draws them all.
   *
   * **Counted in groups and not in rows**, which is the point of the folding: a
   * preview bounded at five rows showed five `Read` calls, and the same height
   * bounded at five groups shows five different things the Drone did.
   */
  most?: number;
}) {
  const held = new Map(rows.map((row) => [row.id, row]));
  const groups: WorkGroup[] = [];
  for (const run of runsOf(workingOf(turns, stepId).acts)) {
    // A run whose rows are not in hand draws nothing. The two derivations drop
    // the same rows, so this is empty in practice and never an assumption.
    // **The call's row and its answer's.** The derivation folds an answer into
    // its call to count one thing that happened; a surface drawing the
    // transcript still has to draw both, and selecting on the call alone
    // dropped every answer row — 351 of them on one real step.
    const mine = run.acts.flatMap((act) => {
      const row = held.get(act.id);
      const answer = act.answeredId === undefined ? undefined : held.get(act.answeredId);
      return [...(row === undefined ? [] : [row]), ...(answer === undefined ? [] : [answer])];
    });
    if (mine.length === 0) continue;
    groups.push({
      id: run.id,
      name: run.tool ?? "",
      mono: true,
      // **Counted in calls, not in rows.** `mine` holds each call's answer
      // beside it, so counting rows here said six for a run of three.
      meta: `${run.acts.length} ${run.acts.length === 1 ? "call" : "calls"} · ${briefly(run.ms)}`,
      // Folded only where the derivation says so, which is a run of more than
      // one with nothing wrong in it.
      folded: run.folded,
      body: <Log rows={mine} emptyNote={emptyNote} calls={calls} {...log} />,
    });
  }
  return (
    <WorkGroups
      groups={most === undefined ? groups : groups.slice(-most)}
      {...(unread === undefined ? {} : { unread })}
      emptyNote={emptyNote}
    />
  );
}
