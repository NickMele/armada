// A picture a person attached, and where it came from. Draft, for
// `crates/ipc/src/capturing.rs`.
//
// Source of truth today: `StagedFrame` — `staged_path`, `width`, `height`, "the
// PNG Bridge took, written to disk before the request" — and the `produced_by`
// that rides beside it on `CaptureStudioNote`, naming the Studio node the
// capture was made from.
//
// **Provenance is the point, and it follows `StagedFrame` rather than
// inventing a shape** (#1532, 22 Sep): a picture plus the node that produced
// it. A sketch with no provenance is an image nobody can trace to what it was
// about.

import type { CaptureStudioNote, StagedFrame } from "@armada/protocol";

/** A picture attached to a prompt, with what produced it. */
export type SketchAttachment = {
  /** Where Bridge wrote the PNG before the request. **Never read back.** */
  staged_path: string;
  width: number;
  height: number;
  /**
   * The Studio node the picture was made from, where there was one. **Absent
   * is a sketch drawn from nothing** — a person opening a blank canvas — which
   * is a real answer and not a missing link.
   */
  produced_by?: string;
  /** What the person said about it. Empty where they said nothing. */
  said: string;
};

/**
 * The sketch on a Studio note, where it carries one.
 *
 * **`undefined` is a note with no picture**, which is most of them: a note is
 * words and a pointer, and the frame is optional on the wire for that reason.
 */
export function sketchOf(note: CaptureStudioNote): SketchAttachment | undefined {
  if (note.frame === undefined) {
    return undefined;
  }
  return sketchFromFrame(note.frame, note.said, note.produced_by);
}

/** The same shape from a frame Bridge just staged, before any note exists. */
export function sketchFromFrame(
  frame: StagedFrame,
  said: string,
  producedBy?: string,
): SketchAttachment {
  const sketch: SketchAttachment = {
    staged_path: frame.staged_path,
    width: frame.width,
    height: frame.height,
    said,
  };
  if (producedBy !== undefined) sketch.produced_by = producedBy;
  return sketch;
}
