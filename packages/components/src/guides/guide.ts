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

/** A picture beside the words. Absent until one is drawn — no frame is held for a missing one. */
export type GuidePicture = {
  /** Resolved by the bundler, so a path that does not exist fails the build. */
  src: string;
  /** What the picture shows, for somebody who cannot see it. Never "diagram". */
  alt: string;
};

/**
 * A drawing a guide can lead with or carry, by id rather than by path: it is
 * built from tokens in `GuideFigure`, not a file the bundler resolves.
 *
 * **A relation is drawn once.** `docs/contracts/design-system.md`, *Teaching
 * borrows a game's motion* — three animations of one relation, drawn three
 * ways, is worse than none. So an id names one drawing, and both shapes below
 * scale that same drawing rather than each having their own.
 */
export type GuideFigureId = "members-landing" | "completion-rules";

/**
 * The same guide written as something other than paragraphs — a **prototype**,
 * so the owner can look at three shapes of one guide and choose one.
 *
 * The note this answers, 25 September 2026: *"I would prefer guides be
 * animations or steps, not just a wall of text."* Both shapes here are real and
 * both are switchable in the mock; the one he does not choose gets deleted,
 * along with this field.
 *
 * **Absent means prose under every shape**, which is why it is an added field
 * rather than a change to `body`: twelve of the fourteen guides are untouched,
 * and the contrast between a rewritten guide and an untouched one is part of
 * what is being looked at.
 */
export type GuideShapes = {
  /**
   * `steps`: a numbered sequence, **one line each and never a paragraph**.
   *
   * The figure is present only where a relation is the thing being learned. A
   * guide with no relation to draw is the lines alone — no figure, and no frame
   * held open where one would go.
   */
  steps: {
    lines: readonly string[];
    figure?: {
      id: GuideFigureId;
      /** The step it sits under, by its own number. The relation belongs to one line. */
      at: number;
    };
  };
  /**
   * `figure`: the drawing leads, at size, and the words are its captions
   * underneath — the No Man's Sky reading.
   *
   * **A figure is mandatory here, which is the shape's cost.** A guide whose
   * subject is a rule rather than a relation has no honest picture, and this
   * shape still makes it lead with one.
   */
  figure: {
    id: GuideFigureId;
    /** Under the drawing, subordinate to it. Each carries a fact `body` carries. */
    captions: readonly string[];
  };
};

/**
 * One guide: a number, a title, the piece it explains, a few short paragraphs,
 * and room for a picture.
 *
 * **The number is the guide's own and is never reused.** It is what a person
 * cites and what the catalogue counts by, so a guide that is retired leaves
 * its number behind rather than handing it to the next one.
 */
export type Guide = {
  number: number;
  group: GuideGroupId;
  /** Sentence case. It names the thing; it never asks the question. */
  title: string;
  /**
   * The piece it explains, as a stable id. A `?` naming this opens this guide,
   * and the first time the piece is on a screen somebody is looking at, the
   * card opens once by itself. The id is what is remembered, so renaming one
   * gives every reader their first contact back.
   */
  piece: string;
  /** Short paragraphs. Each stands on its own, so a reader can stop after one. */
  body: readonly string[];
  picture?: GuidePicture;
  /** The `docs/concepts/` page holding the same knowledge for a reader of the repository. */
  concept?: string;
  /**
   * The same guide as steps and as a figure. **A prototype for one decision**,
   * carried by two of the fourteen; the rest read as prose whatever the mock's
   * `?guides=` says.
   */
  shapes?: GuideShapes;
};
