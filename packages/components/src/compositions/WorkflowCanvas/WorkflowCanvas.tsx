import {
  BaseEdge,
  EdgeLabelRenderer,
  Handle,
  MarkerType,
  Position,
  getBezierPath,
  useReactFlow,
  type Edge,
  type EdgeProps,
  type Node,
  type NodeProps,
} from "@xyflow/react";
import { useEffect, useMemo, type ReactNode } from "react";

import { Button } from "../../primitives/Button/Button";
import { GRAPH_CANVAS_SIDES, GraphCanvas, facingSides } from "../GraphCanvas/GraphCanvas";
import { WorkflowStepCard, type WorkflowStepCardProps } from "../WorkflowStepCard/WorkflowStepCard";

/**
 * A Job's run, drawn as the workflow it froze — steps along a spine, a step's
 * groups hanging under it, and a loop returning above. `#1539`.
 *
 * **Placement is the caller's**, computed from step order, which is the whole
 * difference between this surface and a Studio's whiteboard. Nothing here
 * drags and nothing is saved.
 *
 * **Selection lives on the card, not on the node.** Each card is a button that
 * opens the inspector, so React Flow's own node focus is off: two tab stops on
 * one card, one of which does nothing, is worse than either alone.
 */

export type WorkflowCanvasNode = {
  id: string;
  /** Where it sits, in the canvas's own coordinates. Computed from step order. */
  position: { x: number; y: number };
  card: WorkflowStepCardProps;
};

export type WorkflowCanvasEdge = {
  id: string;
  source: string;
  target: string;
  /**
   * A loop back to an earlier step. **Dashed, and drawn above the spine** — the
   * run reads left to right, so a line returning along it would be read as
   * another way forward.
   */
  returning?: boolean;
  /** What the edge says — a loop's cap, `up to 5 passes`. Absent on the spine. */
  label?: string;
};

export type WorkflowCanvasProps = {
  nodes: readonly WorkflowCanvasNode[];
  edges: readonly WorkflowCanvasEdge[];
  /** What the run is, read to somebody who cannot see it. */
  label: string;
  /**
   * The node the canvas keeps in view while `following`. The step the Job is
   * on — absent on a Job that has stopped, which is when there is nothing to
   * stay on.
   */
  running?: string | null;
  /** Whether the canvas follows the running step. Held by the caller, and remembered. */
  following?: boolean;
  onFollowing?: (following: boolean) => void;
  /**
   * Open on these nodes rather than on the whole run. Narrow hands the step a
   * person is on and its neighbours: a nine-step run fitted whole is nine
   * cards too small to read.
   */
  opensOn?: readonly string[];
  /** Drawn over the top-right corner — the canvas/stacked toggle. */
  aside?: ReactNode;
};

type CanvasNode = Node<{ card: WorkflowStepCardProps }, "workflow">;
type CanvasEdge = Edge<{ label?: string; returning: boolean }, "workflow">;

function NodeView({ data }: NodeProps<CanvasNode>) {
  return (
    <>
      {GRAPH_CANVAS_SIDES.map((side) => (
        <Handle key={`t-${side}`} id={`t-${side}`} type="target" position={side} isConnectable={false} />
      ))}
      <WorkflowStepCard {...data.card} />
      {GRAPH_CANVAS_SIDES.map((side) => (
        <Handle key={`s-${side}`} id={`s-${side}`} type="source" position={side} isConnectable={false} />
      ))}
    </>
  );
}

function EdgeView(props: EdgeProps<CanvasEdge>) {
  const [path, labelX, labelY] = getBezierPath(props);
  const label = props.data?.label;
  return (
    <>
      <BaseEdge
        id={props.id}
        path={path}
        markerEnd={props.markerEnd}
        className={props.data?.returning ? "armada-workflow-edge--returning" : undefined}
      />
      {label === undefined ? null : (
        <EdgeLabelRenderer>
          <span
            className="armada-workflow-edge__label"
            style={{ transform: `translate(-50%, -50%) translate(${labelX}px, ${labelY}px)` }}
          >
            {label}
          </span>
        </EdgeLabelRenderer>
      )}
    </>
  );
}

const NODE_TYPES = { workflow: NodeView };
const EDGE_TYPES = { workflow: EdgeView };

/** A returning edge leaves and arrives on the top edge, which is what puts its arc above the spine. */
const OVER_THE_SPINE = { sourceHandle: `s-${Position.Top}`, targetHandle: `t-${Position.Top}` };

/**
 * Keep the running step in view as the run moves.
 *
 * An effect, because what it writes is React Flow's viewport — state outside
 * React that a render cannot reach. It runs on the id changing and not on
 * every render, so a person who has panned away stays where they panned until
 * the Job moves on.
 */
function Follows({ running, following }: { running: string | null; following: boolean }) {
  const flow = useReactFlow();
  useEffect(() => {
    if (!following || running === null) return;
    void flow.fitView({ nodes: [{ id: running }], duration: 0, maxZoom: 1 });
  }, [flow, following, running]);
  return null;
}

export function WorkflowCanvas({
  nodes: given,
  edges: givenEdges,
  label,
  running = null,
  following = true,
  onFollowing,
  opensOn,
  aside,
}: WorkflowCanvasProps) {
  const nodes = useMemo<CanvasNode[]>(
    () =>
      given.map((entry) => ({
        id: entry.id,
        position: entry.position,
        type: "workflow",
        draggable: false,
        selectable: false,
        focusable: false,
        data: { card: entry.card },
      })),
    [given],
  );

  const edges = useMemo<CanvasEdge[]>(() => {
    const placed = new Map(nodes.map((node) => [node.id, node]));
    const named = new Map(given.map((entry) => [entry.id, entry.card.name]));
    return givenEdges.map((edge) => {
      const returning = edge.returning === true;
      const from = named.get(edge.source) ?? edge.source;
      const to = named.get(edge.target) ?? edge.target;
      return {
        id: edge.id,
        source: edge.source,
        target: edge.target,
        ...(returning ? OVER_THE_SPINE : facingSides(placed.get(edge.source), placed.get(edge.target))),
        type: "workflow" as const,
        ariaLabel: returning ? `${from} returns to ${to}` : `${from} leads to ${to}`,
        markerEnd: { type: MarkerType.ArrowClosed },
        data: { returning, ...(edge.label === undefined ? {} : { label: edge.label }) },
      };
    });
  }, [given, givenEdges, nodes]);

  const fitViewOptions = useMemo(
    () => (opensOn === undefined ? { maxZoom: 1 } : { nodes: opensOn.map((id) => ({ id })), maxZoom: 1 }),
    [opensOn],
  );

  const stay =
    onFollowing === undefined || running === null ? undefined : (
      <Button size="sm" aria-pressed={following} onClick={() => onFollowing(!following)}>
        Stay on the running step
      </Button>
    );

  return (
    <GraphCanvas<CanvasNode, CanvasEdge>
      surface="armada-workflow-canvas"
      label={label}
      nodes={nodes}
      edges={edges}
      nodeTypes={NODE_TYPES}
      edgeTypes={EDGE_TYPES}
      controls="signs"
      besideControls={stay}
      fitViewOptions={fitViewOptions}
      aside={aside}
    >
      <Follows running={running} following={following} />
    </GraphCanvas>
  );
}
