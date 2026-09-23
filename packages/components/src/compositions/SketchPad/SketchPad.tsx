import {
  Handle,
  MarkerType,
  ViewportPortal,
  applyNodeChanges,
  useReactFlow,
  type Edge,
  type FitViewOptions,
  type Node,
  type NodeChange,
  type NodeProps,
} from "@xyflow/react";
import { useCallback, useEffect, useMemo, useRef, useState } from "react";

import { Button } from "../../primitives/Button/Button";
import { Textarea } from "../../primitives/Textarea/Textarea";
import { GRAPH_CANVAS_SIDES, GraphCanvas, facingSides } from "../GraphCanvas/GraphCanvas";

/**
 * Sketch pad — boxes, the lines between them, and what a person draws by hand,
 * beside a prompt. `docs/journeys/dispatch-a-job.md`, The sketch.
 *
 * **What a prompt cannot say** (`#1547`): when what somebody means is a shape,
 * they type a paragraph describing a picture and the Drone reads the paragraph.
 *
 * **`GraphCanvas` is the graph half** (`#1539`) — no second canvas and no
 * drawing library. A box is words and a place, the way a Studio's `Sketch` node
 * is `{ body: String }`: no colour, no size, nothing to pick. The pen is this
 * surface's alone and no other canvas inherits it — see `Ink` below.
 *
 * **Nothing here stages anything.** What goes out is a PNG Bridge writes; every
 * edit is reported to the caller, so the drawing survives a switch to Write.
 */
export type SketchPadProps = {
  /** What the picture is, read to somebody who cannot see it. */
  label: string;
  boxes: readonly SketchBox[];
  lines: readonly SketchLine[];
  /** Everything drawn by hand, in the order it was drawn. */
  strokes: readonly SketchStroke[];
  /**
   * A box put down somewhere new, by pointer or by arrow key. Reported when it
   * lands, never while it travels.
   */
  onMove: (id: string, at: { x: number; y: number }) => void;
  /** The words in one box, as they are typed. */
  onBody: (id: string, body: string) => void;
  /** A box added, in the pad's own coordinates. **The caller mints the id.** */
  onAdd: (at: { x: number; y: number }) => void;
  /** Whatever is selected, taken off. Never called with nothing selected. */
  onRemove: (ids: readonly string[]) => void;
  /** Two boxes joined. Offered only while exactly two are selected. */
  onJoin: (from: string, to: string) => void;
  /**
   * A line drawn by hand, in the pad's own coordinates. **The caller mints the
   * id**, as it does for a box. Reported when the pointer lifts, never during.
   */
  onDraw: (points: readonly SketchPoint[]) => void;
  /**
   * The last line drawn, taken back. Never called with nothing drawn by hand.
   * It takes strokes and nothing else — a box comes off under Remove.
   */
  onUndo: () => void;
  /**
   * What a person said about the picture. **Beside the pad and not in a box**:
   * it is about the whole sketch, and a box holding it would be read as part
   * of the shape.
   */
  said: string;
  onSaid: (said: string) => void;
  /**
   * The Studio node this was made from. **Absent is a pad opened blank**, which
   * is a real answer (`draft/sketch.ts`), so it draws no line rather than an
   * empty one.
   */
  from?: string;
  /** Nothing may be drawn while the connection is not live. */
  disabled?: boolean;
};

/** One box: where a person put it, and the words in it. */
export type SketchBox = { id: string; x: number; y: number; body: string };

/** One box joined to another, in the direction it was drawn. */
export type SketchLine = { id: string; from: string; to: string };

/** One place on the pad, in the same coordinates a box's `x` and `y` are in. */
export type SketchPoint = { x: number; y: number };

/** One line drawn by hand, kept as the line rather than as a picture of it. */
export type SketchStroke = { id: string; points: readonly SketchPoint[] };

/** What the field beside the pad asks for. Never a Wh- opener. */
const SAID_LABEL = "About this sketch";

