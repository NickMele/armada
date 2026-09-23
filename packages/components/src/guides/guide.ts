// What a guide is, and the groups the catalogue files them under.
//
// **A guide is data in its own file, never a sentence in JSX.** The app shows
// facts; an explanation is something a person chooses to see, and it only
// stays choosable if it has a home that can be listed, numbered, diffed and
// read end to end — `.claude/decisions/2026-09-23-facts-on-screen-guides-behind-a-mark.md`.
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
};
