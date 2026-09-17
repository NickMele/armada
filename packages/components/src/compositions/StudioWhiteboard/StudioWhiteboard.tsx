import {
  BaseEdge,
  EdgeLabelRenderer,
  Handle,
  Panel,
  Position,
  ReactFlow,
  MarkerType,
  ReactFlowProvider,
  applyNodeChanges,
  getBezierPath,
  useReactFlow,
  type Edge,
  type EdgeProps,
  type Node,
  type NodeChange,
  type NodeProps,
} from "@xyflow/react";
import { useCallback, useMemo, useState, type ReactNode } from "react";

import { Button } from "../../primitives/Button/Button";
import { STUDIO_NODE_KIND, StudioNode, studioNodeLabel, type StudioNodeOf } from "../StudioNode/StudioNode";

/**
 * Studio whiteboard — a Studio's nodes and edges, placed freely: drag, pan,
 * zoom, fit and select. React Flow draws it; `docs/contracts/design-system.md`,
 * Hard rules, names it the one sanctioned graph surface.
 *
 * **Nothing inside is React Flow's own.** Every node is a `StudioNode`, every
 * edge is drawn here, and the controls are `Button`. React Flow's stylesheet is
 * its structural `base.css`, and every value it would paint is set to a token in
 * `StudioWhiteboard.css`.
 *
 * **No edge carries colour.** Produced is a bare line; Same as, Blocks and
 * Answers carry their label; a proposed relation is dashed until a person
 * accepts it. Nothing here draws or deletes a relation — that is a person's act.
 */

export type StudioWhiteboardNode = {
  id: string;
  /** Where the node sits, in the whiteboard's own coordinates. */
  position: { x: number; y: number };
  node: StudioNodeOf & { title: string; facts?: readonly string[] };
};

export type StudioEdgeRelation = "same_as" | "blocks" | "answers";

export type StudioWhiteboardEdge = {
  id: string;
  source: string;
  target: string;
} & (
  | { kind: "produced" }
  /** Proposed by Helm or a scout, and not yet accepted by a person. */
  | { kind: StudioEdgeRelation; proposed: boolean }
);

export type StudioWhiteboardProps = {
  nodes: readonly StudioWhiteboardNode[];
  edges: readonly StudioWhiteboardEdge[];
  /** A node was put down somewhere new, by pointer or by arrow key. */
  onNodeMoved?: (id: string, position: { x: number; y: number }) => void;
  onSelectionChange?: (ids: readonly string[]) => void;
  /**
   * Nothing moves. A Studio reopens read-only (`docs/concepts/studio.md`), so
   * no node drags, by pointer or by arrow key, and `onNodeMoved` is never
   * called. Pan, zoom, fit and selection still work: reading is looking around.
   */
  readOnly?: boolean;
  /**
   * What the surface draws over the board's top-right corner — the acts on
   * what is selected, and the relations waiting on a person. The whiteboard
   * itself decides nothing.
   */
  children?: ReactNode;
};

/** The relation's label, as `studio.md`, Edges, names it. Produced has none. */
export const STUDIO_EDGE_LABEL: Readonly<Record<StudioEdgeRelation, string>> = {
  same_as: "same as",
  blocks: "blocks",
  answers: "answers",
};

type BoardNodeData = StudioWhiteboardNode["node"];
type BoardNode = Node<BoardNodeData, "studio">;
type BoardEdgeData = { label: string | null; proposed: boolean };
type BoardEdge = Edge<BoardEdgeData, "studio">;

const SIDES = [Position.Left, Position.Right, Position.Top, Position.Bottom] as const;

/**
 * The card, with an anchor for each end of an edge on every side. Which side
 * an edge takes is chosen from where the two nodes sit — see `sides`. Nothing
 * connects by hand, so none is drawn.
 */
function BoardNodeView({ data, selected }: NodeProps<BoardNode>) {
  return (
    <>
      {SIDES.map((side) => (
        <Handle key={`t-${side}`} id={`t-${side}`} type="target" position={side} isConnectable={false} />
      ))}
      <StudioNode {...data} selected={selected} />
      {SIDES.map((side) => (
        <Handle key={`s-${side}`} id={`s-${side}`} type="source" position={side} isConnectable={false} />
      ))}
    </>
  );
}

/**
 * The facing sides of two nodes: across when they are further apart
 * horizontally than vertically, otherwise up or down. A fixed right-to-left
 * pair looped every edge whose target sat left of or above its source.
 */
