// Which rows changed status while a list was showing them, and how far each mark has decayed —
// the decay clock in `docs/contracts/design-system.md` under Motion. `JobRowStacked` draws it.
//
// **A change is a badge read twice and different**: not the first reading, and not a Job seen for
// the first time, which is a new row rather than a changed one.
//
// **Dated by when Bridge saw it.** `JobSummary` carries no instant for a status change, so this is
// the one span here Fleet did not date; a reconnect marks what moved while Bridge was away.
//
// **One ticker per list, and none while nothing decays.** A timer rather than a CSS transition,
// because the elapsed words count up in the same pass. Reduced motion does not stop it: a colour
// fading moves nothing, and `motion.css` leaves `--duration-decay` unzeroed for that reason.

import type { JobSummary } from "@armada/protocol";
import { useEffect, useState } from "react";

import { readingOf } from "./reading";

/** How far one row's mark has gone. */
export type Recent = {
  /** Milliseconds since the change was seen. */
  age: number;
  /** 1 the moment it changed, 0 once it rests at `--row-tint`. */
  remaining: number;
};

/** What a list has seen: each Job's badge as last read, and when each change was noticed. */
export type Seen = {
  statuses: ReadonlyMap<string, string>;
  changedAt: ReadonlyMap<string, number>;
};

/** Half the smallest unit the words show, so the count never lags a whole second. */
const TICK_MS = 500;

/** The badge's reading: the state, and the reason's word where the badge takes one. */
function badgeOf(job: JobSummary): string {
  return `${job.status}|${readingOf(job).verb ?? ""}`;
}

/**
 * The next reading, from the last. `null` is the first, which marks nothing. A mark past `decayMs`
 * is dropped, and a Job that left the list takes its mark with it.
 */
export function observe(last: Seen | null, jobs: readonly JobSummary[], at: number, decayMs: number): Seen {
  const statuses = new Map<string, string>();
  const changedAt = new Map<string, number>();
  for (const job of jobs) {
    const now = badgeOf(job);
    statuses.set(job.id, now);
    if (last === null) continue;
    const before = last.statuses.get(job.id);
    const marked = last.changedAt.get(job.id);
    if (before !== undefined && before !== now) changedAt.set(job.id, at);
    else if (marked !== undefined && at - marked < decayMs) changedAt.set(job.id, marked);
  }
  return { statuses, changedAt };
}

/** Every mark still decaying at `now`, by job id. */
export function decaying(seen: Seen, now: number, decayMs: number): Map<string, Recent> {
  const out = new Map<string, Recent>();
  if (!(decayMs > 0)) return out;
  for (const [id, at] of seen.changedAt) {
    const age = Math.max(0, now - at);
    const remaining = Math.max(0, 1 - age / decayMs);
    if (remaining > 0) out.set(id, { age, remaining });
  }
  return out;
}

/** A duration token in milliseconds, read off the document so the token stays the one number. */
export function tokenMs(name: string): number {
  if (typeof document === "undefined") return 0;
  const raw = getComputedStyle(document.documentElement).getPropertyValue(name).trim();
  const value = Number.parseFloat(raw);
  // `0` where it will not parse: nothing decays, rather than a guessed length.
  if (!Number.isFinite(value)) return 0;
  return raw.endsWith("ms") ? value : raw.endsWith("s") ? value * 1000 : 0;
}

/**
 * The rows of `jobs` whose status changed while this list was mounted, and how far each has gone.
 * `decayMs` and `clock` are a test's; the app passes neither.
 */
export function useRecentChanges(
  jobs: readonly JobSummary[],
  options: { decayMs?: number; clock?: () => number } = {},
): ReadonlyMap<string, Recent> {
  const clock = options.clock ?? Date.now;
  const [decayMs] = useState(() => options.decayMs ?? tokenMs("--duration-decay"));
  // Derived in render, React's pattern for state following a prop: an effect would paint the
  // changed row once at rest before marking it.
  const [held, setHeld] = useState(() => ({ jobs, seen: observe(null, jobs, clock(), decayMs) }));
  let seen = held.seen;
  if (held.jobs !== jobs) {
    seen = observe(held.seen, jobs, clock(), decayMs);
    setHeld({ jobs, seen });
  }
  const recent = decaying(seen, clock(), decayMs);
  const live = recent.size > 0;

  // It only re-renders; `decaying` reads the clock, so a late tick still draws the right age.
  const [, setTick] = useState(0);
  useEffect(() => {
    if (!live) return;
    const timer = setInterval(() => setTick((n) => n + 1), TICK_MS);
    return () => clearInterval(timer);
  }, [live]);

  return recent;
}