const SAID_PLACEHOLDER = "What the picture is meant to show.";

/** What an empty box is called, for somebody who cannot see it. */
const EMPTY_BOX = "An empty box";

/** A box's own field, which carries no visible label — the box is the label. */
const BOX_LABEL = "The words in this box";

/** What a box holds, shown rather than described. A phrase, never a paragraph. */
const BOX_PLACEHOLDER = "A panel, a read, a step";

/**
 * What one stroke is, for somebody who cannot see it. Never "a line": a join
 * between two boxes is already read as one, and the two are different things.
 */
const A_HAND_LINE = "Drawn by hand";

type PadNodeData = { body: string; onBody: (body: string) => void; disabled: boolean };
type PadNode = Node<PadNodeData, "sketch">;
type PadEdge = Edge<Record<string, never>, "default">;

/**
 * One box. The card is the grip and the well inside it is `nodrag`, so a drag
 * starting in the words selects text rather than moving the box out from under
 * the cursor.
 */
function BoxView({ data, selected }: NodeProps<PadNode>) {
  return (
    <>
      {GRAPH_CANVAS_SIDES.map((side) => (
        <Handle key={`t-${side}`} id={`t-${side}`} type="target" position={side} isConnectable={false} />
      ))}
      <div className="armada-sketch-box" aria-current={selected || undefined}>
        {/* `nodrag` is read by walking up from whatever the pointer hit, so the
            wrapper carries it and the whole well is exempt. */}
        <div className="nodrag">
          <Textarea
            rows={2}
            value={data.body}
            placeholder={BOX_PLACEHOLDER}
            disabled={data.disabled}
            aria-label={BOX_LABEL}
            onChange={(event) => data.onBody(event.target.value)}
          />
        </div>
      </div>
      {GRAPH_CANVAS_SIDES.map((side) => (
        <Handle key={`s-${side}`} id={`s-${side}`} type="source" position={side} isConnectable={false} />
      ))}
    </>
  );
}

const NODE_TYPES = { sketch: BoxView };
const EDGE_TYPES = {};

/**
 * Room around a fitted picture. **A tenth rather than React Flow's default**:
 * the acts sit over the top-right corner and the zoom pair over the
 * bottom-right, so a picture fitted edge to edge has a box under each of them.
 */
const FIT: FitViewOptions = { padding: 0.1 };

/**
 * Held down, a second box joins the selection rather than replacing it — which
 * is how a person picks the two they are joining. Named rather than left to
 * React Flow's platform read, for the two reasons `StudioWhiteboard` states.
 */
const JOINS_THE_SELECTION = ["Meta", "Control"];

/** How many boxes a join takes. Two, and the control says so while it is off. */
const A_JOIN_TAKES = 2;

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
    d += ` Q ${String(here.x)} ${String(here.y)} ${String((here.x + next.x) / 2)} ${String((here.y + next.y) / 2)}`;
  }
  const last = points[points.length - 1]!;
  return points.length === 1 ? d : `${d} L ${String(last.x)} ${String(last.y)}`;
}

/**
 * The ink: every stroke already drawn, the one being drawn, and — while the
 * pen is down — the layer that catches the pointer.
 *
 * **Mounted inside the flow through `GraphCanvas`'s `children`, so nothing was
 * added to the shared canvas.** `ViewportPortal` puts the ink in React Flow's
 * own transformed viewport, which is what makes a stroke pan and zoom with the
 * box it was drawn beside. The catcher sits over the pane and under React
 * Flow's panels, so the acts and the zoom pair stay pressable with the pen on.
 */
