import {
  BaseEdge,
  EdgeLabelRenderer,
  Handle,
  MarkerType,
  Position,
  getBezierPath,
  getNodesBounds,
  useReactFlow,
  useStore,
  type Edge,
  type EdgeProps,
  type FitViewOptions,
  type Node,
  type NodeProps,
} from "@xyflow/react";
import { useEffect, useMemo } from "react";

import { Button } from "../../primitives/Button/Button";
import { GRAPH_CANVAS_SIDES, GraphCanvas, facingSides } from "../GraphCanvas/GraphCanvas";
import { WorkflowStepCard, type WorkflowStepCardProps } from "../WorkflowStepCard/WorkflowStepCard";

/**
 * A Job's graph: the run as the workflow it froze — the steps, the one Plan
 * node hanging off the step that recorded it, and a loop returning above — or
 * the plan itself, a group and the tasks it holds. `#1539`.
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

/**
 * What one edge is. **Only `returns` is painted differently** — the rest are
 * told apart by what they join, which is the reading itself, and a second dash
 * pattern would be a vocabulary nobody asked for.
 */
export type WorkflowCanvasEdgeKind = "leads" | "returns" | "made" | "holds";

/** What each kind is read as to somebody who cannot see the line. */
const SAYS: Record<WorkflowCanvasEdgeKind, string> = {
  leads: "leads to",
  returns: "returns to",
  made: "made",
  holds: "holds",
};