function sides(source: BoardNode | undefined, target: BoardNode | undefined) {
  const centre = (node: BoardNode | undefined) => ({
    x: (node?.position.x ?? 0) + (node?.measured?.width ?? 0) / 2,
    y: (node?.position.y ?? 0) + (node?.measured?.height ?? 0) / 2,
  });
  const from = centre(source);
  const to = centre(target);
  const dx = to.x - from.x;
  const dy = to.y - from.y;
  const [out, into] =
    Math.abs(dx) >= Math.abs(dy)
      ? dx >= 0 ? [Position.Right, Position.Left] : [Position.Left, Position.Right]
      : dy >= 0 ? [Position.Bottom, Position.Top] : [Position.Top, Position.Bottom];
  return { sourceHandle: `s-${out}`, targetHandle: `t-${into}` };
}

function BoardEdgeView(props: EdgeProps<BoardEdge>) {
  const [path, labelX, labelY] = getBezierPath(props);
  const label = props.data?.label ?? null;
  return (
    <>
      <BaseEdge
        id={props.id}
        path={path}
        markerEnd={props.markerEnd}
        className={props.data?.proposed ? "armada-studio-edge--proposed" : undefined}
      />
      {label === null ? null : (
        <EdgeLabelRenderer>
          <span
            className="armada-studio-edge__label"
            style={{ transform: `translate(-50%, -50%) translate(${labelX}px, ${labelY}px)` }}
          >
            {label}
          </span>
        </EdgeLabelRenderer>
      )}
    </>
  );
}

/**
 * Held down, a second node joins the selection rather than replacing it — which is how a person
 * picks the Notes they are accepting as one Cluster (`#1291`).
 *
 * **Named rather than left to React Flow's default**, which reads the platform off the user agent
 * and so differs between the app and a test of it. **A constant rather than a literal in the
 * element**: React Flow writes this into its own store on every render it sees a new value, and a
 * fresh array each time is an update loop it never leaves.
 */
const JOINS_THE_SELECTION = ["Meta", "Control"];

const NODE_TYPES = { studio: BoardNodeView };
const EDGE_TYPES = { studio: BoardEdgeView };

/** React Flow's default descriptions offer delete, which this surface never does. */
const ARIA = {
  "node.a11yDescription.default": "Press Enter or Space to select a node, then the arrow keys to move it.",
  "node.a11yDescription.keyboardDisabled": "Press Enter or Space to select a node.",
  "edge.a11yDescription.default": "Press Enter or Space to select an edge.",
  "node.a11yDescription.ariaLiveMessage": ({ direction }: { direction: string }) => `Moved the node ${direction}.`,
};

function toBoardNode({ id, position, node }: StudioWhiteboardNode): BoardNode {
  return { id, position, type: "studio", data: node, ariaLabel: studioNodeLabel(node) };
}

/**
 * What the caller gave, over what React Flow keeps per node — where it was put,
 * its measured size, whether it is selected. A node's kind, title and state
 * always come from the caller, so a Finding that freezes redraws in place.
 */
function merged(given: readonly StudioWhiteboardNode[], kept: readonly BoardNode[]): BoardNode[] {
  const byId = new Map(kept.map((node) => [node.id, node]));
  return given.map((entry) => {
    const fresh = toBoardNode(entry);
    const held = byId.get(entry.id);
    return held === undefined ? fresh : { ...held, data: fresh.data, ariaLabel: fresh.ariaLabel };
  });
}

function edgeLabel(edge: StudioWhiteboardEdge, titleOf: (id: string) => string): string {
  const said = edge.kind === "produced" ? "produced" : STUDIO_EDGE_LABEL[edge.kind];
  const proposed = edge.kind !== "produced" && edge.proposed ? ", proposed" : "";
  return `${titleOf(edge.source)} ${said} ${titleOf(edge.target)}${proposed}`;
}

/**
 * Where a node a person adds now should land: the middle of what they are
 * looking at, in the board's own coordinates. `#1364`.
 *
 * **Only callable from inside the board**, since what it reads is the viewport
 * React Flow is holding — which is why the aside is a component of the
 * caller's rather than markup passed down. A node placed at the origin on a
 * board panned somewhere else is a node a person has to go and find.
 */
