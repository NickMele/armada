import {
  Panel,
  Position,
  ReactFlow,
  ReactFlowProvider,
  useReactFlow,
  type Edge,
  type EdgeTypes,
  type FitViewOptions,
  type Node,
  type NodeTypes,
  type OnNodesChange,
} from "@xyflow/react";
import { useCallback, type ReactNode } from "react";

import { Button } from "../../primitives/Button/Button";

/**
 * The graph surface itself — pan, zoom, fit, selection and the controls that
 * drive them. `docs/contracts/design-system.md`, Hard rules, names React Flow
 * the one sanctioned graph surface and the two things it draws.
 *
 * **Extracted from `StudioWhiteboard`, never copied out of it** (`#1539`).
 * **Placement is the caller's, always**: a Studio remembers where a person put
 * a node, a Job's workflow computes placement from step order, and this holds
 * no positions and reports no moves.
 *
 * **Nothing inside is React Flow's own.** Every node and edge is the caller's
 * type, the controls are `Button`, and `GraphCanvas.css` sets every variable
 * `base.css` would paint with to a token.
 */

/**
 * How the zoom pair is worded. `words` on a surface a person works in, `signs`
 * on one drawn over a run — three words of chrome compete with a Job's run,
 * and the boards draw `−` `+` `Fit` there. The accessible name is the same in
 * both, so nothing a screen reader hears changes with the wording.
 */
export type GraphCanvasControls = "words" | "signs";

export type GraphCanvasProps<N extends Node, E extends Edge> = {
  /**
   * The surface's own class, so a surface can paint what only it has — a
   * Studio's edge labels, a workflow's returning loop. `armada-graph-canvas`
   * is always on the element beside it.
   */
  surface: string;
  /** What the graph is, read to somebody who cannot see it. */
  label: string;
  nodes: N[];
  edges: E[];
  /** Stable module constants in the caller. A fresh object per render is a React Flow update loop. */
  nodeTypes: NodeTypes;
  edgeTypes: EdgeTypes;
  /** Absent means nothing in this graph moves, which is the workflow canvas. */
  onNodesChange?: OnNodesChange<N>;
  nodesDraggable?: boolean;
  /** Held down, a second node joins the selection rather than replacing it. */
  multiSelectionKeyCode?: string[];
  onSelectionChange?: (ids: readonly string[]) => void;
  fitView?: boolean;
  fitViewOptions?: FitViewOptions;
  /**
   * How far out a person — or a fit — may zoom. **React Flow's own floor is
   * 0.5**, which silently clamps `fitView`: a run wider than twice its frame
   * is fitted to a scale it cannot reach and drawn clipped at both ends.
   */
  minZoom?: number;
  controls?: GraphCanvasControls;
  /**
   * Drawn beside the zoom pair and Fit — the one thing a surface adds to its
   * own controls, like the workflow canvas's *Stay on the running step*.
   */
  besideControls?: ReactNode;
  /**
   * What the surface draws over the top-right corner — the acts on what is
   * selected, the relations waiting on a person. The canvas decides nothing.
   */
  aside?: ReactNode;
  /**
   * Mounted inside the graph, where React Flow's own hooks resolve. For a
   * surface that has to read or write the viewport — the workflow canvas stays
   * on the running step this way.
   */
  children?: ReactNode;
};

/** Enough of a placed node to find its centre. Both surfaces hang edges this way. */
type Placed = {
  position: { x: number; y: number };
  measured?: { width?: number; height?: number };
};

/**
 * The facing sides of two nodes: across when they are further apart
 * horizontally than vertically, otherwise up or down. A fixed right-to-left
 * pair looped every edge whose target sat left of or above its source.
 */
export function facingSides(source: Placed | undefined, target: Placed | undefined) {
  const centre = (node: Placed | undefined) => ({
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

/** The four sides a node hangs an edge from. Nothing connects by hand, so none is drawn. */
export const GRAPH_CANVAS_SIDES = [Position.Left, Position.Right, Position.Top, Position.Bottom] as const;

/** React Flow's default descriptions offer delete, which neither surface ever does. */
const ARIA = {
  "node.a11yDescription.default": "Press Enter or Space to select a node, then the arrow keys to move it.",
  "node.a11yDescription.keyboardDisabled": "Press Enter or Space to select a node.",
  "edge.a11yDescription.default": "Press Enter or Space to select an edge.",
  "node.a11yDescription.ariaLiveMessage": ({ direction }: { direction: string }) => `Moved the node ${direction}.`,
};

/** `−` and `+`, the signs the boards draw. Not glyphs: `iconography.md` defaults to none. */
const SIGN = { in: "+", out: "−" } as const;

function Controls({ wording, beside }: { wording: GraphCanvasControls; beside: ReactNode }) {
  const flow = useReactFlow();
  const signs = wording === "signs";
  return (
    <Panel position="bottom-right" className="armada-graph-canvas__controls">
      {beside}
      <Button size="sm" aria-label={signs ? "Zoom out" : undefined} onClick={() => void flow.zoomOut()}>
        {signs ? SIGN.out : "Zoom out"}
      </Button>
      <Button size="sm" aria-label={signs ? "Zoom in" : undefined} onClick={() => void flow.zoomIn()}>
        {signs ? SIGN.in : "Zoom in"}
      </Button>
      <Button size="sm" onClick={() => void flow.fitView()}>
        Fit
      </Button>
    </Panel>
  );
}

function Surface<N extends Node, E extends Edge>({
  surface,
  label,
  nodes,
  edges,
  nodeTypes,
  edgeTypes,
  onNodesChange,
  nodesDraggable = false,
  multiSelectionKeyCode,
  onSelectionChange,
  fitView = true,
  fitViewOptions,
  minZoom,
  controls = "words",
  besideControls,
  aside,
  children,
}: GraphCanvasProps<N, E>) {
  const onPicked = useCallback(
    ({ nodes: picked }: { nodes: N[] }) => onSelectionChange?.(picked.map((node) => node.id)),
    [onSelectionChange],
  );

  return (
    <ReactFlow<N, E>
      className={`armada-graph-canvas ${surface}`}
      aria-label={label}
      colorMode="dark"
      nodes={nodes}
      edges={edges}
      nodeTypes={nodeTypes}
      edgeTypes={edgeTypes}
      onNodesChange={onNodesChange}
      onSelectionChange={onPicked}
      nodesConnectable={false}
      nodesDraggable={nodesDraggable}
      multiSelectionKeyCode={multiSelectionKeyCode}
      edgesReconnectable={false}
      deleteKeyCode={null}
      ariaLabelConfig={ARIA}
      fitView={fitView}
      fitViewOptions={fitViewOptions}
      {...(minZoom === undefined ? {} : { minZoom })}
      // The attribution is a link out of the app, and no surface may navigate.
      proOptions={{ hideAttribution: true }}
    >
      {aside === undefined || aside === null ? null : (
        <Panel position="top-right" className="armada-graph-canvas__aside">
          {aside}
        </Panel>
      )}
      <Controls wording={controls} beside={besideControls} />
      {children}
    </ReactFlow>
  );
}

export function GraphCanvas<N extends Node, E extends Edge>(props: GraphCanvasProps<N, E>) {
  return (
    <div className="armada-graph-canvas-frame">
      <ReactFlowProvider>
        <Surface {...props} />
      </ReactFlowProvider>
    </div>
  );
}
