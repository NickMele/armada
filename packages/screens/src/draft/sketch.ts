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

/**
 * One box on a sketch: where a person put it, and the words in it.
 *
 * **A Studio's `Sketch` node is `{ body: String }`** (`crates/ipc/src/studio.rs`),
 * and this is that body with a place. Nothing here carries a size — a box is
 * drawn at one width and grows down the page with its words, so a height in
 * the draft would be a measurement Bridge took and a person never chose.
 */
export type SketchShape = { id: string; x: number; y: number; body: string };

/** One box joined to another, in the direction a person drew it. */
export type SketchJoin = { id: string; from: string; to: string };

/** One place on the pad, in the same coordinates a box's `x` and `y` are in. */
export type SketchPoint = { x: number; y: number };

/**
 * One line drawn freehand, **kept as the line rather than as a picture of it**.
 *
 * A box says what a thing is and a join says what feeds it; a stroke is what
 * neither can say — a circle round the part that matters, an arrow at an angle
 * no join draws, a shape that is not a box. The owner asked for it on
 * 23 Sep 2026, on a pad that only drew boxes.
 *
 * **Points, never a path string or an image.** The pad has to redraw a stroke
 * when a person comes back to it, take the last one back, and scale the whole
 * picture when they zoom — all three read the points. A flattened stroke can
 * do none of them, which is the same argument `drawn` already makes against
 * reopening the PNG.
 */
export type SketchStroke = { id: string; points: readonly SketchPoint[] };

/**
 * What is on the pad while a person is still drawing on it.
 *
 * **Held apart from the attachment, because it is the half that never
 * crosses.** `SketchAttachment` is what goes out with the request; this is
 * what the canvas draws and what every edit below folds over.
 */
export type Drawing = {
  shapes: readonly SketchShape[];
  joins: readonly SketchJoin[];
  strokes: readonly SketchStroke[];
};

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
  /**
   * What the PNG was drawn from, so the canvas can reopen what a person made
   * rather than an image of it.
   *
   * **This half never crosses the wire, and `staged_path` is the half that
   * does** (#1547). A Drone reads the picture; a person coming back to the
   * composer reads their own boxes, and a flattened PNG cannot be edited. It
   * is absent on a sketch that arrived as a frame — `sketchFromFrame` is
   * given a capture, which has no shapes behind it.
   */
  drawn?: Drawing;
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

/** A pad with nothing on it. What Sketch opens on where no moment carries one. */
export const NOTHING_DRAWN: Drawing = { shapes: [], joins: [], strokes: [] };

/** What a sketch was drawn from, or an empty pad where it carries none. */
export function drawingOf(sketch: SketchAttachment | undefined): Drawing {
  return sketch?.drawn ?? NOTHING_DRAWN;
}

/**
 * Whether anything is on the pad. **What decides the chip** — an empty pad
 * attaches nothing.
 *
 * A stroke counts on its own: a pad somebody drew on and put no box on is a
 * picture, and a chip that appeared only for boxes would drop it silently.
 * A join still does not, because a join needs the two boxes it hangs on.
 */
export function isDrawn(drawing: Drawing): boolean {
  return drawing.shapes.length > 0 || drawing.strokes.length > 0;
}

/**
 * The id the next box takes: the lowest `b<n>` nothing on the pad holds.
 *
 * **Reused rather than always counting up**, so a person who adds and removes
 * all afternoon does not end on `b214`. Nothing outside the pad keys on these
 * — the picture is what goes out — so a reused id names nothing that moved.
 */
export function nextShapeId(drawing: Drawing): string {
  const held = new Set(drawing.shapes.map((shape) => shape.id));
  let at = 1;
  while (held.has(`b${String(at)}`)) at += 1;
  return `b${String(at)}`;
}

