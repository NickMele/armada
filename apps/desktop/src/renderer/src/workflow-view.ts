// Which arrangement the Workflow tab opens in, remembered across a restart —
// `#1539`'s "remembered per viewer".
//
// **`localStorage`, not a Fleet preference**, for `panel-open.ts`' own reason:
// Fleet's preference set is closed (`fleet.unknown_preference`) and a SQL
// `CHECK` constraint names every legal key, so a new one is a wire change in
// five crates. This is a way of reading, not a fact about the Job.

import { useState } from "react";
import { workflowViewNamed, type WorkflowView } from "@armada/screens";

const KEY = "armada.bridge.workflow-view";

function read(): WorkflowView {
  try {
    return workflowViewNamed(window.localStorage.getItem(KEY));
  } catch {
    return workflowViewNamed(null);
  }
}

/** The remembered arrangement, and the press that moves it. Canvas where nothing is stored. */
export function useWorkflowView(): [WorkflowView, (view: WorkflowView) => void] {
  const [view, setView] = useState(read);

  function press(next: WorkflowView): void {
    setView(next);
    try {
      window.localStorage.setItem(KEY, next);
    } catch {
      // A failed write leaves the choice unremembered, the honest answer for a preference.
    }
  }

  return [view, press];
}
