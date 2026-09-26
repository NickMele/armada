// What a guide is, and the groups the catalogue files them under.
//
// **A guide is data in its own file, never a sentence in JSX.** The app shows
// facts; an explanation is something a person chooses to see (the owner, 23
// September 2026, #1602), and it only stays choosable if it has a home that
// can be listed, numbered, diffed and read end to end.
//
// The same knowledge for a reader of the repository is in `docs/concepts/`.
// Every guide names the page it agrees with, so the two can be read against
// each other by hand; nothing generates one from the other.

/** The sections of the catalogue, as ids nothing types twice. */
export type GuideGroupId = "job" | "plan" | "run" | "workflow" | "machine";

export type GuideGroup = {
  id: GuideGroupId;
  /** Sentence case, and it names a part of the app rather than a topic. */
  title: string;
};

/**
 * The groups, in the order the catalogue draws them.
 *
 * **The order is a reading order**, not an alphabet: a person reading end to
 * end meets a job, then the plan inside it, then the run that works it, then
 * the picture of both, and last what it costs this machine.
 */
export const GUIDE_GROUPS: readonly GuideGroup[] = [
  { id: "job", title: "A job" },
  { id: "plan", title: "The plan" },
  { id: "run", title: "The run" },
  { id: "workflow", title: "The workflow" },
  { id: "machine", title: "This machine" },
];

/**
 * A drawing a guide can carry, by id rather than by path.
 *
 * **A figure is the real app moving**, so `GuideFigure` mounts the components
 * the screen mounts and animates them arriving. The owner, 26 September 2026:
 * *"I assumed the figure was an animation of the real app."* What that costs
 * is the point — a screen that changes breaks its own guide, loudly, where an
 * anonymous diagram would have drifted in silence.
 *
 * **A relation is drawn once**, `docs/contracts/design-system.md`, *Teaching
 * borrows a game's motion*. An id names one drawing; two guides may name the
 * same id, and it is scaled rather than redrawn.
 */
export type GuideFigureId = "members-landing" | "group-order" | "step-bar" | "workflow-steps";

/**
 * One guide: a number, a title, the piece it explains, the steps it reads as,
 * and room for a drawing under one of them.
 *
 * **The number is the guide's own and is never reused.** It is what a person
 * cites, so a retired guide leaves its number behind — 11 is retired and 11 is
 * gone. A new guide takes the next number above the highest in use, wherever
 * the catalogue files it, and a retitle never moves one.
 */
export type Guide = {
  number: number;
  group: GuideGroupId;
  /**
   * **A question a person would type**, ending in a question mark.
   *
   * The owner, 25 September 2026: a person arrives holding a question, and a
   * row that names a part of the app asks them to translate before they can
   * find it. The rule this replaces — *it names the thing; it never asks the
   * question* — was an agent's, with no decision of his behind it.
   */
  title: string;
  /**
   * The piece it explains, as a stable id. A `?` naming this opens this guide,
   * and the first time the piece is on a screen somebody is looking at, the
   * card opens once by itself. The id is what is remembered, so renaming one
   * gives every reader their first contact back.
   */
  piece: string;
  /**
   * The guide, as a numbered sequence. **One line each, never a paragraph** —
   * a step is a sentence somebody could say out loud.
   *
   * The owner, 25 September 2026: *"I would prefer guides be animations or
   * steps, not just a wall of text."* He took both together on the 26th: the
   * drawing over the steps, in one guide.
   */
  steps: readonly string[];
  /**
   * The drawing, under the step it belongs to.
   *
   * **Absent where there is no relation to draw**, and no frame is held open
   * where one would go: a guide whose subject is a rule has no honest picture.
   */
  figure?: {
    id: GuideFigureId;
    /** The step it sits under, by its own number. The relation belongs to one line. */
    at: number;
  };
  /** The `docs/concepts/` page holding the same knowledge for a reader of the repository. */
  concept?: string;
};
