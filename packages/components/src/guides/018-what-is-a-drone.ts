import type { Guide } from "./guide";

/**
 * What the word means, for a person who has met it on a badge and nowhere
 * else. Written from `docs/concepts/drone.md`.
 *
 * The four recovery causes that end a Drone early are left out: they are what
 * a person does to a job that stopped, and this guide is about one that has
 * not.
 */
export const GUIDE_DRONE: Guide = {
  number: 18,
  group: "run",
  title: "What is a drone?",
  piece: "run.drone",
  concept: "docs/concepts/drone.md",
  steps: [
    "A Drone is one agent working one step of one job.",
    "It runs in the job's own checkout, with the tools Armada gives it.",
    "A fresh Drone starts when a step starts.",
    "It ends when that step's work has passed everything a machine can ask of it.",
    "A step waiting on a person holds no Drone. The work is done and the wait is yours.",
    "Nothing of the session crosses a step boundary.",
    "The checkout, the branch and the uncommitted work survive. The transcript and the context " +
      "do not.",
    "So whatever the next Drone needs is something that was written down.",
    "A Drone moves nothing on its own. It reports, and Fleet decides.",
  ],
};
