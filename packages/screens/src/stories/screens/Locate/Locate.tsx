// Locate in the window Bridge draws it in: the rail's control, the dialog, and Setup for what it
// located. `SetupFrom` is the window; this names the story's own subject.

import { useState } from "react";
import type { RepositorySummary } from "@armada/protocol";

import { SetupFrom } from "../Setup/Setup";

export { CHOSEN } from "../Setup/Setup";

export const LocateFrom = SetupFrom;

/** Two windows on one main: a clone either sends is published to both, as main's `located` is. */
export function TwoWindowsFrom(props: Parameters<typeof SetupFrom>[0]) {
  const [landed, setLanded] = useState<{ repository: RepositorySummary; at: number } | null>(null);
  const hear = (repository: RepositorySummary) => setLanded({ repository, at: Date.now() });
  return (
    <div style={{ display: "flex" }}>
      <section aria-label="The window that asked" style={{ flex: 1, minWidth: 0 }}>
        <SetupFrom {...props} landed={landed} onLanded={hear} />
      </section>
      <section aria-label="Another window" style={{ flex: 1, minWidth: 0 }}>
        <SetupFrom {...props} landed={landed} onLanded={hear} />
      </section>
    </div>
  );
}
