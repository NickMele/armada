import type { Guide } from "./guide";

/**
 * Everything the dispatch composer's Settings block and the proposal put in
 * front of a person, gathered in one place. Read off
 * `DispatchSettings.DispatchSettingsValue`, `ProposalLanding`, `ProposalGates`
 * and `ProposalDoneWhen`, so a control added to any of them makes this stale.
 */
export const GUIDE_BEFORE_A_RUN: Guide = {
  number: 16,
  group: "job",
  title: "What can I change before a job runs?",
  piece: "job.before",
  concept: "docs/concepts/job-proposer.md",
  steps: [
    "Everything on the proposal is yours until you approve it.",
    "The name, and the workflow the job runs.",
    "Where the work lands, what it starts from, and whether its pull request is offered or " +
      "parked as a draft.",
    "What counts as finished, which is the job's landing rule.",
    "The criteria the Judge is held to.",
    "Each step's gate: whether Checks have to pass, whether a Judge reads it, whether you are asked.",
    "Which model each tier runs on, and how many Drones this job may run at once.",
    "Anything you leave alone is decided for you, and the field says who decides it.",
    "Approving freezes all of it, and the job is held to what was on the screen at that moment.",
    "Nothing has started until you approve, so there is nothing to undo.",
  ],
};
