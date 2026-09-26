// Which arrangement the Plan tab is in, and the one place its two words are
// spelled.
//
// **Graph by default** (owner, 25 Sep 2026). He asked the plan for views of
// its own after reading it as a list, and the graph is the one that came off
// Workflow — landing on the list would leave it where nobody would find it.
//
// **Remembered per viewer, not per Job**, on `workflow-view.ts`'s terms: it is
// a way of reading, so it does not reset when the next Job opens, and where it
// is kept is the caller's — `packages/screens` holds no storage.
//
// **Two views, not three.** The repository diagram the same note asked for is
// not designed, and a disabled third tab would be a promise the screen cannot
// keep.

/** The two arrangements of one plan. */
export const PLAN_VIEWS = ["graph", "list"] as const;

export type PlanView = (typeof PLAN_VIEWS)[number];

/** What a person lands on, whatever the width. */
export const FIRST_PLAN_VIEW: PlanView = "graph";

/** One spelling of each, for the toggle and for anything that has to say which. */
export const PLAN_VIEW_LABEL: Record<PlanView, string> = {
  graph: "Graph",
  list: "List",
};

/** A stored or remembered value read back as one of the two, or the default. */
export function planViewNamed(value: string | null | undefined): PlanView {
  return PLAN_VIEWS.find((view) => view === value) ?? FIRST_PLAN_VIEW;
}