export type WorkflowCanvasEdge = {
  id: string;
  source: string;
  target: string;
  /**
   * `returns` is a loop back to an earlier step — **dashed, and drawn above
   * the spine**, because the run reads left to right and a line returning
   * along it would be read as another way forward.
   */
  kind: WorkflowCanvasEdgeKind;
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
  /**
   * Whether the canvas follows the running step.
   *
   * **Off until a person asks for it.** On by default it wins over the fit and
   * the run opens centred on one card with the rest off screen, which is what
   * `#1539`'s first screenshots showed.
   */
  following?: boolean;
  onFollowing?: (following: boolean) => void;
  /**
   * Where to open when the whole run cannot be drawn and still be read,
   * **widest first** — the first one that is readable in this frame wins.
   * A twelve-step spine is twelve cards of noise at any width, so this is not
   * a narrow-window rule, and a single fallback is not enough either: a run
   * with a whole plan hanging off it clips on every side rather than shrinking.
   */
  opensOn?: readonly (readonly string[])[];
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
 * The plan drops out of the step that recorded it: down off the step's own
 * bottom edge and in at the plan's left, so the drop clears the spine rather
 * than running down the middle of the card it leaves.
 */
const WAS_MADE = { sourceHandle: `s-${Position.Bottom}`, targetHandle: `t-${Position.Left}` };

/** How far out a person may take the run by hand, to see its shape. */
const FURTHEST_OUT = 0.2;

/**
 * How far out a *fit* may go. **Legibility wins over completeness**: a run
 * with eight groups fitted whole draws cards nobody can read, which is the v1
 * complaint `bridge.md` names. Below this the run opens on the step a person
 * is on with its neighbours, and the rest is panned to.
 */
const SMALLEST_READABLE = 0.7;

/** What `fitView` leaves around the run by default, as a factor on the scale. */
const ROOM_TO_BREATHE = 0.9;

/** The same room, as canvas units, where the viewport is set rather than fitted. */
const INSET = 16;

/**
 * Fit the run again whenever the frame changes size.
 *
 * **React Flow fits once, at the size it was first measured at.** Inside a Job
 * the inspector takes its column after that first measure, and the run was
 * left clipped at both ends at every width — `#1539`'s second screenshots.
 * The pane's own measured size is what this reads, so nothing observes the DOM.
 */
function FitsTheFrame({
  options,
  opensOn,
  following,
}: {
  options: FitViewOptions;
  opensOn: readonly (readonly string[])[] | undefined;
  following: boolean;
}) {
  const flow = useReactFlow();
  const width = useStore((state) => state.width);
  const height = useStore((state) => state.height);
  useEffect(() => {
    if (following || width === 0 || height === 0) return;
    const placed = new Map(flow.getNodes().map((node) => [node.id, node]));
    // Whether some part of the run would still be legible in this frame.
    // `fitView` clamps at `minZoom` and says nothing, so a run that does not
    // fit is drawn cut off on every side unless something narrower is chosen
    // first. **Narrow what is shown, never shrink it** — that is the
    // legibility rule, one level down from the whole run.
    const reads = (nodes: readonly { id: string }[]): boolean => {
      const bounds = getNodesBounds(nodes.map((one) => placed.get(one.id)).filter((one) => one !== undefined));
      const scale = Math.min(width / bounds.width, height / bounds.height) * ROOM_TO_BREATHE;
      return scale >= SMALLEST_READABLE;
    };
    const all = flow.getNodes();
    if (opensOn === undefined || reads(all)) {
      void flow.fitView(options);
      return;
    }
    const narrower = opensOn.map((ids) => ids.map((id) => ({ id })));
    const fits = narrower.find(reads);
    if (fits !== undefined) {
      void flow.fitView({ ...options, nodes: fits });
      return;
    }
    // Nothing narrow enough reads in this frame. **Open at the top of the
    // narrowest rather than centred on it**: a fit centres, so a plan taller
    // than its frame loses the step row off the top and its last groups off
    // the bottom at once — and the step a person is on is the one thing that
    // has to be in the picture.
    const last = narrower[narrower.length - 1];
    const found = (last ?? []).map((one) => placed.get(one.id)).filter((one) => one !== undefined);
    if (found.length === 0) {
      void flow.fitView(options);
      return;
    }
    const bounds = getNodesBounds(found);
    void flow.setViewport({
      x: INSET - bounds.x * SMALLEST_READABLE,
      y: INSET - bounds.y * SMALLEST_READABLE,
      zoom: SMALLEST_READABLE,
    });
  }, [flow, following, options, opensOn, width, height]);
  return null;
}

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
  following = false,
  onFollowing,
  opensOn,
}: WorkflowCanvasProps) {
  const nodes = useMemo<CanvasNode[]>(
    () =>
      given.map((entry) => ({
        id: entry.id,
        position: entry.position,
        type: "workflow",
        draggable: false,
        // **`selectable` is what gives the node pointer events at all.** React
        // Flow sets `pointer-events: none` on a node nothing can select, drag
        // or focus, and the pane behind it then swallows every press on the
        // card. Nothing is drawn for a selected node — the card's own
        // `aria-current` is what says which one is open.
        selectable: true,
        focusable: false,
        data: { card: entry.card },
      })),
    [given],
  );

  const edges = useMemo<CanvasEdge[]>(() => {
    const placed = new Map(nodes.map((node) => [node.id, node]));
    const named = new Map(given.map((entry) => [entry.id, entry.card.name]));
    return givenEdges.map((edge) => {
      const returning = edge.kind === "returns";
      const from = named.get(edge.source) ?? edge.source;
      const to = named.get(edge.target) ?? edge.target;
      const sides =
        edge.kind === "returns"
          ? OVER_THE_SPINE
          : edge.kind === "made"
            ? WAS_MADE
            : facingSides(placed.get(edge.source), placed.get(edge.target));
      return {
        id: edge.id,
        source: edge.source,
        target: edge.target,
        ...sides,
        type: "workflow" as const,
        ariaLabel: `${from} ${SAYS[edge.kind]} ${to}`,
        markerEnd: { type: MarkerType.ArrowClosed },
        data: { returning, ...(edge.label === undefined ? {} : { label: edge.label }) },
      };
    });
  }, [given, givenEdges, nodes]);

  const fitViewOptions = useMemo(
    () => ({ maxZoom: 1, minZoom: SMALLEST_READABLE }),
    [],
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
      minZoom={FURTHEST_OUT}
      besideControls={stay}
      fitViewOptions={fitViewOptions}
    >
      <FitsTheFrame options={fitViewOptions} opensOn={opensOn} following={following} />
      <Follows running={running} following={following} />
    </GraphCanvas>
  );
}