export function useStudioPlacement(): () => { x: number; y: number } {
  const flow = useReactFlow();
  return useCallback(() => {
    const board = document.querySelector(".armada-studio-whiteboard");
    const at = board?.getBoundingClientRect();
    if (board === null || at === undefined) return { x: 0, y: 0 };
    const middle = flow.screenToFlowPosition({ x: at.x + at.width / 2, y: at.y + at.height / 2 });
    // A node is placed by its top-left corner, so half a card back puts the
    // card in the middle rather than starting there. The width is read off the
    // token the card is drawn at rather than restated here.
    const wide = Number.parseFloat(getComputedStyle(board).getPropertyValue("--w-studio-node"));
    const half = Number.isFinite(wide) ? wide / 2 : 0;
    return { x: Math.round(middle.x - half), y: Math.round(middle.y) };
  }, [flow]);
}

function Controls() {
  const flow = useReactFlow();
  return (
    <Panel position="bottom-right" className="armada-studio-whiteboard__controls">
      <Button size="sm" onClick={() => void flow.zoomIn()}>
        Zoom in
      </Button>
      <Button size="sm" onClick={() => void flow.zoomOut()}>
        Zoom out
      </Button>
      <Button size="sm" onClick={() => void flow.fitView()}>
        Fit
      </Button>
    </Panel>
  );
}

function Board({
  nodes: given,
  edges: givenEdges,
  onNodeMoved,
  onSelectionChange,
  readOnly = false,
  children,
}: StudioWhiteboardProps) {
  // Placement is the whiteboard's to hold between moves; the caller hears each
  // one through `onNodeMoved` and keeps it wherever positions are kept.
  const [kept, setKept] = useState<BoardNode[]>(() => given.map(toBoardNode));
  const nodes = useMemo(() => merged(given, kept), [given, kept]);

  const onNodesChange = useCallback(
    (changes: NodeChange<BoardNode>[]) => {
      setKept((current) => applyNodeChanges(changes, merged(given, current)));
      // `dragging: false` is a node put down: a drag ending, or an arrow key.
      for (const change of changes) {
        if (readOnly) break;
        if (change.type === "position" && change.dragging === false && change.position) {
          onNodeMoved?.(change.id, change.position);
        }
      }
    },
    [given, onNodeMoved, readOnly],
  );

  const edges = useMemo<BoardEdge[]>(() => {
    const titles = new Map(given.map(({ id, node }) => [id, `${STUDIO_NODE_KIND[node.kind]} ${node.title}`]));
    const titleOf = (id: string) => titles.get(id) ?? id;
    const placed = new Map(nodes.map((node) => [node.id, node]));
    return givenEdges.map((edge) => ({
      id: edge.id,
      source: edge.source,
      target: edge.target,
      ...sides(placed.get(edge.source), placed.get(edge.target)),
      type: "studio",
      ariaLabel: edgeLabel(edge, titleOf),
      markerEnd: { type: MarkerType.ArrowClosed },
      data: {
        label: edge.kind === "produced" ? null : STUDIO_EDGE_LABEL[edge.kind],
        proposed: edge.kind !== "produced" && edge.proposed,
      },
    }));
  }, [given, givenEdges, nodes]);

  const onPicked = useCallback(
    ({ nodes: picked }: { nodes: BoardNode[] }) => onSelectionChange?.(picked.map((node) => node.id)),
    [onSelectionChange],
  );

  return (
    <ReactFlow<BoardNode, BoardEdge>
      className="armada-studio-whiteboard"
      colorMode="dark"
      nodes={nodes}
      edges={edges}
      nodeTypes={NODE_TYPES}
      edgeTypes={EDGE_TYPES}
      onNodesChange={onNodesChange}
      onSelectionChange={onPicked}
      nodesConnectable={false}
      nodesDraggable={!readOnly}
      multiSelectionKeyCode={JOINS_THE_SELECTION}
      edgesReconnectable={false}
      deleteKeyCode={null}
      ariaLabelConfig={ARIA}
      fitView
      // The attribution is a link out of the app, and no surface may navigate.
      proOptions={{ hideAttribution: true }}
    >
      {children === undefined ? null : (
        <Panel position="top-right" className="armada-studio-whiteboard__aside">
          {children}
        </Panel>
      )}
      <Controls />
    </ReactFlow>
  );
}

export function StudioWhiteboard(props: StudioWhiteboardProps) {
  return (
    <div className="armada-studio-whiteboard-frame">
      <ReactFlowProvider>
        <Board {...props} />
      </ReactFlowProvider>
    </div>
  );
}
