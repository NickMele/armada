import type { Guide } from "./guide";

/**
 * What `Models by tier` is, written once for both the surfaces that drew it —
 * the dispatch composer's settings and the proposal. `TierModels` carried two
 * sentences saying the same thing in different words, which was the drift a
 * second copy always is.
 *
 * **No `docs/concepts/` page holds this.** `docs/journeys/dispatch-a-job.md`
 * is the nearest, and a journey is not what a guide agrees with. Named here so
 * the gap is on the record rather than invisible.
 */
export const GUIDE_TIERS: Guide = {
  number: 7,
  group: "plan",
  title: "A tier picks the model",
  piece: "plan.tiers",
  body: [
    "The planning Drone marks every task it writes Difficult, Medium or Easy. That mark is the " +
      "tier, and it is the planner's reading of the work rather than anything you set per task.",
    "The map from tier to model is what you set, once, for the whole job. A task's own agent runs " +
      "on whatever its tier names.",
    "A tier left on Auto is Armada's to pick. It is not the same as picking the cheapest: the " +
      "harness chooses, and what it chooses can move between releases.",
  ],
};
