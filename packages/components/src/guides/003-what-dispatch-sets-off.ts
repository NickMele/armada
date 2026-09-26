import type { Guide } from "./guide";

/**
 * The composer's own three-line *What happens next*, and the two sentences
 * under the proposal about what approving does (`#1540`). All of it was
 * standing copy: true before anything is typed, and still true of a job that
 * never ran.
 */
export const GUIDE_DISPATCH: Guide = {
  number: 3,
  group: "job",
  title: "What dispatch sets off",
  piece: "dispatch.what-happens",
  concept: "docs/concepts/job-proposer.md",
  body: [
    "Dispatching sends the request to Armada, which reads it, picks the workflow and names the " +
      "job. Nothing runs on it yet.",
    "What comes back is a proposal. The workflow, the name and the split are what you approve; no " +
      "file is named at that point, because scope is the workflow's first step. Approving is what " +
      "starts the work.",
    "Where one request becomes several jobs, each is approved on its own after the one before it " +
      "completes, so nothing starts until you approve the first.",
    "The workflow is frozen into the job when it is created and becomes what the work is judged " +
      "against. Nothing is assigned by default — a request no workflow fits is refused, and the " +
      "request is left as you wrote it.",
  ],
};
