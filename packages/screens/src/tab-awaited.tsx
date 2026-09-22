// The two destinations the strip names before anything draws them.
//
// **Named rather than hidden.** A strip that grew tabs as issues landed would
// change shape under a reader between releases, and the order a tab sits in is
// what a hand learns. Each says what it will hold and where that reading is
// today, which is the one thing a person on it needs.

import type { DetailTab } from "./detail-tabs";
import { TAB_LABEL } from "./detail-tabs";

/** What each awaited tab says, and where its reading is in the meantime. */
const AWAITED: Partial<Record<DetailTab, string>> = {
  workflow: "The workflow canvas is not built. The run reads as a tree on Overview.",
  plan: "The plan board is not built. The plan reads beside the run on Overview.",
};

export function AwaitedTab({ tab }: { tab: DetailTab }) {
  return (
    <div className="armada-detail-tab" role="tabpanel" aria-label={TAB_LABEL[tab]}>
      <p className="armada-inside__absent" role="note">
        {AWAITED[tab]}
      </p>
    </div>
  );
}
