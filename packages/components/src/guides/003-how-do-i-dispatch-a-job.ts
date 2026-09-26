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
  title: "How do I dispatch a job?",
  piece: "dispatch.what-happens",
  concept: "docs/concepts/job-proposer.md",
  steps: [
    "Describe the work and press Dispatch.",
    "Armada reads the request, picks the workflow and names the job.",
    "Nothing runs on it yet.",
    "What comes back is a proposal: the workflow, the name and the split.",
    "No file is named at that point, because scope is the workflow's first step.",
    "Approving the proposal is what starts the work.",
    "Where one request becomes several jobs, each is approved on its own after the one before " +
      "it completes.",
    "The workflow freezes into the job when it is created, and the work is judged against it.",
    "A request no workflow fits is refused, and the request is left as you wrote it.",
  ],
};
