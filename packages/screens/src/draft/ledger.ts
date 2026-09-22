// One line of the Record: what happened, where, and what it came to. Draft,
// for `crates/ipc/src/history.rs`.
//
// Source of truth today: `JobHistory.moves` — `Recorded` with its `seq`,
// `status`, `actor`, `at` and one of three `Movement` shapes.
//
// **`kind` stays an opaque string** (#1532). A new kind of row is then a minor
// bump rather than a major one: nothing here branches on the value, a surface
// looks it up and renders the spelling itself where it finds no word for it.
// That is the same argument `FleetCapacity.held_by` is made on in
// `docs/practices/protocol.md`.

import type { JobHistory, Recorded } from "@armada/protocol";

import type { RunCoord } from "./coord";

/** Who a row is about. The wire's `Actor`, plus the two the new shape needs. */
export type LedgerActor = "person" | "fleet" | "drone" | "judge" | "check";

/**
 * One row of the Record.
 *
 * **`coord` and `actor` replace `step` and `task?`** (#1532). A row inside a
 * group could not be placed by a step id alone, and a row about no step at all
 * — the Job's own machine moving — needs to say so rather than carry a blank.
 */
export type LedgerRow = {
  at: string;
  /**
   * Where it happened. **`null` is a fact about the Job and not about any
   * step** — its own machine moving, a person approving it. Not a gap.
   */
  coord: RunCoord | null;
  actor: LedgerActor;
  /** An opaque string. A surface renders the spelling where it has no word. */
  kind: string;
  /** What happened, in words. */
  what: string;
  /** What it came to, in words. Empty where the row states no outcome. */
  outcome: string;
  /**
   * What a reader asks for the next page with.
   *
   * **The log's own `seq`, which is monotonic and never reused** — and never
   * `at`, which is injected rather than read from a clock, so two rows inside
   * one millisecond carry the same instant.
   */
  cursor: number;
};

/** Every row of a Job's Record, oldest first. */
export function ledgerRowsOf(history: JobHistory): LedgerRow[] {
  return history.moves.map(ledgerRowOf);
}

/** One recorded move, as a Record row. */
export function ledgerRowOf(move: Recorded): LedgerRow {
  return {
    at: move.at,
    coord: coordOf(move),
    actor: actorOf(move.actor),
    kind: kindOf(move),
    what: whatOf(move),
    outcome: outcomeOf(move),
    cursor: move.seq,
  };
}

// The Job's own machine moving names no step, so it is `null`. A step move and
// a Drone arriving both name one, and neither names a group or a task — the
// wire has neither, so the coordinate stops at the step.
function coordOf(move: Recorded): RunCoord | null {
  if (move.moved.kind === "status") {
    return null;
  }
  return { step: move.moved.step_id, step_attempt: 1 };
}

// `movement_kind` is the registry's own vocabulary for the three shapes. A
// status move is qualified by where it went, and a Drone move by its
// `presence`, so one Record can hold `drone_spawned` and `drone_exited` as
// different rows rather than as one row a reader has to open.
function kindOf(move: Recorded): string {
  switch (move.moved.kind) {
    case "status":
      return `status_${move.moved.to}`;
    case "step":
      return "step";
    case "drone":
      return move.moved.presence;
  }
}

function whatOf(move: Recorded): string {
  switch (move.moved.kind) {
    case "status":
      return `${move.status} to ${move.moved.to}`;
    case "step":
      return `${move.moved.step_id}: ${move.moved.from} to ${move.moved.to}`;
    case "drone":
      return `${move.moved.drone_id} on ${move.moved.step_id}`;
  }
}

// The reason a row carries, where it carries one. A status move's `reason` is
// absent on the eight destinations that store none, and a step move's `why` is
// present on the one move that stops it — so an empty outcome is the ordinary
// case rather than a value that failed to load.
function outcomeOf(move: Recorded): string {
  switch (move.moved.kind) {
    case "status":
      return move.moved.reason?.named ?? "";
    case "step":
      return move.moved.why ?? "";
    case "drone":
      return "";
  }
}

// The wire's `Actor` is `human`, `fleet` or `drone`. `judge` and `check` are
// the draft's own — a Judge call and a Check run are what the new Record draws
// most of, and today they arrive as Fleet acting. So neither can be derived,
// and a row that would be one of them reads as `fleet` rather than as a guess.
function actorOf(actor: string): LedgerActor {
  switch (actor) {
    case "drone":
      return "drone";
    case "human":
      return "person";
    default:
      return "fleet";
  }
}