function Ink({
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

/**
 * The acts on the pad, drawn over its top-right corner.
 *
 * **A component rather than markup**, because where a new box lands is read off
 * the viewport React Flow holds, and that hook only resolves inside the board —
 * `useStudioPlacement`'s own finding.
 */
function Acts({
  picked,
  onAdd,
  onRemove,
  onJoin,
  pen,
  onPen,
  drawn,
  onUndo,
  disabled,
}: {
  picked: readonly string[];
  onAdd: (at: { x: number; y: number }) => void;
  onRemove: (ids: readonly string[]) => void;
  onJoin: (from: string, to: string) => void;
  pen: boolean;
  onPen: (pen: boolean) => void;
  drawn: boolean;
  onUndo: () => void;
  disabled: boolean;
}) {
  const flow = useReactFlow();
  const middle = useCallback(() => {
    const at = document.querySelector(".armada-sketch-pad__canvas")?.getBoundingClientRect();
    if (at === undefined) return { x: 0, y: 0 };
    const centre = flow.screenToFlowPosition({ x: at.x + at.width / 2, y: at.y + at.height / 2 });
    // Half a box back, because a box is placed by its top-left corner. The
    // width is read off the token it is drawn at rather than restated here.
    const wide = Number.parseFloat(
      getComputedStyle(document.body).getPropertyValue("--w-sketch-box"),
    );
    const half = Number.isFinite(wide) ? wide / 2 : 0;
    return { x: Math.round(centre.x - half), y: Math.round(centre.y) };
  }, [flow]);

  const joinable = picked.length === A_JOIN_TAKES;
  // The pen is a mode, so any other act puts it down. A press on Add is a box
  // somebody is about to type in, and Join and Remove read a selection the pen
  // cannot change — each of the three is a person having finished drawing.
  const then = (act: () => void) => () => {
    onPen(false);
    act();
  };
  return (
    <div className="armada-sketch-pad__acts" role="group" aria-label="What you can draw">
      <Button
        size="sm"
        aria-pressed={pen}
        disabled={disabled}
        title={pen ? "Drag on the pad to draw. Press again to stop." : "Draw on the pad by hand."}
        onClick={() => onPen(!pen)}
      >
        Draw
      </Button>
      <Button
        size="sm"
        disabled={disabled || !drawn}
        title={drawn ? "Takes the last line you drew back." : "Nothing has been drawn by hand."}
        onClick={onUndo}
      >
        Undo
      </Button>
      <Button size="sm" disabled={disabled} onClick={then(() => onAdd(middle()))}>
        Add a box
      </Button>
      <Button
        size="sm"
        disabled={disabled || !joinable}
        title={joinable ? undefined : "Pick two boxes to join them."}
        onClick={then(() => onJoin(picked[0]!, picked[1]!))}
      >
        Join
      </Button>
      <Button
        size="sm"
        disabled={disabled || picked.length === 0}
        title={picked.length === 0 ? "Pick a box to take it off." : undefined}
        onClick={then(() => onRemove(picked))}
      >
        Remove
      </Button>
    </div>
  );
}

/** One box as React Flow holds it, rebuilt from what the caller gave. */
function toPadNode(box: SketchBox, props: SketchPadProps): PadNode {
  return {
    id: box.id,
    position: { x: box.x, y: box.y },
    type: "sketch",
    data: {
      body: box.body,
      onBody: (body: string) => props.onBody(box.id, body),
      disabled: props.disabled ?? false,
    },
    ariaLabel: box.body.trim() === "" ? EMPTY_BOX : `Box: ${box.body}`,
  };
}

/**
 * What the caller gave, over what React Flow keeps per box — where it sits,
 * what it measured, whether it is selected. The words are always the caller's,
 * so a box typed into redraws in place.
 *
 * **The caller's boxes are merged in before any change is applied**, or a box
 * added after mount is never in the list React Flow measures and stays at
 * `visibility: hidden` forever. `StudioWhiteboard` folds the same way.
 */
function merged(
  given: readonly SketchBox[],
  kept: readonly PadNode[],
  props: SketchPadProps,
): PadNode[] {
  const byId = new Map(kept.map((node) => [node.id, node]));
  return given.map((box) => {
    const fresh = toPadNode(box, props);
    const held = byId.get(box.id);
    return held === undefined ? fresh : { ...held, data: fresh.data, ariaLabel: fresh.ariaLabel };
  });
}

export function SketchPad(props: SketchPadProps) {
  const { label, boxes, lines, strokes, onMove, onAdd, onRemove, onJoin } = props;
  const { onDraw, onUndo, said, onSaid, from } = props;
  const disabled = props.disabled ?? false;
  // Placement is the pad's to hold between moves, the way the whiteboard holds
  // it; the caller hears each one through `onMove` and keeps it in the draft.
  const [kept, setKept] = useState<PadNode[]>(() => boxes.map((box) => toPadNode(box, props)));
  const [picked, setPicked] = useState<readonly string[]>([]);
  // Which of the two the pointer does. A mode the pad holds and never reports:
  // it dies with the surface, and nothing outside it is a picture.
  const [pen, setPen] = useState(false);
  const nodes = merged(boxes, kept, props);
  const drawing = pen && !disabled;

  const onNodesChange = useCallback(
    (changes: NodeChange<PadNode>[]) => {
      setKept((current) => applyNodeChanges(changes, merged(boxes, current, props)));
      // `dragging: false` is a box put down: a drag ending, or an arrow key.
      for (const change of changes) {
        if (change.type === "position" && change.dragging === false && change.position) {
          onMove(change.id, change.position);
        }
      }
    },
    [boxes, props, onMove],
  );

  // Hung off the boxes rather than off `nodes`, which is a fresh array every
  // render — the sides an edge leaves from are decided by where the two sit.
  const edges = useMemo<PadEdge[]>(() => {
    const placed = new Map(boxes.map((box) => [box.id, { position: { x: box.x, y: box.y } }]));
    return lines.map((line) => ({
      id: line.id,
      source: line.from,
      target: line.to,
      ...facingSides(placed.get(line.from), placed.get(line.to)),
      ariaLabel: `A line from ${line.from} to ${line.to}`,
      markerEnd: { type: MarkerType.ArrowClosed },
    }));
  }, [lines, boxes]);

  return (
    <div className="armada-sketch-pad">
      <div className="armada-sketch-pad__canvas">
        <GraphCanvas<PadNode, PadEdge>
          surface="armada-sketch-pad__graph"
          label={label}
          nodes={nodes}
          edges={edges}
          nodeTypes={NODE_TYPES}
          edgeTypes={EDGE_TYPES}
          onNodesChange={onNodesChange}
          onSelectionChange={setPicked}
          nodesDraggable={!disabled}
          multiSelectionKeyCode={JOINS_THE_SELECTION}
          fitViewOptions={FIT}
          controls="signs"
          aside={
            <Acts
              picked={picked}
              onAdd={onAdd}
              onRemove={onRemove}
              onJoin={onJoin}
              pen={drawing}
              onPen={setPen}
              drawn={strokes.length > 0}
              onUndo={onUndo}
              disabled={disabled}
            />
          }
        >
          <Ink strokes={strokes} pen={drawing} onDraw={onDraw} />
        </GraphCanvas>
        {/* A blank canvas under the controls says nothing about what it is for,
            and this is the one moment with no picture to read instead. */}
        {boxes.length > 0 || strokes.length > 0 ? null : (
          <p className="armada-sketch-pad__empty" role="note">
            Nothing is drawn yet. Add a box and write what it is, join the boxes that feed each
            other, or draw on the pad by hand.
          </p>
        )}
      </div>
      <Textarea
        label={SAID_LABEL}
        rows={2}
        value={said}
        placeholder={SAID_PLACEHOLDER}
        disabled={disabled}
        onChange={(event) => onSaid(event.target.value)}
      />
      {from === undefined ? null : (
        <p className="armada-sketch-pad__from">
          Made from <span className="mono">{from}</span> in a Studio.
        </p>
      )}
    </div>
  );
}
