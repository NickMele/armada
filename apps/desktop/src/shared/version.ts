// What each side speaks, and what a mismatch between them means.
//
// **The mirror of `crates/ipc/src/version.rs`, and deliberately the same four
// readings.** Bridge is the side that decides — it reads Fleet's version out of
// the runtime file before it opens a socket — so the rule has to be spelled
// where that decision is made. `protocol-version.toml` carries the rule in
// words and `docs/practices/protocol.md` carries the table both spellings
// implement.

/** One side's protocol version. Both numbers, never one. */
export type ProtocolVersion = { major: number; minor: number };

/**
 * What a version read off Fleet means for the connection.
 *
 * **Only `same` and `fleet_ahead` connect**, and the asymmetry is the point. A
 * minor bump is additive-only, so the newer side's additions are things the
 * older side never asks for and never reads — safe when Fleet is the newer one,
 * because Bridge ignores what it does not recognise. Reversed it is not: a
 * newer Bridge may require a field an older Fleet was built before sending, and
 * additive-only promises nothing about what a newer *reader* needs.
 */
export type Skew = "same" | "fleet_ahead" | "fleet_behind" | "incompatible";

/** Named rather than positional: two versions of the same shape swap silently. */
export function skew(sides: { fleet: ProtocolVersion; bridge: ProtocolVersion }): Skew {
  const { fleet, bridge } = sides;
  if (fleet.major !== bridge.major) return "incompatible";
  if (fleet.minor === bridge.minor) return "same";
  return fleet.minor > bridge.minor ? "fleet_ahead" : "fleet_behind";
}

/**
 * Whether the full protocol may be spoken at all.
 *
 * A predicate rather than a boolean so the two readings that refuse are what is
 * left in the other branch. A caller cannot put `fleet_ahead` on a skew screen
 * or `incompatible` on a live connection without the compiler saying so.
 */
export function connects(reading: Skew): reading is "same" | "fleet_ahead" {
  return reading === "same" || reading === "fleet_ahead";
}

/** How a version is written on screen: `4.0`. */
export function spoken(version: ProtocolVersion): string {
  return `${version.major}.${version.minor}`;
}

/**
 * A version as it arrived, or `null` where it is not one.
 *
 * A bare integer is read as that major at minor zero, which is what version 4
 * shipped as. Without it an older Fleet comes back as a message nothing wrote,
 * and a person is told their runtime file is corrupt when it is merely old.
 */
export function versionOf(value: unknown): ProtocolVersion | null {
  if (typeof value === "number") {
    return Number.isInteger(value) ? { major: value, minor: 0 } : null;
  }
  if (typeof value !== "object" || value === null) return null;
  const { major, minor } = value as Record<string, unknown>;
  if (!Number.isInteger(major) || !Number.isInteger(minor)) return null;
  return { major: major as number, minor: minor as number };
}
