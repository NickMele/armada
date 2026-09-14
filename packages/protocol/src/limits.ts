// The four limits a person changes while Fleet runs.
// `crates/ipc/src/limits.rs`.
//
// **A saved value applies at Fleet's next admission turn**, with no restart,
// and survives one. Nothing already running is stopped: lowering the number of
// Drones below how many are working holds new starts until enough finish.
//
// **A value out of range is refused whole.** The save answers 400 in the
// ordinary error shape and nothing is saved, including any in-range field sent
// beside it. The ranges are the comments below, and Fleet is their authority.
//
// The header rules in `protocol.ts` hold here: these are hand-written, and they
// drift the day a field moves.

/** One value for each of the four limits. */
export type LimitValues = {
  /** Drones at once, 1 to 8. */
  concurrency: number;
  /** The share of memory that must be free before a Drone starts, 0 to 50. */
  memory_spare_percent: number;
  /** The GiB that must be free on the worktree volume, 0 to 100. */
  disk_floor_gib: number;
  /** How many of one step's Checks run at once, 1 to 8. Since 13.39. */
  checks_at_once: number;
};

/** The limits in force — `GET /limits` — and the ones Fleet shipped with. */
export type FleetLimits = LimitValues & { shipped: LimitValues };

/**
 * A save — `POST /limits/save`. **An omitted field keeps its current value**,
 * saved or shipped. The answer is the `FleetLimits` now in force.
 */
export type SaveLimits = Partial<LimitValues>;
