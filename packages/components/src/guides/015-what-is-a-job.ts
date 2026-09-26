import type { Guide } from "./guide";

/**
 * The first question anybody has, and the one the first fourteen guides could
 * not answer: they were written by lifting sentences off screens, so every one
 * of them started halfway through.
 *
 * Written from `docs/concepts/job.md` and `docs/concepts/drone.md` rather than
 * from a surface. Nothing here is drawn anywhere, which is why no `?` claims
 * `job.what` yet.
 */
export const GUIDE_JOB: Guide = {
  number: 15,
  group: "job",
  title: "What is a job?",
  piece: "job.what",
  concept: "docs/concepts/job.md",
  steps: [
    "A job is one piece of work you ask Armada for.",
    "It is a record rather than a worker.",
    "It holds the workflow it follows, where it has got to, and what it has produced.",
    "The work itself is the workflow's steps, run in order.",
    "A Drone does the work of one step and reports what it did.",
    "Fleet decides whether that report is true, and Fleet is the only thing that moves a job on.",
    "The job's checkout and its branch survive a step ending, so the next step starts where the " +
      "last one stopped.",
    "A job is done when its landing rule is met.",
  ],
  // Under the line that names the steps: the relation is a job running them in
  // order, which is the same drawing guide 19 leads with.
  figure: { id: "workflow-steps", at: 4 },
};
