// Locate: a repository added by its folder or cloned from a URL, then listed. The window that asked picks it; a
// clone that lands is told to every window, and moves nobody's pick (#926). `docs/journeys/set-up-a-project-manifest.md`.

import { realpath, stat } from "node:fs/promises";

import type { AddRepository, CloneRepository, RepositorySummary } from "@armada/protocol";
import type { LocateAnswer } from "@armada/screens/src/locate-reads";

import { ask, COMMAND_MS, type Answer } from "./request";

/**
 * What a clone waits. **Past Fleet's own bound**, `CLONE_BUDGET` in
 * `crates/fleet/src/repositories/cloning.rs` — ten minutes — plus one, so Fleet's coded answer
 * arrives rather than Bridge's abort. Coupled to the Rust constant by this comment.
 */
export const CLONE_MS = 11 * 60_000;

export type LocatingWiring = {
  port: () => number | null;
  /** Read what Fleet serves again, so the window's pick finds the new root listed. */
  list: (port: number) => Promise<void>;
  /** A clone Fleet now serves, after it is listed and before the window that asked hears its answer. */
  landed?: (repository: RepositorySummary) => void;
};

export class Locating {
  private readonly wiring: LocatingWiring;
  private readonly asking: typeof ask;
  /** One add or clone at a time: a second press while a clone runs is not sent. */
  private out = false;

  constructor(wiring: LocatingWiring, asking: typeof ask = ask) {
    this.wiring = wiring;
    this.asking = asking;
  }

  add(path: string): Promise<LocateAnswer> {
    const body: AddRepository = { path };
    return this.send("/repositories/add", body, COMMAND_MS);
  }

  clone(url: string, parent: string): Promise<LocateAnswer> {
    const body: CloneRepository = { url, parent };
    return this.send("/repositories/clone", body, CLONE_MS, true);
  }

  private async send(path: string, body: unknown, waitMs: number, cloning = false): Promise<LocateAnswer> {
    const port = this.wiring.port();
    if (port === null) return { state: "failed", outcome: { ok: false, why: "not_connected" } };
    if (this.out) return { state: "busy" };
    this.out = true;
    try {
      const answer = locateAnswerOf(await this.asking(port, "POST", path, body, waitMs));
      if (answer.state !== "located") return answer;
      // The port a clone started on may be gone ten minutes later.
      const now = this.wiring.port();
      if (now !== null) await this.wiring.list(now);
      // A clone can outlast its dialog and its window's attention, so every window hears it land.
      if (cloning) this.wiring.landed?.(answer.repository);
      return answer;
    } finally {
      this.out = false;
    }
  }
}

/**
 * The folder a clone's parent names, as Fleet canonicalises it — `/tmp` is `/private/tmp` — or `null`
 * where it is not an absolute folder on this machine, which Fleet refuses and the preview leaves bare.
 */
export async function resolvedFolder(path: string): Promise<string | null> {
  if (!path.startsWith("/")) return null;
  try {
    const resolved = await realpath(path);
    return (await stat(resolved)).isDirectory() ? resolved : null;
  } catch {
    return null;
  }
}

/** Fleet's answer, read: the repository it now serves, its refusal in its own words, or the failure. */
export function locateAnswerOf(answer: Answer): LocateAnswer {
  if (answer.ok) return { state: "located", repository: answer.body as RepositorySummary };
  const { outcome } = answer;
  if (!outcome.ok && outcome.why === "refused") {
    return { state: "refused", code: outcome.error.code, saying: outcome.error.message };
  }
  return { state: "failed", outcome };
}
