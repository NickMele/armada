// `helmActionAuthorityValue` alone — `#1127`. `BridgeSettings.tsx` draws it into the "This
// machine" row; this pins the fold from `HealthRead`'s four states to the string, or its absence.

import { describe, expect, it } from "vitest";

import type { FleetHealth } from "@armada/protocol";
import { helmActionAuthorityValue } from "./BridgeSettings";
import type { HealthRead } from "./overview-reads";

const health = (helm_action_authority: FleetHealth["helm_action_authority"]): FleetHealth => ({
  probes: [{ module: "Fleet", outcome: "pass", detail: "answering" }],
  not_probed: [],
  helm_action_authority,
});

describe("helmActionAuthorityValue", () => {
  it("reads Fleet's resolved value once health has answered", () => {
    expect(helmActionAuthorityValue({ state: "read", health: health("acting") })).toBe("Acting");
    expect(helmActionAuthorityValue({ state: "read", health: health("read_only") })).toBe("Read-only");
  });

  it.each<HealthRead>([{ state: "none" }, { state: "reading" }, { state: "failed", outcome: { ok: false, why: "not_connected" } }])(
    "is absent while health has not been read — %o",
    (read) => {
      expect(helmActionAuthorityValue(read)).toBeUndefined();
    },
  );
});
