// Which arrangement a Job's two graph destinations open in, remembered across
// a restart — Workflow's canvas or stacked run (`#1539`), and Plan's graph or
// list (owner, 25 Sep 2026).
//
// **One module, because it is one mechanism.** Plan's toggle was asked for
// after Workflow already had one, and a second way of remembering a way of
// reading is two places to keep in step.
//
// **`localStorage`, not a Fleet preference**, for `panel-open.ts`' own reason:
// Fleet's preference set is closed (`fleet.unknown_preference`) and a SQL
// `CHECK` constraint names every legal key, so a new one is a wire change in
// five crates. These are ways of reading, not facts about the Job.

import { useState } from "react";
import { planViewNamed, workflowViewNamed, type PlanView, type WorkflowView } from "@armada/screens";

const WORKFLOW_KEY = "armada.bridge.workflow-view";
const PLAN_KEY = "armada.bridge.plan-view";

/**
 * A remembered arrangement and the press that moves it, on whatever the
 * package's own reader makes of what was stored.
 *
 * **A failed read is the default, not a throw.** `localStorage` is denied
 * outright in some window configurations, and a preference is the last thing
 * that should stop a window drawing.
 */
function remembered<T extends string>(key: string, named: (value: string | null) => T): [T, (view: T) => void] {
  const read = (): T => {
    try {
      return named(window.localStorage.getItem(key));
    } catch {
      return named(null);
    }
  };
  const [view, setView] = useState(read);

  function press(next: T): void {
    setView(next);
    try {
      window.localStorage.setItem(key, next);
    } catch {
      // A failed write leaves the choice unremembered, the honest answer for a preference.
    }
  }

  return [view, press];
}

/** Canvas or stacked on Workflow. Canvas where nothing is stored. */
export function useWorkflowView(): [WorkflowView, (view: WorkflowView) => void] {
  return remembered(WORKFLOW_KEY, workflowViewNamed);
}

/** Graph or list on Plan. Graph where nothing is stored. */
export function usePlanView(): [PlanView, (view: PlanView) => void] {
  return remembered(PLAN_KEY, planViewNamed);
}
