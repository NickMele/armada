// Locate: a repository added by its folder or cloned from a URL, then listed and picked so the
// window lands on Setup for it. `docs/journeys/set-up-a-project-manifest.md`, *Getting in*.

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
  /** Read what Fleet serves again, so the pick below finds the new root listed. */
  list: (port: number) => Promise<void>;
  pick: (root: string) => Promise<void>;
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
    return this.send("/repositories/clone", body, CLONE_MS);
  }

  private async send(path: string, body: unknown, waitMs: number): Promise<LocateAnswer> {
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
      await this.wiring.pick(answer.repository.root);
      return answer;
    } finally {
      this.out = false;
    }
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
