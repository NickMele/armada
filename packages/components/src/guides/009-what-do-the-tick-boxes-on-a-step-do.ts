import type { Guide } from "./guide";

/**
 * `FLEET_ALWAYS_LOOKS`, which stood over the gate boxes on every approval
 * screen. It is the only place on a surface that said the two readings are
 * Fleet's own and not a tier anybody ticks — `crates/fleet/src/gate.rs`, and
 * `docs/concepts/judge.md` for a reader of the repository.
 */
export const GUIDE_ALWAYS_LOOKS: Guide = {
  number: 9,
  group: "run",
  title: "What do the tick boxes on a step do?",
  piece: "run.always-looks",
  concept: "docs/concepts/judge.md",
  steps: [
    "The boxes on a step choose its gate.",
    "Whether Checks have to pass, whether a Judge reads it, whether a person answers.",
    "They are the whole of what you are choosing.",
    "Two readings run whatever is ticked.",
    "Fleet checks that the work stayed inside what the plan declared.",
    "Fleet looks for a Check that was gamed: a test deleted, a skip added, a command narrowed " +
      "so the gate still passes over less.",
    "Neither is a tier and neither can be turned off.",
    "A step with nothing ticked is still read for work that went outside the plan.",
    "A tick moves the gate and never what the step declares.",
    "Asking for Checks on a step whose workflow declares none runs nothing, and the screen says so.",
  ],
};
