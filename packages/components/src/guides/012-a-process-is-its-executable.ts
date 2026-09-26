import type { Guide } from "./guide";

/**
 * Why the process list is three words wide. `packages/protocol/src/resources.ts`
 * carries the same rule for a reader of the wire; this is it for a person
 * looking at Pulse and wondering where the command line went.
 */
export const GUIDE_PROCESSES: Guide = {
  number: 12,
  group: "machine",
  title: "A process is named by its executable",
  piece: "pulse.processes",
  concept: "docs/concepts/job.md",
  body: [
    "Pulse lists what this job is running on this machine right now, each row named by the " +
      "executable alone: node, cargo, git.",
    "The arguments are left out on purpose. They carry absolute paths, a repository layout and " +
      "whatever a Check was invoked with, and none of that answers what the process is.",
    "At most one row is the process Fleet wrote down. Fleet believing something is running that " +
      "is not is the reading worth looking for, and the row says so in its own words.",
  ],
};
