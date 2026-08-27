// How long, between two instants the wire carried.
//
// **Bridge computes no durations Fleet did not date.** Both ends come from
// served fields — `created_at` for a whole Job, `entered_at` to `updated_at`
// for one step — so what renders is the record's span rather than a figure
// this process invented.
//
// `fleet.elapsed` is the connection's own reading and rounds to one unit,
// which is right for "last read 4m ago" and wrong for a step that took 4m 09s.

/** An RFC 3339 instant, in epoch milliseconds. `null` where it will not parse. */
export function instant(at: string): number | null {
  const ms = Date.parse(at);
  return Number.isNaN(ms) ? null : ms;
}

/**
 * The span between two instants, as "4m 09s". Hours drop the seconds, because
 * a Job three hours in is not read to the second.
 *
 * `null` where either end will not parse — a Job whose dates are unreadable
 * says nothing rather than showing a span measured from zero.
 */
export function span(from: string, to: string | number): string | null {
  const start = instant(from);
  const end = typeof to === "number" ? to : instant(to);
  if (start === null || end === null) return null;
  const seconds = Math.max(0, Math.round((end - start) / 1000));
  if (seconds < 60) return `${seconds}s`;
  const minutes = Math.floor(seconds / 60);
  if (minutes < 60) return `${minutes}m ${pad(seconds % 60)}s`;
  return `${Math.floor(minutes / 60)}h ${pad(minutes % 60)}m`;
}

function pad(value: number): string {
  return value < 10 ? `0${value}` : String(value);
}
