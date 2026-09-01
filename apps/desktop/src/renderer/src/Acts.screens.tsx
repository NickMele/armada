// The Job header, at every state that draws one, rendered from the app's own
// component rather than from a picture of it.
//
// **This exists because the gallery was not the app.** `pnpm shoot` captured a
// side it called "app", and that side was `packages/components`' gallery, whose
// `Screens/Inside a job` hand-builds its header out of four `<Button>`s. A
// change to `Acts.tsx` could not move it, so the header could ship rebuilt and
// the shot would show the old one, green. Every screen here imports the thing
// it is a screen of.
//
// **Fixtures, not a running Fleet.** These are the values the wire would carry,
// written down: enough of a `JobSummary` and a `JobDetail` for `recourseOf` to
// answer, and nothing else. A screen that needed a daemon would be a screen
// nobody captures.
//
// # Marks are the pairing, and they are chosen here
//
// `data-shot` is what pairs a capture against a drawing's frame. The gallery
// derives its marks from story export names; this file states them, because
// there is no export name to derive from and a mark a drawing has to match is
// not something to leave to a transform.
//
// # What a screen is allowed to be
//
// One composition at one state, and no arrangement of its own. The callbacks
// are inert — nothing here is pressed — and a screen that wired one would be
// asserting behaviour in a file whose whole output is a PNG.

import type { ReactElement } from "react";

import type { JobDetail as JobWhole, JobSummary } from "../../shared/protocol";
import { Acts } from "./Acts";
import type { Render } from "./render";

/** One capture: the mark it pairs by, what to call it, and what to draw. */
export type Screen = { mark: string; name: string; element: ReactElement };

/**
 * The fields every fixture shares, and the ones no header reads.
 *
 * **Spread rather than defaulted in a helper**, so a screen states the two or
 * three values it is actually about and a reader can see them without opening
 * anything. The rest is the wire's required shape and is noise on this screen.
 */
const JOB: JobSummary = {
  id: "01M1D80SFQ0042MT27VBCXXWHX",
  title: "Clear finished jobs should remove the worktree it left behind",
  status: "escalated",
  workflow_id: "bug",
  owner_manifest_id: "armada",
  origin: "person",
  urgency: "normal",
  atomic: false,
  model: "sonnet",
};

/**
 * A detail carrying one classification and nothing else.
 *
 * `Acts` reads `stuck` through `recourseOf` and reads nothing else on the
 * whole, so the arrays are empty rather than filled with plausible steps: a
 * fixture that carried a run would be claiming this screen depends on one.
 */
function detail(recourse: string[]): JobWhole {
  return {
    job: JOB,
    created_at: "2026-09-01T09:16:40Z",
    steps: [],
    acceptance_criteria: [],
    dependencies: [],
    stuck: { recourse },
  };
}

/** Every callback the header takes, inert. Nothing here is pressed. */
const INERT = {
  acting: false,
  approving: false,
  stale: false,
  reporting: false,
  onAct: () => {},
  onApprove: () => {},
  onReporting: () => {},
  onCopied: () => {},
  onReport: async () => ({ ok: true }) as never,
};

function header(job: JobSummary, whole: JobWhole | null, render: Render): ReactElement {
  return <Acts job={job} whole={whole} render={render} {...INERT} />;
}

export const title = "Job header";

export const screens: Screen[] = [
  {
    mark: "job-header-evidence-disputed",
    name: "Evidence disputed",
    // Waiting on a person, so the control takes the accent, and Fleet offers a
    // replacement — which is the lead the drawing names for this state.
    element: header(JOB, detail(["redispatch_job"]), "stopped"),
  },
  {
    mark: "job-header-escalated-holding",
    name: "Escalated, drone still on it",
    // The kill of a live Drone joins the menu beside the kill of the Job. Both
    // are behind the caret: the lead is never destructive.
    element: header(
      { ...JOB, assigned_drone: "drone_2d90bb" },
      detail(["redispatch_job", "redirect_drone"]),
      "stopped",
    ),
  },
  {
    mark: "job-header-killed",
    name: "Killed",
    // Terminal, so nobody is waiting and the same control is quiet. The fill is
    // the state and not the act — the lead has not changed.
    element: header({ ...JOB, status: "killed" }, detail(["redispatch_job"]), "stopped"),
  },
  {
    mark: "job-header-failed-no-replacement",
    name: "Failed, no replacement offered",
    // A split button with nothing in its menu is a button. Fleet offers no
    // redispatch, the Job is over, and the report is all that is left.
    element: header({ ...JOB, status: "completed_failed" }, detail([]), "stopped"),
  },
  {
    mark: "job-header-running",
    name: "Running",
    // The render the split button has no legal lead on: the drawing leads this
    // one with Pilot, which is #250. Captured so that the day Pilot lands, the
    // shot before it is on record.
    element: header(
      { ...JOB, status: "running", assigned_drone: "drone_2d90bb" },
      null,
      "working",
    ),
  },
];
