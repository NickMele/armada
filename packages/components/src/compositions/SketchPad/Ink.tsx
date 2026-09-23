import { ViewportPortal, useReactFlow } from "@xyflow/react";
import { useCallback, useEffect, useRef, useState } from "react";

/**
 * The pen on the sketch pad: every line already drawn by hand, the one being
 * drawn, and the layer that catches the pointer while the pen is down.
 *
 * **It belongs to `SketchPad` and to no other canvas.** Hard rule six makes
 * `GraphCanvas` the extraction every graph draws on, so this adds nothing to
 * it — the whole thing mounts through the `children` seam `GraphCanvas`
 * already has, inside the flow where React Flow's hooks resolve.
 *
 * `docs/journeys/dispatch-a-job.md`, The sketch, has the two rules.
 */

/** One place on the pad, in the same coordinates a box's `x` and `y` are in. */
export type SketchPoint = { x: number; y: number };

/** One line drawn by hand, kept as the line rather than as a picture of it. */
export type SketchStroke = { id: string; points: readonly SketchPoint[] };

/**
 * What one stroke is, for somebody who cannot see it. Never "a line": a join
 * between two boxes is already read as one, and the two are different things.
 */
export const A_HAND_LINE = "Drawn by hand";

/**
 * How far the pointer travels before a stroke keeps another point, in pad
 * coordinates. A pointer reports every frame, so an unthinned five-second line
 * is hundreds of points in the draft for a shape three dozen would draw.
 */
const A_STEP = 3;

/** Fewer points than this is a press with no travel, not a line. */
const A_LINE_TAKES = 2;

/** The point kept, or the run unchanged where the pointer has barely moved. */
function further(points: readonly SketchPoint[], point: SketchPoint): readonly SketchPoint[] {
  const last = points[points.length - 1];
  if (last !== undefined && Math.hypot(point.x - last.x, point.y - last.y) < A_STEP) {
    return points;
  }
  return [...points, point];
}

/**
 * The points as one path, curved through their midpoints rather than joined
 * corner to corner — a thinned freehand line drawn as straight segments reads
 * as a saw, which is not what the hand did.
 */
function pathOf(points: readonly SketchPoint[]): string {
  const first = points[0];
  if (first === undefined) return "";
  let d = `M ${String(first.x)} ${String(first.y)}`;
  for (let at = 1; at < points.length - 1; at += 1) {
    const here = points[at]!;
    const next = points[at + 1]!;
    const mid = { x: (here.x + next.x) / 2, y: (here.y + next.y) / 2 };
    d += ` Q ${String(here.x)} ${String(here.y)} ${String(mid.x)} ${String(mid.y)}`;
  }
  const last = points[points.length - 1]!;
  return points.length === 1 ? d : `${d} L ${String(last.x)} ${String(last.y)}`;
}

export function Ink({
  strokes,
  pen,
  onDraw,
}: {
  strokes: readonly SketchStroke[];
  pen: boolean;
  onDraw: (points: readonly SketchPoint[]) => void;
}) {
  const flow = useReactFlow();
  const going = useRef<readonly SketchPoint[] | null>(null);
  const stop = useRef<(() => void) | null>(null);
  const [line, setLine] = useState<readonly SketchPoint[]>([]);

  const at = useCallback(
    (event: { clientX: number; clientY: number }): SketchPoint => {
      const point = flow.screenToFlowPosition({ x: event.clientX, y: event.clientY });
      return { x: Math.round(point.x), y: Math.round(point.y) };
    },
    [flow],
  );

  const drop = useCallback(() => {
    stop.current?.();
    stop.current = null;
    going.current = null;
    setLine([]);
  }, []);

  /**
   * The rest of the stroke, watched from the press itself rather than from an
   * effect the press schedules — a pointer that has already moved by the time
   * React commits would lose those points, and the whole line where every move
   * lands in one task. The window rather than the layer, so a line drawn off
   * the edge of the pad ends where the pointer lifted instead of hanging open.
   */
  const begin = (first: SketchPoint) => {
    stop.current?.();
    going.current = [first];
    setLine(going.current);
    const moved = (event: PointerEvent) => {
      const next = further(going.current ?? [], at(event));
      going.current = next;
      setLine(next);
    };
    const lifted = () => {
      const drawn = going.current;
      drop();
      if (drawn !== null && drawn.length >= A_LINE_TAKES) onDraw(drawn);
    };
    window.addEventListener("pointermove", moved);
    window.addEventListener("pointerup", lifted);
    window.addEventListener("pointercancel", lifted);
    stop.current = () => {
      window.removeEventListener("pointermove", moved);
      window.removeEventListener("pointerup", lifted);
      window.removeEventListener("pointercancel", lifted);
    };
  };

  // The pen going away mid-line drops it rather than reporting half a stroke,
  // and so does the pad going away — the listeners are the window's.
  useEffect(() => {
    if (!pen) drop();
    return drop;
  }, [pen, drop]);

  return (
    <>
      {/* `ViewportPortal` puts the ink in React Flow's own transformed
          viewport, which is what makes a stroke pan and zoom with the box it
          was drawn beside rather than float over it. */}
      <ViewportPortal>
        {/* One pixel and `overflow: visible`: the paths carry pad coordinates,
            which are negative as often as not, so the box is an origin rather
            than a frame. */}
        <svg className="armada-sketch-pad__ink" width="1" height="1">
          {strokes.map((stroke) => (
            <path key={stroke.id} d={pathOf(stroke.points)} role="img" aria-label={A_HAND_LINE} />
          ))}
          {line.length < A_LINE_TAKES ? null : <path d={pathOf(line)} aria-hidden />}
        </svg>
      </ViewportPortal>
      {pen ? (
        <div
          className="armada-sketch-pad__pen"
          onPointerDown={(event) => {
            if (event.button === 0) begin(at(event));
          }}
        />
      ) : null}
    </>
  );
}
