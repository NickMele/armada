import {
  Handle,
  MarkerType,
  applyNodeChanges,
  useReactFlow,
  type Edge,
  type Node,
  type NodeChange,
  type NodeProps,
} from "@xyflow/react";
import { useCallback, useMemo, useState } from "react";

import { Button } from "../../primitives/Button/Button";
import { Textarea } from "../../primitives/Textarea/Textarea";
import { GRAPH_CANVAS_SIDES, GraphCanvas, facingSides } from "../GraphCanvas/GraphCanvas";

/**
 * Sketch pad — boxes and the lines between them, drawn beside a prompt.
 *
 * **What a prompt cannot say** (`#1547`): when what somebody means is a shape,
 * they type a paragraph describing a picture and the Drone reads the paragraph.
 *
 * **`GraphCanvas` is the graph half** (`#1539`) — no second canvas and no
 * drawing library. A box is words and a place, the way a Studio's `Sketch` node
 * is `{ body: String }`: no colour, no size, nothing to pick.
 *
 * **Nothing here stages anything.** What goes out is a PNG Bridge writes; every
 * edit is reported to the caller, so the drawing survives a switch to Write.
 */
export type SketchPadProps = {
  /** What the picture is, read to somebody who cannot see it. */
  label: string;
  boxes: readonly SketchBox[];
  lines: readonly SketchLine[];
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

/** What the field beside the pad asks for. Never a Wh- opener. */
const SAID_LABEL = "About this sketch";

const SAID_PLACEHOLDER = "What the picture is meant to show.";

/** What an empty box is called, for somebody who cannot see it. */
const EMPTY_BOX = "An empty box";

/** A box's own field, which carries no visible label — the box is the label. */
const BOX_LABEL = "The words in this box";

/** What a box holds, shown rather than described. A phrase, never a paragraph. */
const BOX_PLACEHOLDER = "A panel, a read, a step";

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
 * Held down, a second box joins the selection rather than replacing it — which
 * is how a person picks the two they are joining. Named rather than left to
 * React Flow's platform read, for the two reasons `StudioWhiteboard` states.
 */
const JOINS_THE_SELECTION = ["Meta", "Control"];

/** How many boxes a join takes. Two, and the control says so while it is off. */
const A_JOIN_TAKES = 2;

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
  disabled,
}: {
  picked: readonly string[];
  onAdd: (at: { x: number; y: number }) => void;
  onRemove: (ids: readonly string[]) => void;
  onJoin: (from: string, to: string) => void;
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
  return (
    <div className="armada-sketch-pad__acts" role="group" aria-label="What you can draw">
      <Button size="sm" disabled={disabled} onClick={() => onAdd(middle())}>
        Add a box
      </Button>
      <Button
        size="sm"
        disabled={disabled || !joinable}
        title={joinable ? undefined : "Pick two boxes to join them."}
        onClick={() => onJoin(picked[0]!, picked[1]!)}
      >
        Join
      </Button>
      <Button
        size="sm"
        disabled={disabled || picked.length === 0}
        title={picked.length === 0 ? "Pick a box to take it off." : undefined}
        onClick={() => onRemove(picked)}
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
  const { label, boxes, lines, onMove, onAdd, onRemove, onJoin, said, onSaid, from } = props;
  const disabled = props.disabled ?? false;
  // Placement is the pad's to hold between moves, the way the whiteboard holds
  // it; the caller hears each one through `onMove` and keeps it in the draft.
  const [kept, setKept] = useState<PadNode[]>(() => boxes.map((box) => toPadNode(box, props)));
  const [picked, setPicked] = useState<readonly string[]>([]);
  const nodes = merged(boxes, kept, props);

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
          controls="signs"
          aside={
            <Acts
              picked={picked}
              onAdd={onAdd}
              onRemove={onRemove}
              onJoin={onJoin}
              disabled={disabled}
            />
          }
        />
        {/* A blank canvas under three controls says nothing about what it is
            for, and this is the one moment with no picture to read instead. */}
        {boxes.length > 0 ? null : (
          <p className="armada-sketch-pad__empty" role="note">
            Nothing is drawn yet. Add a box, write what it is, and join the boxes that feed each
            other.
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
