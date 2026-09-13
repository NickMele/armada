// New job's own reads for the repository its ask answered, while the pick stays on All — #959.
// No React, so main imports the answer's shape, on `locate-reads.ts`'s terms.

import type { LeftOutWorkflow, ManifestReading, Outcome } from "@armada/protocol";

/**
 * Why some workflows do not fit the answered repository's own configuration,
 * and what Fleet's last read of its `armada.yml` came to. Both taken by root
 * rather than by the pick, which never moves to name the repository the ask
 * answered — `Picked.manifestOf` is what names it instead.
 */
export type ComposingRead =
  | { ok: true; leftOut: LeftOutWorkflow[]; reading: ManifestReading | null }
  | { ok: false; outcome: Outcome };
