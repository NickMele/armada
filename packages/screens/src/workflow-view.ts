// Which arrangement the Workflow tab is in, and the one place its two words
// are spelled.
//
// **Canvas by default, at every width** (owner, 21 and 22 Sep, #1530). The
// revised Narrow board drops the toggle; the toggle stays, and a narrow window
// opens the canvas on the step a person is on rather than on the whole run.
//
// **Remembered per viewer, not per Job.** It is a way of reading, so it does
// not reset when the next Job opens. Where it is kept is the caller's —
// `packages/screens` holds no storage.

/** The two arrangements of one run. */
export const WORKFLOW_VIEWS = ["canvas", "stacked"] as const;

export type WorkflowView = (typeof WORKFLOW_VIEWS)[number];

/** What a person lands on, whatever the width. */
export const FIRST_WORKFLOW_VIEW: WorkflowView = "canvas";

/** One spelling of each, for the toggle and for anything that has to say which. */
export const WORKFLOW_VIEW_LABEL: Record<WorkflowView, string> = {
  canvas: "Canvas",
  stacked: "Stacked",
};

/** A stored or remembered value read back as one of the two, or the default. */
export function workflowViewNamed(value: string | null | undefined): WorkflowView {
  return WORKFLOW_VIEWS.find((view) => view === value) ?? FIRST_WORKFLOW_VIEW;
}
