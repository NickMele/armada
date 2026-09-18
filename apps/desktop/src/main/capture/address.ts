// What the capture window may load, and what holds it there — #1294,
// `docs/practices/capture-window.md`, *What it may load*.
//
// **The renderer names a server id and one of that server's links.** Main
// resolves the address off the live holder Fleet published and loads no string
// a click handler composed — `servers.ts`'s rule for opening a link in the
// system browser, which loading one in a window is strictly more than.
//
// Pure, and tested as such. Nothing here reaches Electron.

import type { CaptureOpened } from "@armada/protocol";
import type { BridgeState } from "../../shared/bridge";

/** The three hosts a process on this machine binds. Nothing else opens a window. */
const LOOPBACK = ["127.0.0.1", "[::1]", "::1", "localhost"] as const;

/** The two schemes. A `file:`, a `data:` or a custom scheme opens no window. */
const SCHEMES = ["http:", "https:"] as const;

/**
 * An address this window is pinned to: the whole URL as the link gave it, and
 * the origin every later navigation is compared against.
 */
export type Pinned = {
  /** The instance Fleet holds, by its id. */
  run: string;
  /** What the Manifest calls it. */
  name: string;
  /** Scheme, host and port — what a comparison is on. */
  origin: string;
  /** The link in full, which is what is loaded once. */
  url: string;
  /** The repository the server runs in, which keys the session partition. */
  manifestId: string | null;
};

/**
 * The address for a server id and one of its own links, or why there is none.
 *
 * **Three tests and the order matters.** A server Fleet no longer holds is not
 * the same answer as a link that is not its own, and a Manifest declaring a
 * server behind a public host or a tunnel is a third — that one names the
 * address, because the person is owed the reason their own declaration was
 * refused rather than a window that did not open.
 */
export function pinned(state: BridgeState, serverId: string, url: string): Pinned | CaptureOpened {
  const server = state.servers.servers.find((one) => one.id === serverId);
  if (server === undefined) return { ok: false, why: "no_address" };
  // A loopback port is not an identity: once the process is gone anything on
  // the machine may bind it, so a window is never opened onto one that ended.
  if (server.phase === "exited") return { ok: false, why: "not_serving" };
  if (!server.links.some((link) => link.url === url)) return { ok: false, why: "no_address" };
  const origin = loopbackOrigin(url);
  if (origin === null) return { ok: false, why: "not_loopback", address: url };
  return {
    run: server.id,
    name: server.name,
    origin,
    url,
    manifestId: server.manifest_id ?? null,
  };
}

/** Whether `answer` is an address rather than a refusal. */
export function isPinned(answer: Pinned | CaptureOpened): answer is Pinned {
  return "origin" in answer;
}

/**
 * The origin of a loopback web address, or `null` for anything else.
 *
 * **The host is matched whole, never by suffix.** `127.0.0.1.example.com` ends
 * in nothing this list holds because nothing in this list is a suffix test.
 */
export function loopbackOrigin(address: string): string | null {
  let parsed: URL;
  try {
    parsed = new URL(address);
  } catch {
    return null;
  }
  if (!SCHEMES.some((scheme) => scheme === parsed.protocol)) return null;
  const host = parsed.hostname.toLowerCase();
  if (!LOOPBACK.some((one) => one === host || `[${one}]` === host)) return null;
  return parsed.origin;
}

/**
 * Whether an address is on the origin this window opened on.
 *
 * **Scheme, host and port together, and nothing matched by prefix, by suffix
 * or by hostname alone.** `URL.origin` is the whole triple, so a comparison on
 * it is the comparison the review asks for and a string test would not be.
 *
 * An address the engine will not parse is not on the origin. `about:blank`
 * parses and its origin is `null`, which no pinned origin ever equals.
 */
export function onOrigin(origin: string, address: string): boolean {
  try {
    return new URL(address).origin === origin;
  } catch {
    return false;
  }
}

/**
 * Whether a refused address is one the system browser would take. **`http:`
 * and `https:` only** — a refusal offers the browser a person already uses
 * rather than a second browser inside Armada, and it offers nothing for a
 * scheme that is not the web.
 */
export function offerable(address: string): boolean {
  try {
    return SCHEMES.some((scheme) => scheme === new URL(address).protocol);
  } catch {
    return false;
  }
}

/**
 * The session partition this window runs in: persistent, and one per
 * repository.
 *
 * **An app under development needs a login**, and a window that forgot it
 * every time is a window used once. Bridge's own session holds none of it, and
 * two repositories share nothing — so a Manifest id keys it, and a server Fleet
 * answered without one gets a partition of its own rather than everyone else's.
 */
export function partitionFor(manifestId: string | null): string {
  return `persist:armada-capture-${manifestId ?? "unnamed"}`;
}