/** A box put down. An id already on the pad replaces the box holding it. */
export function withShape(drawing: Drawing, shape: SketchShape): Drawing {
  const held = drawing.shapes.some((one) => one.id === shape.id);
  return {
    ...drawing,
    shapes: held
      ? drawing.shapes.map((one) => (one.id === shape.id ? shape : one))
      : [...drawing.shapes, shape],
  };
}

/**
 * Boxes taken off, **and every join that hung on one of them**. A join to a
 * box that is gone draws as a line into nothing, which is the one thing a
 * picture must not do.
 */
export function withoutShapes(drawing: Drawing, ids: readonly string[]): Drawing {
  const going = new Set(ids);
  return {
    ...drawing,
    shapes: drawing.shapes.filter((shape) => !going.has(shape.id)),
    joins: drawing.joins.filter((join) => !going.has(join.from) && !going.has(join.to)),
  };
}

/**
 * The id the next stroke takes: the lowest `s<n>` nothing on the pad holds.
 *
 * **The same rule as a box's, and `s` rather than `b` so the two never
 * collide** — a person who draws and undoes all afternoon does not end on
 * `s214`, and nothing outside the pad keys on either.
 */
export function nextStrokeId(drawing: Drawing): string {
  const held = new Set(drawing.strokes.map((stroke) => stroke.id));
  let at = 1;
  while (held.has(`s${String(at)}`)) at += 1;
  return `s${String(at)}`;
}

/**
 * A line drawn, put on top of what is already there.
 *
 * **A stroke of fewer than two points is not a line**, and the pad never
 * reports one — a press with no travel is somebody clicking the pad, and a
 * stroke with nothing to draw would sit in the picture invisibly and take a
 * press of Undo to get rid of.
 */
export function withStroke(drawing: Drawing, stroke: SketchStroke): Drawing {
  if (stroke.points.length < 2) return drawing;
  return { ...drawing, strokes: [...drawing.strokes, stroke] };
}

/**
 * The last line drawn, taken back. **The first thing anybody does with a pen
 * is draw the wrong line**, so this is the act the pen is unusable without.
 *
 * It takes strokes and nothing else: a box comes off under Remove, where the
 * box going is the one a person picked and can see.
 */
export function withoutLastStroke(drawing: Drawing): Drawing {
  if (drawing.strokes.length === 0) return drawing;
  return { ...drawing, strokes: drawing.strokes.slice(0, -1) };
}

/**
 * Two boxes joined, once.
 *
 * **A pair already joined is left exactly as it was**, whichever way round it
 * was drawn — joining the same two again is a person pressing twice, not a
 * request to reverse the arrow.
 */
export function withJoin(drawing: Drawing, from: string, to: string): Drawing {
  if (from === to) return drawing;
  const already = drawing.joins.some(
    (join) =>
      (join.from === from && join.to === to) || (join.from === to && join.to === from),
  );
  if (already) return drawing;
  return { ...drawing, joins: [...drawing.joins, { id: `${from}-${to}`, from, to }] };
}

/** The words in one box, rewritten. A box the pad does not hold changes nothing. */
export function withBody(drawing: Drawing, id: string, body: string): Drawing {
  return { ...drawing, shapes: drawing.shapes.map((one) => (one.id === id ? { ...one, body } : one)) };
}

/** A box put down somewhere new. */
export function withPlace(drawing: Drawing, id: string, at: { x: number; y: number }): Drawing {
  return {
    ...drawing,
    shapes: drawing.shapes.map((one) => (one.id === id ? { ...one, x: at.x, y: at.y } : one)),
  };
}

/**
 * The sketch as it would go out, with what is on the pad kept beside the path.
 *
 * **The staged path is the caller's**, because staging is a write Bridge makes
 * and this module makes none. `width` and `height` are the PNG's, for the same
 * reason.
 */
export function sketchDrawn(
  staged: StagedFrame,
  said: string,
  drawing: Drawing,
  producedBy?: string,
): SketchAttachment {
  return { ...sketchFromFrame(staged, said, producedBy), drawn: drawing };
}
