import type { Guide } from "./guide";

/**
 * What `Look now` is, and what a reading's age means. Pulse's headline used to
 * carry *Looking costs no model call* and its footer *a process can exit
 * between the reading and this screen* — two standing sentences beside the one
 * fact a reader came for.
 *
 * **No `docs/concepts/` page holds this.** The looks are `crates/ipc`'s
 * `Look`, and nothing under `docs/concepts/` writes them up; saying so here is
 * better than naming a page that does not carry it.
 */
export const GUIDE_LOOK: Guide = {
  number: 14,
  group: "machine",
  title: "Looking at the machine",
  piece: "pulse.look",
  body: [
    "A look asks Fleet to go and check this job on this machine: its process, its checkout, what " +
      "is being written, where the job has got to, and whether anything has gone quiet or is " +
      "repeating itself.",
    "It costs no model call. Nothing about the job moves and nothing is spent, so there is no " +
      "reason not to press it.",
    "Every figure here is one instant. A process can exit between the reading and this screen, " +
      "which is why the age is drawn under the tables rather than left to be assumed.",
    "A look answers working, not working, or could not tell. The third is its own answer and " +
      "never a softened pass — it means the checks came back short.",
  ],
};
