// How often Pulse's reading is taken again, in the one place both sides read
// it from: main's `resources-poll.ts` runs the timer on it, and `resources.ts`
// builds the line under the figures out of it. A number spelled twice is how
// the board came to promise a poll nothing ran — `#1571`.

/** The design's *every 10s while open*. */
export const PULSE_INTERVAL_MS = 10_000;

/** The same interval as a board says it, so the sentence cannot drift from the timer. */
export const PULSE_INTERVAL_SAID = `${PULSE_INTERVAL_MS / 1000}s`;
